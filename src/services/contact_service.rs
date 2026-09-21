use anyhow::Context;
use chrono::{Datelike, NaiveDate, Utc};
use kubuno_db::search::normalize;
use kubuno_db::{new_id, params, DbPool, DbTx, DbValue};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::contact::{
        AddressField, Contact, ContactField, ContactWithLabels, ContactsListResponse,
        CreateContactDto, CustomField, DateField, ListContactsParams, UpdateContactDto,
    },
    sync,
};

// ─── Derived columns (display_name, etag, the *_norm search columns) ─────────
//
// PostgreSQL derived these in triggers; MySQL and SQLite have no portable form,
// so the module computes them here on every write and binds them explicitly.

/// The display name the old `contacts_search_vector` trigger produced: an
/// explicit non-empty name wins; otherwise the name parts are joined; failing
/// that, the nickname, the organization, or a placeholder.
fn derive_display_name(
    explicit: &str,
    name_prefix: Option<&str>,
    given_name: Option<&str>,
    middle_name: Option<&str>,
    family_name: Option<&str>,
    nickname: Option<&str>,
    organization: Option<&str>,
) -> String {
    let explicit = explicit.trim();
    if !explicit.is_empty() {
        return explicit.to_string();
    }
    let joined = [name_prefix, given_name, middle_name, family_name]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if !joined.is_empty() {
        return joined;
    }
    nickname
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| organization.map(str::trim).filter(|s| !s.is_empty()))
        .unwrap_or("Contact sans nom")
        .to_string()
}

/// Digits of a phone value, so `tel:` search and the phone norm column ignore
/// spaces, `+` and dashes.
fn phone_digits(raw: &str) -> String {
    raw.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// The seven normalized search columns, computed the same way at write time and
/// query time (Snowball French stems + deaccenting), so a stored token and a
/// query token are byte-identical on the three engines.
struct SearchNorms {
    name:   String,
    org:    String,
    email:  String,
    phone:  String,
    job:    String,
    notes:  String,
    addr:   String,
}

fn compute_norms(
    display_name: &str,
    organization: Option<&str>,
    job_title: Option<&str>,
    notes: Option<&str>,
    emails: &[ContactField],
    phones: &[ContactField],
    addresses: &[AddressField],
) -> SearchNorms {
    let email_text = emails.iter().map(|e| e.value.as_str()).collect::<Vec<_>>().join(" ");
    let phone_text = phones.iter().map(|p| phone_digits(&p.value)).collect::<Vec<_>>().join(" ");
    let addr_text = addresses
        .iter()
        .map(|a| {
            [
                a.street.as_deref(),
                a.city.as_deref(),
                a.region.as_deref(),
                a.postcode.as_deref(),
                a.country.as_deref(),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" ");
    SearchNorms {
        name:  normalize(display_name),
        org:   normalize(organization.unwrap_or("")),
        email: normalize(&email_text),
        phone: phone_text,
        job:   normalize(job_title.unwrap_or("")),
        notes: normalize(notes.unwrap_or("")),
        addr:  normalize(&addr_text),
    }
}

/// A fresh, random ETag (formerly `md5(random() || clock_timestamp())`).
fn new_etag() -> String {
    format!("{:x}", Uuid::new_v4().as_u128())
}

// ─── Search parsing ─────────────────────────────────────────────────────────

/// A single parsed search token: an optional field scope and its term.
struct SearchToken {
    scope: Option<String>,
    term:  String,
}

/// Parses a raw query string into scoped tokens. Supports field operators like
/// `email:exemple`, `tel:06`, `org:acme`, `name:dupont`, `job:`, `note:`, `addr:`.
/// Quoted segments keep their spaces. Anything without an operator is a generic
/// term matched against the most common normalized fields.
fn parse_query(raw: &str) -> Vec<SearchToken> {
    let known = ["email", "tel", "phone", "org", "name", "job", "note", "addr", "label"];
    let mut tokens = Vec::new();
    for part in split_respecting_quotes(raw) {
        if let Some((maybe_scope, rest)) = part.split_once(':') {
            let scope = maybe_scope.to_ascii_lowercase();
            if known.contains(&scope.as_str()) && !rest.is_empty() {
                tokens.push(SearchToken { scope: Some(scope), term: rest.to_string() });
                continue;
            }
        }
        tokens.push(SearchToken { scope: None, term: part });
    }
    tokens
}

fn split_respecting_quotes(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for ch in raw.trim().chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !cur.is_empty() { out.push(std::mem::take(&mut cur)); }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

/// A `%pattern%` bind for a substring `LIKE`.
fn like(pattern: &str) -> DbValue {
    DbValue::Text(Some(format!("%{pattern}%")))
}

/// A small dynamic-SQL accumulator: it appends `$n` placeholders in strictly
/// increasing order and keeps the matching binds in lockstep. Every SQL fragment
/// it emits is a `&'static str` or a placeholder; no user input is interpolated.
struct FilterBuilder {
    sql:   String,
    binds: Vec<DbValue>,
    n:     usize,
}

impl FilterBuilder {
    fn new() -> Self {
        FilterBuilder { sql: String::new(), binds: Vec::new(), n: 1 }
    }

    /// Appends ` <lhs> $n <suffix>` and binds `v`. `lhs`/`suffix` are static.
    fn cond(&mut self, prefix: &str, v: DbValue, suffix: &str) {
        self.sql.push_str(prefix);
        self.sql.push('$');
        self.sql.push_str(&self.n.to_string());
        self.sql.push_str(suffix);
        self.binds.push(v);
        self.n += 1;
    }

    /// Appends a static fragment with no bind.
    fn raw(&mut self, s: &str) {
        self.sql.push_str(s);
    }
}

/// Appends the WHERE predicate for one search token.
fn push_token_condition(fb: &mut FilterBuilder, tok: &SearchToken) {
    match tok.scope.as_deref() {
        Some("email") => fb.cond(" email_norm LIKE ", like(&normalize(&tok.term)), ""),
        Some("tel") | Some("phone") => fb.cond(" phone_norm LIKE ", like(&phone_digits(&tok.term)), ""),
        Some("org")  => fb.cond(" org_norm LIKE ",  like(&normalize(&tok.term)), ""),
        Some("name") => fb.cond(" name_norm LIKE ", like(&normalize(&tok.term)), ""),
        Some("job")  => fb.cond(" job_norm LIKE ",  like(&normalize(&tok.term)), ""),
        Some("note") => fb.cond(" notes_norm LIKE ", like(&normalize(&tok.term)), ""),
        Some("addr") => fb.cond(" addr_norm LIKE ", like(&normalize(&tok.term)), ""),
        Some("label") => fb.cond(
            " EXISTS (SELECT 1 FROM contacts.contact_labels cl \
              JOIN contacts.labels l ON l.id = cl.label_id \
              WHERE cl.contact_id = c.id AND LOWER(l.name) LIKE ",
            like(&tok.term.to_lowercase()),
            ")",
        ),
        _ => {
            // Generic term: normalized OR-match over the most useful fields.
            let norm = normalize(&tok.term);
            let digits = phone_digits(&tok.term);
            if norm.is_empty() && digits.is_empty() {
                // Nothing to match on — leave the token out rather than let an
                // empty `%%` return the whole address book.
                fb.raw(" 1 = 1");
                return;
            }
            fb.raw(" (");
            let mut first = true;
            if !norm.is_empty() {
                for col in ["name_norm", "org_norm", "email_norm"] {
                    if !first { fb.raw(" OR"); }
                    first = false;
                    fb.cond(&format!(" {col} LIKE "), like(&norm), "");
                }
            }
            if !digits.is_empty() {
                if !first { fb.raw(" OR"); }
                fb.cond(" phone_norm LIKE ", like(&digits), "");
            }
            fb.raw(")");
        }
    }
}

/// The ORDER BY expression for a sort key. Kept portable: PostgreSQL's
/// `NULLS LAST` has no MySQL/SQLite form, so a "nulls last" ordering is written
/// as `(col IS NULL), col DESC` — a form all three engines share.
fn order_clause(sort: Option<&str>) -> &'static str {
    match sort {
        Some("name_desc")        => "c.display_name DESC",
        Some("first_name")       => "COALESCE(c.given_name, c.display_name) ASC",
        Some("recent")           => "c.created_at DESC",
        Some("updated")          => "c.updated_at DESC",
        Some("organization")     => "COALESCE(NULLIF(c.organization,''), 'zzz') ASC, c.display_name ASC",
        Some("last_interaction") => "(c.last_interaction_at IS NULL), c.last_interaction_at DESC",
        _                        => "c.display_name ASC",
    }
}

/// Ceiling the interactive listing accepts, whatever the caller asks for.
const LIST_MAX_LIMIT: i64 = 500;

pub async fn list_contacts(
    db: &DbPool,
    owner_id: Uuid,
    params: &ListContactsParams,
) -> Result<ContactsListResponse> {
    list_contacts_capped(db, owner_id, params, LIST_MAX_LIMIT).await
}

/// Same listing under an explicit ceiling. Export is the caller that needs it.
pub async fn list_contacts_capped(
    db: &DbPool,
    owner_id: Uuid,
    params: &ListContactsParams,
    max_limit: i64,
) -> Result<ContactsListResponse> {
    let limit  = params.limit.unwrap_or(50).clamp(1, max_limit.max(1));
    let offset = params.offset.unwrap_or(0).max(0);
    let trashed  = params.trashed.unwrap_or(false);
    let archived = params.archived.unwrap_or(false);

    // The shared WHERE, built once so list and count stay identical.
    let mut fb = FilterBuilder::new();
    fb.cond(" c.owner_id = ", owner_id.into(), "");
    fb.cond(" AND c.is_trashed = ", trashed.into(), "");
    if trashed {
        // trash shows everything trashed
    } else if archived {
        fb.cond(" AND c.is_archived = ", true.into(), "");
    } else {
        fb.cond(" AND c.is_archived = ", false.into(), "");
    }
    if let Some(true) = params.starred {
        fb.cond(" AND c.is_starred = ", true.into(), "");
    }
    if let Some(gid) = params.group_id {
        fb.cond(
            " AND EXISTS (SELECT 1 FROM contacts.group_members gm WHERE gm.contact_id = c.id AND gm.group_id = ",
            gid.into(),
            ")",
        );
    }
    if let Some(lid) = params.label_id {
        fb.cond(
            " AND EXISTS (SELECT 1 FROM contacts.contact_labels cl WHERE cl.contact_id = c.id AND cl.label_id = ",
            lid.into(),
            ")",
        );
    }
    match params.filter.as_deref() {
        Some("missing_email") => fb.raw(" AND email_norm = ''"),
        Some("missing_phone") => fb.raw(" AND phone_norm = ''"),
        Some("missing_org")   => fb.raw(" AND (c.organization IS NULL OR c.organization = '')"),
        Some("has_email")     => fb.raw(" AND email_norm <> ''"),
        Some("has_phone")     => fb.raw(" AND phone_norm <> ''"),
        Some("blocked")       => fb.raw(" AND c.is_blocked = TRUE"),
        Some("no_group")      => fb.raw(" AND NOT EXISTS (SELECT 1 FROM contacts.group_members gm WHERE gm.contact_id = c.id)"),
        Some("no_label")      => fb.raw(" AND NOT EXISTS (SELECT 1 FROM contacts.contact_labels cl WHERE cl.contact_id = c.id)"),
        Some("incomplete")    => fb.raw(" AND (email_norm = '' OR phone_norm = '')"),
        _ => {}
    }
    if params.filter.as_deref().is_none_or(|f| f != "blocked") {
        fb.raw(" AND c.is_blocked = FALSE");
    }
    if let Some(q) = params.q.as_ref().filter(|q| !q.trim().is_empty()) {
        for tok in parse_query(q) {
            fb.raw(" AND");
            push_token_condition(&mut fb, &tok);
        }
    }

    let where_sql = fb.sql.clone();
    let where_binds = fb.binds.clone();

    // List: WHERE binds, then LIMIT/OFFSET at the next two placeholders.
    let list_sql = format!(
        "SELECT c.* FROM contacts.contacts c WHERE{where_sql} ORDER BY {} LIMIT ${} OFFSET ${}",
        order_clause(params.sort.as_deref()),
        fb.n,
        fb.n + 1,
    );
    let mut list_binds = where_binds.clone();
    list_binds.push(limit.into());
    list_binds.push(offset.into());
    let contacts = db
        .fetch_all_as::<Contact>(&list_sql, list_binds)
        .await
        .map_err(ContactsError::Database)?;

    let count_sql = format!(
        "SELECT {} FROM contacts.contacts c WHERE{where_sql}",
        db.backend().count_bigint("*"),
    );
    let total: i64 = db
        .fetch_scalar::<i64>(&count_sql, where_binds)
        .await
        .map_err(ContactsError::Database)?;

    let decorated = decorate_with_labels(db, contacts).await?;
    Ok(ContactsListResponse { contacts: decorated, total })
}

/// Attaches each contact's label ids in a single round-trip.
pub async fn decorate_with_labels(
    db: &DbPool,
    contacts: Vec<Contact>,
) -> Result<Vec<ContactWithLabels>> {
    if contacts.is_empty() {
        return Ok(vec![]);
    }
    let ids: Vec<Uuid> = contacts.iter().map(|c| c.id).collect();
    let list = db.backend().in_list(1, ids.len());
    let sql = format!(
        "SELECT contact_id, label_id FROM contacts.contact_labels WHERE contact_id IN ({list})"
    );
    let binds: Vec<DbValue> = ids.iter().map(|id| (*id).into()).collect();
    let rows = db
        .fetch_all_as::<ContactLabelRow>(&sql, binds)
        .await
        .map_err(ContactsError::Database)?;

    let mut map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for r in rows {
        map.entry(r.contact_id).or_default().push(r.label_id);
    }
    Ok(contacts
        .into_iter()
        .map(|c| {
            let label_ids = map.remove(&c.id).unwrap_or_default();
            ContactWithLabels { contact: c, label_ids }
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct ContactLabelRow {
    contact_id: Uuid,
    label_id:   Uuid,
}

pub async fn get_contact(db: &DbPool, owner_id: Uuid, contact_id: Uuid) -> Result<Contact> {
    db.fetch_optional_as::<Contact>(
        "SELECT * FROM contacts.contacts WHERE id = $1 AND owner_id = $2",
        params![contact_id, owner_id],
    )
    .await
    .map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound(format!("Contact {contact_id}")))
}

/// Refuses a creation that would take the account past the instance quota.
pub async fn assert_quota(db: &DbPool, owner_id: Uuid, max: i64, extra: i64) -> Result<()> {
    if max <= 0 {
        return Ok(());
    }
    let count: i64 = db
        .fetch_scalar::<i64>(
            &format!(
                "SELECT {} FROM contacts.contacts WHERE owner_id = $1",
                db.backend().count_bigint("*")
            ),
            params![owner_id],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Comptage des contacts pour le quota");
            ContactsError::Database(e)
        })?;

    if count + extra > max {
        return Err(ContactsError::Validation(format!(
            "Quota de contacts atteint ({max} par utilisateur)"
        )));
    }
    Ok(())
}

pub async fn create_contact(
    db: &DbPool,
    owner_id: Uuid,
    dto: &CreateContactDto,
) -> Result<Contact> {
    let id = dto.id.unwrap_or_else(new_id);
    let display_name = derive_display_name(
        dto.display_name.as_deref().unwrap_or(""),
        dto.name_prefix.as_deref(),
        dto.given_name.as_deref(),
        dto.middle_name.as_deref(),
        dto.family_name.as_deref(),
        dto.nickname.as_deref(),
        dto.organization.as_deref(),
    );
    let norms = compute_norms(
        &display_name,
        dto.organization.as_deref(),
        dto.job_title.as_deref(),
        dto.notes.as_deref(),
        &dto.emails,
        &dto.phones,
        &dto.addresses,
    );
    let now = Utc::now();
    let vcard_uid = new_id().to_string();
    let etag = new_etag();

    let emails    = serde_json::to_value(&dto.emails).unwrap_or(Value::Array(vec![]));
    let phones    = serde_json::to_value(&dto.phones).unwrap_or(Value::Array(vec![]));
    let addresses = serde_json::to_value(&dto.addresses).unwrap_or(Value::Array(vec![]));
    let urls      = serde_json::to_value(&dto.urls).unwrap_or(Value::Array(vec![]));
    let dates     = serde_json::to_value(&dto.dates).unwrap_or(Value::Array(vec![]));
    let relations = serde_json::to_value(&dto.relations).unwrap_or(Value::Array(vec![]));
    let ims       = serde_json::to_value(&dto.instant_messages).unwrap_or(Value::Array(vec![]));
    let custom    = serde_json::to_value(&dto.custom_fields).unwrap_or(Value::Array(vec![]));

    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_contact_seq(&mut tx).await.map_err(ContactsError::Database)?;

    tx.execute(
        "INSERT INTO contacts.contacts \
         (id, owner_id, given_name, middle_name, family_name, name_prefix, name_suffix, \
          nickname, display_name, organization, department, job_title, avatar_color, \
          emails, phones, addresses, urls, dates, relations, instant_messages, \
          custom_fields, notes, is_starred, pronouns, vcard_uid, etag, \
          name_norm, org_norm, email_norm, phone_norm, job_norm, notes_norm, addr_norm, \
          change_seq, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, \
                 $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, \
                 $33, $34, $35, $36)",
        params![
            id, owner_id,
            dto.given_name.as_deref(), dto.middle_name.as_deref(), dto.family_name.as_deref(),
            dto.name_prefix.as_deref(), dto.name_suffix.as_deref(), dto.nickname.as_deref(),
            display_name,
            dto.organization.as_deref(), dto.department.as_deref(), dto.job_title.as_deref(),
            dto.avatar_color.as_deref().unwrap_or("#1a73e8"),
            emails, phones, addresses, urls, dates, relations, ims, custom,
            dto.notes.as_deref(), dto.is_starred.unwrap_or(false), dto.pronouns.as_deref(),
            vcard_uid, etag,
            norms.name, norms.org, norms.email, norms.phone, norms.job, norms.notes, norms.addr,
            seq, now, now,
        ],
    )
    .await
    .map_err(ContactsError::Database)?;

    tx.commit().await.map_err(ContactsError::Database)?;

    get_contact(db, owner_id, id).await
}

pub async fn update_contact(
    db: &DbPool,
    owner_id: Uuid,
    contact_id: Uuid,
    dto: &UpdateContactDto,
) -> Result<Contact> {
    let existing = get_contact(db, owner_id, contact_id).await?;

    let given_name   = dto.given_name.as_deref().or(existing.given_name.as_deref());
    let middle_name  = dto.middle_name.as_deref().or(existing.middle_name.as_deref());
    let family_name  = dto.family_name.as_deref().or(existing.family_name.as_deref());
    let name_prefix  = dto.name_prefix.as_deref().or(existing.name_prefix.as_deref());
    let name_suffix  = dto.name_suffix.as_deref().or(existing.name_suffix.as_deref());
    let nickname     = dto.nickname.as_deref().or(existing.nickname.as_deref());
    let organization = dto.organization.as_deref().or(existing.organization.as_deref());
    let department   = dto.department.as_deref().or(existing.department.as_deref());
    let job_title    = dto.job_title.as_deref().or(existing.job_title.as_deref());
    let avatar_color = dto.avatar_color.as_deref().unwrap_or(&existing.avatar_color);
    let pronouns     = dto.pronouns.as_deref().or(existing.pronouns.as_deref());
    let notes        = dto.notes.as_deref().or(existing.notes.as_deref());
    let is_starred   = dto.is_starred.unwrap_or(existing.is_starred);

    let emails_v    = dto.emails.clone().unwrap_or_else(|| existing.emails.0.clone());
    let phones_v    = dto.phones.clone().unwrap_or_else(|| existing.phones.0.clone());
    let addresses_v = dto.addresses.clone().unwrap_or_else(|| existing.addresses.0.clone());
    let urls_v      = dto.urls.clone().unwrap_or_else(|| existing.urls.0.clone());
    let dates_v     = dto.dates.clone().unwrap_or_else(|| existing.dates.0.clone());
    let relations_v = dto.relations.clone().unwrap_or_else(|| existing.relations.0.clone());
    let ims_v       = dto.instant_messages.clone().unwrap_or_else(|| existing.instant_messages.0.clone());
    let custom_v    = dto.custom_fields.clone().unwrap_or_else(|| existing.custom_fields.0.clone());

    let display_name = derive_display_name(
        dto.display_name.as_deref().unwrap_or(&existing.display_name),
        name_prefix, given_name, middle_name, family_name, nickname, organization,
    );
    let norms = compute_norms(&display_name, organization, job_title, notes, &emails_v, &phones_v, &addresses_v);
    let etag = new_etag();
    let now = Utc::now();

    let emails    = serde_json::to_value(&emails_v).unwrap_or(Value::Array(vec![]));
    let phones    = serde_json::to_value(&phones_v).unwrap_or(Value::Array(vec![]));
    let addresses = serde_json::to_value(&addresses_v).unwrap_or(Value::Array(vec![]));
    let urls      = serde_json::to_value(&urls_v).unwrap_or(Value::Array(vec![]));
    let dates     = serde_json::to_value(&dates_v).unwrap_or(Value::Array(vec![]));
    let relations = serde_json::to_value(&relations_v).unwrap_or(Value::Array(vec![]));
    let ims       = serde_json::to_value(&ims_v).unwrap_or(Value::Array(vec![]));
    let custom    = serde_json::to_value(&custom_v).unwrap_or(Value::Array(vec![]));

    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_contact_seq(&mut tx).await.map_err(ContactsError::Database)?;
    tx.execute(
        "UPDATE contacts.contacts SET \
         given_name = $1, middle_name = $2, family_name = $3, \
         name_prefix = $4, name_suffix = $5, nickname = $6, display_name = $7, \
         organization = $8, department = $9, job_title = $10, avatar_color = $11, \
         emails = $12, phones = $13, addresses = $14, urls = $15, dates = $16, \
         relations = $17, instant_messages = $18, custom_fields = $19, \
         notes = $20, is_starred = $21, pronouns = $22, etag = $23, \
         name_norm = $24, org_norm = $25, email_norm = $26, phone_norm = $27, \
         job_norm = $28, notes_norm = $29, addr_norm = $30, change_seq = $31, updated_at = $32 \
         WHERE id = $33 AND owner_id = $34",
        params![
            given_name, middle_name, family_name, name_prefix, name_suffix, nickname, display_name,
            organization, department, job_title, avatar_color,
            emails, phones, addresses, urls, dates, relations, ims, custom,
            notes, is_starred, pronouns, etag,
            norms.name, norms.org, norms.email, norms.phone, norms.job, norms.notes, norms.addr,
            seq, now,
            contact_id, owner_id,
        ],
    )
    .await
    .map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;

    let updated = get_contact(db, owner_id, contact_id).await?;

    // Record field-level history for scalar fields (best-effort).
    record_changes(db, owner_id, contact_id, &existing, &updated).await;

    Ok(updated)
}

/// Inserts a change_log row for each scalar field that actually changed.
async fn record_changes(db: &DbPool, owner_id: Uuid, contact_id: Uuid, before: &Contact, after: &Contact) {
    let tracked: [(&str, &Option<String>, &Option<String>); 6] = [
        ("organization", &before.organization, &after.organization),
        ("department",   &before.department,   &after.department),
        ("job_title",    &before.job_title,    &after.job_title),
        ("nickname",     &before.nickname,     &after.nickname),
        ("notes",        &before.notes,        &after.notes),
        ("pronouns",     &before.pronouns,     &after.pronouns),
    ];
    let mut changes: Vec<(&str, Option<String>, Option<String>)> = Vec::new();
    if before.display_name != after.display_name {
        changes.push(("display_name", Some(before.display_name.clone()), Some(after.display_name.clone())));
    }
    for (field, old, new) in tracked {
        if old != new {
            changes.push((field, old.clone(), new.clone()));
        }
    }
    for (field, old, new) in changes {
        if let Err(e) = db
            .execute(
                "INSERT INTO contacts.change_log (contact_id, owner_id, field, old_value, new_value) \
                 VALUES ($1, $2, $3, $4, $5)",
                params![contact_id, owner_id, field, old, new],
            )
            .await
        {
            tracing::warn!(error = %e, "Enregistrement de l'historique échoué");
        }
    }
}

/// One field-level change entry, for the contact history timeline.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ChangeEntry {
    pub field:      String,
    pub old_value:  Option<String>,
    pub new_value:  Option<String>,
    pub changed_at: chrono::DateTime<Utc>,
}

pub async fn get_history(db: &DbPool, owner_id: Uuid, contact_id: Uuid) -> Result<Vec<ChangeEntry>> {
    db.fetch_all_as::<ChangeEntry>(
        "SELECT field, old_value, new_value, changed_at \
         FROM contacts.change_log \
         WHERE contact_id = $1 AND owner_id = $2 \
         ORDER BY changed_at DESC \
         LIMIT 200",
        params![contact_id, owner_id],
    )
    .await
    .map_err(ContactsError::Database)
}

// ─── Per-contact flag mutations (portable delta: fresh seq + updated_at) ─────

async fn set_trashed_tx(tx: &mut DbTx, owner_id: Uuid, id: Uuid, trashed: bool) -> std::result::Result<u64, sqlx::Error> {
    let seq = sync::next_contact_seq(tx).await?;
    let now = Utc::now();
    let trashed_at: Option<chrono::DateTime<Utc>> = if trashed { Some(now) } else { None };
    tx.execute(
        "UPDATE contacts.contacts SET is_trashed = $1, trashed_at = $2, change_seq = $3, updated_at = $4 \
         WHERE id = $5 AND owner_id = $6",
        params![trashed, trashed_at, seq, now, id, owner_id],
    )
    .await
}

async fn set_starred_tx(tx: &mut DbTx, owner_id: Uuid, id: Uuid, starred: bool) -> std::result::Result<u64, sqlx::Error> {
    let seq = sync::next_contact_seq(tx).await?;
    let now = Utc::now();
    tx.execute(
        "UPDATE contacts.contacts SET is_starred = $1, change_seq = $2, updated_at = $3 \
         WHERE id = $4 AND owner_id = $5",
        params![starred, seq, now, id, owner_id],
    )
    .await
}

async fn set_archived_tx(tx: &mut DbTx, owner_id: Uuid, id: Uuid, archived: bool) -> std::result::Result<u64, sqlx::Error> {
    let seq = sync::next_contact_seq(tx).await?;
    let now = Utc::now();
    let archived_at: Option<chrono::DateTime<Utc>> = if archived { Some(now) } else { None };
    tx.execute(
        "UPDATE contacts.contacts SET is_archived = $1, archived_at = $2, change_seq = $3, updated_at = $4 \
         WHERE id = $5 AND owner_id = $6",
        params![archived, archived_at, seq, now, id, owner_id],
    )
    .await
}

async fn set_blocked_tx(tx: &mut DbTx, owner_id: Uuid, id: Uuid, blocked: bool) -> std::result::Result<u64, sqlx::Error> {
    let seq = sync::next_contact_seq(tx).await?;
    let now = Utc::now();
    tx.execute(
        "UPDATE contacts.contacts SET is_blocked = $1, change_seq = $2, updated_at = $3 \
         WHERE id = $4 AND owner_id = $5",
        params![blocked, seq, now, id, owner_id],
    )
    .await
}

/// Hard-deletes one contact and writes its tombstone in the same transaction.
async fn delete_contact_tx(tx: &mut DbTx, owner_id: Uuid, id: Uuid) -> std::result::Result<u64, sqlx::Error> {
    let seq = sync::next_contact_seq(tx).await?;
    let affected = tx
        .execute(
            "DELETE FROM contacts.contacts WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await?;
    if affected == 1 {
        sync::record_contact_tombstone(tx, id, owner_id, seq).await?;
    }
    Ok(affected)
}

// ─── Bulk operations ────────────────────────────────────────────────────────

pub enum BulkAction {
    Trash,
    Restore,
    DeletePermanently,
    Star,
    Unstar,
    Archive,
    Unarchive,
    Block,
    Unblock,
}

pub async fn bulk_action(
    db: &DbPool,
    owner_id: Uuid,
    ids: &[Uuid],
    action: BulkAction,
) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let mut total = 0u64;
    for &id in ids {
        let affected = match action {
            BulkAction::Trash             => set_trashed_tx(&mut tx, owner_id, id, true).await,
            BulkAction::Restore           => set_trashed_tx(&mut tx, owner_id, id, false).await,
            BulkAction::DeletePermanently => delete_contact_tx(&mut tx, owner_id, id).await,
            BulkAction::Star              => set_starred_tx(&mut tx, owner_id, id, true).await,
            BulkAction::Unstar            => set_starred_tx(&mut tx, owner_id, id, false).await,
            BulkAction::Archive           => set_archived_tx(&mut tx, owner_id, id, true).await,
            BulkAction::Unarchive         => set_archived_tx(&mut tx, owner_id, id, false).await,
            BulkAction::Block             => set_blocked_tx(&mut tx, owner_id, id, true).await,
            BulkAction::Unblock           => set_blocked_tx(&mut tx, owner_id, id, false).await,
        }
        .map_err(ContactsError::Database)?;
        total += affected;
    }
    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(total)
}

pub async fn set_archived(db: &DbPool, owner_id: Uuid, contact_id: Uuid, archived: bool) -> Result<()> {
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    set_archived_tx(&mut tx, owner_id, contact_id, archived).await.map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(())
}

pub async fn set_blocked(db: &DbPool, owner_id: Uuid, contact_id: Uuid, blocked: bool) -> Result<()> {
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    set_blocked_tx(&mut tx, owner_id, contact_id, blocked).await.map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(())
}

// ─── Duplicate detection & merge ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroup {
    pub reason:   String,
    pub contacts: Vec<Contact>,
}

fn norm_phone(raw: &str) -> Option<String> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 6 {
        return None;
    }
    Some(digits.chars().rev().take(9).collect::<String>().chars().rev().collect())
}

pub async fn find_duplicates(db: &DbPool, owner_id: Uuid) -> Result<Vec<DuplicateGroup>> {
    let contacts = db
        .fetch_all_as::<Contact>(
            "SELECT * FROM contacts.contacts \
             WHERE owner_id = $1 AND is_trashed = FALSE AND is_archived = FALSE",
            params![owner_id],
        )
        .await
        .map_err(ContactsError::Database)?;

    if contacts.len() < 2 {
        return Ok(vec![]);
    }

    let ignored: HashSet<(Uuid, Uuid)> = db
        .fetch_all_as::<DedupPair>(
            "SELECT contact_a, contact_b FROM contacts.dedup_ignored WHERE owner_id = $1",
            params![owner_id],
        )
        .await
        .map_err(ContactsError::Database)?
        .into_iter()
        .map(|p| ordered_pair(p.contact_a, p.contact_b))
        .collect();

    let n = contacts.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let union = |parent: &mut Vec<usize>, a: usize, b: usize| {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[ra] = rb;
        }
    };

    let mut key_map: HashMap<String, usize> = HashMap::new();
    let mut reason_by_root: HashMap<usize, &'static str> = HashMap::new();
    for (i, c) in contacts.iter().enumerate() {
        let mut keys: Vec<(String, &'static str)> = Vec::new();
        for e in c.emails.0.iter() {
            let v = e.value.trim().to_lowercase();
            if !v.is_empty() {
                keys.push((format!("email:{v}"), "email"));
            }
        }
        for p in c.phones.0.iter() {
            if let Some(np) = norm_phone(&p.value) {
                keys.push((format!("phone:{np}"), "phone"));
            }
        }
        let dn = c.display_name.trim().to_lowercase();
        if !dn.is_empty() {
            keys.push((format!("name:{dn}"), "name"));
        }
        for (k, reason) in keys {
            if let Some(&j) = key_map.get(&k) {
                union(&mut parent, i, j);
                let root = find(&mut parent, i);
                reason_by_root.entry(root).or_insert(reason);
            } else {
                key_map.insert(k, i);
            }
        }
    }

    let mut comps: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        comps.entry(r).or_default().push(i);
    }

    let mut groups = Vec::new();
    for (root, members) in comps {
        if members.len() < 2 {
            continue;
        }
        if members.len() == 2 {
            let pair = ordered_pair(contacts[members[0]].id, contacts[members[1]].id);
            if ignored.contains(&pair) {
                continue;
            }
        }
        let reason = reason_by_root.get(&root).copied().unwrap_or("name");
        let reason = match reason {
            "email" => "Même adresse e-mail",
            "phone" => "Même numéro de téléphone",
            _        => "Même nom",
        };
        let mut list: Vec<Contact> = members.iter().map(|&idx| contacts[idx].clone()).collect();
        list.sort_by_key(|a| a.created_at);
        groups.push(DuplicateGroup { reason: reason.to_string(), contacts: list });
    }
    groups.sort_by_key(|b| std::cmp::Reverse(b.contacts.len()));
    Ok(groups)
}

#[derive(sqlx::FromRow)]
struct DedupPair {
    contact_a: Uuid,
    contact_b: Uuid,
}

fn ordered_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a <= b { (a, b) } else { (b, a) }
}

pub async fn ignore_duplicate_pair(db: &DbPool, owner_id: Uuid, a: Uuid, b: Uuid) -> Result<()> {
    let (a, b) = ordered_pair(a, b);
    let backend = db.backend();
    let sql = format!(
        "INSERT {}INTO contacts.dedup_ignored (owner_id, contact_a, contact_b) VALUES ($1, $2, $3){}",
        backend.insert_ignore_prefix(),
        backend.on_conflict_do_nothing(&["owner_id", "contact_a", "contact_b"]),
    );
    db.execute(&sql, params![owner_id, a, b])
        .await
        .map_err(ContactsError::Database)?;
    Ok(())
}

fn merge_contact_fields(into: &mut Vec<ContactField>, from: &[ContactField]) {
    let mut seen: HashSet<String> = into.iter().map(|f| f.value.trim().to_lowercase()).collect();
    for f in from {
        let key = f.value.trim().to_lowercase();
        if !key.is_empty() && seen.insert(key) {
            into.push(f.clone());
        }
    }
}

fn merge_json_dedup<T: Serialize + Clone>(into: &mut Vec<T>, from: &[T]) {
    let mut seen: HashSet<String> =
        into.iter().filter_map(|v| serde_json::to_string(v).ok()).collect();
    for v in from {
        if let Ok(key) = serde_json::to_string(v) {
            if seen.insert(key) {
                into.push(v.clone());
            }
        }
    }
}

pub async fn merge_contacts(
    db: &DbPool,
    owner_id: Uuid,
    primary_id: Uuid,
    duplicate_ids: &[Uuid],
) -> Result<Contact> {
    let dup_ids: Vec<Uuid> = duplicate_ids.iter().copied().filter(|id| *id != primary_id).collect();
    if dup_ids.is_empty() {
        return get_contact(db, owner_id, primary_id).await;
    }

    let mut primary = get_contact(db, owner_id, primary_id).await?;
    let dup_list = db.backend().in_list(1, dup_ids.len());
    let mut dup_binds: Vec<DbValue> = dup_ids.iter().map(|id| (*id).into()).collect();
    dup_binds.push(owner_id.into());
    let owner_ph = dup_ids.len() + 1;
    let dups = db
        .fetch_all_as::<Contact>(
            &format!("SELECT * FROM contacts.contacts WHERE id IN ({dup_list}) AND owner_id = ${owner_ph}"),
            dup_binds,
        )
        .await
        .map_err(ContactsError::Database)?;

    if dups.is_empty() {
        return Err(ContactsError::NotFound("Aucun doublon valide à fusionner".into()));
    }

    fn fill(target: &mut Option<String>, source: &Option<String>) {
        if target.as_deref().unwrap_or("").trim().is_empty() {
            if let Some(s) = source {
                if !s.trim().is_empty() {
                    *target = Some(s.clone());
                }
            }
        }
    }
    for d in &dups {
        fill(&mut primary.given_name,  &d.given_name);
        fill(&mut primary.middle_name, &d.middle_name);
        fill(&mut primary.family_name, &d.family_name);
        fill(&mut primary.name_prefix, &d.name_prefix);
        fill(&mut primary.name_suffix, &d.name_suffix);
        fill(&mut primary.nickname,    &d.nickname);
        fill(&mut primary.organization, &d.organization);
        fill(&mut primary.department,   &d.department);
        fill(&mut primary.job_title,    &d.job_title);
        fill(&mut primary.pronouns,     &d.pronouns);
        if primary.notes.as_deref().unwrap_or("").trim().is_empty() {
            primary.notes = d.notes.clone();
        }
        merge_contact_fields(&mut primary.emails.0, &d.emails.0);
        merge_contact_fields(&mut primary.phones.0, &d.phones.0);
        merge_contact_fields(&mut primary.urls.0, &d.urls.0);
        merge_contact_fields(&mut primary.relations.0, &d.relations.0);
        merge_contact_fields(&mut primary.instant_messages.0, &d.instant_messages.0);
        merge_json_dedup::<AddressField>(&mut primary.addresses.0, &d.addresses.0);
        merge_json_dedup::<DateField>(&mut primary.dates.0, &d.dates.0);
        merge_json_dedup::<CustomField>(&mut primary.custom_fields.0, &d.custom_fields.0);
        primary.is_starred = primary.is_starred || d.is_starred;
    }

    // Groups whose membership will gain the primary — read before the write so
    // they can be bumped afterwards (the old trigger's job), letting a syncing
    // client see the new member.
    let gm_list = db.backend().in_list(1, dup_ids.len());
    let gm_binds: Vec<DbValue> = dup_ids.iter().map(|id| (*id).into()).collect();
    let affected_groups: Vec<GroupIdRow> = db
        .fetch_all_as::<GroupIdRow>(
            &format!("SELECT DISTINCT group_id FROM contacts.group_members WHERE contact_id IN ({gm_list})"),
            gm_binds,
        )
        .await
        .map_err(ContactsError::Database)?;

    let mut tx = db.begin().await.map_err(ContactsError::Database)?;

    // Reassign group + label memberships (INSERT ... SELECT, skip conflicts).
    let backend = tx.backend();
    let gm_sel_list = backend.in_list(2, dup_ids.len());
    let mut gm_ins_binds: Vec<DbValue> = vec![primary_id.into()];
    gm_ins_binds.extend(dup_ids.iter().map(|id| DbValue::from(*id)));
    tx.execute(
        &format!(
            "INSERT {}INTO contacts.group_members (group_id, contact_id) \
             SELECT group_id, $1 FROM contacts.group_members WHERE contact_id IN ({gm_sel_list}){}",
            backend.insert_ignore_prefix(),
            backend.on_conflict_do_nothing(&["group_id", "contact_id"]),
        ),
        gm_ins_binds,
    )
    .await
    .map_err(ContactsError::Database)?;

    let cl_sel_list = backend.in_list(2, dup_ids.len());
    let mut cl_ins_binds: Vec<DbValue> = vec![primary_id.into()];
    cl_ins_binds.extend(dup_ids.iter().map(|id| DbValue::from(*id)));
    tx.execute(
        &format!(
            "INSERT {}INTO contacts.contact_labels (label_id, contact_id) \
             SELECT label_id, $1 FROM contacts.contact_labels WHERE contact_id IN ({cl_sel_list}){}",
            backend.insert_ignore_prefix(),
            backend.on_conflict_do_nothing(&["label_id", "contact_id"]),
        ),
        cl_ins_binds,
    )
    .await
    .map_err(ContactsError::Database)?;

    let rem_list = backend.in_list(2, dup_ids.len());
    let mut rem_binds: Vec<DbValue> = vec![primary_id.into()];
    rem_binds.extend(dup_ids.iter().map(|id| DbValue::from(*id)));
    tx.execute(
        &format!("UPDATE contacts.reminders SET contact_id = $1 WHERE contact_id IN ({rem_list})"),
        rem_binds,
    )
    .await
    .map_err(ContactsError::Database)?;

    let il_list = backend.in_list(2, dup_ids.len());
    let mut il_binds: Vec<DbValue> = vec![primary_id.into()];
    il_binds.extend(dup_ids.iter().map(|id| DbValue::from(*id)));
    tx.execute(
        &format!("UPDATE contacts.interaction_log SET contact_id = $1 WHERE contact_id IN ({il_list})"),
        il_binds,
    )
    .await
    .map_err(ContactsError::Database)?;

    // Persist the merged primary with a fresh seq, etag and search columns.
    let display_name = derive_display_name(
        &primary.display_name,
        primary.name_prefix.as_deref(), primary.given_name.as_deref(),
        primary.middle_name.as_deref(), primary.family_name.as_deref(),
        primary.nickname.as_deref(), primary.organization.as_deref(),
    );
    let norms = compute_norms(
        &display_name, primary.organization.as_deref(), primary.job_title.as_deref(),
        primary.notes.as_deref(), &primary.emails.0, &primary.phones.0, &primary.addresses.0,
    );
    let etag = new_etag();
    let now = Utc::now();
    let seq = sync::next_contact_seq(&mut tx).await.map_err(ContactsError::Database)?;

    let emails    = serde_json::to_value(&primary.emails.0).unwrap_or(Value::Array(vec![]));
    let phones    = serde_json::to_value(&primary.phones.0).unwrap_or(Value::Array(vec![]));
    let addresses = serde_json::to_value(&primary.addresses.0).unwrap_or(Value::Array(vec![]));
    let urls      = serde_json::to_value(&primary.urls.0).unwrap_or(Value::Array(vec![]));
    let dates     = serde_json::to_value(&primary.dates.0).unwrap_or(Value::Array(vec![]));
    let relations = serde_json::to_value(&primary.relations.0).unwrap_or(Value::Array(vec![]));
    let ims       = serde_json::to_value(&primary.instant_messages.0).unwrap_or(Value::Array(vec![]));
    let custom    = serde_json::to_value(&primary.custom_fields.0).unwrap_or(Value::Array(vec![]));

    tx.execute(
        "UPDATE contacts.contacts SET \
         given_name=$1, middle_name=$2, family_name=$3, name_prefix=$4, name_suffix=$5, \
         nickname=$6, display_name=$7, organization=$8, department=$9, job_title=$10, pronouns=$11, \
         emails=$12, phones=$13, addresses=$14, urls=$15, dates=$16, relations=$17, \
         instant_messages=$18, custom_fields=$19, notes=$20, is_starred=$21, etag=$22, \
         name_norm=$23, org_norm=$24, email_norm=$25, phone_norm=$26, job_norm=$27, notes_norm=$28, \
         addr_norm=$29, change_seq=$30, updated_at=$31 \
         WHERE id=$32 AND owner_id=$33",
        params![
            primary.given_name.as_deref(), primary.middle_name.as_deref(), primary.family_name.as_deref(),
            primary.name_prefix.as_deref(), primary.name_suffix.as_deref(), primary.nickname.as_deref(),
            display_name, primary.organization.as_deref(), primary.department.as_deref(),
            primary.job_title.as_deref(), primary.pronouns.as_deref(),
            emails, phones, addresses, urls, dates, relations, ims, custom,
            primary.notes.as_deref(), primary.is_starred, etag,
            norms.name, norms.org, norms.email, norms.phone, norms.job, norms.notes, norms.addr,
            seq, now,
            primary_id, owner_id,
        ],
    )
    .await
    .map_err(ContactsError::Database)?;

    // Delete the duplicates, each with its tombstone.
    for d in &dups {
        delete_contact_tx(&mut tx, owner_id, d.id).await.map_err(ContactsError::Database)?;
    }

    // Bump the groups that gained the primary as a member.
    for g in &affected_groups {
        sync::touch_group(&mut tx, g.group_id).await.map_err(ContactsError::Database)?;
    }

    tx.commit().await.map_err(ContactsError::Database)?;
    get_contact(db, owner_id, primary_id).await
}

#[derive(sqlx::FromRow)]
struct GroupIdRow {
    group_id: Uuid,
}

// ─── Birthdays / upcoming dates ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct UpcomingDate {
    pub contact_id:   Uuid,
    pub display_name: String,
    pub avatar_color: String,
    pub label:        String,
    pub date:         String,
    pub next_occurrence: String,
    pub days_until:   i64,
    pub age:          Option<i32>,
}

fn parse_date_value(raw: &str) -> Option<(Option<i32>, u32, u32)> {
    let t = raw.trim();
    if let Ok(d) = NaiveDate::parse_from_str(t, "%Y-%m-%d") {
        return Some((Some(d.year()), d.month(), d.day()));
    }
    if let Some(rest) = t.strip_prefix("--") {
        let parts: Vec<&str> = rest.split(['-', '/']).collect();
        if parts.len() == 2 {
            if let (Ok(m), Ok(d)) = (parts[0].parse(), parts[1].parse()) {
                return Some((None, m, d));
            }
        }
    }
    let seps: &[char] = &['-', '/', '.'];
    let parts: Vec<&str> = t.split(seps).collect();
    if parts.len() == 3 {
        if let (Ok(a), Ok(b), Ok(c)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>(), parts[2].parse::<i32>()) {
            if a > 31 {
                return Some((Some(a), b as u32, c as u32));
            } else {
                return Some((Some(c), b as u32, a as u32));
            }
        }
    } else if parts.len() == 2 {
        if let (Ok(m), Ok(d)) = (parts[0].parse(), parts[1].parse()) {
            return Some((None, m, d));
        }
    }
    None
}

pub async fn upcoming_dates(db: &DbPool, owner_id: Uuid, within_days: i64) -> Result<Vec<UpcomingDate>> {
    // The `dates` JSON array is walked in Rust rather than in SQL, so no engine
    // needs a `jsonb_array_length`.
    let contacts = db
        .fetch_all_as::<Contact>(
            "SELECT * FROM contacts.contacts \
             WHERE owner_id = $1 AND is_trashed = FALSE AND is_archived = FALSE",
            params![owner_id],
        )
        .await
        .map_err(ContactsError::Database)?;

    let today = Utc::now().date_naive();
    let mut out: Vec<UpcomingDate> = Vec::new();
    for c in &contacts {
        for d in c.dates.0.iter() {
            let (year, month, day) = match parse_date_value(&d.value) {
                Some(v) => v,
                None => continue,
            };
            let mut next = match NaiveDate::from_ymd_opt(today.year(), month, day) {
                Some(date) => date,
                None => continue,
            };
            if next < today {
                next = NaiveDate::from_ymd_opt(today.year() + 1, month, day).unwrap_or(next);
            }
            let days_until = (next - today).num_days();
            if days_until < 0 || days_until > within_days {
                continue;
            }
            let age = year.map(|y| next.year() - y);
            let label = d.label.clone().unwrap_or_else(|| {
                if d.field_type.eq_ignore_ascii_case("birthday") { "Anniversaire".into() }
                else { d.field_type.clone() }
            });
            out.push(UpcomingDate {
                contact_id:      c.id,
                display_name:    c.display_name.clone(),
                avatar_color:    c.avatar_color.clone(),
                label,
                date:            d.value.clone(),
                next_occurrence: next.to_string(),
                days_until,
                age,
            });
        }
    }
    out.sort_by_key(|u| u.days_until);
    Ok(out)
}

pub async fn trash_contact(db: &DbPool, owner_id: Uuid, contact_id: Uuid) -> Result<()> {
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let rows = set_trashed_tx(&mut tx, owner_id, contact_id, true).await.map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;
    if rows == 0 { return Err(ContactsError::NotFound(format!("Contact {contact_id}"))); }
    Ok(())
}

pub async fn restore_contact(db: &DbPool, owner_id: Uuid, contact_id: Uuid) -> Result<()> {
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    set_trashed_tx(&mut tx, owner_id, contact_id, false).await.map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(())
}

pub async fn delete_contact_permanently(
    db: &DbPool,
    owner_id: Uuid,
    contact_id: Uuid,
) -> Result<()> {
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let rows = delete_contact_tx(&mut tx, owner_id, contact_id).await.map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;
    if rows == 0 { return Err(ContactsError::NotFound(format!("Contact {contact_id}"))); }
    Ok(())
}

pub async fn empty_trash(db: &DbPool, owner_id: Uuid) -> Result<u64> {
    // Read the ids first so each gets a tombstone (the old AFTER DELETE trigger's
    // job), then delete them one by one in a single transaction.
    let ids: Vec<Uuid> = db
        .fetch_all_as::<ContactIdRow>(
            "SELECT id FROM contacts.contacts WHERE owner_id = $1 AND is_trashed = TRUE",
            params![owner_id],
        )
        .await
        .map_err(ContactsError::Database)?
        .into_iter()
        .map(|r| r.id)
        .collect();
    if ids.is_empty() {
        return Ok(0);
    }
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let mut total = 0u64;
    for id in ids {
        total += delete_contact_tx(&mut tx, owner_id, id).await.map_err(ContactsError::Database)?;
    }
    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(total)
}

#[derive(sqlx::FromRow)]
struct ContactIdRow {
    id: Uuid,
}

pub async fn star_contact(db: &DbPool, owner_id: Uuid, contact_id: Uuid, starred: bool) -> Result<()> {
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    set_starred_tx(&mut tx, owner_id, contact_id, starred).await.map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(())
}

/// Stores a contact's avatar path, bumping its seq so the change re-syncs.
pub async fn set_avatar_path(db: &DbPool, owner_id: Uuid, contact_id: Uuid, path: &str) -> Result<()> {
    let now = Utc::now();
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_contact_seq(&mut tx).await.map_err(ContactsError::Database)?;
    tx.execute(
        "UPDATE contacts.contacts SET avatar_path = $1, change_seq = $2, updated_at = $3 \
         WHERE id = $4 AND owner_id = $5",
        params![path, seq, now, contact_id, owner_id],
    )
    .await
    .map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(())
}

/// Links a personal contact to the account it was created from.
pub async fn set_kubuno_user_id(db: &DbPool, owner_id: Uuid, contact_id: Uuid, kubuno_user_id: Uuid) -> Result<()> {
    let now = Utc::now();
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_contact_seq(&mut tx).await.map_err(ContactsError::Database)?;
    tx.execute(
        "UPDATE contacts.contacts SET kubuno_user_id = $1, change_seq = $2, updated_at = $3 \
         WHERE id = $4 AND owner_id = $5",
        params![kubuno_user_id, seq, now, contact_id, owner_id],
    )
    .await
    .map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(())
}

pub async fn get_avatar_path(db: &DbPool, contact_id: Uuid) -> Result<Option<String>> {
    Ok(db
        .fetch_optional_scalar::<Option<String>>(
            "SELECT avatar_path FROM contacts.contacts WHERE id = $1",
            params![contact_id],
        )
        .await
        .map_err(ContactsError::Database)?
        .flatten())
}

pub async fn log_interaction(
    db: &DbPool,
    contact_id: Uuid,
    owner_id: Uuid,
    interaction_type: &str,
    source_module: Option<&str>,
) -> anyhow::Result<()> {
    db.execute(
        "INSERT INTO contacts.interaction_log (id, contact_id, owner_id, interaction_type, source_module) \
         VALUES ($1, $2, $3, $4, $5)",
        params![new_id(), contact_id, owner_id, interaction_type, source_module],
    )
    .await
    .context("log_interaction")?;
    Ok(())
}

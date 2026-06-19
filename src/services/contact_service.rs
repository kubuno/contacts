use anyhow::Context;
use chrono::{Datelike, NaiveDate, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::contact::{
        AddressField, Contact, ContactField, ContactWithLabels, ContactsListResponse,
        CreateContactDto, CustomField, DateField, ListContactsParams, UpdateContactDto,
    },
};

/// A single parsed search token: an optional field scope and its term.
struct SearchToken {
    scope: Option<String>,
    term:  String,
}

/// Parses a raw query string into scoped tokens. Supports field operators like
/// `email:gmail`, `tel:06`, `org:acme`, `name:dupont`, `job:`, `note:`, `addr:`.
/// Quoted segments keep their spaces. Anything without an operator is a generic
/// term matched against the full-text vector and the most common fields.
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

/// Appends the WHERE predicate for one search token to the query builder.
fn push_token_condition(qb: &mut QueryBuilder<'_, Postgres>, tok: &SearchToken) {
    let like = format!("%{}%", tok.term);
    match tok.scope.as_deref() {
        Some("email") => {
            qb.push("EXISTS (SELECT 1 FROM jsonb_array_elements(c.emails) e WHERE e->>'value' ILIKE ");
            qb.push_bind(like).push(")");
        }
        Some("tel") | Some("phone") => {
            // Compare digits-only so formatting (spaces, +, dashes) is ignored.
            let digits: String = tok.term.chars().filter(|c| c.is_ascii_digit()).collect();
            qb.push("EXISTS (SELECT 1 FROM jsonb_array_elements(c.phones) p \
                     WHERE regexp_replace(p->>'value', '\\D', '', 'g') LIKE ");
            qb.push_bind(format!("%{digits}%")).push(")");
        }
        Some("org") => {
            qb.push("unaccent(COALESCE(c.organization,'')) ILIKE unaccent(");
            qb.push_bind(like).push(")");
        }
        Some("name") => {
            qb.push("unaccent(c.display_name) ILIKE unaccent(");
            qb.push_bind(like).push(")");
        }
        Some("job") => {
            qb.push("unaccent(COALESCE(c.job_title,'')) ILIKE unaccent(");
            qb.push_bind(like).push(")");
        }
        Some("note") => {
            qb.push("unaccent(COALESCE(c.notes,'')) ILIKE unaccent(");
            qb.push_bind(like).push(")");
        }
        Some("addr") => {
            qb.push("c.addresses::text ILIKE ");
            qb.push_bind(like);
        }
        Some("label") => {
            qb.push("EXISTS (SELECT 1 FROM contacts.contact_labels cl \
                     JOIN contacts.labels l ON l.id = cl.label_id \
                     WHERE cl.contact_id = c.id AND unaccent(l.name) ILIKE unaccent(");
            qb.push_bind(like).push("))");
        }
        _ => {
            // Generic term: full-text OR fuzzy match on the most useful fields.
            qb.push("(c.search_vector @@ plainto_tsquery('simple', unaccent(");
            qb.push_bind(tok.term.clone());
            qb.push(")) OR unaccent(c.display_name) ILIKE unaccent(");
            qb.push_bind(like.clone());
            qb.push(") OR unaccent(COALESCE(c.organization,'')) ILIKE unaccent(");
            qb.push_bind(like);
            qb.push("))");
        }
    }
}

fn order_clause(sort: Option<&str>) -> &'static str {
    match sort {
        Some("name_desc")        => "c.display_name DESC",
        Some("first_name")       => "COALESCE(c.given_name, c.display_name) ASC",
        Some("recent")           => "c.created_at DESC",
        Some("updated")          => "c.updated_at DESC",
        Some("organization")     => "COALESCE(NULLIF(c.organization,''), 'zzz') ASC, c.display_name ASC",
        Some("last_interaction") => "c.last_interaction_at DESC NULLS LAST",
        _                        => "c.display_name ASC",
    }
}

pub async fn list_contacts(
    db: &PgPool,
    owner_id: Uuid,
    params: &ListContactsParams,
) -> Result<ContactsListResponse> {
    let limit  = params.limit.unwrap_or(50).clamp(1, 500);
    let offset = params.offset.unwrap_or(0).max(0);
    let trashed  = params.trashed.unwrap_or(false);
    let archived = params.archived.unwrap_or(false);

    // Build the shared FROM/WHERE so list and count stay in sync.
    let build_filters = |qb: &mut QueryBuilder<'_, Postgres>| {
        qb.push(" WHERE c.owner_id = ").push_bind(owner_id);
        qb.push(" AND c.is_trashed = ").push_bind(trashed);
        // Archived contacts are hidden from the normal lists unless requested.
        if trashed {
            // trash shows everything trashed
        } else if archived {
            qb.push(" AND c.is_archived = TRUE");
        } else {
            qb.push(" AND c.is_archived = FALSE");
        }
        if let Some(true) = params.starred {
            qb.push(" AND c.is_starred = TRUE");
        }
        if let Some(gid) = params.group_id {
            qb.push(" AND EXISTS (SELECT 1 FROM contacts.group_members gm \
                     WHERE gm.contact_id = c.id AND gm.group_id = ");
            qb.push_bind(gid).push(")");
        }
        if let Some(lid) = params.label_id {
            qb.push(" AND EXISTS (SELECT 1 FROM contacts.contact_labels cl \
                     WHERE cl.contact_id = c.id AND cl.label_id = ");
            qb.push_bind(lid).push(")");
        }
        match params.filter.as_deref() {
            Some("missing_email") => { qb.push(" AND jsonb_array_length(c.emails) = 0"); }
            Some("missing_phone") => { qb.push(" AND jsonb_array_length(c.phones) = 0"); }
            Some("missing_org")   => { qb.push(" AND (c.organization IS NULL OR c.organization = '')"); }
            Some("has_email")     => { qb.push(" AND jsonb_array_length(c.emails) > 0"); }
            Some("has_phone")     => { qb.push(" AND jsonb_array_length(c.phones) > 0"); }
            Some("blocked")       => { qb.push(" AND c.is_blocked = TRUE"); }
            Some("no_group")      => { qb.push(" AND NOT EXISTS (SELECT 1 FROM contacts.group_members gm WHERE gm.contact_id = c.id)"); }
            Some("no_label")      => { qb.push(" AND NOT EXISTS (SELECT 1 FROM contacts.contact_labels cl WHERE cl.contact_id = c.id)"); }
            Some("incomplete")    => { qb.push(" AND (jsonb_array_length(c.emails) = 0 OR jsonb_array_length(c.phones) = 0)"); }
            _ => {}
        }
        if params.filter.as_deref().is_none_or(|f| f != "blocked") {
            qb.push(" AND c.is_blocked = FALSE");
        }
        if let Some(q) = params.q.as_ref().filter(|q| !q.trim().is_empty()) {
            for tok in parse_query(q) {
                qb.push(" AND ");
                push_token_condition(qb, &tok);
            }
        }
    };

    let mut list_qb: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT c.* FROM contacts.contacts c");
    build_filters(&mut list_qb);
    list_qb.push(" ORDER BY ").push(order_clause(params.sort.as_deref()));
    list_qb.push(" LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(offset);

    let contacts = list_qb
        .build_query_as::<Contact>()
        .fetch_all(db)
        .await
        .map_err(ContactsError::Database)?;

    let mut count_qb: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM contacts.contacts c");
    build_filters(&mut count_qb);
    let total = count_qb
        .build_query_scalar::<i64>()
        .fetch_one(db)
        .await
        .map_err(ContactsError::Database)?;

    let decorated = decorate_with_labels(db, contacts).await?;
    Ok(ContactsListResponse { contacts: decorated, total })
}

/// Attaches each contact's label ids in a single round-trip.
pub async fn decorate_with_labels(
    db: &PgPool,
    contacts: Vec<Contact>,
) -> Result<Vec<ContactWithLabels>> {
    if contacts.is_empty() {
        return Ok(vec![]);
    }
    let ids: Vec<Uuid> = contacts.iter().map(|c| c.id).collect();
    let rows = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT contact_id, label_id FROM contacts.contact_labels WHERE contact_id = ANY($1)",
    )
    .bind(&ids)
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)?;

    let mut map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (contact_id, label_id) in rows {
        map.entry(contact_id).or_default().push(label_id);
    }
    Ok(contacts
        .into_iter()
        .map(|c| {
            let label_ids = map.remove(&c.id).unwrap_or_default();
            ContactWithLabels { contact: c, label_ids }
        })
        .collect())
}

pub async fn get_contact(db: &PgPool, owner_id: Uuid, contact_id: Uuid) -> Result<Contact> {
    sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts.contacts WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id).bind(owner_id)
    .fetch_optional(db).await
    .map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound(format!("Contact {contact_id}")))
}

pub async fn create_contact(
    db: &PgPool,
    owner_id: Uuid,
    dto: &CreateContactDto,
) -> Result<Contact> {
    let emails   = serde_json::to_value(&dto.emails).unwrap_or(Value::Array(vec![]));
    let phones   = serde_json::to_value(&dto.phones).unwrap_or(Value::Array(vec![]));
    let addresses = serde_json::to_value(&dto.addresses).unwrap_or(Value::Array(vec![]));
    let urls     = serde_json::to_value(&dto.urls).unwrap_or(Value::Array(vec![]));
    let dates    = serde_json::to_value(&dto.dates).unwrap_or(Value::Array(vec![]));
    let relations = serde_json::to_value(&dto.relations).unwrap_or(Value::Array(vec![]));
    let ims      = serde_json::to_value(&dto.instant_messages).unwrap_or(Value::Array(vec![]));
    let custom   = serde_json::to_value(&dto.custom_fields).unwrap_or(Value::Array(vec![]));

    sqlx::query_as::<_, Contact>(
        "INSERT INTO contacts.contacts
         (owner_id, given_name, middle_name, family_name, name_prefix, name_suffix,
          nickname, display_name, organization, department, job_title, avatar_color,
          emails, phones, addresses, urls, dates, relations, instant_messages,
          custom_fields, notes, is_starred, pronouns)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                 $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
         RETURNING *",
    )
    .bind(owner_id)
    .bind(&dto.given_name).bind(&dto.middle_name).bind(&dto.family_name)
    .bind(&dto.name_prefix).bind(&dto.name_suffix).bind(&dto.nickname)
    .bind(dto.display_name.as_deref().unwrap_or(""))
    .bind(&dto.organization).bind(&dto.department).bind(&dto.job_title)
    .bind(dto.avatar_color.as_deref().unwrap_or("#1a73e8"))
    .bind(emails).bind(phones).bind(addresses).bind(urls)
    .bind(dates).bind(relations).bind(ims).bind(custom)
    .bind(&dto.notes).bind(dto.is_starred.unwrap_or(false)).bind(&dto.pronouns)
    .fetch_one(db).await.map_err(ContactsError::Database)
}

pub async fn update_contact(
    db: &PgPool,
    owner_id: Uuid,
    contact_id: Uuid,
    dto: &UpdateContactDto,
) -> Result<Contact> {
    let existing = get_contact(db, owner_id, contact_id).await?;

    let given_name   = dto.given_name.as_ref().or(existing.given_name.as_ref());
    let middle_name  = dto.middle_name.as_ref().or(existing.middle_name.as_ref());
    let family_name  = dto.family_name.as_ref().or(existing.family_name.as_ref());
    let name_prefix  = dto.name_prefix.as_ref().or(existing.name_prefix.as_ref());
    let name_suffix  = dto.name_suffix.as_ref().or(existing.name_suffix.as_ref());
    let nickname     = dto.nickname.as_ref().or(existing.nickname.as_ref());
    let display_name = dto.display_name.as_deref().unwrap_or(&existing.display_name);
    let organization = dto.organization.as_ref().or(existing.organization.as_ref());
    let department   = dto.department.as_ref().or(existing.department.as_ref());
    let job_title    = dto.job_title.as_ref().or(existing.job_title.as_ref());
    let avatar_color = dto.avatar_color.as_deref().unwrap_or(&existing.avatar_color);
    let pronouns     = dto.pronouns.as_ref().or(existing.pronouns.as_ref());
    let notes        = dto.notes.as_ref().or(existing.notes.as_ref());
    let is_starred   = dto.is_starred.unwrap_or(existing.is_starred);

    let emails    = dto.emails.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.emails.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let phones    = dto.phones.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.phones.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let addresses = dto.addresses.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.addresses.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let urls      = dto.urls.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.urls.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let dates     = dto.dates.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.dates.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let relations = dto.relations.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.relations.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let ims       = dto.instant_messages.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.instant_messages.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let custom    = dto.custom_fields.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.custom_fields.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());

    let updated = sqlx::query_as::<_, Contact>(
        "UPDATE contacts.contacts SET
         given_name = $3, middle_name = $4, family_name = $5,
         name_prefix = $6, name_suffix = $7, nickname = $8, display_name = $9,
         organization = $10, department = $11, job_title = $12, avatar_color = $13,
         emails = $14, phones = $15, addresses = $16, urls = $17, dates = $18,
         relations = $19, instant_messages = $20, custom_fields = $21,
         notes = $22, is_starred = $23, pronouns = $24
         WHERE id = $1 AND owner_id = $2
         RETURNING *",
    )
    .bind(contact_id).bind(owner_id)
    .bind(given_name).bind(middle_name).bind(family_name)
    .bind(name_prefix).bind(name_suffix).bind(nickname).bind(display_name)
    .bind(organization).bind(department).bind(job_title).bind(avatar_color)
    .bind(emails).bind(phones).bind(addresses).bind(urls).bind(dates)
    .bind(relations).bind(ims).bind(custom).bind(notes).bind(is_starred).bind(pronouns)
    .fetch_one(db).await.map_err(ContactsError::Database)?;

    // Record field-level history for scalar fields (best-effort).
    record_changes(db, owner_id, contact_id, &existing, &updated).await;

    Ok(updated)
}

/// Inserts a change_log row for each scalar field that actually changed.
async fn record_changes(db: &PgPool, owner_id: Uuid, contact_id: Uuid, before: &Contact, after: &Contact) {
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
        if let Err(e) = sqlx::query(
            "INSERT INTO contacts.change_log (contact_id, owner_id, field, old_value, new_value)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(contact_id).bind(owner_id).bind(field).bind(old).bind(new)
        .execute(db)
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

pub async fn get_history(db: &PgPool, owner_id: Uuid, contact_id: Uuid) -> Result<Vec<ChangeEntry>> {
    sqlx::query_as::<_, ChangeEntry>(
        "SELECT field, old_value, new_value, changed_at
         FROM contacts.change_log
         WHERE contact_id = $1 AND owner_id = $2
         ORDER BY changed_at DESC
         LIMIT 200",
    )
    .bind(contact_id)
    .bind(owner_id)
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)
}

// ─── Bulk operations ────────────────────────────────────────────────────────

/// Bulk action applied to a set of contacts owned by `owner_id`.
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
    db: &PgPool,
    owner_id: Uuid,
    ids: &[Uuid],
    action: BulkAction,
) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let rows = match action {
        BulkAction::Trash => sqlx::query(
            "UPDATE contacts.contacts SET is_trashed = TRUE, trashed_at = NOW()
             WHERE id = ANY($1) AND owner_id = $2",
        ),
        BulkAction::Restore => sqlx::query(
            "UPDATE contacts.contacts SET is_trashed = FALSE, trashed_at = NULL
             WHERE id = ANY($1) AND owner_id = $2",
        ),
        BulkAction::DeletePermanently => sqlx::query(
            "DELETE FROM contacts.contacts WHERE id = ANY($1) AND owner_id = $2",
        ),
        BulkAction::Star => sqlx::query(
            "UPDATE contacts.contacts SET is_starred = TRUE WHERE id = ANY($1) AND owner_id = $2",
        ),
        BulkAction::Unstar => sqlx::query(
            "UPDATE contacts.contacts SET is_starred = FALSE WHERE id = ANY($1) AND owner_id = $2",
        ),
        BulkAction::Archive => sqlx::query(
            "UPDATE contacts.contacts SET is_archived = TRUE, archived_at = NOW()
             WHERE id = ANY($1) AND owner_id = $2",
        ),
        BulkAction::Unarchive => sqlx::query(
            "UPDATE contacts.contacts SET is_archived = FALSE, archived_at = NULL
             WHERE id = ANY($1) AND owner_id = $2",
        ),
        BulkAction::Block => sqlx::query(
            "UPDATE contacts.contacts SET is_blocked = TRUE WHERE id = ANY($1) AND owner_id = $2",
        ),
        BulkAction::Unblock => sqlx::query(
            "UPDATE contacts.contacts SET is_blocked = FALSE WHERE id = ANY($1) AND owner_id = $2",
        ),
    }
    .bind(ids)
    .bind(owner_id)
    .execute(db)
    .await
    .map_err(ContactsError::Database)?
    .rows_affected();
    Ok(rows)
}

/// Toggles archive flag on a single contact.
pub async fn set_archived(db: &PgPool, owner_id: Uuid, contact_id: Uuid, archived: bool) -> Result<()> {
    sqlx::query(
        "UPDATE contacts.contacts
         SET is_archived = $3, archived_at = CASE WHEN $3 THEN NOW() ELSE NULL END
         WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id).bind(owner_id).bind(archived)
    .execute(db).await.map_err(ContactsError::Database)?;
    Ok(())
}

/// Toggles the blocked flag on a single contact.
pub async fn set_blocked(db: &PgPool, owner_id: Uuid, contact_id: Uuid, blocked: bool) -> Result<()> {
    sqlx::query(
        "UPDATE contacts.contacts SET is_blocked = $3 WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id).bind(owner_id).bind(blocked)
    .execute(db).await.map_err(ContactsError::Database)?;
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
    // Compare on the last 9 digits to bridge national vs international forms.
    Some(digits.chars().rev().take(9).collect::<String>().chars().rev().collect())
}

/// Finds groups of likely-duplicate contacts that share an email, a phone
/// number or an identical display name. Ignored pairs are pruned.
pub async fn find_duplicates(db: &PgPool, owner_id: Uuid) -> Result<Vec<DuplicateGroup>> {
    let contacts = sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts.contacts
         WHERE owner_id = $1 AND is_trashed = FALSE AND is_archived = FALSE",
    )
    .bind(owner_id)
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)?;

    if contacts.len() < 2 {
        return Ok(vec![]);
    }

    let ignored: HashSet<(Uuid, Uuid)> = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT contact_a, contact_b FROM contacts.dedup_ignored WHERE owner_id = $1",
    )
    .bind(owner_id)
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)?
    .into_iter()
    .map(|(a, b)| ordered_pair(a, b))
    .collect();

    // Union-find over contacts that share a normalised key.
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

    // Collect components of size >= 2.
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
        // Drop a pair that the user explicitly dismissed.
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

fn ordered_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a <= b { (a, b) } else { (b, a) }
}

/// Marks a pair of contacts as "not a duplicate" so it stops being suggested.
pub async fn ignore_duplicate_pair(db: &PgPool, owner_id: Uuid, a: Uuid, b: Uuid) -> Result<()> {
    let (a, b) = ordered_pair(a, b);
    sqlx::query(
        "INSERT INTO contacts.dedup_ignored (owner_id, contact_a, contact_b)
         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(owner_id).bind(a).bind(b)
    .execute(db).await.map_err(ContactsError::Database)?;
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

/// Merges `duplicate_ids` into `primary_id`: arrays are unioned, empty scalar
/// fields on the primary are filled from duplicates, group/label memberships and
/// reminders/interactions are reassigned, then the duplicates are deleted.
pub async fn merge_contacts(
    db: &PgPool,
    owner_id: Uuid,
    primary_id: Uuid,
    duplicate_ids: &[Uuid],
) -> Result<Contact> {
    let dup_ids: Vec<Uuid> = duplicate_ids.iter().copied().filter(|id| *id != primary_id).collect();
    if dup_ids.is_empty() {
        return get_contact(db, owner_id, primary_id).await;
    }

    let mut primary = get_contact(db, owner_id, primary_id).await?;
    let dups = sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts.contacts WHERE id = ANY($1) AND owner_id = $2",
    )
    .bind(&dup_ids)
    .bind(owner_id)
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)?;

    if dups.is_empty() {
        return Err(ContactsError::NotFound("Aucun doublon valide à fusionner".into()));
    }

    // Combine scalar + array fields into the primary record.
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

    let mut tx = db.begin().await.map_err(ContactsError::Database)?;

    // Reassign group + label memberships, reminders and interactions.
    sqlx::query(
        "INSERT INTO contacts.group_members (group_id, contact_id)
         SELECT group_id, $1 FROM contacts.group_members WHERE contact_id = ANY($2)
         ON CONFLICT DO NOTHING",
    )
    .bind(primary_id).bind(&dup_ids).execute(&mut *tx).await.map_err(ContactsError::Database)?;
    sqlx::query(
        "INSERT INTO contacts.contact_labels (label_id, contact_id)
         SELECT label_id, $1 FROM contacts.contact_labels WHERE contact_id = ANY($2)
         ON CONFLICT DO NOTHING",
    )
    .bind(primary_id).bind(&dup_ids).execute(&mut *tx).await.map_err(ContactsError::Database)?;
    sqlx::query("UPDATE contacts.reminders SET contact_id = $1 WHERE contact_id = ANY($2)")
        .bind(primary_id).bind(&dup_ids).execute(&mut *tx).await.map_err(ContactsError::Database)?;
    sqlx::query("UPDATE contacts.interaction_log SET contact_id = $1 WHERE contact_id = ANY($2)")
        .bind(primary_id).bind(&dup_ids).execute(&mut *tx).await.map_err(ContactsError::Database)?;

    // Persist the merged primary.
    let emails    = serde_json::to_value(&primary.emails.0).unwrap_or(Value::Array(vec![]));
    let phones    = serde_json::to_value(&primary.phones.0).unwrap_or(Value::Array(vec![]));
    let addresses = serde_json::to_value(&primary.addresses.0).unwrap_or(Value::Array(vec![]));
    let urls      = serde_json::to_value(&primary.urls.0).unwrap_or(Value::Array(vec![]));
    let dates     = serde_json::to_value(&primary.dates.0).unwrap_or(Value::Array(vec![]));
    let relations = serde_json::to_value(&primary.relations.0).unwrap_or(Value::Array(vec![]));
    let ims       = serde_json::to_value(&primary.instant_messages.0).unwrap_or(Value::Array(vec![]));
    let custom    = serde_json::to_value(&primary.custom_fields.0).unwrap_or(Value::Array(vec![]));

    let merged = sqlx::query_as::<_, Contact>(
        "UPDATE contacts.contacts SET
         given_name=$3, middle_name=$4, family_name=$5, name_prefix=$6, name_suffix=$7,
         nickname=$8, organization=$9, department=$10, job_title=$11, pronouns=$12,
         emails=$13, phones=$14, addresses=$15, urls=$16, dates=$17, relations=$18,
         instant_messages=$19, custom_fields=$20, notes=$21, is_starred=$22
         WHERE id=$1 AND owner_id=$2 RETURNING *",
    )
    .bind(primary_id).bind(owner_id)
    .bind(&primary.given_name).bind(&primary.middle_name).bind(&primary.family_name)
    .bind(&primary.name_prefix).bind(&primary.name_suffix).bind(&primary.nickname)
    .bind(&primary.organization).bind(&primary.department).bind(&primary.job_title)
    .bind(&primary.pronouns)
    .bind(emails).bind(phones).bind(addresses).bind(urls).bind(dates)
    .bind(relations).bind(ims).bind(custom).bind(&primary.notes).bind(primary.is_starred)
    .fetch_one(&mut *tx).await.map_err(ContactsError::Database)?;

    sqlx::query("DELETE FROM contacts.contacts WHERE id = ANY($1) AND owner_id = $2")
        .bind(&dup_ids).bind(owner_id).execute(&mut *tx).await.map_err(ContactsError::Database)?;

    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(merged)
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
    // Accept YYYY-MM-DD, YYYY/MM/DD, --MM-DD, MM-DD, DD/MM/YYYY (best-effort).
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
                return Some((Some(a), b as u32, c as u32)); // YYYY-MM-DD
            } else {
                return Some((Some(c), b as u32, a as u32)); // DD-MM-YYYY
            }
        }
    } else if parts.len() == 2 {
        if let (Ok(m), Ok(d)) = (parts[0].parse(), parts[1].parse()) {
            return Some((None, m, d));
        }
    }
    None
}

/// Returns upcoming birthdays/anniversaries within `within_days` days.
pub async fn upcoming_dates(db: &PgPool, owner_id: Uuid, within_days: i64) -> Result<Vec<UpcomingDate>> {
    let contacts = sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts.contacts
         WHERE owner_id = $1 AND is_trashed = FALSE AND is_archived = FALSE
           AND jsonb_array_length(dates) > 0",
    )
    .bind(owner_id)
    .fetch_all(db)
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
            // Next occurrence this year or next.
            let mut next = match NaiveDate::from_ymd_opt(today.year(), month, day) {
                Some(date) => date,
                None => continue, // e.g. Feb 29 in a non-leap year — skip safely
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

pub async fn trash_contact(db: &PgPool, owner_id: Uuid, contact_id: Uuid) -> Result<()> {
    let rows = sqlx::query(
        "UPDATE contacts.contacts SET is_trashed = TRUE, trashed_at = NOW()
         WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id).bind(owner_id)
    .execute(db).await.map_err(ContactsError::Database)?.rows_affected();

    if rows == 0 { return Err(ContactsError::NotFound(format!("Contact {contact_id}"))); }
    Ok(())
}

pub async fn restore_contact(db: &PgPool, owner_id: Uuid, contact_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE contacts.contacts SET is_trashed = FALSE, trashed_at = NULL
         WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id).bind(owner_id)
    .execute(db).await.map_err(ContactsError::Database)?;
    Ok(())
}

pub async fn delete_contact_permanently(
    db: &PgPool,
    owner_id: Uuid,
    contact_id: Uuid,
) -> Result<()> {
    let rows = sqlx::query(
        "DELETE FROM contacts.contacts WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id).bind(owner_id)
    .execute(db).await.map_err(ContactsError::Database)?.rows_affected();

    if rows == 0 { return Err(ContactsError::NotFound(format!("Contact {contact_id}"))); }
    Ok(())
}

pub async fn empty_trash(db: &PgPool, owner_id: Uuid) -> Result<u64> {
    let rows = sqlx::query(
        "DELETE FROM contacts.contacts WHERE owner_id = $1 AND is_trashed = TRUE",
    )
    .bind(owner_id)
    .execute(db).await.map_err(ContactsError::Database)?.rows_affected();
    Ok(rows)
}

pub async fn star_contact(db: &PgPool, owner_id: Uuid, contact_id: Uuid, starred: bool) -> Result<()> {
    sqlx::query(
        "UPDATE contacts.contacts SET is_starred = $3 WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id).bind(owner_id).bind(starred)
    .execute(db).await.map_err(ContactsError::Database)?;
    Ok(())
}

pub async fn log_interaction(
    db: &PgPool,
    contact_id: Uuid,
    owner_id: Uuid,
    interaction_type: &str,
    source_module: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO contacts.interaction_log (contact_id, owner_id, interaction_type, source_module)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(contact_id).bind(owner_id).bind(interaction_type).bind(source_module)
    .execute(db).await.context("log_interaction")?;
    Ok(())
}

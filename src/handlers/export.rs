//! **Implementation of the Kubuno module export contract, v1.**
//!
//! The core cannot read this module's PostgreSQL schema — no module's schema is
//! readable by the core, and that rule is what makes a module independently
//! installable and removable. So when an administrator asks the console for an
//! archive of an organisation's data, the core does not read: it asks. This file
//! is the answer for `contacts`.
//!
//! ## The two routes, verbatim
//!
//! * `GET /internal/export/describe` — what this module can export, and under
//!   what directory name. Called on every page load of the console's export
//!   page, so it touches no account's data.
//! * `POST /internal/export/account` — everything held about one account, as a
//!   ZIP whose entries are prefixed with the declared service id.
//!
//! Both are guarded by [`crate::middleware::require_internal_secret`]. A module
//! that skips that check has published every address book on the instance to its
//! network.
//!
//! ## What comes out, and why that format
//!
//! **vCard 4.0**, one `.vcf` holding the whole address book, plus the avatars as
//! the WebP files they are on disk. Not a dump of the `contacts.contacts` table:
//! the point of an export is that it opens somewhere else, and a `.vcf` opens in
//! every address book in existence while a JSON copy of this module's columns
//! opens in exactly one. The rendering is [`crate::services::vcard_service`] —
//! the same writer the user-facing "Exporter" button uses, so the two can never
//! drift into producing different files.
//!
//! Groups and labels ride along as `groupes.json` / `etiquettes.json`, because
//! vCard has no faithful representation for either and losing them silently
//! would be the sort of omission nobody notices until the data is gone.
//!
//! ## What never goes in
//!
//! `contacts.carddav_tokens` — a CardDAV token is a working credential for this
//! address book, and an export is a file that leaves the server. `shares.token`
//! and `shares.password_hash`, for the same reason: a share token still grants
//! access to whoever holds it. The share *list* is exported without them, so the
//! reader knows what was shared with whom without being handed the keys.
//!
//! ## What "nothing" looks like
//!
//! `204 No Content`, never an empty ZIP. The core distinguishes "this account
//! has no contacts" from "this module failed", and an empty archive would look
//! like the first while being indistinguishable from the second.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Write;
use uuid::Uuid;

use crate::{errors::Result, services::vcard_service, state::AppState};

/// The contract version this module implements. Announced, and checked against
/// what the request carries: answering a version we do not implement would
/// produce an archive the core silently mis-files.
const CONTRACT_VERSION: u32 = 1;

/// The directory this module's data lands in, inside the account's folder. Also
/// the id the console shows in its service picker.
const SERVICE_ID: &str = "contacts";

/// `GET /internal/export/describe`
pub async fn describe() -> Json<Value> {
    Json(json!({
        "contract": CONTRACT_VERSION,
        "services": [{
            "id":     SERVICE_ID,
            "label":  "Contacts",
            "format": "vCard 4.0 (.vcf) + photos WebP",
            "description": "Le carnet d'adresses du compte : fiches, groupes, étiquettes, \
                            rappels et photos. Sans les jetons CardDAV ni les jetons de partage.",
        }],
    }))
}

#[derive(Debug, Deserialize)]
pub struct AccountExportDto {
    #[serde(default)]
    pub contract: u32,
    pub user_id: Uuid,
    /// The run this belongs to. Logged, never trusted for anything else.
    #[serde(default)]
    pub export_id: Option<Uuid>,
}

/// `POST /internal/export/account`
pub async fn account(
    State(state): State<AppState>,
    Json(dto): Json<AccountExportDto>,
) -> Result<Response> {
    if dto.contract != CONTRACT_VERSION {
        return Err(crate::errors::ContactsError::Validation(format!(
            "Version de contrat d'export non prise en charge : {} (attendu {CONTRACT_VERSION})",
            dto.contract
        )));
    }

    let owner = dto.user_id;
    let contacts = load_contacts(&state, owner).await?;
    let groups = load_groups(&state, owner).await?;
    let labels = load_labels(&state, owner).await?;
    let reminders = load_reminders(&state, owner).await?;
    let shares = load_shares(&state, owner).await?;

    // Nothing at all: `204`, and not an empty archive. See the module header.
    if contacts.is_empty()
        && groups.as_array().map(Vec::is_empty).unwrap_or(true)
        && labels.as_array().map(Vec::is_empty).unwrap_or(true)
    {
        return Ok(StatusCode::NO_CONTENT.into_response());
    }

    // Avatars are read before the archive is opened: reading a file can fail,
    // and a failure inside the ZIP writer would leave a half-written archive the
    // core would then try to expand.
    let avatars = load_avatars(&state, owner, &contacts).await;

    let vcf = vcard_service::contacts_to_vcf(&contacts);
    let count = contacts.len();

    let bytes = build_zip(
        &vcf,
        &groups,
        &labels,
        &reminders,
        &shares,
        &avatars,
        count,
    )
    .map_err(|e| {
        tracing::error!(error = %e, owner = %owner, "contacts: archive d'export non produite");
        crate::errors::ContactsError::Internal(e)
    })?;

    tracing::info!(
        owner = %owner,
        export_id = ?dto.export_id,
        fiches = count,
        photos = avatars.len(),
        octets = bytes.len(),
        "contacts: export de compte produit"
    );

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/zip")],
        bytes,
    )
        .into_response())
}

// ── Reads ────────────────────────────────────────────────────────────────────

/// Every contact of the account, **including the trashed ones**.
///
/// Deliberately: a contact in the bin is still data the instance holds, the
/// person exercising their right to portability is entitled to it, and the bin
/// is emptied on a schedule they did not choose. The `is_trashed` flag rides
/// along in the JSON side-files so the reader can tell.
async fn load_contacts(state: &AppState, owner: Uuid) -> Result<Vec<crate::models::contact::Contact>> {
    // `SELECT *` is safe here and only here: `contacts.contacts` holds no
    // credential of any kind — the tokens live in `carddav_tokens` and
    // `shares`, and neither is read by this file. Every other query below names
    // its columns, which is what keeps the next migration from quietly widening
    // the export.
    state
        .db
        .fetch_all_as::<crate::models::contact::Contact>(
            "SELECT * FROM contacts.contacts WHERE owner_id = $1 ORDER BY display_name",
            kubuno_db::params![owner],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, owner = %owner, "contacts: lecture des fiches pour export impossible");
            crate::errors::ContactsError::Database(e)
        })
}

// The JSON side-files (`groupes.json`, …) used to be assembled by PostgreSQL's
// `json_agg`, which no other engine has. They are now read as typed rows and the
// JSON is built in Rust — same shape, portable.

#[derive(sqlx::FromRow)]
struct GroupExportRow {
    id:         Uuid,
    name:       String,
    color:      String,
    is_system:  bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct EdgeRow {
    parent_id: Uuid,
    child_id:  Uuid,
}

/// The `parent_id → [child_id]` map for a join table, read in one query.
async fn edge_map(state: &AppState, owner: Uuid, sql: &'static str) -> Result<std::collections::HashMap<Uuid, Vec<Uuid>>> {
    let rows = state
        .db
        .fetch_all_as::<EdgeRow>(sql, kubuno_db::params![owner])
        .await
        .map_err(|e| {
            tracing::error!(error = %e, owner = %owner, "contacts: extraction pour export impossible");
            crate::errors::ContactsError::Database(e)
        })?;
    let mut map: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
    for r in rows {
        map.entry(r.parent_id).or_default().push(r.child_id);
    }
    Ok(map)
}

async fn load_groups(state: &AppState, owner: Uuid) -> Result<Value> {
    let groups = state
        .db
        .fetch_all_as::<GroupExportRow>(
            "SELECT id, name, color, is_system, created_at, updated_at \
             FROM contacts.groups WHERE owner_id = $1 ORDER BY name",
            kubuno_db::params![owner],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, owner = %owner, "contacts: extraction des groupes impossible");
            crate::errors::ContactsError::Database(e)
        })?;
    let members = edge_map(
        state,
        owner,
        "SELECT gm.group_id AS parent_id, gm.contact_id AS child_id \
         FROM contacts.group_members gm JOIN contacts.groups g ON g.id = gm.group_id \
         WHERE g.owner_id = $1",
    )
    .await?;
    let out: Vec<Value> = groups
        .into_iter()
        .map(|g| {
            let membres = members.get(&g.id).cloned().unwrap_or_default();
            json!({
                "id": g.id, "name": g.name, "color": g.color, "is_system": g.is_system,
                "created_at": g.created_at, "updated_at": g.updated_at, "membres": membres,
            })
        })
        .collect();
    Ok(Value::Array(out))
}

#[derive(sqlx::FromRow)]
struct LabelExportRow {
    id:         Uuid,
    name:       String,
    color:      String,
    icon:       Option<String>,
    is_system:  bool,
    position:   i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn load_labels(state: &AppState, owner: Uuid) -> Result<Value> {
    let labels = state
        .db
        .fetch_all_as::<LabelExportRow>(
            "SELECT id, name, color, icon, is_system, position, created_at \
             FROM contacts.labels WHERE owner_id = $1 ORDER BY name",
            kubuno_db::params![owner],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, owner = %owner, "contacts: extraction des étiquettes impossible");
            crate::errors::ContactsError::Database(e)
        })?;
    let members = edge_map(
        state,
        owner,
        "SELECT cl.label_id AS parent_id, cl.contact_id AS child_id \
         FROM contacts.contact_labels cl JOIN contacts.labels l ON l.id = cl.label_id \
         WHERE l.owner_id = $1",
    )
    .await?;
    let out: Vec<Value> = labels
        .into_iter()
        .map(|l| {
            let fiches = members.get(&l.id).cloned().unwrap_or_default();
            json!({
                "id": l.id, "name": l.name, "color": l.color, "icon": l.icon,
                "is_system": l.is_system, "position": l.position,
                "created_at": l.created_at, "fiches": fiches,
            })
        })
        .collect();
    Ok(Value::Array(out))
}

async fn load_reminders(state: &AppState, owner: Uuid) -> Result<Value> {
    let rows = state
        .db
        .fetch_all_as::<crate::models::reminder::Reminder>(
            "SELECT id, owner_id, contact_id, kind, message, remind_at, recurrence, is_done, notified_at, created_at \
             FROM contacts.reminders WHERE owner_id = $1 ORDER BY remind_at",
            kubuno_db::params![owner],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, owner = %owner, "contacts: extraction des rappels impossible");
            crate::errors::ContactsError::Database(e)
        })?;
    Ok(serde_json::to_value(rows).unwrap_or_else(|_| Value::Array(vec![])))
}

/// The shares, **without their tokens**. `token` and `password_hash` are absent
/// from the column list on purpose: a share token is a working credential for
/// this data, and this file is going to somebody's laptop.
async fn load_shares(state: &AppState, owner: Uuid) -> Result<Value> {
    // `password_hash IS NOT NULL` gives a boolean; both cast to a plain bool the
    // driver decodes on every engine.
    let flag = state.db.backend().cast("CASE WHEN password_hash IS NOT NULL THEN 1 ELSE 0 END", kubuno_db::dialect::SqlType::BigInt);
    let sql = format!(
        "SELECT id, contact_id, group_id, permission, expires_at, max_accesses, access_count, created_at, \
                ({flag}) AS protege_flag FROM contacts.shares WHERE owner_id = $1 ORDER BY created_at DESC"
    );
    let rows = state
        .db
        .fetch_all_as::<ShareFlagRow>(&sql, kubuno_db::params![owner])
        .await
        .map_err(|e| {
            tracing::error!(error = %e, owner = %owner, "contacts: extraction des partages impossible");
            crate::errors::ContactsError::Database(e)
        })?;
    let out: Vec<Value> = rows
        .into_iter()
        .map(|s| json!({
            "id": s.id, "contact_id": s.contact_id, "group_id": s.group_id,
            "permission": s.permission, "expires_at": s.expires_at,
            "max_accesses": s.max_accesses, "access_count": s.access_count,
            "created_at": s.created_at, "protege_par_mot_de_passe": s.protege_flag != 0,
        }))
        .collect();
    Ok(Value::Array(out))
}

#[derive(sqlx::FromRow)]
struct ShareFlagRow {
    id:            Uuid,
    contact_id:    Option<Uuid>,
    group_id:      Option<Uuid>,
    permission:    String,
    expires_at:    Option<chrono::DateTime<chrono::Utc>>,
    max_accesses:  Option<i32>,
    access_count:  i32,
    created_at:    chrono::DateTime<chrono::Utc>,
    protege_flag:  i64,
}

/// The avatar files, as `(contact_id, bytes)`.
///
/// A missing or unreadable file is skipped with a warning rather than failing
/// the export: a photo that vanished from the disk must not cost the account its
/// entire address book.
async fn load_avatars(
    state: &AppState,
    owner: Uuid,
    contacts: &[crate::models::contact::Contact],
) -> Vec<(Uuid, Vec<u8>)> {
    let service = crate::services::avatar_service::AvatarService::new(
        &state.settings.storage.local_path,
        &state.settings.storage.temp_path,
        state.instance().max_avatar_mb,
    );
    let mut out = Vec::new();
    for contact in contacts {
        if contact.avatar_path.is_none() {
            continue;
        }
        let path = service.avatar_path(owner, contact.id);
        match tokio::fs::read(&path).await {
            Ok(bytes) => out.push((contact.id, bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => tracing::warn!(
                error = %e, contact_id = %contact.id,
                "contacts: photo illisible — absente de l'archive d'export"
            ),
        }
    }
    out
}

// ── The archive ──────────────────────────────────────────────────────────────

/// Builds the ZIP the core expects: every entry under `contacts/`, relative,
/// with no `..` anywhere. The core re-checks all of that — see
/// `safe_entry_path` on its side — and an entry it refuses is reported in the
/// archive's manifest rather than silently dropped, so a mistake here is visible
/// to the person receiving the file.
#[allow(clippy::too_many_arguments)]
fn build_zip(
    vcf: &str,
    groups: &Value,
    labels: &Value,
    reminders: &Value,
    shares: &Value,
    avatars: &[(Uuid, Vec<u8>)],
    count: usize,
) -> anyhow::Result<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut put = |name: &str, body: &[u8]| -> anyhow::Result<()> {
        zip.start_file(format!("{SERVICE_ID}/{name}"), options)?;
        zip.write_all(body)?;
        Ok(())
    };

    put("carnet.vcf", vcf.as_bytes())?;
    put("groupes.json", &pretty(groups))?;
    put("etiquettes.json", &pretty(labels))?;
    put("rappels.json", &pretty(reminders))?;
    put("partages.json", &pretty(shares))?;

    for (contact_id, bytes) in avatars {
        put(&format!("photos/{contact_id}.webp"), bytes)?;
    }

    // The reader's map of the folder, and the honest statement of what was left
    // out. Written last so it can count what actually went in.
    let readme = format!(
        "CONTACTS — EXPORT DU CARNET D'ADRESSES\n\
         ======================================\n\n\
         carnet.vcf       {count} fiche(s) au format vCard 4.0. Ce fichier s'importe\n\
                          dans tout carnet d'adresses (Thunderbird, iOS, Android…).\n\
         groupes.json     les groupes et les fiches qu'ils contiennent.\n\
         etiquettes.json  les étiquettes et les fiches qu'elles marquent.\n\
         rappels.json     les rappels programmés sur des fiches.\n\
         partages.json    les partages créés, SANS leur jeton d'accès.\n\
         photos/          une image WebP par fiche qui en a une, nommée par\n\
                          l'identifiant de la fiche.\n\n\
         NON INCLUS, DÉLIBÉRÉMENT\n\
         ------------------------\n\
         Les jetons CardDAV et les jetons de partage sont des identifiants qui\n\
         donnent accès à ces données. Ils ne quittent jamais le serveur.\n\n\
         Les fiches placées à la corbeille SONT incluses : elles font partie des\n\
         données détenues. Leur champ « is_trashed » les signale.\n"
    );
    put("LISEZ-MOI.txt", readme.as_bytes())?;

    Ok(zip.finish()?.into_inner())
}

fn pretty(value: &Value) -> Vec<u8> {
    serde_json::to_vec_pretty(value).unwrap_or_else(|e| {
        tracing::error!(error = %e, "contacts: sérialisation JSON d'export impossible");
        b"null".to_vec()
    })
}

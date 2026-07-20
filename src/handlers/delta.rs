//! Sync deltas for the local-first pull (contacts / labels / groups / reminders).
//! Same contract as the office sub-modules: owner-scoped changes past `cursor`
//! (monotonic change_seq), live rows + tombstones, ordered, paginated.
//! `kind ∈ modified | deleted`. Contact changes inline their `label_ids`; group
//! changes inline their `member_ids`.

use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{errors::Result, middleware::ContactsUser, state::AppState};

#[derive(serde::Deserialize)]
pub struct DeltaQuery {
    #[serde(default)]
    cursor: i64,
    limit: Option<i64>,
}

async fn rows(
    state: &AppState,
    user: Uuid,
    live: &str,
    tomb: &str,
    cursor: i64,
    limit: i64,
) -> Result<Vec<(Uuid, i64, String)>> {
    let out = sqlx::query_as::<_, (Uuid, i64, String)>(&format!(
        r#"SELECT id, change_seq, 'live' AS src FROM {live} WHERE owner_id=$1 AND change_seq>$2
           UNION ALL
           SELECT id, change_seq, 'tomb' AS src FROM {tomb} WHERE owner_id=$1 AND change_seq>$2
           ORDER BY change_seq LIMIT $3"#
    ))
    .bind(user)
    .bind(cursor)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;
    Ok(out)
}

const CONTACT_COLS: &str = "id, owner_id, given_name, middle_name, family_name, name_prefix, name_suffix, \
    nickname, display_name, organization, department, job_title, avatar_path, avatar_color, \
    emails, phones, addresses, urls, dates, relations, instant_messages, custom_fields, notes, \
    is_starred, is_trashed, trashed_at, kubuno_user_id, is_archived, archived_at, is_blocked, \
    last_interaction_at, interaction_count, pronouns, vcard_uid, etag, import_source, created_at, updated_at";

pub async fn contacts_delta(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let rows = rows(&state, user.id, "contacts.contacts", "contacts.contact_tombstones", q.cursor, limit).await?;
    let has_more = rows.len() as i64 == limit;
    let new_cursor = rows.last().map(|r| r.1).unwrap_or(q.cursor);
    let mut changes = Vec::with_capacity(rows.len());
    for (id, seq, src) in &rows {
        if src == "tomb" {
            changes.push(json!({ "uuid": id, "kind": "deleted", "change_seq": seq }));
            continue;
        }
        let contact: Option<Value> = sqlx::query_scalar(&format!(
            "SELECT to_jsonb(c) FROM (SELECT {CONTACT_COLS} FROM contacts.contacts WHERE id=$1) c"
        ))
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
        let Some(contact) = contact else { continue };
        let label_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT label_id FROM contacts.contact_labels WHERE contact_id=$1",
        )
        .bind(id)
        .fetch_all(&state.db)
        .await?;
        changes.push(json!({ "uuid": id, "kind": "modified", "change_seq": seq, "contact": contact, "label_ids": label_ids }));
    }
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}

pub async fn labels_delta(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let rows = rows(&state, user.id, "contacts.labels", "contacts.label_tombstones", q.cursor, limit).await?;
    let has_more = rows.len() as i64 == limit;
    let new_cursor = rows.last().map(|r| r.1).unwrap_or(q.cursor);
    let mut changes = Vec::with_capacity(rows.len());
    for (id, seq, src) in &rows {
        if src == "tomb" {
            changes.push(json!({ "uuid": id, "kind": "deleted", "change_seq": seq }));
            continue;
        }
        let label: Option<Value> = sqlx::query_scalar(
            "SELECT to_jsonb(l) FROM (SELECT id, owner_id, name, color, icon, is_system, position, created_at, updated_at \
             FROM contacts.labels WHERE id=$1) l",
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
        let Some(label) = label else { continue };
        changes.push(json!({ "uuid": id, "kind": "modified", "change_seq": seq, "label": label }));
    }
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}

pub async fn groups_delta(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let rows = rows(&state, user.id, "contacts.groups", "contacts.group_tombstones", q.cursor, limit).await?;
    let has_more = rows.len() as i64 == limit;
    let new_cursor = rows.last().map(|r| r.1).unwrap_or(q.cursor);
    let mut changes = Vec::with_capacity(rows.len());
    for (id, seq, src) in &rows {
        if src == "tomb" {
            changes.push(json!({ "uuid": id, "kind": "deleted", "change_seq": seq }));
            continue;
        }
        let group: Option<Value> = sqlx::query_scalar(
            "SELECT to_jsonb(g) FROM (SELECT id, owner_id, name, color, is_system, created_at, updated_at \
             FROM contacts.groups WHERE id=$1) g",
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
        let Some(group) = group else { continue };
        let member_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT contact_id FROM contacts.group_members WHERE group_id=$1",
        )
        .bind(id)
        .fetch_all(&state.db)
        .await?;
        changes.push(json!({ "uuid": id, "kind": "modified", "change_seq": seq, "group": group, "member_ids": member_ids }));
    }
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}

pub async fn reminders_delta(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let rows = rows(&state, user.id, "contacts.reminders", "contacts.reminder_tombstones", q.cursor, limit).await?;
    let has_more = rows.len() as i64 == limit;
    let new_cursor = rows.last().map(|r| r.1).unwrap_or(q.cursor);
    let mut changes = Vec::with_capacity(rows.len());
    for (id, seq, src) in &rows {
        if src == "tomb" {
            changes.push(json!({ "uuid": id, "kind": "deleted", "change_seq": seq }));
            continue;
        }
        let reminder: Option<Value> = sqlx::query_scalar(
            "SELECT to_jsonb(r) FROM (SELECT id, owner_id, contact_id, kind, message, remind_at, recurrence, \
             is_done, notified_at, created_at FROM contacts.reminders WHERE id=$1) r",
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
        let Some(reminder) = reminder else { continue };
        changes.push(json!({ "uuid": id, "kind": "modified", "change_seq": seq, "reminder": reminder }));
    }
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}

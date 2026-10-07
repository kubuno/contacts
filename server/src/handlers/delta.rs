//! Sync deltas for the local-first pull (contacts / labels / groups / reminders).
//! Owner-scoped changes past `cursor` (monotonic change_seq), live rows +
//! tombstones, ordered, paginated. `kind ∈ modified | deleted`. Contact changes
//! inline their `label_ids`; group changes inline their `member_ids`.
//!
//! The change feed comes from `kubuno_db::journal::changes_since` (the portable
//! `live UNION ALL tombstones`, replacing the PostgreSQL-only sequence/trigger
//! delta layer). Row bodies are reselected as typed structs and serialised in
//! Rust — no `to_jsonb`.

use axum::{
    extract::{Query, State},
    Extension, Json,
};
use kubuno_db::{journal, params};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::ContactsUser,
    models::{contact::Contact, group::Group, label::Label, reminder::Reminder},
    state::AppState,
    sync,
};

#[derive(serde::Deserialize)]
pub struct DeltaQuery {
    #[serde(default)]
    cursor: i64,
    limit: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct IdRow {
    id: Uuid,
}

pub async fn contacts_delta(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let feed = journal::changes_since(
        &state.db, sync::CONTACTS_TABLE, sync::CONTACT_TOMBSTONES, user.id, q.cursor, limit,
    )
    .await?;
    let has_more = feed.len() as i64 == limit;
    let new_cursor = feed.last().map(|c| c.change_seq).unwrap_or(q.cursor);

    let mut changes = Vec::with_capacity(feed.len());
    for c in &feed {
        if c.deleted {
            changes.push(json!({ "uuid": c.id, "kind": "deleted", "change_seq": c.change_seq }));
            continue;
        }
        let contact = state
            .db
            .fetch_optional_as::<Contact>(
                "SELECT * FROM contacts.contacts WHERE id = $1",
                params![c.id],
            )
            .await?;
        let Some(contact) = contact else { continue };
        let label_ids: Vec<Uuid> = state
            .db
            .fetch_all_as::<LabelIdRow>(
                "SELECT label_id FROM contacts.contact_labels WHERE contact_id = $1",
                params![c.id],
            )
            .await?
            .into_iter()
            .map(|r| r.label_id)
            .collect();
        changes.push(json!({
            "uuid": c.id, "kind": "modified", "change_seq": c.change_seq,
            "contact": contact, "label_ids": label_ids,
        }));
    }
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}

#[derive(sqlx::FromRow)]
struct LabelIdRow {
    label_id: Uuid,
}

pub async fn labels_delta(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let feed = journal::changes_since(
        &state.db, sync::LABELS_TABLE, sync::LABEL_TOMBSTONES, user.id, q.cursor, limit,
    )
    .await?;
    let has_more = feed.len() as i64 == limit;
    let new_cursor = feed.last().map(|c| c.change_seq).unwrap_or(q.cursor);

    let mut changes = Vec::with_capacity(feed.len());
    for c in &feed {
        if c.deleted {
            changes.push(json!({ "uuid": c.id, "kind": "deleted", "change_seq": c.change_seq }));
            continue;
        }
        let label = state
            .db
            .fetch_optional_as::<Label>(
                "SELECT * FROM contacts.labels WHERE id = $1",
                params![c.id],
            )
            .await?;
        let Some(label) = label else { continue };
        changes.push(json!({ "uuid": c.id, "kind": "modified", "change_seq": c.change_seq, "label": label }));
    }
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}

pub async fn groups_delta(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let feed = journal::changes_since(
        &state.db, sync::GROUPS_TABLE, sync::GROUP_TOMBSTONES, user.id, q.cursor, limit,
    )
    .await?;
    let has_more = feed.len() as i64 == limit;
    let new_cursor = feed.last().map(|c| c.change_seq).unwrap_or(q.cursor);

    let mut changes = Vec::with_capacity(feed.len());
    for c in &feed {
        if c.deleted {
            changes.push(json!({ "uuid": c.id, "kind": "deleted", "change_seq": c.change_seq }));
            continue;
        }
        let group = state
            .db
            .fetch_optional_as::<Group>(
                "SELECT * FROM contacts.groups WHERE id = $1",
                params![c.id],
            )
            .await?;
        let Some(group) = group else { continue };
        let member_ids: Vec<Uuid> = state
            .db
            .fetch_all_as::<IdRow>(
                "SELECT contact_id AS id FROM contacts.group_members WHERE group_id = $1",
                params![c.id],
            )
            .await?
            .into_iter()
            .map(|r| r.id)
            .collect();
        changes.push(json!({
            "uuid": c.id, "kind": "modified", "change_seq": c.change_seq,
            "group": group, "member_ids": member_ids,
        }));
    }
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}

pub async fn reminders_delta(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let feed = journal::changes_since(
        &state.db, sync::REMINDERS_TABLE, sync::REMINDER_TOMBSTONES, user.id, q.cursor, limit,
    )
    .await?;
    let has_more = feed.len() as i64 == limit;
    let new_cursor = feed.last().map(|c| c.change_seq).unwrap_or(q.cursor);

    let mut changes = Vec::with_capacity(feed.len());
    for c in &feed {
        if c.deleted {
            changes.push(json!({ "uuid": c.id, "kind": "deleted", "change_seq": c.change_seq }));
            continue;
        }
        let reminder = state
            .db
            .fetch_optional_as::<Reminder>(
                "SELECT * FROM contacts.reminders WHERE id = $1",
                params![c.id],
            )
            .await?;
        let Some(reminder) = reminder else { continue };
        changes.push(json!({ "uuid": c.id, "kind": "modified", "change_seq": c.change_seq, "reminder": reminder }));
    }
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}

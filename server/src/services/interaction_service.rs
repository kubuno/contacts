use chrono::{DateTime, Duration, Utc};
use kubuno_db::{new_id, params, DbPool};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::contact::{Contact, ContactWithLabels},
    services::contact_service,
    sync,
};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Interaction {
    pub id:               Uuid,
    pub contact_id:       Uuid,
    pub interaction_type: String,
    pub summary:          Option<String>,
    pub source_module:    Option<String>,
    pub occurred_at:      DateTime<Utc>,
}

pub async fn list_for_contact(
    db: &DbPool,
    owner_id: Uuid,
    contact_id: Uuid,
    limit: i64,
) -> Result<Vec<Interaction>> {
    db.fetch_all_as::<Interaction>(
        "SELECT id, contact_id, interaction_type, summary, source_module, occurred_at \
         FROM contacts.interaction_log \
         WHERE contact_id = $1 AND owner_id = $2 \
         ORDER BY occurred_at DESC \
         LIMIT $3",
        params![contact_id, owner_id, limit.clamp(1, 500)],
    )
    .await
    .map_err(ContactsError::Database)
}

/// Records an interaction and bumps the contact's denormalised counters and its
/// change_seq (so the updated counters re-sync).
pub async fn record(
    db: &DbPool,
    owner_id: Uuid,
    contact_id: Uuid,
    interaction_type: &str,
    summary: Option<&str>,
    source_module: Option<&str>,
    source_id: Option<Uuid>,
) -> Result<()> {
    let now = Utc::now();
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;

    tx.execute(
        "INSERT INTO contacts.interaction_log \
         (id, contact_id, owner_id, interaction_type, summary, source_module, source_id, occurred_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        params![new_id(), contact_id, owner_id, interaction_type, summary, source_module, source_id, now],
    )
    .await
    .map_err(ContactsError::Database)?;

    let seq = sync::next_contact_seq(&mut tx).await.map_err(ContactsError::Database)?;
    tx.execute(
        "UPDATE contacts.contacts \
         SET last_interaction_at = $1, interaction_count = interaction_count + 1, change_seq = $2, updated_at = $3 \
         WHERE id = $4 AND owner_id = $5",
        params![now, seq, now, contact_id, owner_id],
    )
    .await
    .map_err(ContactsError::Database)?;

    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(())
}

/// Most-contacted people (by recorded interaction count).
pub async fn frequent(db: &DbPool, owner_id: Uuid, limit: i64) -> Result<Vec<ContactWithLabels>> {
    let rows = db
        .fetch_all_as::<Contact>(
            "SELECT * FROM contacts.contacts \
             WHERE owner_id = $1 AND is_trashed = FALSE AND is_archived = FALSE \
               AND interaction_count > 0 \
             ORDER BY interaction_count DESC, (last_interaction_at IS NULL), last_interaction_at DESC \
             LIMIT $2",
            params![owner_id, limit.clamp(1, 100)],
        )
        .await
        .map_err(ContactsError::Database)?;
    contact_service::decorate_with_labels(db, rows).await
}

/// Recently interacted-with people.
pub async fn recent(db: &DbPool, owner_id: Uuid, limit: i64) -> Result<Vec<ContactWithLabels>> {
    let rows = db
        .fetch_all_as::<Contact>(
            "SELECT * FROM contacts.contacts \
             WHERE owner_id = $1 AND is_trashed = FALSE AND is_archived = FALSE \
               AND last_interaction_at IS NOT NULL \
             ORDER BY last_interaction_at DESC \
             LIMIT $2",
            params![owner_id, limit.clamp(1, 100)],
        )
        .await
        .map_err(ContactsError::Database)?;
    contact_service::decorate_with_labels(db, rows).await
}

/// People not interacted with for `days` days (or never), most stale first.
/// Only starred contacts are considered.
pub async fn to_follow_up(
    db: &DbPool,
    owner_id: Uuid,
    days: i64,
    limit: i64,
) -> Result<Vec<ContactWithLabels>> {
    // The cutoff is computed in Rust (portable — no `make_interval`).
    let cutoff = Utc::now() - Duration::days(days.clamp(1, 3650));
    let rows = db
        .fetch_all_as::<Contact>(
            "SELECT * FROM contacts.contacts \
             WHERE owner_id = $1 AND is_trashed = FALSE AND is_archived = FALSE AND is_starred = TRUE \
               AND (last_interaction_at IS NULL OR last_interaction_at < $2) \
             ORDER BY (last_interaction_at IS NOT NULL), last_interaction_at ASC \
             LIMIT $3",
            params![owner_id, cutoff, limit.clamp(1, 100)],
        )
        .await
        .map_err(ContactsError::Database)?;
    contact_service::decorate_with_labels(db, rows).await
}

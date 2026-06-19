use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::contact::{Contact, ContactWithLabels},
    services::contact_service,
};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Interaction {
    pub id:               Uuid,
    pub contact_id:       Uuid,
    pub interaction_type: String,
    pub summary:          Option<String>,
    pub source_module:    Option<String>,
    pub occurred_at:      DateTime<Utc>,
}

pub async fn list_for_contact(
    db: &PgPool,
    owner_id: Uuid,
    contact_id: Uuid,
    limit: i64,
) -> Result<Vec<Interaction>> {
    sqlx::query_as::<_, Interaction>(
        "SELECT id, contact_id, interaction_type, summary, source_module, occurred_at
         FROM contacts.interaction_log
         WHERE contact_id = $1 AND owner_id = $2
         ORDER BY occurred_at DESC
         LIMIT $3",
    )
    .bind(contact_id)
    .bind(owner_id)
    .bind(limit.clamp(1, 500))
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)
}

/// Records an interaction and bumps the contact's denormalised counters. Used
/// both by the manual UI and by the cross-module event handler.
pub async fn record(
    db: &PgPool,
    owner_id: Uuid,
    contact_id: Uuid,
    interaction_type: &str,
    summary: Option<&str>,
    source_module: Option<&str>,
    source_id: Option<Uuid>,
) -> Result<()> {
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;

    sqlx::query(
        "INSERT INTO contacts.interaction_log
         (contact_id, owner_id, interaction_type, summary, source_module, source_id)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(contact_id)
    .bind(owner_id)
    .bind(interaction_type)
    .bind(summary)
    .bind(source_module)
    .bind(source_id)
    .execute(&mut *tx)
    .await
    .map_err(ContactsError::Database)?;

    sqlx::query(
        "UPDATE contacts.contacts
         SET last_interaction_at = NOW(), interaction_count = interaction_count + 1
         WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id)
    .bind(owner_id)
    .execute(&mut *tx)
    .await
    .map_err(ContactsError::Database)?;

    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(())
}

/// Most-contacted people (by recorded interaction count).
pub async fn frequent(db: &PgPool, owner_id: Uuid, limit: i64) -> Result<Vec<ContactWithLabels>> {
    let rows = sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts.contacts
         WHERE owner_id = $1 AND is_trashed = FALSE AND is_archived = FALSE
           AND interaction_count > 0
         ORDER BY interaction_count DESC, last_interaction_at DESC NULLS LAST
         LIMIT $2",
    )
    .bind(owner_id)
    .bind(limit.clamp(1, 100))
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)?;
    contact_service::decorate_with_labels(db, rows).await
}

/// Recently interacted-with people.
pub async fn recent(db: &PgPool, owner_id: Uuid, limit: i64) -> Result<Vec<ContactWithLabels>> {
    let rows = sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts.contacts
         WHERE owner_id = $1 AND is_trashed = FALSE AND is_archived = FALSE
           AND last_interaction_at IS NOT NULL
         ORDER BY last_interaction_at DESC
         LIMIT $2",
    )
    .bind(owner_id)
    .bind(limit.clamp(1, 100))
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)?;
    contact_service::decorate_with_labels(db, rows).await
}

/// People not interacted with for `days` days (or never), most stale first.
/// Only starred contacts are considered, to avoid noise — these are the people
/// the user cares about and hasn't reached out to in a while.
pub async fn to_follow_up(
    db: &PgPool,
    owner_id: Uuid,
    days: i64,
    limit: i64,
) -> Result<Vec<ContactWithLabels>> {
    let rows = sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts.contacts
         WHERE owner_id = $1 AND is_trashed = FALSE AND is_archived = FALSE AND is_starred = TRUE
           AND (last_interaction_at IS NULL OR last_interaction_at < NOW() - make_interval(days => $2::int))
         ORDER BY last_interaction_at ASC NULLS FIRST
         LIMIT $3",
    )
    .bind(owner_id)
    .bind(days.clamp(1, 3650) as i32)
    .bind(limit.clamp(1, 100))
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)?;
    contact_service::decorate_with_labels(db, rows).await
}

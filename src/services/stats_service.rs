use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::{ContactsError, Result};

#[derive(Debug, Serialize)]
pub struct Stats {
    pub total:       i64,
    pub starred:     i64,
    pub archived:    i64,
    pub trashed:     i64,
    pub blocked:     i64,
    pub groups:      i64,
    pub labels:      i64,
    pub with_email:  i64,
    pub with_phone:  i64,
    pub with_avatar: i64,
    pub incomplete:  i64,
    pub completeness_pct: i64,
}

pub async fn compute(db: &PgPool, owner_id: Uuid) -> Result<Stats> {
    // A single scan computes every counter via conditional aggregates.
    let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, i64, i64)>(
        "SELECT
            COUNT(*) FILTER (WHERE is_trashed = FALSE AND is_archived = FALSE)              AS total,
            COUNT(*) FILTER (WHERE is_trashed = FALSE AND is_starred = TRUE)                AS starred,
            COUNT(*) FILTER (WHERE is_archived = TRUE AND is_trashed = FALSE)               AS archived,
            COUNT(*) FILTER (WHERE is_trashed = TRUE)                                       AS trashed,
            COUNT(*) FILTER (WHERE is_blocked = TRUE AND is_trashed = FALSE)                AS blocked,
            COUNT(*) FILTER (WHERE is_trashed = FALSE AND jsonb_array_length(emails) > 0)   AS with_email,
            COUNT(*) FILTER (WHERE is_trashed = FALSE AND jsonb_array_length(phones) > 0)   AS with_phone,
            COUNT(*) FILTER (WHERE is_trashed = FALSE AND avatar_path IS NOT NULL)          AS with_avatar
         FROM contacts.contacts
         WHERE owner_id = $1",
    )
    .bind(owner_id)
    .fetch_one(db)
    .await
    .map_err(ContactsError::Database)?;

    let groups = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM contacts.groups WHERE owner_id = $1")
        .bind(owner_id).fetch_one(db).await.map_err(ContactsError::Database)?;
    let labels = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM contacts.labels WHERE owner_id = $1")
        .bind(owner_id).fetch_one(db).await.map_err(ContactsError::Database)?;

    let (total, starred, archived, trashed, blocked, with_email, with_phone, with_avatar) = row;
    let incomplete = (total - with_email).max(0) + (total - with_phone).max(0);
    // Completeness: average of the email/phone/avatar coverage ratios.
    let completeness_pct = if total > 0 {
        ((with_email + with_phone + with_avatar) * 100 / (total * 3)).clamp(0, 100)
    } else {
        0
    };

    Ok(Stats {
        total, starred, archived, trashed, blocked, groups, labels,
        with_email, with_phone, with_avatar,
        incomplete,
        completeness_pct,
    })
}

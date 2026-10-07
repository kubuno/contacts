use kubuno_db::{params, DbPool};
use serde::Serialize;
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

#[derive(sqlx::FromRow)]
struct StatsRow {
    total:       i64,
    starred:     i64,
    archived:    i64,
    trashed:     i64,
    blocked:     i64,
    with_email:  i64,
    with_phone:  i64,
    with_avatar: i64,
}

pub async fn compute(db: &DbPool, owner_id: Uuid) -> Result<Stats> {
    let b = db.backend();
    // `COUNT(*) FILTER (WHERE …)` is PostgreSQL-only; the portable form is a
    // `SUM(CASE WHEN … THEN 1 ELSE 0 END)` cast to BIGINT (via `sum_bigint`).
    let sql = format!(
        "SELECT \
            {total}       AS total, \
            {starred}     AS starred, \
            {archived}    AS archived, \
            {trashed}     AS trashed, \
            {blocked}     AS blocked, \
            {with_email}  AS with_email, \
            {with_phone}  AS with_phone, \
            {with_avatar} AS with_avatar \
         FROM contacts.contacts WHERE owner_id = $1",
        total       = b.sum_bigint("CASE WHEN is_trashed = FALSE AND is_archived = FALSE THEN 1 ELSE 0 END"),
        starred     = b.sum_bigint("CASE WHEN is_trashed = FALSE AND is_starred = TRUE THEN 1 ELSE 0 END"),
        archived    = b.sum_bigint("CASE WHEN is_archived = TRUE AND is_trashed = FALSE THEN 1 ELSE 0 END"),
        trashed     = b.sum_bigint("CASE WHEN is_trashed = TRUE THEN 1 ELSE 0 END"),
        blocked     = b.sum_bigint("CASE WHEN is_blocked = TRUE AND is_trashed = FALSE THEN 1 ELSE 0 END"),
        with_email  = b.sum_bigint("CASE WHEN is_trashed = FALSE AND email_norm <> '' THEN 1 ELSE 0 END"),
        with_phone  = b.sum_bigint("CASE WHEN is_trashed = FALSE AND phone_norm <> '' THEN 1 ELSE 0 END"),
        with_avatar = b.sum_bigint("CASE WHEN is_trashed = FALSE AND avatar_path IS NOT NULL THEN 1 ELSE 0 END"),
    );
    let row = db
        .fetch_one_as::<StatsRow>(&sql, params![owner_id])
        .await
        .map_err(ContactsError::Database)?;

    let groups = db
        .fetch_scalar::<i64>(
            &format!("SELECT {} FROM contacts.groups WHERE owner_id = $1", b.count_bigint("*")),
            params![owner_id],
        )
        .await
        .map_err(ContactsError::Database)?;
    let labels = db
        .fetch_scalar::<i64>(
            &format!("SELECT {} FROM contacts.labels WHERE owner_id = $1", b.count_bigint("*")),
            params![owner_id],
        )
        .await
        .map_err(ContactsError::Database)?;

    let StatsRow { total, starred, archived, trashed, blocked, with_email, with_phone, with_avatar } = row;
    let incomplete = (total - with_email).max(0) + (total - with_phone).max(0);
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

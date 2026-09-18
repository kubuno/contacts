use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::{ContactsError, Result};

/// One remembered interlocutor, as the list view shows it.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OtherContact {
    pub id:            Uuid,
    pub kind:          String,
    pub value:         String,
    pub display_name:  Option<String>,
    pub source_module: String,
    pub first_seen_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at:  chrono::DateTime<chrono::Utc>,
    pub seen_count:    i32,
}

/// What a module reports after dealing with someone.
#[derive(Debug, Deserialize)]
pub struct SeenInterlocutor {
    pub owner_id:     Uuid,
    pub kind:         String,
    pub value:        String,
    pub display_name: Option<String>,
}

/// Lower-cased and trimmed, so the same person reported by two modules — or with
/// a different capitalisation — lands on the same row.
fn normalise(kind: &str, value: &str) -> String {
    let v = value.trim();
    if kind == "email" { v.to_lowercase() } else { v.to_string() }
}

/// Records interlocutors seen by a module. Upserts: an interlocutor already
/// known only has their counters bumped, and a name is filled in when we did not
/// have one (a module that only knows an address never erases a better name).
///
/// Returns how many rows were written. Never fails the caller's own work: the
/// endpoint above it answers 200 even when nothing was usable.
pub async fn record_seen(db: &PgPool, module: &str, seen: &[SeenInterlocutor]) -> Result<usize> {
    let mut written = 0usize;
    for s in seen {
        let value = normalise(&s.kind, &s.value);
        if value.is_empty() || s.kind.trim().is_empty() {
            continue;
        }
        let name = s.display_name.as_deref().map(str::trim).filter(|n| !n.is_empty());

        let res = sqlx::query(
            r#"
            INSERT INTO contacts.other_contacts
                   (owner_id, kind, value, display_name, source_module)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (owner_id, kind, value) DO UPDATE
               SET last_seen_at  = NOW(),
                   seen_count    = contacts.other_contacts.seen_count + 1,
                   display_name  = COALESCE(contacts.other_contacts.display_name, EXCLUDED.display_name),
                   source_module = EXCLUDED.source_module
            "#,
        )
        .bind(s.owner_id)
        .bind(&s.kind)
        .bind(&value)
        .bind(name)
        .bind(module)
        .execute(db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, owner = %s.owner_id, module, "other_contacts: enregistrement impossible");
            ContactsError::Database(e)
        })?;
        written += res.rows_affected() as usize;
    }
    Ok(written)
}

/// The suggestions worth showing: never dismissed, and never someone already in
/// the address book. The exclusion is done in SQL against the contact's own
/// e-mails and phones, because the two sets drift constantly — a contact saved
/// this morning must disappear from here without anyone rewriting a row.
pub async fn list(db: &PgPool, owner: Uuid, limit: i64) -> Result<Vec<OtherContact>> {
    sqlx::query_as::<_, OtherContact>(
        r#"
        SELECT o.id, o.kind, o.value, o.display_name, o.source_module,
               o.first_seen_at, o.last_seen_at, o.seen_count
          FROM contacts.other_contacts AS o
         WHERE o.owner_id = $1
           AND o.dismissed_at IS NULL
           AND NOT EXISTS (
                 SELECT 1
                   FROM contacts.contacts AS c,
                        LATERAL jsonb_array_elements(
                          CASE WHEN o.kind = 'phone' THEN c.phones ELSE c.emails END
                        ) AS f(item)
                  WHERE c.owner_id = o.owner_id
                    AND c.is_trashed = FALSE
                    AND lower(trim(f.item ->> 'value')) = o.value
               )
         ORDER BY o.last_seen_at DESC
         LIMIT $2
        "#,
    )
    .bind(owner)
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, %owner, "other_contacts: lecture impossible");
        ContactsError::Database(e)
    })
}

/// "Not interesting": keep the row so the next message does not bring the
/// suggestion back, but never list it again.
pub async fn dismiss(db: &PgPool, owner: Uuid, id: Uuid) -> Result<()> {
    let res = sqlx::query(
        "UPDATE contacts.other_contacts SET dismissed_at = NOW() WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner)
    .execute(db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, %owner, %id, "other_contacts: rejet impossible");
        ContactsError::Database(e)
    })?;
    if res.rows_affected() == 0 {
        return Err(ContactsError::NotFound("other contact".into()));
    }
    Ok(())
}

/// Reads one suggestion, to build a real contact out of it.
pub async fn get(db: &PgPool, owner: Uuid, id: Uuid) -> Result<OtherContact> {
    sqlx::query_as::<_, OtherContact>(
        r#"SELECT id, kind, value, display_name, source_module,
                  first_seen_at, last_seen_at, seen_count
             FROM contacts.other_contacts
            WHERE id = $1 AND owner_id = $2"#,
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, %owner, %id, "other_contacts: lecture unitaire impossible");
        ContactsError::Database(e)
    })?
    .ok_or_else(|| ContactsError::NotFound("other contact".into()))
}

/// Once promoted into the address book the suggestion has no reason to exist:
/// the list query would hide it anyway, and keeping it would resurrect it if the
/// contact were later deleted.
pub async fn forget(db: &PgPool, owner: Uuid, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM contacts.other_contacts WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(owner)
        .execute(db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %owner, %id, "other_contacts: suppression impossible");
            ContactsError::Database(e)
        })?;
    Ok(())
}

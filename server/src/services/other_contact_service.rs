use kubuno_db::dialect::Assign;
use kubuno_db::{new_id, params, DbPool};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::contact::Contact,
};

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

/// Records interlocutors seen by a module. Upserts on (owner, kind, value); an
/// interlocutor already known only has their counters bumped, and a name is
/// filled in when we did not have one.
pub async fn record_seen(db: &DbPool, module: &str, seen: &[SeenInterlocutor]) -> Result<usize> {
    let backend = db.backend();
    let clause = backend.upsert(
        "contacts.other_contacts",
        &["owner_id", "kind", "value"],
        &[
            Assign::Incoming("last_seen_at"),
            Assign::Expr { col: "seen_count", expr: "{cur} + 1" },
            Assign::Expr { col: "display_name", expr: "COALESCE({cur}, {new})" },
            Assign::Incoming("source_module"),
        ],
    );
    let sql = format!(
        "INSERT INTO contacts.other_contacts \
             (id, owner_id, kind, value, display_name, source_module, first_seen_at, last_seen_at, seen_count) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1){clause}"
    );

    let mut written = 0usize;
    for s in seen {
        let value = normalise(&s.kind, &s.value);
        if value.is_empty() || s.kind.trim().is_empty() {
            continue;
        }
        let name = s.display_name.as_deref().map(str::trim).filter(|n| !n.is_empty());
        let now = chrono::Utc::now();

        let res = db
            .execute(
                &sql,
                params![new_id(), s.owner_id, s.kind.as_str(), value, name, module, now, now],
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, owner = %s.owner_id, module, "other_contacts: enregistrement impossible");
                ContactsError::Database(e)
            })?;
        written += res as usize;
    }
    Ok(written)
}

/// The suggestions worth showing: never dismissed, and never someone already in
/// the address book. The address-book exclusion is done in Rust (the contact's
/// e-mails/phones are JSON arrays; walking them in SQL portably is not worth a
/// per-engine `LATERAL`).
pub async fn list(db: &DbPool, owner: Uuid, limit: i64) -> Result<Vec<OtherContact>> {
    // Candidates: this owner's non-dismissed suggestions, freshest first.
    let candidates = db
        .fetch_all_as::<OtherContact>(
            "SELECT id, kind, value, display_name, source_module, first_seen_at, last_seen_at, seen_count \
             FROM contacts.other_contacts \
             WHERE owner_id = $1 AND dismissed_at IS NULL \
             ORDER BY last_seen_at DESC",
            params![owner],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %owner, "other_contacts: lecture impossible");
            ContactsError::Database(e)
        })?;

    // The owner's saved contacts, so an interlocutor already in the address book
    // drops out of the suggestions.
    let contacts = db
        .fetch_all_as::<Contact>(
            "SELECT * FROM contacts.contacts WHERE owner_id = $1 AND is_trashed = FALSE",
            params![owner],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %owner, "other_contacts: lecture des fiches impossible");
            ContactsError::Database(e)
        })?;

    let mut emails = std::collections::HashSet::<String>::new();
    let mut phones = std::collections::HashSet::<String>::new();
    for c in &contacts {
        for e in c.emails.0.iter() {
            emails.insert(e.value.trim().to_lowercase());
        }
        for p in c.phones.0.iter() {
            phones.insert(p.value.trim().to_lowercase());
        }
    }

    let out: Vec<OtherContact> = candidates
        .into_iter()
        .filter(|o| {
            let known = if o.kind == "phone" {
                phones.contains(&o.value)
            } else {
                emails.contains(&o.value)
            };
            !known
        })
        .take(limit.max(0) as usize)
        .collect();
    Ok(out)
}

/// "Not interesting": keep the row so the next message does not bring the
/// suggestion back, but never list it again.
pub async fn dismiss(db: &DbPool, owner: Uuid, id: Uuid) -> Result<()> {
    let rows = db
        .execute(
            "UPDATE contacts.other_contacts SET dismissed_at = $1 WHERE id = $2 AND owner_id = $3",
            params![chrono::Utc::now(), id, owner],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %owner, %id, "other_contacts: rejet impossible");
            ContactsError::Database(e)
        })?;
    if rows == 0 {
        return Err(ContactsError::NotFound("other contact".into()));
    }
    Ok(())
}

/// Reads one suggestion, to build a real contact out of it.
pub async fn get(db: &DbPool, owner: Uuid, id: Uuid) -> Result<OtherContact> {
    db.fetch_optional_as::<OtherContact>(
        "SELECT id, kind, value, display_name, source_module, first_seen_at, last_seen_at, seen_count \
         FROM contacts.other_contacts WHERE id = $1 AND owner_id = $2",
        params![id, owner],
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, %owner, %id, "other_contacts: lecture unitaire impossible");
        ContactsError::Database(e)
    })?
    .ok_or_else(|| ContactsError::NotFound("other contact".into()))
}

/// Once promoted into the address book the suggestion has no reason to exist.
pub async fn forget(db: &DbPool, owner: Uuid, id: Uuid) -> Result<()> {
    db.execute(
        "DELETE FROM contacts.other_contacts WHERE id = $1 AND owner_id = $2",
        params![id, owner],
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, %owner, %id, "other_contacts: suppression impossible");
        ContactsError::Database(e)
    })?;
    Ok(())
}

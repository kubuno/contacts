use kubuno_db::dialect::Assign;
use kubuno_db::{params, DbPool};
use serde_json::Value;
use uuid::Uuid;

use crate::errors::{ContactsError, Result};

/// The stored preferences blob, decoded through `#[sqlx(json)]` so it reads back
/// from PostgreSQL JSONB, MySQL JSON and a SQLite TEXT column alike.
#[derive(sqlx::FromRow)]
struct PrefsRow {
    #[sqlx(json)]
    prefs: Value,
}

pub async fn get_settings(db: &DbPool, owner_id: Uuid) -> Result<Value> {
    let row = db
        .fetch_optional_as::<PrefsRow>(
            "SELECT prefs FROM contacts.user_settings WHERE owner_id = $1",
            params![owner_id],
        )
        .await
        .map_err(ContactsError::Database)?;
    Ok(row.map(|r| r.prefs).unwrap_or_else(|| Value::Object(Default::default())))
}

/// Shallow-merges `patch` into the stored preferences and persists the result.
pub async fn update_settings(db: &DbPool, owner_id: Uuid, patch: Value) -> Result<Value> {
    let mut current = get_settings(db, owner_id).await?;
    if let (Value::Object(cur), Value::Object(p)) = (&mut current, &patch) {
        for (k, v) in p {
            cur.insert(k.clone(), v.clone());
        }
    } else if patch.is_object() {
        current = patch;
    }

    let backend = db.backend();
    let clause = backend.upsert(
        "contacts.user_settings",
        &["owner_id"],
        &[Assign::Incoming("prefs"), Assign::Incoming("updated_at")],
    );
    let sql = format!(
        "INSERT INTO contacts.user_settings (owner_id, prefs, updated_at) VALUES ($1, $2, $3){clause}"
    );
    db.execute(&sql, params![owner_id, current.clone(), chrono::Utc::now()])
        .await
        .map_err(ContactsError::Database)?;
    Ok(current)
}

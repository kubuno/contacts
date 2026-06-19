use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::{ContactsError, Result};

pub async fn get_settings(db: &PgPool, owner_id: Uuid) -> Result<Value> {
    let prefs = sqlx::query_scalar::<_, Value>(
        "SELECT prefs FROM contacts.user_settings WHERE owner_id = $1",
    )
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .map_err(ContactsError::Database)?;
    Ok(prefs.unwrap_or_else(|| Value::Object(Default::default())))
}

/// Shallow-merges `patch` into the stored preferences and persists the result.
pub async fn update_settings(db: &PgPool, owner_id: Uuid, patch: Value) -> Result<Value> {
    let mut current = get_settings(db, owner_id).await?;
    if let (Value::Object(cur), Value::Object(p)) = (&mut current, &patch) {
        for (k, v) in p {
            cur.insert(k.clone(), v.clone());
        }
    } else if patch.is_object() {
        current = patch;
    }

    sqlx::query(
        "INSERT INTO contacts.user_settings (owner_id, prefs, updated_at)
         VALUES ($1, $2, NOW())
         ON CONFLICT (owner_id) DO UPDATE SET prefs = EXCLUDED.prefs, updated_at = NOW()",
    )
    .bind(owner_id)
    .bind(&current)
    .execute(db)
    .await
    .map_err(ContactsError::Database)?;
    Ok(current)
}

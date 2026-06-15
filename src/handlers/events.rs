use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{errors::Result, state::AppState};

#[derive(Deserialize)]
pub struct KubunoEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload:    serde_json::Value,
}

/// Called by the core when a subscribed event fires.
pub async fn handle_event(
    State(state): State<AppState>,
    Json(event): Json<KubunoEvent>,
) -> Result<Json<Value>> {
    match event.event_type.as_str() {
        "UserCreated" | "UserUpdated" => {
            if let (Some(user_id), Some(email), Some(display_name)) = (
                event.payload.get("user_id").and_then(|v| v.as_str()).and_then(|s| s.parse::<uuid::Uuid>().ok()),
                event.payload.get("email").and_then(|v| v.as_str()),
                event.payload.get("display_name").and_then(|v| v.as_str()),
            ) {
                sqlx::query(
                    "INSERT INTO contacts.directory_profiles (kubuno_user_id, display_name, email)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (kubuno_user_id) DO UPDATE SET
                     display_name = EXCLUDED.display_name,
                     email        = EXCLUDED.email,
                     updated_at   = NOW()",
                )
                .bind(user_id).bind(display_name).bind(email)
                .execute(&state.db).await?;
            }
        }
        "UserDeleted" => {
            if let Some(user_id) = event.payload.get("user_id").and_then(|v| v.as_str()).and_then(|s| s.parse::<uuid::Uuid>().ok()) {
                sqlx::query(
                    "DELETE FROM contacts.directory_profiles WHERE kubuno_user_id = $1",
                )
                .bind(user_id)
                .execute(&state.db).await?;
            }
        }
        _ => {}
    }

    Ok(Json(json!({ "ok": true })))
}

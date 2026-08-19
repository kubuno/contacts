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
///
/// The module used to mirror `UserCreated`/`UserUpdated`/`UserDeleted` into a
/// local `directory_profiles` table and serve the staff directory from it. That
/// mirror bypassed the instance sharing policy (`directory.enabled` /
/// `share_email` / `audience`), so it was removed: the directory is now read
/// straight from the core's governed endpoints. Account events therefore need no
/// local bookkeeping and are acknowledged without side effects.
pub async fn handle_event(
    State(_state): State<AppState>,
    Json(_event): Json<KubunoEvent>,
) -> Result<Json<Value>> {
    Ok(Json(json!({ "ok": true })))
}

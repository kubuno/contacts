//! The instance policy, as the browser is allowed to see it.
//!
//! The interface uses it to leave out what the server would refuse anyway — a
//! share button when public links are off, a CardDAV panel when the endpoint is
//! closed. It is a courtesy, never the control: every flag returned here is
//! enforced again in the handler that owns the action.

use axum::{extract::State, Json};
use serde_json::Value;

use crate::{errors::Result, state::AppState};

/// GET /config — public flags of the instance settings.
pub async fn get_config(State(state): State<AppState>) -> Result<Json<Value>> {
    Ok(Json(state.instance().public_flags()))
}

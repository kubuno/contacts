use axum::{extract::State, Extension, Json};
use serde_json::{json, Value};

use crate::{
    errors::Result,
    middleware::ContactsUser,
    services::{settings_service, stats_service},
    state::AppState,
};

pub async fn get(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
) -> Result<Json<Value>> {
    let prefs = settings_service::get_settings(&state.db, user.id).await?;
    Ok(Json(json!({ "settings": prefs })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>> {
    let prefs = settings_service::update_settings(&state.db, user.id, patch).await?;
    Ok(Json(json!({ "settings": prefs })))
}

pub async fn stats(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
) -> Result<Json<Value>> {
    let s = stats_service::compute(&state.db, user.id).await?;
    Ok(Json(json!({ "stats": s })))
}

use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::ContactsUser,
    services::interaction_service,
    state::AppState,
};

pub async fn list_for_contact(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let items = interaction_service::list_for_contact(&state.db, user.id, id, 100).await?;
    Ok(Json(json!({ "interactions": items })))
}

#[derive(Deserialize)]
pub struct AddInteractionDto {
    pub interaction_type: String,
    pub summary:          Option<String>,
}

pub async fn add(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<AddInteractionDto>,
) -> Result<Json<Value>> {
    interaction_service::record(
        &state.db,
        user.id,
        id,
        &dto.interaction_type,
        dto.summary.as_deref(),
        Some("manual"),
        None,
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
    pub days:  Option<i64>,
}

pub async fn frequent(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Value>> {
    let contacts = interaction_service::frequent(&state.db, user.id, q.limit.unwrap_or(12)).await?;
    Ok(Json(json!({ "contacts": contacts })))
}

pub async fn recent(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Value>> {
    let contacts = interaction_service::recent(&state.db, user.id, q.limit.unwrap_or(12)).await?;
    Ok(Json(json!({ "contacts": contacts })))
}

pub async fn to_follow_up(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Value>> {
    let contacts =
        interaction_service::to_follow_up(&state.db, user.id, q.days.unwrap_or(90), q.limit.unwrap_or(20)).await?;
    Ok(Json(json!({ "contacts": contacts })))
}

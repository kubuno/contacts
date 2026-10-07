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
    models::reminder::{CreateReminderDto, UpdateReminderDto},
    services::reminder_service,
    state::AppState,
};

#[derive(Deserialize)]
pub struct ListQuery {
    pub include_done: Option<bool>,
}

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>> {
    let reminders = reminder_service::list_reminders(&state.db, user.id, q.include_done.unwrap_or(false)).await?;
    let due = reminder_service::due_count(&state.db, user.id).await?;
    Ok(Json(json!({ "reminders": reminders, "due_count": due })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Json(dto): Json<CreateReminderDto>,
) -> Result<Json<Value>> {
    let reminder = reminder_service::create_reminder(&state.db, user.id, &dto).await?;
    Ok(Json(json!({ "reminder": reminder })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateReminderDto>,
) -> Result<Json<Value>> {
    let reminder = reminder_service::update_reminder(&state.db, user.id, id, &dto).await?;
    Ok(Json(json!({ "reminder": reminder })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    reminder_service::delete_reminder(&state.db, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

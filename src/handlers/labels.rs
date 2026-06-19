use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::ContactsUser,
    models::label::{CreateLabelDto, LabelMembersDto, UpdateLabelDto},
    services::label_service,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
) -> Result<Json<Value>> {
    let labels = label_service::list_labels(&state.db, user.id).await?;
    Ok(Json(json!({ "labels": labels })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Json(dto): Json<CreateLabelDto>,
) -> Result<Json<Value>> {
    let label = label_service::create_label(&state.db, user.id, &dto).await?;
    Ok(Json(json!({ "label": label })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateLabelDto>,
) -> Result<Json<Value>> {
    let label = label_service::update_label(&state.db, user.id, id, &dto).await?;
    Ok(Json(json!({ "label": label })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    label_service::delete_label(&state.db, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn add_members(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<LabelMembersDto>,
) -> Result<Json<Value>> {
    let added = label_service::add_label_to_contacts(&state.db, user.id, id, &dto.contact_ids).await?;
    Ok(Json(json!({ "added": added })))
}

pub async fn remove_members(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<LabelMembersDto>,
) -> Result<Json<Value>> {
    let removed = label_service::remove_label_from_contacts(&state.db, user.id, id, &dto.contact_ids).await?;
    Ok(Json(json!({ "removed": removed })))
}

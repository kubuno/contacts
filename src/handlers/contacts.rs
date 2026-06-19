use axum::{
    extract::{Multipart, Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    middleware::ContactsUser,
    models::contact::{CreateContactDto, ListContactsParams, UpdateContactDto},
    services::{avatar_service::AvatarService, contact_service, core_client},
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(params): Query<ListContactsParams>,
) -> Result<Json<Value>> {
    let result = contact_service::list_contacts(&state.db, user.id, &params).await?;
    Ok(Json(json!({ "contacts": result.contacts, "total": result.total })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Json(dto): Json<CreateContactDto>,
) -> Result<Json<Value>> {
    let contact = contact_service::create_contact(&state.db, user.id, &dto).await?;
    core_client::publish(&state.settings, core_client::contact_lifecycle("ContactCreated", contact.id, user.id)).await;
    Ok(Json(json!({ "contact": contact })))
}

pub async fn get(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let contact = contact_service::get_contact(&state.db, user.id, id).await?;
    let label_ids = crate::services::label_service::labels_for_contact(&state.db, id).await?;
    Ok(Json(json!({ "contact": contact, "label_ids": label_ids })))
}

pub async fn archive(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    contact_service::set_archived(&state.db, user.id, id, true).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn unarchive(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    contact_service::set_archived(&state.db, user.id, id, false).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn block(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    contact_service::set_blocked(&state.db, user.id, id, true).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn unblock(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    contact_service::set_blocked(&state.db, user.id, id, false).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateContactDto>,
) -> Result<Json<Value>> {
    let contact = contact_service::update_contact(&state.db, user.id, id, &dto).await?;
    core_client::publish(&state.settings, core_client::contact_updated(contact.id, user.id)).await;
    Ok(Json(json!({ "contact": contact })))
}

pub async fn trash(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    contact_service::trash_contact(&state.db, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn restore(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    contact_service::restore_contact(&state.db, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_permanently(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    contact_service::delete_contact_permanently(&state.db, user.id, id).await?;
    core_client::publish(&state.settings, core_client::contact_lifecycle("ContactDeleted", id, user.id)).await;
    Ok(Json(json!({ "ok": true })))
}

pub async fn empty_trash(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
) -> Result<Json<Value>> {
    let count = contact_service::empty_trash(&state.db, user.id).await?;
    Ok(Json(json!({ "deleted": count })))
}

pub async fn star(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    contact_service::star_contact(&state.db, user.id, id, true).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn unstar(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    contact_service::star_contact(&state.db, user.id, id, false).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn duplicates(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
) -> Result<Json<Value>> {
    let groups = contact_service::find_duplicates(&state.db, user.id).await?;
    Ok(Json(json!({ "groups": groups })))
}

#[derive(serde::Deserialize)]
pub struct MergeDto {
    pub primary_id:    Uuid,
    pub duplicate_ids: Vec<Uuid>,
}

pub async fn merge(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Json(dto): Json<MergeDto>,
) -> Result<Json<Value>> {
    let contact = contact_service::merge_contacts(&state.db, user.id, dto.primary_id, &dto.duplicate_ids).await?;
    Ok(Json(json!({ "contact": contact })))
}

#[derive(serde::Deserialize)]
pub struct IgnorePairDto {
    pub contact_a: Uuid,
    pub contact_b: Uuid,
}

pub async fn ignore_duplicate(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Json(dto): Json<IgnorePairDto>,
) -> Result<Json<Value>> {
    contact_service::ignore_duplicate_pair(&state.db, user.id, dto.contact_a, dto.contact_b).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn history(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let entries = contact_service::get_history(&state.db, user.id, id).await?;
    Ok(Json(json!({ "history": entries })))
}

#[derive(serde::Deserialize)]
pub struct BirthdaysQuery {
    pub days: Option<i64>,
}

pub async fn birthdays(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<BirthdaysQuery>,
) -> Result<Json<Value>> {
    let dates = contact_service::upcoming_dates(&state.db, user.id, q.days.unwrap_or(365).clamp(1, 366)).await?;
    Ok(Json(json!({ "dates": dates })))
}

pub async fn upload_avatar(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<Value>> {
    let svc = AvatarService::new(
        &state.settings.storage.local_path,
        &state.settings.storage.temp_path,
        state.settings.contacts.max_avatar_mb,
    );

    let mut saved_path = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name().unwrap_or("") == "avatar" {
            let data = field.bytes().await.map_err(|e| ContactsError::Validation(e.to_string()))?;
            let path = svc.save_avatar(user.id, id, data).await
                .map_err(|e| ContactsError::Validation(e.to_string()))?;
            saved_path = Some(path);
            break;
        }
    }

    let avatar_path = saved_path.ok_or_else(|| ContactsError::Validation("Champ 'avatar' manquant".into()))?;

    sqlx::query(
        "UPDATE contacts.contacts SET avatar_path = $1 WHERE id = $2 AND owner_id = $3",
    )
    .bind(&avatar_path).bind(id).bind(user.id)
    .execute(&state.db).await.map_err(ContactsError::Database)?;

    Ok(Json(json!({ "avatar_path": avatar_path })))
}

pub async fn get_avatar(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    let row = sqlx::query_scalar::<_, Option<String>>(
        "SELECT avatar_path FROM contacts.contacts WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db).await.map_err(ContactsError::Database)?
    .flatten();

    let path = row.ok_or_else(|| ContactsError::NotFound("Avatar introuvable".into()))?;

    let svc = AvatarService::new(
        &state.settings.storage.local_path,
        &state.settings.storage.temp_path,
        state.settings.contacts.max_avatar_mb,
    );
    let data = svc.read_avatar(&path).await
        .map_err(|e| ContactsError::NotFound(e.to_string()))?;

    Ok((
        [(header::CONTENT_TYPE, "image/webp"),
         (header::CACHE_CONTROL, "public, max-age=86400")],
        data,
    ).into_response())
}

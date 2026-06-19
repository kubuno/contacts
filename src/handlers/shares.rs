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
    models::share::CreateShareDto,
    services::share_service,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
) -> Result<Json<Value>> {
    let shares = share_service::list_shares(&state.db, user.id).await?;
    Ok(Json(json!({ "shares": shares })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Json(dto): Json<CreateShareDto>,
) -> Result<Json<Value>> {
    let share = share_service::create_share(&state.db, user.id, &dto).await?;
    Ok(Json(json!({ "share": share })))
}

pub async fn revoke(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    share_service::revoke_share(&state.db, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct PublicQuery {
    pub password: Option<String>,
}

/// Public, unauthenticated endpoint resolving a share token.
pub async fn public_view(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(q): Query<PublicQuery>,
) -> Result<Json<Value>> {
    let payload = share_service::resolve_share(&state.db, &token, q.password.as_deref()).await?;
    Ok(Json(json!({ "kind": payload.kind, "contacts": payload.contacts })))
}

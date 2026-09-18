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
    models::contact::CreateContactDto,
    services::{contact_service, other_contact_service as svc},
    state::AppState,
};

#[derive(Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
}

/// The "Other contacts" view: people met through other modules and never saved.
pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let items = svc::list(&state.db, user.id, limit).await?;
    Ok(Json(json!({ "other_contacts": items })))
}

/// "Not interesting" — the suggestion stops being shown, and seeing the person
/// again will not bring it back.
pub async fn dismiss(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    svc::dismiss(&state.db, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Promotes a suggestion into a real contact, then forgets it: from now on the
/// person lives in the address book like any other.
pub async fn save(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let other = svc::get(&state.db, user.id, id).await?;

    let mut dto = CreateContactDto::default();
    dto.given_name = other.display_name.clone();
    match other.kind.as_str() {
        "phone" => dto.phones = vec![crate::models::contact::ContactField {
            label: None, field_type: "mobile".into(), value: other.value.clone(),
        }],
        // An address is the default: a chat handle has nowhere better to go and
        // is at least searchable there.
        _ => dto.emails = vec![crate::models::contact::ContactField {
            label: None, field_type: "other".into(), value: other.value.clone(),
        }],
    }

    let contact = contact_service::create_contact(&state.db, user.id, &dto).await?;
    svc::forget(&state.db, user.id, id).await?;
    Ok(Json(json!({ "contact": contact })))
}

#[derive(Deserialize)]
pub struct SeenBody {
    /// Which module saw them — recorded so the list can say where a name is from.
    pub module: String,
    pub seen:   Vec<svc::SeenInterlocutor>,
}

/// Module → contacts, behind `X-Internal-Secret`.
///
/// Modules that deal with interlocutors (mail, chat…) report whom the user
/// actually exchanged with. It is deliberately fire-and-forget: the caller's own
/// work (delivering a message) must never fail because an address book is busy,
/// so anything unusable is skipped rather than rejected.
pub async fn record_seen(
    State(state): State<AppState>,
    Json(body): Json<SeenBody>,
) -> Result<Json<Value>> {
    let written = svc::record_seen(&state.db, &body.module, &body.seen).await?;
    Ok(Json(json!({ "ok": true, "recorded": written })))
}

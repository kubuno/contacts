use axum::{extract::State, Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    middleware::ContactsUser,
    services::contact_service::{self, BulkAction},
    state::AppState,
};

#[derive(Deserialize)]
pub struct BulkDto {
    pub ids:    Vec<Uuid>,
    pub action: String,
}

/// POST /contacts/bulk — applies one action to many contacts at once.
pub async fn bulk(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Json(dto): Json<BulkDto>,
) -> Result<Json<Value>> {
    let action = match dto.action.as_str() {
        "trash"      => BulkAction::Trash,
        "restore"    => BulkAction::Restore,
        "delete"     => BulkAction::DeletePermanently,
        "star"       => BulkAction::Star,
        "unstar"     => BulkAction::Unstar,
        "archive"    => BulkAction::Archive,
        "unarchive"  => BulkAction::Unarchive,
        "block"      => BulkAction::Block,
        "unblock"    => BulkAction::Unblock,
        other => return Err(ContactsError::Validation(format!("Action inconnue: {other}"))),
    };
    let affected = contact_service::bulk_action(&state.db, user.id, &dto.ids, action).await?;
    Ok(Json(json!({ "affected": affected })))
}

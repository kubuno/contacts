use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    middleware::ContactsUser,
    state::AppState,
};

/// One account as the core's governed directory answers it.
///
/// Same projection every module gets from the core; a contacts module never
/// keeps its own copy of the account list.
#[derive(Deserialize)]
struct DirectoryUser {
    display_name: String,
    #[serde(default)]
    email:        String,
}

/// Add a directory member to the caller's personal contacts.
///
/// The person is resolved from the CORE, never from a local mirror. The core is
/// the single source of truth for accounts, so the identity written here is the
/// authoritative one — a client cannot make us store a name that does not match
/// the id it points at.
///
/// Browsing the directory itself is done by the frontend against the core's
/// authenticated `/users/search`, which resolves the instance sharing policy
/// (`directory.enabled` / `share_email` / `audience`) for the caller. A module
/// has no caller once the proxy has stripped `Authorization`, so it must not,
/// and here does not, try to reproduce that policy.
pub async fn add_to_contacts(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(kubuno_user_id): Path<Uuid>,
) -> Result<Json<Value>> {
    let url = format!(
        "{}/internal/directory/users/{}",
        state.settings.core.url, kubuno_user_id
    );
    let resp = state
        .http
        .get(&url)
        .header("X-Internal-Secret", state.settings.core.internal_secret.as_str())
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %kubuno_user_id, "annuaire : le core est injoignable");
            ContactsError::Internal(anyhow::anyhow!("core directory unreachable"))
        })?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(ContactsError::NotFound("Profil introuvable".into()));
    }
    if !resp.status().is_success() {
        tracing::error!(status = %resp.status(), %kubuno_user_id, "annuaire : le core a refusé la résolution");
        return Err(ContactsError::Internal(anyhow::anyhow!("core directory error")));
    }

    let profile: DirectoryUser = resp.json().await.map_err(|e| {
        tracing::error!(error = %e, "annuaire : réponse du core illisible");
        ContactsError::Internal(anyhow::anyhow!("core directory decode"))
    })?;

    let emails = if profile.email.is_empty() {
        vec![]
    } else {
        vec![crate::models::contact::ContactField {
            label: None,
            value: profile.email,
            field_type: "work".into(),
        }]
    };

    let dto = crate::models::contact::CreateContactDto {
        id: None,
        given_name: None, middle_name: None, family_name: None,
        name_prefix: None, name_suffix: None, nickname: None,
        display_name: Some(profile.display_name), organization: None,
        department: None, job_title: None, avatar_color: None, pronouns: None,
        emails, phones: vec![], addresses: vec![], urls: vec![],
        dates: vec![], relations: vec![], instant_messages: vec![],
        custom_fields: vec![], notes: None, is_starred: None,
    };

    let contact = crate::services::contact_service::create_contact(&state.db, user.id, &dto).await?;

    // Link the personal contact to the account it was created from, so presence
    // and future lookups can find it again.
    sqlx::query("UPDATE contacts.contacts SET kubuno_user_id = $2 WHERE id = $1")
        .bind(contact.id)
        .bind(kubuno_user_id)
        .execute(&state.db)
        .await
        .map_err(ContactsError::Database)?;

    Ok(Json(json!({ "contact": contact })))
}

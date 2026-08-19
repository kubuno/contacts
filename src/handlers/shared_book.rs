//! The instance's shared address book.
//!
//! Read by everyone, written by administrators only. The read is additionally
//! gated by the instance setting `shared_book_enabled`, so an administrator can
//! prepare the book before publishing it — writes keep working while it is
//! unpublished, reads do not.
//!
//! This is NOT the account directory: accounts live in `core.users` under the
//! core's `directory.*` policy. Everything here is a person without an account.

use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    middleware::ContactsUser,
    models::shared_contact::{CreateSharedContactDto, SharedContact, UpdateSharedContactDto},
    state::AppState,
};

#[derive(Deserialize)]
pub struct ListParams {
    pub q: Option<String>,
}

/// Refuses anyone but an administrator of the instance. The role is the one the
/// core put in `X-Kubuno-User-Role`; the module never derives it itself.
fn assert_admin(user: &ContactsUser) -> Result<()> {
    if user.role == "admin" {
        Ok(())
    } else {
        Err(ContactsError::Forbidden)
    }
}

/// An empty string means "clear this field"; `None` means "leave it alone".
fn blank_to_none(v: Option<String>) -> Option<Option<String>> {
    v.map(|s| {
        let t = s.trim().to_string();
        if t.is_empty() { None } else { Some(t) }
    })
}

/// GET /shared-book — the published book, or the whole of it for an admin.
pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>> {
    let enabled = state.instance().shared_book_enabled;

    // Unpublished: users see nothing, administrators keep seeing it so they can
    // fill it in before turning it on.
    if !enabled && user.role != "admin" {
        return Ok(Json(json!({ "contacts": [], "enabled": false })));
    }

    let q = params.q.map(|s| s.trim().to_lowercase()).unwrap_or_default();
    let rows: Vec<SharedContact> = sqlx::query_as(
        "SELECT * FROM contacts.shared_contacts
         WHERE $1 = ''
            OR LOWER(display_name) LIKE '%' || $1 || '%'
            OR LOWER(COALESCE(organization, '')) LIKE '%' || $1 || '%'
            OR LOWER(COALESCE(email, ''))        LIKE '%' || $1 || '%'
         ORDER BY display_name
         LIMIT 500",
    )
    .bind(&q)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Lecture du carnet partagé");
        ContactsError::Database(e)
    })?;

    Ok(Json(json!({ "contacts": rows, "enabled": enabled })))
}

/// POST /shared-book — add an entry (administrators only).
pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Json(dto): Json<CreateSharedContactDto>,
) -> Result<Json<Value>> {
    assert_admin(&user)?;

    let name = dto.display_name.trim();
    if name.is_empty() {
        return Err(ContactsError::Validation("Le nom est obligatoire".into()));
    }
    if name.chars().count() > 500 {
        return Err(ContactsError::Validation("Nom trop long (500 caractères maximum)".into()));
    }

    let contact: SharedContact = sqlx::query_as(
        "INSERT INTO contacts.shared_contacts
         (display_name, organization, job_title, email, phone, notes, updated_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING *",
    )
    .bind(name)
    .bind(blank_to_none(dto.organization).flatten())
    .bind(blank_to_none(dto.job_title).flatten())
    .bind(blank_to_none(dto.email).flatten())
    .bind(blank_to_none(dto.phone).flatten())
    .bind(blank_to_none(dto.notes).flatten())
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Création dans le carnet partagé");
        ContactsError::Database(e)
    })?;

    Ok(Json(json!({ "contact": contact })))
}

/// PATCH /shared-book/:id — edit an entry (administrators only).
pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateSharedContactDto>,
) -> Result<Json<Value>> {
    assert_admin(&user)?;

    if let Some(name) = dto.display_name.as_deref() {
        if name.trim().is_empty() {
            return Err(ContactsError::Validation("Le nom est obligatoire".into()));
        }
        if name.chars().count() > 500 {
            return Err(ContactsError::Validation("Nom trop long (500 caractères maximum)".into()));
        }
    }

    // COALESCE keeps an absent field untouched; the `..._set` flags are what let
    // an explicitly emptied field be cleared rather than read as "untouched".
    let contact: Option<SharedContact> = sqlx::query_as(
        "UPDATE contacts.shared_contacts SET
            display_name = COALESCE($2, display_name),
            organization = CASE WHEN $3 THEN $4 ELSE organization END,
            job_title    = CASE WHEN $5 THEN $6 ELSE job_title    END,
            email        = CASE WHEN $7 THEN $8 ELSE email        END,
            phone        = CASE WHEN $9 THEN $10 ELSE phone       END,
            notes        = CASE WHEN $11 THEN $12 ELSE notes      END,
            updated_by   = $13
         WHERE id = $1
         RETURNING *",
    )
    .bind(id)
    .bind(dto.display_name.as_deref().map(str::trim))
    .bind(dto.organization.is_some())
    .bind(blank_to_none(dto.organization).flatten())
    .bind(dto.job_title.is_some())
    .bind(blank_to_none(dto.job_title).flatten())
    .bind(dto.email.is_some())
    .bind(blank_to_none(dto.email).flatten())
    .bind(dto.phone.is_some())
    .bind(blank_to_none(dto.phone).flatten())
    .bind(dto.notes.is_some())
    .bind(blank_to_none(dto.notes).flatten())
    .bind(user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Modification dans le carnet partagé");
        ContactsError::Database(e)
    })?;

    contact
        .map(|c| Json(json!({ "contact": c })))
        .ok_or_else(|| ContactsError::NotFound(format!("Contact partagé {id}")))
}

/// DELETE /shared-book/:id — remove an entry (administrators only).
pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    assert_admin(&user)?;

    let rows = sqlx::query("DELETE FROM contacts.shared_contacts WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Suppression dans le carnet partagé");
            ContactsError::Database(e)
        })?
        .rows_affected();

    if rows == 0 {
        return Err(ContactsError::NotFound(format!("Contact partagé {id}")));
    }
    Ok(Json(json!({ "ok": true })))
}

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
use kubuno_db::params;
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
    // The search pattern is built in Rust (no `'%'||$1||'%'` — `||` is logical
    // OR on MySQL) and bound once per column (a placeholder may not be reused).
    let rows: Vec<SharedContact> = if q.is_empty() {
        state
            .db
            .fetch_all_as::<SharedContact>(
                "SELECT * FROM contacts.shared_contacts ORDER BY display_name LIMIT 500",
                params![],
            )
            .await
    } else {
        let pat = format!("%{q}%");
        state
            .db
            .fetch_all_as::<SharedContact>(
                "SELECT * FROM contacts.shared_contacts \
                 WHERE LOWER(display_name) LIKE $1 \
                    OR LOWER(COALESCE(organization, '')) LIKE $2 \
                    OR LOWER(COALESCE(email, '')) LIKE $3 \
                 ORDER BY display_name LIMIT 500",
                params![pat.clone(), pat.clone(), pat],
            )
            .await
    }
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

    let id = kubuno_db::new_id();
    let now = chrono::Utc::now();
    state
        .db
        .execute(
            "INSERT INTO contacts.shared_contacts \
             (id, display_name, organization, job_title, email, phone, notes, updated_by, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            params![
                id, name,
                blank_to_none(dto.organization).flatten(),
                blank_to_none(dto.job_title).flatten(),
                blank_to_none(dto.email).flatten(),
                blank_to_none(dto.phone).flatten(),
                blank_to_none(dto.notes).flatten(),
                user.id, now, now,
            ],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Création dans le carnet partagé");
            ContactsError::Database(e)
        })?;

    let contact = state
        .db
        .fetch_optional_as::<SharedContact>(
            "SELECT * FROM contacts.shared_contacts WHERE id = $1",
            params![id],
        )
        .await
        .map_err(ContactsError::Database)?
        .ok_or_else(|| ContactsError::NotFound("Contact partagé".into()))?;

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
    // MySQL has no RETURNING, so this updates then reselects.
    let now = chrono::Utc::now();
    let rows = state
        .db
        .execute(
            "UPDATE contacts.shared_contacts SET \
                display_name = COALESCE($1, display_name), \
                organization = CASE WHEN $2 THEN $3 ELSE organization END, \
                job_title    = CASE WHEN $4 THEN $5 ELSE job_title    END, \
                email        = CASE WHEN $6 THEN $7 ELSE email        END, \
                phone        = CASE WHEN $8 THEN $9 ELSE phone       END, \
                notes        = CASE WHEN $10 THEN $11 ELSE notes      END, \
                updated_by   = $12, updated_at = $13 \
             WHERE id = $14",
            params![
                dto.display_name.as_deref().map(str::trim),
                dto.organization.is_some(),
                blank_to_none(dto.organization).flatten(),
                dto.job_title.is_some(),
                blank_to_none(dto.job_title).flatten(),
                dto.email.is_some(),
                blank_to_none(dto.email).flatten(),
                dto.phone.is_some(),
                blank_to_none(dto.phone).flatten(),
                dto.notes.is_some(),
                blank_to_none(dto.notes).flatten(),
                user.id, now, id,
            ],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Modification dans le carnet partagé");
            ContactsError::Database(e)
        })?;

    if rows == 0 {
        return Err(ContactsError::NotFound(format!("Contact partagé {id}")));
    }
    let contact = state
        .db
        .fetch_optional_as::<SharedContact>(
            "SELECT * FROM contacts.shared_contacts WHERE id = $1",
            params![id],
        )
        .await
        .map_err(ContactsError::Database)?
        .ok_or_else(|| ContactsError::NotFound(format!("Contact partagé {id}")))?;
    Ok(Json(json!({ "contact": contact })))
}

/// DELETE /shared-book/:id — remove an entry (administrators only).
pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    assert_admin(&user)?;

    let rows = state
        .db
        .execute("DELETE FROM contacts.shared_contacts WHERE id = $1", params![id])
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Suppression dans le carnet partagé");
            ContactsError::Database(e)
        })?;

    if rows == 0 {
        return Err(ContactsError::NotFound(format!("Contact partagé {id}")));
    }
    Ok(Json(json!({ "ok": true })))
}

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
    models::group::{CreateGroupDto, UpdateGroupDto},
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
) -> Result<Json<Value>> {
    let groups = sqlx::query_as::<_, (Uuid, Uuid, String, String, bool, i64, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
        "SELECT g.id, g.owner_id, g.name, g.color, g.is_system,
                COUNT(gm.contact_id)::bigint as contact_count,
                g.created_at, g.updated_at
         FROM contacts.groups g
         LEFT JOIN contacts.group_members gm ON gm.group_id = g.id
         WHERE g.owner_id = $1
         GROUP BY g.id ORDER BY g.name ASC",
    )
    .bind(user.id)
    .fetch_all(&state.db).await.map_err(ContactsError::Database)?;

    let out: Vec<Value> = groups.into_iter().map(|(id, owner_id, name, color, is_system, contact_count, created_at, updated_at)| json!({
        "id": id, "owner_id": owner_id, "name": name, "color": color,
        "is_system": is_system, "contact_count": contact_count,
        "created_at": created_at, "updated_at": updated_at,
    })).collect();

    Ok(Json(json!({ "groups": out })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Json(dto): Json<CreateGroupDto>,
) -> Result<Json<Value>> {
    if dto.name.trim().is_empty() {
        return Err(ContactsError::Validation("Le nom du groupe est requis".into()));
    }

    let group = sqlx::query_as::<_, crate::models::group::Group>(
        "INSERT INTO contacts.groups (owner_id, name, color) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(user.id)
    .bind(dto.name.trim())
    .bind(dto.color.as_deref().unwrap_or("#1a73e8"))
    .fetch_one(&state.db).await
    .map_err(|e| match e {
        sqlx::Error::Database(ref d) if d.constraint() == Some("groups_owner_id_name_key") =>
            ContactsError::Conflict(format!("Un groupe '{}' existe déjà", dto.name)),
        _ => ContactsError::Database(e),
    })?;

    Ok(Json(json!({ "group": group })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateGroupDto>,
) -> Result<Json<Value>> {
    let existing = sqlx::query_as::<_, crate::models::group::Group>(
        "SELECT * FROM contacts.groups WHERE id = $1 AND owner_id = $2",
    )
    .bind(id).bind(user.id)
    .fetch_optional(&state.db).await.map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound(format!("Groupe {id}")))?;

    if existing.is_system {
        return Err(ContactsError::Forbidden);
    }

    let name  = dto.name.as_deref().unwrap_or(&existing.name);
    let color = dto.color.as_deref().unwrap_or(&existing.color);

    let group = sqlx::query_as::<_, crate::models::group::Group>(
        "UPDATE contacts.groups SET name = $3, color = $4 WHERE id = $1 AND owner_id = $2 RETURNING *",
    )
    .bind(id).bind(user.id).bind(name).bind(color)
    .fetch_one(&state.db).await.map_err(ContactsError::Database)?;

    Ok(Json(json!({ "group": group })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let group = sqlx::query_as::<_, crate::models::group::Group>(
        "SELECT * FROM contacts.groups WHERE id = $1 AND owner_id = $2",
    )
    .bind(id).bind(user.id)
    .fetch_optional(&state.db).await.map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound(format!("Groupe {id}")))?;

    if group.is_system {
        return Err(ContactsError::Forbidden);
    }

    sqlx::query("DELETE FROM contacts.groups WHERE id = $1 AND owner_id = $2")
        .bind(id).bind(user.id)
        .execute(&state.db).await.map_err(ContactsError::Database)?;

    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct MembersDto {
    pub contact_ids: Vec<Uuid>,
}

pub async fn add_members(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<MembersDto>,
) -> Result<Json<Value>> {
    // Verify group ownership
    sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM contacts.groups WHERE id = $1 AND owner_id = $2",
    )
    .bind(id).bind(user.id)
    .fetch_optional(&state.db).await.map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound(format!("Groupe {id}")))?;

    for contact_id in &dto.contact_ids {
        sqlx::query(
            "INSERT INTO contacts.group_members (group_id, contact_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(id).bind(contact_id)
        .execute(&state.db).await.map_err(ContactsError::Database)?;
    }

    Ok(Json(json!({ "ok": true })))
}

pub async fn remove_member(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path((id, contact_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM contacts.groups WHERE id = $1 AND owner_id = $2",
    )
    .bind(id).bind(user.id)
    .fetch_optional(&state.db).await.map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound(format!("Groupe {id}")))?;

    sqlx::query(
        "DELETE FROM contacts.group_members WHERE group_id = $1 AND contact_id = $2",
    )
    .bind(id).bind(contact_id)
    .execute(&state.db).await.map_err(ContactsError::Database)?;

    Ok(Json(json!({ "ok": true })))
}

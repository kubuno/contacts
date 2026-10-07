use axum::{
    extract::{Path, State},
    Extension, Json,
};
use kubuno_db::{new_id, params};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    middleware::ContactsUser,
    models::group::{CreateGroupDto, Group, UpdateGroupDto},
    state::AppState,
    sync,
};

/// One row of the group list, with its member count.
#[derive(sqlx::FromRow)]
struct GroupCountRow {
    id:            Uuid,
    owner_id:      Uuid,
    name:          String,
    color:         String,
    is_system:     bool,
    contact_count: i64,
    created_at:    chrono::DateTime<chrono::Utc>,
    updated_at:    chrono::DateTime<chrono::Utc>,
}

async fn select_group(state: &AppState, owner_id: Uuid, id: Uuid) -> Result<Group> {
    state
        .db
        .fetch_optional_as::<Group>(
            "SELECT * FROM contacts.groups WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await
        .map_err(ContactsError::Database)?
        .ok_or_else(|| ContactsError::NotFound(format!("Groupe {id}")))
}

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
) -> Result<Json<Value>> {
    let sql = format!(
        "SELECT g.id, g.owner_id, g.name, g.color, g.is_system, \
                {} AS contact_count, g.created_at, g.updated_at \
         FROM contacts.groups g \
         LEFT JOIN contacts.group_members gm ON gm.group_id = g.id \
         WHERE g.owner_id = $1 \
         GROUP BY g.id, g.owner_id, g.name, g.color, g.is_system, g.created_at, g.updated_at \
         ORDER BY g.name ASC",
        state.db.backend().count_bigint("gm.contact_id"),
    );
    let groups = state
        .db
        .fetch_all_as::<GroupCountRow>(&sql, params![user.id])
        .await
        .map_err(ContactsError::Database)?;

    let out: Vec<Value> = groups
        .into_iter()
        .map(|g| json!({
            "id": g.id, "owner_id": g.owner_id, "name": g.name, "color": g.color,
            "is_system": g.is_system, "contact_count": g.contact_count,
            "created_at": g.created_at, "updated_at": g.updated_at,
        }))
        .collect();

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
    let id = dto.id.unwrap_or_else(new_id);
    let now = chrono::Utc::now();

    let mut tx = state.db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_group_seq(&mut tx).await.map_err(ContactsError::Database)?;
    tx.execute(
        "INSERT INTO contacts.groups (id, owner_id, name, color, change_seq, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        params![id, user.id, dto.name.trim(), dto.color.as_deref().unwrap_or("#1a73e8"), seq, now, now],
    )
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref d) if d.is_unique_violation() =>
            ContactsError::Conflict(format!("Un groupe '{}' existe déjà", dto.name)),
        _ => ContactsError::Database(e),
    })?;
    tx.commit().await.map_err(ContactsError::Database)?;

    let group = select_group(&state, user.id, id).await?;
    Ok(Json(json!({ "group": group })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateGroupDto>,
) -> Result<Json<Value>> {
    let existing = select_group(&state, user.id, id).await?;
    if existing.is_system {
        return Err(ContactsError::Forbidden);
    }

    let name  = dto.name.as_deref().unwrap_or(&existing.name);
    let color = dto.color.as_deref().unwrap_or(&existing.color);
    let now = chrono::Utc::now();

    let mut tx = state.db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_group_seq(&mut tx).await.map_err(ContactsError::Database)?;
    tx.execute(
        "UPDATE contacts.groups SET name = $1, color = $2, change_seq = $3, updated_at = $4 \
         WHERE id = $5 AND owner_id = $6",
        params![name, color, seq, now, id, user.id],
    )
    .await
    .map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;

    let group = select_group(&state, user.id, id).await?;
    Ok(Json(json!({ "group": group })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let group = select_group(&state, user.id, id).await?;
    if group.is_system {
        return Err(ContactsError::Forbidden);
    }

    let mut tx = state.db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_group_seq(&mut tx).await.map_err(ContactsError::Database)?;
    tx.execute(
        "DELETE FROM contacts.groups WHERE id = $1 AND owner_id = $2",
        params![id, user.id],
    )
    .await
    .map_err(ContactsError::Database)?;
    sync::record_group_tombstone(&mut tx, id, user.id, seq).await.map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;

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
    // Verify group ownership.
    select_group(&state, user.id, id).await?;

    let backend = state.db.backend();
    let now = chrono::Utc::now();
    let mut tx = state.db.begin().await.map_err(ContactsError::Database)?;
    let mut changed = false;
    for contact_id in &dto.contact_ids {
        let sql = format!(
            "INSERT {}INTO contacts.group_members (group_id, contact_id, added_at) VALUES ($1, $2, $3){}",
            backend.insert_ignore_prefix(),
            backend.on_conflict_do_nothing(&["group_id", "contact_id"]),
        );
        let rows = tx
            .execute(&sql, params![id, contact_id, now])
            .await
            .map_err(ContactsError::Database)?;
        changed = changed || rows > 0;
    }
    // A member change bumps the group so its inline member_ids re-sync.
    if changed {
        sync::touch_group(&mut tx, id).await.map_err(ContactsError::Database)?;
    }
    tx.commit().await.map_err(ContactsError::Database)?;

    Ok(Json(json!({ "ok": true })))
}

pub async fn remove_member(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path((id, contact_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    select_group(&state, user.id, id).await?;

    let mut tx = state.db.begin().await.map_err(ContactsError::Database)?;
    let rows = tx
        .execute(
            "DELETE FROM contacts.group_members WHERE group_id = $1 AND contact_id = $2",
            params![id, contact_id],
        )
        .await
        .map_err(ContactsError::Database)?;
    if rows > 0 {
        sync::touch_group(&mut tx, id).await.map_err(ContactsError::Database)?;
    }
    tx.commit().await.map_err(ContactsError::Database)?;

    Ok(Json(json!({ "ok": true })))
}

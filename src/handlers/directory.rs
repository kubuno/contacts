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
    state::AppState,
};

#[derive(Deserialize)]
pub struct DirectoryParams {
    pub q:      Option<String>,
    pub limit:  Option<i64>,
    pub offset: Option<i64>,
}

pub async fn search(
    State(state): State<AppState>,
    Extension(_user): Extension<ContactsUser>,
    Query(params): Query<DirectoryParams>,
) -> Result<Json<Value>> {
    let limit  = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    let rows = if let Some(q) = &params.q {
        sqlx::query_as::<_, (Uuid, String, String, Option<String>, Option<String>, Option<String>, Option<String>)>(
            "SELECT kubuno_user_id, display_name, email, avatar_url, department, job_title, phone
             FROM contacts.directory_profiles
             WHERE is_visible = TRUE
               AND (to_tsvector('simple', COALESCE(display_name, '') || ' ' || COALESCE(email, ''))
                    @@ plainto_tsquery('simple', $1)
                    OR display_name ILIKE '%' || $1 || '%'
                    OR email ILIKE '%' || $1 || '%')
             ORDER BY display_name ASC LIMIT $2 OFFSET $3",
        )
        .bind(q).bind(limit).bind(offset)
        .fetch_all(&state.db).await.map_err(ContactsError::Database)?
    } else {
        sqlx::query_as::<_, (Uuid, String, String, Option<String>, Option<String>, Option<String>, Option<String>)>(
            "SELECT kubuno_user_id, display_name, email, avatar_url, department, job_title, phone
             FROM contacts.directory_profiles
             WHERE is_visible = TRUE
             ORDER BY display_name ASC LIMIT $1 OFFSET $2",
        )
        .bind(limit).bind(offset)
        .fetch_all(&state.db).await.map_err(ContactsError::Database)?
    };

    let profiles: Vec<Value> = rows.into_iter().map(|(id, name, email, avatar, dept, title, phone)| json!({
        "kubuno_user_id": id,
        "display_name":   name,
        "email":          email,
        "avatar_url":     avatar,
        "department":     dept,
        "job_title":      title,
        "phone":          phone,
    })).collect();

    Ok(Json(json!({ "profiles": profiles })))
}

pub async fn add_to_contacts(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Path(kubuno_user_id): Path<Uuid>,
) -> Result<Json<Value>> {
    let profile = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, Option<String>)>(
        "SELECT display_name, email, avatar_url, department, job_title, phone
         FROM contacts.directory_profiles WHERE kubuno_user_id = $1 AND is_visible = TRUE",
    )
    .bind(kubuno_user_id)
    .fetch_optional(&state.db).await.map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound("Profil introuvable".into()))?;

    let (display_name, email, _avatar_url, department, job_title, phone) = profile;

    let emails = if email.is_empty() { vec![] } else {
        vec![crate::models::contact::ContactField { label: None, value: email, field_type: "work".into() }]
    };
    let phones = if let Some(p) = phone {
        vec![crate::models::contact::ContactField { label: None, value: p, field_type: "work".into() }]
    } else { vec![] };

    let dto = crate::models::contact::CreateContactDto {
        given_name: None, middle_name: None, family_name: None,
        name_prefix: None, name_suffix: None, nickname: None,
        display_name: Some(display_name), organization: None,
        department, job_title, avatar_color: None,
        emails, phones, addresses: vec![], urls: vec![],
        dates: vec![], relations: vec![], instant_messages: vec![],
        custom_fields: vec![], notes: None, is_starred: None,
    };

    // Set kubuno_user_id after creation
    let contact = crate::services::contact_service::create_contact(&state.db, user.id, &dto).await?;

    sqlx::query(
        "UPDATE contacts.contacts SET kubuno_user_id = $2 WHERE id = $1",
    )
    .bind(contact.id).bind(kubuno_user_id)
    .execute(&state.db).await.map_err(ContactsError::Database)?;

    Ok(Json(json!({ "contact": contact })))
}

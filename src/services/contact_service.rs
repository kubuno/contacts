use anyhow::Context;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::contact::{Contact, ContactsListResponse, CreateContactDto, ListContactsParams, UpdateContactDto},
};

pub async fn list_contacts(
    db: &PgPool,
    owner_id: Uuid,
    params: &ListContactsParams,
) -> Result<ContactsListResponse> {
    let limit  = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);
    let trashed = params.trashed.unwrap_or(false);

    let contacts = if let Some(q) = &params.q {
        if q.trim().is_empty() {
            fetch_contacts_plain(db, owner_id, params.group_id, params.starred, trashed, limit, offset).await?
        } else {
            fetch_contacts_search(db, owner_id, q, trashed, limit, offset).await?
        }
    } else {
        fetch_contacts_plain(db, owner_id, params.group_id, params.starred, trashed, limit, offset).await?
    };

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM contacts.contacts WHERE owner_id = $1 AND is_trashed = $2",
    )
    .bind(owner_id)
    .bind(trashed)
    .fetch_one(db)
    .await
    .map_err(ContactsError::Database)?;

    Ok(ContactsListResponse { contacts, total })
}

async fn fetch_contacts_plain(
    db: &PgPool,
    owner_id: Uuid,
    group_id: Option<Uuid>,
    starred: Option<bool>,
    trashed: bool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Contact>> {
    if let Some(gid) = group_id {
        sqlx::query_as::<_, Contact>(
            "SELECT c.* FROM contacts.contacts c
             JOIN contacts.group_members gm ON gm.contact_id = c.id
             WHERE c.owner_id = $1 AND c.is_trashed = $2 AND gm.group_id = $3
             ORDER BY c.display_name ASC LIMIT $4 OFFSET $5",
        )
        .bind(owner_id).bind(trashed).bind(gid).bind(limit).bind(offset)
        .fetch_all(db).await.map_err(ContactsError::Database)
    } else if let Some(s) = starred {
        sqlx::query_as::<_, Contact>(
            "SELECT * FROM contacts.contacts
             WHERE owner_id = $1 AND is_trashed = $2 AND is_starred = $3
             ORDER BY display_name ASC LIMIT $4 OFFSET $5",
        )
        .bind(owner_id).bind(trashed).bind(s).bind(limit).bind(offset)
        .fetch_all(db).await.map_err(ContactsError::Database)
    } else {
        sqlx::query_as::<_, Contact>(
            "SELECT * FROM contacts.contacts
             WHERE owner_id = $1 AND is_trashed = $2
             ORDER BY display_name ASC LIMIT $3 OFFSET $4",
        )
        .bind(owner_id).bind(trashed).bind(limit).bind(offset)
        .fetch_all(db).await.map_err(ContactsError::Database)
    }
}

async fn fetch_contacts_search(
    db: &PgPool,
    owner_id: Uuid,
    q: &str,
    trashed: bool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Contact>> {
    sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts.contacts
         WHERE owner_id = $1 AND is_trashed = $2
           AND search_vector @@ plainto_tsquery('simple', unaccent($3))
         ORDER BY ts_rank(search_vector, plainto_tsquery('simple', unaccent($3))) DESC
         LIMIT $4 OFFSET $5",
    )
    .bind(owner_id).bind(trashed).bind(q).bind(limit).bind(offset)
    .fetch_all(db).await.map_err(ContactsError::Database)
}

pub async fn get_contact(db: &PgPool, owner_id: Uuid, contact_id: Uuid) -> Result<Contact> {
    sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts.contacts WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id).bind(owner_id)
    .fetch_optional(db).await
    .map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound(format!("Contact {contact_id}")))
}

pub async fn create_contact(
    db: &PgPool,
    owner_id: Uuid,
    dto: &CreateContactDto,
) -> Result<Contact> {
    let emails   = serde_json::to_value(&dto.emails).unwrap_or(Value::Array(vec![]));
    let phones   = serde_json::to_value(&dto.phones).unwrap_or(Value::Array(vec![]));
    let addresses = serde_json::to_value(&dto.addresses).unwrap_or(Value::Array(vec![]));
    let urls     = serde_json::to_value(&dto.urls).unwrap_or(Value::Array(vec![]));
    let dates    = serde_json::to_value(&dto.dates).unwrap_or(Value::Array(vec![]));
    let relations = serde_json::to_value(&dto.relations).unwrap_or(Value::Array(vec![]));
    let ims      = serde_json::to_value(&dto.instant_messages).unwrap_or(Value::Array(vec![]));
    let custom   = serde_json::to_value(&dto.custom_fields).unwrap_or(Value::Array(vec![]));

    sqlx::query_as::<_, Contact>(
        "INSERT INTO contacts.contacts
         (owner_id, given_name, middle_name, family_name, name_prefix, name_suffix,
          nickname, display_name, organization, department, job_title, avatar_color,
          emails, phones, addresses, urls, dates, relations, instant_messages,
          custom_fields, notes, is_starred)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                 $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
         RETURNING *",
    )
    .bind(owner_id)
    .bind(&dto.given_name).bind(&dto.middle_name).bind(&dto.family_name)
    .bind(&dto.name_prefix).bind(&dto.name_suffix).bind(&dto.nickname)
    .bind(dto.display_name.as_deref().unwrap_or(""))
    .bind(&dto.organization).bind(&dto.department).bind(&dto.job_title)
    .bind(dto.avatar_color.as_deref().unwrap_or("#1a73e8"))
    .bind(emails).bind(phones).bind(addresses).bind(urls)
    .bind(dates).bind(relations).bind(ims).bind(custom)
    .bind(&dto.notes).bind(dto.is_starred.unwrap_or(false))
    .fetch_one(db).await.map_err(ContactsError::Database)
}

pub async fn update_contact(
    db: &PgPool,
    owner_id: Uuid,
    contact_id: Uuid,
    dto: &UpdateContactDto,
) -> Result<Contact> {
    let existing = get_contact(db, owner_id, contact_id).await?;

    let given_name   = dto.given_name.as_ref().or(existing.given_name.as_ref());
    let middle_name  = dto.middle_name.as_ref().or(existing.middle_name.as_ref());
    let family_name  = dto.family_name.as_ref().or(existing.family_name.as_ref());
    let name_prefix  = dto.name_prefix.as_ref().or(existing.name_prefix.as_ref());
    let name_suffix  = dto.name_suffix.as_ref().or(existing.name_suffix.as_ref());
    let nickname     = dto.nickname.as_ref().or(existing.nickname.as_ref());
    let display_name = dto.display_name.as_deref().unwrap_or(&existing.display_name);
    let organization = dto.organization.as_ref().or(existing.organization.as_ref());
    let department   = dto.department.as_ref().or(existing.department.as_ref());
    let job_title    = dto.job_title.as_ref().or(existing.job_title.as_ref());
    let avatar_color = dto.avatar_color.as_deref().unwrap_or(&existing.avatar_color);
    let notes        = dto.notes.as_ref().or(existing.notes.as_ref());
    let is_starred   = dto.is_starred.unwrap_or(existing.is_starred);

    let emails    = dto.emails.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.emails.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let phones    = dto.phones.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.phones.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let addresses = dto.addresses.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.addresses.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let urls      = dto.urls.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.urls.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let dates     = dto.dates.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.dates.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let relations = dto.relations.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.relations.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let ims       = dto.instant_messages.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.instant_messages.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());
    let custom    = dto.custom_fields.as_ref().map(|v| serde_json::to_value(v).unwrap_or(Value::Array(vec![]))).unwrap_or_else(|| existing.custom_fields.0.iter().cloned().map(|f| serde_json::to_value(f).unwrap()).collect());

    sqlx::query_as::<_, Contact>(
        "UPDATE contacts.contacts SET
         given_name = $3, middle_name = $4, family_name = $5,
         name_prefix = $6, name_suffix = $7, nickname = $8, display_name = $9,
         organization = $10, department = $11, job_title = $12, avatar_color = $13,
         emails = $14, phones = $15, addresses = $16, urls = $17, dates = $18,
         relations = $19, instant_messages = $20, custom_fields = $21,
         notes = $22, is_starred = $23
         WHERE id = $1 AND owner_id = $2
         RETURNING *",
    )
    .bind(contact_id).bind(owner_id)
    .bind(given_name).bind(middle_name).bind(family_name)
    .bind(name_prefix).bind(name_suffix).bind(nickname).bind(display_name)
    .bind(organization).bind(department).bind(job_title).bind(avatar_color)
    .bind(emails).bind(phones).bind(addresses).bind(urls).bind(dates)
    .bind(relations).bind(ims).bind(custom).bind(notes).bind(is_starred)
    .fetch_one(db).await.map_err(ContactsError::Database)
}

pub async fn trash_contact(db: &PgPool, owner_id: Uuid, contact_id: Uuid) -> Result<()> {
    let rows = sqlx::query(
        "UPDATE contacts.contacts SET is_trashed = TRUE, trashed_at = NOW()
         WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id).bind(owner_id)
    .execute(db).await.map_err(ContactsError::Database)?.rows_affected();

    if rows == 0 { return Err(ContactsError::NotFound(format!("Contact {contact_id}"))); }
    Ok(())
}

pub async fn restore_contact(db: &PgPool, owner_id: Uuid, contact_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE contacts.contacts SET is_trashed = FALSE, trashed_at = NULL
         WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id).bind(owner_id)
    .execute(db).await.map_err(ContactsError::Database)?;
    Ok(())
}

pub async fn delete_contact_permanently(
    db: &PgPool,
    owner_id: Uuid,
    contact_id: Uuid,
) -> Result<()> {
    let rows = sqlx::query(
        "DELETE FROM contacts.contacts WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id).bind(owner_id)
    .execute(db).await.map_err(ContactsError::Database)?.rows_affected();

    if rows == 0 { return Err(ContactsError::NotFound(format!("Contact {contact_id}"))); }
    Ok(())
}

pub async fn empty_trash(db: &PgPool, owner_id: Uuid) -> Result<u64> {
    let rows = sqlx::query(
        "DELETE FROM contacts.contacts WHERE owner_id = $1 AND is_trashed = TRUE",
    )
    .bind(owner_id)
    .execute(db).await.map_err(ContactsError::Database)?.rows_affected();
    Ok(rows)
}

pub async fn find_duplicates(db: &PgPool, owner_id: Uuid) -> Result<Vec<Vec<Contact>>> {
    // Groupes de contacts avec même display_name
    let rows = sqlx::query_as::<_, Contact>(
        "SELECT c.* FROM contacts.contacts c
         INNER JOIN (
             SELECT display_name FROM contacts.contacts
             WHERE owner_id = $1 AND is_trashed = FALSE AND display_name != ''
             GROUP BY display_name HAVING COUNT(*) > 1
         ) dups ON c.display_name = dups.display_name
         WHERE c.owner_id = $1 AND c.is_trashed = FALSE
         ORDER BY c.display_name, c.created_at",
    )
    .bind(owner_id)
    .fetch_all(db).await.map_err(ContactsError::Database)?;

    // Grouper par display_name
    let mut groups: Vec<Vec<Contact>> = vec![];
    let mut current_name = String::new();
    for c in rows {
        if c.display_name != current_name {
            current_name = c.display_name.clone();
            groups.push(vec![c]);
        } else if let Some(last) = groups.last_mut() {
            last.push(c);
        }
    }
    Ok(groups)
}

pub async fn star_contact(db: &PgPool, owner_id: Uuid, contact_id: Uuid, starred: bool) -> Result<()> {
    sqlx::query(
        "UPDATE contacts.contacts SET is_starred = $3 WHERE id = $1 AND owner_id = $2",
    )
    .bind(contact_id).bind(owner_id).bind(starred)
    .execute(db).await.map_err(ContactsError::Database)?;
    Ok(())
}

pub async fn log_interaction(
    db: &PgPool,
    contact_id: Uuid,
    owner_id: Uuid,
    interaction_type: &str,
    source_module: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO contacts.interaction_log (contact_id, owner_id, interaction_type, source_module)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(contact_id).bind(owner_id).bind(interaction_type).bind(source_module)
    .execute(db).await.context("log_interaction")?;
    Ok(())
}

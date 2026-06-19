use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::contact::Contact,
    services::{contact_service, share_service::sha256_hex, vcard_service},
};

/// Generates a fresh CardDAV token for the owner (replacing any existing one)
/// and returns the raw token (shown once).
pub async fn regenerate_token(db: &PgPool, owner_id: Uuid) -> Result<String> {
    let raw = crate::services::share_service::gen_token();
    let hash = sha256_hex(&raw);
    sqlx::query(
        "INSERT INTO contacts.carddav_tokens (owner_id, token_hash, created_at)
         VALUES ($1, $2, NOW())
         ON CONFLICT (owner_id) DO UPDATE SET token_hash = EXCLUDED.token_hash, created_at = NOW()",
    )
    .bind(owner_id)
    .bind(&hash)
    .execute(db)
    .await
    .map_err(ContactsError::Database)?;
    Ok(raw)
}

pub async fn has_token(db: &PgPool, owner_id: Uuid) -> Result<bool> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM contacts.carddav_tokens WHERE owner_id = $1)",
    )
    .bind(owner_id)
    .fetch_one(db)
    .await
    .map_err(ContactsError::Database)
}

pub async fn revoke_token(db: &PgPool, owner_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM contacts.carddav_tokens WHERE owner_id = $1")
        .bind(owner_id)
        .execute(db)
        .await
        .map_err(ContactsError::Database)?;
    Ok(())
}

/// Resolves a raw token to its owner (and stamps last_used_at).
pub async fn owner_for_token(db: &PgPool, raw_token: &str) -> Result<Option<Uuid>> {
    let hash = sha256_hex(raw_token);
    let owner = sqlx::query_scalar::<_, Uuid>(
        "SELECT owner_id FROM contacts.carddav_tokens WHERE token_hash = $1",
    )
    .bind(&hash)
    .fetch_optional(db)
    .await
    .map_err(ContactsError::Database)?;
    if let Some(o) = owner {
        let _ = sqlx::query("UPDATE contacts.carddav_tokens SET last_used_at = NOW() WHERE owner_id = $1")
            .bind(o)
            .execute(db)
            .await;
    }
    Ok(owner)
}

/// Collection sync tag: changes whenever any contact changes.
pub async fn ctag(db: &PgPool, owner_id: Uuid) -> Result<String> {
    let row = sqlx::query_as::<_, (i64, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT COUNT(*), MAX(updated_at) FROM contacts.contacts
         WHERE owner_id = $1 AND is_trashed = FALSE",
    )
    .bind(owner_id)
    .fetch_one(db)
    .await
    .map_err(ContactsError::Database)?;
    let stamp = row.1.map(|d| d.timestamp_millis()).unwrap_or(0);
    Ok(format!("{}-{}", row.0, stamp))
}

/// (vcard_uid, etag) of all non-trashed contacts.
pub async fn list_refs(db: &PgPool, owner_id: Uuid) -> Result<Vec<(String, String)>> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT vcard_uid, etag FROM contacts.contacts
         WHERE owner_id = $1 AND is_trashed = FALSE
         ORDER BY display_name ASC",
    )
    .bind(owner_id)
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)
}

pub async fn get_by_uid(db: &PgPool, owner_id: Uuid, uid: &str) -> Result<Option<Contact>> {
    sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts.contacts
         WHERE owner_id = $1 AND vcard_uid = $2 AND is_trashed = FALSE",
    )
    .bind(owner_id)
    .bind(uid)
    .fetch_optional(db)
    .await
    .map_err(ContactsError::Database)
}

/// Creates or updates a contact from a PUT'd vCard, keyed by `uid`.
pub async fn put_vcard(db: &PgPool, owner_id: Uuid, uid: &str, vcf: &str) -> Result<String> {
    let dtos = vcard_service::parse_vcf(vcf);
    let dto = dtos
        .into_iter()
        .next()
        .ok_or_else(|| ContactsError::Validation("vCard invalide".into()))?;

    let existing = get_by_uid(db, owner_id, uid).await?;
    let etag = if let Some(c) = existing {
        // Update existing contact via the standard service, preserving uid.
        let update = crate::models::contact::UpdateContactDto {
            given_name: dto.given_name, middle_name: dto.middle_name, family_name: dto.family_name,
            name_prefix: dto.name_prefix, name_suffix: dto.name_suffix, nickname: dto.nickname,
            display_name: dto.display_name, organization: dto.organization, department: dto.department,
            job_title: dto.job_title, avatar_color: None, pronouns: dto.pronouns,
            emails: Some(dto.emails), phones: Some(dto.phones), addresses: Some(dto.addresses),
            urls: Some(dto.urls), dates: Some(dto.dates), relations: Some(dto.relations),
            instant_messages: Some(dto.instant_messages), custom_fields: Some(dto.custom_fields),
            notes: dto.notes, is_starred: None,
        };
        let updated = contact_service::update_contact(db, owner_id, c.id, &update).await?;
        updated.etag
    } else {
        let created = contact_service::create_contact(db, owner_id, &dto).await?;
        // Pin the vcard_uid to the client-provided value so future syncs match.
        sqlx::query("UPDATE contacts.contacts SET vcard_uid = $1, import_source = 'carddav' WHERE id = $2 AND owner_id = $3")
            .bind(uid)
            .bind(created.id)
            .bind(owner_id)
            .execute(db)
            .await
            .map_err(ContactsError::Database)?;
        sqlx::query_scalar::<_, String>("SELECT etag FROM contacts.contacts WHERE id = $1")
            .bind(created.id)
            .fetch_one(db)
            .await
            .map_err(ContactsError::Database)?
    };
    Ok(etag)
}

pub async fn delete_by_uid(db: &PgPool, owner_id: Uuid, uid: &str) -> Result<bool> {
    let rows = sqlx::query(
        "DELETE FROM contacts.contacts WHERE owner_id = $1 AND vcard_uid = $2",
    )
    .bind(owner_id)
    .bind(uid)
    .execute(db)
    .await
    .map_err(ContactsError::Database)?
    .rows_affected();
    Ok(rows > 0)
}

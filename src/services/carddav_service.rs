use chrono::{DateTime, Utc};
use kubuno_db::dialect::Assign;
use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::contact::Contact,
    services::{contact_service, share_service::sha256_hex, vcard_service},
};

/// Generates a fresh CardDAV token for the owner (replacing any existing one)
/// and returns the raw token (shown once).
pub async fn regenerate_token(db: &DbPool, owner_id: Uuid) -> Result<String> {
    let raw = crate::services::share_service::gen_token();
    let hash = sha256_hex(&raw);
    let backend = db.backend();
    let clause = backend.upsert(
        "contacts.carddav_tokens",
        &["owner_id"],
        &[Assign::Incoming("token_hash"), Assign::Incoming("created_at")],
    );
    let sql = format!(
        "INSERT INTO contacts.carddav_tokens (owner_id, token_hash, created_at) VALUES ($1, $2, $3){clause}"
    );
    db.execute(&sql, params![owner_id, hash, Utc::now()])
        .await
        .map_err(ContactsError::Database)?;
    Ok(raw)
}

pub async fn has_token(db: &DbPool, owner_id: Uuid) -> Result<bool> {
    let n = db
        .fetch_scalar::<i64>(
            "SELECT COUNT(*) FROM contacts.carddav_tokens WHERE owner_id = $1",
            params![owner_id],
        )
        .await
        .map_err(ContactsError::Database)?;
    Ok(n > 0)
}

pub async fn revoke_token(db: &DbPool, owner_id: Uuid) -> Result<()> {
    db.execute("DELETE FROM contacts.carddav_tokens WHERE owner_id = $1", params![owner_id])
        .await
        .map_err(ContactsError::Database)?;
    Ok(())
}

/// Resolves a raw token to its owner (and stamps last_used_at).
pub async fn owner_for_token(db: &DbPool, raw_token: &str) -> Result<Option<Uuid>> {
    let hash = sha256_hex(raw_token);
    let owner = db
        .fetch_optional_scalar::<Uuid>(
            "SELECT owner_id FROM contacts.carddav_tokens WHERE token_hash = $1",
            params![hash],
        )
        .await
        .map_err(ContactsError::Database)?;
    if let Some(o) = owner {
        let _ = db
            .execute(
                "UPDATE contacts.carddav_tokens SET last_used_at = $1 WHERE owner_id = $2",
                params![Utc::now(), o],
            )
            .await;
    }
    Ok(owner)
}

#[derive(sqlx::FromRow)]
struct CtagRow {
    cnt:         i64,
    max_updated: Option<DateTime<Utc>>,
}

/// Collection sync tag: changes whenever any contact changes.
pub async fn ctag(db: &DbPool, owner_id: Uuid) -> Result<String> {
    let sql = format!(
        "SELECT {} AS cnt, MAX(updated_at) AS max_updated FROM contacts.contacts \
         WHERE owner_id = $1 AND is_trashed = FALSE",
        db.backend().count_bigint("*"),
    );
    let row = db
        .fetch_one_as::<CtagRow>(&sql, params![owner_id])
        .await
        .map_err(ContactsError::Database)?;
    let stamp = row.max_updated.map(|d| d.timestamp_millis()).unwrap_or(0);
    Ok(format!("{}-{}", row.cnt, stamp))
}

#[derive(sqlx::FromRow)]
struct RefRow {
    vcard_uid: String,
    etag:      String,
}

/// (vcard_uid, etag) of all non-trashed contacts.
pub async fn list_refs(db: &DbPool, owner_id: Uuid) -> Result<Vec<(String, String)>> {
    let rows = db
        .fetch_all_as::<RefRow>(
            "SELECT vcard_uid, etag FROM contacts.contacts \
             WHERE owner_id = $1 AND is_trashed = FALSE \
             ORDER BY display_name ASC",
            params![owner_id],
        )
        .await
        .map_err(ContactsError::Database)?;
    Ok(rows.into_iter().map(|r| (r.vcard_uid, r.etag)).collect())
}

pub async fn get_by_uid(db: &DbPool, owner_id: Uuid, uid: &str) -> Result<Option<Contact>> {
    db.fetch_optional_as::<Contact>(
        "SELECT * FROM contacts.contacts WHERE owner_id = $1 AND vcard_uid = $2 AND is_trashed = FALSE",
        params![owner_id, uid],
    )
    .await
    .map_err(ContactsError::Database)
}

/// Creates or updates a contact from a PUT'd vCard, keyed by `uid`.
pub async fn put_vcard(db: &DbPool, owner_id: Uuid, uid: &str, vcf: &str) -> Result<String> {
    let dtos = vcard_service::parse_vcf(vcf);
    let dto = dtos
        .into_iter()
        .next()
        .ok_or_else(|| ContactsError::Validation("vCard invalide".into()))?;

    let existing = get_by_uid(db, owner_id, uid).await?;
    let etag = if let Some(c) = existing {
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
        db.execute(
            "UPDATE contacts.contacts SET vcard_uid = $1, import_source = 'carddav', updated_at = $2 \
             WHERE id = $3 AND owner_id = $4",
            params![uid, Utc::now(), created.id, owner_id],
        )
        .await
        .map_err(ContactsError::Database)?;
        db.fetch_optional_scalar::<String>(
            "SELECT etag FROM contacts.contacts WHERE id = $1",
            params![created.id],
        )
        .await
        .map_err(ContactsError::Database)?
        .unwrap_or_default()
    };
    Ok(etag)
}

pub async fn delete_by_uid(db: &DbPool, owner_id: Uuid, uid: &str) -> Result<bool> {
    // Resolve the id first so the delete can record a tombstone (the delta feed
    // must learn the contact is gone). A CardDAV delete is a permanent delete.
    let Some(c) = get_by_uid(db, owner_id, uid).await? else {
        return Ok(false);
    };
    contact_service::delete_contact_permanently(db, owner_id, c.id).await?;
    Ok(true)
}

use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::{
        contact::Contact,
        share::{CreateShareDto, Share},
    },
};

/// Generates a URL-safe random token.
pub fn gen_token() -> String {
    let mut buf = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut buf);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn create_share(db: &PgPool, owner_id: Uuid, dto: &CreateShareDto) -> Result<Share> {
    if dto.contact_id.is_none() && dto.group_id.is_none() {
        return Err(ContactsError::Validation("contact_id ou group_id requis".into()));
    }
    // Validate ownership of the shared target.
    if let Some(cid) = dto.contact_id {
        let owns = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM contacts.contacts WHERE id = $1 AND owner_id = $2)",
        )
        .bind(cid).bind(owner_id).fetch_one(db).await.map_err(ContactsError::Database)?;
        if !owns { return Err(ContactsError::NotFound(format!("Contact {cid}"))); }
    }
    if let Some(gid) = dto.group_id {
        let owns = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM contacts.groups WHERE id = $1 AND owner_id = $2)",
        )
        .bind(gid).bind(owner_id).fetch_one(db).await.map_err(ContactsError::Database)?;
        if !owns { return Err(ContactsError::NotFound(format!("Groupe {gid}"))); }
    }

    let token = gen_token();
    let password_hash = dto.password.as_ref().map(|p| sha256_hex(p));
    let expires_at = dto.expires_in_days.map(|d| chrono::Utc::now() + chrono::Duration::days(d));

    sqlx::query_as::<_, Share>(
        "INSERT INTO contacts.shares
         (owner_id, contact_id, group_id, token, permission, expires_at, password_hash, max_accesses)
         VALUES ($1, $2, $3, $4, 'view', $5, $6, $7)
         RETURNING *",
    )
    .bind(owner_id)
    .bind(dto.contact_id)
    .bind(dto.group_id)
    .bind(&token)
    .bind(expires_at)
    .bind(&password_hash)
    .bind(dto.max_accesses)
    .fetch_one(db)
    .await
    .map_err(ContactsError::Database)
}

pub async fn list_shares(db: &PgPool, owner_id: Uuid) -> Result<Vec<Share>> {
    sqlx::query_as::<_, Share>(
        "SELECT * FROM contacts.shares WHERE owner_id = $1 ORDER BY created_at DESC",
    )
    .bind(owner_id)
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)
}

pub async fn revoke_share(db: &PgPool, owner_id: Uuid, id: Uuid) -> Result<()> {
    let rows = sqlx::query("DELETE FROM contacts.shares WHERE id = $1 AND owner_id = $2")
        .bind(id).bind(owner_id)
        .execute(db).await.map_err(ContactsError::Database)?
        .rows_affected();
    if rows == 0 { return Err(ContactsError::NotFound(format!("Partage {id}"))); }
    Ok(())
}

pub struct SharedPayload {
    pub kind:     String,
    pub contacts: Vec<Contact>,
}

/// Resolves a public share token to the contact(s) behind it, enforcing
/// expiry, access caps and the optional password.
pub async fn resolve_share(db: &PgPool, token: &str, password: Option<&str>) -> Result<SharedPayload> {
    let share = sqlx::query_as::<_, Share>(
        "SELECT * FROM contacts.shares WHERE token = $1",
    )
    .bind(token)
    .fetch_optional(db)
    .await
    .map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound("Partage introuvable".into()))?;

    if let Some(exp) = share.expires_at {
        if exp < chrono::Utc::now() {
            return Err(ContactsError::Forbidden);
        }
    }
    if let Some(max) = share.max_accesses {
        if share.access_count >= max {
            return Err(ContactsError::Forbidden);
        }
    }
    // Password gate: stored as a SHA-256 hex digest.
    let stored_hash = sqlx::query_scalar::<_, Option<String>>(
        "SELECT password_hash FROM contacts.shares WHERE id = $1",
    )
    .bind(share.id)
    .fetch_one(db)
    .await
    .map_err(ContactsError::Database)?;
    if let Some(hash) = stored_hash {
        match password {
            Some(p) if sha256_hex(p) == hash => {}
            _ => return Err(ContactsError::Unauthorized),
        }
    }

    let (kind, contacts) = if let Some(cid) = share.contact_id {
        let c = sqlx::query_as::<_, Contact>(
            "SELECT * FROM contacts.contacts WHERE id = $1 AND is_trashed = FALSE",
        )
        .bind(cid)
        .fetch_optional(db)
        .await
        .map_err(ContactsError::Database)?
        .ok_or_else(|| ContactsError::NotFound("Contact introuvable".into()))?;
        ("contact".to_string(), vec![c])
    } else if let Some(gid) = share.group_id {
        let list = sqlx::query_as::<_, Contact>(
            "SELECT c.* FROM contacts.contacts c
             JOIN contacts.group_members gm ON gm.contact_id = c.id
             WHERE gm.group_id = $1 AND c.is_trashed = FALSE
             ORDER BY c.display_name ASC",
        )
        .bind(gid)
        .fetch_all(db)
        .await
        .map_err(ContactsError::Database)?;
        ("group".to_string(), list)
    } else {
        ("empty".to_string(), vec![])
    };

    sqlx::query("UPDATE contacts.shares SET access_count = access_count + 1 WHERE id = $1")
        .bind(share.id)
        .execute(db)
        .await
        .map_err(ContactsError::Database)?;

    Ok(SharedPayload { kind, contacts })
}

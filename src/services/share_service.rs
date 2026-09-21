use base64::Engine;
use kubuno_db::{new_id, params, DbPool};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    config::instance::InstanceConfig,
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

/// Creates a public link, under the instance policy.
pub async fn create_share(
    db: &DbPool,
    owner_id: Uuid,
    dto: &CreateShareDto,
    cfg: &InstanceConfig,
) -> Result<Share> {
    if !cfg.public_shares_enabled {
        return Err(ContactsError::Forbidden);
    }
    if dto.contact_id.is_none() && dto.group_id.is_none() {
        return Err(ContactsError::Validation("contact_id ou group_id requis".into()));
    }
    if cfg.share_password_required
        && dto.password.as_deref().map(str::trim).unwrap_or("").is_empty()
    {
        return Err(ContactsError::Validation(
            "Cette instance exige un mot de passe sur les liens de partage".into(),
        ));
    }
    if let Some(cid) = dto.contact_id {
        let owns = db
            .fetch_scalar::<i64>(
                "SELECT COUNT(*) FROM contacts.contacts WHERE id = $1 AND owner_id = $2",
                params![cid, owner_id],
            )
            .await
            .map_err(ContactsError::Database)?
            > 0;
        if !owns { return Err(ContactsError::NotFound(format!("Contact {cid}"))); }
    }
    if let Some(gid) = dto.group_id {
        let owns = db
            .fetch_scalar::<i64>(
                "SELECT COUNT(*) FROM contacts.groups WHERE id = $1 AND owner_id = $2",
                params![gid, owner_id],
            )
            .await
            .map_err(ContactsError::Database)?
            > 0;
        if !owns { return Err(ContactsError::NotFound(format!("Groupe {gid}"))); }
    }

    let token = gen_token();
    let password_hash = dto.password.as_ref().map(|p| sha256_hex(p));
    let days = if cfg.share_max_expiry_days > 0 {
        Some(
            dto.expires_in_days
                .unwrap_or(cfg.share_max_expiry_days)
                .clamp(1, cfg.share_max_expiry_days),
        )
    } else {
        dto.expires_in_days
    };
    let expires_at = days.map(|d| chrono::Utc::now() + chrono::Duration::days(d));
    let id = new_id();
    let now = chrono::Utc::now();

    db.execute(
        "INSERT INTO contacts.shares \
         (id, owner_id, contact_id, group_id, token, permission, expires_at, password_hash, max_accesses, access_count, created_at) \
         VALUES ($1, $2, $3, $4, $5, 'view', $6, $7, $8, 0, $9)",
        params![
            id, owner_id, dto.contact_id, dto.group_id, token,
            expires_at, password_hash, dto.max_accesses, now,
        ],
    )
    .await
    .map_err(ContactsError::Database)?;

    db.fetch_optional_as::<Share>(
        "SELECT * FROM contacts.shares WHERE id = $1",
        params![id],
    )
    .await
    .map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound("Partage".into()))
}

pub async fn list_shares(db: &DbPool, owner_id: Uuid) -> Result<Vec<Share>> {
    db.fetch_all_as::<Share>(
        "SELECT * FROM contacts.shares WHERE owner_id = $1 ORDER BY created_at DESC",
        params![owner_id],
    )
    .await
    .map_err(ContactsError::Database)
}

pub async fn revoke_share(db: &DbPool, owner_id: Uuid, id: Uuid) -> Result<()> {
    let rows = db
        .execute(
            "DELETE FROM contacts.shares WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await
        .map_err(ContactsError::Database)?;
    if rows == 0 { return Err(ContactsError::NotFound(format!("Partage {id}"))); }
    Ok(())
}

pub struct SharedPayload {
    pub kind:     String,
    pub contacts: Vec<Contact>,
}

/// Resolves a public share token to the contact(s) behind it.
pub async fn resolve_share(db: &DbPool, token: &str, password: Option<&str>) -> Result<SharedPayload> {
    let share = db
        .fetch_optional_as::<Share>(
            "SELECT * FROM contacts.shares WHERE token = $1",
            params![token],
        )
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
    let stored_hash: Option<String> = db
        .fetch_optional_scalar::<Option<String>>(
            "SELECT password_hash FROM contacts.shares WHERE id = $1",
            params![share.id],
        )
        .await
        .map_err(ContactsError::Database)?
        .flatten();
    if let Some(hash) = stored_hash {
        match password {
            Some(p) if sha256_hex(p) == hash => {}
            _ => return Err(ContactsError::Unauthorized),
        }
    }

    let (kind, contacts) = if let Some(cid) = share.contact_id {
        let c = db
            .fetch_optional_as::<Contact>(
                "SELECT * FROM contacts.contacts WHERE id = $1 AND is_trashed = FALSE",
                params![cid],
            )
            .await
            .map_err(ContactsError::Database)?
            .ok_or_else(|| ContactsError::NotFound("Contact introuvable".into()))?;
        ("contact".to_string(), vec![c])
    } else if let Some(gid) = share.group_id {
        let list = db
            .fetch_all_as::<Contact>(
                "SELECT c.* FROM contacts.contacts c \
                 JOIN contacts.group_members gm ON gm.contact_id = c.id \
                 WHERE gm.group_id = $1 AND c.is_trashed = FALSE \
                 ORDER BY c.display_name ASC",
                params![gid],
            )
            .await
            .map_err(ContactsError::Database)?;
        ("group".to_string(), list)
    } else {
        ("empty".to_string(), vec![])
    };

    db.execute(
        "UPDATE contacts.shares SET access_count = access_count + 1 WHERE id = $1",
        params![share.id],
    )
    .await
    .map_err(ContactsError::Database)?;

    Ok(SharedPayload { kind, contacts })
}

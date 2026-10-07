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

/// Hashes a share link password: Argon2id with a random salt, as a PHC string.
pub fn hash_share_password(password: &str) -> Result<String> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    let salt = SaltString::generate(&mut OsRng);
    argon2::Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ContactsError::Internal(anyhow::anyhow!("share password hash: {e}")))
}

/// Outcome of checking a password against a stored share hash.
#[derive(Debug, PartialEq, Eq)]
pub enum PasswordCheck {
    /// Matches an Argon2 hash.
    Match,
    /// Matches a legacy unsalted SHA-256 digest: valid, and to be re-hashed.
    MatchLegacy,
    NoMatch,
}

/// Checks `given` against `stored`. Both comparisons are constant-time: Argon2's
/// own verification, and a byte fold for the legacy hex digests that links made
/// before Argon2 still carry.
pub fn check_share_password(stored: &str, given: &str) -> PasswordCheck {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    if stored.starts_with("$argon2") {
        return match PasswordHash::new(stored) {
            Ok(parsed) if argon2::Argon2::default().verify_password(given.as_bytes(), &parsed).is_ok() => {
                PasswordCheck::Match
            }
            _ => PasswordCheck::NoMatch,
        };
    }
    let digest = sha256_hex(given);
    let (a, b) = (digest.as_bytes(), stored.as_bytes());
    if a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0 {
        PasswordCheck::MatchLegacy
    } else {
        PasswordCheck::NoMatch
    }
}

/// Longest password accepted on a share link (Argon2 cost grows with it).
const MAX_SHARE_PASSWORD_LEN: usize = 256;

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
    let password_hash = match dto.password.as_deref() {
        Some(p) if !p.is_empty() => {
            if p.len() > MAX_SHARE_PASSWORD_LEN {
                return Err(ContactsError::Validation("Mot de passe trop long".into()));
            }
            Some(hash_share_password(p)?)
        }
        _ => None,
    };
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
    pub contacts: Vec<PublicContact>,
}

/// What a public link discloses of a contact: how to reach the person, nothing
/// about the owner's relationship with them. Never the owner or account ids,
/// notes, custom fields, relations, dates (birthdays), instant messaging, the
/// internal flags and counters, nor storage paths and sync identifiers.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PublicContact {
    pub display_name: String,
    pub given_name:   Option<String>,
    pub middle_name:  Option<String>,
    pub family_name:  Option<String>,
    pub name_prefix:  Option<String>,
    pub name_suffix:  Option<String>,
    pub nickname:     Option<String>,
    pub organization: Option<String>,
    pub department:   Option<String>,
    pub job_title:    Option<String>,
    pub emails:       Vec<crate::models::contact::ContactField>,
    pub phones:       Vec<crate::models::contact::ContactField>,
    pub addresses:    Vec<crate::models::contact::AddressField>,
    pub urls:         Vec<crate::models::contact::ContactField>,
}

impl From<Contact> for PublicContact {
    fn from(c: Contact) -> Self {
        Self {
            display_name: c.display_name,
            given_name:   c.given_name,
            middle_name:  c.middle_name,
            family_name:  c.family_name,
            name_prefix:  c.name_prefix,
            name_suffix:  c.name_suffix,
            nickname:     c.nickname,
            organization: c.organization,
            department:   c.department,
            job_title:    c.job_title,
            emails:       c.emails.0,
            phones:       c.phones.0,
            addresses:    c.addresses.0,
            urls:         c.urls.0,
        }
    }
}

/// Resolves a public share token to the contact(s) behind it.
///
/// Every refusal except a missing or wrong password is the same "not found", so
/// a visitor learns nothing about a link that expired, ran out of views, or
/// whose contact was archived. The contacts are re-checked against the share's
/// owner (a contact moved to another owner is not served), and only their
/// public projection ([`PublicContact`]) leaves this function.
pub async fn resolve_share(db: &DbPool, token: &str, password: Option<&str>) -> Result<SharedPayload> {
    let not_found = || ContactsError::NotFound("Partage introuvable".into());
    if token.is_empty() || token.len() > 128 || password.is_some_and(|p| p.len() > MAX_SHARE_PASSWORD_LEN) {
        return Err(not_found());
    }
    let share = db
        .fetch_optional_as::<Share>(
            "SELECT * FROM contacts.shares WHERE token = $1",
            params![token],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "contacts: reading a public share");
            ContactsError::Database(e)
        })?
        .ok_or_else(not_found)?;

    if share.expires_at.is_some_and(|exp| exp < chrono::Utc::now()) {
        return Err(not_found());
    }
    if share.max_accesses.is_some_and(|max| share.access_count >= max) {
        return Err(not_found());
    }
    let stored_hash: Option<String> = db
        .fetch_optional_scalar::<Option<String>>(
            "SELECT password_hash FROM contacts.shares WHERE id = $1",
            params![share.id],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "contacts: reading a share password hash");
            ContactsError::Database(e)
        })?
        .flatten();
    if let Some(hash) = stored_hash {
        let given = password.ok_or(ContactsError::Unauthorized)?;
        match check_share_password(&hash, given) {
            PasswordCheck::Match => {}
            PasswordCheck::MatchLegacy => {
                // Upgrade the unsalted digest now that the password is known.
                let upgraded = hash_share_password(given)?;
                db.execute(
                    "UPDATE contacts.shares SET password_hash = $1 WHERE id = $2 AND password_hash = $3",
                    params![upgraded, share.id, hash],
                )
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "contacts: upgrading a share password hash");
                    ContactsError::Database(e)
                })?;
            }
            PasswordCheck::NoMatch => return Err(ContactsError::Unauthorized),
        }
    }

    let (kind, contacts) = if let Some(cid) = share.contact_id {
        let c = db
            .fetch_optional_as::<Contact>(
                "SELECT * FROM contacts.contacts WHERE id = $1 AND owner_id = $2 \
                 AND is_trashed = FALSE AND is_archived = FALSE AND is_blocked = FALSE",
                params![cid, share.owner_id],
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "contacts: reading a shared contact");
                ContactsError::Database(e)
            })?
            .ok_or_else(not_found)?;
        ("contact".to_string(), vec![c])
    } else if let Some(gid) = share.group_id {
        let list = db
            .fetch_all_as::<Contact>(
                "SELECT c.* FROM contacts.contacts c \
                 JOIN contacts.group_members gm ON gm.contact_id = c.id \
                 JOIN contacts.groups g ON g.id = gm.group_id \
                 WHERE gm.group_id = $1 AND g.owner_id = $2 AND c.owner_id = $3 \
                 AND c.is_trashed = FALSE AND c.is_archived = FALSE AND c.is_blocked = FALSE \
                 ORDER BY c.display_name ASC",
                params![gid, share.owner_id, share.owner_id],
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "contacts: reading a shared group");
                ContactsError::Database(e)
            })?;
        ("group".to_string(), list)
    } else {
        ("empty".to_string(), vec![])
    };

    // Counted atomically: two concurrent views cannot both take the last one.
    let counted = db
        .execute(
            "UPDATE contacts.shares SET access_count = access_count + 1 \
             WHERE id = $1 AND (max_accesses IS NULL OR access_count < max_accesses)",
            params![share.id],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "contacts: counting a share view");
            ContactsError::Database(e)
        })?;
    if counted == 0 {
        return Err(not_found());
    }

    Ok(SharedPayload { kind, contacts: contacts.into_iter().map(PublicContact::from).collect() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_hashes_are_salted_argon2() {
        let a = hash_share_password("s3cret").expect("hash");
        let b = hash_share_password("s3cret").expect("hash");
        assert!(a.starts_with("$argon2id$"));
        assert_ne!(a, b, "a random salt makes two hashes of one password differ");
        assert_eq!(check_share_password(&a, "s3cret"), PasswordCheck::Match);
        assert_eq!(check_share_password(&a, "wrong"), PasswordCheck::NoMatch);
    }

    #[test]
    fn legacy_digests_still_open_and_ask_for_an_upgrade() {
        let legacy = sha256_hex("s3cret");
        assert_eq!(check_share_password(&legacy, "s3cret"), PasswordCheck::MatchLegacy);
        assert_eq!(check_share_password(&legacy, "wrong"), PasswordCheck::NoMatch);
        assert_eq!(check_share_password("", "s3cret"), PasswordCheck::NoMatch);
        assert_eq!(check_share_password("$argon2id$garbage", "s3cret"), PasswordCheck::NoMatch);
    }
}

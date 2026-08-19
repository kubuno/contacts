use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{errors::ContactsError, state::AppState};

#[derive(Debug, Clone)]
pub struct ContactsUser {
    pub id:    Uuid,
    pub role:  String,
    pub email: String,
}

pub type ContactsUserExt = axum::Extension<ContactsUser>;

pub async fn require_auth(
    State(_state): State<AppState>,
    mut req: Request,
    next: Next,
) -> std::result::Result<Response, ContactsError> {
    let user_id = req
        .headers()
        .get("x-kubuno-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or(ContactsError::Unauthorized)?;

    let role = req
        .headers()
        .get("x-kubuno-user-role")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("user")
        .to_string();

    let email = req
        .headers()
        .get("x-kubuno-user-email")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    req.extensions_mut()
        .insert(ContactsUser { id: user_id, role, email });
    Ok(next.run(req).await)
}

/// Guard of the `/internal/*` sub-router: the core, and nothing else.
///
/// Unlike every other route of this module, an internal one is not reached
/// through the core's proxy and therefore carries no `x-kubuno-user-id` to
/// trust. What authenticates it is the shared secret the core handed this
/// process at startup (`KUBUNO_INTERNAL_SECRET`), presented verbatim in
/// `X-Internal-Secret`.
///
/// Two refusals, and the second one matters as much as the first: an **empty**
/// configured secret refuses everything. Without that check, a module started
/// outside the supervisor — with no secret in its environment — would accept any
/// request sending an empty header, which for the export route means handing an
/// entire address book to anyone who can reach the port.
///
/// The comparison is constant-time. The secret is long and random, so a timing
/// attack against it is theoretical; the reason to do it anyway is that "the
/// theoretical one did not apply here" is a judgement that has to be re-made
/// correctly every time somebody copies this function.
pub async fn require_internal_secret(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> std::result::Result<Response, ContactsError> {
    let expected = state.settings.core.internal_secret.as_str();
    if expected.is_empty() {
        tracing::error!(
            "contacts: core.internal_secret vide — route interne refusée. \
             Renseignez KUBUNO_INTERNAL_SECRET."
        );
        return Err(ContactsError::Unauthorized);
    }

    let provided = req
        .headers()
        .get("x-internal-secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return Err(ContactsError::Unauthorized);
    }
    Ok(next.run(req).await)
}

/// Byte comparison whose duration does not depend on where the first difference
/// is. The length check leaks the length, which is not a secret.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn the_comparison_still_compares() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(constant_time_eq(b"", b""));
    }
}

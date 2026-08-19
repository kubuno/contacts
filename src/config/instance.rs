//! Instance-wide settings of the contacts module, as the administrator left them
//! in the console.
//!
//! Declared by `module.toml`'s `[[settings]]`, stored in `core.settings`, and read
//! back through `/internal/modules/contacts/settings`. Refreshed in the background
//! so an edit takes effect without restarting the module.
//!
//! What is deliberately NOT here: the directory policy (who appears in the
//! account directory, whether e-mail addresses are shared, which profile fields a
//! user may edit). That policy belongs to the core — `directory.enabled`,
//! `directory.share_email`, `directory.audience`, `directory.profile_edit_*` —
//! and migration `000006` dropped this module's local mirror precisely because it
//! bypassed it. A module must not re-implement it.

use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub struct InstanceConfig {
    /// Maximum number of contacts one account may hold. `0` = unlimited.
    pub max_contacts_per_user: i64,
    /// Maximum size, in megabytes, of a contact photo.
    pub max_avatar_mb: u64,
    /// Maximum number of records accepted in a single vCard/CSV import.
    pub import_max_rows: i64,
    /// Maximum number of records written by a single vCard/CSV export.
    pub export_max_rows: i64,
    /// Whether users may publish a contact or a group behind a public link.
    pub public_shares_enabled: bool,
    /// Longest lifetime, in days, a public link may be given. `0` = unlimited.
    pub share_max_expiry_days: i64,
    /// Whether a public link must be protected by a password.
    pub share_password_required: bool,
    /// Whether the CardDAV endpoint answers at all (address book synchronisation
    /// with an external client).
    pub carddav_enabled: bool,
    /// Whether the shared address book of the instance is served to users.
    pub shared_book_enabled: bool,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            max_contacts_per_user:   0,
            max_avatar_mb:           5,
            import_max_rows:         5_000,
            export_max_rows:         10_000,
            public_shares_enabled:   true,
            share_max_expiry_days:   0,
            share_password_required: false,
            carddav_enabled:         true,
            shared_book_enabled:     true,
        }
    }
}

impl InstanceConfig {
    /// Maps the core's `{key: value}` object onto the struct. Every read falls
    /// back to the compiled default; an out-of-range number is ignored rather
    /// than clamped, so a nonsensical value never silently becomes a policy.
    pub fn from_settings(settings: &Value) -> Self {
        let d = Self::default();
        let int_in = |key: &str, min: i64, max: i64, fallback: i64| -> i64 {
            settings
                .get(key)
                .and_then(Value::as_i64)
                .filter(|n| (min..=max).contains(n))
                .unwrap_or(fallback)
        };
        let bool_at = |key: &str, fallback: bool| -> bool {
            settings.get(key).and_then(Value::as_bool).unwrap_or(fallback)
        };
        Self {
            max_contacts_per_user: int_in("max_contacts_per_user", 0, 1_000_000, d.max_contacts_per_user),
            // Ceiling of 20 MB: the router refuses any body past 25 MB, so a
            // larger value would promise an upload the transport rejects first.
            max_avatar_mb: int_in("max_avatar_mb", 1, 20, d.max_avatar_mb as i64) as u64,
            import_max_rows: int_in("import_max_rows", 1, 1_000_000, d.import_max_rows),
            export_max_rows: int_in("export_max_rows", 1, 1_000_000, d.export_max_rows),
            public_shares_enabled: bool_at("public_shares_enabled", d.public_shares_enabled),
            share_max_expiry_days: int_in("share_max_expiry_days", 0, 3650, d.share_max_expiry_days),
            share_password_required: bool_at("share_password_required", d.share_password_required),
            carddav_enabled: bool_at("carddav_enabled", d.carddav_enabled),
            shared_book_enabled: bool_at("shared_book_enabled", d.shared_book_enabled),
        }
    }

    /// The flags the browser may know about, so the interface can leave out what
    /// the server would refuse anyway. Enforcement stays server-side.
    pub fn public_flags(&self) -> Value {
        serde_json::json!({
            "max_contacts_per_user":   self.max_contacts_per_user,
            "max_avatar_mb":           self.max_avatar_mb,
            "import_max_rows":         self.import_max_rows,
            "export_max_rows":         self.export_max_rows,
            "public_shares_enabled":   self.public_shares_enabled,
            "share_max_expiry_days":   self.share_max_expiry_days,
            "share_password_required": self.share_password_required,
            "carddav_enabled":         self.carddav_enabled,
            "shared_book_enabled":     self.shared_book_enabled,
        })
    }
}

/// Reads the instance settings from the core. Any failure yields `None`, so the
/// caller keeps the values it already had rather than reverting to defaults
/// because the core was briefly unreachable.
pub async fn fetch(http: &reqwest::Client, core_url: &str, secret: &str) -> Option<InstanceConfig> {
    let url = format!("{core_url}/internal/modules/contacts/settings");
    let resp = http
        .get(&url)
        .header("X-Internal-Secret", secret)
        .send()
        .await
        .map_err(|e| tracing::warn!(error = %e, "Lecture des réglages d'instance contacts"))
        .ok()?;

    if !resp.status().is_success() {
        tracing::warn!(status = %resp.status(), "Réglages d'instance contacts refusés par le core");
        return None;
    }

    let body: Value = resp
        .json()
        .await
        .map_err(|e| tracing::warn!(error = %e, "Réglages d'instance contacts : réponse illisible"))
        .ok()?;

    Some(InstanceConfig::from_settings(body.get("settings")?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_keys_keep_the_compiled_defaults() {
        let c = InstanceConfig::from_settings(&json!({}));
        assert_eq!(c.max_contacts_per_user, 0);
        assert_eq!(c.max_avatar_mb, 5);
        assert_eq!(c.import_max_rows, 5_000);
        assert_eq!(c.export_max_rows, 10_000);
        assert!(c.public_shares_enabled);
        assert_eq!(c.share_max_expiry_days, 0);
        assert!(!c.share_password_required);
        assert!(c.carddav_enabled);
        assert!(c.shared_book_enabled);
    }

    #[test]
    fn values_are_read() {
        let c = InstanceConfig::from_settings(&json!({
            "max_contacts_per_user":   2_000,
            "max_avatar_mb":           2,
            "import_max_rows":         100,
            "export_max_rows":         250,
            "public_shares_enabled":   false,
            "share_max_expiry_days":   30,
            "share_password_required": true,
            "carddav_enabled":         false,
            "shared_book_enabled":     false,
        }));
        assert_eq!(c.max_contacts_per_user, 2_000);
        assert_eq!(c.max_avatar_mb, 2);
        assert_eq!(c.import_max_rows, 100);
        assert_eq!(c.export_max_rows, 250);
        assert!(!c.public_shares_enabled);
        assert_eq!(c.share_max_expiry_days, 30);
        assert!(c.share_password_required);
        assert!(!c.carddav_enabled);
        assert!(!c.shared_book_enabled);
    }

    /// An out-of-range value must not become the policy.
    #[test]
    fn out_of_range_numbers_are_ignored() {
        let c = InstanceConfig::from_settings(&json!({ "max_avatar_mb": 0, "share_max_expiry_days": -3 }));
        assert_eq!(c.max_avatar_mb, 5);
        assert_eq!(c.share_max_expiry_days, 0);
    }
}

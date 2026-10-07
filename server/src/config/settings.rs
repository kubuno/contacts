use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server:   ServerSettings,
    pub core:     CoreSettings,
    pub database: DatabaseSettings,
    pub storage:  StorageSettings,
    pub contacts: ContactsSettings,
    pub logging:  LoggingSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoreSettings {
    pub url:             String,
    pub internal_secret: String,
}

/// The `[database]` section is owned by kubuno-db: which of its fields matter
/// depends on the engine the administrator chooses at run time, and the pool is
/// opened by `kubuno_db::connect`.
pub use kubuno_db::DbSettings as DatabaseSettings;

#[derive(Debug, Clone, Deserialize)]
pub struct StorageSettings {
    pub backend:    String,
    pub local_path: String,
    pub temp_path:  String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContactsSettings {
    /// Superseded by the instance setting `contacts.max_avatar_mb`, which the
    /// administrator edits in the console and which the handlers now read
    /// (see `config::instance`). Kept so an existing deployment file still
    /// parses; the value itself is no longer consulted.
    #[allow(dead_code)]
    pub max_avatar_mb:        u64,
    #[allow(dead_code)]
    pub auto_share_profiles:  bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Pretty,
    Json,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingSettings {
    pub level:  String,
    pub format: LogFormat,
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        let mut builder = Config::builder()
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 3110i64)?
            .set_default("core.url", "http://127.0.0.1:8080")?
            .set_default("core.internal_secret", "")?
            .set_default("database.max_connections", 10u64)?
            .set_default("database.min_connections", 1u64)?
            .set_default("database.connect_timeout", 10u64)?
            .set_default("database.run_migrations", true)?
            .set_default("database.engine", "postgres")?
            // SQLite only: where `<schema>.sqlite` lives.
            .set_default("database.path", "./data/db")?
            .set_default("storage.backend", "local")?
            .set_default("storage.local_path", "/var/lib/kubuno/modules/contacts/avatars")?
            .set_default("storage.temp_path", "/var/lib/kubuno/modules/contacts/temp")?
            .set_default("contacts.max_avatar_mb", 5i64)?
            .set_default("contacts.auto_share_profiles", true)?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "pretty")?
            .add_source(File::with_name("config").required(false))
            .add_source(File::with_name("/etc/kubuno/modules/contacts/config").required(false))
            .add_source(
                Environment::with_prefix("KC")
                    .separator("__")
                    .try_parsing(true),
            );

        // Variables injectées par le superviseur core — priorité maximale.
        builder = builder
            .set_override_option("core.url",             std::env::var("KUBUNO_CORE_URL").ok())?
            .set_override_option("core.internal_secret", std::env::var("KUBUNO_INTERNAL_SECRET").ok())?
            .set_override_option("database.host",     std::env::var("KUBUNO_DB_HOST").ok())?
            .set_override_option("database.port",     std::env::var("KUBUNO_DB_PORT").ok()
                                                        .and_then(|v| v.parse::<u64>().ok().map(|n| n.to_string())))?
            .set_override_option("database.user",     std::env::var("KUBUNO_DB_USER").ok())?
            .set_override_option("database.password", std::env::var("KUBUNO_DB_PASSWORD").ok())?
            .set_override_option("database.database", std::env::var("KUBUNO_DB_NAME").ok())?
            .set_override_option("database.path",     std::env::var("KUBUNO_DB_PATH").ok())?
            .set_override_option("database.engine",   std::env::var("KUBUNO_DB_ENGINE").ok())?;

        builder.build()?.try_deserialize()
    }
}

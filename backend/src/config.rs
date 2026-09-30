use std::env;

/// Runtime configuration, read once from the environment (`.env` supported via dotenvy).
#[derive(Clone, Debug)]
pub struct Config {
    pub bind: String,
    pub database_url: String,
    /// Bearer token for `/api/admin/*`. If unset a random one is generated and logged once.
    pub admin_token: String,
    pub admin_token_generated: bool,
    pub cors_origins: Vec<String>,
    /// A racer whose HiveShock heartbeat is older than this is marked offline (clock stops).
    pub stale_heartbeat_secs: i64,
    /// Dev convenience: when set, the tokens generated on first run are also written here (JSON).
    pub dev_tokens_file: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let (admin_token, generated) = match env::var("ADMIN_TOKEN") {
            Ok(t) if t.len() >= 16 => (t, false),
            _ => (crate::auth::random_token(), true),
        };
        Self {
            bind: env::var("BIND").unwrap_or_else(|_| "127.0.0.1:8080".into()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://zeldathon.db?mode=rwc".into()),
            admin_token,
            admin_token_generated: generated,
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:5173,http://127.0.0.1:5173".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            stale_heartbeat_secs: env::var("STALE_HEARTBEAT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(20),
            dev_tokens_file: env::var("DEV_TOKENS_FILE").ok().filter(|v| !v.is_empty()),
        }
    }
}

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
    /// First organizer account, created when the users table is empty (ADMIN_USER, default `admin`).
    pub admin_user: String,
    /// Its password (ADMIN_PASSWORD, at least 12 chars). Unset: a random one is printed once.
    pub admin_password: Option<String>,
    /// COOKIE_SECURE=true|false forces the session cookie's `Secure` flag; unset = automatic
    /// (Secure everywhere except plain http on localhost, where browsers would drop it).
    pub cookie_secure: Option<bool>,
    /// UPLOADS_DIR: where racer photos uploaded from the panel are kept (`<dir>/racers`). In Docker
    /// this lives on the data volume.
    pub uploads_dir: String,
    /// DISCORD_WEBHOOK_URL: where "X is live" announcements go. A secret: never logged or returned.
    pub discord_webhook_url: Option<String>,
    /// PUBLIC_URL: the site's address (no trailing slash) for links in Discord messages.
    pub public_url: String,
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
            admin_user: env::var("ADMIN_USER")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "admin".into()),
            admin_password: env::var("ADMIN_PASSWORD").ok().filter(|v| !v.is_empty()),
            cookie_secure: env::var("COOKIE_SECURE")
                .ok()
                .and_then(|v| match v.as_str() {
                    "true" | "1" => Some(true),
                    "false" | "0" => Some(false),
                    _ => None,
                }),
            discord_webhook_url: env::var("DISCORD_WEBHOOK_URL")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| crate::discord::valid_webhook_url(v)),
            public_url: env::var("PUBLIC_URL")
                .ok()
                .map(|v| v.trim().trim_end_matches('/').to_string())
                .filter(|v| v.starts_with("https://") || v.starts_with("http://"))
                .unwrap_or_default(),
            uploads_dir: env::var("UPLOADS_DIR")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "uploads".into()),
        }
    }
}

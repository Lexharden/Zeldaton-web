//! Organizer accounts and sessions for the admin panel. Passwords are Argon2id hashes; a session
//! is a random 256-bit token whose SHA-256 is stored (the plain value only exists in the browser
//! cookie). Everything here talks to SQLite directly because authentication needs to read its own
//! writes immediately, unlike the race state which is written through a queue.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use crate::auth::{hash_password, hash_token, random_token};

pub const MIN_PASSWORD_LEN: usize = 12;
pub const MAX_PASSWORD_LEN: usize = 128;
/// Sliding expiry: every use pushes it forward, up to the absolute limit.
pub const SESSION_IDLE_HOURS: i64 = 8;
pub const SESSION_MAX_DAYS: i64 = 7;

/// `Moderator < Admin`: a check for a role also lets every higher role through.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Moderator,
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Moderator => "moderator",
            Role::Admin => "admin",
        }
    }

    pub fn parse(s: &str) -> Self {
        if s == "admin" {
            Role::Admin
        } else {
            Role::Moderator
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    pub username: String,
    pub role: Role,
    pub disabled: bool,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub csrf: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum AccountError {
    #[error("{0}")]
    Invalid(String),
    #[error("that username is already taken")]
    Taken,
    #[error("user not found")]
    NotFound,
    #[error("there must be at least one active admin")]
    LastAdmin,
    #[error("database error: {0}")]
    Db(String),
}

impl From<sqlx::Error> for AccountError {
    fn from(e: sqlx::Error) -> Self {
        AccountError::Db(e.to_string())
    }
}

fn rfc(d: DateTime<Utc>) -> String {
    d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

pub fn normalize_username(raw: &str) -> Result<String, AccountError> {
    let name = raw.trim().to_lowercase();
    let ok = (3..=32).contains(&name.chars().count())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'));
    if ok {
        Ok(name)
    } else {
        Err(AccountError::Invalid(
            "username must be 3-32 characters: letters, digits, dot, dash or underscore".into(),
        ))
    }
}

pub fn check_password(password: &str) -> Result<(), AccountError> {
    let len = password.chars().count();
    if !(MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&len) {
        return Err(AccountError::Invalid(format!(
            "password must be {MIN_PASSWORD_LEN}-{MAX_PASSWORD_LEN} characters"
        )));
    }
    Ok(())
}

/// Argon2 is deliberately slow: keep it off the async workers.
async fn hash_blocking(password: String) -> Result<String, AccountError> {
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|e| AccountError::Db(e.to_string()))?
        .map_err(AccountError::Db)
}

fn user_from(r: &sqlx::sqlite::SqliteRow) -> User {
    User {
        id: r.get("id"),
        username: r.get("username"),
        role: Role::parse(&r.get::<String, _>("role")),
        disabled: r.get::<i64, _>("disabled") != 0,
        created_at: r.get("created_at"),
        last_login_at: r.get("last_login_at"),
    }
}

pub async fn count_users(pool: &SqlitePool) -> Result<i64, AccountError> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?)
}

async fn active_admins(pool: &SqlitePool) -> Result<i64, AccountError> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin' AND disabled = 0")
            .fetch_one(pool)
            .await?,
    )
}

pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    role: Role,
) -> Result<User, AccountError> {
    let username = normalize_username(username)?;
    check_password(password)?;
    let hash = hash_blocking(password.to_string()).await?;
    let now = rfc(Utc::now());
    let res = sqlx::query(
        "INSERT INTO users (username, password_hash, role, disabled, created_at) VALUES (?,?,?,0,?)",
    )
    .bind(&username)
    .bind(&hash)
    .bind(role.as_str())
    .bind(&now)
    .execute(pool)
    .await;
    match res {
        Ok(r) => find_by_id(pool, r.last_insert_rowid())
            .await?
            .ok_or(AccountError::NotFound),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(AccountError::Taken),
        Err(e) => Err(e.into()),
    }
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> Result<Option<User>, AccountError> {
    Ok(sqlx::query("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .map(|r| user_from(&r)))
}

/// User plus password hash, for verifying a login.
pub async fn find_for_login(
    pool: &SqlitePool,
    username: &str,
) -> Result<Option<(User, String)>, AccountError> {
    let name = username.trim().to_lowercase();
    Ok(sqlx::query("SELECT * FROM users WHERE username = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?
        .map(|r| (user_from(&r), r.get::<String, _>("password_hash"))))
}

pub async fn password_hash_of(pool: &SqlitePool, id: i64) -> Result<Option<String>, AccountError> {
    Ok(
        sqlx::query_scalar("SELECT password_hash FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn list_users(pool: &SqlitePool) -> Result<Vec<User>, AccountError> {
    Ok(sqlx::query("SELECT * FROM users ORDER BY username")
        .fetch_all(pool)
        .await?
        .iter()
        .map(user_from)
        .collect())
}

pub async fn touch_login(pool: &SqlitePool, id: i64) -> Result<(), AccountError> {
    sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
        .bind(rfc(Utc::now()))
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Changes role and/or the disabled flag. Keeps at least one active admin and ends the sessions of
/// whoever changed, so a demotion or a block takes effect immediately.
pub async fn update_user(
    pool: &SqlitePool,
    id: i64,
    role: Option<Role>,
    disabled: Option<bool>,
) -> Result<User, AccountError> {
    let current = find_by_id(pool, id).await?.ok_or(AccountError::NotFound)?;
    let new_role = role.unwrap_or(current.role);
    let new_disabled = disabled.unwrap_or(current.disabled);
    let loses_admin = current.role == Role::Admin
        && !current.disabled
        && (new_role != Role::Admin || new_disabled);
    if loses_admin && active_admins(pool).await? <= 1 {
        return Err(AccountError::LastAdmin);
    }
    sqlx::query("UPDATE users SET role = ?, disabled = ? WHERE id = ?")
        .bind(new_role.as_str())
        .bind(new_disabled)
        .bind(id)
        .execute(pool)
        .await?;
    if new_role != current.role || new_disabled != current.disabled {
        delete_user_sessions(pool, id, None).await?;
    }
    find_by_id(pool, id).await?.ok_or(AccountError::NotFound)
}

pub async fn set_password(
    pool: &SqlitePool,
    id: i64,
    password: &str,
    keep_session: Option<&str>,
) -> Result<(), AccountError> {
    check_password(password)?;
    let hash = hash_blocking(password.to_string()).await?;
    let res = sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(hash)
        .bind(id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AccountError::NotFound);
    }
    // A new password ends every other session (a stolen cookie must not survive it).
    delete_user_sessions(pool, id, keep_session).await?;
    Ok(())
}

pub async fn delete_user(pool: &SqlitePool, id: i64) -> Result<(), AccountError> {
    let current = find_by_id(pool, id).await?.ok_or(AccountError::NotFound)?;
    if current.role == Role::Admin && !current.disabled && active_admins(pool).await? <= 1 {
        return Err(AccountError::LastAdmin);
    }
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---- sessions ---------------------------------------------------------------------------------

/// Starts a session. Returns the plain token (for the cookie) and the session data.
pub async fn create_session(
    pool: &SqlitePool,
    user_id: i64,
    ip: Option<&str>,
    user_agent: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(String, Session), AccountError> {
    let token = random_token();
    let csrf = random_token();
    let expires_at = now + Duration::hours(SESSION_IDLE_HOURS);
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, csrf, created_at, expires_at, last_seen_at, ip, user_agent)
         VALUES (?,?,?,?,?,?,?,?)",
    )
    .bind(hash_token(&token))
    .bind(user_id)
    .bind(&csrf)
    .bind(rfc(now))
    .bind(rfc(expires_at))
    .bind(rfc(now))
    .bind(ip.map(|s| s.chars().take(64).collect::<String>()))
    .bind(user_agent.map(|s| s.chars().take(200).collect::<String>()))
    .execute(pool)
    .await?;
    Ok((token, Session { csrf, expires_at }))
}

/// Finds the user behind a session token, refreshing the sliding expiry. `None` for unknown,
/// expired or disabled.
pub async fn session_user(
    pool: &SqlitePool,
    token: &str,
    now: DateTime<Utc>,
) -> Result<Option<(User, Session)>, AccountError> {
    let hash = hash_token(token);
    let Some(r) = sqlx::query(
        "SELECT s.csrf, s.created_at AS s_created, s.expires_at, s.last_seen_at, u.*
         FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token_hash = ?",
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    let expires = parse_dt(&r.get::<String, _>("expires_at"));
    let created = parse_dt(&r.get::<String, _>("s_created"));
    let user = user_from(&r);
    if expires <= now || now - created > Duration::days(SESSION_MAX_DAYS) || user.disabled {
        sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(&hash)
            .execute(pool)
            .await?;
        return Ok(None);
    }
    let mut session = Session {
        csrf: r.get("csrf"),
        expires_at: expires,
    };
    let last_seen = parse_dt(&r.get::<String, _>("last_seen_at"));
    if now - last_seen > Duration::seconds(60) {
        let next = (now + Duration::hours(SESSION_IDLE_HOURS))
            .min(created + Duration::days(SESSION_MAX_DAYS));
        sqlx::query("UPDATE sessions SET last_seen_at = ?, expires_at = ? WHERE token_hash = ?")
            .bind(rfc(now))
            .bind(rfc(next))
            .bind(&hash)
            .execute(pool)
            .await?;
        session.expires_at = next;
    }
    Ok(Some((user, session)))
}

pub async fn delete_session(pool: &SqlitePool, token: &str) -> Result<(), AccountError> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
        .bind(hash_token(token))
        .execute(pool)
        .await?;
    Ok(())
}

/// Ends every session of a user, optionally keeping the one identified by `keep_token`.
pub async fn delete_user_sessions(
    pool: &SqlitePool,
    user_id: i64,
    keep_token: Option<&str>,
) -> Result<(), AccountError> {
    let keep = keep_token.map(hash_token).unwrap_or_default();
    sqlx::query("DELETE FROM sessions WHERE user_id = ? AND token_hash != ?")
        .bind(user_id)
        .bind(keep)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn purge_expired(pool: &SqlitePool, now: DateTime<Utc>) -> Result<u64, AccountError> {
    Ok(sqlx::query("DELETE FROM sessions WHERE expires_at <= ?")
        .bind(rfc(now))
        .execute(pool)
        .await?
        .rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usernames_are_normalized_and_validated() {
        assert_eq!(normalize_username("  Ana.Lopez ").unwrap(), "ana.lopez");
        assert!(normalize_username("ab").is_err());
        assert!(normalize_username("has space").is_err());
        assert!(normalize_username("ñandú").is_err());
        assert!(normalize_username(&"a".repeat(33)).is_err());
    }

    #[test]
    fn passwords_need_a_minimum_length() {
        assert!(check_password("short").is_err());
        assert!(check_password("a-long-enough-pass").is_ok());
        assert!(check_password(&"x".repeat(129)).is_err());
    }

    #[test]
    fn roles_are_ordered() {
        assert!(Role::Admin > Role::Moderator);
        assert_eq!(Role::parse("admin"), Role::Admin);
        assert_eq!(Role::parse("whatever"), Role::Moderator);
    }
}

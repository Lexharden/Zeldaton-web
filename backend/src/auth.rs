use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand::RngExt;
use sha2::{Digest, Sha256};

/// 32 random bytes, hex encoded. Shown to the operator once; only the hash is stored.
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Compares two hashes without early exit.
pub fn constant_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Extracts the token from an `Authorization: Bearer ...` header value.
pub fn bearer(header: Option<&str>) -> Option<&str> {
    header?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| !t.is_empty())
}

/// Argon2id with the crate defaults (memory-hard). The result is a self-describing PHC string.
pub fn hash_password(password: &str) -> Result<String, String> {
    let mut bytes = [0u8; 16];
    rand::rng().fill(&mut bytes);
    let salt = SaltString::encode_b64(&bytes).map_err(|e| e.to_string())?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

/// A valid hash nobody knows the password of: verifying against it costs the same as a real one,
/// so a login for an unknown user is not measurably faster than one for a known user.
pub fn dummy_hash() -> &'static str {
    static HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    HASH.get_or_init(|| hash_password(&random_token()).unwrap_or_default())
}

pub const SESSION_COOKIE: &str = "zt_session";

pub fn cookie_value<'a>(header: Option<&'a str>, name: &str) -> Option<&'a str> {
    header?
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v)
        .filter(|v| !v.is_empty())
}

/// `Set-Cookie` for a session: unreadable by scripts, never sent cross-site.
pub fn session_cookie(token: &str, secure: bool, max_age_secs: i64) -> String {
    format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={max_age_secs}{}",
        if secure { "; Secure" } else { "" }
    )
}

pub fn clear_session_cookie(secure: bool) -> String {
    session_cookie("", secure, 0)
}

/// Failed-login throttle: after `max` failures inside `window` a key is locked until the window
/// ends. In memory on purpose: a restart clears it, which is fine for a brute-force guard.
pub struct LoginLimiter {
    max: u32,
    window: Duration,
    entries: Mutex<HashMap<String, (u32, Instant)>>,
}

impl LoginLimiter {
    pub fn new(max: u32, window: Duration) -> Self {
        Self {
            max,
            window,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// `Err(seconds)` while the key is locked.
    pub fn check(&self, key: &str) -> Result<(), u64> {
        let mut map = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((count, since)) = map.get(key).copied() {
            let elapsed = since.elapsed();
            if elapsed >= self.window {
                map.remove(key);
            } else if count >= self.max {
                return Err((self.window - elapsed).as_secs().max(1));
            }
        }
        Ok(())
    }

    pub fn fail(&self, key: &str) {
        let mut map = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let entry = map.entry(key.to_string()).or_insert((0, Instant::now()));
        if entry.1.elapsed() >= self.window {
            *entry = (0, Instant::now());
        }
        entry.0 += 1;
        // Bound memory: forget the oldest keys if someone sprays random usernames.
        if map.len() > 10_000 {
            let window = self.window;
            map.retain(|_, (_, since)| since.elapsed() < window);
        }
    }

    pub fn success(&self, key: &str) {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_unique_and_hash_stably() {
        let a = random_token();
        assert_eq!(a.len(), 64);
        assert_ne!(a, random_token());
        assert_eq!(hash_token("x"), hash_token("x"));
        assert!(constant_eq(&hash_token("x"), &hash_token("x")));
        assert!(!constant_eq(&hash_token("x"), &hash_token("y")));
    }

    #[test]
    fn passwords_hash_verify_and_never_repeat() {
        let a = hash_password("correct horse battery").unwrap();
        let b = hash_password("correct horse battery").unwrap();
        assert_ne!(a, b, "salted");
        assert!(a.starts_with("$argon2id$"));
        assert!(verify_password(&a, "correct horse battery"));
        assert!(!verify_password(&a, "correct horse batterY"));
        assert!(!verify_password("not a hash", "x"));
        assert!(!verify_password(dummy_hash(), "x"));
    }

    #[test]
    fn cookies_are_parsed_and_built_safely() {
        assert_eq!(
            cookie_value(Some("a=1; zt_session=abc; b=2"), SESSION_COOKIE),
            Some("abc")
        );
        assert_eq!(cookie_value(Some("a=1"), SESSION_COOKIE), None);
        assert_eq!(cookie_value(Some("zt_session="), SESSION_COOKIE), None);
        let c = session_cookie("tok", true, 60);
        assert!(c.contains("HttpOnly") && c.contains("SameSite=Strict") && c.contains("Secure"));
        assert!(!session_cookie("tok", false, 60).contains("Secure"));
        assert!(clear_session_cookie(true).contains("Max-Age=0"));
    }

    #[test]
    fn the_login_limiter_locks_after_too_many_failures_and_resets_on_success() {
        let l = LoginLimiter::new(3, Duration::from_secs(60));
        for _ in 0..3 {
            assert!(l.check("u").is_ok());
            l.fail("u");
        }
        assert!(l.check("u").is_err());
        assert!(l.check("other").is_ok());
        l.success("u");
        assert!(l.check("u").is_ok());

        let short = LoginLimiter::new(1, Duration::from_millis(30));
        short.fail("k");
        assert!(short.check("k").is_err());
        std::thread::sleep(Duration::from_millis(40));
        assert!(short.check("k").is_ok(), "the lock expires with the window");
    }

    #[test]
    fn parses_bearer() {
        assert_eq!(bearer(Some("Bearer abc")), Some("abc"));
        assert_eq!(bearer(Some("Basic abc")), None);
        assert_eq!(bearer(None), None);
    }
}

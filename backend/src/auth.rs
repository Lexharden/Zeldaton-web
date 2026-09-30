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
    fn parses_bearer() {
        assert_eq!(bearer(Some("Bearer abc")), Some("abc"));
        assert_eq!(bearer(Some("Basic abc")), None);
        assert_eq!(bearer(None), None);
    }
}

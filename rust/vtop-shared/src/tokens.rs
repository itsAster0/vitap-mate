//! How secrets are compared: by their SHA-256, as hex. The edge
//! (`backend/convex/lib/crypto.ts`) hashes the same way.

use sha2::{Digest, Sha256};

pub fn hash(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

/// Compares two hashes without an early exit.
pub fn hashes_equal(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shared with the edge's `tokens.test.ts`.
    fn vectors() -> serde_json::Value {
        serde_json::from_str(include_str!("../vectors/tokens.json")).unwrap()
    }

    #[test]
    fn hashes_are_hex_sha256() {
        for v in vectors()["hashes"].as_array().unwrap() {
            assert_eq!(hash(v["secret"].as_str().unwrap()), v["hash"]);
        }
    }

    #[test]
    fn hashes_equal_compares_whole_strings() {
        assert!(hashes_equal(&hash("a"), &hash("a")));
        assert!(!hashes_equal(&hash("a"), &hash("b")));
        assert!(!hashes_equal("ab", "abc"));
    }
}

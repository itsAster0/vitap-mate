//! How secrets are compared: by their SHA-256, as hex. The edge
//! (`vtop-edge/src/shared/tokens.ts`) hashes the same way.

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

    #[test]
    fn hashes_are_hex_sha256() {
        assert_eq!(
            hash("a"),
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
        );
    }

    #[test]
    fn hashes_equal_compares_whole_strings() {
        assert!(hashes_equal(&hash("a"), &hash("a")));
        assert!(!hashes_equal(&hash("a"), &hash("b")));
        assert!(!hashes_equal("ab", "abc"));
    }
}

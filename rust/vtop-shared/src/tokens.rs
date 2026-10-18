//! Random secrets and how they are stored: only their SHA-256, as hex.

use rand::RngCore;
use sha2::{Digest, Sha256};

/// Personal access key for clients (extension, MCP).
pub const KEY_PREFIX: &str = "vtm_";
/// The linked app's secret; proves account changes come from that phone.
pub const APP_SECRET_PREFIX: &str = "vta_";
/// The secret a forwarding mailbox's phone holds.
pub const MAILBOX_SECRET_PREFIX: &str = "vmb_";
/// The secret an app install's event channel uses.
pub const DEVICE_SECRET_PREFIX: &str = "vdv_";

/// `prefix` + 64 hex characters from the OS random source.
pub fn random_token(prefix: &str) -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!("{prefix}{}", hex::encode(bytes))
}

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
    fn tokens_are_prefixed_and_hash_to_hex() {
        let token = random_token(MAILBOX_SECRET_PREFIX);
        assert!(token.starts_with("vmb_"));
        assert_eq!(token.len(), 4 + 64);
        assert_eq!(hash(&token).len(), 64);
        assert_ne!(token, random_token(MAILBOX_SECRET_PREFIX));
    }

    #[test]
    fn hashes_equal_compares_whole_strings() {
        assert!(hashes_equal(&hash("a"), &hash("a")));
        assert!(!hashes_equal(&hash("a"), &hash("b")));
        assert!(!hashes_equal("ab", "abc"));
    }
}

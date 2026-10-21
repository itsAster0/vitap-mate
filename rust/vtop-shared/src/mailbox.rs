//! Which student a forwarding mailbox belongs to.

use sha2::{Digest, Sha256};

/// Which student a mailbox belongs to, without naming them: keyed by the
/// engine's vault key, so only the engine can compute it.
pub fn owner_hash(vault_key: &[u8; 32], registration_number: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"vitapmate mail-otp owner v1\0");
    hasher.update(vault_key);
    hasher.update(b"\0");
    hasher.update(registration_number.trim().to_uppercase().as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_hash_ignores_case_and_spaces() {
        let key = [7u8; 32];
        assert_eq!(
            owner_hash(&key, " 23abc4567 "),
            owner_hash(&key, "23ABC4567")
        );
        assert_ne!(
            owner_hash(&key, "23ABC4567"),
            owner_hash(&[8u8; 32], "23ABC4567")
        );
    }
}

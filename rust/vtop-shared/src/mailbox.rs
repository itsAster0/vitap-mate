//! Forwarding mailboxes: `vtop-otp-<id>@<domain>`.

use rand::Rng;
use sha2::{Digest, Sha256};

pub const ID_LEN: usize = 10;
/// No `l`, `o`, `0` or `1`, so an id read aloud or typed is unambiguous.
pub const ID_ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
/// How long a sealed box waits to be collected.
pub const SEALED_TTL_SECS: u64 = 300;
pub const MAX_SEALED_LEN: usize = 2048;
pub const MAX_FCM_TOKEN_LEN: usize = 4096;
pub const ADDRESS_PREFIX: &str = "vtop-otp-";

pub fn new_id() -> String {
    let mut rng = rand::rngs::OsRng;
    (0..ID_LEN)
        .map(|_| ID_ALPHABET[rng.gen_range(0..ID_ALPHABET.len())] as char)
        .collect()
}

pub fn is_id(id: &str) -> bool {
    id.len() == ID_LEN && id.bytes().all(|byte| ID_ALPHABET.contains(&byte))
}

pub fn address(id: &str, domain: &str) -> String {
    format!("{ADDRESS_PREFIX}{id}@{domain}")
}

/// The mailbox id in a recipient address, if it is one of ours.
pub fn id_from_address(recipient: &str) -> Option<String> {
    let local = recipient.trim().split('@').next()?.to_ascii_lowercase();
    let id = local.strip_prefix(ADDRESS_PREFIX)?;
    is_id(id).then(|| id.to_string())
}

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
    fn ids_round_trip_through_addresses() {
        let id = new_id();
        assert!(is_id(&id));
        assert_eq!(
            id_from_address(&address(&id, "aster0.dev")),
            Some(id.clone())
        );
        assert_eq!(
            id_from_address(&format!(" VTOP-OTP-{}@Aster0.dev", id.to_uppercase())),
            Some(id)
        );
        assert_eq!(id_from_address("someone@aster0.dev"), None);
        assert_eq!(id_from_address("vtop-otp-short@aster0.dev"), None);
    }

    #[test]
    fn owner_hash_ignores_case_and_spaces() {
        let key = [7u8; 32];
        assert_eq!(
            owner_hash(&key, " 24mic7076 "),
            owner_hash(&key, "24MIC7076")
        );
        assert_ne!(
            owner_hash(&key, "24MIC7076"),
            owner_hash(&[8u8; 32], "24MIC7076")
        );
    }
}

//! Seals secrets for the edge's Account objects with AES-256-GCM. A sealed value is
//! `base64(nonce || ciphertext)`; the associated data names the document it
//! belongs to, so a blob copied into another document does not open.

use aes_gcm::aead::{Aead, Generate, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::de::DeserializeOwned;
use serde::Serialize;

const NONCE_LEN: usize = 12;

/// No `Debug`: it holds the key.
pub struct Sealer {
    cipher: Aes256Gcm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultError {
    /// Wrong key, wrong document, or not a sealed value.
    Corrupt,
}

impl Sealer {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            cipher: Aes256Gcm::new(&key.into()),
        }
    }

    pub fn seal(&self, aad: &str, plaintext: &[u8]) -> String {
        let nonce = Nonce::generate();
        let ciphertext = self
            .cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad: aad.as_bytes(),
                },
            )
            .expect("AES-GCM encryption of in-memory data cannot fail");
        let mut out = nonce.to_vec();
        out.extend_from_slice(&ciphertext);
        STANDARD.encode(out)
    }

    pub fn open(&self, aad: &str, sealed: &str) -> Result<Vec<u8>, VaultError> {
        let bytes = STANDARD.decode(sealed).map_err(|_| VaultError::Corrupt)?;
        if bytes.len() <= NONCE_LEN {
            return Err(VaultError::Corrupt);
        }
        let (nonce, ciphertext) = bytes.split_at(NONCE_LEN);
        let nonce = Nonce::try_from(nonce).map_err(|_| VaultError::Corrupt)?;
        self.cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: ciphertext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| VaultError::Corrupt)
    }

    pub fn seal_json<T: Serialize>(&self, aad: &str, value: &T) -> String {
        let bytes = serde_json::to_vec(value).expect("serialising plain data cannot fail");
        self.seal(aad, &bytes)
    }

    pub fn open_json<T: DeserializeOwned>(&self, aad: &str, sealed: &str) -> Result<T, VaultError> {
        serde_json::from_slice(&self.open(aad, sealed)?).map_err(|_| VaultError::Corrupt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let sealer = Sealer::new([7; 32]);
        let sealed = sealer.seal("accounts/A", b"pw");
        assert_eq!(sealer.open("accounts/A", &sealed).unwrap(), b"pw");
    }

    #[test]
    fn sealed_value_hides_plaintext() {
        let sealer = Sealer::new([7; 32]);
        let sealed = sealer.seal("accounts/A", b"hunter2");
        assert!(!sealed.contains("hunter2"));
        assert!(!STANDARD
            .decode(&sealed)
            .unwrap()
            .windows(7)
            .any(|w| w == b"hunter2"));
    }

    #[test]
    fn nonce_differs_each_seal() {
        let sealer = Sealer::new([7; 32]);
        assert_ne!(sealer.seal("a", b"x"), sealer.seal("a", b"x"));
    }

    #[test]
    fn wrong_aad_rejected() {
        let sealer = Sealer::new([7; 32]);
        let sealed = sealer.seal("accounts/A", b"x");
        assert_eq!(sealer.open("accounts/B", &sealed), Err(VaultError::Corrupt));
    }

    #[test]
    fn wrong_key_rejected() {
        let sealed = Sealer::new([7; 32]).seal("a", b"x");
        assert_eq!(
            Sealer::new([8; 32]).open("a", &sealed),
            Err(VaultError::Corrupt)
        );
    }

    #[test]
    fn garbage_rejected() {
        let sealer = Sealer::new([7; 32]);
        assert_eq!(sealer.open("a", "not base64!"), Err(VaultError::Corrupt));
        assert_eq!(sealer.open("a", "AAAA"), Err(VaultError::Corrupt));
        assert_eq!(sealer.open("a", ""), Err(VaultError::Corrupt));
    }

    #[test]
    fn json_round_trip() {
        let sealer = Sealer::new([1; 32]);
        let sealed = sealer.seal_json("x", &vec!["a".to_string()]);
        assert_eq!(
            sealer.open_json::<Vec<String>>("x", &sealed).unwrap(),
            vec!["a"]
        );
    }

    /// A value sealed before the aes-gcm 0.11 bump (key [7; 32], document
    /// `accounts/A`): Account objects keep such blobs, so it must still open.
    #[test]
    fn known_answer() {
        const SEALED: &str = "sHmWs9EMGXPskalSCxUM0+Qnr82cVuV4nc2Kjp6TDYeoGHoyKUN+tgcEYShOYpaltPA=";
        assert_eq!(
            Sealer::new([7; 32]).open("accounts/A", SEALED).unwrap(),
            br#"{"password":"hunter2"}"#
        );
        assert_eq!(
            Sealer::new([7; 32]).open("accounts/B", SEALED),
            Err(VaultError::Corrupt)
        );
    }
}

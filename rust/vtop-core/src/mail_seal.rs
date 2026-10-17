//! Sealed boxes for forwarded OTP mail. The mail Worker seals the code to
//! the phone's X25519 public key; vtop-bridge only ever relays the box, and
//! the private key never leaves the phone.
//!
//! Box v1, base64url without padding:
//! `0x01 | ephemeral public key (32) | nonce (12) | AES-256-GCM ciphertext`.
//! The key is HKDF-SHA256 over the X25519 shared secret, salted with the
//! ephemeral and recipient public keys, info [`INFO`]. The mailbox id is the
//! associated data, so a box cannot be replayed to another mailbox.
//! `workers/mail-otp/src/seal.js` implements the same format.

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

pub const VERSION: u8 = 1;
pub const INFO: &[u8] = b"vitapmate mail-otp v1";
const HEADER_LEN: usize = 1 + 32 + 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealError {
    /// Not base64url, the wrong length or an unknown version.
    Malformed,
    /// Wrong key or mailbox id, or the box was altered.
    Rejected,
}

/// A new keypair as base64url (secret, public). The secret goes into the
/// phone's secure storage; the public key to the bridge.
pub fn generate_keypair() -> (String, String) {
    let mut secret = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut secret);
    let secret = StaticSecret::from(secret);
    let public = PublicKey::from(&secret);
    (
        URL_SAFE_NO_PAD.encode(secret.to_bytes()),
        URL_SAFE_NO_PAD.encode(public.as_bytes()),
    )
}

fn key32(encoded: &str) -> Result<[u8; 32], SealError> {
    URL_SAFE_NO_PAD
        .decode(encoded.trim().trim_end_matches('='))
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or(SealError::Malformed)
}

/// True for a base64url X25519 public key.
pub fn is_public_key(encoded: &str) -> bool {
    key32(encoded).is_ok()
}

fn cipher(shared: &[u8; 32], ephemeral: &[u8; 32], recipient: &[u8; 32]) -> Aes256Gcm {
    let mut salt = [0u8; 64];
    salt[..32].copy_from_slice(ephemeral);
    salt[32..].copy_from_slice(recipient);
    let mut key = [0u8; 32];
    Hkdf::<Sha256>::new(Some(&salt), shared)
        .expand(INFO, &mut key)
        .expect("32 bytes is a valid HKDF length");
    Aes256Gcm::new(&key.into())
}

/// Seals with a fresh ephemeral key and nonce.
pub fn seal(
    recipient_public: &str,
    mailbox_id: &str,
    plaintext: &[u8],
) -> Result<String, SealError> {
    let mut ephemeral = [0u8; 32];
    let mut nonce = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut ephemeral);
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    seal_with(recipient_public, mailbox_id, plaintext, ephemeral, nonce)
}

/// Seals with the given ephemeral secret and nonce. Only for test vectors:
/// reusing either breaks the encryption.
pub fn seal_with(
    recipient_public: &str,
    mailbox_id: &str,
    plaintext: &[u8],
    ephemeral_secret: [u8; 32],
    nonce: [u8; 12],
) -> Result<String, SealError> {
    let recipient = key32(recipient_public)?;
    let ephemeral = StaticSecret::from(ephemeral_secret);
    let ephemeral_public = PublicKey::from(&ephemeral).to_bytes();
    let shared = ephemeral.diffie_hellman(&PublicKey::from(recipient));
    let ciphertext = cipher(shared.as_bytes(), &ephemeral_public, &recipient)
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad: mailbox_id.as_bytes(),
            },
        )
        .map_err(|_| SealError::Rejected)?;
    let mut out = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    out.push(VERSION);
    out.extend_from_slice(&ephemeral_public);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(URL_SAFE_NO_PAD.encode(out))
}

/// Opens a box sealed to `secret` for `mailbox_id`.
pub fn open(secret: &str, mailbox_id: &str, sealed: &str) -> Result<Vec<u8>, SealError> {
    let secret = StaticSecret::from(key32(secret)?);
    let recipient = PublicKey::from(&secret).to_bytes();
    let bytes = URL_SAFE_NO_PAD
        .decode(sealed.trim().trim_end_matches('='))
        .map_err(|_| SealError::Malformed)?;
    if bytes.len() < HEADER_LEN + 16 || bytes[0] != VERSION {
        return Err(SealError::Malformed);
    }
    let ephemeral: [u8; 32] = bytes[1..33].try_into().expect("32 bytes");
    let nonce = &bytes[33..HEADER_LEN];
    let shared = secret.diffie_hellman(&PublicKey::from(ephemeral));
    cipher(shared.as_bytes(), &ephemeral, &recipient)
        .decrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: &bytes[HEADER_LEN..],
                aad: mailbox_id.as_bytes(),
            },
        )
        .map_err(|_| SealError::Rejected)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixed inputs shared with the Worker's test, which must produce
    /// [`VECTOR_BOX`] byte for byte.
    pub const VECTOR_SECRET: [u8; 32] = [7; 32];
    pub const VECTOR_EPHEMERAL: [u8; 32] = [9; 32];
    pub const VECTOR_NONCE: [u8; 12] = [3; 12];
    pub const VECTOR_ID: &str = "k3f9x2ab7q";
    pub const VECTOR_PLAINTEXT: &str = r#"{"kind":"otp","code":"401295","sentAt":1760000000}"#;
    pub const VECTOR_PUBLIC: &str = "E75P6uryBMf9M1j8nAByGIHRdCeBKCJ-xnTzf3_pe20";
    pub const VECTOR_BOX: &str = "AVfbSzWfI65eFG5OJRIFZwRyJQY0jBUMFHU9DJM9BNQhAwMDAwMDAwMDAwMDnR9M8IRFhOh0oPy3nhcTWiVq8Xs8ocPlv23PRQgSBmfqtiQ3PzC4XkT7cks8QLqiNiJ4346hVr5T_nZryOFoQRCE";

    fn vector_public() -> String {
        URL_SAFE_NO_PAD.encode(PublicKey::from(&StaticSecret::from(VECTOR_SECRET)).as_bytes())
    }

    #[test]
    fn round_trips() {
        let (secret, public) = generate_keypair();
        assert!(is_public_key(&public));
        let sealed = seal(&public, "abc", b"401295").unwrap();
        assert_eq!(open(&secret, "abc", &sealed).unwrap(), b"401295");
    }

    #[test]
    fn wrong_mailbox_or_key_is_rejected() {
        let (secret, public) = generate_keypair();
        let (other, _) = generate_keypair();
        let sealed = seal(&public, "abc", b"x").unwrap();
        assert_eq!(open(&secret, "abd", &sealed), Err(SealError::Rejected));
        assert_eq!(open(&other, "abc", &sealed), Err(SealError::Rejected));
    }

    #[test]
    fn altered_or_short_boxes_fail() {
        let (secret, public) = generate_keypair();
        let sealed = seal(&public, "abc", b"x").unwrap();
        let mut bytes = URL_SAFE_NO_PAD.decode(&sealed).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        let altered = URL_SAFE_NO_PAD.encode(&bytes);
        assert_eq!(open(&secret, "abc", &altered), Err(SealError::Rejected));
        assert_eq!(open(&secret, "abc", "AQ"), Err(SealError::Malformed));
        assert_eq!(
            open(&secret, "abc", "not base64!"),
            Err(SealError::Malformed)
        );
        bytes[0] = 2;
        let future = URL_SAFE_NO_PAD.encode(&bytes);
        assert_eq!(open(&secret, "abc", &future), Err(SealError::Malformed));
    }

    #[test]
    fn vector_matches_the_worker() {
        let sealed = seal_with(
            &vector_public(),
            VECTOR_ID,
            VECTOR_PLAINTEXT.as_bytes(),
            VECTOR_EPHEMERAL,
            VECTOR_NONCE,
        )
        .unwrap();
        assert_eq!(vector_public(), VECTOR_PUBLIC);
        assert_eq!(sealed, VECTOR_BOX);
        let secret = URL_SAFE_NO_PAD.encode(VECTOR_SECRET);
        assert_eq!(
            open(&secret, VECTOR_ID, &sealed).unwrap(),
            VECTOR_PLAINTEXT.as_bytes()
        );
    }
}

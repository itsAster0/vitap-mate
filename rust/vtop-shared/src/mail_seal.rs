//! Sealed boxes for forwarded OTP mail. The edge Worker seals the code to
//! a public key: the phone's, or one the engine made for a sign-in. Neither
//! private key leaves its holder.
//!
//! Box v1, base64url without padding:
//! `0x01 | ephemeral public key (32) | nonce (12) | AES-256-GCM ciphertext`.
//! The key is HKDF-SHA256 over the X25519 shared secret, salted with the
//! ephemeral and recipient public keys, info [`INFO`]. The mailbox id is the
//! associated data, so a box cannot be replayed to another mailbox.
//! The app and the engine open boxes through this module; the edge seals
//! with its TypeScript copy (`backend/convex/lib/mailSeal.ts`), checked
//! against the vector in `vectors/mail_seal.json`.

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hkdf::Hkdf;
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
// No key is better than a weak one: without the OS random source this
// aborts. Making it fallible changes the callers in vtop-ffi and vtop-bridge.
#[allow(
    clippy::expect_used,
    reason = "no key is better than a weak one: abort without the OS random source"
)]
pub fn generate_keypair() -> (String, String) {
    let mut secret = [0u8; 32];
    getrandom::fill(&mut secret).expect("the OS random source");
    let secret = StaticSecret::from(secret);
    let public = PublicKey::from(&secret);
    (
        URL_SAFE_NO_PAD.encode(secret.to_bytes()),
        URL_SAFE_NO_PAD.encode(public.as_bytes()),
    )
}

/// A base64url 32-byte key (X25519), padded or not.
pub fn key32(encoded: &str) -> Result<[u8; 32], SealError> {
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

fn cipher(
    shared: &[u8; 32],
    ephemeral: &[u8; 32],
    recipient: &[u8; 32],
) -> Result<Aes256Gcm, SealError> {
    let mut salt = [0u8; 64];
    let (head, tail) = salt.split_at_mut(32);
    head.copy_from_slice(ephemeral);
    tail.copy_from_slice(recipient);
    let mut key = [0u8; 32];
    Hkdf::<Sha256>::new(Some(&salt), shared)
        .expand(INFO, &mut key)
        .map_err(|_| SealError::Rejected)?;
    Ok(Aes256Gcm::new(&key.into()))
}

/// Seals with a fresh ephemeral key and nonce.
pub fn seal(
    recipient_public: &str,
    mailbox_id: &str,
    plaintext: &[u8],
) -> Result<String, SealError> {
    let mut ephemeral = [0u8; 32];
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut ephemeral).map_err(|_| SealError::Rejected)?;
    getrandom::fill(&mut nonce).map_err(|_| SealError::Rejected)?;
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
    let ciphertext = cipher(shared.as_bytes(), &ephemeral_public, &recipient)?
        .encrypt(
            &Nonce::from(nonce),
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
    let (version, rest) = bytes.split_first().ok_or(SealError::Malformed)?;
    let (ephemeral, rest) = rest.split_first_chunk::<32>().ok_or(SealError::Malformed)?;
    let (nonce, ciphertext) = rest.split_first_chunk::<12>().ok_or(SealError::Malformed)?;
    if *version != VERSION || ciphertext.len() < 16 {
        return Err(SealError::Malformed);
    }
    let shared = secret.diffie_hellman(&PublicKey::from(*ephemeral));
    cipher(shared.as_bytes(), ephemeral, &recipient)?
        .decrypt(
            &Nonce::from(*nonce),
            Payload {
                msg: ciphertext,
                aad: mailbox_id.as_bytes(),
            },
        )
        .map_err(|_| SealError::Rejected)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixed inputs shared with the edge's test (`mail-seal.test.ts`),
    /// which must produce the same box byte for byte. Bytes are hex.
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Vector {
        secret: String,
        ephemeral: String,
        nonce: String,
        mailbox_id: String,
        plaintext: String,
        public: String,
        #[serde(rename = "box")]
        sealed: String,
    }

    impl Vector {
        fn load() -> Self {
            serde_json::from_str(include_str!("../vectors/mail_seal.json")).unwrap()
        }

        fn secret(&self) -> [u8; 32] {
            hex::decode(&self.secret).unwrap().try_into().unwrap()
        }

        fn ephemeral(&self) -> [u8; 32] {
            hex::decode(&self.ephemeral).unwrap().try_into().unwrap()
        }

        fn nonce(&self) -> [u8; 12] {
            hex::decode(&self.nonce).unwrap().try_into().unwrap()
        }

        fn public_of_secret(&self) -> String {
            URL_SAFE_NO_PAD.encode(PublicKey::from(&StaticSecret::from(self.secret())).as_bytes())
        }
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
        let v = Vector::load();
        let sealed = seal_with(
            &v.public_of_secret(),
            &v.mailbox_id,
            v.plaintext.as_bytes(),
            v.ephemeral(),
            v.nonce(),
        )
        .unwrap();
        assert_eq!(v.public_of_secret(), v.public);
        assert_eq!(sealed, v.sealed);
        let secret = URL_SAFE_NO_PAD.encode(v.secret());
        assert_eq!(
            open(&secret, &v.mailbox_id, &sealed).unwrap(),
            v.plaintext.as_bytes()
        );
    }

    /// Known answers pinned before the x25519-dalek 3 / hkdf 0.13 / aes-gcm
    /// 0.11 bump: RFC 7748 §6.1, RFC 5869 A.1, the vector's shared secret, and
    /// the edge's box opened as given.
    #[test]
    fn known_answers() {
        let v = Vector::load();
        let key = |h: &str| <[u8; 32]>::try_from(hex::decode(h).unwrap()).unwrap();
        let alice = StaticSecret::from(key(
            "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a",
        ));
        let bob = key("de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f");
        assert_eq!(
            hex::encode(PublicKey::from(&alice).as_bytes()),
            "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"
        );
        assert_eq!(
            hex::encode(alice.diffie_hellman(&PublicKey::from(bob)).as_bytes()),
            "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742"
        );
        let vector = StaticSecret::from(v.ephemeral())
            .diffie_hellman(&PublicKey::from(&StaticSecret::from(v.secret())));
        assert_eq!(
            hex::encode(vector.as_bytes()),
            "2ffe325b2a26611bf1911325d4a62b33749b8f350d0e88ba91e316acaae66554"
        );

        let mut okm = [0u8; 42];
        Hkdf::<Sha256>::new(
            Some(&hex::decode("000102030405060708090a0b0c").unwrap()),
            &[0x0b; 22],
        )
        .expand(&hex::decode("f0f1f2f3f4f5f6f7f8f9").unwrap(), &mut okm)
        .unwrap();
        assert_eq!(
            hex::encode(okm),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
        );

        let secret = URL_SAFE_NO_PAD.encode(v.secret());
        assert_eq!(
            open(&secret, &v.mailbox_id, &v.sealed).unwrap(),
            v.plaintext.as_bytes()
        );
        let (secret, public) = generate_keypair();
        assert_eq!(
            URL_SAFE_NO_PAD
                .encode(PublicKey::from(&StaticSecret::from(key32(&secret).unwrap())).as_bytes()),
            public
        );
    }
}

//! What the Durable Objects store, and the checks on what clients send.
//! No Worker APIs here, so it is tested natively.

use serde::{Deserialize, Serialize};
use vtop_shared::mail_seal;
use vtop_shared::mailbox::{MAX_FCM_TOKEN_LEN, MAX_SEALED_LEN, SEALED_TTL_SECS};

/// One app install: where its pushes go and its event log's position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceDoc {
    pub id: String,
    pub secret_hash: String,
    pub fcm_token: String,
    /// The last sequence number handed out.
    pub seq: u64,
    pub created_at: u64,
}

/// One forwarding address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxDoc {
    pub id: String,
    pub secret_hash: String,
    pub public_key: String,
    pub device_id: String,
    #[serde(default)]
    pub owner_hash: Option<String>,
    #[serde(default)]
    pub sealed: Option<String>,
    #[serde(default)]
    pub sealed_at: Option<u64>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl MailboxDoc {
    /// The latest box, while it may still be collected.
    pub fn fresh_sealed(&self, now: u64) -> Option<(&str, u64)> {
        let at = self.sealed_at?;
        let sealed = self.sealed.as_deref()?;
        (now.saturating_sub(at) <= SEALED_TTL_SECS).then_some((sealed, at))
    }

    /// A box sealed for the old key can no longer be opened.
    pub fn set_public_key(&mut self, key: String) {
        if key != self.public_key {
            self.sealed = None;
            self.sealed_at = None;
        }
        self.public_key = key;
    }
}

/// How long an engine sign-in may wait for forwarded OTPs.
pub const WAITER_TTL_SECS: u64 = 300;

/// An engine sign-in waiting for its student's next OTP mail. Codes come
/// sealed to its key; the edge keeps nothing it can read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waiter {
    pub public_key: String,
    pub owner_hash: String,
    /// Mail sent before this (unix seconds) is not for this sign-in.
    pub since: u64,
    pub expires_at: u64,
    #[serde(default)]
    pub results: Vec<String>,
}

impl Waiter {
    pub fn wants(&self, sent_at: u64, now: u64) -> bool {
        now < self.expires_at && sent_at >= self.since
    }
}

/// Device ids are 32 hex characters.
pub fn is_device_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

pub fn fcm_token(raw: Option<&str>) -> Result<Option<String>, &'static str> {
    match raw.map(str::trim) {
        None => Ok(None),
        Some(token) if token.is_empty() || token.len() > MAX_FCM_TOKEN_LEN => {
            Err("fcmToken is not an FCM token.")
        }
        Some(token) => Ok(Some(token.to_string())),
    }
}

pub fn public_key(raw: Option<&str>) -> Result<Option<String>, &'static str> {
    match raw.map(str::trim) {
        None => Ok(None),
        Some(key) if mail_seal::is_public_key(key) => Ok(Some(key.to_string())),
        Some(_) => Err("publicKey must be a base64url X25519 key."),
    }
}

pub fn is_sealed_box(sealed: &str) -> bool {
    !sealed.is_empty()
        && sealed.len() <= MAX_SEALED_LEN
        && sealed
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mailbox() -> MailboxDoc {
        MailboxDoc {
            id: "abcdefghij".into(),
            secret_hash: "h".into(),
            public_key: "k1".into(),
            device_id: "d".into(),
            owner_hash: None,
            sealed: Some("AQ".into()),
            sealed_at: Some(1000),
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn sealed_boxes_expire() {
        let doc = mailbox();
        assert_eq!(doc.fresh_sealed(1000 + SEALED_TTL_SECS), Some(("AQ", 1000)));
        assert_eq!(doc.fresh_sealed(1001 + SEALED_TTL_SECS), None);
    }

    #[test]
    fn a_new_key_drops_the_box() {
        let mut doc = mailbox();
        doc.set_public_key("k1".into());
        assert!(doc.sealed.is_some());
        doc.set_public_key("k2".into());
        assert_eq!((doc.sealed, doc.sealed_at), (None, None));
    }

    #[test]
    fn inputs_are_checked() {
        assert!(is_device_id(&"a".repeat(32)));
        assert!(!is_device_id(&"A".repeat(32)));
        assert!(!is_device_id("abc"));
        assert_eq!(fcm_token(Some("  t ")), Ok(Some("t".into())));
        assert!(fcm_token(Some(" ")).is_err());
        assert_eq!(fcm_token(None), Ok(None));
        assert!(public_key(Some("nope")).is_err());
        let (_, public) = mail_seal::generate_keypair();
        assert_eq!(public_key(Some(&public)), Ok(Some(public.clone())));
        assert!(is_sealed_box("AQ-_z9"));
        assert!(!is_sealed_box("AQ=="));
        assert!(!is_sealed_box(""));
    }
}

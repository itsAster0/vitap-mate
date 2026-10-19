//! What the Durable Objects store, and the checks on what clients send.
//! No Worker APIs here, so it is tested natively.

use serde::{Deserialize, Serialize};
use vtop_shared::mail_seal;
use vtop_shared::mailbox::{MAX_FCM_TOKEN_LEN, MAX_SEALED_LEN, SEALED_TTL_SECS};
use vtop_shared::tokens;

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

/// Installs one address forwards to, at most. Adding one more drops the
/// one seen least recently.
pub const MAX_MEMBERS: usize = 5;
/// An install not seen for this long is dropped when another joins.
pub const MEMBER_STALE_SECS: u64 = 60 * 24 * 3600;
/// `seen_at` is written at most this often, so polling stays read-only.
const SEEN_EVERY_SECS: u64 = 3600;
pub const MAX_LABEL_LEN: usize = 60;

/// One install an address forwards to: its own key, secret and pending
/// box, so each opens only what was sealed for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxMember {
    pub device_id: String,
    pub secret_hash: String,
    pub public_key: String,
    /// What the app calls this install ("Pixel 8"), for the device list.
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub sealed: Option<String>,
    #[serde(default)]
    pub sealed_at: Option<u64>,
    pub added_at: u64,
    /// When this install last used the address.
    #[serde(default)]
    pub seen_at: u64,
}

impl MailboxMember {
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

/// One forwarding address and the installs it forwards to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxDoc {
    pub id: String,
    #[serde(default)]
    pub owner_hash: Option<String>,
    #[serde(default)]
    pub members: Vec<MailboxMember>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl MailboxDoc {
    /// The member whose secret this is.
    pub fn member_by_secret(&self, secret_hash: &str) -> Option<usize> {
        self.members
            .iter()
            .position(|m| tokens::hashes_equal(&m.secret_hash, secret_hash))
    }

    /// Adds `member`, replacing an earlier entry for the same install.
    /// Installs gone quiet are dropped, then the least recently seen while
    /// there are too many. The new member always stays.
    pub fn add_member(&mut self, member: MailboxMember, now: u64) {
        self.members.retain(|m| m.device_id != member.device_id);
        self.members
            .retain(|m| now.saturating_sub(m.seen_at.max(m.added_at)) < MEMBER_STALE_SECS);
        while self.members.len() >= MAX_MEMBERS {
            let oldest = self
                .members
                .iter()
                .enumerate()
                .min_by_key(|(_, m)| m.seen_at.max(m.added_at))
                .map(|(i, _)| i)
                .expect("members is not empty");
            self.members.remove(oldest);
        }
        self.members.push(member);
    }

    /// Records a visit; true when that changed what is stored.
    pub fn touch(&mut self, index: usize, now: u64) -> bool {
        let member = &mut self.members[index];
        if now.saturating_sub(member.seen_at) < SEEN_EVERY_SECS {
            return false;
        }
        member.seen_at = now;
        true
    }

    /// Drops boxes that can no longer be collected; true when any was.
    pub fn clear_expired(&mut self, now: u64) -> bool {
        let mut changed = false;
        for member in &mut self.members {
            if member.sealed.is_some() && member.fresh_sealed(now).is_none() {
                member.sealed = None;
                member.sealed_at = None;
                changed = true;
            }
        }
        changed
    }

    /// When the last pending box expires, for the cleanup alarm.
    pub fn next_expiry(&self) -> Option<u64> {
        self.members
            .iter()
            .filter_map(|m| m.sealed_at)
            .max()
            .map(|at| at + SEALED_TTL_SECS + 1)
    }
}

/// A label as sent by the app: trimmed, short, or none.
pub fn member_label(raw: Option<&str>) -> Option<String> {
    let label = raw?.trim();
    (!label.is_empty()).then(|| label.chars().take(MAX_LABEL_LEN).collect())
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

    fn member(device: &str, seen_at: u64) -> MailboxMember {
        MailboxMember {
            device_id: device.into(),
            secret_hash: format!("hash-{device}"),
            public_key: format!("key-{device}"),
            label: None,
            sealed: None,
            sealed_at: None,
            added_at: seen_at,
            seen_at,
        }
    }

    fn mailbox(members: Vec<MailboxMember>) -> MailboxDoc {
        MailboxDoc {
            id: "abcdefghij".into(),
            owner_hash: None,
            members,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn sealed_boxes_expire() {
        let mut m = member("a", 0);
        m.sealed = Some("AQ".into());
        m.sealed_at = Some(1000);
        assert_eq!(m.fresh_sealed(1000 + SEALED_TTL_SECS), Some(("AQ", 1000)));
        assert_eq!(m.fresh_sealed(1001 + SEALED_TTL_SECS), None);
        let mut doc = mailbox(vec![m]);
        assert_eq!(doc.next_expiry(), Some(1001 + SEALED_TTL_SECS));
        assert!(!doc.clear_expired(1000 + SEALED_TTL_SECS));
        assert!(doc.clear_expired(1001 + SEALED_TTL_SECS));
        assert_eq!(doc.members[0].sealed, None);
    }

    #[test]
    fn a_new_key_drops_the_box() {
        let mut m = member("a", 0);
        m.sealed = Some("AQ".into());
        m.sealed_at = Some(1);
        m.set_public_key("key-a".into());
        assert!(m.sealed.is_some());
        m.set_public_key("k2".into());
        assert_eq!((m.sealed, m.sealed_at), (None, None));
    }

    #[test]
    fn members_are_found_by_secret() {
        let doc = mailbox(vec![member("a", 0), member("b", 0)]);
        assert_eq!(doc.member_by_secret("hash-b"), Some(1));
        assert_eq!(doc.member_by_secret("hash-c"), None);
    }

    #[test]
    fn a_device_joining_again_replaces_its_entry() {
        let now = 1_000_000;
        let mut doc = mailbox(vec![member("a", now), member("b", now)]);
        let mut again = member("a", now);
        again.public_key = "new".into();
        doc.add_member(again, now);
        assert_eq!(doc.members.len(), 2);
        assert_eq!(doc.members[1].device_id, "a");
        assert_eq!(doc.members[1].public_key, "new");
    }

    #[test]
    fn the_least_recently_seen_make_room() {
        let now = 1_000_000;
        let mut doc = mailbox(
            (0..MAX_MEMBERS as u64)
                .map(|i| member(&format!("d{i}"), now - 100 + i))
                .collect(),
        );
        doc.add_member(member("new", now), now);
        assert_eq!(doc.members.len(), MAX_MEMBERS);
        assert!(doc.members.iter().all(|m| m.device_id != "d0"));
        assert_eq!(doc.members.last().unwrap().device_id, "new");
    }

    #[test]
    fn quiet_devices_are_dropped_when_one_joins() {
        let now = MEMBER_STALE_SECS * 2;
        let mut doc = mailbox(vec![member("old", 1), member("recent", now - 10)]);
        doc.add_member(member("new", now), now);
        let ids: Vec<_> = doc.members.iter().map(|m| m.device_id.as_str()).collect();
        assert_eq!(ids, ["recent", "new"]);
    }

    #[test]
    fn visits_are_written_at_most_hourly() {
        let mut doc = mailbox(vec![member("a", 0)]);
        assert!(doc.touch(0, 10_000));
        assert!(!doc.touch(0, 10_000 + 60));
        assert!(doc.touch(0, 10_000 + 3600));
    }

    #[test]
    fn labels_are_trimmed_and_capped() {
        assert_eq!(member_label(Some("  Pixel 8 ")), Some("Pixel 8".into()));
        assert_eq!(member_label(Some("  ")), None);
        assert_eq!(member_label(None), None);
        assert_eq!(
            member_label(Some(&"x".repeat(100))).unwrap().len(),
            MAX_LABEL_LEN
        );
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

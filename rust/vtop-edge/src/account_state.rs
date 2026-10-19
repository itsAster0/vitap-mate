//! What an `Account` object keeps, and the decisions it makes: when a
//! cached session or saved credentials may be used, who is asked for a
//! session first, how a request looks to its client. No Worker APIs here,
//! so it is tested natively.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use vtop_shared::account::{key_id, AccountSettings, MAX_KEYS_PER_ACCOUNT};
use vtop_shared::cookies;

/// How long a cached session is handed out before a new one is fetched.
pub const SESSION_CACHE_SECS: u64 = 7 * 24 * 3600;
/// How long a client may wait on one session request.
pub const REQUEST_TIMEOUT_SECS: u64 = 120;
/// How long a finished request is kept for late polls and answers.
pub const REQUEST_KEEP_SECS: u64 = 600;
pub const POLL_AFTER_MS: u64 = 1500;
/// Session requests one account may make per minute.
pub const ACCOUNT_RATE_PER_MINUTE: u32 = 60;
/// Phones one account may link. Linking one more drops the one seen least
/// recently.
pub const MAX_PHONES: usize = 5;
/// `seen_at` is written at most this often.
const PHONE_SEEN_EVERY_SECS: u64 = 3600;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRecord {
    pub hash: String,
    pub label: String,
    pub created_at: u64,
    #[serde(default)]
    pub last_used_at: Option<u64>,
    /// The install of the phone that created it: the only phone its
    /// session requests go to. None for keys from before keys had one;
    /// those ask every linked phone.
    #[serde(default)]
    pub device_id: Option<String>,
}

/// One linked app install: its own app secret, and the install session
/// requests are pushed to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneRecord {
    pub device_id: String,
    pub secret_hash: String,
    /// What the app calls itself ("OnePlus CPH2573"), for the phone list.
    #[serde(default)]
    pub label: Option<String>,
    pub added_at: u64,
    #[serde(default)]
    pub seen_at: u64,
}

/// The short id a phone is shown and removed by; the device id itself
/// stays private.
pub fn phone_id(device_id: &str) -> String {
    key_id(&vtop_shared::tokens::hash(device_id)).to_string()
}

/// One student's account. Secrets are sealed by the engine with `VAULT_KEY`,
/// an edge secret the edge passes to the engine: storage alone does not
/// open them, but whoever controls the Worker can.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRecord {
    pub id: String,
    /// The one phone of accounts from before several could link; moved to
    /// [`Self::phones`] when read ([`Self::adopt_legacy_phone`]).
    #[serde(default)]
    pub app_secret_hash: Option<String>,
    #[serde(default)]
    pub settings: AccountSettings,
    #[serde(default)]
    pub semester_id: Option<String>,
    /// The legacy phone's install; see [`Self::app_secret_hash`].
    #[serde(default)]
    pub device_id: Option<String>,
    /// The linked phones; each is asked for sessions.
    #[serde(default)]
    pub phones: Vec<PhoneRecord>,
    /// The student's forwarding address, so the dashboard leads from the
    /// registration number to it (its Index is named by a keyed hash).
    #[serde(default)]
    pub mailbox_id: Option<String>,
    #[serde(default)]
    pub sealed_vault: Option<String>,
    #[serde(default)]
    pub vault_saved_at: Option<u64>,
    #[serde(default)]
    pub sealed_session: Option<String>,
    #[serde(default)]
    pub session_saved_at: Option<u64>,
    /// Who made the cached session: `phone` or `server`.
    #[serde(default)]
    pub session_source: Option<String>,
    /// When VTOP last rejected the saved password (on the server or on the
    /// phone). Saved credentials are dropped then and not used again until
    /// the phone sends new ones.
    #[serde(default)]
    pub credentials_rejected_at: Option<u64>,
    #[serde(default)]
    pub keys: Vec<KeyRecord>,
    pub created_at: u64,
    pub updated_at: u64,
    /// Rate limit window: its start and the hits in it.
    #[serde(default)]
    pub window: (u64, u32),
    /// The record as read from storage, so a save writes only what this
    /// request changed and leaves changes other requests made meanwhile.
    #[serde(skip)]
    pub loaded: Snapshot,
}

/// A copy of a stored record; never part of comparisons.
#[derive(Debug, Clone, Default)]
pub struct Snapshot(pub Option<Box<AccountRecord>>);

impl PartialEq for Snapshot {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for Snapshot {}

impl AccountRecord {
    /// Remembers the current state as what storage holds.
    pub fn mark_loaded(&mut self) {
        let mut base = self.clone();
        base.loaded = Snapshot::default();
        self.loaded = Snapshot(Some(Box::new(base)));
    }

    pub fn new(id: &str, now: u64) -> Self {
        Self {
            id: id.to_string(),
            app_secret_hash: None,
            settings: AccountSettings::default(),
            semester_id: None,
            device_id: None,
            phones: Vec::new(),
            mailbox_id: None,
            sealed_vault: None,
            vault_saved_at: None,
            sealed_session: None,
            session_saved_at: None,
            session_source: None,
            credentials_rejected_at: None,
            keys: Vec::new(),
            created_at: now,
            updated_at: now,
            window: (0, 0),
            loaded: Snapshot::default(),
        }
    }

    /// Saved credentials that may still be used: not past the student's
    /// chosen lifetime.
    pub fn usable_vault(&self, now: u64) -> Option<&str> {
        let sealed = self.sealed_vault.as_deref()?;
        match (self.settings.vault_ttl_secs, self.vault_saved_at) {
            (Some(ttl), Some(at)) if now >= at + ttl => None,
            _ => Some(sealed),
        }
    }

    pub fn cached_session(&self, now: u64) -> Option<&str> {
        let sealed = self.sealed_session.as_deref()?;
        let at = self.session_saved_at.unwrap_or(0);
        (now < at + SESSION_CACHE_SECS).then_some(sealed)
    }

    /// Caches a session; `source` is who made it (`phone` or `server`).
    pub fn save_session(&mut self, sealed: String, now: u64, source: &str) {
        self.sealed_session = Some(sealed);
        self.session_saved_at = Some(now);
        self.session_source = Some(source.to_string());
    }

    pub fn clear_session(&mut self) {
        self.sealed_session = None;
        self.session_saved_at = None;
        self.session_source = None;
    }

    /// Where the cached session came from, for clients to show.
    pub fn cached_source(&self) -> Value {
        source_view(self.session_source.as_deref(), self.session_saved_at, true)
    }

    pub fn save_vault(&mut self, sealed: String, now: u64) {
        self.sealed_vault = Some(sealed);
        self.vault_saved_at = Some(now);
        self.credentials_rejected_at = None;
    }

    pub fn clear_vault(&mut self) {
        self.sealed_vault = None;
        self.vault_saved_at = None;
    }

    pub fn reject_credentials(&mut self, now: u64) {
        self.clear_vault();
        self.credentials_rejected_at = Some(now);
    }

    /// The phone should send credentials with its answer: none are saved,
    /// or saved ones expire (each answer restarts their clock).
    pub fn want_credentials(&self, now: u64) -> bool {
        self.usable_vault(now).is_none() || self.settings.vault_ttl_secs.is_some()
    }

    /// Moves the phone of a record from before several could link into
    /// [`Self::phones`]; true when there was one.
    pub fn adopt_legacy_phone(&mut self) -> bool {
        let (Some(secret_hash), device) = (self.app_secret_hash.take(), self.device_id.take())
        else {
            self.device_id = None;
            return false;
        };
        // A secret without an install cannot be asked; it links again.
        let Some(device_id) = device else {
            return true;
        };
        if !self.phones.iter().any(|p| p.device_id == device_id) {
            self.phones.push(PhoneRecord {
                device_id,
                secret_hash,
                label: None,
                added_at: self.created_at,
                seen_at: self.updated_at,
            });
        }
        true
    }

    pub fn linked(&self) -> bool {
        !self.phones.is_empty()
    }

    /// The linked phone holding this app secret.
    pub fn phone_by_secret(&self, presented: Option<&str>) -> Option<usize> {
        let secret = presented.map(str::trim).filter(|s| !s.is_empty())?;
        let hash = vtop_shared::tokens::hash(secret);
        self.phones
            .iter()
            .position(|p| vtop_shared::tokens::hashes_equal(&p.secret_hash, &hash))
    }

    /// Adds `phone`, replacing an earlier entry for the same install; the
    /// least recently seen go while there are too many. The new one stays.
    pub fn add_phone(&mut self, phone: PhoneRecord) {
        self.phones.retain(|p| p.device_id != phone.device_id);
        while self.phones.len() >= MAX_PHONES {
            let oldest = self
                .phones
                .iter()
                .enumerate()
                .min_by_key(|(_, p)| p.seen_at.max(p.added_at))
                .map(|(i, _)| i)
                .expect("phones is not empty");
            self.phones.remove(oldest);
        }
        self.phones.push(phone);
    }

    /// Unlinks the phone with this short id.
    pub fn remove_phone(&mut self, id: &str) -> Option<PhoneRecord> {
        let id = id.trim();
        if id.is_empty() {
            return None;
        }
        let at = self
            .phones
            .iter()
            .position(|p| phone_id(&p.device_id) == id)?;
        Some(self.phones.remove(at))
    }

    /// The installs a key's session requests go to: the phone that
    /// created it (none once that phone is unlinked), or every linked
    /// phone for keys from before keys had one.
    pub fn phones_for_key(&self, hash: &str) -> Vec<&str> {
        let owner = self
            .keys
            .iter()
            .find(|key| key.hash == hash)
            .and_then(|key| key.device_id.as_deref());
        self.phones
            .iter()
            .map(|p| p.device_id.as_str())
            .filter(|device| owner.is_none_or(|owner| owner == *device))
            .collect()
    }

    /// A phone moved to another install: its keys follow it.
    pub fn phone_moved(&mut self, from: &str, to: &str) {
        for key in &mut self.keys {
            if key.device_id.as_deref() == Some(from) {
                key.device_id = Some(to.to_string());
            }
        }
    }

    /// Records a visit of a phone; true when that changed what is stored.
    pub fn touch_phone(&mut self, index: usize, now: u64) -> bool {
        let phone = &mut self.phones[index];
        if now.saturating_sub(phone.seen_at) < PHONE_SEEN_EVERY_SECS {
            return false;
        }
        phone.seen_at = now;
        true
    }

    pub fn has_key(&self, hash: &str) -> bool {
        self.keys
            .iter()
            .any(|key| vtop_shared::tokens::hashes_equal(&key.hash, hash))
    }

    /// Records a use of a key; true when it was due to be written.
    pub fn touch_key(&mut self, hash: &str, now: u64) -> bool {
        let Some(key) = self.keys.iter_mut().find(|key| key.hash == hash) else {
            return false;
        };
        let due = key
            .last_used_at
            .is_none_or(|at| now >= at + vtop_shared::account::TOUCH_EVERY_SECS);
        if due {
            key.last_used_at = Some(now);
        }
        due
    }

    /// False when the account has the most keys allowed.
    pub fn add_key(
        &mut self,
        hash: String,
        label: String,
        device_id: Option<String>,
        now: u64,
    ) -> bool {
        if self.keys.len() >= MAX_KEYS_PER_ACCOUNT {
            return false;
        }
        self.keys.push(KeyRecord {
            hash,
            label,
            created_at: now,
            last_used_at: None,
            device_id,
        });
        true
    }

    /// Removes the key with this short id; its hash, for the index.
    pub fn revoke_key(&mut self, id: &str) -> Option<String> {
        let id = id.trim();
        if id.is_empty() {
            return None;
        }
        let at = self.keys.iter().position(|key| key_id(&key.hash) == id)?;
        Some(self.keys.remove(at).hash)
    }

    /// Counts a session call; false once the minute's allowance is used.
    pub fn allow(&mut self, now: u64) -> bool {
        let (start, hits) = self.window;
        if now >= start + 60 {
            self.window = (now, 1);
            return true;
        }
        self.window = (start, hits + 1);
        hits < ACCOUNT_RATE_PER_MINUTE
    }

    /// What `/v1/account` shows. Keys and the phone list only to a linked
    /// phone (`this_phone`, its index).
    pub fn view(&self, this_phone: Option<usize>, forwarding: bool, now: u64) -> Value {
        let phones: Vec<Value> = match this_phone {
            Some(me) => self
                .phones
                .iter()
                .enumerate()
                .map(|(i, phone)| {
                    json!({
                        "id": phone_id(&phone.device_id),
                        "label": phone.label,
                        "addedAt": phone.added_at,
                        "seenAt": phone.seen_at.max(phone.added_at),
                        "thisPhone": i == me,
                    })
                })
                .collect(),
            None => Vec::new(),
        };
        let keys: Vec<Value> = if this_phone.is_some() {
            self.keys
                .iter()
                .map(|key| {
                    json!({
                        "id": key_id(&key.hash),
                        "label": key.label,
                        "createdAt": key.created_at,
                        "lastUsedAt": key.last_used_at,
                        "phoneId": key.device_id.as_deref().map(phone_id),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        json!({
            "registrationNumber": self.id,
            "settings": self.settings,
            "semesterId": self.semester_id,
            "linked": self.linked(),
            "savedCredentials": self.usable_vault(now).is_some(),
            "vaultSavedAt": self.usable_vault(now).and(self.vault_saved_at),
            "credentialsRejectedAt": self.credentials_rejected_at,
            "forwarding": forwarding,
            "phoneLinked": self.linked(),
            "phoneCount": self.phones.len(),
            "thisPhone": this_phone.is_some(),
            "phones": phones,
            "keys": keys,
        })
    }
}

/// Who is asked for a session, in what order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    /// Sign in with saved credentials; ask the phone if that fails.
    VaultFirst,
    /// Ask the phone; sign in with saved credentials if it is silent for
    /// this long.
    PhoneFirst { wait_secs: u64 },
    /// Only the phone can answer.
    PhoneOnly,
}

pub fn plan(settings: &AccountSettings, has_vault: bool) -> Plan {
    match (has_vault, settings.always_use_vault) {
        (false, _) => Plan::PhoneOnly,
        (true, true) => Plan::VaultFirst,
        (true, false) => Plan::PhoneFirst {
            wait_secs: settings.phone_wait_secs,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    Pending,
    Success,
    Error,
}

/// One session request of an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestRecord {
    pub id: String,
    pub response_token: String,
    pub status: RequestStatus,
    /// The `Cookie` header, once answered.
    #[serde(default)]
    pub cookies: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    pub created_at: u64,
    pub expires_at: u64,
    /// When a sign-in with saved credentials is due (phone first) or
    /// started.
    #[serde(default)]
    pub vault_due_at: Option<u64>,
    #[serde(default)]
    pub vault_started: bool,
    #[serde(default)]
    pub phone_asked: bool,
    /// Why the phone could not be asked, kept to end the request with if
    /// the sign-in fails too.
    #[serde(default)]
    pub phone_error: Option<String>,
    /// Who made the session, once answered: `phone` or `server`.
    #[serde(default)]
    pub source: Option<String>,
    /// The installs this request may ask: the key's phone.
    #[serde(default)]
    pub phones: Vec<String>,
}

impl RequestRecord {
    pub fn new(id: String, response_token: String, now: u64) -> Self {
        Self {
            id,
            response_token,
            status: RequestStatus::Pending,
            cookies: None,
            error: None,
            created_at: now,
            expires_at: now + REQUEST_TIMEOUT_SECS,
            vault_due_at: None,
            vault_started: false,
            phone_asked: false,
            phone_error: None,
            source: None,
            phones: Vec::new(),
        }
    }

    pub fn pending(&self, now: u64) -> bool {
        self.status == RequestStatus::Pending && now < self.expires_at
    }

    /// Answers the request; `source` is who made the session.
    pub fn succeed(&mut self, cookies: String, source: &str) {
        self.status = RequestStatus::Success;
        self.cookies = Some(cookies);
        self.error = None;
        self.source = Some(source.to_string());
    }

    pub fn fail(&mut self, error: &str) {
        self.status = RequestStatus::Error;
        self.error = Some(error.to_string());
    }

    /// Who is working on a pending request, for clients to say what they
    /// wait for: the server signing in, the phone, or neither yet.
    pub fn stage(&self) -> &'static str {
        if self.vault_started {
            "server"
        } else if self.phone_asked {
            "phone"
        } else {
            "starting"
        }
    }

    /// The pending reply's progress fields. `serverDueAt` is when the
    /// server takes over from a phone that has not answered.
    fn progress(&self) -> Value {
        let due = self
            .vault_due_at
            .filter(|_| !self.vault_started)
            .map(rfc3339);
        json!({ "status": "pending", "stage": self.stage(), "serverDueAt": due })
    }

    /// The pending reply of `POST /v1/session`.
    pub fn started_view(&self) -> Value {
        let mut view = self.progress();
        view["requestId"] = json!(self.id);
        view["pollAfterMs"] = json!(POLL_AFTER_MS);
        view["expiresAt"] = json!(rfc3339(self.expires_at));
        view
    }

    /// The reply of a poll, and its status code.
    pub fn poll_view(&self, now: u64) -> (u16, Value) {
        if now >= self.expires_at && self.status == RequestStatus::Pending {
            return (
                410,
                json!({ "code": "request_expired", "error": "The session request expired." }),
            );
        }
        match self.status {
            RequestStatus::Pending => (202, self.progress()),
            RequestStatus::Success => (
                200,
                ready(
                    self.cookies.as_deref().unwrap_or_default(),
                    source_view(self.source.as_deref(), None, false),
                ),
            ),
            RequestStatus::Error => {
                let error = self.error.clone().unwrap_or_default();
                (
                    200,
                    json!({
                        "status": "error",
                        "code": vtop_shared::account::error_code(&error),
                        "error": error,
                    }),
                )
            }
        }
    }
}

/// A ready session as clients get it: VTOP's `SessionState` shape, the
/// cookies in Cookie-Editor form, and where the session came from.
pub fn ready(cookie_header: &str, source: Value) -> Value {
    json!({
        "status": "ready",
        "session": { "cookies": cookie_header },
        "cookies": cookies::from_header(cookie_header),
        "source": source,
    })
}

/// `from`: who signed in to VTOP (`phone`, `server`, or `unknown` for
/// sessions cached before this was recorded). `cached`: handed out again
/// from the cache. `madeAt`: when it was made, if known.
pub fn source_view(from: Option<&str>, made_at: Option<u64>, cached: bool) -> Value {
    json!({
        "from": from.unwrap_or("unknown"),
        "cached": cached,
        "madeAt": made_at.map(rfc3339),
    })
}

/// Unix seconds as RFC 3339 UTC, as the bridge writes `expiresAt`.
pub fn rfc3339(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil from days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_800_000_000;

    fn account() -> AccountRecord {
        AccountRecord::new("22BCE0001", NOW)
    }

    #[test]
    fn saved_credentials_follow_their_lifetime() {
        let mut a = account();
        assert!(a.usable_vault(NOW).is_none());
        assert!(a.want_credentials(NOW));
        a.save_vault("v".into(), NOW);
        assert_eq!(a.usable_vault(NOW + 10 * 86_400), Some("v"));
        assert!(!a.want_credentials(NOW));
        a.settings.vault_ttl_secs = Some(86_400);
        assert!(a.want_credentials(NOW));
        assert_eq!(a.usable_vault(NOW + 86_399), Some("v"));
        assert_eq!(a.usable_vault(NOW + 86_400), None);
    }

    #[test]
    fn rejected_credentials_are_dropped_until_new_ones_come() {
        let mut a = account();
        a.save_vault("v".into(), NOW);
        a.reject_credentials(NOW + 5);
        assert_eq!(a.usable_vault(NOW + 5), None);
        assert_eq!(a.credentials_rejected_at, Some(NOW + 5));
        assert_eq!(a.view(None, false, NOW)["credentialsRejectedAt"], NOW + 5);
        a.save_vault("w".into(), NOW + 9);
        assert_eq!(a.credentials_rejected_at, None);
    }

    #[test]
    fn sessions_are_cached_for_a_week() {
        let mut a = account();
        a.save_session("s".into(), NOW, "server");
        assert_eq!(a.cached_source()["from"], "server");
        assert_eq!(a.cached_source()["cached"], true);
        assert_eq!(a.cached_session(NOW + SESSION_CACHE_SECS - 1), Some("s"));
        assert_eq!(a.cached_session(NOW + SESSION_CACHE_SECS), None);
        a.clear_session();
        assert_eq!(a.cached_session(NOW), None);
    }

    #[test]
    fn plans() {
        let mut settings = AccountSettings::default();
        assert_eq!(plan(&settings, false), Plan::PhoneOnly);
        assert_eq!(plan(&settings, true), Plan::VaultFirst);
        settings.always_use_vault = false;
        assert_eq!(plan(&settings, true), Plan::PhoneFirst { wait_secs: 20 });
    }

    #[test]
    fn keys_limits_touches_and_revokes() {
        let mut a = account();
        for n in 0..MAX_KEYS_PER_ACCOUNT {
            assert!(a.add_key(format!("{n:02}abcdefgh"), "k".into(), None, NOW));
        }
        assert!(!a.add_key("x".into(), "k".into(), None, NOW));
        assert!(a.has_key("03abcdefgh"));
        assert!(a.touch_key("03abcdefgh", NOW));
        assert!(!a.touch_key("03abcdefgh", NOW + 59));
        assert!(a.touch_key("03abcdefgh", NOW + 60));
        assert_eq!(a.revoke_key("03abcdef").as_deref(), Some("03abcdefgh"));
        assert!(!a.has_key("03abcdefgh"));
        assert_eq!(a.revoke_key(""), None);
        a.add_phone(phone("d1", "vta_x"));
        let view = a.view(Some(0), true, NOW);
        assert_eq!(
            view["keys"].as_array().unwrap().len(),
            MAX_KEYS_PER_ACCOUNT - 1
        );
        assert_eq!(view["keys"][0]["id"], "00abcdef");
        assert!(a.view(None, true, NOW)["keys"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    fn phone(device: &str, secret: &str) -> PhoneRecord {
        PhoneRecord {
            device_id: device.into(),
            secret_hash: vtop_shared::tokens::hash(secret),
            label: None,
            added_at: NOW,
            seen_at: NOW,
        }
    }

    #[test]
    fn app_secret() {
        let mut a = account();
        assert_eq!(a.phone_by_secret(Some("vta_x")), None);
        a.add_phone(phone("d1", "vta_x"));
        a.add_phone(phone("d2", "vta_y"));
        assert_eq!(a.phone_by_secret(Some(" vta_x ")), Some(0));
        assert_eq!(a.phone_by_secret(Some("vta_y")), Some(1));
        assert_eq!(a.phone_by_secret(Some("vta_z")), None);
        assert_eq!(a.phone_by_secret(None), None);
    }

    #[test]
    fn several_phones_link_and_unlink() {
        let mut a = account();
        for n in 0..MAX_PHONES {
            let mut p = phone(&format!("d{n}"), &format!("vta_{n}"));
            p.seen_at = NOW + n as u64;
            a.add_phone(p);
        }
        // Linking the same install again replaces it, not adds.
        a.add_phone(phone("d3", "vta_new"));
        assert_eq!(a.phones.len(), MAX_PHONES);
        assert_eq!(a.phone_by_secret(Some("vta_3")), None);
        // One more drops the least recently seen (d0).
        a.add_phone(phone("d9", "vta_9"));
        assert_eq!(a.phones.len(), MAX_PHONES);
        assert!(a.phones.iter().all(|p| p.device_id != "d0"));
        let view = a.view(a.phone_by_secret(Some("vta_9")), false, NOW);
        assert_eq!(view["phones"].as_array().unwrap().len(), MAX_PHONES);
        assert_eq!(view["phones"][MAX_PHONES - 1]["thisPhone"], true);
        assert!(a.view(None, false, NOW)["phones"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(a.view(None, false, NOW)["phoneCount"], MAX_PHONES);
        let removed = a.remove_phone(&phone_id("d9")).unwrap();
        assert_eq!(removed.device_id, "d9");
        assert_eq!(a.remove_phone(&phone_id("d9")), None);
        assert_eq!(a.remove_phone(""), None);
        assert!(a.touch_phone(0, NOW + 7200));
        assert!(!a.touch_phone(0, NOW + 7201));
    }

    #[test]
    fn a_key_asks_only_the_phone_that_made_it() {
        let mut a = account();
        a.add_phone(phone("d1", "vta_1"));
        a.add_phone(phone("d2", "vta_2"));
        a.add_key("mine".into(), "k".into(), Some("d2".into()), NOW);
        a.add_key("old".into(), "k".into(), None, NOW);
        assert_eq!(a.phones_for_key("mine"), vec!["d2"]);
        assert_eq!(a.phones_for_key("old"), vec!["d1", "d2"]);
        assert_eq!(
            a.view(Some(0), false, NOW)["keys"][0]["phoneId"],
            phone_id("d2")
        );
        a.phone_moved("d2", "d3");
        a.phones[1].device_id = "d3".into();
        assert_eq!(a.phones_for_key("mine"), vec!["d3"]);
        a.remove_phone(&phone_id("d3"));
        assert!(a.phones_for_key("mine").is_empty());
    }

    #[test]
    fn the_one_phone_of_older_accounts_carries_over() {
        let mut a = account();
        a.app_secret_hash = Some(vtop_shared::tokens::hash("vta_x"));
        a.device_id = Some("d1".into());
        assert!(a.adopt_legacy_phone());
        assert_eq!(a.phone_by_secret(Some("vta_x")), Some(0));
        assert_eq!(a.phones[0].device_id, "d1");
        assert_eq!(
            (a.app_secret_hash.clone(), a.device_id.clone()),
            (None, None)
        );
        assert!(!a.adopt_legacy_phone());
        // Without an install there is nothing to ask: it links again.
        a.app_secret_hash = Some("h".into());
        assert!(a.adopt_legacy_phone());
        assert_eq!(a.phones.len(), 1);
    }

    #[test]
    fn rate_limit_resets_each_minute() {
        let mut a = account();
        for _ in 0..ACCOUNT_RATE_PER_MINUTE {
            assert!(a.allow(NOW));
        }
        assert!(!a.allow(NOW + 59));
        assert!(a.allow(NOW + 60));
    }

    #[test]
    fn request_views() {
        let mut r = RequestRecord::new("r".into(), "h".into(), NOW);
        assert!(r.pending(NOW));
        assert_eq!(r.poll_view(NOW).0, 202);
        assert_eq!(r.poll_view(NOW).1["stage"], "starting");
        r.phone_asked = true;
        r.vault_due_at = Some(NOW + 10);
        assert_eq!(r.poll_view(NOW).1["stage"], "phone");
        assert_eq!(r.started_view()["serverDueAt"], rfc3339(NOW + 10));
        r.vault_started = true;
        assert_eq!(r.poll_view(NOW).1["stage"], "server");
        assert!(r.poll_view(NOW).1["serverDueAt"].is_null());
        assert_eq!(r.poll_view(NOW + REQUEST_TIMEOUT_SECS).0, 410);
        r.fail(vtop_shared::account::PHONE_UNREACHABLE);
        let (status, body) = r.poll_view(NOW);
        assert_eq!(status, 200);
        assert_eq!(body["code"], "phone_unreachable");
        r.succeed("JSESSIONID=a".into(), "phone");
        let (_, body) = r.poll_view(NOW + 1000);
        assert_eq!(body["status"], "ready");
        assert_eq!(body["session"]["cookies"], "JSESSIONID=a");
        assert_eq!(body["cookies"][0]["name"], "JSESSIONID");
        assert_eq!(r.started_view()["pollAfterMs"], POLL_AFTER_MS);
    }

    /// The shape `accounts-to-edge` (vtop-bridge's `edge_import`) sends.
    #[test]
    fn reads_imported_accounts() {
        let record: AccountRecord = serde_json::from_value(json!({
            "id": "22BCE0001",
            "app_secret_hash": "h",
            "settings": {"alwaysUseVault": false, "phoneWaitSecs": 45, "vaultTtlSecs": null},
            "semester_id": null,
            "sealed_vault": "v",
            "vault_saved_at": 5,
            "sealed_session": null,
            "session_saved_at": null,
            "keys": [{"hash": "abc", "label": "Claude", "created_at": 1, "last_used_at": null}],
            "created_at": 7,
            "updated_at": 7,
        }))
        .unwrap();
        assert_eq!(record.settings.phone_wait_secs, 45);
        assert_eq!(record.usable_vault(NOW), Some("v"));
        assert_eq!(record.device_id, None);
        assert!(record.has_key("abc"));
    }

    #[test]
    fn rfc3339_dates() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_709_210_096), "2024-02-29T12:34:56Z");
    }
}

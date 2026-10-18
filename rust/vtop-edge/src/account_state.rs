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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRecord {
    pub hash: String,
    pub label: String,
    pub created_at: u64,
    #[serde(default)]
    pub last_used_at: Option<u64>,
}

/// One student's account. Secrets are sealed by the engine with a key the
/// edge never has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRecord {
    pub id: String,
    #[serde(default)]
    pub app_secret_hash: Option<String>,
    #[serde(default)]
    pub settings: AccountSettings,
    #[serde(default)]
    pub semester_id: Option<String>,
    /// The install pushes go to.
    #[serde(default)]
    pub device_id: Option<String>,
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
            mailbox_id: None,
            sealed_vault: None,
            vault_saved_at: None,
            sealed_session: None,
            session_saved_at: None,
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

    pub fn save_session(&mut self, sealed: String, now: u64) {
        self.sealed_session = Some(sealed);
        self.session_saved_at = Some(now);
    }

    pub fn clear_session(&mut self) {
        self.sealed_session = None;
        self.session_saved_at = None;
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

    pub fn app_secret_matches(&self, presented: Option<&str>) -> bool {
        match (&self.app_secret_hash, presented.map(str::trim)) {
            (Some(expected), Some(secret)) if !secret.is_empty() => {
                vtop_shared::tokens::hashes_equal(expected, &vtop_shared::tokens::hash(secret))
            }
            _ => false,
        }
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
    pub fn add_key(&mut self, hash: String, label: String, now: u64) -> bool {
        if self.keys.len() >= MAX_KEYS_PER_ACCOUNT {
            return false;
        }
        self.keys.push(KeyRecord {
            hash,
            label,
            created_at: now,
            last_used_at: None,
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

    /// What `/v1/account` shows. Keys only to the linked phone.
    pub fn view(&self, this_phone: bool, forwarding: bool, now: u64) -> Value {
        let keys: Vec<Value> = if this_phone {
            self.keys
                .iter()
                .map(|key| {
                    json!({
                        "id": key_id(&key.hash),
                        "label": key.label,
                        "createdAt": key.created_at,
                        "lastUsedAt": key.last_used_at,
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
            "linked": self.app_secret_hash.is_some(),
            "savedCredentials": self.usable_vault(now).is_some(),
            "vaultSavedAt": self.usable_vault(now).and(self.vault_saved_at),
            "credentialsRejectedAt": self.credentials_rejected_at,
            "forwarding": forwarding,
            "phoneLinked": self.device_id.is_some(),
            "thisPhone": this_phone,
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
        }
    }

    pub fn pending(&self, now: u64) -> bool {
        self.status == RequestStatus::Pending && now < self.expires_at
    }

    pub fn succeed(&mut self, cookies: String) {
        self.status = RequestStatus::Success;
        self.cookies = Some(cookies);
        self.error = None;
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
            RequestStatus::Success => (200, ready(self.cookies.as_deref().unwrap_or_default())),
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

/// A ready session as clients get it: VTOP's `SessionState` shape and the
/// cookies in Cookie-Editor form.
pub fn ready(cookie_header: &str) -> Value {
    json!({
        "status": "ready",
        "session": { "cookies": cookie_header },
        "cookies": cookies::from_header(cookie_header),
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
        assert_eq!(a.view(false, false, NOW)["credentialsRejectedAt"], NOW + 5);
        a.save_vault("w".into(), NOW + 9);
        assert_eq!(a.credentials_rejected_at, None);
    }

    #[test]
    fn sessions_are_cached_for_a_week() {
        let mut a = account();
        a.save_session("s".into(), NOW);
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
            assert!(a.add_key(format!("{n:02}abcdefgh"), "k".into(), NOW));
        }
        assert!(!a.add_key("x".into(), "k".into(), NOW));
        assert!(a.has_key("03abcdefgh"));
        assert!(a.touch_key("03abcdefgh", NOW));
        assert!(!a.touch_key("03abcdefgh", NOW + 59));
        assert!(a.touch_key("03abcdefgh", NOW + 60));
        assert_eq!(a.revoke_key("03abcdef").as_deref(), Some("03abcdefgh"));
        assert!(!a.has_key("03abcdefgh"));
        assert_eq!(a.revoke_key(""), None);
        let view = a.view(true, true, NOW);
        assert_eq!(
            view["keys"].as_array().unwrap().len(),
            MAX_KEYS_PER_ACCOUNT - 1
        );
        assert_eq!(view["keys"][0]["id"], "00abcdef");
        assert!(a.view(false, true, NOW)["keys"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn app_secret() {
        let mut a = account();
        assert!(!a.app_secret_matches(Some("vta_x")));
        a.app_secret_hash = Some(vtop_shared::tokens::hash("vta_x"));
        assert!(a.app_secret_matches(Some(" vta_x ")));
        assert!(!a.app_secret_matches(Some("vta_y")));
        assert!(!a.app_secret_matches(None));
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
        r.succeed("JSESSIONID=a".into());
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

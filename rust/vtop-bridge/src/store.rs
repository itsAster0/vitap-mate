//! Persistent state: cookie requests, accounts and the FCM-token index.
//! Production uses Firestore ([`crate::firestore::FirestoreStore`]); tests
//! use [`MemoryStore`]. Account and index values arrive here already sealed.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use subtle::ConstantTimeEq;

pub const REQUESTS: &str = "bridge_requests";
pub const ACCOUNTS: &str = "accounts";
pub const FCM_INDEX: &str = "fcm_index";
pub const ACCESS_KEYS: &str = "access_keys";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestStatus {
    Pending,
    Success,
    Error,
}

impl RequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Success => "success",
            Self::Error => "error",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "success" => Some(Self::Success),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// A cookie request. No `Debug`: it holds the response token and cookies.
#[derive(Clone, PartialEq, Eq)]
pub struct RequestDoc {
    pub id: String,
    pub response_token: Option<String>,
    pub status: RequestStatus,
    /// Cookie-editor cookies as a JSON array.
    pub cookies_json: Option<String>,
    pub error: Option<String>,
    /// SHA-256 hex of the FCM token the request was sent to.
    pub fcm_hash: Option<String>,
    pub want_credentials: bool,
    /// The account (registration number) a key-authenticated request is
    /// for. `None` for legacy FCM-token requests.
    pub account: Option<String>,
    pub created_at: u64,
    pub expires_at: u64,
}

/// How the bridge serves an account's sessions; the student picks these in
/// the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSettings {
    /// Sign in with saved credentials as soon as a session is needed,
    /// without asking the phone first.
    pub always_use_vault: bool,
    /// When not `always_use_vault`: how long the phone gets to answer
    /// before the server signs in with saved credentials.
    pub phone_wait_secs: u64,
    /// How long saved credentials are kept after they were last sent;
    /// `None` keeps them until VTOP or Google rejects them.
    pub vault_ttl_secs: Option<u64>,
}

impl Default for AccountSettings {
    fn default() -> Self {
        Self {
            always_use_vault: true,
            phone_wait_secs: 20,
            vault_ttl_secs: None,
        }
    }
}

impl AccountSettings {
    pub const PHONE_WAIT_CHOICES: [u64; 3] = [10, 20, 45];
    pub const VAULT_TTL_CHOICES: [Option<u64>; 3] = [Some(86_400), Some(172_800), None];

    pub fn is_valid(&self) -> bool {
        Self::PHONE_WAIT_CHOICES.contains(&self.phone_wait_secs)
            && Self::VAULT_TTL_CHOICES.contains(&self.vault_ttl_secs)
    }
}

/// An account. Every secret field is sealed. No `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct AccountDoc {
    /// Upper-cased VTOP username; also the document id.
    pub username: String,
    pub vault: Option<String>,
    pub session: Option<String>,
    pub session_expires_at: Option<u64>,
    pub fcm_token: Option<String>,
    /// SHA-256 hex of the secret the linked app holds.
    pub app_secret_hash: Option<String>,
    /// When the saved credentials expire; `None` keeps them.
    pub vault_expires_at: Option<u64>,
    /// `None` until the student changes something.
    pub settings: Option<AccountSettings>,
    pub updated_at: u64,
}

impl AccountDoc {
    pub fn empty(username: &str, now: u64) -> Self {
        Self {
            username: username.to_string(),
            vault: None,
            session: None,
            session_expires_at: None,
            fcm_token: None,
            app_secret_hash: None,
            vault_expires_at: None,
            settings: None,
            updated_at: now,
        }
    }
}

/// An access key, stored by the SHA-256 hex of the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyDoc {
    pub hash: String,
    pub registration_number: String,
    pub label: String,
    pub created_at: u64,
    pub last_used_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError(pub String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompleteError {
    NotFound,
    BadToken,
    Expired,
    AlreadyHandled,
    Store(StoreError),
}

/// The outcome a callback writes into a pending request.
pub struct Completion {
    pub status: RequestStatus,
    pub cookies_json: Option<String>,
    pub error: Option<String>,
}

#[async_trait]
pub trait Store: Send + Sync {
    /// Fails if a request with this id exists.
    async fn create_request(&self, doc: &RequestDoc) -> Result<(), StoreError>;
    async fn get_request(&self, id: &str) -> Result<Option<RequestDoc>, StoreError>;
    /// Atomically checks the response token (constant time), expiry and
    /// `Pending`, then writes `completion` and drops the response token.
    async fn complete_request(
        &self,
        id: &str,
        token: &str,
        completion: Completion,
        now: u64,
    ) -> Result<RequestDoc, CompleteError>;
    /// A missing request is not an error.
    async fn delete_request(&self, id: &str) -> Result<(), StoreError>;
    async fn get_account(&self, username: &str) -> Result<Option<AccountDoc>, StoreError>;
    /// Overwrites the whole document.
    async fn put_account(&self, doc: &AccountDoc) -> Result<(), StoreError>;
    async fn username_for_fcm(&self, fcm_hash: &str) -> Result<Option<String>, StoreError>;
    async fn put_fcm_index(&self, fcm_hash: &str, username: &str) -> Result<(), StoreError>;
    async fn health(&self) -> Result<(), StoreError>;
    /// Creates or replaces a key.
    async fn put_key(&self, doc: &KeyDoc) -> Result<(), StoreError>;
    async fn get_key(&self, hash: &str) -> Result<Option<KeyDoc>, StoreError>;
    async fn list_keys(&self, registration_number: &str) -> Result<Vec<KeyDoc>, StoreError>;
    /// A missing key is not an error.
    async fn delete_key(&self, hash: &str) -> Result<(), StoreError>;
    /// Removes the account, its keys and its requests.
    async fn delete_account(&self, registration_number: &str) -> Result<(), StoreError>;
}

pub fn tokens_equal(expected: &str, actual: &str) -> bool {
    expected.len() == actual.len() && bool::from(expected.as_bytes().ct_eq(actual.as_bytes()))
}

/// Applies the callback checks shared by every store.
pub fn apply_completion(
    doc: &mut RequestDoc,
    token: &str,
    completion: Completion,
    now: u64,
) -> Result<(), CompleteError> {
    // A completed request has no token left, so a replay fails here.
    let expected = doc
        .response_token
        .as_deref()
        .ok_or(CompleteError::AlreadyHandled)?;
    if !tokens_equal(expected, token) {
        return Err(CompleteError::BadToken);
    }
    if now >= doc.expires_at {
        return Err(CompleteError::Expired);
    }
    if doc.status != RequestStatus::Pending {
        return Err(CompleteError::AlreadyHandled);
    }
    doc.status = completion.status;
    doc.cookies_json = completion.cookies_json;
    doc.error = completion.error;
    doc.response_token = None;
    Ok(())
}

#[derive(Default)]
pub struct MemoryStore {
    requests: Mutex<HashMap<String, RequestDoc>>,
    accounts: Mutex<HashMap<String, AccountDoc>>,
    fcm_index: Mutex<HashMap<String, String>>,
    keys: Mutex<HashMap<String, KeyDoc>>,
    /// When set, every call fails with this error.
    pub fail: Mutex<Option<String>>,
}

impl MemoryStore {
    fn check(&self) -> Result<(), StoreError> {
        match self.fail.lock().unwrap().clone() {
            Some(message) => Err(StoreError(message)),
            None => Ok(()),
        }
    }
}

#[async_trait]
impl Store for MemoryStore {
    async fn create_request(&self, doc: &RequestDoc) -> Result<(), StoreError> {
        self.check()?;
        let mut requests = self.requests.lock().unwrap();
        if requests.contains_key(&doc.id) {
            return Err(StoreError(format!("request {} exists", doc.id)));
        }
        requests.insert(doc.id.clone(), doc.clone());
        Ok(())
    }

    async fn get_request(&self, id: &str) -> Result<Option<RequestDoc>, StoreError> {
        self.check()?;
        Ok(self.requests.lock().unwrap().get(id).cloned())
    }

    async fn complete_request(
        &self,
        id: &str,
        token: &str,
        completion: Completion,
        now: u64,
    ) -> Result<RequestDoc, CompleteError> {
        self.check().map_err(CompleteError::Store)?;
        let mut requests = self.requests.lock().unwrap();
        let doc = requests.get_mut(id).ok_or(CompleteError::NotFound)?;
        let mut updated = doc.clone();
        apply_completion(&mut updated, token, completion, now)?;
        *doc = updated.clone();
        Ok(updated)
    }

    async fn delete_request(&self, id: &str) -> Result<(), StoreError> {
        self.check()?;
        self.requests.lock().unwrap().remove(id);
        Ok(())
    }

    async fn get_account(&self, username: &str) -> Result<Option<AccountDoc>, StoreError> {
        self.check()?;
        Ok(self.accounts.lock().unwrap().get(username).cloned())
    }

    async fn put_account(&self, doc: &AccountDoc) -> Result<(), StoreError> {
        self.check()?;
        self.accounts
            .lock()
            .unwrap()
            .insert(doc.username.clone(), doc.clone());
        Ok(())
    }

    async fn username_for_fcm(&self, fcm_hash: &str) -> Result<Option<String>, StoreError> {
        self.check()?;
        Ok(self.fcm_index.lock().unwrap().get(fcm_hash).cloned())
    }

    async fn put_fcm_index(&self, fcm_hash: &str, username: &str) -> Result<(), StoreError> {
        self.check()?;
        self.fcm_index
            .lock()
            .unwrap()
            .insert(fcm_hash.to_string(), username.to_string());
        Ok(())
    }

    async fn health(&self) -> Result<(), StoreError> {
        self.check()
    }

    async fn put_key(&self, doc: &KeyDoc) -> Result<(), StoreError> {
        self.check()?;
        self.keys
            .lock()
            .unwrap()
            .insert(doc.hash.clone(), doc.clone());
        Ok(())
    }

    async fn get_key(&self, hash: &str) -> Result<Option<KeyDoc>, StoreError> {
        self.check()?;
        Ok(self.keys.lock().unwrap().get(hash).cloned())
    }

    async fn list_keys(&self, registration_number: &str) -> Result<Vec<KeyDoc>, StoreError> {
        self.check()?;
        Ok(self
            .keys
            .lock()
            .unwrap()
            .values()
            .filter(|key| key.registration_number == registration_number)
            .cloned()
            .collect())
    }

    async fn delete_key(&self, hash: &str) -> Result<(), StoreError> {
        self.check()?;
        self.keys.lock().unwrap().remove(hash);
        Ok(())
    }

    async fn delete_account(&self, registration_number: &str) -> Result<(), StoreError> {
        self.check()?;
        self.accounts.lock().unwrap().remove(registration_number);
        self.keys
            .lock()
            .unwrap()
            .retain(|_, key| key.registration_number != registration_number);
        self.requests
            .lock()
            .unwrap()
            .retain(|_, request| request.account.as_deref() != Some(registration_number));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(id: &str, expires_at: u64) -> RequestDoc {
        RequestDoc {
            id: id.into(),
            response_token: Some("token".into()),
            status: RequestStatus::Pending,
            cookies_json: None,
            error: None,
            fcm_hash: Some("hash".into()),
            want_credentials: true,
            account: None,
            created_at: 100,
            expires_at,
        }
    }

    fn success() -> Completion {
        Completion {
            status: RequestStatus::Success,
            cookies_json: Some("[]".into()),
            error: None,
        }
    }

    #[tokio::test]
    async fn create_then_get_and_duplicate_create_fails() {
        let store = MemoryStore::default();
        store.create_request(&pending("r", 200)).await.unwrap();
        assert!(store.get_request("r").await.unwrap() == Some(pending("r", 200)));
        assert!(store.create_request(&pending("r", 200)).await.is_err());
        assert!(store.get_request("missing").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn complete_rejects_bad_token() {
        let store = MemoryStore::default();
        store.create_request(&pending("r", 200)).await.unwrap();
        let result = store.complete_request("r", "wrong", success(), 150).await;
        assert_eq!(result.err(), Some(CompleteError::BadToken));
        let doc = store.get_request("r").await.unwrap().unwrap();
        assert_eq!(doc.status, RequestStatus::Pending);
    }

    #[tokio::test]
    async fn complete_rejects_missing() {
        let store = MemoryStore::default();
        let result = store.complete_request("r", "token", success(), 150).await;
        assert_eq!(result.err(), Some(CompleteError::NotFound));
    }

    #[tokio::test]
    async fn complete_rejects_expired() {
        let store = MemoryStore::default();
        store.create_request(&pending("r", 200)).await.unwrap();
        let result = store.complete_request("r", "token", success(), 200).await;
        assert_eq!(result.err(), Some(CompleteError::Expired));
    }

    #[tokio::test]
    async fn complete_twice_is_already_handled() {
        let store = MemoryStore::default();
        store.create_request(&pending("r", 200)).await.unwrap();
        store
            .complete_request("r", "token", success(), 150)
            .await
            .unwrap();
        // The token is gone after the first callback, so a replay with the
        // original token no longer matches.
        let second = store.complete_request("r", "token", success(), 151).await;
        assert!(matches!(
            second.err(),
            Some(CompleteError::AlreadyHandled) | Some(CompleteError::BadToken)
        ));
        let doc = store.get_request("r").await.unwrap().unwrap();
        assert_eq!(doc.cookies_json.as_deref(), Some("[]"));
    }

    #[tokio::test]
    async fn complete_writes_outcome_and_clears_response_token() {
        let store = MemoryStore::default();
        store.create_request(&pending("r", 200)).await.unwrap();
        let doc = store
            .complete_request("r", "token", success(), 150)
            .await
            .unwrap();
        assert_eq!(doc.status, RequestStatus::Success);
        assert_eq!(doc.cookies_json.as_deref(), Some("[]"));
        assert!(doc.response_token.is_none());
        let error = Completion {
            status: RequestStatus::Error,
            cookies_json: None,
            error: Some("no account".into()),
        };
        store.create_request(&pending("e", 200)).await.unwrap();
        let doc = store
            .complete_request("e", "token", error, 150)
            .await
            .unwrap();
        assert_eq!(doc.status, RequestStatus::Error);
        assert_eq!(doc.error.as_deref(), Some("no account"));
    }

    #[tokio::test]
    async fn delete_missing_is_ok() {
        let store = MemoryStore::default();
        store.delete_request("missing").await.unwrap();
        store.create_request(&pending("r", 200)).await.unwrap();
        store.delete_request("r").await.unwrap();
        assert!(store.get_request("r").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn account_round_trip_and_fcm_index() {
        let store = MemoryStore::default();
        assert!(store.get_account("A").await.unwrap().is_none());
        let mut account = AccountDoc::empty("A", 5);
        account.vault = Some("sealed".into());
        store.put_account(&account).await.unwrap();
        assert!(store.get_account("A").await.unwrap() == Some(account));
        assert_eq!(store.username_for_fcm("h").await.unwrap(), None);
        store.put_fcm_index("h", "A").await.unwrap();
        assert_eq!(
            store.username_for_fcm("h").await.unwrap().as_deref(),
            Some("A")
        );
    }

    #[tokio::test]
    async fn failing_store_reports_errors() {
        let store = MemoryStore::default();
        *store.fail.lock().unwrap() = Some("down".into());
        assert!(store.health().await.is_err());
        assert!(store.get_account("A").await.is_err());
    }

    fn key(hash: &str, account: &str) -> KeyDoc {
        KeyDoc {
            hash: hash.into(),
            registration_number: account.into(),
            label: "laptop".into(),
            created_at: 1,
            last_used_at: None,
        }
    }

    #[tokio::test]
    async fn keys_round_trip_and_list_by_account() {
        let store = MemoryStore::default();
        store.put_key(&key("h1", "A")).await.unwrap();
        store.put_key(&key("h2", "A")).await.unwrap();
        store.put_key(&key("h3", "B")).await.unwrap();
        assert_eq!(store.get_key("h1").await.unwrap(), Some(key("h1", "A")));
        assert_eq!(store.get_key("nope").await.unwrap(), None);
        let mut hashes: Vec<String> = store
            .list_keys("A")
            .await
            .unwrap()
            .into_iter()
            .map(|k| k.hash)
            .collect();
        hashes.sort();
        assert_eq!(hashes, ["h1", "h2"]);
        store.delete_key("h1").await.unwrap();
        store.delete_key("h1").await.unwrap();
        assert_eq!(store.list_keys("A").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn delete_account_removes_keys_and_requests() {
        let store = MemoryStore::default();
        store.put_account(&AccountDoc::empty("A", 1)).await.unwrap();
        store.put_account(&AccountDoc::empty("B", 1)).await.unwrap();
        store.put_key(&key("ha", "A")).await.unwrap();
        store.put_key(&key("hb", "B")).await.unwrap();
        let mine = RequestDoc {
            account: Some("A".into()),
            ..pending("ra", 200)
        };
        let theirs = RequestDoc {
            account: Some("B".into()),
            ..pending("rb", 200)
        };
        store.create_request(&mine).await.unwrap();
        store.create_request(&theirs).await.unwrap();

        store.delete_account("A").await.unwrap();

        assert!(store.get_account("A").await.unwrap().is_none());
        assert!(store.get_key("ha").await.unwrap().is_none());
        assert!(store.get_request("ra").await.unwrap().is_none());
        assert!(store.get_account("B").await.unwrap().is_some());
        assert!(store.get_key("hb").await.unwrap().is_some());
        assert!(store.get_request("rb").await.unwrap().is_some());
    }

    #[test]
    fn tokens_compare_exactly() {
        assert!(tokens_equal("abc", "abc"));
        assert!(!tokens_equal("abc", "abd"));
        assert!(!tokens_equal("abc", "ab"));
    }
}

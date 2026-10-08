//! Accounts: the sealed vault, the cached session and the phone's FCM
//! token, keyed by the registration number VTOP confirmed.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use vtop_core::SessionState;

use crate::gmail::GmailGrant;
pub use crate::store::AccountSettings;
use crate::store::{AccountDoc, Store, StoreError, ACCOUNTS};
use crate::vault::Sealer;

/// What lets the bridge log in without the phone. No `Debug`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vault {
    /// The VTOP login name; it can change, the registration number cannot.
    pub username: String,
    pub password: String,
    pub gmail: GmailGrant,
}

pub struct Accounts {
    store: Arc<dyn Store>,
    sealer: Arc<Sealer>,
    session_ttl: Duration,
}

/// When a vault saved now expires under the account's settings.
fn vault_expiry(doc: &AccountDoc, now: u64) -> Option<u64> {
    let ttl = doc.settings.unwrap_or_default().vault_ttl_secs?;
    doc.vault.as_ref().map(|_| now + ttl)
}

/// Account ids are upper-cased registration numbers.
pub fn account_id(registration_number: &str) -> String {
    registration_number.trim().to_uppercase()
}

impl Accounts {
    pub fn new(store: Arc<dyn Store>, sealer: Arc<Sealer>, session_ttl: Duration) -> Self {
        Self {
            store,
            sealer,
            session_ttl,
        }
    }

    fn aad(id: &str) -> String {
        format!("{ACCOUNTS}/{id}")
    }

    async fn load(&self, id: &str) -> Result<Option<AccountDoc>, StoreError> {
        self.store.get_account(id).await
    }

    /// Opens `field` of the account. A value that no longer opens (the vault
    /// key was rotated) is removed and read as absent.
    async fn open<T: serde::de::DeserializeOwned>(
        &self,
        id: &str,
        field: fn(&mut AccountDoc) -> &mut Option<String>,
    ) -> Result<Option<(AccountDoc, T)>, StoreError> {
        let Some(mut doc) = self.load(id).await? else {
            return Ok(None);
        };
        let Some(sealed) = field(&mut doc).clone() else {
            return Ok(None);
        };
        match self.sealer.open_json::<T>(&Self::aad(id), &sealed) {
            Ok(value) => Ok(Some((doc, value))),
            Err(_) => {
                tracing::warn!("account {id}: a sealed value no longer opens; removing it");
                *field(&mut doc) = None;
                self.store.put_account(&doc).await?;
                Ok(None)
            }
        }
    }

    pub async fn fcm_token(&self, username: &str) -> Result<Option<String>, StoreError> {
        let id = account_id(username);
        Ok(self
            .open::<String>(&id, |doc| &mut doc.fcm_token)
            .await?
            .map(|(_, token)| token))
    }

    /// The cached session while it is younger than the cache lifetime.
    pub async fn cached_session(
        &self,
        username: &str,
        now: u64,
    ) -> Result<Option<SessionState>, StoreError> {
        let id = account_id(username);
        let Some((doc, session)) = self
            .open::<SessionState>(&id, |doc| &mut doc.session)
            .await?
        else {
            return Ok(None);
        };
        Ok(doc
            .session_expires_at
            .filter(|expires_at| now < *expires_at)
            .map(|_| session))
    }

    /// The saved credentials; expired ones are removed and read as absent.
    pub async fn vault(&self, username: &str) -> Result<Option<Vault>, StoreError> {
        let id = account_id(username);
        let Some((mut doc, vault)) = self.open::<Vault>(&id, |doc| &mut doc.vault).await? else {
            return Ok(None);
        };
        if doc
            .vault_expires_at
            .is_some_and(|expires_at| vtop_core::now_unix() >= expires_at)
        {
            tracing::info!("account {id}: saved credentials expired; removing them");
            doc.vault = None;
            doc.vault_expires_at = None;
            self.store.put_account(&doc).await?;
            return Ok(None);
        }
        Ok(Some(vault))
    }

    pub async fn settings(&self, username: &str) -> Result<AccountSettings, StoreError> {
        Ok(self
            .load(&account_id(username))
            .await?
            .and_then(|doc| doc.settings)
            .unwrap_or_default())
    }

    /// Stores the settings. Saved credentials restart their timer under the
    /// new expiry.
    pub async fn set_settings(
        &self,
        username: &str,
        settings: AccountSettings,
        now: u64,
    ) -> Result<(), StoreError> {
        self.update(username, now, |doc, _, _| {
            doc.settings = Some(settings);
            doc.vault_expires_at = vault_expiry(doc, now);
        })
        .await
    }

    async fn update(
        &self,
        username: &str,
        now: u64,
        change: impl FnOnce(&mut AccountDoc, &Sealer, &str),
    ) -> Result<(), StoreError> {
        let id = account_id(username);
        let mut doc = self
            .load(&id)
            .await?
            .unwrap_or_else(|| AccountDoc::empty(&id, now));
        change(&mut doc, &self.sealer, &Self::aad(&id));
        doc.updated_at = now;
        self.store.put_account(&doc).await
    }

    pub async fn save_session(
        &self,
        username: &str,
        session: &SessionState,
        now: u64,
    ) -> Result<(), StoreError> {
        let expires_at = now + self.session_ttl.as_secs();
        self.update(username, now, |doc, sealer, aad| {
            doc.session = Some(sealer.seal_json(aad, session));
            doc.session_expires_at = Some(expires_at);
        })
        .await
    }

    pub async fn clear_session(&self, username: &str) -> Result<(), StoreError> {
        if self.load(&account_id(username)).await?.is_none() {
            return Ok(());
        }
        self.update(username, vtop_core::now_unix(), |doc, _, _| {
            doc.session = None;
            doc.session_expires_at = None;
        })
        .await
    }

    pub async fn clear_vault(&self, username: &str) -> Result<(), StoreError> {
        if self.load(&account_id(username)).await?.is_none() {
            return Ok(());
        }
        self.update(username, vtop_core::now_unix(), |doc, _, _| {
            doc.vault = None;
            doc.vault_expires_at = None;
        })
        .await
    }

    /// Caches a verified session; stores the FCM token when given and the
    /// vault only when `vault` is given.
    pub async fn record_phone(
        &self,
        registration_number: &str,
        fcm_token: Option<&str>,
        session: &SessionState,
        vault: Option<Vault>,
        now: u64,
    ) -> Result<(), StoreError> {
        let expires_at = now + self.session_ttl.as_secs();
        self.update(registration_number, now, |doc, sealer, aad| {
            doc.session = Some(sealer.seal_json(aad, session));
            doc.session_expires_at = Some(expires_at);
            if let Some(fcm_token) = fcm_token {
                doc.fcm_token = Some(sealer.seal_json(aad, &fcm_token));
            }
            if let Some(vault) = &vault {
                doc.vault = Some(sealer.seal_json(aad, vault));
                doc.vault_expires_at = vault_expiry(doc, now);
            }
        })
        .await
    }

    /// Whether `secret` is the one the linked app holds.
    pub async fn app_secret_matches(
        &self,
        registration_number: &str,
        secret: Option<&str>,
    ) -> Result<bool, StoreError> {
        let Some(secret) = secret.map(str::trim).filter(|secret| !secret.is_empty()) else {
            return Ok(false);
        };
        let doc = self.load(&account_id(registration_number)).await?;
        Ok(doc
            .and_then(|doc| doc.app_secret_hash)
            .is_some_and(|stored| stored == crate::keys::hash(secret)))
    }

    /// Links the app: stores its FCM token, session and secret hash. Without
    /// `keep_vault` (the app could not show the old secret) saved
    /// credentials are dropped unless new ones come with the link.
    #[allow(clippy::too_many_arguments)]
    pub async fn link(
        &self,
        registration_number: &str,
        fcm_token: &str,
        session: &SessionState,
        vault: Option<Vault>,
        app_secret_hash: String,
        keep_vault: bool,
        now: u64,
    ) -> Result<(), StoreError> {
        let expires_at = now + self.session_ttl.as_secs();
        self.update(registration_number, now, |doc, sealer, aad| {
            if !keep_vault {
                doc.vault = None;
                doc.vault_expires_at = None;
            }
            doc.session = Some(sealer.seal_json(aad, session));
            doc.session_expires_at = Some(expires_at);
            doc.fcm_token = Some(sealer.seal_json(aad, &fcm_token));
            if let Some(vault) = &vault {
                doc.vault = Some(sealer.seal_json(aad, vault));
                doc.vault_expires_at = vault_expiry(doc, now);
            }
            doc.app_secret_hash = Some(app_secret_hash);
        })
        .await
    }

    pub async fn set_fcm_token(
        &self,
        registration_number: &str,
        fcm_token: &str,
        now: u64,
    ) -> Result<(), StoreError> {
        self.update(registration_number, now, |doc, sealer, aad| {
            doc.fcm_token = Some(sealer.seal_json(aad, &fcm_token));
        })
        .await
    }

    /// The phone's token stopped working (FCM said it is unregistered).
    pub async fn clear_fcm_token(&self, registration_number: &str) -> Result<(), StoreError> {
        if !self.exists(registration_number).await? {
            return Ok(());
        }
        self.update(registration_number, vtop_core::now_unix(), |doc, _, _| {
            doc.fcm_token = None
        })
        .await
    }

    pub async fn exists(&self, registration_number: &str) -> Result<bool, StoreError> {
        Ok(self.load(&account_id(registration_number)).await?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::session;
    use crate::store::MemoryStore;

    fn vault() -> Vault {
        Vault {
            username: "yaswanth314".into(),
            password: "hunter2".into(),
            gmail: GmailGrant {
                refresh_token: "refresh".into(),
                client_id: "client".into(),
                client_secret: None,
                delete_after_reading: true,
            },
        }
    }

    fn accounts(store: Arc<MemoryStore>) -> Accounts {
        Accounts::new(
            store,
            Arc::new(Sealer::new([3; 32])),
            Duration::from_secs(1800),
        )
    }

    #[tokio::test]
    async fn record_phone_stores_everything_sealed() {
        let store = Arc::new(MemoryStore::default());
        let accounts = accounts(store.clone());
        accounts
            .record_phone(
                "22bce0001 ",
                Some("fcm-1"),
                &session("A=1"),
                Some(vault()),
                100,
            )
            .await
            .unwrap();

        let doc = store.get_account("22BCE0001").await.unwrap().unwrap();
        for sealed in [&doc.vault, &doc.session, &doc.fcm_token] {
            let sealed = sealed.as_deref().unwrap();
            assert!(
                !sealed.contains("hunter2") && !sealed.contains("fcm-1") && !sealed.contains("A=1")
            );
        }
        assert_eq!(doc.session_expires_at, Some(1900));
        assert_eq!(
            accounts.fcm_token("22bce0001").await.unwrap().as_deref(),
            Some("fcm-1")
        );
        assert!(accounts.vault("22BCE0001").await.unwrap() == Some(vault()));
        assert_eq!(
            accounts.cached_session("22BCE0001", 1899).await.unwrap(),
            Some(session("A=1"))
        );
    }

    #[tokio::test]
    async fn record_phone_without_token_keeps_the_existing_token() {
        let store = Arc::new(MemoryStore::default());
        let accounts = accounts(store);
        accounts
            .record_phone("U", Some("fcm-1"), &session("A=1"), None, 100)
            .await
            .unwrap();
        accounts
            .record_phone("U", None, &session("A=2"), None, 200)
            .await
            .unwrap();
        assert_eq!(
            accounts.fcm_token("U").await.unwrap().as_deref(),
            Some("fcm-1")
        );
        assert_eq!(
            accounts.cached_session("U", 201).await.unwrap(),
            Some(session("A=2"))
        );
    }

    #[tokio::test]
    async fn set_and_clear_fcm_token() {
        let store = Arc::new(MemoryStore::default());
        let accounts = accounts(store);
        accounts.set_fcm_token("U", "fcm-1", 100).await.unwrap();
        assert_eq!(
            accounts.fcm_token("U").await.unwrap().as_deref(),
            Some("fcm-1")
        );
        accounts.set_fcm_token("U", "fcm-2", 101).await.unwrap();
        assert_eq!(
            accounts.fcm_token("U").await.unwrap().as_deref(),
            Some("fcm-2")
        );
        accounts.clear_fcm_token("U").await.unwrap();
        assert!(accounts.fcm_token("U").await.unwrap().is_none());
        assert!(accounts.exists("U").await.unwrap());
        assert!(!accounts.exists("NOBODY").await.unwrap());
    }

    #[tokio::test]
    async fn cached_session_expires() {
        let store = Arc::new(MemoryStore::default());
        let accounts = accounts(store);
        accounts
            .save_session("U", &session("A=1"), 100)
            .await
            .unwrap();
        assert!(accounts.cached_session("U", 1899).await.unwrap().is_some());
        assert!(accounts.cached_session("U", 1900).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn record_callback_without_vault_keeps_existing_vault() {
        let store = Arc::new(MemoryStore::default());
        let accounts = accounts(store);
        accounts
            .record_phone("U", Some("fcm-1"), &session("A=1"), Some(vault()), 100)
            .await
            .unwrap();
        accounts
            .record_phone("U", Some("fcm-2"), &session("A=2"), None, 200)
            .await
            .unwrap();
        assert!(accounts.vault("U").await.unwrap() == Some(vault()));
        assert_eq!(
            accounts.fcm_token("U").await.unwrap().as_deref(),
            Some("fcm-2")
        );
    }

    #[tokio::test]
    async fn clear_vault_and_session_are_independent() {
        let store = Arc::new(MemoryStore::default());
        let accounts = accounts(store);
        accounts
            .record_phone("U", Some("fcm-1"), &session("A=1"), Some(vault()), 100)
            .await
            .unwrap();
        accounts.clear_session("U").await.unwrap();
        assert!(accounts.cached_session("U", 101).await.unwrap().is_none());
        assert!(accounts.vault("U").await.unwrap().is_some());
        accounts.clear_vault("U").await.unwrap();
        assert!(accounts.vault("U").await.unwrap().is_none());
        assert!(accounts.fcm_token("U").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn undecryptable_vault_is_cleared_and_treated_absent() {
        let store = Arc::new(MemoryStore::default());
        accounts(store.clone())
            .record_phone("U", Some("fcm-1"), &session("A=1"), Some(vault()), 100)
            .await
            .unwrap();
        // A rotated key cannot open the old values.
        let rotated = Accounts::new(
            store.clone(),
            Arc::new(Sealer::new([4; 32])),
            Duration::from_secs(1800),
        );
        assert!(rotated.vault("U").await.unwrap().is_none());
        assert!(rotated.cached_session("U", 101).await.unwrap().is_none());
        assert!(rotated.fcm_token("U").await.unwrap().is_none());
        let doc = store.get_account("U").await.unwrap().unwrap();
        assert!(doc.vault.is_none() && doc.session.is_none());
    }

    #[tokio::test]
    async fn missing_account_reads_as_empty() {
        let accounts = accounts(Arc::new(MemoryStore::default()));
        assert!(accounts.vault("U").await.unwrap().is_none());
        assert!(accounts.fcm_token("U").await.unwrap().is_none());
        assert!(accounts.cached_session("U", 1).await.unwrap().is_none());
        accounts.clear_vault("U").await.unwrap();
        accounts.clear_session("U").await.unwrap();
    }

    #[tokio::test]
    async fn settings_default_until_changed() {
        let store = Arc::new(MemoryStore::default());
        let accounts = accounts(store);
        let defaults = AccountSettings::default();
        assert_eq!(defaults.phone_wait_secs, 20);
        assert_eq!(defaults.vault_ttl_secs, None);
        assert!(defaults.always_use_vault);
        assert_eq!(accounts.settings("22BCE0001").await.unwrap(), defaults);

        let changed = AccountSettings {
            phone_wait_secs: 10,
            vault_ttl_secs: Some(86_400),
            always_use_vault: false,
        };
        accounts
            .record_phone("22BCE0001", Some("fcm-1"), &session("A=1"), None, 100)
            .await
            .unwrap();
        accounts
            .set_settings("22BCE0001", changed, 100)
            .await
            .unwrap();
        assert_eq!(accounts.settings("22bce0001").await.unwrap(), changed);
    }

    #[test]
    fn settings_only_take_the_offered_choices() {
        let ok = AccountSettings::default();
        assert!(ok.is_valid());
        assert!(!AccountSettings {
            phone_wait_secs: 0,
            ..ok
        }
        .is_valid());
        for wait in [10, 20, 45] {
            assert!(AccountSettings {
                phone_wait_secs: wait,
                ..ok
            }
            .is_valid());
        }
        assert!(!AccountSettings {
            phone_wait_secs: 5,
            ..ok
        }
        .is_valid());
        for ttl in [Some(86_400), Some(172_800), None] {
            assert!(AccountSettings {
                vault_ttl_secs: ttl,
                ..ok
            }
            .is_valid());
        }
        assert!(!AccountSettings {
            vault_ttl_secs: Some(60),
            ..ok
        }
        .is_valid());
    }

    #[tokio::test]
    async fn saved_credentials_expire_after_the_chosen_time() {
        let store = Arc::new(MemoryStore::default());
        let accounts = accounts(store.clone());
        let now = vtop_core::now_unix();
        accounts
            .record_phone("22BCE0001", Some("fcm-1"), &session("A=1"), None, now)
            .await
            .unwrap();
        let one_day = AccountSettings {
            vault_ttl_secs: Some(86_400),
            ..AccountSettings::default()
        };
        accounts
            .set_settings("22BCE0001", one_day, now)
            .await
            .unwrap();

        // Saved two days ago: expired, so wiped on read.
        accounts
            .record_phone(
                "22BCE0001",
                None,
                &session("A=1"),
                Some(vault()),
                now - 172_800,
            )
            .await
            .unwrap();
        assert!(accounts.vault("22BCE0001").await.unwrap().is_none());
        assert!(store
            .get_account("22BCE0001")
            .await
            .unwrap()
            .unwrap()
            .vault
            .is_none());

        // Saved now: kept.
        accounts
            .record_phone("22BCE0001", None, &session("A=1"), Some(vault()), now)
            .await
            .unwrap();
        assert!(accounts.vault("22BCE0001").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn never_keeps_saved_credentials_and_changing_ttl_restarts_the_timer() {
        let store = Arc::new(MemoryStore::default());
        let accounts = accounts(store.clone());
        let now = vtop_core::now_unix();
        accounts
            .record_phone(
                "22BCE0001",
                Some("fcm-1"),
                &session("A=1"),
                Some(vault()),
                now - 10 * 86_400,
            )
            .await
            .unwrap();
        assert!(accounts.vault("22BCE0001").await.unwrap().is_some());

        let two_days = AccountSettings {
            vault_ttl_secs: Some(172_800),
            ..AccountSettings::default()
        };
        accounts
            .set_settings("22BCE0001", two_days, now)
            .await
            .unwrap();
        let doc = store.get_account("22BCE0001").await.unwrap().unwrap();
        assert_eq!(doc.vault_expires_at, Some(now + 172_800));
        assert!(accounts.vault("22BCE0001").await.unwrap().is_some());
    }
}

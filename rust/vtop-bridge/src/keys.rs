//! Personal access keys: `vtm_` + 64 hex characters, stored only as their
//! SHA-256. One kind of key serves every client.

use std::sync::Arc;

use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::store::{KeyDoc, Store, StoreError};

pub const KEY_PREFIX: &str = "vtm_";
/// The linked app's secret; proves account changes come from that phone.
pub const APP_SECRET_PREFIX: &str = "vta_";
pub const MAX_KEYS_PER_ACCOUNT: usize = 10;
/// `last_used_at` is written at most this often per key.
const TOUCH_EVERY_SECS: u64 = 60;

pub fn new_key() -> String {
    random_token(KEY_PREFIX)
}

pub fn new_app_secret() -> String {
    random_token(APP_SECRET_PREFIX)
}

fn random_token(prefix: &str) -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!("{prefix}{}", hex::encode(bytes))
}

pub fn hash(key: &str) -> String {
    hex::encode(Sha256::digest(key.as_bytes()))
}

/// The short id users see and revoke by.
pub fn key_id(hash: &str) -> &str {
    &hash[..8.min(hash.len())]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    Limit,
    Store(StoreError),
}

impl From<StoreError> for KeyError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

pub struct Keys {
    store: Arc<dyn Store>,
}

impl Keys {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }

    /// Creates a key for the account; returns (id, key). The key is not
    /// stored and cannot be shown again.
    pub async fn create(
        &self,
        registration_number: &str,
        label: &str,
        now: u64,
    ) -> Result<(String, String), KeyError> {
        if self.store.list_keys(registration_number).await?.len() >= MAX_KEYS_PER_ACCOUNT {
            return Err(KeyError::Limit);
        }
        let key = new_key();
        let doc = KeyDoc {
            hash: hash(&key),
            registration_number: registration_number.to_string(),
            label: label.trim().chars().take(60).collect(),
            created_at: now,
            last_used_at: None,
        };
        self.store.put_key(&doc).await?;
        Ok((key_id(&doc.hash).to_string(), key))
    }

    /// The account a presented key belongs to.
    pub async fn resolve(&self, key: &str, now: u64) -> Result<Option<String>, StoreError> {
        Ok(self
            .resolve_key(key, now)
            .await?
            .map(|doc| doc.registration_number))
    }

    /// The record of a presented key; touches `last_used_at`.
    pub async fn resolve_key(&self, key: &str, now: u64) -> Result<Option<KeyDoc>, StoreError> {
        let key = key.trim();
        if !key.starts_with(KEY_PREFIX) {
            return Ok(None);
        }
        let Some(mut doc) = self.store.get_key(&hash(key)).await? else {
            return Ok(None);
        };
        if doc
            .last_used_at
            .is_none_or(|last| now >= last + TOUCH_EVERY_SECS)
        {
            doc.last_used_at = Some(now);
            if let Err(error) = self.store.put_key(&doc).await {
                tracing::warn!("touch access key: {error}");
            }
        }
        Ok(Some(doc))
    }

    pub async fn list(&self, registration_number: &str) -> Result<Vec<KeyDoc>, StoreError> {
        let mut keys = self.store.list_keys(registration_number).await?;
        keys.sort_by_key(|key| key.created_at);
        Ok(keys)
    }

    /// False when no key of this account has `id`.
    /// Removes every key of the account.
    pub async fn revoke_all(&self, registration_number: &str) -> Result<(), StoreError> {
        for key in self.store.list_keys(registration_number).await? {
            self.store.delete_key(&key.hash).await?;
        }
        Ok(())
    }

    pub async fn revoke(&self, registration_number: &str, id: &str) -> Result<bool, StoreError> {
        let id = id.trim();
        let found = self
            .store
            .list_keys(registration_number)
            .await?
            .into_iter()
            .find(|key| !id.is_empty() && key_id(&key.hash) == id);
        match found {
            Some(key) => {
                self.store.delete_key(&key.hash).await?;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryStore;

    fn keys() -> (Arc<MemoryStore>, Keys) {
        let store = Arc::new(MemoryStore::default());
        (store.clone(), Keys::new(store))
    }

    #[test]
    fn new_key_format() {
        let key = new_key();
        assert!(key.starts_with("vtm_"));
        assert_eq!(key.len(), 4 + 64);
        assert!(key[4..].chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(new_key(), key);
    }

    #[test]
    fn hash_is_not_key() {
        let key = new_key();
        assert_eq!(hash(&key).len(), 64);
        assert!(!hash(&key).contains(&key[4..]));
        assert_eq!(key_id(&hash(&key)).len(), 8);
    }

    #[tokio::test]
    async fn create_stores_only_the_hash_and_resolves() {
        let (store, keys) = keys();
        let (id, key) = keys.create("A", "laptop", 100).await.unwrap();
        let stored = store.get_key(&hash(&key)).await.unwrap().unwrap();
        assert_eq!(stored.registration_number, "A");
        assert_eq!(stored.label, "laptop");
        assert_eq!(id, key_id(&stored.hash));
        assert_eq!(keys.resolve(&key, 101).await.unwrap().as_deref(), Some("A"));
        assert_eq!(keys.resolve("vtm_unknown", 101).await.unwrap(), None);
    }

    #[tokio::test]
    async fn create_respects_limit() {
        let (_, keys) = keys();
        for _ in 0..MAX_KEYS_PER_ACCOUNT {
            keys.create("A", "k", 1).await.unwrap();
        }
        assert_eq!(keys.create("A", "k", 1).await.err(), Some(KeyError::Limit));
        keys.create("B", "k", 1).await.unwrap();
    }

    #[tokio::test]
    async fn resolve_touches_last_used_once_per_minute() {
        let (store, keys) = keys();
        let (_, key) = keys.create("A", "k", 100).await.unwrap();
        keys.resolve(&key, 200).await.unwrap();
        assert_eq!(
            store
                .get_key(&hash(&key))
                .await
                .unwrap()
                .unwrap()
                .last_used_at,
            Some(200)
        );
        keys.resolve(&key, 230).await.unwrap();
        assert_eq!(
            store
                .get_key(&hash(&key))
                .await
                .unwrap()
                .unwrap()
                .last_used_at,
            Some(200)
        );
        keys.resolve(&key, 261).await.unwrap();
        assert_eq!(
            store
                .get_key(&hash(&key))
                .await
                .unwrap()
                .unwrap()
                .last_used_at,
            Some(261)
        );
    }

    #[tokio::test]
    async fn revoke_other_accounts_key_is_false() {
        let (_, keys) = keys();
        let (id, key) = keys.create("A", "k", 1).await.unwrap();
        assert!(!keys.revoke("B", &id).await.unwrap());
        assert_eq!(keys.resolve(&key, 2).await.unwrap().as_deref(), Some("A"));
        assert!(keys.revoke("A", &id).await.unwrap());
        assert_eq!(keys.resolve(&key, 3).await.unwrap(), None);
        assert!(keys.list("A").await.unwrap().is_empty());
    }
}

//! Short-lived in-memory cache of signed-in clients, keyed by a SHA-256 of
//! the caller's cookie header. A hit skips re-validating the cookie with VTOP
//! and reuses the client's warm connections. Nothing is written to disk and
//! the raw cookie is never used as a key.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use vtop_core::VtopClient;

type CacheKey = [u8; 32];

struct Entry {
    expires_at: Instant,
    client: Arc<VtopClient>,
}

pub struct SessionCache {
    ttl: Duration,
    max_entries: usize,
    entries: Mutex<HashMap<CacheKey, Entry>>,
}

impl SessionCache {
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            ttl,
            max_entries,
            entries: Mutex::new(HashMap::new()),
        }
    }

    fn key(cookies: &str) -> CacheKey {
        Sha256::digest(cookies.trim()).into()
    }

    fn enabled(&self) -> bool {
        !self.ttl.is_zero() && self.max_entries > 0
    }

    fn entries(&self) -> std::sync::MutexGuard<'_, HashMap<CacheKey, Entry>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn get(&self, cookies: &str) -> Option<Arc<VtopClient>> {
        self.get_at(cookies, Instant::now())
    }

    fn get_at(&self, cookies: &str, now: Instant) -> Option<Arc<VtopClient>> {
        if !self.enabled() {
            return None;
        }
        let key = Self::key(cookies);
        let mut entries = self.entries();
        match entries.get(&key) {
            Some(entry) if entry.expires_at > now => Some(entry.client.clone()),
            Some(_) => {
                entries.remove(&key);
                None
            }
            None => None,
        }
    }

    /// The cached client for `cookies`, or a new one from `build` that is
    /// cached straight away. Requests that arrive together with the same
    /// cookie therefore share one client, and its validation lock makes
    /// VTOP see a single validation instead of one per request.
    pub fn client_for<E>(
        &self,
        cookies: &str,
        build: impl FnOnce() -> Result<VtopClient, E>,
    ) -> Result<Arc<VtopClient>, E> {
        if !self.enabled() {
            return build().map(Arc::new);
        }
        let now = Instant::now();
        let key = Self::key(cookies);
        let mut entries = self.entries();
        if let Some(entry) = entries.get(&key) {
            if entry.expires_at > now {
                return Ok(entry.client.clone());
            }
        }
        let client = Arc::new(build()?);
        if entries.len() >= self.max_entries {
            entries.retain(|_, entry| entry.expires_at > now);
        }
        if entries.len() < self.max_entries {
            entries.insert(
                key,
                Entry {
                    expires_at: now + self.ttl,
                    client: client.clone(),
                },
            );
        }
        Ok(client)
    }

    /// Caches `client` for `cookies` if it is signed in.
    pub fn put(&self, cookies: &str, client: Arc<VtopClient>) {
        self.put_at(cookies, client, Instant::now());
    }

    fn put_at(&self, cookies: &str, client: Arc<VtopClient>, now: Instant) {
        if !self.enabled() || !client.is_authenticated() {
            return;
        }
        let key = Self::key(cookies);
        let mut entries = self.entries();
        if entries.len() >= self.max_entries && !entries.contains_key(&key) {
            entries.retain(|_, entry| entry.expires_at > now);
            if entries.len() >= self.max_entries {
                return;
            }
        }
        entries.insert(
            key,
            Entry {
                expires_at: now + self.ttl,
                client,
            },
        );
    }

    pub fn remove(&self, cookies: &str) {
        self.entries().remove(&Self::key(cookies));
    }

    pub fn len(&self) -> usize {
        self.entries().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use vtop_core::SessionState;

    use super::*;

    fn signed_in_client() -> Arc<VtopClient> {
        let client = VtopClient::builder()
            .with_session(&SessionState {
                cookies: "JSESSIONID=abc".into(),
                csrf_token: Some("csrf".into()),
                registration_number: Some("22BCE0001".into()),
                otp_issued_at: None,
                logged_in_at: None,
            })
            .unwrap();
        Arc::new(client)
    }

    #[test]
    fn entries_expire() {
        let cache = SessionCache::new(Duration::from_secs(60), 10);
        let now = Instant::now();
        cache.put_at("JSESSIONID=abc", signed_in_client(), now);
        assert!(cache.get_at("JSESSIONID=abc", now).is_some());
        assert!(cache.get_at(" JSESSIONID=abc ", now).is_some());
        assert!(cache.get_at("JSESSIONID=other", now).is_none());
        assert!(cache
            .get_at("JSESSIONID=abc", now + Duration::from_secs(61))
            .is_none());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn full_cache_drops_expired_entries_then_refuses() {
        let cache = SessionCache::new(Duration::from_secs(60), 1);
        let now = Instant::now();
        cache.put_at("a", signed_in_client(), now);
        cache.put_at("b", signed_in_client(), now);
        assert!(cache.get_at("b", now).is_none());
        cache.put_at("b", signed_in_client(), now + Duration::from_secs(61));
        assert!(cache.get_at("b", now + Duration::from_secs(61)).is_some());
    }

    #[test]
    fn zero_ttl_disables_the_cache() {
        let cache = SessionCache::new(Duration::ZERO, 10);
        cache.put("a", signed_in_client());
        assert!(cache.get("a").is_none());
    }

    #[test]
    fn concurrent_callers_share_one_client() {
        let cache = SessionCache::new(Duration::from_secs(60), 10);
        let build = || {
            VtopClient::builder().with_session(&SessionState {
                cookies: "JSESSIONID=abc".into(),
                csrf_token: None,
                registration_number: None,
                otp_issued_at: None,
                logged_in_at: None,
            })
        };
        let first = cache.client_for("JSESSIONID=abc", build).unwrap();
        let second = cache.client_for("JSESSIONID=abc", build).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn unauthenticated_clients_are_not_cached() {
        let cache = SessionCache::new(Duration::from_secs(60), 10);
        let client = VtopClient::builder()
            .with_session(&SessionState {
                cookies: "JSESSIONID=abc".into(),
                csrf_token: None,
                registration_number: None,
                otp_issued_at: None,
                logged_in_at: None,
            })
            .unwrap();
        cache.put("JSESSIONID=abc", Arc::new(client));
        assert!(cache.get("JSESSIONID=abc").is_none());
    }
}

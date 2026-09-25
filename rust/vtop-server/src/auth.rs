//! `X-API-Key` check. Keys are compared in constant time and never logged.
//! With no keys configured the server is open: every request is let through
//! and shares one rate-limit bucket.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::error::ApiError;
use crate::AppState;

pub const API_KEY_HEADER: &str = "x-api-key";

/// Which configured key a request used (its index in the key list). Used to
/// rate limit and to tag logs without exposing the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyId(pub usize);

/// Holds SHA-256 digests of the keys so every comparison has the same length.
pub struct ApiKeys {
    digests: Vec<[u8; 32]>,
}

impl ApiKeys {
    pub fn new(keys: &[String]) -> Self {
        Self {
            digests: keys.iter().map(|key| Sha256::digest(key).into()).collect(),
        }
    }

    /// True when no keys are configured and the header is not checked.
    pub fn is_open(&self) -> bool {
        self.digests.is_empty()
    }

    pub fn identify(&self, presented: &str) -> Option<KeyId> {
        let digest: [u8; 32] = Sha256::digest(presented).into();
        // Check every key so timing does not reveal which one matched.
        let mut found = None;
        for (index, known) in self.digests.iter().enumerate() {
            if bool::from(known.ct_eq(&digest)) {
                found = Some(KeyId(index));
            }
        }
        found
    }
}

pub async fn require_api_key(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let key_id = if state.api_keys.is_open() {
        KeyId(0)
    } else {
        request
            .headers()
            .get(API_KEY_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|presented| state.api_keys.identify(presented.trim()))
            .ok_or(ApiError::Unauthorized)?
    };
    state.rate_limiter.check(key_id)?;
    request.extensions_mut().insert(key_id);
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_only_configured_keys() {
        let keys = ApiKeys::new(&["first-key-aaaaaaaa".into(), "second-key-bbbbbbb".into()]);
        assert_eq!(keys.identify("second-key-bbbbbbb"), Some(KeyId(1)));
        assert_eq!(keys.identify("first-key-aaaaaaaa"), Some(KeyId(0)));
        assert_eq!(keys.identify("first-key-aaaaaaa"), None);
        assert_eq!(keys.identify(""), None);
        assert!(!keys.is_open());
        assert!(ApiKeys::new(&[]).is_open());
    }
}

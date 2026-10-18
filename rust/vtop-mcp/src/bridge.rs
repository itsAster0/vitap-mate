//! Sessions from vtop-bridge, fetched with the caller's own access key.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use reqwest::StatusCode;
use serde_json::Value;
use vtop_core::SessionState;

/// Who a key belongs to.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub registration_number: String,
    pub key_label: Option<String>,
    /// The semester picked in the student's app, once it has sent one.
    pub semester_id: Option<String>,
}

/// Where tools get VTOP sessions from. Every call carries the caller's key.
#[async_trait]
pub trait SessionSource: Send + Sync {
    /// Who `key` belongs to, or `None` for an unknown or revoked key.
    async fn whoami(&self, key: &str) -> Result<Option<Identity>, String>;
    async fn session(&self, key: &str) -> Result<SessionState, String>;
    async fn expire(&self, key: &str) -> Result<(), String>;
}

const CALL_TIMEOUT: Duration = Duration::from_secs(20);
/// How long a tool waits for a pending request: up to 45 s for the phone,
/// then a vault login that may wait 45 s for the OTP mail. The bridge gives
/// up at 120 s.
const PENDING_WAIT: Duration = Duration::from_secs(110);

pub struct BridgeClient {
    http: reqwest::Client,
    base_url: String,
}

fn error_text(body: &Value, fallback: &str) -> String {
    let code = body
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("bridge_error");
    let message = body
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or(fallback);
    if message.starts_with(code) {
        message.to_string()
    } else {
        format!("{code}: {message}")
    }
}

fn session_of(body: &Value) -> Result<SessionState, String> {
    body.get("session")
        .cloned()
        .and_then(|session| serde_json::from_value(session).ok())
        .ok_or_else(|| "bridge: unreadable session reply".to_string())
}

impl BridgeClient {
    pub fn new(http: reqwest::Client, base_url: String) -> Self {
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Sends a request with the key. A connection error, or a 502/503
    /// without a bridge error body (Railway waking a sleeping service), is
    /// tried once more. Returns the status and JSON body.
    async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        key: &str,
    ) -> Result<(StatusCode, Value), String> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let sent = self
                .http
                .request(method.clone(), format!("{}{path}", self.base_url))
                .bearer_auth(key)
                .timeout(CALL_TIMEOUT)
                .send()
                .await;
            let response = match sent {
                Ok(response) => response,
                Err(_) if attempt == 1 => continue,
                Err(error) => return Err(format!("bridge unreachable: {}", error.without_url())),
            };
            let status = response.status();
            let body: Value = response.json().await.unwrap_or(Value::Null);
            let waking = matches!(status.as_u16(), 502 | 503) && body.get("code").is_none();
            if waking && attempt == 1 {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
            return Ok((status, body));
        }
    }
}

#[async_trait]
impl SessionSource for BridgeClient {
    async fn whoami(&self, key: &str) -> Result<Option<Identity>, String> {
        let (status, body) = self.call(reqwest::Method::GET, "/v1/whoami", key).await?;
        match status {
            StatusCode::OK => Ok(body.get("registrationNumber").and_then(Value::as_str).map(
                |registration_number| Identity {
                    registration_number: registration_number.to_string(),
                    key_label: body
                        .get("keyLabel")
                        .and_then(Value::as_str)
                        .map(String::from),
                    semester_id: body
                        .get("semesterId")
                        .and_then(Value::as_str)
                        .map(String::from),
                },
            )),
            StatusCode::UNAUTHORIZED => Ok(None),
            other => Err(error_text(&body, other.as_str())),
        }
    }

    async fn session(&self, key: &str) -> Result<SessionState, String> {
        let (status, body) = self.call(reqwest::Method::POST, "/v1/session", key).await?;
        match status {
            StatusCode::OK => return session_of(&body),
            StatusCode::ACCEPTED => {}
            other => return Err(error_text(&body, other.as_str())),
        }
        let request_id = body
            .get("requestId")
            .and_then(Value::as_str)
            .ok_or_else(|| "bridge: pending reply without requestId".to_string())?
            .to_string();
        let poll_after = Duration::from_millis(
            body.get("pollAfterMs")
                .and_then(Value::as_u64)
                .unwrap_or(1500)
                .clamp(10, 5000),
        );
        let deadline = Instant::now() + PENDING_WAIT;
        loop {
            tokio::time::sleep(poll_after).await;
            let path = format!("/v1/session/requests/{request_id}");
            let (status, body) = self.call(reqwest::Method::GET, &path, key).await?;
            match status {
                StatusCode::ACCEPTED => {}
                StatusCode::OK if body.get("status").and_then(Value::as_str) == Some("ready") => {
                    return session_of(&body)
                }
                StatusCode::OK => return Err(error_text(&body, "the session request failed")),
                StatusCode::GONE => {
                    return Err("phone_timeout: The phone did not answer in time.".to_string())
                }
                other => return Err(error_text(&body, other.as_str())),
            }
            if Instant::now() >= deadline {
                return Err("phone_timeout: The phone did not answer in time.".to_string());
            }
        }
    }

    async fn expire(&self, key: &str) -> Result<(), String> {
        let (status, body) = self
            .call(reqwest::Method::POST, "/v1/session/expire", key)
            .await?;
        if status.is_success() {
            Ok(())
        } else {
            Err(error_text(&body, status.as_str()))
        }
    }
}

/// Keeps each key's session for `ttl`, so tools called one after another
/// skip the trip to the edge (and the edge's VTOP check of its cached
/// session). When VTOP rejects a kept session, the tools call `expire`,
/// which drops it here too and fetches a fresh one.
pub struct CachedSessions {
    inner: std::sync::Arc<dyn SessionSource>,
    ttl: Duration,
    kept: tokio::sync::Mutex<std::collections::HashMap<String, (Instant, SessionState)>>,
}

impl CachedSessions {
    pub fn new(inner: std::sync::Arc<dyn SessionSource>, ttl: Duration) -> Self {
        Self {
            inner,
            ttl,
            kept: Default::default(),
        }
    }
}

#[async_trait]
impl SessionSource for CachedSessions {
    async fn whoami(&self, key: &str) -> Result<Option<Identity>, String> {
        self.inner.whoami(key).await
    }

    async fn session(&self, key: &str) -> Result<SessionState, String> {
        {
            let mut kept = self.kept.lock().await;
            kept.retain(|_, (at, _)| at.elapsed() < self.ttl);
            if let Some((_, session)) = kept.get(key) {
                return Ok(session.clone());
            }
        }
        let session = self.inner.session(key).await?;
        // Only a signed-in session is worth reusing, not one waiting on an OTP.
        if session.otp_issued_at.is_none() {
            self.kept
                .lock()
                .await
                .insert(key.to_string(), (Instant::now(), session.clone()));
        }
        Ok(session)
    }

    async fn expire(&self, key: &str) -> Result<(), String> {
        self.kept.lock().await.remove(key);
        self.inner.expire(key).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;

    #[derive(Default)]
    struct Counting {
        sessions: AtomicUsize,
        expires: AtomicUsize,
    }

    #[async_trait]
    impl SessionSource for Counting {
        async fn whoami(&self, _key: &str) -> Result<Option<Identity>, String> {
            Ok(None)
        }
        async fn session(&self, key: &str) -> Result<SessionState, String> {
            let n = self.sessions.fetch_add(1, Ordering::SeqCst);
            Ok(SessionState {
                cookies: format!("{key}-{n}"),
                csrf_token: None,
                registration_number: None,
                otp_issued_at: None,
                logged_in_at: None,
            })
        }
        async fn expire(&self, _key: &str) -> Result<(), String> {
            self.expires.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn keeps_a_session_until_expired() {
        let inner = Arc::new(Counting::default());
        let cached = CachedSessions::new(inner.clone(), Duration::from_secs(60));
        let first = cached.session("a").await.unwrap();
        assert_eq!(cached.session("a").await.unwrap(), first);
        assert_eq!(inner.sessions.load(Ordering::SeqCst), 1);
        // Another key gets its own.
        assert_ne!(cached.session("b").await.unwrap(), first);
        cached.expire("a").await.unwrap();
        assert_eq!(inner.expires.load(Ordering::SeqCst), 1);
        assert_ne!(cached.session("a").await.unwrap(), first);
        assert_eq!(inner.sessions.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn fetches_again_after_the_ttl() {
        let inner = Arc::new(Counting::default());
        let cached = CachedSessions::new(inner.clone(), Duration::ZERO);
        cached.session("a").await.unwrap();
        cached.session("a").await.unwrap();
        assert_eq!(inner.sessions.load(Ordering::SeqCst), 2);
    }
}

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
    /// Whether the student let this key apply for and cancel outings.
    pub outings: bool,
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
/// Cap phone waits under Convex's 32 s `/mcp` proxy timeout so the client gets
/// an actionable tool error instead of a generic 504.
pub const PENDING_WAIT: Duration = Duration::from_secs(25);
pub const PHONE_TIMEOUT_ERROR: &str = "Your phone didn't answer in time. Open VITAP Mate on your phone and retry, or turn on Store on server in Connected apps for offline access.";

pub struct BridgeClient {
    http: reqwest::Client,
    base_url: String,
    pending_wait: Duration,
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
            pending_wait: PENDING_WAIT,
        }
    }

    pub fn with_pending_wait(mut self, pending_wait: Duration) -> Self {
        self.pending_wait = pending_wait;
        self
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
        self.call_with_timeout(method, path, key, CALL_TIMEOUT)
            .await
    }

    async fn call_with_timeout(
        &self,
        method: reqwest::Method,
        path: &str,
        key: &str,
        timeout: Duration,
    ) -> Result<(StatusCode, Value), String> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let sent = self
                .http
                .request(method.clone(), format!("{}{path}", self.base_url))
                .bearer_auth(key)
                // So the account shows this key was used by an AI agent.
                .header("x-vtop-client", "mcp")
                .timeout(timeout)
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
                    outings: body.get("keyOutings").and_then(Value::as_bool) == Some(true),
                },
            )),
            StatusCode::UNAUTHORIZED => Ok(None),
            other => Err(error_text(&body, other.as_str())),
        }
    }

    async fn session(&self, key: &str) -> Result<SessionState, String> {
        let deadline = Instant::now() + self.pending_wait;
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
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(PHONE_TIMEOUT_ERROR.to_string());
            }
            tokio::time::sleep(poll_after.min(remaining)).await;
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(PHONE_TIMEOUT_ERROR.to_string());
            }
            let path = format!("/v1/session/requests/{request_id}");
            let (status, body) = match self
                .call_with_timeout(
                    reqwest::Method::GET,
                    &path,
                    key,
                    remaining.min(CALL_TIMEOUT),
                )
                .await
            {
                Ok(reply) => reply,
                Err(_) if Instant::now() >= deadline => {
                    return Err(PHONE_TIMEOUT_ERROR.to_string());
                }
                Err(err) => return Err(err),
            };
            match status {
                StatusCode::ACCEPTED => {}
                StatusCode::OK if body.get("status").and_then(Value::as_str) == Some("ready") => {
                    return session_of(&body);
                }
                StatusCode::OK => return Err(error_text(&body, "the session request failed")),
                StatusCode::GONE => {
                    return Err(PHONE_TIMEOUT_ERROR.to_string());
                }
                other => return Err(error_text(&body, other.as_str())),
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

/// Keeps each key's session in RAM for `ttl`, so tools called one after another
/// skip the trip to the edge (and the edge's VTOP check of its cached
/// session). Keyed by SHA-256 hex of the access key so the backend can evict
/// entries when storage settings change or a key is revoked. When VTOP rejects
/// a kept session, the tools call `expire`, which drops it here too and fetches
/// a fresh one.
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

    /// Drops cached sessions for the given SHA-256 hex key hashes.
    pub async fn evict_hashes(&self, key_hashes: &[String]) -> usize {
        let mut kept = self.kept.lock().await;
        let mut removed = 0;
        for hash in key_hashes {
            if kept.remove(hash).is_some() {
                removed += 1;
            }
        }
        removed
    }
}

#[async_trait]
impl SessionSource for CachedSessions {
    async fn whoami(&self, key: &str) -> Result<Option<Identity>, String> {
        self.inner.whoami(key).await
    }

    async fn session(&self, key: &str) -> Result<SessionState, String> {
        let hash = crate::hex_sha256(key);
        {
            let mut kept = self.kept.lock().await;
            kept.retain(|_, (at, _)| at.elapsed() < self.ttl);
            if let Some((_, session)) = kept.get(&hash) {
                return Ok(session.clone());
            }
        }
        let session = self.inner.session(key).await?;
        // Only a signed-in session is worth reusing, not one waiting on an OTP.
        if session.otp_issued_at.is_none() {
            self.kept
                .lock()
                .await
                .insert(hash, (Instant::now(), session.clone()));
        }
        Ok(session)
    }

    async fn expire(&self, key: &str) -> Result<(), String> {
        self.kept.lock().await.remove(&crate::hex_sha256(key));
        self.inner.expire(key).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

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

    #[tokio::test]
    async fn evicts_by_key_hash_without_touching_other_keys() {
        let inner = Arc::new(Counting::default());
        let cached = CachedSessions::new(inner.clone(), Duration::from_secs(300));
        let a1 = cached.session("vtm_a").await.unwrap();
        let b1 = cached.session("vtm_b").await.unwrap();
        assert_eq!(cached.session("vtm_a").await.unwrap(), a1);
        assert_eq!(inner.sessions.load(Ordering::SeqCst), 2);

        let removed = cached.evict_hashes(&[crate::hex_sha256("vtm_a")]).await;
        assert_eq!(removed, 1);
        assert_eq!(cached.session("vtm_b").await.unwrap(), b1);
        assert_ne!(cached.session("vtm_a").await.unwrap(), a1);
        assert_eq!(inner.sessions.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn pending_wait_caps_below_convex_timeout_with_clear_tool_error() {
        assert!(PENDING_WAIT < Duration::from_secs(32));
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/v1/session"))
            .respond_with(
                wiremock::ResponseTemplate::new(202)
                    .set_body_json(serde_json::json!({"requestId": "req-1", "pollAfterMs": 10})),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/v1/session/requests/req-1"))
            .respond_with(
                wiremock::ResponseTemplate::new(202)
                    .set_body_json(serde_json::json!({"status": "pending", "stage": "phone"})),
            )
            .mount(&server)
            .await;

        let http = vtop_core::client::https_client(&[]).build().unwrap();
        let client =
            BridgeClient::new(http, server.uri()).with_pending_wait(Duration::from_millis(35));
        let err = client.session("vtm_k").await.unwrap_err();
        assert_eq!(err, PHONE_TIMEOUT_ERROR);
    }

    #[tokio::test]
    async fn expired_request_returns_clear_tool_error() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/v1/session"))
            .respond_with(
                wiremock::ResponseTemplate::new(202)
                    .set_body_json(serde_json::json!({"requestId": "req-gone", "pollAfterMs": 10})),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/v1/session/requests/req-gone"))
            .respond_with(
                wiremock::ResponseTemplate::new(410)
                    .set_body_json(serde_json::json!({"code": "request_expired"})),
            )
            .mount(&server)
            .await;

        let http = vtop_core::client::https_client(&[]).build().unwrap();
        let client = BridgeClient::new(http, server.uri());
        let err = client.session("vtm_k").await.unwrap_err();
        assert_eq!(err, PHONE_TIMEOUT_ERROR);
    }
}

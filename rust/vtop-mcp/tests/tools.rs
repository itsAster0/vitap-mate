use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use vtop_core::{SessionState, VtopError};
use vtop_mcp::bridge::{BridgeClient, SessionSource};
use vtop_mcp::config::Config;
use vtop_mcp::tools::with_retry;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const KEY: &str = "vtm_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn config_needs_only_the_bridge_url() {
    let c = Config::from_lookup(|name| {
        (name == "BRIDGE_URL").then(|| "https://bridge.example/".to_string())
    })
    .unwrap();
    assert_eq!(c.bridge_url, "https://bridge.example");
    assert_eq!(c.listen.port(), 8080);
    assert!(c.listen.is_ipv6());
    assert!(Config::from_lookup(|_| None).is_err());
}

struct FakeSource {
    sessions: AtomicUsize,
    expires: AtomicUsize,
}

impl FakeSource {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            sessions: AtomicUsize::new(0),
            expires: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl SessionSource for FakeSource {
    async fn whoami(&self, _key: &str) -> Result<Option<String>, String> {
        Ok(Some("22BCE0001".into()))
    }

    async fn session(&self, _key: &str) -> Result<SessionState, String> {
        self.sessions.fetch_add(1, Ordering::SeqCst);
        Ok(SessionState {
            cookies: "JSESSIONID=x".into(),
            csrf_token: Some("csrf".into()),
            registration_number: Some("22BCE0001".into()),
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
async fn retry_expires_and_retries_once() {
    let source = FakeSource::new();
    let calls = AtomicUsize::new(0);
    let result = with_retry(source.as_ref(), KEY, |_client| {
        let call = calls.fetch_add(1, Ordering::SeqCst);
        async move {
            if call == 0 {
                Err(VtopError::SessionExpired)
            } else {
                Ok(5)
            }
        }
    })
    .await;
    assert_eq!(result, Ok(5));
    assert_eq!(source.sessions.load(Ordering::SeqCst), 2);
    assert_eq!(source.expires.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn retry_gives_up_after_second_expiry() {
    let source = FakeSource::new();
    let result: Result<(), String> = with_retry(source.as_ref(), KEY, |_client| async {
        Err(VtopError::SessionExpired)
    })
    .await;
    assert_eq!(result, Err("Session has expired".to_string()));
    assert_eq!(source.sessions.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn other_errors_are_not_retried() {
    let source = FakeSource::new();
    let result: Result<(), String> = with_retry(source.as_ref(), KEY, |_client| async {
        Err(VtopError::NetworkError)
    })
    .await;
    assert_eq!(result, Err("Network connection error".to_string()));
    assert_eq!(source.expires.load(Ordering::SeqCst), 0);
}

fn client(server: &MockServer) -> BridgeClient {
    BridgeClient::new(reqwest::Client::new(), server.uri())
}

#[tokio::test]
async fn ready_session_uses_the_callers_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/session"))
        .and(header("authorization", format!("Bearer {KEY}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({ "status": "ready", "session": { "cookies": "A=1" }, "cookies": [] }),
        ))
        .mount(&server)
        .await;
    assert_eq!(client(&server).session(KEY).await.unwrap().cookies, "A=1");
}

#[tokio::test]
async fn pending_session_is_polled_until_ready() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/session"))
        .respond_with(ResponseTemplate::new(202).set_body_json(
            serde_json::json!({ "status": "pending", "requestId": "r1", "pollAfterMs": 10 }),
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/session/requests/r1"))
        .respond_with(
            ResponseTemplate::new(202).set_body_json(serde_json::json!({ "status": "pending" })),
        )
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/session/requests/r1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({ "status": "ready", "session": { "cookies": "A=phone" } }),
        ))
        .mount(&server)
        .await;
    assert_eq!(
        client(&server).session(KEY).await.unwrap().cookies,
        "A=phone"
    );
}

#[tokio::test]
async fn pending_error_surfaces() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(202).set_body_json(
            serde_json::json!({ "status": "pending", "requestId": "r1", "pollAfterMs": 10 }),
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": "error", "code": "phone_unreachable", "error": "phone_unreachable: Open VITAP Mate → Connected apps → Reconnect this phone."
    }))).mount(&server).await;
    let error = client(&server).session(KEY).await.unwrap_err();
    assert!(error.starts_with("phone_unreachable"), "{error}");
}

#[tokio::test]
async fn expired_request_is_phone_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(202).set_body_json(
            serde_json::json!({ "status": "pending", "requestId": "r1", "pollAfterMs": 10 }),
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(410)
                .set_body_json(serde_json::json!({ "code": "request_expired", "error": "x" })),
        )
        .mount(&server)
        .await;
    let error = client(&server).session(KEY).await.unwrap_err();
    assert!(error.starts_with("phone_timeout"), "{error}");
}

#[tokio::test]
async fn whoami_maps_401_to_unknown() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .and(header("authorization", format!("Bearer {KEY}").as_str()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "registrationNumber": "22BCE0001" })),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(serde_json::json!({ "code": "key_unknown", "error": "x" })),
        )
        .mount(&server)
        .await;
    assert_eq!(
        client(&server).whoami(KEY).await.unwrap().as_deref(),
        Some("22BCE0001")
    );
    assert_eq!(client(&server).whoami("vtm_other").await.unwrap(), None);
}

/// Railway: the first request to a sleeping bridge may get a 502.
#[tokio::test]
async fn bridge_client_retries_once_after_a_502() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(502))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({ "status": "ready", "session": { "cookies": "A=1" } }),
        ))
        .mount(&server)
        .await;
    assert_eq!(client(&server).session(KEY).await.unwrap().cookies, "A=1");
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn expire_posts_with_the_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/session/expire"))
        .and(header("authorization", format!("Bearer {KEY}").as_str()))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    client(&server).expire(KEY).await.unwrap();
}

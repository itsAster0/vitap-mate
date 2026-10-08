//! Data messages to the phone through FCM HTTP v1.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::google::TokenSource;

pub const DEFAULT_BASE_URL: &str = "https://fcm.googleapis.com/v1";
const SEND_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FcmError {
    /// The registration token is unknown, malformed or for another sender.
    TokenInvalid,
    /// Our credentials are wrong or lack permission.
    Misconfigured,
    Unavailable,
}

#[async_trait]
pub trait Messenger: Send + Sync {
    async fn send(&self, fcm_token: &str, data: &BTreeMap<String, String>) -> Result<(), FcmError>;
}

pub struct FcmMessenger {
    http: reqwest::Client,
    tokens: Arc<dyn TokenSource>,
    base_url: String,
}

impl FcmMessenger {
    pub fn new(http: reqwest::Client, tokens: Arc<dyn TokenSource>, base_url: String) -> Self {
        Self {
            http,
            tokens,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

/// Maps an FCM error response to [`FcmError`].
fn classify(status: reqwest::StatusCode, body: &Value) -> FcmError {
    let details = body["error"]["details"].as_array();
    let has_code = |want: &[&str]| {
        details.is_some_and(|details| {
            details
                .iter()
                .any(|d| d["errorCode"].as_str().is_some_and(|c| want.contains(&c)))
        })
    };
    let grpc_status = body["error"]["status"].as_str().unwrap_or_default();

    if has_code(&["UNREGISTERED", "INVALID_ARGUMENT", "SENDER_ID_MISMATCH"]) {
        return FcmError::TokenInvalid;
    }
    if grpc_status == "INVALID_ARGUMENT" {
        return FcmError::TokenInvalid;
    }
    if has_code(&["THIRD_PARTY_AUTH_ERROR"])
        || status == reqwest::StatusCode::UNAUTHORIZED
        || status == reqwest::StatusCode::FORBIDDEN
    {
        return FcmError::Misconfigured;
    }
    FcmError::Unavailable
}

#[async_trait]
impl Messenger for FcmMessenger {
    async fn send(&self, fcm_token: &str, data: &BTreeMap<String, String>) -> Result<(), FcmError> {
        let token = self
            .tokens
            .token()
            .await
            .map_err(|_| FcmError::Misconfigured)?;
        let url = format!(
            "{}/projects/{}/messages:send",
            self.base_url,
            self.tokens.project_id()
        );
        let body = json!({
            "message": {
                "token": fcm_token,
                "data": data,
                "android": { "priority": "HIGH", "ttl": "120s" }
            }
        });
        let response = self
            .http
            .post(url)
            .bearer_auth(token)
            .timeout(SEND_TIMEOUT)
            .json(&body)
            .send()
            .await
            .map_err(|_| FcmError::Unavailable)?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        Err(classify(status, &body))
    }
}

/// Records sends and answers with `result`, for tests.
pub struct FakeMessenger {
    pub sent: Mutex<Vec<(String, BTreeMap<String, String>)>>,
    pub result: Mutex<Result<(), FcmError>>,
}

impl Default for FakeMessenger {
    fn default() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
            result: Mutex::new(Ok(())),
        }
    }
}

impl FakeMessenger {
    pub fn sends(&self) -> Vec<(String, BTreeMap<String, String>)> {
        self.sent.lock().unwrap().clone()
    }
}

#[async_trait]
impl Messenger for FakeMessenger {
    async fn send(&self, fcm_token: &str, data: &BTreeMap<String, String>) -> Result<(), FcmError> {
        self.sent
            .lock()
            .unwrap()
            .push((fcm_token.to_string(), data.clone()));
        *self.result.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::google::StaticTokens;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const SEND_PATH: &str = "/projects/p/messages:send";

    fn data() -> BTreeMap<String, String> {
        BTreeMap::from([("type".to_string(), "vtop_cookie_request".to_string())])
    }

    async fn send_with(response: ResponseTemplate) -> Result<(), FcmError> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(SEND_PATH))
            .respond_with(response)
            .mount(&server)
            .await;
        FcmMessenger::new(reqwest::Client::new(), StaticTokens::arc("p"), server.uri())
            .send("device", &data())
            .await
    }

    fn fcm_error(status: u16, grpc: &str, code: Option<&str>) -> ResponseTemplate {
        let details = match code {
            Some(code) => json!([{
                "@type": "type.googleapis.com/google.firebase.fcm.v1.FcmError",
                "errorCode": code
            }]),
            None => json!([]),
        };
        ResponseTemplate::new(status).set_body_json(
            json!({ "error": { "code": status, "status": grpc, "details": details } }),
        )
    }

    #[tokio::test]
    async fn sends_high_priority_data_message() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(SEND_PATH))
            .and(header("authorization", "Bearer test-token"))
            .and(body_json(json!({ "message": {
                "token": "device",
                "data": { "type": "vtop_cookie_request" },
                "android": { "priority": "HIGH", "ttl": "120s" }
            } })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "name": "m" })))
            .expect(1)
            .mount(&server)
            .await;
        FcmMessenger::new(reqwest::Client::new(), StaticTokens::arc("p"), server.uri())
            .send("device", &data())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn unregistered_maps_to_token_invalid() {
        assert_eq!(
            send_with(fcm_error(404, "NOT_FOUND", Some("UNREGISTERED"))).await,
            Err(FcmError::TokenInvalid)
        );
    }

    #[tokio::test]
    async fn invalid_argument_maps_to_token_invalid() {
        assert_eq!(
            send_with(fcm_error(400, "INVALID_ARGUMENT", Some("INVALID_ARGUMENT"))).await,
            Err(FcmError::TokenInvalid)
        );
        assert_eq!(
            send_with(fcm_error(400, "INVALID_ARGUMENT", None)).await,
            Err(FcmError::TokenInvalid)
        );
    }

    #[tokio::test]
    async fn sender_mismatch_maps_to_token_invalid() {
        assert_eq!(
            send_with(fcm_error(
                403,
                "PERMISSION_DENIED",
                Some("SENDER_ID_MISMATCH")
            ))
            .await,
            Err(FcmError::TokenInvalid)
        );
    }

    #[tokio::test]
    async fn forbidden_maps_to_misconfigured() {
        assert_eq!(
            send_with(fcm_error(403, "PERMISSION_DENIED", None)).await,
            Err(FcmError::Misconfigured)
        );
        assert_eq!(
            send_with(fcm_error(
                401,
                "UNAUTHENTICATED",
                Some("THIRD_PARTY_AUTH_ERROR")
            ))
            .await,
            Err(FcmError::Misconfigured)
        );
    }

    #[tokio::test]
    async fn server_error_maps_to_unavailable() {
        assert_eq!(
            send_with(fcm_error(503, "UNAVAILABLE", Some("UNAVAILABLE"))).await,
            Err(FcmError::Unavailable)
        );
        assert_eq!(
            send_with(ResponseTemplate::new(500)).await,
            Err(FcmError::Unavailable)
        );
    }

    #[tokio::test]
    async fn unreachable_server_is_unavailable() {
        let messenger = FcmMessenger::new(
            reqwest::Client::new(),
            StaticTokens::arc("p"),
            "http://127.0.0.1:9".into(),
        );
        assert_eq!(
            messenger.send("device", &data()).await,
            Err(FcmError::Unavailable)
        );
    }
}

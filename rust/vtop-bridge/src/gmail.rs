//! Server sign-in with the OTP read from the student's Gmail, for students
//! without a forwarding address. The phone hands over its Gmail refresh
//! token with the OAuth client it was issued to (the app's own on phones,
//! the student's own on desktop), sealed with the saved credentials; each
//! sign-in turns it into a short-lived access token here.

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use vtop_core::gmail::{self, GmailAccess};
pub use vtop_shared::engine::GmailGrant;

use crate::login::OtpCodes;

pub const DEFAULT_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REFRESH_TIMEOUT: Duration = Duration::from_secs(15);

/// No `Debug`: it holds the access token.
pub struct FreshToken {
    pub access_token: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshError {
    /// Google refused the refresh token: revoked, expired or for another
    /// client. The grant is dead.
    InvalidGrant,
    Unavailable(String),
}

#[async_trait]
pub trait GmailRefresher: Send + Sync {
    async fn refresh(&self, grant: &GmailGrant) -> Result<FreshToken, RefreshError>;
}

pub struct GoogleOAuthRefresher {
    http: reqwest::Client,
    token_url: String,
}

impl GoogleOAuthRefresher {
    pub fn new(http: reqwest::Client, token_url: String) -> Self {
        Self { http, token_url }
    }
}

#[async_trait]
impl GmailRefresher for GoogleOAuthRefresher {
    async fn refresh(&self, grant: &GmailGrant) -> Result<FreshToken, RefreshError> {
        let mut form = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", grant.refresh_token.as_str()),
            ("client_id", grant.client_id.as_str()),
        ];
        if let Some(secret) = grant.client_secret.as_deref() {
            form.push(("client_secret", secret));
        }
        let unavailable =
            |reason: &str| RefreshError::Unavailable(format!("gmail refresh: {reason}"));
        let response = self
            .http
            .post(&self.token_url)
            .form(&form)
            .timeout(REFRESH_TIMEOUT)
            .send()
            .await
            .map_err(|error| unavailable(&error.without_url().to_string()))?;
        let status = response.status();
        let body: Value = response.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            if body.get("error").and_then(Value::as_str) == Some("invalid_grant") {
                return Err(RefreshError::InvalidGrant);
            }
            let error = body
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("no error code");
            return Err(unavailable(&format!("HTTP {status}, {error}")));
        }
        let access_token = body
            .get("access_token")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
            .ok_or_else(|| unavailable("no access_token in reply"))?;
        let expires_in = body
            .get("expires_in")
            .and_then(Value::as_u64)
            .unwrap_or(3600);
        Ok(FreshToken {
            access_token: access_token.to_string(),
            expires_at: vtop_core::now_unix() + expires_in,
        })
    }
}

/// OTP codes from the student's Gmail, read with a fresh access token.
pub struct GmailCodes {
    access: GmailAccess,
    /// Each code handed out, with its message, so a code VTOP rejects is
    /// not read again and the accepted one's email can be tidied.
    handed_out: Mutex<Vec<(String, String)>>,
}

impl GmailCodes {
    pub fn new(token: FreshToken, delete_after_reading: bool) -> Self {
        Self {
            access: GmailAccess {
                access_token: token.access_token,
                delete_after_reading,
                expires_at: Some(token.expires_at),
            },
            handed_out: Mutex::new(Vec::new()),
        }
    }

    /// Trashes or marks read the email of the last code handed out (the
    /// one VTOP took). Best effort.
    pub async fn tidy_up(&self) {
        let last = self.handed_out.lock().unwrap().last().cloned();
        if let Some((_, message_id)) = last {
            gmail::tidy_up(&self.access, &message_id).await;
        }
    }
}

#[async_trait]
impl OtpCodes for GmailCodes {
    async fn next(
        &self,
        issued_at: u64,
        remaining: Duration,
        tried: Vec<String>,
    ) -> Option<String> {
        let skip: HashSet<String> = self
            .handed_out
            .lock()
            .unwrap()
            .iter()
            .filter(|(code, _)| tried.contains(code))
            .map(|(_, message_id)| message_id.clone())
            .collect();
        match gmail::wait_for_otp(&self.access, issued_at, remaining, &skip).await {
            Ok(found) => {
                self.handed_out
                    .lock()
                    .unwrap()
                    .push((found.code.clone(), found.message_id));
                Some(found.code)
            }
            Err(error) => {
                tracing::info!("engine: no OTP from Gmail: {}", error.code());
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    fn grant(secret: Option<&str>) -> GmailGrant {
        GmailGrant {
            refresh_token: "refresh-1".into(),
            client_id: "client.apps.googleusercontent.com".into(),
            client_secret: secret.map(String::from),
            delete_after_reading: true,
        }
    }

    fn refresher(server: &MockServer) -> GoogleOAuthRefresher {
        GoogleOAuthRefresher::new(reqwest::Client::new(), format!("{}/token", server.uri()))
    }

    fn form(request: &Request) -> Vec<(String, String)> {
        url::form_urlencoded::parse(&request.body)
            .into_owned()
            .collect()
    }

    #[tokio::test]
    async fn refresh_sends_the_secret_only_when_there_is_one() {
        for secret in [None, Some("shh")] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/token"))
                .and(body_string_contains("grant_type=refresh_token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({ "access_token": "at", "expires_in": 3599, "token_type": "Bearer" }),
                ))
                .mount(&server)
                .await;
            refresher(&server).refresh(&grant(secret)).await.unwrap();
            let requests = server.received_requests().await.unwrap();
            let fields = form(&requests[0]);
            let get = |name: &str| {
                fields
                    .iter()
                    .find(|(key, _)| key == name)
                    .map(|(_, value)| value.clone())
            };
            assert_eq!(get("refresh_token").as_deref(), Some("refresh-1"));
            assert_eq!(
                get("client_id").as_deref(),
                Some("client.apps.googleusercontent.com")
            );
            assert_eq!(get("client_secret").as_deref(), secret);
        }
    }

    #[tokio::test]
    async fn a_fresh_token_knows_when_it_expires() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "access_token": "at", "expires_in": 3599 })),
            )
            .mount(&server)
            .await;
        let before = vtop_core::now_unix();
        let token = refresher(&server).refresh(&grant(None)).await.unwrap();
        assert_eq!(token.access_token, "at");
        assert!(
            token.expires_at >= before + 3599 && token.expires_at <= vtop_core::now_unix() + 3599
        );
    }

    #[tokio::test]
    async fn a_revoked_token_is_an_invalid_grant() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400).set_body_json(
                serde_json::json!({ "error": "invalid_grant", "error_description": "Token has been expired or revoked." }),
            ))
            .mount(&server)
            .await;
        let result = refresher(&server).refresh(&grant(None)).await;
        assert_eq!(result.err(), Some(RefreshError::InvalidGrant));
    }

    #[tokio::test]
    async fn other_failures_are_unavailable() {
        for response in [
            ResponseTemplate::new(500),
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({ "error": "invalid_client" })),
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "expires_in": 10 })),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(response)
                .mount(&server)
                .await;
            let result = refresher(&server).refresh(&grant(None)).await;
            assert!(matches!(result.err(), Some(RefreshError::Unavailable(_))));
        }
    }
}

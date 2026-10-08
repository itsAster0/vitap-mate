//! Turns a saved Gmail refresh token into a short-lived access token.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REFRESH_TIMEOUT: Duration = Duration::from_secs(15);

/// What the phone hands over so the bridge can read OTP emails by itself.
/// No `Debug`: it holds the refresh token and maybe a client secret.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GmailGrant {
    pub refresh_token: String,
    pub client_id: String,
    /// Only for a personal (BYOK) OAuth client.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub delete_after_reading: bool,
}

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
        if let Some(secret) = grant
            .client_secret
            .as_deref()
            .filter(|secret| !secret.is_empty())
        {
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

    async fn refresher(server: &MockServer) -> GoogleOAuthRefresher {
        GoogleOAuthRefresher::new(reqwest::Client::new(), format!("{}/token", server.uri()))
    }

    fn form(request: &Request) -> Vec<(String, String)> {
        url::form_urlencoded::parse(&request.body)
            .into_owned()
            .collect()
    }

    #[tokio::test]
    async fn refresh_sends_secret_only_when_present() {
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
            refresher(&server)
                .await
                .refresh(&grant(secret))
                .await
                .unwrap();
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
    async fn success_computes_expires_at() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "access_token": "at", "expires_in": 3599 })),
            )
            .mount(&server)
            .await;
        let before = vtop_core::now_unix();
        let token = refresher(&server)
            .await
            .refresh(&grant(None))
            .await
            .unwrap();
        assert_eq!(token.access_token, "at");
        assert!(
            token.expires_at >= before + 3599 && token.expires_at <= vtop_core::now_unix() + 3599
        );
    }

    #[tokio::test]
    async fn invalid_grant_maps() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400).set_body_json(
                serde_json::json!({ "error": "invalid_grant", "error_description": "Token has been expired or revoked." }),
            ))
            .mount(&server)
            .await;
        let result = refresher(&server).await.refresh(&grant(None)).await;
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
            let result = refresher(&server).await.refresh(&grant(None)).await;
            assert!(matches!(result.err(), Some(RefreshError::Unavailable(_))));
        }
    }
}

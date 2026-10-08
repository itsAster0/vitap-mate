//! Google OAuth access tokens for Firestore and FCM, from the Firebase
//! service account.

use std::sync::Arc;

use async_trait::async_trait;
use gcp_auth::{CustomServiceAccount, TokenProvider};

use crate::store::StoreError;

const SCOPES: &[&str] = &["https://www.googleapis.com/auth/cloud-platform"];

#[async_trait]
pub trait TokenSource: Send + Sync {
    async fn token(&self) -> Result<String, StoreError>;
    fn project_id(&self) -> &str;
}

pub struct ServiceAccountTokens {
    account: CustomServiceAccount,
    project_id: String,
}

impl ServiceAccountTokens {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let account = CustomServiceAccount::from_json(json)
            .map_err(|_| "FIREBASE_CREDENTIALS_JSON is not a service account".to_string())?;
        let project_id = account
            .project_id()
            .map(String::from)
            .ok_or_else(|| "FIREBASE_CREDENTIALS_JSON has no project_id".to_string())?;
        Ok(Self {
            account,
            project_id,
        })
    }
}

#[async_trait]
impl TokenSource for ServiceAccountTokens {
    async fn token(&self) -> Result<String, StoreError> {
        // gcp_auth caches the token until shortly before it expires.
        self.account
            .token(SCOPES)
            .await
            .map(|token| token.as_str().to_string())
            .map_err(|error| StoreError(format!("google auth: {error}")))
    }

    fn project_id(&self) -> &str {
        &self.project_id
    }
}

/// A fixed token, for tests against mock servers.
pub struct StaticTokens {
    pub token: String,
    pub project_id: String,
}

impl StaticTokens {
    pub fn arc(project_id: &str) -> Arc<dyn TokenSource> {
        Arc::new(Self {
            token: "test-token".into(),
            project_id: project_id.into(),
        })
    }
}

#[async_trait]
impl TokenSource for StaticTokens {
    async fn token(&self) -> Result<String, StoreError> {
        Ok(self.token.clone())
    }

    fn project_id(&self) -> &str {
        &self.project_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_service_account_json() {
        assert!(ServiceAccountTokens::from_json("{}").is_err());
        assert!(ServiceAccountTokens::from_json("{\"type\":\"authorized_user\"}").is_err());
    }
}

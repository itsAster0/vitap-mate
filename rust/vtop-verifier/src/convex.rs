//! Convex's two proof routes, with `Authorization: Bearer $VERIFIER_TOKEN`. All proof state
//! (tickets, proved marks, owner hashes) is Convex's; the verifier keeps none.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Uses up a ticket, `{ticket}` → `{device}`, or 404 when it is unknown, used or expired.
pub const PROOF_TICKET: &str = "/internal/proof-ticket";
/// The verdict on a ticket's proof ([`ProofDone`]).
pub const PROOF_DONE: &str = "/internal/proof-done";

/// The verdict sent to [`PROOF_DONE`]: `{ticket, regno}`, or `{ticket, code}`. Convex asks the
/// engine for the regno's owner hash itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofDone {
    pub ticket: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regno: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[async_trait]
pub trait ConvexLink: Send + Sync {
    /// Uses up a proof ticket: the device it was issued to, or `None` when it is unknown,
    /// used or expired.
    async fn proof_ticket(&self, ticket: &str) -> Result<Option<String>, ConvexError>;
    /// The verdict on a ticket's proof.
    async fn proof_done(&self, done: &ProofDone) -> Result<(), ConvexError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConvexError {
    Unavailable(String),
    /// Convex is at its proof cap for everyone (429).
    Busy,
}

pub struct HttpConvex {
    pub http: reqwest::Client,
    pub base_url: String,
    pub token: String,
}

impl HttpConvex {
    async fn post(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<reqwest::Response, ConvexError> {
        self.http
            .post(format!("{}{path}", self.base_url))
            .bearer_auth(&self.token)
            .timeout(Duration::from_secs(40))
            .json(body)
            .send()
            .await
            .map_err(|error| ConvexError::Unavailable(error.without_url().to_string()))
    }
}

#[async_trait]
impl ConvexLink for HttpConvex {
    async fn proof_ticket(&self, ticket: &str) -> Result<Option<String>, ConvexError> {
        #[derive(Deserialize)]
        struct Ticketed {
            device: String,
        }
        let response = self
            .post(PROOF_TICKET, &json!({ "ticket": ticket }))
            .await?;
        match response.status().as_u16() {
            404 => return Ok(None),
            429 => return Err(ConvexError::Busy),
            _ => {}
        }
        succeeded(&response)?;
        let body: Ticketed = response
            .json()
            .await
            .map_err(|error| ConvexError::Unavailable(error.without_url().to_string()))?;
        Ok(Some(body.device))
    }

    async fn proof_done(&self, done: &ProofDone) -> Result<(), ConvexError> {
        let response = self.post(PROOF_DONE, done).await?;
        succeeded(&response)
    }
}

fn succeeded(response: &reqwest::Response) -> Result<(), ConvexError> {
    if response.status().is_success() {
        Ok(())
    } else {
        Err(ConvexError::Unavailable(format!(
            "Convex answered {}",
            response.status()
        )))
    }
}

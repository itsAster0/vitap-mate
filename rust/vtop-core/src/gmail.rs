//! Reads VTOP's security OTP from the user's Gmail. Shared by the app (the
//! OTP prompt's email auto-fetch, over FFI) and the engine (finishing a
//! login in one request), so both find and parse the email the same way:
//! the newest mail from `noreply.sdc@vitap.ac.in` after the OTP was issued,
//! the first standalone 6-digit number in it, then trash it or mark it read.
//!
//! Only a short-lived OAuth access token is taken; obtaining and refreshing
//! it stays with the caller. The token is never stored or logged.

use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::Value;
/// The parser, as `vtop_core::gmail` has without the `reqwest` feature too.
pub use vtop_proto::gmail::{Body, Header, Message, Part, SENDER, otp};

const API: &str = "https://gmail.googleapis.com/gmail/v1/users/me/messages";
const POLL_EVERY: Duration = Duration::from_secs(3);
/// Tolerance for clock differences between VTOP, Gmail and this server.
const CLOCK_SLACK_SECS: u64 = 30;

/// A plain HTTPS client for the Gmail API (not the VTOP one: no VTOP
/// headers or certificates). Built once.
fn http() -> Result<&'static reqwest::Client, GmailOtpError> {
    static CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            crate::client::https_client(&[])
                .timeout(Duration::from_secs(15))
                .build()
                .ok()
        })
        .as_ref()
        .ok_or(GmailOtpError::Unavailable)
}

/// Gmail access lent for one operation. No `Debug`: it holds a token.
#[derive(Deserialize)]
pub struct GmailAccess {
    pub access_token: String,
    /// Trash the OTP email once read (the app's "Delete After Reading");
    /// otherwise it is only marked read.
    #[serde(default)]
    pub delete_after_reading: bool,
    /// Unix seconds when the access token expires, if known. Waiting stops
    /// before this instead of polling with a dead token.
    #[serde(default)]
    pub expires_at: Option<u64>,
}

impl GmailAccess {
    /// True when the token is expired or will be within `margin`.
    pub fn expires_within(&self, margin: Duration) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= crate::now_unix() + margin.as_secs())
    }
}

/// An OTP found in the mailbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GmailOtp {
    pub code: String,
    /// Gmail message id, so a code VTOP rejects is not tried again.
    pub message_id: String,
}

/// Why no OTP came back. Never includes the token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GmailOtpError {
    /// Gmail rejected the token (HTTP 401) or it expired: refresh it and
    /// try again.
    Unauthorized,
    /// Gmail refused access (HTTP 403): missing scope, or the Gmail API is
    /// not enabled for the OAuth client.
    Forbidden,
    /// No matching email arrived before the deadline.
    NotFound,
    Unavailable,
}

/// One look at the mailbox: the newest OTP email sent after `issued_at`
/// (unix seconds), skipping message ids in `tried`.
pub async fn find_otp(
    access: &GmailAccess,
    issued_at: u64,
    tried: &HashSet<String>,
) -> Result<Option<GmailOtp>, GmailOtpError> {
    if access.expires_within(Duration::from_secs(5)) {
        return Err(GmailOtpError::Unauthorized);
    }
    let since = issued_at.saturating_sub(CLOCK_SLACK_SECS);
    newest_otp(http()?, access, since, tried).await
}

impl GmailOtpError {
    /// Stable machine-readable name, used in API replies and FFI errors.
    pub fn code(self) -> &'static str {
        match self {
            Self::Unauthorized => "gmail_unauthorized",
            Self::Forbidden => "gmail_forbidden",
            Self::NotFound => "gmail_otp_not_found",
            Self::Unavailable => "gmail_unavailable",
        }
    }
}

/// Polls with [`find_otp`] every few seconds until an OTP shows up or `wait`
/// runs out.
pub async fn wait_for_otp(
    access: &GmailAccess,
    issued_at: u64,
    wait: Duration,
    tried: &HashSet<String>,
) -> Result<GmailOtp, GmailOtpError> {
    let deadline = Instant::now() + wait;
    loop {
        if let Some(found) = find_otp(access, issued_at, tried).await? {
            return Ok(found);
        }
        if Instant::now() + POLL_EVERY > deadline {
            return Err(GmailOtpError::NotFound);
        }
        // Stop before the token lapses rather than failing mid-poll.
        if access.expires_within(POLL_EVERY + Duration::from_secs(5)) {
            return Err(GmailOtpError::Unauthorized);
        }
        tokio::time::sleep(POLL_EVERY).await;
    }
}

async fn get_json(
    http: &reqwest::Client,
    access: &GmailAccess,
    url: &str,
    query: &[(&str, String)],
) -> Result<Value, GmailOtpError> {
    let started = Instant::now();
    let response = http
        .get(url)
        .bearer_auth(access.access_token.trim())
        .query(query)
        .send()
        .await
        .map_err(|error| {
            log::warn!(target: "rust.network", "gmail GET failed: {error}");
            GmailOtpError::Unavailable
        })?;
    log::info!(
        target: "rust.network",
        "gmail GET {} {} {}ms for the sign-in OTP",
        response.url().path(),
        response.status().as_u16(),
        started.elapsed().as_millis()
    );
    match response.status().as_u16() {
        200 => response
            .json::<Value>()
            .await
            .map_err(|_| GmailOtpError::Unavailable),
        401 => Err(GmailOtpError::Unauthorized),
        403 => Err(GmailOtpError::Forbidden),
        _ => Err(GmailOtpError::Unavailable),
    }
}

async fn newest_otp(
    http: &reqwest::Client,
    access: &GmailAccess,
    since: u64,
    tried: &HashSet<String>,
) -> Result<Option<GmailOtp>, GmailOtpError> {
    let list = get_json(
        http,
        access,
        API,
        &[
            ("maxResults", "3".to_string()),
            ("q", format!("from:{SENDER} after:{since}")),
        ],
    )
    .await?;
    let ids = list
        .get("messages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|message| message.get("id").and_then(Value::as_str))
        .filter(|id| !tried.contains(*id))
        .map(str::to_string)
        .collect::<Vec<_>>();

    for id in ids {
        let message = get_json(
            http,
            access,
            &format!("{API}/{id}"),
            &[("format", "full".to_string())],
        )
        .await?;
        let message = serde_json::from_value::<Message>(message).unwrap_or_default();
        if let Some(code) = otp(&message, since) {
            return Ok(Some(GmailOtp {
                code,
                message_id: id,
            }));
        }
    }
    Ok(None)
}

/// Trashes the message or marks it read, per
/// [`GmailAccess::delete_after_reading`]. Best effort: a failure only means
/// the email stays in the inbox.
pub async fn tidy_up(access: &GmailAccess, id: &str) {
    let Ok(http) = http() else {
        return;
    };
    let request = if access.delete_after_reading {
        http.post(format!("{API}/{id}/trash"))
    } else {
        http.post(format!("{API}/{id}/modify"))
            .json(&serde_json::json!({ "removeLabelIds": ["UNREAD"] }))
    };
    let result = request.bearer_auth(access.access_token.trim()).send().await;
    log::info!(
        target: "rust.network",
        "gmail POST tidy up OTP email: {}",
        result.map_or_else(|error| error.to_string(), |r| r.status().to_string())
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiring_tokens_are_refused_up_front() {
        let access = GmailAccess {
            access_token: "t".into(),
            delete_after_reading: false,
            expires_at: Some(crate::now_unix() + 2),
        };
        assert!(access.expires_within(Duration::from_secs(5)));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(find_otp(&access, 0, &HashSet::new()));
        assert_eq!(result, Err(GmailOtpError::Unauthorized));
        let fresh = GmailAccess {
            expires_at: Some(crate::now_unix() + 3600),
            ..access
        };
        assert!(!fresh.expires_within(Duration::from_secs(60)));
    }
}

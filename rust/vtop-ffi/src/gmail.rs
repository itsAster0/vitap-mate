//! VTOP's sign-in OTP read from the student's Gmail with their own Google OAuth client (BYOK):
//! import a Desktop-app client JSON, sign in in the browser (loopback redirect with PKCE S256),
//! find the newest mail from [`SENDER`], fill in its 6-digit code and trash it (or mark it read).
//!
//! The account (refresh token, client secret) is sealed through [`crate::SecretBox`] under the
//! exact name (`"gmail"`) and JSON format Kotlin's `Gmail.kt` used, so existing linked accounts
//! survive the update. Tokens, codes, mail contents and the email address are never logged.

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, warn};
use vtop_core::client::{Hop, Transport, TransportError};
use vtop_core::gmail as proto_gmail;
use vtop_shared::base64::Engine as _;
use vtop_shared::base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::settings::{OtpSource, SettingChange};
use crate::{Core, Shared, traced, traced_sync};

const SENDER: &str = "noreply.sdc@vitap.ac.in";
pub(crate) const SECRET_NAME: &str = "gmail";
const SECRET_FILE: &str = "gmail.sealed";
const SCOPE: &str = "https://www.googleapis.com/auth/gmail.modify";
const AUTH: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN: &str = "https://oauth2.googleapis.com/token";
const REVOKE: &str = "https://oauth2.googleapis.com/revoke";
const API: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
const CLIENT_SUFFIX: &str = ".apps.googleusercontent.com";
const MAX_CLIENT_BYTES: usize = 64 * 1024;

/// VTOP's, Gmail's and the phone's clocks differ a little.
const CLOCK_SLACK_S: i64 = 30;
/// How long the browser sign-in may take.
const SIGN_IN_MS: u64 = 10 * 60_000;
const REACHABLE_WINDOW_MS: i64 = 3 * 60_000;
const RETRY_BASE_MS: u64 = 2_000;
const RETRY_MAX_MS: u64 = 15_000;
const HTTP_TIMEOUT: Duration = Duration::from_secs(20);

/// A Desktop-app OAuth client from Google Cloud's JSON.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct GmailClient {
    pub id: String,
    pub secret: Option<String>,
    pub project: String,
}

impl fmt::Debug for GmailClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GmailClient")
            .field("project", &self.project)
            .field("has_secret", &self.secret.is_some())
            .finish_non_exhaustive()
    }
}

/// The connected Gmail account.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct GmailAccount {
    pub email: String,
    pub client: GmailClient,
    pub refresh_token: String,
    pub delete_after_reading: bool,
    /// When Google ends the grant (unix ms): a project in Testing gets 7 days; 0 when unset.
    pub grant_ends_at_ms: i64,
    /// Google turned the grant down: reconnect with the same client.
    pub ended: bool,
}

impl fmt::Debug for GmailAccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GmailAccount")
            .field("delete_after_reading", &self.delete_after_reading)
            .field("grant_ends_at_ms", &self.grant_ends_at_ms)
            .field("ended", &self.ended)
            .finish_non_exhaustive()
    }
}

/// A code found in the mailbox; `message_id` skips a code VTOP turns down.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct GmailOtp {
    pub code: String,
    pub message_id: String,
}

impl fmt::Debug for GmailOtp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GmailOtp").finish_non_exhaustive()
    }
}

/// What went wrong: `reason` for the student, stable `id` for logs and `PauseReason`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum GmailError {
    Unreachable { reason: String },
    Offline { reason: String },
    BadFile { reason: String },
    BadJson { reason: String },
    WrongClientType { reason: String },
    BadClientId { reason: String },
    NoTokens { reason: String },
    NoScope { reason: String },
    NoEmail { reason: String },
    WrongAccount { reason: String },
    GmailOff { reason: String },
    NotConnected { reason: String },
    SignInRefused { code: String, reason: String },
    BadState { reason: String },
    Timeout { reason: String },
    Unauthorized { reason: String },
    Forbidden { status: String, reason: String },
    InvalidGrant { reason: String },
    InvalidClient { reason: String },
    Http { status: u16, reason: String },
    Ended { reason: String },
}

impl GmailError {
    pub(crate) fn id(&self) -> String {
        match self {
            Self::Unreachable { .. } => "unreachable".into(),
            Self::Offline { .. } => "offline".into(),
            Self::BadFile { .. } => "bad_file".into(),
            Self::BadJson { .. } => "bad_json".into(),
            Self::WrongClientType { .. } => "wrong_client_type".into(),
            Self::BadClientId { .. } => "bad_client_id".into(),
            Self::NoTokens { .. } => "no_tokens".into(),
            Self::NoScope { .. } => "no_scope".into(),
            Self::NoEmail { .. } => "no_email".into(),
            Self::WrongAccount { .. } => "wrong_account".into(),
            Self::GmailOff { .. } => "gmail_off".into(),
            Self::NotConnected { .. } => "not_connected".into(),
            Self::SignInRefused { code, .. } => code.clone(),
            Self::BadState { .. } => "bad_state".into(),
            Self::Timeout { .. } => "timeout".into(),
            Self::Unauthorized { .. } => "unauthorized".into(),
            Self::Forbidden { .. } => "forbidden".into(),
            Self::InvalidGrant { .. } => "invalid_grant".into(),
            Self::InvalidClient { .. } => "invalid_client".into(),
            Self::Http { status, .. } => format!("http_{status}"),
            Self::Ended { .. } => "ended".into(),
        }
    }

    fn unreachable() -> Self {
        Self::Unreachable {
            reason: "Couldn't reach Google. Check your connection.".into(),
        }
    }

    fn offline() -> Self {
        Self::Offline {
            reason: "Couldn't reach Google. Check your connection.".into(),
        }
    }

    fn ended() -> Self {
        Self::Ended {
            reason: "Google ended the Gmail connection (a project in Testing does this every 7 days). Reconnect it in Settings → OTP auto-fill.".into(),
        }
    }

    fn timeout() -> Self {
        Self::Timeout {
            reason: "Google sign-in took too long. Try again.".into(),
        }
    }

    fn sign_in_refused(error: &str) -> Self {
        let reason = if error == "access_denied" {
            "Google denied access. Is your college email a test user (step 4)?".into()
        } else {
            format!("Google sign-in failed: {error}.")
        };
        Self::SignInRefused {
            code: error.into(),
            reason,
        }
    }
}

impl fmt::Display for GmailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::Unreachable { reason }
            | Self::Offline { reason }
            | Self::BadFile { reason }
            | Self::BadJson { reason }
            | Self::WrongClientType { reason }
            | Self::BadClientId { reason }
            | Self::NoTokens { reason }
            | Self::NoScope { reason }
            | Self::NoEmail { reason }
            | Self::WrongAccount { reason }
            | Self::GmailOff { reason }
            | Self::NotConnected { reason }
            | Self::SignInRefused { reason, .. }
            | Self::BadState { reason }
            | Self::Timeout { reason }
            | Self::Unauthorized { reason }
            | Self::Forbidden { reason, .. }
            | Self::InvalidGrant { reason }
            | Self::InvalidClient { reason }
            | Self::Http { reason, .. }
            | Self::Ended { reason } => reason,
        };
        f.write_str(reason)
    }
}

impl std::error::Error for GmailError {}

/// Exact JSON shape sealed under [`SECRET_NAME`] by Kotlin's `Gmail.kt`.
#[derive(Clone, Serialize, Deserialize)]
struct StoredGmailAccount {
    email: String,
    client_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client_secret: Option<String>,
    #[serde(default)]
    project: String,
    refresh_token: String,
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    expires_at_ms: i64,
    #[serde(default = "default_true")]
    delete_after_reading: bool,
    #[serde(default)]
    grant_ends_at_ms: i64,
    #[serde(default)]
    ended: bool,
}

fn default_true() -> bool {
    true
}

impl StoredGmailAccount {
    fn normalized(mut self) -> Self {
        if self.client_secret.as_deref() == Some("") {
            self.client_secret = None;
        }
        self
    }

    fn needs_reconnect(&self, now_ms: i64) -> bool {
        self.ended || (self.grant_ends_at_ms > 0 && now_ms >= self.grant_ends_at_ms)
    }

    fn days_left(&self, now_ms: i64) -> Option<i64> {
        (self.grant_ends_at_ms > 0).then(|| ((self.grant_ends_at_ms - now_ms) / 86_400_000).max(0))
    }

    fn to_public(&self) -> GmailAccount {
        GmailAccount {
            email: self.email.clone(),
            client: GmailClient {
                id: self.client_id.clone(),
                secret: self.client_secret.clone(),
                project: self.project.clone(),
            },
            refresh_token: self.refresh_token.clone(),
            delete_after_reading: self.delete_after_reading,
            grant_ends_at_ms: self.grant_ends_at_ms,
            ended: self.ended,
        }
    }
}

/// In-memory cached account, lazily loaded from [`crate::SecretBox`].
#[derive(Default)]
pub(crate) struct State {
    loaded: bool,
    account: Option<StoredGmailAccount>,
    #[cfg(test)]
    pub(crate) sign_in_ms_override: Option<u64>,
    #[cfg(test)]
    pub(crate) reachable_window_ms_override: Option<i64>,
}

/// Parses a Desktop-app OAuth client JSON downloaded from Google Cloud.
#[uniffi::export]
pub fn gmail_parse_client(bytes: Vec<u8>) -> Result<GmailClient, GmailError> {
    if bytes.is_empty() || bytes.len() > MAX_CLIENT_BYTES {
        return Err(GmailError::BadFile {
            reason: "This isn't a Google OAuth client file.".into(),
        });
    }
    let json: Value = serde_json::from_slice(&bytes).map_err(|_| GmailError::BadJson {
        reason: "This isn't a valid Google OAuth JSON file.".into(),
    })?;
    let Some(obj) = json.as_object() else {
        return Err(GmailError::BadJson {
            reason: "This isn't a valid Google OAuth JSON file.".into(),
        });
    };
    if obj.contains_key("web") {
        return Err(GmailError::WrongClientType {
            reason: "Create a Desktop app OAuth client, not a Web application client.".into(),
        });
    }
    let Some(installed) = obj.get("installed").and_then(Value::as_object) else {
        return Err(GmailError::WrongClientType {
            reason: "Pick the JSON downloaded for a Desktop app OAuth client.".into(),
        });
    };
    let id = installed
        .get("client_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if !id.ends_with(CLIENT_SUFFIX) || id.len() <= CLIENT_SUFFIX.len() {
        return Err(GmailError::BadClientId {
            reason: "The file has no valid Google OAuth client ID.".into(),
        });
    }
    let secret = installed
        .get("client_secret")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let project = installed
        .get("project_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Google Cloud project")
        .to_owned();
    Ok(GmailClient {
        id: id.to_owned(),
        secret,
        project,
    })
}

#[uniffi::export]
impl Core {
    /// The connected Gmail account, if any; reads the sealed file on first call.
    pub fn gmail_account(&self, flow: String) -> Option<GmailAccount> {
        traced_sync("gmail_account", &flow, || {
            Ok(self.gmail_load().map(|a| a.to_public()))
        })
        .ok()
        .flatten()
    }

    /// Signs in with `client` in the browser (loopback redirect + PKCE S256), verifies the
    /// `@vitapstudent.ac.in` account matches `regno`, checks Gmail API access, seals the account,
    /// and selects Gmail as the OTP source.
    pub async fn gmail_connect(
        &self,
        flow: String,
        client: GmailClient,
        regno: String,
    ) -> Result<GmailAccount, GmailError> {
        let span = crate::span("gmail_connect", &flow);
        let result = crate::transport::with_flow(
            flow.clone(),
            tracing::Instrument::instrument(
                self.gmail_sign_in(&flow, client, &regno),
                span.clone(),
            ),
        )
        .await;
        match &result {
            Ok(_) => {
                span.record("outcome", "ok");
            }
            Err(error) => {
                span.record("outcome", error.id().as_str());
            }
        }
        result
    }

    /// Forgets the saved Gmail account and revokes its refresh token (best effort).
    pub async fn gmail_disconnect(&self, flow: String) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced already logs the call; disconnect is best-effort"
        )]
        let _ = traced("gmail_disconnect", &flow, async {
            let old = {
                let _op = self.shared.gmail_lock.lock().await;
                let existing = self.gmail_load();
                {
                    let mut state = self.shared.gmail();
                    state.loaded = true;
                    state.account = None;
                }
                if let Some(secrets) = &self.shared.secrets {
                    #[expect(
                        clippy::let_underscore_must_use,
                        reason = "clearing the secret box on disconnect is best-effort"
                    )]
                    let _ = secrets.seal(SECRET_NAME.into(), Vec::new());
                }
                #[expect(
                    clippy::let_underscore_must_use,
                    reason = "the sealed file may already be absent on disconnect"
                )]
                let _ = fs::remove_file(self.at(SECRET_FILE));
                existing
            };
            if let Some(account) = old {
                self.gmail_revoke(&account.refresh_token, &flow).await;
            }
            info!("disconnected");
            Ok(())
        })
        .await;
    }

    /// Updates `delete_after_reading` on the connected account and re-seals it.
    pub async fn gmail_set_delete_after_reading(
        &self,
        flow: String,
        on: bool,
    ) -> Option<GmailAccount> {
        traced("gmail_set_delete_after_reading", &flow, async {
            let _op = self.shared.gmail_lock.lock().await;
            let Some(mut latest) = self.gmail_load() else {
                return Ok(None);
            };
            latest.delete_after_reading = on;
            self.gmail_save(&latest);
            Ok(Some(latest.to_public()))
        })
        .await
        .ok()
        .flatten()
    }

    /// Looks in the mailbox for VTOP's OTP sent at or after `since_ms` (unix ms), skipping message
    /// ids in `tried`. Refreshes an expired or rejected access token once and saves it.
    pub async fn gmail_find_otp(
        &self,
        flow: String,
        since_ms: i64,
        tried: Vec<String>,
    ) -> Result<Option<GmailOtp>, GmailError> {
        let span = crate::span("gmail_find_otp", &flow);
        let result = crate::transport::with_flow(
            flow.clone(),
            tracing::Instrument::instrument(
                self.gmail_find_otp_inner(&flow, since_ms, &tried),
                span.clone(),
            ),
        )
        .await;
        match &result {
            Ok(_) => {
                span.record("outcome", "ok");
            }
            Err(error) => {
                span.record("outcome", error.id().as_str());
            }
        }
        result
    }

    /// Trashes the OTP mail, or marks it read when `delete_after_reading` is off; best effort.
    pub async fn gmail_tidy_up(&self, flow: String, otp: GmailOtp) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "tidy-up is best effort and logs its own warning"
        )]
        let _ = traced("gmail_tidy_up", &flow, async {
            let Some(account) = self.gmail_load() else {
                return Ok(());
            };
            let token = match self.gmail_token(&flow, false).await {
                Ok(token) => token,
                Err(error) => {
                    warn!(error = error.id(), "tidy up failed");
                    return Ok(());
                }
            };
            let (path, body) = if account.delete_after_reading {
                (format!("/messages/{}/trash", otp.message_id), "")
            } else {
                (
                    format!("/messages/{}/modify", otp.message_id),
                    r#"{"removeLabelIds":["UNREAD"]}"#,
                )
            };
            if let Err(error) = self
                .gmail_send(
                    &format!("{API}{path}"),
                    &flow,
                    Some(&token),
                    Some(body),
                    None,
                )
                .await
            {
                warn!(error = error.id(), "tidy up failed");
            }
            Ok(())
        })
        .await;
    }
}

impl Shared {
    pub(crate) fn gmail(&self) -> std::sync::MutexGuard<'_, State> {
        self.gmail.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Core {
    fn gmail_load(&self) -> Option<StoredGmailAccount> {
        {
            let state = self.shared.gmail();
            if state.loaded {
                return state.account.clone();
            }
        }
        let loaded = self.shared.secrets.as_ref().and_then(|secrets| {
            match secrets.open(SECRET_NAME.into()) {
                Ok(Some(bytes)) if !bytes.is_empty() => {
                    serde_json::from_slice::<StoredGmailAccount>(&bytes)
                        .ok()
                        .map(StoredGmailAccount::normalized)
                }
                Ok(_) => None,
                Err(error) => {
                    warn!(%error, "gmail secret not opened");
                    None
                }
            }
        });
        let mut state = self.shared.gmail();
        if !state.loaded {
            state.account = loaded;
            state.loaded = true;
        }
        state.account.clone()
    }

    fn gmail_save(&self, account: &StoredGmailAccount) {
        {
            let mut state = self.shared.gmail();
            state.loaded = true;
            state.account = Some(account.clone());
        }
        let Some(secrets) = &self.shared.secrets else {
            return;
        };
        match serde_json::to_vec(account) {
            Ok(bytes) => {
                if let Err(error) = secrets.seal(SECRET_NAME.into(), bytes) {
                    warn!(%error, "gmail secret not saved");
                }
            }
            Err(error) => warn!(%error, "gmail account not serialized"),
        }
    }

    async fn gmail_sign_in(
        &self,
        flow: &str,
        client: GmailClient,
        regno: &str,
    ) -> Result<GmailAccount, GmailError> {
        let verifier = random_b64(32);
        let challenge = pkce_s256(&verifier);
        let oauth_state = random_b64(16);

        let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| GmailError::offline())?;
        let port = listener
            .local_addr()
            .map_err(|_| GmailError::offline())?
            .port();
        let redirect = format!("http://127.0.0.1:{port}");
        let scope_param = format!("openid email {SCOPE}");
        let url = format!(
            "{AUTH}?{}",
            encode_query(&[
                ("client_id", &client.id),
                ("redirect_uri", &redirect),
                ("response_type", "code"),
                ("scope", &scope_param),
                ("code_challenge", &challenge),
                ("code_challenge_method", "S256"),
                ("state", &oauth_state),
                ("access_type", "offline"),
                ("prompt", "consent"),
                ("hd", "vitapstudent.ac.in"),
            ])
        );
        info!(port, "sign-in opened");
        if let Some(urls) = &self.shared.urls {
            urls.open(url);
        }
        let sign_in_ms = {
            #[cfg(test)]
            {
                self.shared
                    .gmail()
                    .sign_in_ms_override
                    .unwrap_or(SIGN_IN_MS)
            }
            #[cfg(not(test))]
            {
                SIGN_IN_MS
            }
        };
        let code = await_redirect(listener, port, oauth_state, sign_in_ms, &self.shared).await?;

        let token_fields = [
            ("code", Some(code.as_str())),
            ("client_id", Some(client.id.as_str())),
            ("client_secret", client.secret.as_deref()),
            ("redirect_uri", Some(redirect.as_str())),
            ("grant_type", Some("authorization_code")),
            ("code_verifier", Some(verifier.as_str())),
        ];
        let tokens = self
            .gmail_until_reachable(flow, || self.gmail_post(TOKEN, flow, &token_fields))
            .await?;

        let access = tokens
            .get("access_token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let refresh = tokens
            .get("refresh_token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let revoke_token = if refresh.is_empty() {
            access.clone()
        } else {
            refresh.clone()
        };

        let finish: Result<GmailAccount, GmailError> = async {
            if access.is_empty() || refresh.is_empty() {
                return Err(GmailError::NoTokens {
                    reason: "Google didn't send the tokens. Try again.".into(),
                });
            }
            let scope = tokens
                .get("scope")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if !scope.split(' ').any(|s| s == SCOPE) {
                return Err(GmailError::NoScope {
                    reason: "Tick the Gmail permission on Google's screen, then try again.".into(),
                });
            }
            let id_token = tokens
                .get("id_token")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let email = email_from_id_token(id_token).ok_or_else(|| GmailError::NoEmail {
                reason: "Couldn't read your Google account's email.".into(),
            })?;
            if let Some(problem) = college_account_problem(&email, regno) {
                return Err(GmailError::WrongAccount { reason: problem });
            }
            self.gmail_verify(&access, flow).await?;
            let now_ms = self.shared.runtime.clock().now_ms();
            let ends = tokens
                .get("refresh_token_expires_in")
                .and_then(Value::as_i64)
                .filter(|&secs| secs > 0)
                .map_or(0, |secs| now_ms + secs * 1000);
            let expires_at_ms = token_expiry(&tokens, now_ms);
            let saved = {
                let _op = self.shared.gmail_lock.lock().await;
                let delete_after = self
                    .gmail_load()
                    .is_none_or(|prev| prev.delete_after_reading);
                let stored = StoredGmailAccount {
                    email,
                    client_id: client.id,
                    client_secret: client.secret,
                    project: client.project,
                    refresh_token: refresh,
                    access_token: access,
                    expires_at_ms,
                    delete_after_reading: delete_after,
                    grant_ends_at_ms: ends,
                    ended: false,
                };
                self.gmail_save(&stored);
                stored
            };
            self.settings.set(SettingChange::OtpSource {
                value: Some(OtpSource::Gmail),
            });
            let grant_days = saved
                .days_left(now_ms)
                .map_or_else(|| "-".into(), |d| d.to_string());
            info!(grant_days, "connected");
            Ok(saved.to_public())
        }
        .await;

        if finish.is_err() {
            self.gmail_revoke(&revoke_token, flow).await;
        }
        finish
    }

    async fn gmail_find_otp_inner(
        &self,
        flow: &str,
        since_ms: i64,
        tried: &[String],
    ) -> Result<Option<GmailOtp>, GmailError> {
        let Some(account) = self.gmail_load() else {
            return Ok(None);
        };
        let now_ms = self.shared.runtime.clock().now_ms();
        if account.needs_reconnect(now_ms) {
            return Err(GmailError::ended());
        }
        let since_s = (since_ms / 1000) - CLOCK_SLACK_S;
        let tried_set: HashSet<&str> = tried.iter().map(String::as_str).collect();
        let token = self.gmail_token(flow, false).await?;
        match self.gmail_newest(&token, since_s, &tried_set, flow).await {
            Err(GmailError::Unauthorized { .. }) => {
                let refreshed = self.gmail_token(flow, true).await?;
                self.gmail_newest(&refreshed, since_s, &tried_set, flow)
                    .await
            }
            other => other,
        }
    }

    async fn gmail_newest(
        &self,
        token: &str,
        since_s: i64,
        tried: &HashSet<&str>,
        flow: &str,
    ) -> Result<Option<GmailOtp>, GmailError> {
        let q = format!("from:{SENDER} after:{since_s}");
        let url = format!(
            "{API}/messages?{}",
            encode_query(&[("maxResults", "3"), ("q", &q)])
        );
        let list = self.gmail_send(&url, flow, Some(token), None, None).await?;
        let ids: Vec<String> = list
            .get("messages")
            .and_then(Value::as_array)
            .map_or_default(|arr| {
                arr.iter()
                    .filter_map(|item| item.get("id").and_then(Value::as_str))
                    .filter(|id| !id.is_empty() && !tried.contains(*id))
                    .map(str::to_owned)
                    .collect()
            });
        let since_u64 = u64::try_from(since_s).unwrap_or(0);
        for id in ids {
            let msg_url = format!("{API}/messages/{id}?format=full");
            let raw = self
                .gmail_send(&msg_url, flow, Some(token), None, None)
                .await?;
            if let Ok(message) = serde_json::from_value::<proto_gmail::Message>(raw)
                && let Some(code) = proto_gmail::otp(&message, since_u64)
            {
                return Ok(Some(GmailOtp {
                    code,
                    message_id: id,
                }));
            }
        }
        Ok(None)
    }

    async fn gmail_verify(&self, token: &str, flow: &str) -> Result<(), GmailError> {
        match self
            .gmail_send(&format!("{API}/profile"), flow, Some(token), None, None)
            .await
        {
            Ok(_) => Ok(()),
            Err(GmailError::Forbidden { .. }) => Err(GmailError::GmailOff {
                reason: "Gmail didn't let the app in. Turn on the Gmail API in your project (step 2), wait a minute and connect again.".into(),
            }),
            Err(error) => Err(error),
        }
    }

    async fn gmail_token(&self, flow: &str, force: bool) -> Result<String, GmailError> {
        let _op = self.shared.gmail_lock.lock().await;
        let account = self.gmail_load().ok_or_else(|| GmailError::NotConnected {
            reason: "Gmail isn't connected.".into(),
        })?;
        let now_ms = self.shared.runtime.clock().now_ms();
        if !force && account.expires_at_ms > now_ms + 60_000 {
            return Ok(account.access_token);
        }
        let tokens = match self
            .gmail_post(
                TOKEN,
                flow,
                &[
                    ("refresh_token", Some(account.refresh_token.as_str())),
                    ("client_id", Some(account.client_id.as_str())),
                    ("client_secret", account.client_secret.as_deref()),
                    ("grant_type", Some("refresh_token")),
                ],
            )
            .await
        {
            Ok(tokens) => tokens,
            Err(GmailError::InvalidGrant { .. }) => {
                let mut ended = account;
                ended.ended = true;
                self.gmail_save(&ended);
                info!("grant ended");
                return Err(GmailError::ended());
            }
            Err(error) => return Err(error),
        };
        let mut fresh = account;
        fresh.access_token = tokens
            .get("access_token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        fresh.expires_at_ms = token_expiry(&tokens, self.shared.runtime.clock().now_ms());
        self.gmail_save(&fresh);
        Ok(fresh.access_token)
    }

    async fn gmail_until_reachable<F, Fut>(
        &self,
        _flow: &str,
        mut call: F,
    ) -> Result<Value, GmailError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<Value, GmailError>>,
    {
        let window_ms = {
            #[cfg(test)]
            {
                self.shared
                    .gmail()
                    .reachable_window_ms_override
                    .unwrap_or(REACHABLE_WINDOW_MS)
            }
            #[cfg(not(test))]
            {
                REACHABLE_WINDOW_MS
            }
        };
        let clock = self.shared.runtime.clock();
        let deadline = clock.now_ms() + window_ms;
        let mut tries: u32 = 1;
        let mut backoff_ms = RETRY_BASE_MS;
        loop {
            match call().await {
                Ok(value) => {
                    if tries > 1 {
                        info!(tries, "reached Google");
                    }
                    return Ok(value);
                }
                Err(error @ GmailError::Unreachable { .. }) => {
                    let remaining = deadline - clock.now_ms();
                    if remaining <= 0 {
                        return Err(error);
                    }
                    let wait = backoff_ms.min(u64::try_from(remaining).unwrap_or(0));
                    clock.sleep(wait).await;
                    backoff_ms = (backoff_ms * 2).min(RETRY_MAX_MS);
                    tries = tries.saturating_add(1);
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn gmail_post(
        &self,
        url: &str,
        flow: &str,
        fields: &[(&str, Option<&str>)],
    ) -> Result<Value, GmailError> {
        let mut form = String::new();
        for (k, v) in fields {
            let Some(value) = v else { continue };
            if !form.is_empty() {
                form.push('&');
            }
            form.push_str(&form_encode(k));
            form.push('=');
            form.push_str(&form_encode(value));
        }
        self.gmail_send(url, flow, None, None, Some(&form)).await
    }

    async fn gmail_revoke(&self, token: &str, flow: &str) {
        if token.is_empty() {
            return;
        }
        #[expect(
            clippy::let_underscore_must_use,
            reason = "revoking a Gmail token on error or disconnect is best effort"
        )]
        let _ = self
            .gmail_post(REVOKE, flow, &[("token", Some(token))])
            .await;
    }

    async fn gmail_send(
        &self,
        url: &str,
        flow: &str,
        token: Option<&str>,
        json_body: Option<&str>,
        form_body: Option<&str>,
    ) -> Result<Value, GmailError> {
        let transport: Arc<dyn Transport> =
            self.shared.http.clone().ok_or_else(GmailError::offline)?;
        let body_str = json_body.or(form_body);
        let method = if body_str.is_some() { "POST" } else { "GET" };
        let path = url_path(url);
        let started = self.shared.runtime.clock().now_ms();

        let mut headers = vec![("x-request-id".to_owned(), flow.to_owned())];
        if let Some(bearer) = token {
            headers.push(("Authorization".to_owned(), format!("Bearer {bearer}")));
        }
        let body = if let Some(b) = body_str {
            let content_type = if json_body.is_some() {
                "application/json"
            } else {
                "application/x-www-form-urlencoded"
            };
            headers.push(("Content-Type".to_owned(), content_type.to_owned()));
            b.as_bytes().to_vec()
        } else {
            Vec::new()
        };
        let hop = Hop {
            method: method.to_owned(),
            url: url.to_owned(),
            headers,
            body,
            timeout: HTTP_TIMEOUT,
            connect_timeout: HTTP_TIMEOUT,
            download_to: None,
        };
        let response = match transport.send(hop).await {
            Ok(response) => response,
            Err(TransportError::Connect(_)) => {
                warn!(method, path, "request failed: connect");
                return Err(GmailError::unreachable());
            }
            Err(_) => {
                warn!(method, path, "request failed: offline");
                return Err(GmailError::offline());
            }
        };
        let status = response.status;
        let bytes = response.bytes().await.map_err(|_| GmailError::offline())?;
        let ms = (self.shared.runtime.clock().now_ms() - started).max(0);
        info!(method, path, status, ms, "gmail http");

        let parsed =
            serde_json::from_slice::<Value>(&bytes).unwrap_or_else(|_| serde_json::json!({}));
        if (200..300).contains(&status) {
            return Ok(parsed);
        }
        if status == 401 {
            return Err(GmailError::Unauthorized {
                reason: "Gmail turned down the sign-in. Connect Gmail again.".into(),
            });
        }
        if status == 403 {
            let err_status = parsed
                .get("error")
                .and_then(|e| e.get("status"))
                .and_then(Value::as_str)
                .unwrap_or("403")
                .to_owned();
            return Err(GmailError::Forbidden {
                reason: format!("Gmail refused access ({err_status})."),
                status: err_status,
            });
        }
        match parsed.get("error").and_then(Value::as_str) {
            Some("invalid_grant") => Err(GmailError::InvalidGrant {
                reason: "Google ended the connection.".into(),
            }),
            Some("invalid_client") => Err(GmailError::InvalidClient {
                reason: "Google doesn't know this client. Import a new client JSON.".into(),
            }),
            _ => Err(GmailError::Http {
                status,
                reason: format!("Google answered HTTP {status}."),
            }),
        }
    }
}

/// Wakes a blocking loopback `TcpListener::accept` if the sign-in future is cancelled or times out.
struct WakeListener {
    port: u16,
    cancelled: Arc<AtomicBool>,
    armed: bool,
}

impl Drop for WakeListener {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        self.cancelled.store(true, Ordering::Release);
        #[expect(
            clippy::let_underscore_must_use,
            reason = "best-effort self-connect to unblock TcpListener::accept"
        )]
        let _ = TcpStream::connect(("127.0.0.1", self.port));
    }
}

async fn await_redirect(
    listener: TcpListener,
    port: u16,
    expected_state: String,
    sign_in_ms: u64,
    shared: &Shared,
) -> Result<String, GmailError> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut guard = WakeListener {
        port,
        cancelled: cancelled.clone(),
        armed: true,
    };
    let wait = smol::unblock(move || accept_redirect(listener, &expected_state, &cancelled));
    let result = shared.runtime.timeout(sign_in_ms, wait).await;
    match result {
        Ok(res) => {
            guard.armed = false;
            res
        }
        Err(_) => Err(GmailError::timeout()),
    }
}

fn accept_redirect(
    listener: TcpListener,
    expected_state: &str,
    cancelled: &AtomicBool,
) -> Result<String, GmailError> {
    loop {
        let Ok((mut stream, _)) = listener.accept() else {
            return Err(GmailError::timeout());
        };
        if cancelled.load(Ordering::Acquire) {
            return Err(GmailError::timeout());
        }
        #[expect(
            clippy::let_underscore_must_use,
            reason = "read timeout on loopback stream is best effort"
        )]
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut line = String::new();
        {
            let mut reader = BufReader::new(&stream);
            #[expect(
                clippy::let_underscore_must_use,
                reason = "empty or closed request line is handled as missing params"
            )]
            let _ = reader.read_line(&mut line);
        }
        let target = line.split(' ').nth(1).unwrap_or_default();
        let query_str = target.split_once('?').map_or("", |(_, q)| q);
        let params: Vec<(String, String)> = query_str
            .split('&')
            .filter_map(|part| {
                let (k, v) = part.split_once('=')?;
                Some((percent_decode(k), percent_decode(v)))
            })
            .collect();
        if params.is_empty() {
            reply_http(&mut stream, 404, "Not found");
            continue;
        }
        let find = |key: &str| {
            params
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };
        let error = find("error");
        let code = find("code");
        let done = code.is_some() && find("state") == Some(expected_state);
        reply_http(
            &mut stream,
            200,
            if done {
                "Almost done: go back to Vitap Mate to finish connecting Gmail."
            } else {
                "Google sign-in didn't finish. Go back to Vitap Mate and try again."
            },
        );
        if let Some(err) = error {
            info!(error = err, "sign-in refused");
            return Err(GmailError::sign_in_refused(err));
        }
        if !done {
            return Err(GmailError::BadState {
                reason: "Google's answer didn't match this sign-in. Try again.".into(),
            });
        }
        if let Some(code) = code {
            return Ok(code.to_owned());
        }
    }
}

fn reply_http(stream: &mut TcpStream, status: u16, text: &str) {
    let status_text = if status == 200 { "OK" } else { "Not Found" };
    let html = format!(
        "<!doctype html><meta name=viewport content=\"width=device-width\"><body style=\"font:18px sans-serif;padding:24px\">{text}</body>"
    );
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
        html.len()
    );
    #[expect(
        clippy::let_underscore_must_use,
        reason = "best-effort HTTP reply to the browser redirect tab"
    )]
    let _ = stream.write_all(response.as_bytes());
    #[expect(
        clippy::let_underscore_must_use,
        reason = "best-effort flush of HTTP reply"
    )]
    let _ = stream.flush();
}

fn token_expiry(tokens: &Value, now_ms: i64) -> i64 {
    let expires_in = tokens
        .get("expires_in")
        .and_then(Value::as_i64)
        .unwrap_or(3600);
    now_ms + expires_in * 1000
}

/// Validates a VIT-AP college Gmail address holding `regno`.
fn college_account_problem(email: &str, regno: &str) -> Option<String> {
    if !email.to_ascii_lowercase().ends_with("@vitapstudent.ac.in") {
        return Some("Use your @vitapstudent.ac.in Google account.".into());
    }
    let local = email.split('@').next().unwrap_or_default();
    let from_email: String = local
        .split('.')
        .rev()
        .find(|part| !part.is_empty())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    let expected: String = regno
        .to_ascii_lowercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    if expected.is_empty() || !from_email.ends_with(&expected) {
        return Some(format!(
            "Use the college Google account with your registration number ({regno})."
        ));
    }
    None
}

fn email_from_id_token(id_token: &str) -> Option<String> {
    let payload_b64 = id_token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload_b64.trim_end_matches('='))
        .ok()?;
    let json: Value = serde_json::from_slice(&bytes).ok()?;
    let email = json.get("email")?.as_str()?.trim();
    (!email.is_empty()).then(|| email.to_owned())
}

fn random_b64(bytes: usize) -> String {
    let seed = vtop_shared::device_sig::generate_seed();
    if bytes == 32 {
        return seed;
    }
    let decoded = URL_SAFE_NO_PAD.decode(&seed).unwrap_or_default();
    let slice = decoded.get(..bytes).unwrap_or(&decoded);
    URL_SAFE_NO_PAD.encode(slice)
}

fn pkce_s256(verifier: &str) -> String {
    let hex_hash = vtop_shared::tokens::hash(verifier);
    let bytes = hex_decode(&hex_hash);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn hex_decode(hex: &str) -> Vec<u8> {
    let (pairs, _) = hex.as_bytes().as_chunks::<2>();
    let mut out = Vec::with_capacity(pairs.len());
    for &[h, l] in pairs {
        if let (Some(hi), Some(lo)) = (hex_nibble(h), hex_nibble(l)) {
            out.push((hi << 4) | lo);
        }
    }
    out
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn url_path(url: &str) -> &str {
    let without_query = url.split('?').next().unwrap_or(url);
    let after_scheme = without_query
        .split_once("://")
        .map_or(without_query, |(_, rest)| rest);
    after_scheme
        .find('/')
        .and_then(|idx| after_scheme.get(idx..))
        .unwrap_or("/")
}

fn encode_query(pairs: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (k, v) in pairs {
        if !out.is_empty() {
            out.push('&');
        }
        out.push_str(k);
        out.push('=');
        out.push_str(&percent_encode(v, false));
    }
    out
}

fn form_encode(value: &str) -> String {
    percent_encode(value, true)
}

fn percent_encode(input: &str, space_as_plus: bool) -> String {
    let mut out = String::with_capacity(input.len());
    for &b in input.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' => {
                out.push(b as char);
            }
            b' ' if space_as_plus => out.push('+'),
            _ => {
                use std::fmt::Write as _;
                #[expect(
                    clippy::let_underscore_must_use,
                    reason = "writing to a String never fails"
                )]
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let Some(&b) = bytes.get(i) else { break };
        if b == b'+' {
            out.push(b' ');
            i += 1;
        } else if b == b'%'
            && let (Some(&h), Some(&l)) = (bytes.get(i + 1), bytes.get(i + 2))
            && let (Some(hi), Some(lo)) = (hex_nibble(h), hex_nibble(l))
        {
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests;

//! The engine's side of the edge: VTOP work and nothing else kept. The
//! edge holds accounts, keys and requests; it asks here whose a session is,
//! to seal saved credentials (only the engine has `VAULT_KEY`) and to sign
//! in with them. A sign-in answers at once and reports to the edge when
//! done; the OTP comes from the student's forwarded mail, sealed by the
//! edge to a key made for that one sign-in, or, for students without a
//! forwarding address, from their Gmail with the access sealed in their
//! credentials.
//!
//! Routes (`Authorization: Bearer $INTERNAL_TOKEN`):
//! `POST /engine/session`, `/engine/seal`, `/engine/login`, `/engine/fetch`
//! and `/engine/fetch-batch` (in `fetch`), and
//! [`MAIL_OWNER`](vtop_shared::engine::MAIL_OWNER) (whose a session is, as
//! an owner hash).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use vtop_core::SessionState;
use vtop_shared::account::account_id;
use vtop_shared::engine::{
    GmailGrant, LOGIN_DONE, LoginDone, LoginOutcome, LoginStart, OTP_AWAIT, OtpAwait, OtpMethod,
    SealVault, SealedVault, SessionCheck, SessionChecked, SessionStatus,
};
use vtop_shared::mail_seal;
use vtop_shared::mailbox::owner_hash;
use vtop_shared::tokens;

use crate::AppState;
use crate::error::ApiError;
use crate::gmail::{GmailCodes, RefreshError};
use crate::login::{LoginError, OtpCodes};
use crate::verify::VerifyError;

/// What the engine asks of the edge.
#[async_trait]
pub trait EdgeLink: Send + Sync {
    /// The next forwarded mail of `request.owner_hash`, sealed to
    /// `request.public_key`; `None` when nothing came in time.
    async fn await_otp(&self, request: &OtpAwait) -> Result<Option<String>, EdgeError>;
    async fn login_done(&self, done: &LoginDone) -> Result<(), EdgeError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeError {
    /// The student has no forwarding address.
    NoMailbox,
    Unavailable(String),
}

/// The edge over HTTPS, with `Authorization: Bearer $INTERNAL_TOKEN`.
pub struct HttpEdge {
    pub http: reqwest::Client,
    pub base_url: String,
    pub token: String,
}

impl HttpEdge {
    async fn post(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<reqwest::Response, EdgeError> {
        self.http
            .post(format!("{}{path}", self.base_url))
            .bearer_auth(&self.token)
            .timeout(Duration::from_secs(40))
            .json(body)
            .send()
            .await
            .map_err(|error| EdgeError::Unavailable(error.without_url().to_string()))
    }
}

#[async_trait]
impl EdgeLink for HttpEdge {
    async fn await_otp(&self, request: &OtpAwait) -> Result<Option<String>, EdgeError> {
        let response = self.post(OTP_AWAIT, request).await?;
        match response.status().as_u16() {
            200 => {
                let body: vtop_shared::engine::OtpAwaited = response
                    .json()
                    .await
                    .map_err(|error| EdgeError::Unavailable(error.to_string()))?;
                Ok(body.sealed)
            }
            404 => Err(EdgeError::NoMailbox),
            status => Err(EdgeError::Unavailable(format!("edge answered {status}"))),
        }
    }

    async fn login_done(&self, done: &LoginDone) -> Result<(), EdgeError> {
        let response = self.post(LOGIN_DONE, done).await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(EdgeError::Unavailable(format!(
                "edge answered {}",
                response.status()
            )))
        }
    }
}

/// OTP codes from forwarded mail, opened with a key made for this sign-in.
pub struct ForwardedCodes {
    edge: Arc<dyn EdgeLink>,
    owner_hash: String,
    secret: String,
    public_key: String,
    /// Set when the edge says the student has no forwarding address.
    pub no_mailbox: AtomicBool,
}

/// Mail dates come from VTOP's clock; allow it to run a little behind.
const CLOCK_SLACK_SECS: u64 = 120;
/// How long one wait at the edge lasts; the edge's own limit is shorter
/// than a request may take.
const AWAIT_STEP_SECS: u64 = 25;

impl ForwardedCodes {
    pub fn new(edge: Arc<dyn EdgeLink>, owner_hash: String) -> Self {
        let (secret, public_key) = mail_seal::generate_keypair();
        Self {
            edge,
            owner_hash,
            secret,
            public_key,
            no_mailbox: AtomicBool::new(false),
        }
    }

    /// Tells the edge to keep this sign-in's codes from now on, before VTOP
    /// is asked for one, so a mail that comes quickly is not missed.
    pub async fn register(&self) -> Result<(), EdgeError> {
        let request = OtpAwait {
            owner_hash: self.owner_hash.clone(),
            public_key: self.public_key.clone(),
            since: vtop_core::now_unix().saturating_sub(CLOCK_SLACK_SECS),
            wait_secs: 0,
        };
        self.edge.await_otp(&request).await.map(|_| ())
    }

    /// The code in a sealed mail payload, if it is a VTOP OTP.
    fn open(&self, sealed: &str) -> Option<String> {
        #[derive(Deserialize)]
        struct Payload {
            kind: String,
            #[serde(default)]
            code: Option<String>,
        }
        let plaintext = mail_seal::open(&self.secret, &self.owner_hash, sealed).ok()?;
        let payload: Payload = serde_json::from_slice(&plaintext).ok()?;
        (payload.kind == "otp").then_some(payload.code?)
    }
}

#[async_trait]
impl OtpCodes for ForwardedCodes {
    async fn next(
        &self,
        issued_at: u64,
        remaining: Duration,
        tried: Vec<String>,
    ) -> Option<String> {
        let deadline = tokio::time::Instant::now() + remaining;
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left < Duration::from_secs(1) {
                return None;
            }
            let request = OtpAwait {
                owner_hash: self.owner_hash.clone(),
                public_key: self.public_key.clone(),
                since: issued_at.saturating_sub(CLOCK_SLACK_SECS),
                wait_secs: left.as_secs().min(AWAIT_STEP_SECS),
            };
            match self.edge.await_otp(&request).await {
                Ok(Some(sealed)) => match self.open(&sealed) {
                    Some(code) if !tried.contains(&code) => return Some(code),
                    _ => {}
                },
                Ok(None) => {}
                Err(EdgeError::NoMailbox) => {
                    self.no_mailbox.store(true, Ordering::SeqCst);
                    return None;
                }
                Err(EdgeError::Unavailable(reason)) => {
                    tracing::info!("engine: waiting for a forwarded OTP: {reason}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}

/// Saved credentials, sealed to `accounts/<id>`. No `Debug`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vault {
    /// The VTOP login name; it can change, the registration number cannot.
    pub username: String,
    pub password: String,
    /// Gmail access, used when the student has no forwarding address.
    /// Vaults from the old bridge carry one too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gmail: Option<GmailGrant>,
    /// The OTP auto-fill picked on the phone: Gmail goes straight to Gmail, even with a
    /// forwarding address the student never set up in their mail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub otp: Option<OtpMethod>,
}

fn bad_request(message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "bad_request", message)
}

/// What sealed account fields are bound to, as the bridge always did.
fn accounts_aad(account: &str) -> String {
    format!("accounts/{account}")
}

/// Every route needs `INTERNAL_TOKEN`, shared with the edge and the gateway.
pub(crate) fn require_secret(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let presented = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default()
        .trim();
    if !tokens::hashes_equal(
        &tokens::hash(&state.config.internal_token),
        &tokens::hash(presented),
    ) {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "The internal token is required.",
        ));
    }
    Ok(())
}

fn cookies_only(cookies: &str) -> SessionState {
    SessionState {
        cookies: cookies.trim().to_string(),
        csrf_token: None,
        registration_number: None,
        otp_issued_at: None,
        logged_in_at: None,
    }
}

fn checked(status: SessionStatus) -> SessionChecked {
    SessionChecked {
        status,
        registration_number: None,
        cookies: None,
        csrf_token: None,
        sealed_session: None,
        owner_hash: None,
    }
}

/// Whose session fresh cookies are, or whether an account's cached one
/// still works. A cached session VTOP cannot be asked about is handed back
/// as it is, as the bridge always did.
pub async fn session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<SessionCheck>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(check) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let (session, expected) = match (&check.cookies, &check.sealed_session, &check.account) {
        (Some(cookies), _, _) if !cookies.trim().is_empty() => (cookies_only(cookies), None),
        (_, Some(sealed), Some(account)) => {
            let account = account_id(account);
            match state
                .accounts_sealer()
                .open_json::<SessionState>(&accounts_aad(&account), sealed)
            {
                Ok(session) => (cookies_only(&session.cookies), Some(account)),
                Err(_) => return Ok(Json(checked(SessionStatus::Expired)).into_response()),
            }
        }
        _ => {
            return Err(bad_request(
                "cookies, or sealedSession and account, are required.",
            ));
        }
    };
    let outcome = match state.verifier.registration_number(&session).await {
        Ok(owner) if expected.as_ref().is_none_or(|expected| *expected == owner) => {
            let owner = account_id(&owner);
            SessionChecked {
                status: SessionStatus::Valid,
                sealed_session: Some(
                    state
                        .accounts_sealer()
                        .seal_json(&accounts_aad(&owner), &session),
                ),
                owner_hash: Some(owner_hash(&state.config.vault_key, &owner)),
                csrf_token: session.csrf_token.clone(),
                cookies: Some(session.cookies),
                registration_number: Some(owner),
            }
        }
        Ok(_) | Err(VerifyError::Expired) => checked(SessionStatus::Expired),
        Err(VerifyError::Unavailable(reason)) => {
            tracing::info!("engine: cannot check a session: {reason}");
            SessionChecked {
                // Only a cached session is handed out unchecked.
                csrf_token: expected.as_ref().and(session.csrf_token.clone()),
                cookies: expected.map(|_| session.cookies),
                ..checked(SessionStatus::Unknown)
            }
        }
    };
    Ok(Json(outcome).into_response())
}

/// Seals saved credentials for an account; the edge stores the result.
pub async fn seal(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<SealVault>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let account = account_id(&body.account);
    let username = body.username.trim().to_string();
    if account.is_empty() || username.is_empty() || body.password.is_empty() {
        return Err(bad_request("account, username and password are required."));
    }
    let vault = Vault {
        username,
        password: body.password,
        gmail: body.gmail.and_then(GmailGrant::usable),
        otp: body.otp,
    };
    let sealed_vault = state
        .accounts_sealer()
        .seal_json(&accounts_aad(&account), &vault);
    Ok(Json(SealedVault { sealed_vault }).into_response())
}

/// Starts a sign-in with saved credentials; the outcome goes to the edge.
pub async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<LoginStart>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(start) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let edge = state.edge.clone();
    let account = account_id(&start.account);
    tokio::spawn(async move {
        let outcome = sign_in(&state, edge.clone(), &account, &start.sealed_vault).await;
        let done = LoginDone {
            account: account.clone(),
            request_id: start.request_id,
            outcome,
        };
        // The edge's request times out on its own if this never lands.
        for attempt in 0..3 {
            match edge.login_done(&done).await {
                Ok(()) => return,
                Err(error) => {
                    tracing::warn!(
                        "engine: report sign-in of {account} (try {attempt}): {error:?}"
                    );
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    });
    Ok(StatusCode::ACCEPTED.into_response())
}

async fn sign_in(
    state: &AppState,
    edge: Arc<dyn EdgeLink>,
    account: &str,
    sealed_vault: &str,
) -> LoginOutcome {
    let Ok(vault) = state
        .accounts_sealer()
        .open_json::<Vault>(&accounts_aad(account), sealed_vault)
    else {
        return LoginOutcome::VaultUnusable;
    };
    // Gmail when the phone picked it; else forwarding first, Gmail only
    // for students without it.
    if vault.otp == Some(OtpMethod::Gmail)
        && let Some(grant) = vault.gmail.clone().and_then(GmailGrant::usable)
    {
        return sign_in_with_gmail(state, account, vault, grant).await;
    }
    let codes = ForwardedCodes::new(edge, owner_hash(&state.config.vault_key, account));
    match codes.register().await {
        Ok(()) => {}
        // No forwarding address: VTOP is not asked unless Gmail can read
        // the OTP (it would mail one nobody can read here).
        Err(EdgeError::NoMailbox) => {
            return match vault.gmail.clone().and_then(GmailGrant::usable) {
                Some(grant) => sign_in_with_gmail(state, account, vault, grant).await,
                None => LoginOutcome::ForwardingRequired,
            };
        }
        Err(EdgeError::Unavailable(reason)) => {
            tracing::info!("engine: could not register for {account}'s OTP: {reason}");
        }
    }
    let result = state
        .forwarded_login
        .login(
            &vault.username,
            &vault.password,
            &codes,
            state.config.otp_wait,
        )
        .await;
    if matches!(result, Err(LoginError::Failed(_))) && codes.no_mailbox.load(Ordering::SeqCst) {
        return LoginOutcome::ForwardingRequired;
    }
    signed_in(state, account, result)
}

/// Signs in with the OTP read from the student's Gmail. Access Google
/// refuses is dropped from the credentials; the rest are kept.
async fn sign_in_with_gmail(
    state: &AppState,
    account: &str,
    vault: Vault,
    grant: GmailGrant,
) -> LoginOutcome {
    let token = match state.gmail.refresh(&grant).await {
        Ok(token) => token,
        Err(RefreshError::InvalidGrant) => {
            tracing::info!("engine: Gmail access of {account} was revoked; dropping it");
            let vault = Vault {
                gmail: None,
                ..vault
            };
            return LoginOutcome::GmailRevoked {
                sealed_vault: state
                    .accounts_sealer()
                    .seal_json(&accounts_aad(account), &vault),
            };
        }
        Err(RefreshError::Unavailable(reason)) => {
            tracing::info!("engine: no Gmail access token for {account} for now: {reason}");
            return LoginOutcome::Failed {
                reason: "gmail_unavailable: Gmail could not be reached".into(),
            };
        }
    };
    let codes = GmailCodes::new(token, grant.delete_after_reading);
    let result = state
        .forwarded_login
        .login(
            &vault.username,
            &vault.password,
            &codes,
            state.config.otp_wait,
        )
        .await;
    if result.is_ok() {
        codes.tidy_up().await;
    }
    signed_in(state, account, result)
}

/// What a sign-in with saved credentials ended in.
fn signed_in(
    state: &AppState,
    account: &str,
    result: Result<SessionState, LoginError>,
) -> LoginOutcome {
    match result {
        Ok(session) => {
            let owner = session.registration_number.as_deref().map(account_id);
            if owner.as_deref() != Some(account) {
                tracing::warn!("engine: saved credentials of {account} sign in someone else");
                return LoginOutcome::VaultUnusable;
            }
            tracing::info!("engine: signed {account} in with saved credentials");
            LoginOutcome::Ok {
                sealed_session: state
                    .accounts_sealer()
                    .seal_json(&accounts_aad(account), &session),
                csrf_token: session.csrf_token.clone(),
                cookies: session.cookies,
            }
        }
        Err(LoginError::InvalidCredentials) => LoginOutcome::InvalidCredentials,
        Err(LoginError::Failed(reason)) => {
            tracing::info!("engine: sign-in of {account} failed for now: {reason}");
            LoginOutcome::Failed { reason }
        }
    }
}

#[derive(Deserialize)]
pub struct SessionBody {
    #[serde(default)]
    cookies: String,
}

/// Whose a live session is, as the owner hash the edge files forwarding
/// addresses under, and the registration number (the edge's account tokens
/// name it). The session is used for this check only.
pub async fn owner_check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<SessionBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    if body.cookies.trim().is_empty() {
        return Err(bad_request("session.cookies is required."));
    }
    match state
        .verifier
        .registration_number(&cookies_only(&body.cookies))
        .await
    {
        Ok(account) => {
            let account = account_id(&account);
            Ok(Json(json!({
                "ownerHash": owner_hash(&state.config.vault_key, &account),
                "registrationNumber": account,
            }))
            .into_response())
        }
        Err(VerifyError::Expired) => Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "session_invalid",
            "VTOP does not accept this session; sign in again.",
        )),
        Err(VerifyError::Unavailable(reason)) => {
            tracing::info!("engine: owner check: {reason}");
            Err(ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "vtop_unreachable",
                "VTOP is unreachable; try again.",
            ))
        }
    }
}

#[cfg(test)]
mod tests;

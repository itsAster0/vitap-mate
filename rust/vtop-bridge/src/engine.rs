//! The engine's side of the edge: VTOP work and nothing else kept. The
//! edge holds accounts, keys and requests; it asks here whose a session is,
//! to seal saved credentials (only the engine has `VAULT_KEY`) and to sign
//! in with them. A sign-in runs one step per call and answers when the step
//! is done: with the OTP read from the student's Gmail (with the access
//! sealed in their credentials), the step signs in whole; with forwarded
//! mail, it stops at VTOP's OTP and hands back the half-done sign-in sealed.
//! Convex sends it back with the code when the mail comes.
//!
//! Routes (`Authorization: Bearer $INTERNAL_TOKEN`):
//! `POST /engine/session`, `/engine/seal`, `/engine/login`, `/engine/fetch`
//! and `/engine/fetch-batch` (in `fetch`),
//! [`MAIL_OWNER`](vtop_shared::engine::MAIL_OWNER) (whose a session is, as
//! an owner hash) and [`OWNER_HASH`] (a proved regno's owner hash).

use std::sync::Arc;

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
    GmailGrant, LoginOutcome, OtpMethod, SealVault, SealedVault, SessionCheck, SessionChecked,
    SessionStatus,
};
use vtop_shared::mailbox::owner_hash;
use vtop_shared::tokens;

use crate::AppState;
use crate::error::ApiError;
use crate::gmail::{GmailCodes, RefreshError};
use crate::login::{LoginError, Started};
use crate::verify::VerifyError;

/// Mail dates come from VTOP's clock; allow it to run a little behind.
const CLOCK_SLACK_SECS: u64 = 120;

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

/// Every route needs `INTERNAL_TOKEN`, shared with the edge.
pub(crate) fn require_secret(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    check_token(headers, &[Some(&state.config.internal_token)])
}

/// The routes the UDP gateway calls (page fetches and the phone's sign-in)
/// take its own `GATEWAY_TOKEN` too, or Convex's `INTERNAL_TOKEN`.
pub(crate) fn require_gateway(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    check_token(
        headers,
        &[
            Some(&state.config.internal_token),
            Some(&state.config.gateway_token),
        ],
    )
}

fn check_token(headers: &HeaderMap, accepted: &[Option<&String>]) -> Result<(), ApiError> {
    let presented = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default()
        .trim();
    let presented = tokens::hash(presented);
    if !accepted
        .iter()
        .flatten()
        .any(|token| tokens::hashes_equal(&tokens::hash(token), &presented))
    {
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
                // Its CSRF token too: handed back with the cached session, it saves the
                // caller a trip to VTOP for one.
                Ok(session) => (
                    SessionState {
                        csrf_token: session.csrf_token,
                        ..cookies_only(&session.cookies)
                    },
                    Some(account),
                ),
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

/// One step of a sign-in with saved credentials: the first (`sealed_vault`,
/// and whether the student has a forwarding address), or the next with the
/// mailed `otp` (`sealed_state` from [`LoginStepped::OtpNeeded`]). No `Debug`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStep {
    account: String,
    #[serde(default)]
    sealed_vault: Option<String>,
    #[serde(default)]
    forwarding: bool,
    #[serde(default)]
    sealed_state: Option<String>,
    #[serde(default)]
    otp: Option<String>,
}

/// How a step ended: done, or waiting for the OTP mail. No `Debug`.
#[derive(Serialize)]
#[serde(untagged)]
pub enum LoginStepped {
    #[serde(rename_all = "camelCase")]
    OtpNeeded {
        status: &'static str,
        /// [`Pending`], sealed to the account.
        sealed_state: String,
        /// Only mail sent from then on (unix seconds) can hold the code.
        since: u64,
    },
    Done(LoginOutcome),
}

/// A sign-in waiting for its OTP, between two steps. No `Debug`.
#[derive(Serialize, Deserialize)]
struct Pending {
    session: SessionState,
    since: u64,
    /// Codes VTOP turned down, so a mail sent again is not tried again.
    tried: Vec<String>,
}

/// What a waiting sign-in is sealed to, apart from saved sessions.
fn logins_aad(account: &str) -> String {
    format!("logins/{account}")
}

/// Runs one step of a sign-in with saved credentials and answers how it
/// ended; nothing waits here for mail.
pub async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<LoginStep>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(step) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let account = account_id(&step.account);
    let stepped = match (step.sealed_state, step.otp, step.sealed_vault) {
        (Some(pending), Some(otp), _) => resume(&state, &account, &pending, otp.trim()).await,
        (None, None, Some(vault)) => start(&state, &account, &vault, step.forwarding).await,
        _ => {
            return Err(bad_request(
                "sealedVault, or sealedState and otp, are required.",
            ));
        }
    };
    Ok(Json(stepped).into_response())
}

async fn start(
    state: &AppState,
    account: &str,
    sealed_vault: &str,
    forwarding: bool,
) -> LoginStepped {
    let Ok(vault) = state
        .accounts_sealer()
        .open_json::<Vault>(&accounts_aad(account), sealed_vault)
    else {
        return LoginStepped::Done(LoginOutcome::VaultUnusable);
    };
    // Gmail when the phone picked it; else forwarding first, Gmail only
    // for students without it. Without either VTOP is not asked: it would
    // mail an OTP nobody can read here.
    match vault.gmail.clone().and_then(GmailGrant::usable) {
        Some(grant) if vault.otp == Some(OtpMethod::Gmail) || !forwarding => {
            return LoginStepped::Done(sign_in_with_gmail(state, account, vault, grant).await);
        }
        None if !forwarding => return LoginStepped::Done(LoginOutcome::ForwardingRequired),
        _ => {}
    }
    match state
        .forwarded_login
        .start(&vault.username, &vault.password)
        .await
    {
        Ok(Started::SignedIn(session)) => {
            LoginStepped::Done(signed_in(state, account, Ok(session)))
        }
        Ok(Started::OtpNeeded { session, issued_at }) => {
            let pending = Pending {
                session,
                since: issued_at.saturating_sub(CLOCK_SLACK_SECS),
                tried: Vec::new(),
            };
            otp_needed(state, account, &pending)
        }
        Err(error) => LoginStepped::Done(signed_in(state, account, Err(error))),
    }
}

/// Sends the mailed code on the waiting sign-in; one VTOP turns down (an
/// older mail's) leaves it waiting for the next mail.
async fn resume(state: &AppState, account: &str, sealed: &str, code: &str) -> LoginStepped {
    let Ok(mut pending) = state
        .accounts_sealer()
        .open_json::<Pending>(&logins_aad(account), sealed)
    else {
        return LoginStepped::Done(LoginOutcome::Failed {
            reason: "login_expired: the waiting sign-in does not open".into(),
        });
    };
    if pending.tried.iter().any(|tried| tried == code) {
        return otp_needed(state, account, &pending);
    }
    match state.forwarded_login.submit(&pending.session, code).await {
        Err(LoginError::OtpRejected) => {
            pending.tried.push(code.to_string());
            otp_needed(state, account, &pending)
        }
        result => LoginStepped::Done(signed_in(state, account, result)),
    }
}

fn otp_needed(state: &AppState, account: &str, pending: &Pending) -> LoginStepped {
    LoginStepped::OtpNeeded {
        status: "otp_needed",
        sealed_state: state
            .accounts_sealer()
            .seal_json(&logins_aad(account), pending),
        since: pending.since,
    }
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
        // Only a later step sends an OTP of its own; Gmail's are tried in its sign-in.
        Err(LoginError::OtpRejected) => LoginOutcome::Failed {
            reason: "otp_rejected: VTOP did not take the OTP".into(),
        },
        Err(LoginError::Failed(reason)) => {
            tracing::info!("engine: sign-in of {account} failed for now: {reason}");
            LoginOutcome::Failed { reason }
        }
    }
}

/// Convex: the owner hash of a regno the verifier (`vtop-verifier`) proved, `{registrationNumber}`
/// → `{ownerHash}`. Convex never holds `VAULT_KEY`, which keys the hash and seals the vaults.
pub const OWNER_HASH: &str = "/engine/owner-hash";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerHashBody {
    registration_number: String,
}

/// `POST` [`OWNER_HASH`].
pub async fn owner_hash_of(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<OwnerHashBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let account = account_id(&body.registration_number);
    if account.is_empty() || account.len() > 20 {
        return Err(bad_request("registrationNumber is required."));
    }
    Ok(Json(json!({ "ownerHash": owner_hash(&state.config.vault_key, &account) })).into_response())
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

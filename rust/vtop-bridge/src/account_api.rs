//! Account management for the app. Every call carries a live VTOP session;
//! the bridge asks VTOP whose it is and acts on that registration number
//! only.
//!
//! A session alone is not enough to change an account: every key holder can
//! get one from `/v1/session`. Linking hands the app a secret it keeps in
//! secure storage, and key, phone and delete changes need it. Linking again
//! without it (a reinstall) revokes every key and drops saved credentials,
//! so a leaked key cannot outlive its revocation.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use vtop_core::SessionState;

use crate::accounts::{AccountSettings, Vault};
use crate::error::ApiError;
use crate::gmail::GmailGrant;
use crate::keys::{self, key_id, KeyError};
use crate::verify::VerifyError;
use crate::AppState;

/// The proof every call carries. Only `cookies` is used.
#[derive(Deserialize)]
struct Proof {
    cookies: String,
}

/// No `Debug`: it holds a password.
#[derive(Deserialize)]
pub struct LinkCredentials {
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    gmail: Option<GmailGrant>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBody {
    session: Proof,
    #[serde(default)]
    fcm_token: Option<String>,
    #[serde(default)]
    credentials: Option<LinkCredentials>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    app_secret: Option<String>,
    #[serde(default)]
    settings: Option<serde_json::Value>,
}

fn bad_request(message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "bad_request", message)
}

fn store_unavailable(error: impl std::fmt::Display) -> ApiError {
    tracing::warn!("account store: {error}");
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "store_unavailable",
        "The store is unavailable.",
    )
}

fn not_linked() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "not_linked",
        "Link this account from the app first.",
    )
}

/// Parses the body and finds out from VTOP whose session it carries.
async fn verified(
    state: &AppState,
    body: Result<Json<AccountBody>, JsonRejection>,
) -> Result<(String, SessionState, AccountBody), ApiError> {
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let session = SessionState {
        cookies: body.session.cookies.trim().to_string(),
        csrf_token: None,
        registration_number: None,
        otp_issued_at: None,
        logged_in_at: None,
    };
    if session.cookies.is_empty() {
        return Err(bad_request("session.cookies is required."));
    }
    match state.verifier.registration_number(&session).await {
        Ok(account) => Ok((account, session, body)),
        Err(VerifyError::Expired) => Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "session_invalid",
            "VTOP does not accept this session; sign in again.",
        )),
        Err(VerifyError::Unavailable(reason)) => {
            tracing::info!("verify account session: {reason}");
            Err(ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "vtop_unreachable",
                "VTOP is unreachable; try again.",
            ))
        }
    }
}

async fn require_app_secret(
    state: &AppState,
    account: &str,
    body: &AccountBody,
) -> Result<(), ApiError> {
    match state
        .accounts
        .app_secret_matches(account, body.app_secret.as_deref())
        .await
    {
        Ok(true) => Ok(()),
        Ok(false) => Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "app_secret_invalid",
            "This phone is not the linked one; reconnect it from the app.",
        )),
        Err(error) => Err(store_unavailable(error)),
    }
}

async fn require_linked(state: &AppState, account: &str) -> Result<(), ApiError> {
    match state.accounts.exists(account).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(not_linked()),
        Err(error) => Err(store_unavailable(error)),
    }
}

pub async fn link(
    State(state): State<Arc<AppState>>,
    body: Result<Json<AccountBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (account, session, body) = verified(&state, body).await?;
    let fcm_token = body
        .fcm_token
        .as_deref()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| bad_request("fcmToken is required."))?;
    let vault = body.credentials.and_then(|credentials| {
        let gmail = credentials.gmail?;
        let username = credentials.username.trim().to_string();
        (!username.is_empty()
            && !credentials.password.is_empty()
            && !gmail.refresh_token.is_empty())
        .then_some(Vault {
            username,
            password: credentials.password,
            gmail,
        })
    });
    let same_app = state
        .accounts
        .app_secret_matches(&account, body.app_secret.as_deref())
        .await
        .map_err(store_unavailable)?;
    if !same_app {
        state
            .keys
            .revoke_all(&account)
            .await
            .map_err(store_unavailable)?;
    }
    let secret = keys::new_app_secret();
    state
        .accounts
        .link(
            &account,
            fcm_token,
            &session,
            vault,
            keys::hash(&secret),
            same_app,
            vtop_core::now_unix(),
        )
        .await
        .map_err(store_unavailable)?;
    if !same_app {
        tracing::info!("account {account}: linked a new app; keys revoked");
    }
    Ok(Json(json!({ "appSecret": secret })).into_response())
}

pub async fn create_key(
    State(state): State<Arc<AppState>>,
    body: Result<Json<AccountBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (account, _, body) = verified(&state, body).await?;
    require_linked(&state, &account).await?;
    require_app_secret(&state, &account, &body).await?;
    let label = body
        .label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .unwrap_or("Access key");
    match state
        .keys
        .create(&account, label, vtop_core::now_unix())
        .await
    {
        Ok((id, key)) => Ok((
            StatusCode::CREATED,
            Json(json!({ "id": id, "key": key, "mcpUrl": state.config.public_mcp_url })),
        )
            .into_response()),
        Err(KeyError::Limit) => Err(ApiError::new(
            StatusCode::CONFLICT,
            "key_limit",
            "This account already has the most keys allowed; revoke one first.",
        )),
        Err(KeyError::Store(error)) => Err(store_unavailable(error)),
    }
}

pub async fn account(
    State(state): State<Arc<AppState>>,
    body: Result<Json<AccountBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (account, _, body) = verified(&state, body).await?;
    let linked = state
        .accounts
        .exists(&account)
        .await
        .map_err(store_unavailable)?;
    let saved_credentials = state
        .accounts
        .vault(&account)
        .await
        .map_err(store_unavailable)?
        .is_some();
    let phone_linked = state
        .accounts
        .fcm_token(&account)
        .await
        .map_err(store_unavailable)?
        .is_some();
    let this_phone = state
        .accounts
        .app_secret_matches(&account, body.app_secret.as_deref())
        .await
        .map_err(store_unavailable)?;
    let listed = if this_phone {
        state.keys.list(&account).await.map_err(store_unavailable)?
    } else {
        Vec::new()
    };
    let keys: Vec<_> = listed
        .into_iter()
        .map(|key| {
            json!({
                "id": key_id(&key.hash),
                "label": key.label,
                "createdAt": key.created_at,
                "lastUsedAt": key.last_used_at,
            })
        })
        .collect();
    let settings = state
        .accounts
        .settings(&account)
        .await
        .map_err(store_unavailable)?;
    Ok(Json(json!({
        "registrationNumber": account,
        "settings": settings,
        "linked": linked,
        "savedCredentials": saved_credentials,
        "phoneLinked": phone_linked,
        "thisPhone": this_phone,
        "keys": keys,
    }))
    .into_response())
}

pub async fn revoke_key(
    State(state): State<Arc<AppState>>,
    body: Result<Json<AccountBody>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let (account, _, body) = verified(&state, body).await?;
    require_app_secret(&state, &account, &body).await?;
    let id = body.id.unwrap_or_default();
    match state.keys.revoke(&account, &id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "key_not_found",
            "No such key on this account.",
        )),
        Err(error) => Err(store_unavailable(error)),
    }
}

pub async fn update_fcm_token(
    State(state): State<Arc<AppState>>,
    body: Result<Json<AccountBody>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let (account, _, body) = verified(&state, body).await?;
    require_linked(&state, &account).await?;
    require_app_secret(&state, &account, &body).await?;
    let token = body
        .fcm_token
        .as_deref()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| bad_request("fcmToken is required."))?;
    state
        .accounts
        .set_fcm_token(&account, token, vtop_core::now_unix())
        .await
        .map_err(store_unavailable)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn forget_credentials(
    State(state): State<Arc<AppState>>,
    body: Result<Json<AccountBody>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let (account, _, _) = verified(&state, body).await?;
    state
        .accounts
        .clear_vault(&account)
        .await
        .map_err(store_unavailable)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_account(
    State(state): State<Arc<AppState>>,
    body: Result<Json<AccountBody>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let (account, _, body) = verified(&state, body).await?;
    require_app_secret(&state, &account, &body).await?;
    state
        .store
        .delete_account(&account)
        .await
        .map_err(store_unavailable)?;
    tracing::info!("account {account}: deleted at the user's request");
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    body: Result<Json<AccountBody>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let (account, _, body) = verified(&state, body).await?;
    require_linked(&state, &account).await?;
    require_app_secret(&state, &account, &body).await?;
    let settings = body
        .settings
        .and_then(|raw| serde_json::from_value::<AccountSettings>(raw).ok())
        .filter(AccountSettings::is_valid)
        .ok_or_else(|| bad_request("settings must use the offered choices."))?;
    state
        .accounts
        .set_settings(&account, settings, vtop_core::now_unix())
        .await
        .map_err(store_unavailable)?;
    Ok(StatusCode::NO_CONTENT)
}

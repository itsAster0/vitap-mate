//! Sign-in for a phone in "Fetch on the server" mode, kept nowhere: the
//! phone sends its username and password with each sign-in and gets the
//! session back. When VTOP wants the emailed OTP, the half-done session
//! goes back to the phone, which asks the student (or reads its forwarded
//! mail) and sends the code with that session to [`otp`].
//!
//! Routes (`Authorization: Bearer $MAIL_RELAY_SECRET`):
//! `POST /engine/vtop-login`, `/engine/vtop-login/otp`,
//! `/engine/vtop-login/resend`.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use vtop_core::inputs::{Password, Username};
use vtop_core::{SessionState, VtopClient, VtopError, VtopResult};

use crate::engine::require_secret;
use crate::error::ApiError;
use crate::fetch::vtop_error;
use crate::AppState;

/// A sign-in solves a CAPTCHA and may take VTOP a while; still under the
/// app's own limit.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(28);

/// No `Debug`: it holds a password.
#[derive(Deserialize)]
pub struct LoginBody {
    username: String,
    password: String,
}

/// No `Debug`: it holds cookies.
#[derive(Deserialize)]
pub struct OtpBody {
    session: SessionState,
    otp: String,
}

/// No `Debug`: it holds cookies.
#[derive(Deserialize)]
pub struct ResendBody {
    session: SessionState,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<LoginBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let username = Username::parse(body.username.trim()).map_err(|error| login_error(&error))?;
    let password = Password::parse(&body.password).map_err(|error| login_error(&error))?;
    let mut client = VtopClient::builder()
        .config(state.vtop.clone())
        .with_credentials(username, password)
        .map_err(|error| login_error(&error))?;
    let result = timed(client.login()).await;
    outcome(&client, result)
}

pub async fn otp(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<OtpBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let mut client = pending(&state, &body.session)?;
    let result = timed(client.submit_security_otp(body.otp.trim())).await;
    outcome(&client, result)
}

pub async fn resend(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<ResendBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let mut client = pending(&state, &body.session)?;
    timed(client.resend_security_otp())
        .await
        .map_err(|error| login_error(&error))?;
    let session = client.session_state();
    let issued_at = session.otp_issued_at;
    Ok(Json(json!({
        "status": "otp_required",
        "message": "A new OTP was sent to your registered email.",
        "issuedAt": issued_at,
        "session": session,
    }))
    .into_response())
}

/// A client on a session waiting for its OTP.
fn pending(state: &AppState, session: &SessionState) -> Result<VtopClient, ApiError> {
    if session.otp_issued_at.is_none() || session.cookies.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "no_pending_login",
            "This session is not waiting for an OTP; sign in again.",
        ));
    }
    VtopClient::builder()
        .config(state.vtop.clone())
        .with_session(session)
        .map_err(|error| login_error(&error))
}

async fn timed(sign_in: impl std::future::Future<Output = VtopResult<()>>) -> VtopResult<()> {
    tokio::time::timeout(LOGIN_TIMEOUT, sign_in)
        .await
        .unwrap_or_else(|_| {
            Err(VtopError::VtopUnavailable(
                "VTOP took too long to sign in.".into(),
            ))
        })
}

/// The session when signed in; the half-done one when VTOP wants the OTP.
fn outcome(client: &VtopClient, result: VtopResult<()>) -> Result<Response, ApiError> {
    match result {
        Ok(()) => {
            Ok(Json(json!({ "status": "ok", "session": client.session_state() })).into_response())
        }
        Err(VtopError::OTPRequired(message, issued_at)) => Ok(Json(json!({
            "status": "otp_required",
            "message": message,
            "issuedAt": issued_at,
            "session": client.session_state(),
        }))
        .into_response()),
        Err(error) => Err(login_error(&error)),
    }
}

fn bad_request(message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "bad_request", message)
}

/// How a sign-in failed, as the phone reads it back: a rejected password,
/// a refusal with VTOP's words (a wrong OTP among them), or what
/// [`vtop_error`] says for VTOP being down.
pub fn login_error(error: &VtopError) -> ApiError {
    match error {
        VtopError::InvalidCredentials => ApiError::new(
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            error.to_string(),
        ),
        VtopError::AuthenticationFailed(message) => ApiError::new(
            StatusCode::UNAUTHORIZED,
            "authentication_failed",
            message.clone(),
        ),
        VtopError::CaptchaRequired | VtopError::RegistrationParsingError => ApiError::new(
            StatusCode::BAD_GATEWAY,
            "authentication_failed",
            error.to_string(),
        ),
        // Inputs a username or password check rejects.
        VtopError::ConfigurationError(message) => {
            ApiError::new(StatusCode::BAD_REQUEST, "bad_arguments", message.clone())
        }
        _ => vtop_error(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wrong_password_and_a_wrong_otp_are_told_apart() {
        let api = login_error(&VtopError::InvalidCredentials);
        assert_eq!(
            (api.status, api.code),
            (StatusCode::UNAUTHORIZED, "invalid_credentials")
        );
        let api = login_error(&VtopError::AuthenticationFailed("Invalid OTP".into()));
        assert_eq!(api.code, "authentication_failed");
        assert_eq!(api.message, "Invalid OTP");
        let api = login_error(&VtopError::NetworkError);
        assert_eq!(
            (api.status, api.code),
            (StatusCode::BAD_GATEWAY, "vtop_unavailable")
        );
    }
}

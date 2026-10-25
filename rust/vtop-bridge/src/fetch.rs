//! `POST /engine/fetch`: one page, fetched and parsed here on the phone's
//! own session, for a phone in "Fetch on the server" mode. The phone signs
//! in itself; the engine only borrows the session for this request, keeps
//! nothing, and answers `{"data": …}` (the parsed page, as vtop-core's
//! serde JSON), Brotli-compressed when the caller accepts it; or, for a
//! phone holding the same page dictionary, the page packed with it.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use vtop_core::{SessionState, VtopClient, VtopError};
use vtop_shared::engine::PageFetch;
use vtop_shared::page_dictionary;

use crate::engine::require_secret;
use crate::error::ApiError;
use crate::page_pack;
use crate::pages::{self, Args};
use crate::AppState;

/// How long VTOP gets for one page. Under the app's own 30 s limit, so the
/// phone hears "VTOP is slow" from here instead of giving up.
const FETCH_TIMEOUT: Duration = Duration::from_secs(25);

pub async fn fetch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<PageFetch>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(request) = body.map_err(|_| bad_arguments("Invalid JSON body."))?;
    let session: SessionState = serde_json::from_value(request.session)
        .map_err(|_| bad_arguments("session is not a VTOP session."))?;
    if session.cookies.trim().is_empty() {
        return Err(expired());
    }
    let def = pages::find(&request.name)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "unknown_page", format!("No page named {}.", request.name)))?;
    let args: Args = request.args;
    if args.len() != def.args.len() || !def.args.iter().all(|name| args.contains_key(*name)) {
        return Err(bad_arguments(format!("{} takes: {}.", def.name, def.args.join(", "))));
    }
    let client = VtopClient::builder()
        .config(state.vtop.clone())
        .with_session(&session)
        .map_err(|error| vtop_error(&error))?;
    let semester = request.semester.unwrap_or_default();
    let data = tokio::time::timeout(FETCH_TIMEOUT, (def.call)(&client, &semester, &args))
        .await
        .map_err(|_| ApiError::new(StatusCode::GATEWAY_TIMEOUT, "vtop_timeout", "VTOP took too long to answer."))?
        .map_err(|error| vtop_error(&error))?;
    if request.dictionary.as_deref() == Some(page_dictionary::id()) {
        if let Some(packed) = page_pack::pack(def.name, &data) {
            return Ok(([(header::CONTENT_TYPE, page_dictionary::CONTENT_TYPE)], packed).into_response());
        }
    }
    Ok(Json(serde_json::json!({ "data": data })).into_response())
}

fn bad_arguments(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "bad_arguments", message)
}

fn expired() -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, "session_expired", "Session has expired")
}

/// A vtop-core error as the phone reads it back: `session_expired` makes
/// it sign in again; the rest name what failed. VTOP's own messages carry
/// no session data.
pub fn vtop_error(error: &VtopError) -> ApiError {
    let message = error.to_string();
    match error {
        // A borrowed session cannot sign in, so any refusal is an expiry.
        VtopError::SessionExpired
        | VtopError::AuthenticationFailed(_)
        | VtopError::InvalidCredentials
        | VtopError::OTPRequired(..) => expired(),
        VtopError::NetworkError | VtopError::VtopServerError(_) | VtopError::VtopUnavailable(_) => {
            ApiError::new(StatusCode::BAD_GATEWAY, "vtop_unavailable", message)
        }
        VtopError::ParseError(_) | VtopError::InvalidResponse => {
            ApiError::new(StatusCode::BAD_GATEWAY, "parse_error", message)
        }
        // vtop-core's inputs reject a bad argument this way.
        VtopError::ConfigurationError(_) => bad_arguments(message),
        _ => ApiError::new(StatusCode::BAD_GATEWAY, "vtop_error", message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiries_ask_the_phone_to_sign_in() {
        for error in [
            VtopError::SessionExpired,
            VtopError::AuthenticationFailed("x".into()),
            VtopError::OTPRequired("x".into(), 1),
        ] {
            let api = vtop_error(&error);
            assert_eq!((api.status, api.code), (StatusCode::UNAUTHORIZED, "session_expired"));
        }
    }

    #[test]
    fn vtop_being_down_is_a_bad_gateway() {
        let api = vtop_error(&VtopError::VtopUnavailable("slow".into()));
        assert_eq!((api.status, api.code), (StatusCode::BAD_GATEWAY, "vtop_unavailable"));
        let api = vtop_error(&VtopError::ConfigurationError("bad".into()));
        assert_eq!((api.status, api.code), (StatusCode::BAD_REQUEST, "bad_arguments"));
    }
}

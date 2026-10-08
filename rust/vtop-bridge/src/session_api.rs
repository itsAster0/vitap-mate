//! The session API every client (the extension, vtop-mcp) uses, behind
//! `Authorization: Bearer <access key>`.

use std::sync::Arc;

use axum::extract::{Path, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde_json::json;
use vtop_core::SessionState;

use crate::broker::{BrokerError, RequestView, Started};
use crate::cookies;
use crate::error::ApiError;
use crate::requests::rfc3339;
use crate::AppState;

/// The account the presented key belongs to.
#[derive(Clone)]
pub struct Account(pub String);

/// The label of the key a request came with.
#[derive(Clone)]
pub struct KeyLabel(pub String);

fn store_unavailable() -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "store_unavailable",
        "The store is unavailable.",
    )
}

impl From<BrokerError> for ApiError {
    fn from(error: BrokerError) -> Self {
        match error {
            BrokerError::NotFound => ApiError::new(
                StatusCode::NOT_FOUND,
                "request_not_found",
                "Unknown session request.",
            ),
            BrokerError::Store(error) => {
                tracing::warn!("session store: {error}");
                store_unavailable()
            }
        }
    }
}

pub async fn require_key(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let presented = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    let key = match state
        .keys
        .resolve_key(&presented, vtop_core::now_unix())
        .await
    {
        Ok(Some(key)) => key,
        Ok(None) => {
            return ApiError::new(
                StatusCode::UNAUTHORIZED,
                "key_unknown",
                "Unknown or revoked access key.",
            )
            .into_response()
        }
        Err(error) => {
            tracing::warn!("resolve access key: {error}");
            return store_unavailable().into_response();
        }
    };
    let account = key.registration_number;
    if !state.account_limiter.allow(&account) {
        let mut response = ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Too many requests for this account.",
        )
        .into_response();
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
        return response;
    }
    request.extensions_mut().insert(Account(account));
    request.extensions_mut().insert(KeyLabel(key.label));
    next.run(request).await
}

fn ready(session: &SessionState) -> Response {
    Json(json!({
        "status": "ready",
        "session": session,
        "cookies": cookies::from_header(&session.cookies),
    }))
    .into_response()
}

pub async fn whoami(
    Extension(Account(account)): Extension<Account>,
    Extension(KeyLabel(label)): Extension<KeyLabel>,
) -> Response {
    Json(json!({ "registrationNumber": account, "keyLabel": label })).into_response()
}

pub async fn post_session(
    State(state): State<Arc<AppState>>,
    Extension(Account(account)): Extension<Account>,
) -> Result<Response, ApiError> {
    match state.broker.start(&account).await? {
        Started::Ready(session) => Ok(ready(&session)),
        Started::Pending(doc) => {
            let poll_after = state.config.poll_after;
            let mut response = (
                StatusCode::ACCEPTED,
                Json(json!({
                    "status": "pending",
                    "requestId": doc.id,
                    "pollAfterMs": poll_after.as_millis() as u64,
                    "expiresAt": rfc3339(doc.expires_at),
                })),
            )
                .into_response();
            let secs = poll_after.as_millis().div_ceil(1000).max(1).to_string();
            response.headers_mut().insert(
                header::RETRY_AFTER,
                HeaderValue::from_str(&secs).expect("digits"),
            );
            Ok(response)
        }
    }
}

pub async fn get_request(
    State(state): State<Arc<AppState>>,
    Extension(Account(account)): Extension<Account>,
    Path(request_id): Path<String>,
) -> Result<Response, ApiError> {
    Ok(
        match state.broker.poll(&account, request_id.trim()).await? {
            RequestView::Pending => {
                (StatusCode::ACCEPTED, Json(json!({ "status": "pending" }))).into_response()
            }
            RequestView::Ready(session) => ready(&session),
            RequestView::Error(error) => {
                let code = error
                    .split_once(':')
                    .map_or("login_failed", |(code, _)| code)
                    .trim()
                    .to_string();
                Json(json!({ "status": "error", "code": code, "error": error })).into_response()
            }
            RequestView::Expired => {
                return Err(ApiError::new(
                    StatusCode::GONE,
                    "request_expired",
                    "The session request expired.",
                ))
            }
        },
    )
}

pub async fn post_expire(
    State(state): State<Arc<AppState>>,
    Extension(Account(account)): Extension<Account>,
) -> Result<StatusCode, ApiError> {
    state.broker.expire(&account).await?;
    Ok(StatusCode::NO_CONTENT)
}

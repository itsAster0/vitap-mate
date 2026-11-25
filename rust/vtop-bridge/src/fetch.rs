//! `POST /engine/fetch`: one page, fetched and parsed here on the phone's
//! own session, for a phone in "Fetch on the server" mode. The phone signs
//! in itself; the engine only borrows the session for this request, keeps
//! nothing, and answers `{"data": …}` (the parsed page, as vtop-core's
//! serde JSON), Brotli-compressed when the caller accepts it; or, for a
//! phone holding the same page dictionary, the page packed with it.
//! `POST /engine/fetch-batch` ([`fetch_batch`]) does several pages in one round trip.

use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use serde_json::{Value, json};
use vtop_core::{SessionState, VtopClient, VtopError};
use vtop_shared::engine::{PageBatch, PageFetch};
use vtop_shared::page_dictionary;
use vtop_shared::wire_page::BATCH;

use crate::AppState;
use crate::engine::require_gateway;
use crate::error::ApiError;
use crate::page_pack;
use crate::pages::{self, Args};

/// How long VTOP gets for one page. Under the app's own 30 s limit, so the
/// phone hears "VTOP is slow" from here instead of giving up.
const FETCH_TIMEOUT: Duration = Duration::from_secs(25);

pub async fn fetch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<PageFetch>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_gateway(&state, &headers)?;
    let Json(request) = body.map_err(|_| bad_arguments("Invalid JSON body."))?;
    let session: SessionState = serde_json::from_value(request.session)
        .map_err(|_| bad_arguments("session is not a VTOP session."))?;
    if session.cookies.trim().is_empty() {
        return Err(expired());
    }
    let client = client(&state, &session)?;
    let semester = request.semester.unwrap_or_default();
    let data = page(&client, &request.name, &semester, &request.args).await?;
    if request.dictionary.as_deref() == Some(page_dictionary::id())
        && let Some(packed) = page_pack::pack(&request.name, &data)
    {
        return Ok((
            [(header::CONTENT_TYPE, page_dictionary::CONTENT_TYPE)],
            packed,
        )
            .into_response());
    }
    Ok(Json(serde_json::json!({ "data": data })).into_response())
}

/// The pages a batch may name; more are refused.
const MAX_BATCH: usize = 16;
/// Pages of a batch fetched at once on its one client.
const BATCH_AT_ONCE: usize = 4;

/// `POST /engine/fetch-batch`: [`PageBatch`]'s pages on one session, at most
/// [`BATCH_AT_ONCE`] at a time, all within [`FETCH_TIMEOUT`]. Each answer is the page's
/// `{"data"}` or its own `{"code", "error"}`, so one bad page does not fail the rest; an
/// expired session fails the whole batch, as it would every page.
pub async fn fetch_batch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<PageBatch>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_gateway(&state, &headers)?;
    let Json(request) = body.map_err(|_| bad_arguments("Invalid JSON body."))?;
    if request.pages.len() > MAX_BATCH {
        return Err(bad_arguments(format!("At most {MAX_BATCH} pages.")));
    }
    let session: SessionState = serde_json::from_value(request.session)
        .map_err(|_| bad_arguments("session is not a VTOP session."))?;
    if session.cookies.trim().is_empty() {
        return Err(expired());
    }
    let client = client(&state, &session)?;
    let semester = request.semester.unwrap_or_default();
    let deadline = tokio::time::Instant::now() + FETCH_TIMEOUT;
    // Built first: a closure inside the stream trips axum's `Send` check.
    let pages: Vec<_> = request
        .pages
        .iter()
        .map(|p| tokio::time::timeout_at(deadline, page(&client, &p.name, &semester, &p.args)))
        .collect();
    let answers: Vec<Result<Value, PageError>> = futures::stream::iter(pages)
        .buffered(BATCH_AT_ONCE)
        .map(|answer| answer.unwrap_or_else(|_| Err(PageError::Failed(timed_out()))))
        .collect()
        .await;
    if answers
        .iter()
        .any(|answer| matches!(answer, Err(PageError::Expired)))
    {
        return Err(expired());
    }
    let data: Vec<Value> = answers
        .into_iter()
        .map(|answer| match answer.map_err(ApiError::from) {
            Ok(data) => json!({ "data": data }),
            Err(e) => json!({ "code": e.code, "error": e.message }),
        })
        .collect();
    tracing::info!(pages = data.len(), "batch");
    let data = json!({ "data": data });
    if request.dictionary.as_deref() == Some(page_dictionary::id())
        && let Some(packed) = page_pack::pack(BATCH, &data["data"])
    {
        return Ok((
            [(header::CONTENT_TYPE, page_dictionary::CONTENT_TYPE)],
            packed,
        )
            .into_response());
    }
    Ok(Json(data).into_response())
}

/// A client on the phone's session, borrowed for this request.
fn client(state: &AppState, session: &SessionState) -> Result<VtopClient, ApiError> {
    VtopClient::builder()
        .config(state.vtop.clone())
        .with_session(session)
        .map_err(|error| vtop_error(&error))
}

/// Why a page gave no data: the session ran out (which fails a whole batch), or anything else.
enum PageError {
    Expired,
    Failed(ApiError),
}

impl From<PageError> for ApiError {
    fn from(error: PageError) -> Self {
        match error {
            PageError::Expired => expired(),
            PageError::Failed(error) => error,
        }
    }
}

impl From<ApiError> for PageError {
    fn from(error: ApiError) -> Self {
        Self::Failed(error)
    }
}

/// Page `name` with its `args` checked, within [`FETCH_TIMEOUT`].
async fn page(
    client: &VtopClient,
    name: &str,
    semester: &str,
    args: &Args,
) -> Result<Value, PageError> {
    let def = pages::find(name).ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "unknown_page",
            format!("No page named {name}."),
        )
    })?;
    if args.len() != def.args.len() || !def.args.iter().all(|name| args.contains_key(*name)) {
        return Err(bad_arguments(format!("{} takes: {}.", def.name, def.args.join(", "))).into());
    }
    tokio::time::timeout(FETCH_TIMEOUT, (def.call)(client, semester, args))
        .await
        .map_err(|_| timed_out())?
        .map_err(|error| page_error(&error))
}

fn timed_out() -> ApiError {
    ApiError::new(
        StatusCode::GATEWAY_TIMEOUT,
        "vtop_timeout",
        "VTOP took too long to answer.",
    )
}

fn bad_arguments(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "bad_arguments", message)
}

fn expired() -> ApiError {
    ApiError::new(
        StatusCode::UNAUTHORIZED,
        "session_expired",
        "Session has expired",
    )
}

/// A vtop-core error as the phone reads it back: `session_expired` makes
/// it sign in again; the rest name what failed. VTOP's own messages carry
/// no session data.
pub fn vtop_error(error: &VtopError) -> ApiError {
    page_error(error).into()
}

/// [`vtop_error`], an expiry kept apart.
fn page_error(error: &VtopError) -> PageError {
    let message = error.to_string();
    PageError::Failed(match error {
        // A borrowed session cannot sign in, so any refusal is an expiry.
        VtopError::SessionExpired
        | VtopError::AuthenticationFailed(_)
        | VtopError::OtpExpired(_)
        | VtopError::InvalidCredentials
        | VtopError::OTPRequired(..) => return PageError::Expired,
        VtopError::NetworkError | VtopError::VtopServerError(_) | VtopError::VtopUnavailable(_) => {
            ApiError::new(StatusCode::BAD_GATEWAY, "vtop_unavailable", message)
        }
        VtopError::ParseError(_) | VtopError::InvalidResponse => {
            ApiError::new(StatusCode::BAD_GATEWAY, "parse_error", message)
        }
        // vtop-core's inputs reject a bad argument this way.
        VtopError::ConfigurationError(_) => bad_arguments(message),
        _ => ApiError::new(StatusCode::BAD_GATEWAY, "vtop_error", message),
    })
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
            // Kept apart, so a batch fails whole on it.
            assert!(matches!(page_error(&error), PageError::Expired));
            let api = vtop_error(&error);
            assert_eq!(
                (api.status, api.code),
                (StatusCode::UNAUTHORIZED, "session_expired")
            );
        }
        assert!(matches!(
            page_error(&VtopError::NetworkError),
            PageError::Failed(_)
        ));
    }

    #[test]
    fn vtop_being_down_is_a_bad_gateway() {
        let api = vtop_error(&VtopError::VtopUnavailable("slow".into()));
        assert_eq!(
            (api.status, api.code),
            (StatusCode::BAD_GATEWAY, "vtop_unavailable")
        );
        let api = vtop_error(&VtopError::ConfigurationError("bad".into()));
        assert_eq!(
            (api.status, api.code),
            (StatusCode::BAD_REQUEST, "bad_arguments")
        );
    }
}

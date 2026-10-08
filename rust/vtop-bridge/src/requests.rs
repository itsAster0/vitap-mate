//! The cookie-request API the extension and the app use, kept identical to
//! the Go bridge: start a request, poll its status, acknowledge it, and the
//! phone's callback.

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, Path, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rand::RngCore;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::cookies::{self, CookieEditorCookie};
use crate::error::ApiError;
use crate::fcm::{FcmError, Messenger};
use crate::gmail::GmailGrant;
use vtop_core::SessionState;

use crate::accounts::Vault;
use crate::store::{
    tokens_equal, CompleteError, Completion, RequestDoc, RequestStatus, Store, StoreError,
};
use crate::verify::VerifyError;
use crate::AppState;

const MAX_FCM_TOKEN_LEN: usize = 4096;

fn store_unavailable(message: &str) -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "STORE_UNAVAILABLE",
        message,
    )
}

fn invalid_json() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "INVALID_REQUEST",
        "Invalid JSON body.",
    )
}

fn request_id_invalid() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "REQUEST_ID_INVALID",
        "Invalid requestId.",
    )
}

fn expired() -> ApiError {
    ApiError::new(
        StatusCode::GONE,
        "REQUEST_EXPIRED",
        "The cookie request expired.",
    )
}

pub fn fcm_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn rfc3339(secs: u64) -> String {
    i64::try_from(secs)
        .ok()
        .and_then(|secs| OffsetDateTime::from_unix_timestamp(secs).ok())
        .and_then(|time| time.format(&Rfc3339).ok())
        .unwrap_or_default()
}

fn retry_after(poll_after: Duration) -> HeaderValue {
    let secs = poll_after.as_millis().div_ceil(1000).max(1);
    HeaderValue::from_str(&secs.to_string()).expect("digits are a valid header value")
}

fn valid_request_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id.trim()).is_ok()
}

/// Sends cookie requests to the phone.
pub struct PhoneRequests {
    pub store: Arc<dyn Store>,
    pub messenger: Arc<dyn Messenger>,
    pub public_base_url: String,
    pub request_timeout: Duration,
}

impl PhoneRequests {
    /// Legacy flow: creates a pending request and sends it to the phone. On
    /// an FCM failure the request is deleted again.
    pub async fn start(
        &self,
        fcm_token: &str,
        want_credentials: bool,
    ) -> Result<RequestDoc, ApiError> {
        let doc = self.new_doc(Some(fcm_hash(fcm_token)), None, want_credentials);
        if let Err(error) = self.store.create_request(&doc).await {
            tracing::warn!("create cookie request {}: {error}", doc.id);
            return Err(store_unavailable("The request store is unavailable."));
        }
        if let Err(error) = self.notify(&doc, fcm_token, want_credentials).await {
            self.delete_quietly(&doc.id).await;
            return Err(match error {
                FcmError::TokenInvalid => ApiError::new(
                    StatusCode::GONE,
                    "TOKEN_INVALID",
                    "The mobile token is invalid or expired.",
                ),
                FcmError::Misconfigured => ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "FCM_MISCONFIGURED",
                    "Mobile messaging is unavailable.",
                ),
                FcmError::Unavailable => ApiError::new(
                    StatusCode::BAD_GATEWAY,
                    "FCM_UNAVAILABLE",
                    "Could not contact the mobile app.",
                ),
            });
        }
        Ok(doc)
    }

    /// Stores a pending request for an account; nothing is sent yet.
    pub async fn create_for_account(
        &self,
        registration_number: &str,
    ) -> Result<RequestDoc, StoreError> {
        let doc = self.new_doc(None, Some(registration_number.to_string()), false);
        self.store.create_request(&doc).await?;
        Ok(doc)
    }

    fn new_doc(
        &self,
        fcm_hash: Option<String>,
        account: Option<String>,
        want_credentials: bool,
    ) -> RequestDoc {
        let now = vtop_core::now_unix();
        RequestDoc {
            id: uuid::Uuid::new_v4().to_string(),
            response_token: Some(random_token()),
            status: RequestStatus::Pending,
            cookies_json: None,
            error: None,
            fcm_hash,
            want_credentials,
            account,
            created_at: now,
            expires_at: now + self.request_timeout.as_secs(),
        }
    }

    /// Sends `doc` to the phone behind `fcm_token`.
    pub async fn notify(
        &self,
        doc: &RequestDoc,
        fcm_token: &str,
        want_credentials: bool,
    ) -> Result<(), FcmError> {
        let data = BTreeMap::from([
            ("type".to_string(), "vtop_cookie_request".to_string()),
            ("requestId".to_string(), doc.id.clone()),
            (
                "responseToken".to_string(),
                doc.response_token.clone().unwrap_or_default(),
            ),
            (
                "callbackUrl".to_string(),
                format!("{}/cookie/callback", self.public_base_url),
            ),
            (
                "wantCredentials".to_string(),
                if want_credentials { "1" } else { "0" }.to_string(),
            ),
        ]);
        self.messenger.send(fcm_token, &data).await
    }

    async fn delete_quietly(&self, id: &str) {
        if let Err(error) = self.store.delete_request(id).await {
            tracing::warn!("delete cookie request {id}: {error}");
        }
    }
}

// ---- POST /cookie ---------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieBody {
    #[serde(default)]
    fcm_token: String,
    /// The legacy spelling.
    #[serde(default)]
    fmc_token: String,
}

/// 202 with the request's id, expiry and poll interval.
pub fn started_response(state: &AppState, doc: &RequestDoc) -> Response {
    let mut response = (
        StatusCode::ACCEPTED,
        Json(json!({
            "requestId": doc.id,
            "status": "pending",
            "expiresAt": rfc3339(doc.expires_at),
            "pollAfterMs": state.config.poll_after.as_millis() as u64,
        })),
    )
        .into_response();
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, retry_after(state.config.poll_after));
    response
}

/// Reads and checks the FCM token from a `/cookie` body.
pub fn fcm_token_from(body: Result<Json<CookieBody>, JsonRejection>) -> Result<String, ApiError> {
    let Json(body) = body.map_err(|_| invalid_json())?;
    let token = match body.fcm_token.trim() {
        "" => body.fmc_token.trim().to_string(),
        token => token.to_string(),
    };
    if token.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "TOKEN_REQUIRED",
            "fcmToken is required.",
        ));
    }
    if token.len() > MAX_FCM_TOKEN_LEN {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "TOKEN_INVALID",
            "The FCM token is invalid.",
        ));
    }
    Ok(token)
}

/// Legacy flow for the FCM-token extension: always asks the phone and
/// records nothing.
pub async fn post_cookie(
    State(state): State<Arc<AppState>>,
    body: Result<Json<CookieBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let token = fcm_token_from(body)?;
    let doc = state.phone.start(&token, false).await?;
    Ok(started_response(&state, &doc))
}

// ---- GET/DELETE /cookie/status/:id ----------------------------------------

fn request_not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "REQUEST_NOT_FOUND",
        "Unknown requestId.",
    )
}

pub async fn get_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let id = id.trim();
    if !valid_request_id(id) {
        return Err(request_id_invalid());
    }
    let doc = match state.store.get_request(id).await {
        // Account requests are only readable through the key-checked
        // session API.
        Ok(Some(doc)) if doc.account.is_none() => doc,
        Ok(_) => return Err(request_not_found()),
        Err(error) => {
            tracing::warn!("get cookie request {id}: {error}");
            return Err(store_unavailable("The request store is unavailable."));
        }
    };
    if vtop_core::now_unix() >= doc.expires_at {
        state.phone.delete_quietly(id).await;
        return Err(expired());
    }
    match doc.status {
        RequestStatus::Pending => {
            let mut response =
                (StatusCode::ACCEPTED, Json(json!({ "status": "pending" }))).into_response();
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, retry_after(state.config.poll_after));
            Ok(response)
        }
        RequestStatus::Success => {
            let cookies: Vec<CookieEditorCookie> = doc
                .cookies_json
                .as_deref()
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default();
            if cookies::validate(&cookies).is_err() {
                tracing::warn!("invalid stored cookies for request {id}");
                return Err(ApiError::new(
                    StatusCode::BAD_GATEWAY,
                    "COOKIES_INVALID",
                    "The mobile app returned invalid cookies.",
                ));
            }
            Ok(Json(json!({ "status": "success", "cookies": cookies })).into_response())
        }
        RequestStatus::Error => Ok(Json(json!({
            "status": "error",
            "error": doc.error.unwrap_or_default(),
        }))
        .into_response()),
    }
}

pub async fn delete_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = id.trim();
    if !valid_request_id(id) {
        return Err(request_id_invalid());
    }
    match state.store.get_request(id).await {
        Ok(Some(doc)) if doc.account.is_some() => return Err(request_not_found()),
        Ok(_) => {}
        Err(error) => {
            tracing::warn!("get cookie request {id}: {error}");
            return Err(store_unavailable("The request store is unavailable."));
        }
    }
    if let Err(error) = state.store.delete_request(id).await {
        tracing::warn!("delete acknowledged cookie request {id}: {error}");
        return Err(store_unavailable("Could not acknowledge the request."));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---- POST /cookie/callback ------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallbackBody {
    #[serde(default)]
    pub request_id: String,
    #[serde(default)]
    pub response_token: String,
    #[serde(default)]
    pub cookies: Vec<CookieEditorCookie>,
    #[serde(default)]
    pub error: String,
    /// Sent by newer apps: whose session this is, and how to reach them.
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub fcm_token: Option<String>,
    /// Only when the request asked for them.
    #[serde(default)]
    pub credentials: Option<CallbackCredentials>,
}

/// No `Debug`: it holds the password.
#[derive(Deserialize)]
pub struct CallbackCredentials {
    /// Falls back to the body's `username`.
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub gmail: Option<GmailGrant>,
}

pub async fn post_callback(
    State(state): State<Arc<AppState>>,
    body: Result<Json<CallbackBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(body) = body.map_err(|_| invalid_json())?;
    match verify_callback(&state, &body).await? {
        Verified::Late(account) => {
            // The client gave up waiting (the user was typing an OTP), but the
            // answer is genuine: cache it so the next call is served at once.
            record_phone_answer(&state, &account, body).await;
            Ok(Json(json!({ "ok": true, "late": true })).into_response())
        }
        Verified::Current(account) => {
            complete_callback(&state, &body).await?;
            record_phone_answer(&state, &account, body).await;
            Ok(Json(json!({ "ok": true })).into_response())
        }
        Verified::NotAccountRequest => {
            complete_callback(&state, &body).await?;
            Ok(Json(json!({ "ok": true })).into_response())
        }
    }
}

enum Verified {
    /// Legacy request, error answer, or something `complete_callback`
    /// reports (bad token, already handled).
    NotAccountRequest,
    /// A pending request of this account.
    Current(String),
    /// This account's request had expired; the answer is still genuine.
    Late(String),
}

/// Ends a request the phone answered with cookies the bridge cannot use, so
/// the caller stops waiting. The account is left untouched.
async fn fail_request(state: &AppState, doc: &RequestDoc, body: &CallbackBody, error: &str) {
    let completion = Completion {
        status: RequestStatus::Error,
        cookies_json: None,
        error: Some(error.to_string()),
    };
    if let Err(error) = state
        .store
        .complete_request(
            &doc.id,
            body.response_token.trim(),
            completion,
            vtop_core::now_unix(),
        )
        .await
    {
        tracing::info!("fail cookie request {}: {error:?}", doc.id);
    }
}

/// For a request an account is waiting on, checks the response token and
/// then asks VTOP whose cookies these are; they must be the account's.
async fn verify_callback(state: &AppState, body: &CallbackBody) -> Result<Verified, ApiError> {
    let id = body.request_id.trim();
    if !body.error.trim().is_empty()
        || !valid_request_id(id)
        || cookies::validate(&body.cookies).is_err()
    {
        return Ok(Verified::NotAccountRequest);
    }
    let doc = match state.store.get_request(id).await {
        Ok(Some(doc)) => doc,
        Ok(None) => return Ok(Verified::NotAccountRequest),
        Err(error) => {
            tracing::warn!("read cookie request {id}: {error}");
            return Err(store_unavailable("Could not save the callback."));
        }
    };
    let Some(account) = doc.account.clone() else {
        return Ok(Verified::NotAccountRequest);
    };
    let token_matches = doc
        .response_token
        .as_deref()
        .is_some_and(|expected| tokens_equal(expected, body.response_token.trim()));
    if !token_matches || doc.status != RequestStatus::Pending {
        // complete_callback reports these.
        return Ok(Verified::NotAccountRequest);
    }
    let late = vtop_core::now_unix() >= doc.expires_at;
    let session = phone_session(&body.cookies);
    match state.verifier.registration_number(&session).await {
        Ok(owner) if owner == account && late => Ok(Verified::Late(account)),
        Ok(owner) if owner == account => Ok(Verified::Current(account)),
        Ok(_) => {
            fail_request(
                state,
                &doc,
                body,
                "account_mismatch: Your phone is signed in to VTOP as a different student.",
            )
            .await;
            Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "account_mismatch",
                "These cookies belong to a different student than the one who asked.",
            ))
        }
        Err(VerifyError::Expired) => {
            fail_request(
                state,
                &doc,
                body,
                "phone_session_invalid: VTOP rejected your phone's session. Open VITAP Mate, refresh your login, and try again.",
            )
            .await;
            Err(ApiError::new(
                StatusCode::UNAUTHORIZED,
                "session_invalid",
                "VTOP does not accept these cookies.",
            ))
        }
        Err(VerifyError::Unavailable(reason)) => {
            tracing::info!("verify callback for {id}: {reason}");
            Err(ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "vtop_unreachable",
                "VTOP is unreachable; try again.",
            ))
        }
    }
}

fn phone_session(cookies: &[CookieEditorCookie]) -> SessionState {
    SessionState {
        cookies: cookies::to_header(cookies),
        csrf_token: None,
        registration_number: None,
        otp_issued_at: None,
        logged_in_at: None,
    }
}

/// Caches a verified phone answer, and the vault when the user consented
/// (credentials with Gmail access). Failures are logged only: the request
/// is already complete.
async fn record_phone_answer(state: &AppState, account: &str, body: CallbackBody) {
    let session = phone_session(&body.cookies);
    let vault = body.credentials.and_then(|credentials| {
        let gmail = credentials.gmail?;
        let username = match credentials.username.trim() {
            "" => body
                .username
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string(),
            name => name.to_string(),
        };
        (!username.is_empty()
            && !credentials.password.is_empty()
            && !gmail.refresh_token.is_empty())
        .then_some(Vault {
            username,
            password: credentials.password,
            gmail,
        })
    });
    let fcm_token = body
        .fcm_token
        .as_deref()
        .map(str::trim)
        .filter(|token| !token.is_empty());
    if let Err(error) = state
        .accounts
        .record_phone(account, fcm_token, &session, vault, vtop_core::now_unix())
        .await
    {
        tracing::warn!("record phone answer for {account}: {error}");
    }
}

/// Validates the callback and completes its request.
pub async fn complete_callback(
    state: &AppState,
    body: &CallbackBody,
) -> Result<RequestDoc, ApiError> {
    let id = body.request_id.trim();
    let token = body.response_token.trim();
    let error = body.error.trim();
    if id.is_empty() || token.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "CALLBACK_INVALID",
            "requestId and responseToken are required.",
        ));
    }
    if !valid_request_id(id) {
        return Err(request_id_invalid());
    }
    if error.is_empty() && cookies::validate(&body.cookies).is_err() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "COOKIES_INVALID",
            "The callback contained invalid cookies.",
        ));
    }

    let completion = if error.is_empty() {
        Completion {
            status: RequestStatus::Success,
            cookies_json: Some(serde_json::to_string(&body.cookies).expect("cookies serialise")),
            error: None,
        }
    } else {
        Completion {
            status: RequestStatus::Error,
            cookies_json: None,
            error: Some(error.to_string()),
        }
    };
    match state
        .store
        .complete_request(id, token, completion, vtop_core::now_unix())
        .await
    {
        Ok(doc) => Ok(doc),
        Err(CompleteError::NotFound | CompleteError::BadToken) => Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "CALLBACK_NOT_FOUND",
            "Unknown requestId or invalid responseToken.",
        )),
        Err(CompleteError::Expired) => {
            state.phone.delete_quietly(id).await;
            Err(expired())
        }
        Err(CompleteError::AlreadyHandled) => Err(ApiError::new(
            StatusCode::CONFLICT,
            "REQUEST_COMPLETED",
            "The cookie request is already completed.",
        )),
        Err(CompleteError::Store(error)) => {
            tracing::warn!("complete cookie request {id}: {error}");
            Err(store_unavailable("Could not save the callback."))
        }
    }
}

// ---- middleware -----------------------------------------------------------

/// Fixed-window limit per client key.
pub struct RateLimiter {
    limit: u32,
    window: Duration,
    hits: Mutex<HashMap<String, (Instant, u32)>>,
}

impl RateLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit,
            window,
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// Counts a hit for `key`; false when the window is full.
    pub fn allow(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut hits = self
            .hits
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        hits.retain(|_, (start, _)| now.duration_since(*start) < self.window);
        let entry = hits.entry(key.to_string()).or_insert((now, 0));
        entry.1 += 1;
        entry.1 <= self.limit
    }
}

/// The client address: the first `X-Forwarded-For` hop (Railway's proxy
/// sets it), else the peer.
fn client_key(headers: &HeaderMap, request: &Request) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(|ip| ip.trim().to_string())
        .filter(|ip| !ip.is_empty())
        .or_else(|| {
            request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ConnectInfo(addr)| addr.ip().to_string())
        })
        .unwrap_or_else(|| "unknown".into())
}

pub async fn rate_limit(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let key = client_key(request.headers(), &request);
    if !state.rate_limiter.allow(&key) {
        let mut response = ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMITED",
            "Too many login requests. Try again shortly.",
        )
        .into_response();
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
        return response;
    }
    next.run(request).await
}

/// CORS as in the Go bridge; any `OPTIONS` is a 204 preflight.
pub async fn cors(State(state): State<Arc<AppState>>, request: Request, next: Next) -> Response {
    let origin = request.headers().get(header::ORIGIN).cloned();
    let mut response = if request.method() == axum::http::Method::OPTIONS {
        StatusCode::NO_CONTENT.into_response()
    } else {
        next.run(request).await
    };
    let headers = response.headers_mut();
    let allowed = &state.config.allowed_origins;
    if allowed.is_empty() {
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("*"),
        );
    } else if let Some(origin) = origin.filter(|origin| {
        origin
            .to_str()
            .is_ok_and(|origin| allowed.iter().any(|allowed| allowed == origin))
    }) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    }
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, DELETE, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type"),
    );
    response
}

//! FCM cookie bridge, credential vault and VTOP session broker. See the
//! crate README for the API.

pub mod account_api;
pub mod accounts;
pub mod broker;
pub mod config;
pub mod cookies;
pub mod error;
pub mod fakes;
pub mod fcm;
pub mod firestore;
pub mod gmail;
pub mod google;
pub mod keys;
pub mod login;
pub mod requests;
pub mod session_api;
pub mod store;
pub mod vault;
pub mod verify;

use std::sync::Arc;

use std::time::Duration;

use std::time::Instant;

use axum::extract::{DefaultBodyLimit, MatchedPath, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use accounts::Accounts;
use broker::Broker;
use config::Config;
use fcm::Messenger;
use gmail::GmailRefresher;
use keys::Keys;
use login::VtopLogin;
use requests::{PhoneRequests, RateLimiter};
use store::Store;
use vault::Sealer;
use verify::SessionVerifier;

/// The swappable parts: real ones in `main`, in-memory ones in tests.
pub struct Parts {
    pub store: Arc<dyn Store>,
    pub messenger: Arc<dyn Messenger>,
    pub refresher: Arc<dyn GmailRefresher>,
    pub login: Arc<dyn VtopLogin>,
    pub verifier: Arc<dyn SessionVerifier>,
}

pub struct AppState {
    pub config: Config,
    pub store: Arc<dyn Store>,
    pub phone: Arc<PhoneRequests>,
    pub accounts: Arc<Accounts>,
    pub broker: Arc<Broker>,
    pub keys: Arc<Keys>,
    pub verifier: Arc<dyn SessionVerifier>,
    /// `POST /cookie` and the account API, per client IP.
    pub rate_limiter: RateLimiter,
    /// The session API, per account.
    pub account_limiter: RateLimiter,
}

impl AppState {
    pub fn new(config: Config, parts: Parts) -> Self {
        let phone = Arc::new(PhoneRequests {
            store: parts.store.clone(),
            messenger: parts.messenger,
            public_base_url: config.public_base_url.clone(),
            request_timeout: config.request_timeout,
        });
        let accounts = Arc::new(Accounts::new(
            parts.store.clone(),
            Arc::new(Sealer::new(config.vault_key)),
            config.session_cache,
        ));
        let broker = Arc::new(Broker::new(
            accounts.clone(),
            parts.store.clone(),
            phone.clone(),
            parts.refresher,
            parts.login,
            parts.verifier.clone(),
            config.gmail_wait,
        ));
        let keys = Arc::new(Keys::new(parts.store.clone()));
        Self {
            store: parts.store,
            phone,
            accounts,
            broker,
            keys,
            verifier: parts.verifier,
            rate_limiter: RateLimiter::new(10, Duration::from_secs(60)),
            account_limiter: RateLimiter::new(60, Duration::from_secs(60)),
            config,
        }
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    let cookie_start = post(requests::post_cookie).layer(middleware::from_fn_with_state(
        state.clone(),
        requests::rate_limit,
    ));
    let session_routes = Router::new()
        .route("/v1/whoami", get(session_api::whoami))
        .route("/v1/session", post(session_api::post_session))
        .route(
            "/v1/session/requests/{request_id}",
            get(session_api::get_request),
        )
        .route("/v1/session/expire", post(session_api::post_expire))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            session_api::require_key,
        ));
    let account_routes = Router::new()
        .route("/v1/link", post(account_api::link))
        .route("/v1/keys", post(account_api::create_key))
        .route("/v1/keys/revoke", post(account_api::revoke_key))
        .route("/v1/account", post(account_api::account))
        .route("/v1/account/fcm-token", post(account_api::update_fcm_token))
        .route(
            "/v1/account/forget-credentials",
            post(account_api::forget_credentials),
        )
        .route("/v1/account/delete", post(account_api::delete_account))
        .route("/v1/account/settings", post(account_api::update_settings))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            requests::rate_limit,
        ));
    Router::new()
        .merge(session_routes)
        .merge(account_routes)
        .route("/healthz", get(healthz))
        .route("/cookie", cookie_start)
        .route(
            "/cookie/status/{request_id}",
            get(requests::get_status).delete(requests::delete_status),
        )
        .route("/cookie/callback", post(requests::post_callback))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            requests::cors,
        ))
        .layer(middleware::from_fn(access_log))
        .with_state(state)
}

/// Request bodies are small JSON objects; refuse anything bigger.
const MAX_BODY_BYTES: usize = 64 * 1024;

/// Logs method, route, status and latency. Never headers or bodies, which
/// carry cookies, passwords and tokens.
async fn access_log(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|path| path.as_str().to_string())
        .unwrap_or_else(|| "-".into());
    let started = Instant::now();
    let response = next.run(request).await;
    tracing::info!(
        target: "vtop_bridge::access",
        "{method} {route} {} {}ms",
        response.status().as_u16(),
        started.elapsed().as_millis()
    );
    response
}

async fn healthz(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let healthy = matches!(
        tokio::time::timeout(Duration::from_secs(2), state.store.health()).await,
        Ok(Ok(()))
    );
    let (status, word) = if healthy {
        (StatusCode::OK, "ok")
    } else {
        tracing::warn!("health check failed: store unavailable");
        (StatusCode::SERVICE_UNAVAILABLE, "unavailable")
    };
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(serde_json::json!({ "status": word, "service": "vtop-bridge" })),
    )
}

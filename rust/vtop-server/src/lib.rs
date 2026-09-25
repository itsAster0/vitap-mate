//! HTTP front end for vtop-core. See the crate README for the API.

pub mod auth;
pub mod cache;
pub mod config;
pub mod error;
pub mod rate_limit;
pub mod routes;

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{DefaultBodyLimit, Request};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use tower_http::compression::CompressionLayer;

use auth::{ApiKeys, KeyId};
use cache::SessionCache;
use config::ServerConfig;
use rate_limit::RateLimiter;

/// Request bodies are small JSON objects; refuse anything bigger.
const MAX_BODY_BYTES: usize = 64 * 1024;

pub struct AppState {
    pub config: ServerConfig,
    pub api_keys: ApiKeys,
    pub rate_limiter: RateLimiter,
    pub cache: SessionCache,
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            api_keys: ApiKeys::new(&config.api_keys),
            rate_limiter: RateLimiter::new(config.rate_limit_per_minute),
            cache: SessionCache::new(config.session_cache_ttl, config.session_cache_max_entries),
            config,
        }
    }
}

/// Logs method, path, status, latency and key index. Never headers or
/// bodies, which carry cookies, passwords and OTPs.
async fn access_log(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let started = Instant::now();
    let response = next.run(request).await;
    let key = response
        .extensions()
        .get::<KeyId>()
        .map(|KeyId(index)| index.to_string())
        .unwrap_or_else(|| "-".into());
    tracing::info!(
        target: "vtop_server::access",
        "{method} {path} {} {}ms key={key}",
        response.status().as_u16(),
        started.elapsed().as_millis()
    );
    response
}

/// Copies the key id from the request onto the response so the access log
/// (which wraps the auth layer) can see it.
async fn tag_response_with_key(request: Request, next: Next) -> Response {
    let key = request.extensions().get::<KeyId>().copied();
    let mut response = next.run(request).await;
    if let Some(key) = key {
        response.extensions_mut().insert(key);
    }
    response
}

pub fn app(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/auth/login", post(routes::login))
        .route("/auth/otp", post(routes::submit_otp))
        .route("/auth/otp/resend", post(routes::resend_otp))
        .route("/semesters", post(routes::semesters))
        .route("/attendance", post(routes::attendance))
        .route("/attendance/full", post(routes::full_attendance))
        .route("/timetable", post(routes::timetable))
        .route("/marks", post(routes::marks))
        .route("/exam-schedule", post(routes::exam_schedule))
        .route("/grades", post(routes::grades))
        .route("/grades/details", post(routes::grade_details))
        .route("/grade-history", post(routes::grade_history))
        .route("/biometric", post(routes::biometric))
        .route("/refresh", post(routes::refresh))
        .layer(middleware::from_fn(tag_response_with_key))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_key,
        ));

    Router::new()
        .route("/health", get(routes::health))
        .nest("/v1", api)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        // Parsed pages are repetitive JSON; gzip shrinks them several-fold
        // for clients that ask (Accept-Encoding).
        .layer(CompressionLayer::new().gzip(true))
        .layer(middleware::from_fn(access_log))
        .with_state(state)
}

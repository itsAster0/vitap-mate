//! The engine: all VTOP work for the Cloudflare edge, nothing stored. It
//! checks whose a session is, seals saved credentials, signs in with them
//! (the OTP from forwarded mail) and serves the MCP tools. See `engine`.

pub mod config;
pub mod engine;
pub mod error;
pub mod fakes;
pub mod login;
pub mod vault;
pub mod verify;

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{DefaultBodyLimit, MatchedPath, Request};
use axum::http::header;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use config::Config;
use engine::EdgeLink;
use login::ForwardedLogin;
use vault::Sealer;
use verify::SessionVerifier;

/// The swappable parts: real ones in `main`, in-memory ones in tests.
pub struct Parts {
    pub forwarded_login: Arc<dyn ForwardedLogin>,
    pub verifier: Arc<dyn SessionVerifier>,
    pub edge: Arc<dyn EdgeLink>,
}

pub struct AppState {
    pub config: Config,
    pub verifier: Arc<dyn SessionVerifier>,
    pub forwarded_login: Arc<dyn ForwardedLogin>,
    pub edge: Arc<dyn EdgeLink>,
    sealer: Sealer,
}

impl AppState {
    pub fn new(config: Config, parts: Parts) -> Self {
        Self {
            sealer: Sealer::new(config.vault_key),
            verifier: parts.verifier,
            forwarded_login: parts.forwarded_login,
            edge: parts.edge,
            config,
        }
    }

    /// Seals account fields with `VAULT_KEY`.
    pub fn accounts_sealer(&self) -> &Sealer {
        &self.sealer
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/internal/mail-otp/owner", post(engine::owner_check))
        .route(vtop_shared::engine::SESSION, post(engine::session))
        .route(vtop_shared::engine::SEAL, post(engine::seal))
        .route(vtop_shared::engine::LOGIN, post(engine::login))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
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

/// Nothing is stored here, so up is healthy.
async fn healthz() -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(serde_json::json!({ "status": "ok", "service": "vtop-engine" })),
    )
}

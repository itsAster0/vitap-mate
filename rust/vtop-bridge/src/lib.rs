//! The engine: all VTOP work for the Cloudflare edge, nothing stored. It
//! checks whose a session is, seals saved credentials, signs in with them
//! (the OTP from forwarded mail, or from Gmail), fetches pages on a phone's session and
//! serves the MCP tools. See `engine` and `fetch`.

pub mod config;
pub mod engine;
pub mod error;
pub mod fakes;
pub mod fetch;
pub mod gmail;
pub mod login;
pub mod page_pack;
pub mod pages;
pub mod proof;
pub mod vault;
pub mod verify;
pub mod vtop_login;

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{DefaultBodyLimit, MatchedPath, Request};
use axum::http::header;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tower_http::compression::CompressionLayer;
use tower_http::compression::predicate::{DefaultPredicate, NotForContentType, Predicate};
use tracing::Instrument;

use config::Config;
use engine::EdgeLink;
use gmail::GmailRefresher;
use login::ForwardedLogin;
use vault::Sealer;
use verify::SessionVerifier;

/// The swappable parts: real ones in `main`, in-memory ones in tests.
pub struct Parts {
    pub forwarded_login: Arc<dyn ForwardedLogin>,
    pub verifier: Arc<dyn SessionVerifier>,
    pub edge: Arc<dyn EdgeLink>,
    pub gmail: Arc<dyn GmailRefresher>,
}

pub struct AppState {
    pub config: Config,
    pub verifier: Arc<dyn SessionVerifier>,
    pub forwarded_login: Arc<dyn ForwardedLogin>,
    pub edge: Arc<dyn EdgeLink>,
    /// Turns saved Gmail access into access tokens.
    pub gmail: Arc<dyn GmailRefresher>,
    /// The VTOP that phone sign-ins and fetches go to.
    pub vtop: vtop_core::VtopConfig,
    /// The server account proofs are held to and dialed.
    pub proof: proof::ProofTarget,
    /// Slots for proofs running at once.
    pub proofs: Arc<tokio::sync::Semaphore>,
    /// Proofs started per client address.
    pub proof_gate: proof::ProofGate,
    sealer: Sealer,
}

impl AppState {
    pub fn new(config: Config, parts: Parts) -> Self {
        Self {
            sealer: Sealer::new(config.vault_key),
            verifier: parts.verifier,
            forwarded_login: parts.forwarded_login,
            edge: parts.edge,
            gmail: parts.gmail,
            vtop: vtop_core::VtopConfig::default(),
            proof: proof::ProofTarget::vtop(),
            proofs: Arc::new(tokio::sync::Semaphore::new(proof::MAX_PROOFS)),
            proof_gate: proof::ProofGate::default(),
            config,
        }
    }

    /// Sends phone sign-ins and fetches to another VTOP (a mock, in tests).
    pub fn with_vtop(mut self, vtop: vtop_core::VtopConfig) -> Self {
        self.vtop = vtop;
        self
    }

    /// Holds account proofs to another server (a fake VTOP, in tests).
    pub fn with_proof(mut self, target: proof::ProofTarget) -> Self {
        self.proof = target;
        self
    }

    /// Seals account fields with `VAULT_KEY`.
    pub fn accounts_sealer(&self) -> &Sealer {
        &self.sealer
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route(vtop_shared::engine::MAIL_OWNER, post(engine::owner_check))
        .route(vtop_shared::engine::SESSION, post(engine::session))
        .route(vtop_shared::engine::SEAL, post(engine::seal))
        .route(vtop_shared::engine::LOGIN, post(engine::login))
        .route(vtop_shared::engine::PROOF_VERIFY, get(proof::verify))
        .route(proof::PROOF_PUBLIC, get(proof::public))
        .route(vtop_shared::engine::FETCH, post(fetch::fetch))
        .route(vtop_shared::engine::FETCH_BATCH, post(fetch::fetch_batch))
        .route(vtop_shared::engine::VTOP_LOGIN, post(vtop_login::login))
        .route(vtop_shared::engine::VTOP_LOGIN_OTP, post(vtop_login::otp))
        .route(
            vtop_shared::engine::VTOP_LOGIN_RESEND,
            post(vtop_login::resend),
        )
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        // Parsed pages are JSON; Brotli (gzip for callers without it) makes
        // the hop to the edge small. Packed pages are Brotli already.
        .layer(
            CompressionLayer::new().compress_when(DefaultPredicate::new().and(
                NotForContentType::const_new(vtop_shared::page_dictionary::CONTENT_TYPE),
            )),
        )
        .layer(middleware::from_fn(access_log))
        .with_state(state)
}

/// Request bodies are small JSON objects; refuse anything bigger.
const MAX_BODY_BYTES: usize = 64 * 1024;

/// Logs method, route, status and latency, inside a `request` span that
/// carries the caller's `flow` and `regno` (Convex sends them as
/// `x-vtop-flow` and `x-vtop-regno`), so every line the handler writes can
/// be matched to the phone's own log. Never headers or bodies, which carry
/// cookies, passwords and tokens.
async fn access_log(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|path| path.as_str().to_string())
        .unwrap_or_else(|| "-".into());
    // How the phone's request came: the UDP gateway marks its calls; the
    // rest come from the edge over HTTPS. The phone's flow id, if sent.
    let [via, request_id] = [("x-via", "https"), ("x-request-id", "-")].map(|(name, none)| {
        request
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or(none)
            .to_owned()
    });
    let [flow, regno] = [(FLOW_HEADER, MAX_FLOW_LEN), (REGNO_HEADER, MAX_REGNO_LEN)]
        .map(|(name, max)| trace_value(request.headers().get(name), max));
    let span = tracing::info_span!(
        "request",
        flow = tracing::field::Empty,
        regno = tracing::field::Empty
    );
    if let Some(flow) = &flow {
        span.record("flow", tracing::field::display(flow));
    }
    if let Some(regno) = &regno {
        span.record("regno", tracing::field::display(regno));
    }
    let started = Instant::now();
    let response = next.run(request).instrument(span.clone()).await;
    span.in_scope(|| {
        tracing::info!(
            target: "vtop_bridge::access",
            "{method} {route} {} {}ms via={via} request_id={request_id}",
            response.status().as_u16(),
            started.elapsed().as_millis()
        );
    });
    response
}

/// The flow id and registration number Convex puts on its engine calls.
pub const FLOW_HEADER: &str = "x-vtop-flow";
pub const REGNO_HEADER: &str = "x-vtop-regno";
const MAX_FLOW_LEN: usize = 64;
const MAX_REGNO_LEN: usize = 20;

/// A trace header's value, if it is short and only letters, digits and
/// `-_.:`. Anything else is dropped rather than logged: the header is the
/// caller's to set, and a log line must not carry what it was not meant to.
pub fn trace_value(value: Option<&axum::http::HeaderValue>, max: usize) -> Option<String> {
    let text = value?.to_str().ok()?;
    let fits = !text.is_empty()
        && text.len() <= max
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'));
    fits.then(|| text.to_owned())
}

/// Nothing is stored here, so up is healthy.
async fn healthz() -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(serde_json::json!({ "status": "ok", "service": "vtop-engine" })),
    )
}

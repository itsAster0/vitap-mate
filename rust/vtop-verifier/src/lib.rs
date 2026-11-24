//! The verifier: checks a phone's TLSNotary proof of its regno (`GET /proof?ticket=`, see
//! `proof`) and reports the verdict to Convex. Stateless: tickets, verdicts and owner hashes
//! are Convex's, and it holds only the proofs in flight and `VERIFIER_TOKEN`, which Convex
//! takes on its two proof routes only. See `ARCHITECTURE.md`, "Account proof".

pub mod config;
pub mod convex;
pub mod proof;
pub mod queue;

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;

use config::Config;
use convex::ConvexLink;
use proof::{ProofGate, ProofTarget};
use queue::Queue;

pub struct AppState {
    pub convex: Arc<dyn ConvexLink>,
    /// The server proofs are held to and dialed.
    pub proof: ProofTarget,
    pub queue: Queue,
    /// Proofs started per client address.
    pub gate: ProofGate,
    pub proxy_hops: usize,
}

impl AppState {
    pub fn new(config: &Config, convex: Arc<dyn ConvexLink>) -> Self {
        Self {
            convex,
            proof: ProofTarget::vtop(),
            queue: Queue::new(config.max_proofs, config.max_queue, config.queue_wait),
            gate: ProofGate::default(),
            proxy_hops: config.proxy_hops,
        }
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route(proof::PROOF_PUBLIC, get(proof::public))
        .with_state(state)
}

/// Nothing is stored here, so up is healthy.
async fn healthz() -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(serde_json::json!({ "status": "ok", "service": "vtop-verifier" })),
    )
}

#[cfg(test)]
mod tests;

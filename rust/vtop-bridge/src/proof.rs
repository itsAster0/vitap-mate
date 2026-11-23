//! `GET /engine/proof` ([`PROOF_VERIFY`](vtop_shared::engine::PROOF_VERIFY)): the verifier side of a phone's TLSNotary proof of
//! its regno (vtop-tlsn), for the account token. The edge opens this WebSocket with
//! `INTERNAL_TOKEN` once it has checked and consumed the device's challenge, and relays the
//! phone's binary messages to it. The statement is built from the edge's `device` and
//! `challenge` before a byte of the phone's is read; the engine dials VTOP itself and never sees
//! the session. Its last message is one text frame, a [`ProofVerdict`], which the edge keeps
//! from the phone and mints the token from. A few proofs run at once ([`MAX_PROOFS`]); more
//! are turned away with 503 before the upgrade.

use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::rejection::QueryRejection;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use futures::{SinkExt, StreamExt, channel::mpsc, future::join};
use serde::Deserialize;
use tokio_util::compat::TokioAsyncReadCompatExt;
use vtop_shared::account::account_id;
use vtop_shared::engine::ProofVerdict;
use vtop_shared::mailbox::owner_hash;
use vtop_tlsn::RootCertStore;
use vtop_tlsn::content::{Content, VTOP_HOST, binding};
use vtop_tlsn::session::{self, Pipe};

use crate::AppState;
use crate::engine::require_secret;
use crate::error::ApiError;

/// Proofs at once: each holds a TLS relay to VTOP and a few seconds of ZK work.
pub const MAX_PROOFS: usize = 4;
/// The reason in a failed verdict, at most this long.
const MAX_REASON: usize = 200;
/// Longest wait for VTOP to accept the TCP connection.
const DIAL_TIMEOUT: Duration = Duration::from_secs(10);
/// Longest silence from the phone. A prover that connects and sends nothing gives up its
/// slot after this, not after the whole proof's timeout.
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
/// Largest WebSocket message the engine buffers; tlsn sends small ones, and Cloudflare caps
/// a relayed message at 1 MiB anyway.
const MAX_MESSAGE: usize = 1 << 20;

/// The server a proof is held to and dialed: VTOP, or a fake one in tests.
#[derive(Clone)]
pub struct ProofTarget {
    /// The server name the proof must carry.
    pub host: String,
    /// What the engine dials (`host:443`).
    pub addr: String,
    pub roots: RootCertStore,
}

impl ProofTarget {
    pub fn vtop() -> Self {
        Self {
            host: VTOP_HOST.into(),
            addr: format!("{VTOP_HOST}:443"),
            // The bundled roots are a constant: a bad one fails every test.
            roots: session::vtop_roots().unwrap_or_else(|_| RootCertStore::mozilla()),
        }
    }
}

#[derive(Deserialize)]
pub struct ProofQuery {
    device: String,
    challenge: String,
}

fn hex32(s: &str) -> bool {
    s.len() == 32
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub async fn verify(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<ProofQuery>, QueryRejection>,
    upgrade: Result<WebSocketUpgrade, axum::extract::ws::rejection::WebSocketUpgradeRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let bad = || {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "device and challenge are 32 lower-case hex digits.",
        )
    };
    let Query(query) = query.map_err(|_| bad())?;
    if !hex32(&query.device) || !hex32(&query.challenge) {
        return Err(bad());
    }
    let upgrade = upgrade.map_err(|_| {
        ApiError::new(
            StatusCode::UPGRADE_REQUIRED,
            "upgrade_required",
            "A WebSocket is required.",
        )
    })?;
    let Ok(slot) = state.proofs.clone().try_acquire_owned() else {
        tracing::info!("engine: proof turned away, {MAX_PROOFS} running");
        return Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "proof_busy",
            "Too many proofs at once; try again shortly.",
        ));
    };
    let target = state.proof.clone();
    let content = Content {
        host: target.host.clone(),
        binding: binding(&query.challenge, &query.device),
    };
    let stmt = content.statement().map_err(|error| {
        tracing::warn!("engine: proof statement: {error:#}");
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "No statement.",
        )
    })?;
    let upgrade = upgrade
        .max_message_size(MAX_MESSAGE)
        .max_frame_size(MAX_MESSAGE);
    Ok(upgrade.on_upgrade(move |socket| async move {
        run(&state, socket, &stmt, target).await;
        drop(slot);
    }))
}

/// Verifies over `socket`, then sends the verdict as its last message.
async fn run(
    state: &AppState,
    socket: WebSocket,
    stmt: &vtop_tlsn::statement::Statement,
    target: ProofTarget,
) {
    let started = Instant::now();
    let (mut ws_tx, ws_rx) = socket.split();
    // Only non-empty binary messages carry the session (tlsn never sends an empty one);
    // anything else from the edge, or silence past IDLE_TIMEOUT, ends the stream.
    let rx = futures::stream::unfold(ws_rx, |mut ws_rx| async move {
        loop {
            match tokio::time::timeout(IDLE_TIMEOUT, ws_rx.next()).await {
                Ok(Some(Ok(Message::Binary(bytes)))) if !bytes.is_empty() => {
                    return Some((Ok(bytes.to_vec()), ws_rx));
                }
                Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => {}
                _ => return None,
            }
        }
    });
    let (tx, out) = mpsc::channel::<Vec<u8>>(8);
    let pipe = Pipe::new(Box::pin(rx), tx.sink_map_err(io::Error::other));
    let dial = async {
        let connect = tokio::net::TcpStream::connect(&target.addr);
        let stream = tokio::time::timeout(DIAL_TIMEOUT, connect)
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "VTOP did not answer"))??;
        stream.set_nodelay(true)?;
        Ok(stream.compat())
    };
    let verified = session::verify(pipe, stmt, target.roots, dial, tokio::time::sleep);
    // The pipe's sender goes with the session, which ends this loop; `ws_tx` then sends the verdict.
    let forward = async {
        // Owned here: when sending fails this drops it, so the session's writes fail at once
        // instead of blocking on a full channel until the timeout.
        let mut out = out;
        while let Some(message) = out.next().await {
            if ws_tx.send(Message::Binary(message.into())).await.is_err() {
                break;
            }
        }
    };
    let (verified, ()) = join(verified, forward).await;
    let ms = started.elapsed().as_millis();
    let verdict = match verified.map(|proven| proven.page.get("regno").cloned()) {
        Ok(Some(regno)) => {
            let regno = account_id(&regno);
            tracing::info!("engine: proof verified in {ms} ms");
            ProofVerdict::Ok {
                owner_hash: owner_hash(&state.config.vault_key, &regno),
                registration_number: regno,
            }
        }
        Ok(None) => failed(ms, "no registration number in the proof".into()),
        Err(error) => failed(ms, format!("{error:#}")),
    };
    let text = serde_json::to_string(&verdict).unwrap_or_default();
    #[expect(
        clippy::let_underscore_must_use,
        reason = "the edge may have gone; it then mints nothing"
    )]
    let _ = ws_tx.send(Message::Text(text.into())).await;
    #[expect(
        clippy::let_underscore_must_use,
        reason = "the verdict went, or nobody is there"
    )]
    let _ = ws_tx.close().await;
}

fn failed(ms: u128, mut reason: String) -> ProofVerdict {
    // The reason names a step and a status line, never the session.
    reason.truncate(reason.floor_char_boundary(MAX_REASON));
    tracing::info!("engine: no proof after {ms} ms: {reason}");
    ProofVerdict::Failed { reason }
}

#[cfg(test)]
mod tests;

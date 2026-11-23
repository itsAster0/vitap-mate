//! The verifier side of a phone's TLSNotary proof of its regno (vtop-tlsn), on two routes.
//!
//! - `GET /engine/proof` ([`PROOF_VERIFY`](vtop_shared::engine::PROOF_VERIFY)): the v2 edge
//!   opens this WebSocket with `INTERNAL_TOKEN` once it has checked and consumed the device's
//!   challenge, and relays the phone's binary messages to it. Its last message is one text
//!   frame, a [`ProofVerdict`], which the edge keeps from the phone and mints the token from.
//! - `GET /proof?ticket=` ([`PROOF_PUBLIC`]): the phone opens it itself (Convex can't relay a
//!   socket). The engine uses the ticket up at Convex's [`PROOF_TICKET`], which names the
//!   device, verifies as above with the ticket as the challenge, and posts the verdict to
//!   [`PROOF_DONE`]. The phone's last message is `{ok: true, registrationNumber}` or
//!   `{code, error}`; Convex issues the token from the ticket.
//!
//! The statement is built from the device and challenge before a byte of the phone's is read;
//! the engine dials VTOP itself and never sees the session. A few proofs run at once
//! ([`MAX_PROOFS`]); more are turned away with 503 before the upgrade, so no ticket is spent.
//! So are an address's proofs past [`PER_ADDRESS`] a minute ([`ProofGate`]): Convex caps proofs
//! for everyone, and fresh device keys from one machine must not use that cap up.

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::extract::rejection::QueryRejection;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt, channel::mpsc, future::join};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio_util::compat::TokioAsyncReadCompatExt;
use tracing::Instrument;
use vtop_shared::account::account_id;
use vtop_shared::engine::ProofVerdict;
use vtop_shared::mailbox::owner_hash;
use vtop_tlsn::RootCertStore;
use vtop_tlsn::content::{Content, VTOP_HOST, binding};
use vtop_tlsn::session::{self, Pipe};

use crate::AppState;
use crate::engine::{EdgeError, require_secret};
use crate::error::ApiError;

/// Engine: the phone's own proof socket, `GET ?ticket=`, with no token.
pub const PROOF_PUBLIC: &str = "/proof";
/// Convex: uses up a ticket, `{ticket}` → `{device}`, or 404 when it is unknown, used or expired.
pub const PROOF_TICKET: &str = "/internal/proof-ticket";
/// Convex: the verdict on a ticket's proof ([`ProofDone`]).
pub const PROOF_DONE: &str = "/internal/proof-done";

/// The verdict sent to [`PROOF_DONE`]: `{ticket, regno, ownerHash}`, or `{ticket, code}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofDone {
    pub ticket: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regno: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

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
    let upgrade = websocket(upgrade)?;
    let slot = take_slot(&state)?;
    let stmt = statement(&state, &query.challenge, &query.device).map_err(|()| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "No statement.",
        )
    })?;
    // The socket runs after the response, outside the request's span; it keeps it.
    let span = tracing::Span::current();
    Ok(sized(upgrade).on_upgrade(move |socket| {
        async move {
            let (verdict, mut ws_tx) = run(&state, socket, &stmt).await;
            finish(&mut ws_tx, &verdict).await;
            drop(slot);
        }
        .instrument(span)
    }))
}

#[derive(Deserialize)]
pub struct TicketQuery {
    ticket: String,
}

/// Tickets are Convex's random ids; this only keeps junk out of the binding and the log.
fn ticket_ok(s: &str) -> bool {
    (16..=128).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// `GET /proof?ticket=`: the phone's proof, with the ticket from `auth:proveStart`.
pub async fn public(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<TicketQuery>, QueryRejection>,
    upgrade: Result<WebSocketUpgrade, axum::extract::ws::rejection::WebSocketUpgradeRejection>,
) -> Result<Response, ApiError> {
    let Ok(Query(TicketQuery { ticket })) = query else {
        return Err(bad_ticket());
    };
    if !ticket_ok(&ticket) {
        return Err(bad_ticket());
    }
    let upgrade = websocket(upgrade)?;
    if !state.proof_gate.allow(
        client_address(&headers, state.config.proxy_hops),
        Instant::now(),
    ) {
        tracing::info!("engine: proof turned away, {PER_ADDRESS} a minute from this address");
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "too_many_proofs",
            "Too many proofs from this network; try again in a minute.",
        ));
    }
    let slot = take_slot(&state)?;
    let span = tracing::Span::current();
    Ok(sized(upgrade).on_upgrade(move |socket| {
        async move {
            prove_ticket(&state, socket, &ticket).await;
            drop(slot);
        }
        .instrument(span)
    }))
}

fn bad_ticket() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "bad_request",
        "ticket is the one auth:proveStart gave.",
    )
}

/// Uses up `ticket`, verifies the proof for its device, records the verdict at Convex and
/// tells the phone.
async fn prove_ticket(state: &AppState, mut socket: WebSocket, ticket: &str) {
    let device = match state.edge.proof_ticket(ticket).await {
        Ok(Some(device)) if hex32(&device) => device,
        Ok(Some(_)) => {
            tracing::warn!("engine: proof ticket names no device id");
            return reply(&mut socket, &unavailable()).await;
        }
        Ok(None) => {
            tracing::info!("engine: proof ticket unknown, used or expired");
            let answer = error_body(
                "challenge_invalid",
                "Unknown, used or expired ticket; start the proof again.",
            );
            return reply(&mut socket, &answer).await;
        }
        Err(EdgeError::Busy) => {
            tracing::info!("engine: proof ticket: Convex is at its proof cap");
            let answer = error_body(
                "too_many_proofs",
                "The server is proving many accounts right now; try again in a minute.",
            );
            return reply(&mut socket, &answer).await;
        }
        Err(EdgeError::Unavailable(_)) => {
            tracing::warn!("engine: proof ticket: Convex unreachable");
            return reply(&mut socket, &unavailable()).await;
        }
    };
    let Ok(stmt) = statement(state, ticket, &device) else {
        return reply(&mut socket, &unavailable()).await;
    };
    let (verdict, mut ws_tx) = run(state, socket, &stmt).await;
    let answer = record(state, ticket, verdict).await;
    #[expect(
        clippy::let_underscore_must_use,
        reason = "the phone may have gone; Convex has the verdict either way"
    )]
    let _ = ws_tx.send(Message::Text(answer.to_string().into())).await;
    #[expect(
        clippy::let_underscore_must_use,
        reason = "the answer went, or nobody is there"
    )]
    let _ = ws_tx.close().await;
}

/// Posts `verdict` to Convex; the phone's last message.
async fn record(state: &AppState, ticket: &str, verdict: ProofVerdict) -> Value {
    let (done, answer) = match verdict {
        ProofVerdict::Ok {
            registration_number,
            owner_hash,
        } => (
            ProofDone {
                ticket: ticket.into(),
                regno: Some(registration_number.clone()),
                owner_hash: Some(owner_hash),
                code: None,
            },
            json!({ "ok": true, "registrationNumber": registration_number }),
        ),
        ProofVerdict::Failed { reason } => (
            ProofDone {
                ticket: ticket.into(),
                regno: None,
                owner_hash: None,
                code: Some("proof_failed".into()),
            },
            error_body(
                "proof_failed",
                &format!("VTOP's page did not prove who you are: {reason}"),
            ),
        ),
    };
    match state.edge.proof_done(&done).await {
        Ok(()) => answer,
        Err(error) => {
            tracing::warn!("engine: proof verdict not recorded: {error:?}");
            unavailable()
        }
    }
}

fn error_body(code: &str, error: &str) -> Value {
    json!({ "code": code, "error": error })
}

fn unavailable() -> Value {
    error_body(
        "server_unavailable",
        "The server cannot check proofs right now; try again.",
    )
}

/// Sends `answer` on a socket that carried no proof, then closes it.
async fn reply(socket: &mut WebSocket, answer: &Value) {
    #[expect(
        clippy::let_underscore_must_use,
        reason = "the phone may have gone; there is nothing more to do"
    )]
    let _ = socket.send(Message::Text(answer.to_string().into())).await;
    #[expect(
        clippy::let_underscore_must_use,
        reason = "the answer went, or nobody is there"
    )]
    let _ = socket.close().await;
}

fn websocket(
    upgrade: Result<WebSocketUpgrade, axum::extract::ws::rejection::WebSocketUpgradeRejection>,
) -> Result<WebSocketUpgrade, ApiError> {
    upgrade.map_err(|_| {
        ApiError::new(
            StatusCode::UPGRADE_REQUIRED,
            "upgrade_required",
            "A WebSocket is required.",
        )
    })
}

fn take_slot(state: &Arc<AppState>) -> Result<tokio::sync::OwnedSemaphorePermit, ApiError> {
    state.proofs.clone().try_acquire_owned().map_err(|_| {
        tracing::info!("engine: proof turned away, {MAX_PROOFS} running");
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "proof_busy",
            "Too many proofs at once; try again shortly.",
        )
    })
}

/// Proofs one address may start a minute. Generous: students on the campus network share one.
pub const PER_ADDRESS: u32 = 20;

/// Proofs started per client address in the current minute.
#[derive(Default)]
pub struct ProofGate(Mutex<HashMap<String, (Instant, u32)>>);

impl ProofGate {
    /// Counts a proof from `address` at `now`; false past [`PER_ADDRESS`] in its minute.
    pub fn allow(&self, address: String, now: Instant) -> bool {
        const MINUTE: Duration = Duration::from_secs(60);
        let mut seen = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if seen.len() > 10_000 {
            seen.retain(|_, (since, _)| now.duration_since(*since) < MINUTE);
        }
        let (since, count) = seen.entry(address).or_insert((now, 0));
        if now.duration_since(*since) >= MINUTE {
            (*since, *count) = (now, 0);
        }
        *count += 1;
        *count <= PER_ADDRESS
    }
}

/// The phone's address: the `X-Forwarded-For` entry `hops` from the end, written by the first
/// proxy we run (`Config::proxy_hops`); earlier ones came from the phone and can say anything.
/// Fewer entries: the first there is.
fn client_address(headers: &HeaderMap, hops: usize) -> String {
    let entries: Vec<&str> = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    let at = entries.len().saturating_sub(hops);
    entries.get(at).copied().unwrap_or("direct").to_owned()
}

fn sized(upgrade: WebSocketUpgrade) -> WebSocketUpgrade {
    upgrade
        .max_message_size(MAX_MESSAGE)
        .max_frame_size(MAX_MESSAGE)
}

/// The statement a proof for `device` under `challenge` must meet.
fn statement(
    state: &AppState,
    challenge: &str,
    device: &str,
) -> Result<vtop_tlsn::statement::Statement, ()> {
    let content = Content {
        host: state.proof.host.clone(),
        binding: binding(challenge, device),
    };
    content.statement().map_err(|error| {
        tracing::warn!("engine: proof statement: {error:#}");
    })
}

/// Sends the verdict as the socket's last message, for the edge.
async fn finish(ws_tx: &mut SplitSink<WebSocket, Message>, verdict: &ProofVerdict) {
    let text = serde_json::to_string(verdict).unwrap_or_default();
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

/// Verifies over `socket`; the verdict, and the socket's sending half for the last message.
async fn run(
    state: &AppState,
    socket: WebSocket,
    stmt: &vtop_tlsn::statement::Statement,
) -> (ProofVerdict, SplitSink<WebSocket, Message>) {
    let target = state.proof.clone();
    let started = Instant::now();
    let (mut ws_tx, ws_rx) = socket.split();
    // Only non-empty binary messages carry the session (tlsn never sends an empty one);
    // anything else from the other end, or silence past IDLE_TIMEOUT, ends the stream.
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
            // The request span had no regno yet: the proof is what tells it.
            tracing::Span::current().record("regno", tracing::field::display(&regno));
            tracing::info!("engine: proof verified in {ms} ms");
            ProofVerdict::Ok {
                owner_hash: owner_hash(&state.config.vault_key, &regno),
                registration_number: regno,
            }
        }
        Ok(None) => failed(ms, "no registration number in the proof".into()),
        Err(error) => failed(ms, format!("{error:#}")),
    };
    (verdict, ws_tx)
}

fn failed(ms: u128, mut reason: String) -> ProofVerdict {
    // The reason names a step and a status line, never the session.
    reason.truncate(reason.floor_char_boundary(MAX_REASON));
    tracing::info!("engine: no proof after {ms} ms: {reason}");
    ProofVerdict::Failed { reason }
}

#[cfg(test)]
mod tests;

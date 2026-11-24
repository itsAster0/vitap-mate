//! `GET /proof?ticket=`: the phone's TLSNotary proof of its regno (vtop-tlsn), on a WebSocket
//! the phone opens itself (Convex can't relay a socket).
//!
//! The socket waits its turn for a slot ([`Queue`](crate::queue::Queue)), then uses the ticket
//! up at Convex's [`PROOF_TICKET`](crate::convex::PROOF_TICKET), which names the device, and verifies with the ticket as the
//! challenge. The statement is built from the device and ticket before a byte of the phone's is
//! read; the verifier dials VTOP itself (while Convex answers and tlsn sets up) and never sees
//! the session. The verdict goes to [`PROOF_DONE`](crate::convex::PROOF_DONE); the phone's last message is
//! `{ok: true, registrationNumber}` or `{code, error}`.
//!
//! Proofs past [`PER_ADDRESS`] a minute from one address are turned away ([`ProofGate`]):
//! Convex caps proofs for everyone, and fresh device keys from one machine must not use that
//! cap up.

use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::ws::rejection::WebSocketUpgradeRejection;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt, channel::mpsc, future::join};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncReadCompatExt;
use tokio_util::task::AbortOnDropHandle;
use tracing::Instrument;
use vtop_shared::account::account_id;
use vtop_tlsn::RootCertStore;
use vtop_tlsn::content::{Content, VTOP_HOST, binding};
use vtop_tlsn::session::{self, Pipe, phase};

use crate::AppState;
use crate::convex::{ConvexError, ProofDone};
use crate::queue::NoSlot;

pub const PROOF_PUBLIC: &str = "/proof";
/// The reason in a failed verdict, at most this long.
const MAX_REASON: usize = 200;
/// Longest wait for VTOP to accept the TCP connection.
const DIAL_TIMEOUT: Duration = Duration::from_secs(10);
/// Longest silence from the phone. A prover that connects and sends nothing gives up its
/// slot after this, not after the whole proof's timeout.
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
/// Largest WebSocket message the verifier takes; tlsn sends small ones.
const MAX_MESSAGE: usize = 1 << 20;

/// The server a proof is held to and dialed: VTOP, or a fake one in tests.
#[derive(Clone)]
pub struct ProofTarget {
    /// The server name the proof must carry.
    pub host: String,
    /// What the verifier dials (`host:443`).
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
pub struct TicketQuery {
    ticket: String,
}

fn hex32(s: &str) -> bool {
    s.len() == 32
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Tickets are Convex's random ids; this only keeps junk out of the binding and the log.
fn ticket_ok(s: &str) -> bool {
    (16..=128).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn refuse(status: StatusCode, code: &str, error: &str) -> Response {
    (status, Json(error_body(code, error))).into_response()
}

const BUSY: &str = "Too many proofs at once; try again shortly.";

/// `GET /proof?ticket=`: the phone's proof, with the ticket from `auth:proveStart`.
pub async fn public(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<TicketQuery>, QueryRejection>,
    upgrade: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
) -> Response {
    let bad_ticket = || {
        refuse(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "ticket is the one auth:proveStart gave.",
        )
    };
    let Ok(Query(TicketQuery { ticket })) = query else {
        return bad_ticket();
    };
    if !ticket_ok(&ticket) {
        return bad_ticket();
    }
    let Ok(upgrade) = upgrade else {
        return refuse(
            StatusCode::UPGRADE_REQUIRED,
            "upgrade_required",
            "A WebSocket is required.",
        );
    };
    if !state
        .gate
        .allow(client_address(&headers, state.proxy_hops), Instant::now())
    {
        tracing::info!("proof: turned away, {PER_ADDRESS} a minute from this address");
        return refuse(
            StatusCode::TOO_MANY_REQUESTS,
            "too_many_proofs",
            "Too many proofs from this network; try again in a minute.",
        );
    }
    if !state.queue.has_room() {
        tracing::info!(
            waiting = state.queue.waiting(),
            "proof: turned away, the line is full"
        );
        return refuse(StatusCode::SERVICE_UNAVAILABLE, "proof_busy", BUSY);
    }
    // The proof's own span: the socket runs after the response. The proof names the regno.
    let span = tracing::info_span!("proof", regno = tracing::field::Empty);
    upgrade
        .max_message_size(MAX_MESSAGE)
        .max_frame_size(MAX_MESSAGE)
        .on_upgrade(move |socket| prove_ticket(state, socket, ticket).instrument(span))
}

/// Waits for a slot, uses up `ticket`, verifies the proof for its device, records the verdict
/// at Convex and tells the phone.
async fn prove_ticket(state: Arc<AppState>, socket: WebSocket, ticket: String) {
    let (mut ws_tx, ws_rx) = socket.split();
    let slot = phase(
        "queue_wait",
        state
            .queue
            .enter(&mut ws_tx, || Message::Ping(Default::default())),
    )
    .await;
    let _slot = match slot {
        Ok(slot) => slot,
        Err(NoSlot::Gone) => return,
        Err(NoSlot::Full | NoSlot::TimedOut) => {
            return reply(ws_tx, &error_body("proof_busy", BUSY)).await;
        }
    };
    // VTOP's TCP connection opens beside Convex's answer and tlsn's setup.
    let dial = AbortOnDropHandle::new(tokio::spawn(
        connect(state.proof.addr.clone()).in_current_span(),
    ));
    let device = match phase("ticket", state.convex.proof_ticket(&ticket)).await {
        Ok(Some(device)) if hex32(&device) => device,
        Ok(Some(_)) => {
            tracing::warn!("proof: ticket names no device id");
            return reply(ws_tx, &unavailable()).await;
        }
        Ok(None) => {
            tracing::info!("proof: ticket unknown, used or expired");
            let answer = error_body(
                "challenge_invalid",
                "Unknown, used or expired ticket; start the proof again.",
            );
            return reply(ws_tx, &answer).await;
        }
        Err(ConvexError::Busy) => {
            tracing::info!("proof: ticket: Convex is at its proof cap");
            let answer = error_body(
                "too_many_proofs",
                "The server is proving many accounts right now; try again in a minute.",
            );
            return reply(ws_tx, &answer).await;
        }
        Err(ConvexError::Unavailable(error)) => {
            tracing::warn!("proof: ticket: Convex unreachable: {error}");
            return reply(ws_tx, &unavailable()).await;
        }
    };
    let content = Content {
        host: state.proof.host.clone(),
        binding: binding(&ticket, &device),
    };
    let stmt = match content.statement() {
        Ok(stmt) => stmt,
        Err(error) => {
            tracing::warn!("proof: statement: {error:#}");
            return reply(ws_tx, &unavailable()).await;
        }
    };
    let dial = async {
        let stream = dial.await.map_err(io::Error::other)??;
        Ok(stream.compat())
    };
    let (verdict, ws_tx) = run(&state, ws_tx, ws_rx, &stmt, dial).await;
    let answer = phase("record", record(&state, &ticket, verdict)).await;
    reply(ws_tx, &answer).await;
}

/// VTOP's TCP connection, for the TLS relay.
async fn connect(addr: String) -> io::Result<TcpStream> {
    phase("connect", async {
        let stream = tokio::time::timeout(DIAL_TIMEOUT, TcpStream::connect(&addr))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "VTOP did not answer"))??;
        stream.set_nodelay(true)?;
        Ok(stream)
    })
    .await
}

/// How a proof ended: the regno it proved, or why not (never the session: the verifier never
/// sees it).
enum Verdict {
    Proved(String),
    Failed(String),
}

/// Posts `verdict` to Convex; the phone's last message.
async fn record(state: &AppState, ticket: &str, verdict: Verdict) -> Value {
    let (done, answer) = match verdict {
        Verdict::Proved(regno) => (
            ProofDone {
                ticket: ticket.into(),
                regno: Some(regno.clone()),
                code: None,
            },
            json!({ "ok": true, "registrationNumber": regno }),
        ),
        Verdict::Failed(reason) => (
            ProofDone {
                ticket: ticket.into(),
                regno: None,
                code: Some("proof_failed".into()),
            },
            error_body(
                "proof_failed",
                &format!("VTOP's page did not prove who you are: {reason}"),
            ),
        ),
    };
    match state.convex.proof_done(&done).await {
        Ok(()) => answer,
        Err(error) => {
            tracing::warn!("proof: verdict not recorded: {error:?}");
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

/// Sends `answer` as the socket's last message, then closes it.
async fn reply(mut ws_tx: SplitSink<WebSocket, Message>, answer: &Value) {
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
pub(crate) fn client_address(headers: &HeaderMap, hops: usize) -> String {
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

/// Messages and bytes one way.
#[derive(Default)]
struct Count {
    messages: AtomicU64,
    bytes: AtomicU64,
}

impl Count {
    fn add(&self, bytes: usize) {
        self.messages.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    fn get(&self) -> (u64, u64) {
        (
            self.messages.load(Ordering::Relaxed),
            self.bytes.load(Ordering::Relaxed),
        )
    }
}

/// Verifies over the socket; the verdict, and the socket's sending half for the last message.
async fn run<D, T>(
    state: &AppState,
    mut ws_tx: SplitSink<WebSocket, Message>,
    ws_rx: SplitStream<WebSocket>,
    stmt: &vtop_tlsn::statement::Statement,
    dial: D,
) -> (Verdict, SplitSink<WebSocket, Message>)
where
    D: Future<Output = io::Result<T>>,
    T: futures::AsyncRead + futures::AsyncWrite + Send + Unpin,
{
    let started = Instant::now();
    let (from_phone, to_phone) = (Count::default(), Count::default());
    // Only non-empty binary messages carry the session (tlsn never sends an empty one);
    // anything else from the phone, or silence past IDLE_TIMEOUT, ends the stream.
    let rx = futures::stream::unfold(ws_rx, |mut ws_rx| async {
        loop {
            match tokio::time::timeout(IDLE_TIMEOUT, ws_rx.next()).await {
                Ok(Some(Ok(Message::Binary(bytes)))) if !bytes.is_empty() => {
                    from_phone.add(bytes.len());
                    return Some((Ok(bytes.to_vec()), ws_rx));
                }
                Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => {}
                _ => return None,
            }
        }
    });
    let (tx, out) = mpsc::channel::<Vec<u8>>(8);
    let pipe = Pipe::new(Box::pin(rx), tx.sink_map_err(io::Error::other));
    // On the heap: tlsn's futures are large, and a worker thread's stack is not.
    let verified = Box::pin(session::verify(
        pipe,
        stmt,
        state.proof.roots.clone(),
        dial,
        tokio::time::sleep,
    ));
    // The pipe's sender goes with the session, which ends this loop; `ws_tx` then sends the verdict.
    let forward = async {
        // Owned here: when sending fails this drops it, so the session's writes fail at once
        // instead of blocking on a full channel until the timeout.
        let mut out = out;
        let mut next = out.next().await;
        while let Some(mut frame) = next.take() {
            // tlsn writes many small messages; what is already queued goes in one frame of at
            // most the pipe's message size. The phone reads a byte stream (its own `Pipe`), so
            // where frames split does not matter.
            while let Ok(more) = out.try_recv() {
                if frame.len() + more.len() > session::MAX_MESSAGE {
                    next = Some(more);
                    break;
                }
                frame.extend_from_slice(&more);
            }
            to_phone.add(frame.len());
            if ws_tx.send(Message::Binary(frame.into())).await.is_err() {
                break;
            }
            if next.is_none() {
                next = out.next().await;
            }
        }
    };
    let (verified, ()) = join(verified, forward).await;
    let ms = started.elapsed().as_millis() as u64;
    let ((in_messages, in_bytes), (out_frames, out_bytes)) = (from_phone.get(), to_phone.get());
    tracing::info!(
        ms,
        in_messages,
        in_bytes,
        out_frames,
        out_bytes,
        "proof: session over"
    );
    let verdict = match verified.map(|proven| proven.page.get("regno").cloned()) {
        Ok(Some(regno)) => {
            let regno = account_id(&regno);
            tracing::Span::current().record("regno", tracing::field::display(&regno));
            tracing::info!("proof: verified in {ms} ms");
            Verdict::Proved(regno)
        }
        Ok(None) => failed(ms, "no registration number in the proof".into()),
        Err(error) => failed(ms, format!("{error:#}")),
    };
    (verdict, ws_tx)
}

fn failed(ms: u64, mut reason: String) -> Verdict {
    // The reason names a step and a status line, never the session.
    reason.truncate(reason.floor_char_boundary(MAX_REASON));
    tracing::info!("proof: none after {ms} ms: {reason}");
    Verdict::Failed(reason)
}

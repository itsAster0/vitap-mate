//! The account proof (the contract's "Identity: TLSNotary only"): how this phone shows the server
//! who the student is without the VTOP session leaving it. It proves its regno with TLSNotary
//! (vtop-tlsn): `auth:proveStart` hands out a one-use ticket, the phone opens
//! `wss://<host>/proof?ticket=`, which goes straight to the engine; the engine dials VTOP and
//! relays this phone's TLS to it, then checks the proof, which reveals the page's regno and hides
//! the session id. The engine marks the ticket proved and says so as the socket's last message;
//! the phone then trades the ticket for its JWT (`auth:token`, [`crate::remote::backend`]). So no
//! consent is asked: no session goes.
//!
//! Kotlin carries the WebSocket ([`ProofNetwork`]), as it carries VTOP's hops (`transport.rs`);
//! without one, proving fails. The session id and ticket are never logged.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use futures::future::{Either, join, join3, select};
use futures::{SinkExt, StreamExt, channel::mpsc};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{info, warn};
use vtop_core::VtopError;
use vtop_tlsn::session::{MAX_MESSAGE, Phase};

use crate::device::EdgeError;
use crate::remote::backend::Kind;
use crate::remote::server_code::ServerCode;
use crate::transport::{Header, TransportError};
use crate::{Core, CoreError};

/// How long opening the socket may take.
const OPEN_TIMEOUT: Duration = Duration::from_secs(10);

/// One message on the proof's WebSocket.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ProofFrame {
    /// The tlsn session, either way.
    Binary { data: Vec<u8> },
    /// The engine's last message: proved, or why not.
    Text { text: String },
}

/// What opening the socket gave.
#[derive(uniffi::Enum)]
pub enum ProofOpened {
    Open {
        socket: Arc<dyn ProofSocket>,
    },
    /// The server refused before the upgrade: its status and `{ code, error }` body.
    Refused {
        status: u16,
        body: Vec<u8>,
    },
}

/// Kotlin: WebSockets to the server for account proofs, on the app's network.
#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait ProofNetwork: Send + Sync {
    /// Opens a WebSocket to `url` (`wss://…`) with `headers` as they are. Never logs `url` (it
    /// carries the single-use ticket) or `headers`, and neither follows redirects nor retries: a
    /// ticket works once.
    async fn open(
        &self,
        url: String,
        headers: Vec<Header>,
        timeout_ms: u32,
    ) -> Result<ProofOpened, TransportError>;
}

/// One WebSocket. `send` and `receive` are called at the same time; `receive` keeps the
/// order frames arrived in (the engine's last text often comes right before its close), and
/// `shutdown` makes a pending `receive` return promptly.
#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait ProofSocket: Send + Sync {
    /// Sends one binary message.
    async fn send(&self, data: Vec<u8>) -> Result<(), TransportError>;
    /// The next message; None once the socket closed.
    async fn receive(&self) -> Result<Option<ProofFrame>, TransportError>;
    /// Closes the socket (not `close`: UniFFI objects already have one).
    fn shutdown(&self);
}

/// The engine's last message when it proved.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Proved {
    ok: bool,
    registration_number: String,
}

fn failed(reason: impl Into<String>) -> CoreError {
    CoreError::Other {
        reason: format!(
            "Could not prove who you are to the server: {}",
            reason.into()
        ),
    }
}

/// `{ code, error }` from the server as the error the account call would have given.
fn refusal(body: &[u8]) -> CoreError {
    let body: Value = serde_json::from_slice(body).unwrap_or_default();
    let text = |key| body.get(key).and_then(Value::as_str).unwrap_or_default();
    match (text("code"), text("error")) {
        ("", _) => failed("the server sent an unexpected answer"),
        (code, message) => EdgeError::Status {
            code: ServerCode::parse(code),
            message: message.into(),
        }
        .into(),
    }
}

/// A closed socket when the proof is done with it, whichever way.
struct Shut(Arc<dyn ProofSocket>);

impl Drop for Shut {
    fn drop(&mut self) {
        self.0.shutdown();
    }
}

/// A proof's step timings (ms) and its traffic each way, for its one log line: numbers only,
/// never the ticket or the session.
#[derive(Default)]
struct Stats {
    verify_ms: AtomicU64,
    prove_start_ms: AtomicU64,
    ws_open_ms: AtomicU64,
    setup_ms: AtomicU64,
    tls_ms: AtomicU64,
    zk_ms: AtomicU64,
    close_ms: AtomicU64,
    up_msgs: AtomicU64,
    up_bytes: AtomicU64,
    down_msgs: AtomicU64,
    down_bytes: AtomicU64,
}

fn since(at: Instant) -> u64 {
    at.elapsed().as_millis() as u64
}

impl Stats {
    fn set(field: &AtomicU64, ms: u64) {
        field.store(ms, Ordering::Relaxed);
    }

    fn count(msgs: &AtomicU64, bytes: &AtomicU64, len: usize) {
        msgs.fetch_add(1, Ordering::Relaxed);
        bytes.fetch_add(len as u64, Ordering::Relaxed);
    }

    /// The proof's line, `outcome` "proved" or why not; `auth:token`'s time is on the
    /// "server identity" line after it.
    fn log(&self, outcome: &str, ms: u64) {
        let get = |field: &AtomicU64| field.load(Ordering::Relaxed);
        info!(
            route = "proof",
            outcome,
            ms,
            verify_session_ms = get(&self.verify_ms),
            prove_start_ms = get(&self.prove_start_ms),
            ws_open_ms = get(&self.ws_open_ms),
            tlsn_setup_ms = get(&self.setup_ms),
            tls_ms = get(&self.tls_ms),
            zk_prove_ms = get(&self.zk_ms),
            close_ms = get(&self.close_ms),
            up_msgs = get(&self.up_msgs),
            up_bytes = get(&self.up_bytes),
            down_msgs = get(&self.down_msgs),
            down_bytes = get(&self.down_bytes),
            "proof timings"
        );
    }
}

/// Joins what the prover has queued meanwhile onto `message`, up to [`MAX_MESSAGE`] (tlsn writes
/// many small pieces at once; each would be its own FFI call and frame). A piece that would go
/// over waits in `held` for the next message.
fn join_ready(
    from: &mut mpsc::UnboundedReceiver<Vec<u8>>,
    message: &mut Vec<u8>,
    held: &mut Option<Vec<u8>>,
) {
    while held.is_none() {
        match from.try_recv() {
            Ok(more) if message.len() + more.len() <= MAX_MESSAGE => {
                message.extend_from_slice(&more);
            }
            Ok(more) => *held = Some(more),
            Err(_) => break,
        }
    }
}

impl Core {
    /// Proves the signed-in student's regno to the server within `budget`: the ticket, now
    /// proved, for `auth:token`. The regno is the one VTOP named for this session (a proof for
    /// anyone else fails).
    pub(crate) async fn prove_account(
        &self,
        flow: &str,
        budget: Duration,
    ) -> Result<String, CoreError> {
        let stats = Stats::default();
        let started = Instant::now();
        let proving = std::pin::pin!(self.prove_with(flow, &stats));
        let deadline = std::pin::pin!(vtop_core::sync::sleep(budget));
        let out = match select(proving, deadline).await {
            Either::Left((out, _)) => out,
            Either::Right(_) => Err(failed("the server took too long")),
        };
        let outcome = match &out {
            Ok(_) => "proved",
            Err(CoreError::Offline) => "offline",
            Err(CoreError::Other { reason }) if reason.ends_with("too long") => "timed out",
            Err(_) => "failed",
        };
        stats.log(outcome, since(started));
        out
    }

    async fn prove_with(&self, flow: &str, stats: &Stats) -> Result<String, CoreError> {
        let network = (self.shared.proof.clone())
            .ok_or_else(|| failed("this app cannot open the proof's connection"))?;
        {
            // A session id of another shape can never be proved: fail before the server.
            let slot = self.current()?;
            #[expect(
                clippy::disallowed_methods,
                reason = "read here only to check its shape; it never leaves the phone"
            )]
            let cookies = slot.read().await.client.cookie_header().unwrap_or_default();
            if cookies.contains("JSESSIONID=") && vtop_tlsn::content::session_id(&cookies).is_err()
            {
                return Err(failed(
                    "VTOP's session cookie is not one the proof can carry",
                ));
            }
        }
        let device = self.registration().0;

        // VTOP's session is checked (signing in again when it expired, as before sending it)
        // while the ticket comes and the socket opens: neither needs the session.
        let session = async {
            let at = Instant::now();
            #[expect(
                clippy::disallowed_methods,
                reason = "the session id goes only into the TLS request to VTOP, hidden in the proof"
            )]
            let checked = signed_in!(self, |client| async {
                let regno = client.verify_session().await?;
                Ok::<_, VtopError>((regno, client.cookie_header().unwrap_or_default()))
            });
            Stats::set(&stats.verify_ms, since(at));
            let (regno, cookies) = checked?;
            let session_id = vtop_tlsn::content::session_id(&cookies)
                .map_err(|_| failed("VTOP's session cookie is not one the proof can carry"))?
                .to_string();
            Ok::<_, CoreError>((regno, session_id))
        };
        let socket = async {
            let at = Instant::now();
            let start = self
                .signed_call(flow, Kind::Mutation, "auth:proveStart", &json!({}))
                .await;
            Stats::set(&stats.prove_start_ms, since(at));
            // Only URL-safe characters: it goes in the query as it is.
            let ticket = start?
                .get("ticket")
                .and_then(Value::as_str)
                .filter(|t| {
                    (16..=128).contains(&t.len())
                        && t.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                })
                .ok_or_else(|| failed("the server sent no ticket"))?
                .to_string();
            let url = format!(
                "{}/proof?ticket={ticket}",
                self.forwarding_server()
                    .trim_end_matches('/')
                    .replacen("https://", "wss://", 1)
                    .replacen("http://", "ws://", 1)
            );
            let at = Instant::now();
            let opened = network
                .open(url, Vec::new(), OPEN_TIMEOUT.as_millis() as u32)
                .await;
            Stats::set(&stats.ws_open_ms, since(at));
            let opened = opened.map_err(|error| match error {
                TransportError::Connect { .. } | TransportError::Timeout { .. } => {
                    CoreError::Offline
                }
                TransportError::Other { reason } => failed(reason),
            })?;
            let socket = match opened {
                ProofOpened::Open { socket } => socket,
                ProofOpened::Refused { status, body } => {
                    info!(status, route = "proof", "proof refused");
                    return Err(refusal(&body));
                }
            };
            Ok((ticket, Shut(socket)))
        };
        let (session, socket) = join(session, socket).await;
        // The socket's error first: a refused ticket says more than a session that went with it.
        let (ticket, shut) = socket?;
        let (regno, session_id) = session?;
        let socket = shut.0.clone();
        let binding = vtop_tlsn::content::binding(&ticket, &device);
        let (host, roots) = self.proof_target()?;
        let stmt = vtop_tlsn::content::Content { host, binding }
            .statement()
            .map_err(|error| failed(format!("{error:#}")))?;
        let fill = vtop_tlsn::session::Fill {
            values: [("JSESSIONID", session_id.as_str())].into(),
        };

        // One reader for the socket, so no receive is ever dropped half-way: binary messages go
        // to the prover, the engine's text is the answer.
        let (to_prover, from_socket) = mpsc::unbounded::<Vec<u8>>();
        let reader = async {
            // Owned here, so the prover sees the end of the stream the moment the socket
            // closes or the engine answers, instead of waiting out its deadline.
            let to_prover = to_prover;
            loop {
                match socket.receive().await {
                    Ok(Some(ProofFrame::Binary { data })) => {
                        Stats::count(&stats.down_msgs, &stats.down_bytes, data.len());
                        // The prover is gone once its session ended; the text still comes.
                        #[expect(clippy::let_underscore_must_use, reason = "see above")]
                        let _ = to_prover.unbounded_send(data);
                    }
                    Ok(Some(ProofFrame::Text { text })) => return Some(text),
                    Ok(None) | Err(_) => return None,
                }
            }
        };
        // And one writer: tlsn may not flush, so the pipe's sink is a channel that never waits.
        let (to_socket, mut from_prover) = mpsc::unbounded::<Vec<u8>>();
        let writer = async {
            let mut held = None;
            loop {
                let next = match held.take() {
                    Some(data) => Some(data),
                    None => from_prover.next().await,
                };
                let Some(mut message) = next else { break };
                join_ready(&mut from_prover, &mut message, &mut held);
                Stats::count(&stats.up_msgs, &stats.up_bytes, message.len());
                if socket.send(message).await.is_err() {
                    break;
                }
            }
        };
        let sink = to_socket.sink_map_err(std::io::Error::other);
        let pipe = vtop_tlsn::session::Pipe::new(from_socket.map(Ok), sink);
        let started = Instant::now();
        let prover = async {
            let mut last = Instant::now();
            let done = |phase| {
                let field = match phase {
                    Phase::Setup => &stats.setup_ms,
                    Phase::Tls => &stats.tls_ms,
                    Phase::Prove => &stats.zk_ms,
                    Phase::Close => &stats.close_ms,
                };
                Stats::set(field, since(last));
                last = Instant::now();
            };
            // On the heap: tlsn's futures are large, and Kotlin's threads' stacks small.
            let proved = Box::pin(vtop_tlsn::session::prove_phased(
                pipe,
                &stmt,
                &fill,
                roots,
                vtop_core::sync::sleep,
                done,
            ));
            let proved = proved.await;
            if proved.is_err() {
                // Nothing more will come that the engine could answer.
                socket.shutdown();
            }
            proved
        };
        let (proved, last, ()) = join3(prover, reader, writer).await;
        drop(shut);
        let ms = since(started);
        if let Err(error) = proved {
            warn!(
                ms,
                route = "proof",
                error = format!("{error:#}"),
                "proof failed"
            );
            // The prover's own refusal is the cause; otherwise the engine's answer, when it sent
            // one, says why better than tlsn's broken link.
            let refused = error
                .downcast_ref::<vtop_tlsn::session::Refused>()
                .is_some();
            return Err(match last {
                Some(text) if !refused && serde_json::from_str::<Proved>(&text).is_err() => {
                    refusal(text.as_bytes())
                }
                _ => failed(format!("{error:#}")),
            });
        }
        let last = last.ok_or_else(|| failed("the server closed without an answer"))?;
        let proved = match serde_json::from_str::<Proved>(&last) {
            Ok(proved) if proved.ok => proved,
            _ => {
                info!(ms, route = "proof", "proof refused");
                return Err(refusal(last.as_bytes()));
            }
        };
        if !proved.registration_number.eq_ignore_ascii_case(&regno) {
            warn!(ms, route = "proof", "proof names another student");
            return Err(failed("VTOP's session is another student's"));
        }
        Ok(ticket)
    }

    /// The server proofs are held to and its roots: VTOP's, or a test's fake one.
    fn proof_target(&self) -> Result<(String, vtop_tlsn::RootCertStore), CoreError> {
        #[cfg(test)]
        if let Some(target) = self
            .shared
            .proof_target
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
        {
            return Ok(target);
        }
        let roots =
            vtop_tlsn::session::vtop_roots().map_err(|error| failed(format!("{error:#}")))?;
        Ok((vtop_tlsn::content::VTOP_HOST.into(), roots))
    }
}

#[cfg(test)]
pub(crate) mod tests;

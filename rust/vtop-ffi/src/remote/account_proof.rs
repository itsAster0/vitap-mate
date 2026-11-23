//! The account proof (`edge/API.md`, "Account proof"): how this phone gets its account token
//! without the VTOP session leaving it. It proves its regno with TLSNotary (vtop-tlsn): the edge
//! hands out a challenge, the phone opens a signed WebSocket to the edge, which relays it to the
//! engine; the engine dials VTOP and relays this phone's TLS to it, then checks the proof, which
//! reveals the page's regno and hides the session id. The edge mints the token from the engine's
//! verdict and sends it as the socket's last message. So no consent is asked: no session goes.
//!
//! Kotlin carries the WebSocket ([`ProofNetwork`], OkHttp), as it carries VTOP's hops
//! (`transport.rs`); without one, proving fails. The session id, challenge and token are never
//! logged.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use futures::future::{Either, join3, select};
use futures::{SinkExt, StreamExt, channel::mpsc};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{info, warn};
use vtop_core::VtopError;

use crate::device::EdgeError;
use crate::transport::{Header, TransportError};
use crate::{Core, CoreError};

/// How long opening the socket may take.
const OPEN_TIMEOUT: Duration = Duration::from_secs(10);

/// One message on the proof's WebSocket.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ProofFrame {
    /// The tlsn session, either way.
    Binary { data: Vec<u8> },
    /// The edge's last message: the token, or why there is none.
    Text { text: String },
}

/// What opening the socket gave: the socket, or the edge's refusal before the upgrade (its
/// status and `{ code, error }` body).
#[derive(uniffi::Record)]
pub struct ProofOpened {
    pub socket: Option<Arc<dyn ProofSocket>>,
    pub status: u16,
    pub body: Vec<u8>,
}

/// Kotlin: WebSockets to the edge for account proofs, on the app's HTTP stack.
#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait ProofNetwork: Send + Sync {
    /// Opens a WebSocket to `url` (`wss://…`) with `headers` (its vm1 signature) as they are.
    /// Never logs `url` (it carries the single-use challenge) or `headers` (the signature),
    /// and neither follows redirects nor retries: a challenge works once.
    async fn open(
        &self,
        url: String,
        headers: Vec<Header>,
        timeout_ms: u32,
    ) -> Result<ProofOpened, TransportError>;
}

/// One WebSocket. `send` and `receive` are called at the same time; `receive` keeps the
/// order frames arrived in (the edge's last text often comes right before its close), and
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

#[cfg(not(test))]
fn network() -> &'static Mutex<Option<Arc<dyn ProofNetwork>>> {
    static NETWORK: Mutex<Option<Arc<dyn ProofNetwork>>> = Mutex::new(None);
    &NETWORK
}

/// Per test thread, so tests running at once don't share one.
#[cfg(test)]
fn network() -> &'static Mutex<Option<Arc<dyn ProofNetwork>>> {
    thread_local!(static NETWORK: &'static Mutex<Option<Arc<dyn ProofNetwork>>> =
        Box::leak(Box::new(Mutex::new(None))));
    NETWORK.with(|n| *n)
}

/// Opens account proofs' WebSockets through `network`; call before [`crate::Core::new`].
#[uniffi::export]
pub fn set_proof_network(network: Arc<dyn ProofNetwork>) {
    info!("proof network: foreign");
    *self::network()
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(network);
}

/// The edge's last message.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Issued {
    account_token: String,
    /// Unix ms.
    expires_at: i64,
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

/// `{ code, error }` from the edge as the error the account call would have given.
fn refusal(body: &[u8]) -> CoreError {
    let body: Value = serde_json::from_slice(body).unwrap_or_default();
    let text = |key| body.get(key).and_then(Value::as_str).unwrap_or_default();
    match (text("code"), text("error")) {
        ("", _) => failed("the server sent an unexpected answer"),
        (code, message) => EdgeError::Status {
            code: code.into(),
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

impl Core {
    /// Proves the signed-in student's regno to `server`: the account token it issued, its end
    /// (unix ms) and the regno, which VTOP named for this session too (a proof for anyone else
    /// fails).
    pub(crate) async fn prove_account(
        &self,
        server: &str,
        flow: &str,
    ) -> Result<(String, i64, String), CoreError> {
        // A live session: one that expired signs in again first, as before sending it did.
        #[expect(
            clippy::disallowed_methods,
            reason = "the session id goes only into the TLS request to VTOP, hidden in the proof"
        )]
        let (regno, cookies) = signed_in!(self, |client| async {
            let regno = client.verify_session().await?;
            Ok::<_, VtopError>((regno, client.cookie_header().unwrap_or_default()))
        })?;
        let network = network()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let network =
            network.ok_or_else(|| failed("this app cannot open the proof's connection"))?;
        let session_id = vtop_tlsn::content::session_id(&cookies)
            .map_err(|_| failed("VTOP's session cookie is not one the proof can carry"))?
            .to_string();
        drop(cookies);
        let started = Instant::now();
        let device = self.device_id(server, flow).await?;
        let start = self
            .edge_post(server, "/v1/account/prove/start", flow, &json!({}))
            .await?;
        let challenge = start
            .get("challenge")
            .and_then(Value::as_str)
            .filter(|c| c.len() == 32 && c.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| failed("the server sent no challenge"))?
            .to_string();
        let path = format!("/v1/account/prove?challenge={challenge}");
        let headers = self.signed_headers("GET", &path, &device).into_iter();
        let headers = headers
            .map(|(name, value)| Header { name, value })
            .collect();
        let url = format!(
            "{}{path}",
            server
                .replacen("https://", "wss://", 1)
                .replacen("http://", "ws://", 1)
        );
        let opened = network
            .open(url, headers, OPEN_TIMEOUT.as_millis() as u32)
            .await
            .map_err(|error| match error {
                TransportError::Connect { .. } | TransportError::Timeout { .. } => {
                    CoreError::Offline
                }
                TransportError::Other { reason } => failed(reason),
            })?;
        let Some(socket) = opened.socket else {
            info!(status = opened.status, route = "proof", "proof refused");
            return Err(refusal(&opened.body));
        };
        let shut = Shut(socket.clone());
        let binding = vtop_tlsn::content::binding(&challenge, &device);
        let (host, roots) = self.proof_target()?;
        let stmt = vtop_tlsn::content::Content { host, binding }
            .statement()
            .map_err(|error| failed(format!("{error:#}")))?;
        let fill = vtop_tlsn::session::Fill {
            values: [("JSESSIONID", session_id.as_str())].into(),
        };

        // One reader for the socket, so no receive is ever dropped half-way: binary messages go
        // to the prover, the edge's text is the answer.
        let (to_prover, from_socket) = mpsc::unbounded::<Vec<u8>>();
        let reader = async {
            // Owned here, so the prover sees the end of the stream the moment the socket
            // closes or the edge answers, instead of waiting out its deadline.
            let to_prover = to_prover;
            loop {
                match socket.receive().await {
                    Ok(Some(ProofFrame::Binary { data })) => {
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
            while let Some(data) = from_prover.next().await {
                if socket.send(data).await.is_err() {
                    break;
                }
            }
        };
        let sink = to_socket.sink_map_err(std::io::Error::other);
        let pipe = vtop_tlsn::session::Pipe::new(from_socket.map(Ok), sink);
        let prover = async {
            // On the heap: tlsn's futures are large, and Kotlin's threads' stacks small.
            let proved = Box::pin(vtop_tlsn::session::prove(
                pipe,
                &stmt,
                &fill,
                roots,
                vtop_core::sync::sleep,
            ));
            let proved = proved.await;
            if proved.is_err() {
                // Nothing more will come that the edge could answer.
                socket.shutdown();
            }
            proved
        };
        let both = std::pin::pin!(join3(prover, reader, writer));
        let deadline = std::pin::pin!(vtop_core::sync::sleep(
            vtop_tlsn::session::TIMEOUT + OPEN_TIMEOUT
        ));
        let (proved, last, ()) = match select(both, deadline).await {
            Either::Left((done, _)) => done,
            Either::Right(_) => return Err(failed("the server took too long")),
        };
        drop(shut);
        let ms = started.elapsed().as_millis() as u64;
        if let Err(error) = proved {
            warn!(
                ms,
                route = "proof",
                error = format!("{error:#}"),
                "proof failed"
            );
            // The prover's own refusal is the cause; otherwise the edge's answer, when it sent
            // one, says why better than tlsn's broken link.
            let refused = error
                .downcast_ref::<vtop_tlsn::session::Refused>()
                .is_some();
            return Err(match last {
                Some(text) if !refused && serde_json::from_str::<Issued>(&text).is_err() => {
                    refusal(text.as_bytes())
                }
                _ => failed(format!("{error:#}")),
            });
        }
        let last = last.ok_or_else(|| failed("the server closed without an answer"))?;
        let issued: Issued = match serde_json::from_str(&last) {
            Ok(issued) => issued,
            Err(_) => {
                info!(ms, route = "proof", "proof answered without a token");
                return Err(refusal(last.as_bytes()));
            }
        };
        if !issued.registration_number.eq_ignore_ascii_case(&regno) {
            warn!(ms, route = "proof", "proof names another student");
            return Err(failed("VTOP's session is another student's"));
        }
        info!(ms, route = "proof", "account proved");
        Ok((issued.account_token, issued.expires_at, regno))
    }

    /// The server proofs are held to and its roots: VTOP's, or a test's fake one.
    fn proof_target(&self) -> Result<(String, vtop_tlsn::RootCertStore), CoreError> {
        #[cfg(test)]
        if let Some(target) = self
            .shared
            .proof_target
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
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

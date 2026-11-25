//! Convex RPC dispatch (`public`, `signed`, `jwt`, `account`, `device`), timeouts,
//! client initialization and error mapping.

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::FutureExt;
use futures::future::{Either, select};
use serde_json::{Value, json};
use tracing::{info, warn};
use vtop_shared::device_sig;

use super::{Backend, Kind};
use crate::convex::{CallError, ConvexClient, Served};
use crate::device::EdgeError;
use crate::remote::server_code::ServerCode;
use crate::{Core, CoreError, unix_now};

/// How long a call may take; engine calls (`vtop:*`) get [`ENGINE_TIMEOUT`].
const TIMEOUT: Duration = Duration::from_secs(15);
/// The engine signs in or fetches from VTOP behind `vtop:*` (it gives up at 32 s).
const ENGINE_TIMEOUT: Duration = Duration::from_secs(35);

/// `{ code, error }` from what a function threw, in the edge's words.
pub(super) fn edge_error(path: &str, error: CallError) -> EdgeError {
    match error {
        CallError::Closed => EdgeError::Offline,
        CallError::Message(message) => {
            let missing = message.contains("Could not find public function");
            // Only the first line: later ones can quote the arguments (a validator's `Value:`),
            // which may hold credentials. The first carries the request id when redacted.
            let reason: String = message
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(300)
                .collect();
            warn!(path, missing, reason = reason.as_str(), "server failed");
            EdgeError::Status {
                code: if missing {
                    ServerCode::NotFound
                } else {
                    ServerCode::ServerError
                },
                message: "The server sent an unexpected answer.".into(),
            }
        }
        CallError::Convex { message, data } => {
            let text = |key| data.get(key).and_then(Value::as_str).map(str::to_string);
            let code =
                text("code").map_or(ServerCode::ServerError, |code| ServerCode::parse(&code));
            let message = text("error").unwrap_or(message);
            // These answer the phone's own state (an expired JWT, a removed phone, the demo):
            // the caller handles them. Any other code is worth a look.
            if matches!(
                code,
                ServerCode::Unauthenticated
                    | ServerCode::DeviceRemoved
                    | ServerCode::DemoUnavailable
            ) {
                info!(
                    path,
                    code = code.as_str(),
                    message = message.as_str(),
                    "server refused"
                );
            } else {
                warn!(
                    path,
                    code = code.as_str(),
                    message = message.as_str(),
                    "server refused"
                );
            }
            EdgeError::Status { code, message }
        }
    }
}

/// `work`, or `CallError::Closed` once `timeout` passed.
async fn within<T>(
    timeout: Duration,
    work: impl Future<Output = Result<T, CallError>>,
) -> Result<T, CallError> {
    let work = std::pin::pin!(work);
    let late = std::pin::pin!(vtop_core::sync::sleep(timeout));
    match select(work, late).await {
        Either::Left((done, _)) => done,
        Either::Right(_) => Err(CallError::Closed),
    }
}

/// `args` (an object) with the flow added.
fn with_flow(flow: &str, mut args: Value) -> Value {
    crate::json_set(&mut args, "flow", json!(flow));
    args
}

impl Core {
    /// The client, made on first use for the server; None before Kotlin set a network.
    pub(super) fn backend(&self) -> Result<Arc<dyn Backend>, EdgeError> {
        let mut state = self.shared.backend_state();
        if let Some(backend) = &state.backend {
            return Ok(backend.clone());
        }
        #[cfg(test)]
        if let Some(fake) = crate::test_edge::backend_for(&self.forwarding_server()) {
            state.backend = Some(fake.clone());
            return Ok(fake);
        }
        let network = self.shared.convex.clone().ok_or_else(|| {
            warn!("no convex network set");
            EdgeError::Offline
        })?;
        let shared = Arc::downgrade(&self.shared);
        let client = ConvexClient::new(&self.forwarding_server(), network, move || {
            if let Some(shared) = shared.upgrade() {
                shared.on_socket_auth_error();
            }
        });
        state.backend = Some(client.clone());
        Ok(client)
    }

    /// Runs `path` with `args` (an object) and the flow; no auth beyond what the client holds.
    pub(super) async fn run(
        &self,
        flow: &str,
        kind: Kind,
        path: &str,
        args: Value,
    ) -> Result<Value, CallError> {
        // Queries go without it: identical reads then share the client's subscription (a
        // watched query is answered locally) and the server's cache. The flow is still logged.
        let args = if kind == Kind::Query {
            args
        } else {
            with_flow(flow, args)
        };
        self.send(flow, kind, path, args).await
    }

    /// Runs `path` with `args` as they are, logged in `flow`.
    async fn send(
        &self,
        flow: &str,
        kind: Kind,
        path: &str,
        args: Value,
    ) -> Result<Value, CallError> {
        let backend = self.backend().map_err(|_| CallError::Closed)?;
        let timeout = if path.starts_with("vtop:") {
            ENGINE_TIMEOUT
        } else {
            TIMEOUT
        };
        let started = Instant::now();
        let (answer, served) = within(timeout, backend.run(kind, path, args).map(Ok))
            .await
            .unwrap_or_else(|late| (Err(late), Served::Server));
        let outcome = match &answer {
            Ok(_) => "ok",
            Err(CallError::Closed) => "offline",
            Err(CallError::Message(_)) => "failed",
            Err(CallError::Convex { data, .. }) => data
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("refused"),
        };
        let ms = started.elapsed().as_millis() as u64;
        // `served = Local`: a live watch held the answer (Convex never saw this read), so a
        // query's flow is traced here, never in its arguments, which would split the cache.
        info!(flow, path, ?kind, ?served, ms, outcome, "server call");
        answer
    }

    /// A `public` function. These take no `flow` (their arguments are `{}`), so it is only
    /// logged.
    pub(crate) async fn public_call(
        &self,
        flow: &str,
        kind: Kind,
        path: &str,
        args: Value,
    ) -> Result<Value, EdgeError> {
        self.send(flow, kind, path, args)
            .await
            .map_err(|error| edge_error(path, error))
    }

    /// A `jwt` function with the JWT already set (or none), never getting one: for calls
    /// [`Core::identity`] makes itself.
    pub(crate) async fn jwt_call(
        &self,
        flow: &str,
        kind: Kind,
        path: &str,
        args: Value,
    ) -> Result<Value, EdgeError> {
        self.run(flow, kind, path, args)
            .await
            .map_err(|error| edge_error(path, error))
    }

    /// A `signed` function: `payload` as JSON text, signed with this install's key over
    /// `CALL /<path>`. A `stale_time` answer sets the clock offset from its `serverTime` and
    /// retries once.
    pub(crate) async fn signed_call(
        &self,
        flow: &str,
        kind: Kind,
        path: &str,
        payload: &Value,
    ) -> Result<Value, EdgeError> {
        let text = payload.to_string();
        let mut retried = false;
        loop {
            let device = self.device();
            let time = unix_now().saturating_add(device.clock_offset);
            let signed = json!({
                "key": device_sig::public_key(&device.seed).unwrap_or_default(),
                "time": time,
                "sig": device_sig::sign(&device.seed, "CALL", &format!("/{path}"), time, text.as_bytes())
                    .unwrap_or_default(),
            });
            let args = json!({ "signed": signed, "payload": text });
            match self.run(flow, kind, path, args).await {
                Err(CallError::Convex { data, .. })
                    if !retried
                        && data.get("code").and_then(Value::as_str) == Some("stale_time") =>
                {
                    let server_time = data.get("serverTime").and_then(Value::as_i64);
                    // The edge answered seconds; a Convex function may answer ms.
                    let server_time =
                        server_time.map(|t| if t > 10_000_000_000 { t / 1000 } else { t });
                    let offset = server_time
                        .unwrap_or_else(unix_now)
                        .saturating_sub(unix_now());
                    info!(offset, "clock offset from the server");
                    self.set_clock_offset(offset);
                    retried = true;
                }
                answer => return answer.map_err(|error| edge_error(path, error)),
            }
        }
    }

    /// A `jwt` function as the signed-in student (or, signed out, as this install's last one).
    /// Without a JWT it gets one ([`Self::identity`]), proving who the student is only when
    /// `ask` (a call nobody started never proves, and is `ConsentDenied`, which callers take as
    /// "not now"); `action` names it in the logs. Until the student shares their registration
    /// number it is `RegnoNotShared` (with `ask`, they are asked first). A JWT the server turns down is renewed once.
    /// The outer error is the phone's (not signed in, the proof), the inner one the server's.
    pub(crate) async fn account_call(
        &self,
        flow: &str,
        kind: Kind,
        path: &str,
        action: &str,
        ask: bool,
        args: Value,
    ) -> Result<Result<Value, EdgeError>, CoreError> {
        for attempt in 0..2 {
            let stamp = self.ensure_identity(flow, action, ask).await?;
            let answer = self.run(flow, kind, path, args.clone()).await;
            let code = match &answer {
                Err(CallError::Convex { data, .. }) => data
                    .get("code")
                    .and_then(Value::as_str)
                    .map(ServerCode::parse),
                _ => None,
            };
            match code {
                Some(ServerCode::Unauthenticated) if attempt == 0 => {
                    info!(path, jwt_epoch = stamp.jwt_epoch, "jwt refused: renewing");
                    self.on_jwt_rejected(Some(stamp.jwt_epoch));
                }
                Some(ServerCode::DeviceRemoved) => {
                    warn!(path, gen = stamp.generation, "this install was removed on the server");
                    self.on_device_removed(stamp.generation);
                    return Ok(answer.map_err(|error| edge_error(path, error)));
                }
                _ => return Ok(answer.map_err(|error| edge_error(path, error))),
            }
        }
        Err(CoreError::Offline)
    }

    /// [`Self::account_call`] that never proves: with no identity yet it is the server's
    /// `unauthenticated`.
    pub(crate) async fn device_call(
        &self,
        flow: &str,
        kind: Kind,
        path: &str,
        args: Value,
    ) -> Result<Value, EdgeError> {
        match self.account_call(flow, kind, path, path, false, args).await {
            Ok(answer) => answer,
            Err(CoreError::ConsentDenied | CoreError::NotSignedIn | CoreError::RegnoNotShared) => {
                Err(EdgeError::Status {
                    code: ServerCode::Unauthenticated,
                    message: "This phone hasn't proved who you are to the server yet.".into(),
                })
            }
            Err(CoreError::Offline) => Err(EdgeError::Offline),
            Err(error) => Err(EdgeError::Status {
                code: ServerCode::IdentityFailed,
                message: error.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convex_errors_keep_their_code_and_text() {
        let error = CallError::Convex {
            message: "Uncaught ConvexError".into(),
            data: json!({ "code": "key_limit", "error": "Too many keys." }),
        };
        match edge_error("keys:create", error) {
            EdgeError::Status { code, message } => {
                assert_eq!(
                    (code.as_str(), message.as_str()),
                    ("key_limit", "Too many keys.")
                );
            }
            EdgeError::Offline => panic!("offline"),
        }
        assert!(matches!(
            edge_error("x:y", CallError::Closed),
            EdgeError::Offline
        ));
    }
}

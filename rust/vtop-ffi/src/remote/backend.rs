//! Every call to our server (`backend/`, Convex) goes through here, on one WebSocket
//! (`docs/plans/2026-11-23-backend-v3-api.md`). Three kinds, as the contract names them:
//! - public: [`Core::public_call`];
//! - signed: [`Core::signed_call`], the payload signed with this install's Ed25519 key;
//! - jwt: [`Core::account_call`] and [`Core::device_call`], as the student this install proved
//!   to be. The JWT comes from `auth:token`, from the refresh token kept in `device.json`, a v2
//!   account token traded in once, the demo account, or a TLSNotary proof
//!   ([`crate::remote::account_proof`]); it is renewed before it runs out and whenever the
//!   server turns it down.
//!
//! Every call carries the flow id (`flow`). Arguments, answers, tokens and JWTs are never
//! logged.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use futures::future::{BoxFuture, Either, select};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{info, warn};
use vtop_shared::device_sig;

use crate::convex::{CallError, ConvexClient, ConvexNetwork, SubscriptionId};
use crate::device::EdgeError;
use crate::remote::account_token;
use crate::{Core, CoreError, Shared, unix_now};

/// How long a call may take; engine calls (`vtop:*`) get [`ENGINE_TIMEOUT`].
const TIMEOUT: Duration = Duration::from_secs(15);
/// The engine signs in or fetches from VTOP behind `vtop:*` (it gives up at 32 s).
const ENGINE_TIMEOUT: Duration = Duration::from_secs(35);
/// A JWT this close to its end is renewed first.
const RENEW_AHEAD_MS: i64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Query,
    Mutation,
    Action,
}

/// The server's functions: the Convex client, or a test's stand-in.
pub(crate) trait Backend: Send + Sync {
    fn run<'a>(
        &'a self,
        kind: Kind,
        path: &'a str,
        args: Value,
    ) -> BoxFuture<'a, Result<Value, CallError>>;
    /// The JWT every later call goes with (None: none).
    fn set_auth(&self, jwt: Option<String>);
    /// Follows query `path(args)`: `on_update` hears its result now and after every change.
    fn watch(&self, path: &str, args: Value, on_update: OnUpdate) -> u64;
    fn unwatch(&self, id: u64);
}

pub(crate) type OnUpdate = Box<dyn Fn(Result<Value, CallError>) + Send + Sync>;

impl Backend for ConvexClient {
    fn run<'a>(
        &'a self,
        kind: Kind,
        path: &'a str,
        args: Value,
    ) -> BoxFuture<'a, Result<Value, CallError>> {
        Box::pin(async move {
            match kind {
                Kind::Query => self.query(path, args).await,
                Kind::Mutation => self.mutation(path, args).await,
                Kind::Action => self.action(path, args).await,
            }
        })
    }

    fn set_auth(&self, jwt: Option<String>) {
        ConvexClient::set_auth(self, jwt);
    }

    fn watch(&self, path: &str, args: Value, on_update: OnUpdate) -> u64 {
        self.subscribe(path, args, on_update).0
    }

    fn unwatch(&self, id: u64) {
        self.unsubscribe(SubscriptionId(id));
    }
}

/// What a screen follows live ([`Core::live_watch`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum LiveTopic {
    /// `account:view`: the account, its phones and keys.
    Account,
    /// `mail:inbox`: this phone's forwarded mail.
    MailInbox,
    /// `sessions:pending`: connected apps' requests for this phone's VTOP session.
    SessionRequests,
    /// `screen:view`: what the Server screen shows, its address's phones included.
    Server,
}

/// Hears a [`Core::live_watch`], on the socket's threads.
#[uniffi::export(with_foreign)]
pub trait LiveListener: Send + Sync {
    /// The server's data changed (and once at first): read it again.
    fn on_change(&self);
    /// The query failed with `code` and the server's `message` for the student
    /// (`unauthenticated` renews the JWT; the next change follows).
    fn on_error(&self, code: String, message: String);
}

/// One [`Core::live_watch`]; stopping or dropping it unsubscribes.
#[derive(uniffi::Object)]
pub struct LiveWatch {
    backend: Arc<dyn Backend>,
    id: u64,
    /// While any is held, the JWT is renewed before it runs out ([`keep_renewed`]).
    _held: Arc<()>,
}

#[uniffi::export]
impl LiveWatch {
    /// No more calls to its listener once the socket's queue drains.
    pub fn stop(&self) {
        self.backend.unwatch(self.id);
    }
}

impl Drop for LiveWatch {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Every call takes the flow id first.
#[uniffi::export]
impl Core {
    /// Whether this install holds a server identity it may use unasked for the signed-in
    /// student (a JWT, a refresh token or a v2 account token), so a live watch can follow; read
    /// on the phone alone.
    pub async fn has_server_identity(&self, flow: String) -> bool {
        crate::traced("has_server_identity", &flow, async {
            let regno = self.signed_in_regno().await;
            let jwt = self
                .shared
                .backend_state()
                .jwt
                .as_ref()
                .map(|jwt| jwt.regno.clone());
            let (refresh, v2) = self.kept_tokens(regno.as_deref());
            let demo = regno.as_deref() == Some(crate::demo::USERNAME);
            Ok(demo
                || refresh.is_some()
                || v2.is_some()
                || jwt.is_some_and(|jwt| regno.is_none_or(|regno| regno == jwt)))
        })
        .await
        .unwrap_or(false)
    }

    /// Follows `topic` as the student this install proved to be: `listener` hears each change
    /// as soon as the server has it. Without an identity (it never proves) or a forwarding
    /// address for [`LiveTopic::MailInbox`], an error, and the caller reads once instead.
    pub async fn live_watch(
        self: Arc<Self>,
        flow: String,
        topic: LiveTopic,
        listener: Arc<dyn LiveListener>,
    ) -> Result<Arc<LiveWatch>, CoreError> {
        crate::traced("live_watch", &flow, async {
            let (path, args) = match topic {
                LiveTopic::Account => ("account:view", json!({})),
                LiveTopic::MailInbox => {
                    let (_, id, _) =
                        self.saved_mailbox()
                            .ok_or_else(|| CoreError::ForwardingOff {
                                reason: "Mail forwarding is off.".into(),
                            })?;
                    ("mail:inbox", json!({ "id": id }))
                }
                LiveTopic::SessionRequests => ("sessions:pending", json!({})),
                LiveTopic::Server => (
                    "screen:view",
                    crate::remote::server::screen_args(
                        self.saved_mailbox().as_ref().map(|(_, id, _)| id.as_str()),
                    ),
                ),
            };
            self.identity(&flow, path, false).await?;
            let backend = self.backend()?;
            let (core, renew_flow) = (Arc::downgrade(&self), flow.clone());
            let on_update = move |update: Result<Value, CallError>| match update {
                Ok(_) => listener.on_change(),
                Err(error) => {
                    let error = edge_error(path, error);
                    let code = error.code().to_string();
                    let message = error.message().to_string();
                    match (code.as_str(), core.upgrade()) {
                        ("unauthenticated", Some(core)) => {
                            renew(core, renew_flow.clone(), listener.clone());
                        }
                        // Removed on another phone: its tokens are dead, and no watch follows.
                        ("device_removed", core) => {
                            warn!(path, "this install was removed on the server");
                            if let Some(core) = core {
                                core.removed_here();
                            }
                            listener.on_error(NO_IDENTITY.into(), message);
                            return;
                        }
                        _ => {}
                    }
                    listener.on_error(code, message);
                }
            };
            let id = backend.watch(path, args, Box::new(on_update));
            info!(path, "live watch");
            let held = Arc::new(());
            let start = {
                let mut state = self.shared.backend_state();
                state.watched.push(Arc::downgrade(&held));
                !std::mem::replace(&mut state.renewing, true)
            };
            if start {
                keep_renewed(Arc::downgrade(&self), flow.clone());
            }
            Ok(Arc::new(LiveWatch {
                backend,
                id,
                _held: held,
            }))
        })
        .await
    }
}

/// How often [`keep_renewed`] looks again whether watches are held.
const KEEP_CHECK: Duration = Duration::from_secs(30);

/// Renews the JWT a minute before it runs out while any live watch is held, off the socket's
/// thread. Self-hosted Convex drops a socket whose JWT expired, and every watch would then fail
/// and renew on its own. Ends when the last watch goes, or with no identity it may take.
fn keep_renewed(core: std::sync::Weak<Core>, flow: String) {
    let unstarted = core.clone();
    let spawned = std::thread::Builder::new()
        .name("convex-keep-jwt".into())
        .spawn(move || {
            loop {
                let Some(core) = core.upgrade() else { return };
                let due = {
                    let mut state = core.shared.backend_state();
                    state.watched.retain(|held| held.strong_count() > 0);
                    if state.watched.is_empty() {
                        state.renewing = false;
                        return;
                    }
                    state
                        .jwt
                        .as_ref()
                        .map(|jwt| jwt.expires_at - RENEW_AHEAD_MS)
                };
                let wait = if let Some(wait) = renew_in(due, core.server_now_ms()) {
                    wait
                } else {
                    let renewed = futures::executor::block_on(core.identity(
                        &flow,
                        "keep live watches",
                        false,
                    ));
                    match renewed {
                        Ok(()) => Duration::from_secs(1),
                        Err(CoreError::NotSignedIn | CoreError::ConsentDenied) => {
                            info!("live watches: no identity, not renewed");
                            core.shared.backend_state().renewing = false;
                            return;
                        }
                        Err(error) => {
                            warn!(%error, "live watches: no new jwt");
                            KEEP_CHECK
                        }
                    }
                };
                drop(core);
                std::thread::sleep(wait);
            }
        });
    if let Err(error) = spawned {
        warn!(%error, "live watches: renewal not started");
        if let Some(core) = unstarted.upgrade() {
            core.shared.backend_state().renewing = false;
        }
    }
}

/// How long [`keep_renewed`] sleeps before it looks again, for a JWT due for renewal at `due`
/// (none held: at once) at `now` (both unix ms); None: renew now.
fn renew_in(due: Option<i64>, now: i64) -> Option<Duration> {
    let left = due? - now;
    (left > 0).then(|| Duration::from_millis(left.unsigned_abs()).min(KEEP_CHECK))
}

/// What a [`LiveListener`] hears when its watch can't follow any more.
const NO_IDENTITY: &str = "no_identity";

/// A new JWT for a watch the server stopped answering, off the socket's thread; the
/// subscription runs again once it is set. Without an identity it can take, `listener` hears
/// `no_identity`: the watch can't follow any more.
fn renew(core: Arc<Core>, flow: String, listener: Arc<dyn LiveListener>) {
    let spawned = std::thread::Builder::new()
        .name("convex-renew".into())
        .spawn(move || {
            let renewed = futures::executor::block_on(core.identity(&flow, "live watch", false));
            match renewed {
                Ok(()) => {}
                // Signed out, or no identity it may take unasked: the watch just stops following.
                Err(CoreError::NotSignedIn | CoreError::ConsentDenied) => {
                    info!("live watch: no identity, stops following");
                    listener.on_error(
                        NO_IDENTITY.into(),
                        "Sign in again to follow changes.".into(),
                    );
                }
                Err(error) => warn!(%error, "live watch: no new jwt"),
            }
        });
    if let Err(error) = spawned {
        warn!(%error, "live watch: renewal not started");
    }
}

static NETWORK: Mutex<Option<Arc<dyn ConvexNetwork>>> = Mutex::new(None);

/// Opens the server's WebSocket through `network`; call before [`crate::Core::new`].
#[uniffi::export]
pub fn set_convex_network(network: Arc<dyn ConvexNetwork>) {
    info!("convex network: foreign");
    *NETWORK.lock().unwrap_or_else(PoisonError::into_inner) = Some(network);
}

/// The JWT the client goes with. No `Debug`: it holds the token.
pub(crate) struct Jwt {
    /// Unix ms.
    expires_at: i64,
    regno: String,
    token: String,
}

/// The core's server state ([`Shared::backend`]).
#[derive(Default)]
pub(crate) struct State {
    backend: Option<Arc<dyn Backend>>,
    jwt: Option<Jwt>,
    /// The live watches held ([`LiveWatch::_held`]), and whether [`keep_renewed`] runs.
    watched: Vec<std::sync::Weak<()>>,
    renewing: bool,
}

/// The refresh token `auth:token` handed this install. No `Debug`: it holds the token.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Refresh {
    pub(crate) server: String,
    pub(crate) token: String,
    pub(crate) regno: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Issued {
    jwt: String,
    jwt_expires_at: i64,
    refresh_token: String,
    regno: String,
}

/// The last student's refresh token, set aside at a switch ([`Core::set_aside_identity`]).
pub(crate) const LAST_STUDENT: &str = "server_last_student.json";

/// Whether a sign-out answered `code` because there is nothing left to sign out of.
pub(crate) fn signed_out_already(code: &str) -> bool {
    TOKEN_REFUSED.contains(&code) || code == "unauthenticated"
}

/// Codes `auth:token` answers a refresh or v2 token it won't take with: the token is dropped
/// and the next source is tried. Any other failure (offline, a server fault) keeps it, so a
/// passing fault never costs a new proof.
const TOKEN_REFUSED: [&str; 6] = [
    "device_removed",
    "refresh_reused",
    "refresh_invalid",
    "refresh_expired",
    "token_invalid",
    "token_expired",
];

impl Shared {
    fn backend_state(&self) -> std::sync::MutexGuard<'_, State> {
        self.backend.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Another server (Developer): the next call makes a client for it and gets an identity
    /// there.
    pub(crate) fn reset_backend(&self) {
        *self.backend_state() = State::default();
    }

    /// The server turned the JWT down: the next jwt call fetches a new one.
    pub(crate) fn jwt_stale(&self) {
        self.backend_state().jwt = None;
    }
}

/// `{ code, error }` from what a function threw, in the edge's words.
pub(crate) fn edge_error(path: &str, error: CallError) -> EdgeError {
    match error {
        CallError::Closed => EdgeError::Offline,
        CallError::Message(message) => {
            let missing = message.contains("Could not find public function");
            warn!(path, missing, "server failed");
            EdgeError::Status {
                code: if missing { "not_found" } else { "server_error" }.into(),
                message: "The server sent an unexpected answer.".into(),
            }
        }
        CallError::Convex { message, data } => {
            let text = |key| data.get(key).and_then(Value::as_str).map(str::to_string);
            let code = text("code").unwrap_or_else(|| "server_error".into());
            let message = text("error").unwrap_or(message);
            // These answer the phone's own state (an expired JWT, a removed phone, the demo):
            // the caller handles them. Any other code is worth a look.
            if matches!(
                code.as_str(),
                "unauthenticated" | "device_removed" | "demo_unavailable"
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
    fn backend(&self) -> Result<Arc<dyn Backend>, EdgeError> {
        let mut state = self.shared.backend_state();
        if let Some(backend) = &state.backend {
            return Ok(backend.clone());
        }
        #[cfg(test)]
        if let Some(fake) = crate::test_edge::backend_for(&self.forwarding_server()) {
            state.backend = Some(fake.clone());
            return Ok(fake);
        }
        let network = NETWORK
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .ok_or_else(|| {
                warn!("no convex network set");
                EdgeError::Offline
            })?;
        let shared = Arc::downgrade(&self.shared);
        let client = ConvexClient::new(&self.forwarding_server(), network, move || {
            if let Some(shared) = shared.upgrade() {
                shared.jwt_stale();
            }
        });
        state.backend = Some(client.clone());
        Ok(client)
    }

    /// Runs `path` with `args` (an object) and the flow; no auth beyond what the client holds.
    async fn run(
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
        let started = std::time::Instant::now();
        let answer = within(timeout, backend.run(kind, path, args)).await;
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
        info!(flow, path, ?kind, ms, outcome, "server call");
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
    /// "not now"); `action` names it in the logs. A JWT the server turns down is renewed once.
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
            self.identity(flow, action, ask).await?;
            let answer = self.run(flow, kind, path, args.clone()).await;
            let code = match &answer {
                Err(CallError::Convex { data, .. }) => data.get("code").and_then(Value::as_str),
                _ => None,
            };
            match code {
                Some("unauthenticated") if attempt == 0 => {
                    info!(path, "jwt refused: renewing");
                    self.shared.jwt_stale();
                }
                Some("device_removed") => {
                    warn!(path, "this install was removed on the server");
                    self.removed_here();
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
            Err(CoreError::ConsentDenied | CoreError::NotSignedIn) => Err(EdgeError::Status {
                code: "unauthenticated".into(),
                message: "This phone hasn't proved who you are to the server yet.".into(),
            }),
            Err(CoreError::Offline) => Err(EdgeError::Offline),
            Err(error) => Err(EdgeError::Status {
                code: "identity_failed".into(),
                message: error.to_string(),
            }),
        }
    }

    /// The signed-in student's registration number, upper case; None signed out.
    async fn signed_in_regno(&self) -> Option<String> {
        let slot = self.current().ok()?;
        let session = slot.read().await;
        // A restored session that hasn't loaded a page yet knows it only from the saved account.
        let regno = session.client.registration_number().ok();
        regno
            .or_else(|| session.account.registration_number.clone())
            .or_else(|| crate::demo::is(&session.account).then(|| session.account.username.clone()))
            .map(|regno| regno.to_ascii_uppercase())
    }

    /// Makes sure the client goes with a live JWT for the signed-in student, getting one from
    /// the first source that works: the demo account, the kept refresh token, a v2 account
    /// token (traded in once), then (only when `ask`) a proof. One at a time, so a refresh token
    /// is never used twice. A new one first leaves the addresses queued to be left: a leave
    /// queued by a sign-in over another student waits for this phone's next identity.
    pub(crate) async fn identity(
        &self,
        flow: &str,
        action: &str,
        ask: bool,
    ) -> Result<(), CoreError> {
        if self.take_identity(flow, action, ask).await? {
            self.leave_queued(flow).await;
            self.semester_to_server(flow).await;
        }
        Ok(())
    }

    /// The refresh token and v2 account token [`Core::take_identity`] may use for `regno`
    /// (None: signed out): this server's, and the student's.
    fn kept_tokens(&self, regno: Option<&str>) -> (Option<Refresh>, Option<account_token::Saved>) {
        let server = self.forwarding_server();
        let refresh = self.device().refresh.filter(|kept| {
            kept.server == server
                && regno.is_none_or(|regno| regno.eq_ignore_ascii_case(&kept.regno))
        });
        let v2 = self
            .read::<account_token::Saved>(account_token::FILE)
            .filter(|saved| {
                regno.is_some_and(|regno| regno.eq_ignore_ascii_case(&saved.registration_number))
            });
        (refresh, v2)
    }

    /// [`Core::identity`]'s work; true when it got a new JWT.
    /// Unix ms on the server's clock, as the JWT's expiry is: a phone set minutes off would
    /// otherwise renew on every call, or hold on to an expired JWT.
    fn server_now_ms(&self) -> i64 {
        unix_now()
            .saturating_add(self.device().clock_offset)
            .saturating_mul(1000)
    }

    async fn take_identity(&self, flow: &str, action: &str, ask: bool) -> Result<bool, CoreError> {
        let _turn = self.shared.identity_turn.lock().await;
        let regno = self.signed_in_regno().await;
        let now_ms = self.server_now_ms();
        {
            let state = self.shared.backend_state();
            if let Some(jwt) = &state.jwt
                && jwt.expires_at - RENEW_AHEAD_MS > now_ms
                && regno.as_ref().is_none_or(|regno| *regno == jwt.regno)
            {
                return Ok(false);
            }
        }
        let server = self.forwarding_server();
        for _ in 0..3 {
            let (refresh, v2) = self.kept_tokens(regno.as_deref());
            let demo = regno.as_deref() == Some(crate::demo::USERNAME);
            let (source, payload) = if demo {
                (
                    "demo",
                    json!({ "demo": true, "device": self.device_info() }),
                )
            } else if let Some(refresh) = &refresh {
                ("refresh", json!({ "refreshToken": refresh.token }))
            } else if let Some(v2) = &v2 {
                (
                    "v2",
                    json!({ "v2AccountToken": v2.token, "device": self.device_info() }),
                )
            } else if ask && regno.is_some() {
                info!(action, "no identity: proving");
                let ticket = self.prove_account(flow).await?;
                (
                    "proof",
                    json!({ "ticket": ticket, "device": self.device_info() }),
                )
            } else {
                info!(
                    action,
                    signed_in = regno.is_some(),
                    "no identity: not proving"
                );
                return Err(if ask {
                    CoreError::NotSignedIn
                } else {
                    CoreError::ConsentDenied
                });
            };
            let issued = match self
                .signed_call(flow, Kind::Action, "auth:token", &payload)
                .await
            {
                Ok(issued) => issued,
                Err(error) if TOKEN_REFUSED.contains(&error.code()) && source != "proof" => {
                    warn!(source, code = error.code(), "token refused");
                    match (source, error.code()) {
                        ("v2", _) => self.forget_account_token(),
                        (_, "device_removed") => self.removed_here(),
                        _ => self.forget_identity(),
                    }
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            let issued: Issued = serde_json::from_value(issued).map_err(|_| CoreError::Other {
                reason: "The server sent an unexpected answer.".into(),
            })?;
            if source == "v2" {
                self.forget_account_token();
            }
            // ms; seconds from a server that counts them.
            let expires_at = match issued.jwt_expires_at {
                at if at < 10_000_000_000 => at.saturating_mul(1000),
                at => at,
            };
            self.keep_refresh(Some(Refresh {
                server: server.clone(),
                token: issued.refresh_token,
                regno: issued.regno.to_ascii_uppercase(),
            }));
            self.backend()?.set_auth(Some(issued.jwt.clone()));
            self.shared.backend_state().jwt = Some(Jwt {
                expires_at,
                regno: issued.regno.to_ascii_uppercase(),
                token: issued.jwt,
            });
            // Signed out (leaving an old address), the lines still name the student.
            if regno.is_none() {
                crate::obs::set_regno(Some(issued.regno.to_ascii_uppercase()));
            }
            info!(
                source,
                expires_in_s = (expires_at - now_ms) / 1000,
                "server identity"
            );
            return Ok(true);
        }
        Err(CoreError::Other {
            reason: "The server turned down this phone's sign-in.".into(),
        })
    }

    /// Another student signed in over the last one: the last one's refresh token is kept aside
    /// for [`Core::sign_out_last_student`], and this install has no identity until the new
    /// student's.
    pub(crate) fn set_aside_identity(&self) {
        if let Some(refresh) = self.device().refresh {
            self.write(LAST_STUDENT, &refresh);
        }
        self.forget_identity();
    }

    /// Signs this install out of the last student's account with their refresh token, kept
    /// aside at a switch, so it stops getting their OTPs and session requests before the new
    /// student ever proves. Tried again on the next call while the server can't be reached.
    pub(crate) async fn sign_out_last_student(&self, flow: &str) {
        let Some(last) = self.read::<Refresh>(LAST_STUDENT) else {
            return;
        };
        if last.server != self.forwarding_server() {
            self.forget(&[LAST_STUDENT]);
            return;
        }
        // The socket goes with their JWT for this one call: no other identity can be taken
        // meanwhile.
        let _turn = self.shared.identity_turn.lock().await;
        let body = json!({ "refreshToken": last.token });
        let issued = match self
            .signed_call(flow, Kind::Action, "auth:token", &body)
            .await
        {
            Ok(issued) => issued,
            Err(error) if signed_out_already(error.code()) => {
                info!(code = error.code(), "last student already signed out here");
                self.forget(&[LAST_STUDENT]);
                return;
            }
            Err(error) => {
                info!(code = error.code(), "last student not signed out yet");
                return;
            }
        };
        let (Some(jwt), Ok(backend)) = (issued.get("jwt").and_then(Value::as_str), self.backend())
        else {
            return;
        };
        backend.set_auth(Some(jwt.to_owned()));
        let signed_out = self
            .run(
                flow,
                Kind::Mutation,
                "auth:signOut",
                json!({ "everywhere": false }),
            )
            .await;
        backend.set_auth(None);
        match signed_out {
            Ok(_) => {
                info!("signed out of the last student's account");
                self.forget(&[LAST_STUDENT]);
            }
            Err(error) => {
                let code = edge_error("auth:signOut", error).code().to_owned();
                info!(code, "last student not signed out yet");
            }
        }
    }

    /// Removed on the server (from another phone, or signed out everywhere): it took this
    /// install off every address too, so the address is forgotten with the identity.
    pub(crate) fn removed_here(&self) {
        self.forget_identity();
        self.forget_any_forwarding();
    }

    /// The JWT the client goes with, while it has a minute left: the UDP gateway signs in with
    /// it when the session it holds expires.
    pub(crate) fn live_jwt(&self) -> Option<String> {
        let now = self.server_now_ms();
        let state = self.shared.backend_state();
        let jwt = state.jwt.as_ref()?;
        (jwt.expires_at - now > 60_000).then(|| jwt.token.clone())
    }

    /// Drops the JWT and the refresh token (signed out of the server; another student; this
    /// install removed there). The v2 token goes with the student's files.
    pub(crate) fn forget_identity(&self) {
        crate::remote::udp::forget_jwt_sent();
        let backend = {
            let mut state = self.shared.backend_state();
            state.jwt = None;
            state.backend.clone()
        };
        if let Some(backend) = backend {
            backend.set_auth(None);
        }
        self.keep_refresh(None);
    }
}

#[cfg(not(feature = "tlsn"))]
impl Core {
    async fn prove_account(&self, _: &str) -> Result<String, CoreError> {
        Err(CoreError::Other {
            reason: "This build cannot prove who you are to the server.".into(),
        })
    }
}

#[cfg(test)]
mod tests;

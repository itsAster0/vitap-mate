//! Every call to our server (`backend/`, Convex) goes through here, on one WebSocket
//! (`docs/plans/2026-11-23-backend-v3-api.md`). Three kinds, as the contract names them:
//! - public: [`Core::public_call`];
//! - signed: [`Core::signed_call`], the payload signed with this install's Ed25519 key;
//! - jwt: [`Core::account_call`] and [`Core::device_call`], as the student this install proved
//!   to be. The JWT and refresh lifecycle live in one state machine ([`auth::AuthMachine`]):
//!   `auth:token` uses the refresh token kept in `device.json`, a v2 account token traded in once,
//!   the demo account, or a TLSNotary proof ([`crate::remote::account_proof`]); it is renewed
//!   before it runs out and whenever the server turns it down.
//!
//! Every call carries the flow id (`flow`). Arguments, answers, tokens and JWTs are never
//! logged.

mod auth;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use futures::channel::oneshot;
use futures::future::{BoxFuture, Either, select};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{info, warn};
use vtop_shared::device_sig;

use crate::convex::{CallError, ConvexClient, SubscriptionId};
use crate::device::EdgeError;
use crate::remote::account_token;
use crate::remote::server_code::ServerCode;
use crate::{Core, CoreError, Shared, unix_now};

use auth::{AuthMachine, Blocked, Effect, Grant, Stamp, Step, StepInput};

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
    /// Closes the underlying transport and stops background timers (default no-op for test doubles).
    fn close(&self) {}
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

    fn close(&self) {
        ConvexClient::close(self);
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
    /// The watch failed; what follows is in `error`.
    fn on_error(&self, error: LiveError);
}

/// Why a [`LiveListener`]'s watch failed, as the app acts on it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum LiveError {
    /// Renewed here (a refused JWT; the next change follows), or our own fault the student
    /// can't act on (the log has the reason).
    Transient,
    /// No identity left (signed out, this phone removed): the watch ends.
    NoIdentity { message: String },
    /// The server stopped following it for a reason the student may see.
    Stopped { message: String },
}

impl LiveError {
    fn of(error: EdgeError) -> Self {
        match error {
            EdgeError::Status {
                code:
                    ServerCode::Unauthenticated
                    | ServerCode::ServerError
                    | ServerCode::NotFound
                    | ServerCode::BadRequest,
                ..
            } => Self::Transient,
            EdgeError::Status {
                code: ServerCode::DeviceRemoved,
                message,
            } => Self::NoIdentity { message },
            error => Self::Stopped {
                message: error.message().to_string(),
            },
        }
    }
}

/// One [`Core::live_watch`]; stopping or dropping it unsubscribes.
#[derive(uniffi::Object)]
pub struct LiveWatch {
    backend: Arc<dyn Backend>,
    id: u64,
    /// While any is held, the JWT is renewed before it runs out ([`Renewal`]).
    held: Mutex<Option<Arc<()>>>,
    wake: Arc<Condvar>,
}

#[uniffi::export]
impl LiveWatch {
    /// No more calls to its listener once the socket's queue drains.
    pub fn stop(&self) {
        self.backend.unwatch(self.id);
        let dropped = self
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if dropped.is_some() {
            self.wake.notify_all();
        }
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
    /// student (a JWT, a refresh token or a v2 account token) and they shared their
    /// registration number, so a live watch can follow; read on the phone alone.
    pub async fn has_server_identity(&self, flow: String) -> bool {
        crate::traced("has_server_identity", &flow, async {
            let regno = self.signed_in_regno().await;
            if self
                .regno_allowed(&flow, regno.as_deref(), false)
                .await
                .is_err()
            {
                return Ok(false);
            }
            let server = self.forwarding_server();
            let refresh = self.device().refresh;
            let has_v2 = self.kept_v2(regno.as_deref()).is_some();
            Ok(self.shared.backend_state().auth.has_usable_identity(
                &server,
                regno.as_deref(),
                refresh.as_ref(),
                has_v2,
            ))
        })
        .await
        .unwrap_or(false)
    }

    /// Returns once [`Core::has_server_identity`] holds: at once, or when a proof, the student's
    /// Share or a sign-in brings an identity. The app's watch of session requests waits here.
    pub async fn wait_server_identity(&self, flow: String) {
        loop {
            // Registered before the check, so a change between the two still wakes it.
            let changed = self.shared.identity_change();
            if self.has_server_identity(flow.clone()).await {
                return;
            }
            // Dropped unsent only with the core: looking again is all there is to do.
            if changed.await.is_err() {
                info!(flow, "identity wait woken without a change");
            }
        }
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
                                kind: crate::ForwardingOffKind::NotSetUp,
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
            let stamp = self.ensure_identity(&flow, path, false).await?;
            let backend = self.backend()?;
            let held = Arc::new(());
            let (wake, start) = {
                let mut state = self.shared.backend_state();
                let start = state.renewal.watch(&held, &flow);
                (state.renewal.wake.clone(), start)
            };
            if start {
                run_renewal_coordinator(Arc::downgrade(&self), self.shared.clone());
            }
            let (core, renew_flow) = (Arc::downgrade(&self), flow.clone());
            // Tracks the JWT epoch this watch last succeeded under, so a rejected JWT invalidates
            // that epoch once and concurrent watches share one refresh.
            let seen_jwt_epoch = Arc::new(AtomicU64::new(stamp.jwt_epoch));
            let watch_gen = stamp.generation;
            let on_update = move |update: Result<Value, CallError>| match update {
                Ok(_) => {
                    if let Some(core) = core.upgrade() {
                        let current = core.shared.backend_state().auth.stamp().jwt_epoch;
                        seen_jwt_epoch.store(current, Ordering::Relaxed);
                    }
                    listener.on_change();
                }
                Err(error) => {
                    let error = edge_error(path, error);
                    if let Some(core) = core.upgrade() {
                        if error.is(&ServerCode::Unauthenticated) {
                            let failed_epoch = seen_jwt_epoch.load(Ordering::Relaxed);
                            core.on_jwt_rejected(Some(failed_epoch));
                            core.request_watch_renewal(&renew_flow, listener.clone());
                        }
                        // Removed on another phone: only act if this watch belongs to the current
                        // identity generation, so a late error never wipes a newer refresh token.
                        if error.is(&ServerCode::DeviceRemoved) {
                            if !core.on_device_removed(watch_gen) {
                                return;
                            }
                            warn!(path, gen = watch_gen, "this install was removed on the server");
                        }
                    }
                    listener.on_error(LiveError::of(error));
                }
            };
            let id = backend.watch(path, args, Box::new(on_update));
            info!(path, "live watch");
            Ok(Arc::new(LiveWatch {
                backend,
                id,
                held: Mutex::new(Some(held)),
                wake,
            }))
        })
        .await
    }
}

/// How often the live-watch renewal coordinator checks again while waiting.
const KEEP_CHECK: Duration = Duration::from_secs(30);

/// Phase of the single live-watch JWT renewal coordinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum RenewPhase {
    #[default]
    Idle,
    Scheduled,
    Renewing,
}

/// Next step decided by [`Renewal::decide`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum RenewDecision {
    Run { flow: String },
    Wait { duration: Duration, signal: u64 },
    Stop,
}

/// Single `Condvar`-driven coordinator for proactive JWT renewal and coalesced watch
/// `unauthenticated` recovery, replacing per-watch threads and `thread::sleep`.
struct Renewal {
    wake: Arc<Condvar>,
    phase: RenewPhase,
    watched: Vec<std::sync::Weak<()>>,
    /// Immediate renewal requested by one or more watches hearing `unauthenticated`.
    renew_now: bool,
    /// Flow id for the next renewal run.
    flow: String,
    /// Listeners waiting for the outcome of an immediate renewal (notified with `no_identity`
    /// only if no identity can be taken).
    pending: Vec<Arc<dyn LiveListener>>,
    /// Backoff deadline after a transient renewal failure.
    retry_after: Option<Instant>,
    /// Bumped on every wakeup signal so `wait_timeout` never misses a wake between checks.
    signal: u64,
    /// True while the single coordinator OS thread is alive.
    running: bool,
}

impl Default for Renewal {
    fn default() -> Self {
        Self {
            wake: Arc::new(Condvar::new()),
            phase: RenewPhase::Idle,
            watched: Vec::new(),
            renew_now: false,
            flow: String::new(),
            pending: Vec::new(),
            retry_after: None,
            signal: 0,
            running: false,
        }
    }
}

impl Renewal {
    /// Registers a held watch; returns true when the caller must start the coordinator thread.
    fn watch(&mut self, held: &Arc<()>, flow: &str) -> bool {
        self.watched.push(Arc::downgrade(held));
        if self.flow.is_empty() {
            self.flow = flow.to_owned();
        }
        self.signal += 1;
        if self.phase == RenewPhase::Idle {
            self.phase = RenewPhase::Scheduled;
        }
        let start = !self.running;
        self.running = true;
        self.wake.notify_all();
        start
    }

    /// Requests an immediate coalesced renewal for a watch that heard `unauthenticated`; returns
    /// true when the caller must start the coordinator thread.
    fn request_now(&mut self, flow: &str, listener: Arc<dyn LiveListener>) -> bool {
        self.renew_now = true;
        self.retry_after = None;
        self.flow = flow.to_owned();
        if !self.pending.iter().any(|kept| Arc::ptr_eq(kept, &listener)) {
            self.pending.push(listener);
        }
        self.signal += 1;
        if self.phase == RenewPhase::Idle {
            self.phase = RenewPhase::Scheduled;
        }
        let start = !self.running;
        self.running = true;
        self.wake.notify_all();
        start
    }

    /// Clears transient retry backoff and wakes the coordinator without spawning another thread.
    fn poke(&mut self) {
        self.retry_after = None;
        self.signal += 1;
        self.wake.notify_all();
    }

    /// Pure decision for the coordinator loop's next step.
    fn decide(&mut self, due_ms: Option<i64>, now_ms: i64, now: Instant) -> RenewDecision {
        self.watched.retain(|held| held.strong_count() > 0);
        if self.watched.is_empty() && !self.renew_now {
            self.phase = RenewPhase::Idle;
            self.retry_after = None;
            self.pending.clear();
            self.running = false;
            return RenewDecision::Stop;
        }
        if self.renew_now {
            self.renew_now = false;
            self.retry_after = None;
            self.phase = RenewPhase::Renewing;
            return RenewDecision::Run {
                flow: self.flow.clone(),
            };
        }
        if let Some(until) = self.retry_after {
            if until > now {
                self.phase = RenewPhase::Scheduled;
                return RenewDecision::Wait {
                    duration: until.saturating_duration_since(now).min(KEEP_CHECK),
                    signal: self.signal,
                };
            }
            self.retry_after = None;
        }
        if let Some(wait) = renew_in(due_ms, now_ms) {
            self.phase = RenewPhase::Scheduled;
            RenewDecision::Wait {
                duration: wait,
                signal: self.signal,
            }
        } else {
            self.phase = RenewPhase::Renewing;
            RenewDecision::Run {
                flow: self.flow.clone(),
            }
        }
    }

    /// Records the outcome of a renewal attempt; returns listeners to notify with `no_identity`
    /// when the coordinator stops for lack of identity.
    fn finish(
        &mut self,
        result: &Result<(), CoreError>,
        now: Instant,
    ) -> (bool, Vec<Arc<dyn LiveListener>>) {
        let pending = std::mem::take(&mut self.pending);
        match result {
            Ok(()) => {
                self.retry_after = Some(now + Duration::from_secs(1));
                self.phase = RenewPhase::Scheduled;
                (false, Vec::new())
            }
            Err(CoreError::NotSignedIn | CoreError::ConsentDenied | CoreError::RegnoNotShared) => {
                self.renew_now = false;
                self.retry_after = None;
                self.phase = RenewPhase::Idle;
                self.running = false;
                (true, pending)
            }
            Err(_) => {
                self.retry_after = Some(now + KEEP_CHECK);
                self.phase = RenewPhase::Scheduled;
                (false, Vec::new())
            }
        }
    }
}

/// Runs the single live-watch JWT renewal coordinator thread until no watches remain, no
/// identity is available, or the [`Core`] is dropped.
fn run_renewal_coordinator(core: std::sync::Weak<Core>, shared: Arc<Shared>) {
    let fallback_shared = shared.clone();
    let spawned = std::thread::Builder::new()
        .name("convex-keep-jwt".into())
        .spawn(move || {
            loop {
                let Some(strong) = core.upgrade() else {
                    let mut state = shared.backend_state();
                    state.renewal.phase = RenewPhase::Idle;
                    state.renewal.running = false;
                    return;
                };
                let now_ms = strong.server_now_ms();
                let now = Instant::now();
                let decision = {
                    let mut state = shared.backend_state();
                    let due = state.auth.renew_due_ms();
                    state.renewal.decide(due, now_ms, now)
                };
                match decision {
                    RenewDecision::Stop => return,
                    RenewDecision::Wait { duration, signal } => {
                        // Drop the strong Core reference before sleeping so Core can be dropped.
                        drop(strong);
                        let state = shared.backend_state();
                        if state.renewal.signal == signal {
                            let wake = state.renewal.wake.clone();
                            let (_guard, _timeout) = wake
                                .wait_timeout(state, duration)
                                .unwrap_or_else(PoisonError::into_inner);
                        }
                    }
                    RenewDecision::Run { flow } => {
                        let renewed = futures::executor::block_on(strong.identity(
                            &flow,
                            "live watch renewal",
                            false,
                        ));
                        match &renewed {
                            Ok(()) => {}
                            Err(
                                CoreError::NotSignedIn
                                | CoreError::ConsentDenied
                                | CoreError::RegnoNotShared,
                            ) => {
                                info!("live watches: no identity, stops following");
                            }
                            Err(error) => warn!(%error, "live watches: no new jwt"),
                        }
                        let (stop, notify) = shared
                            .backend_state()
                            .renewal
                            .finish(&renewed, Instant::now());
                        drop(strong);
                        for listener in notify {
                            listener.on_error(LiveError::NoIdentity {
                                message: "Sign in again to follow changes.".into(),
                            });
                        }
                        if stop {
                            return;
                        }
                    }
                }
            }
        });
    if let Err(error) = spawned {
        warn!(%error, "live watches: renewal not started");
        let mut state = fallback_shared.backend_state();
        state.renewal.phase = RenewPhase::Idle;
        state.renewal.running = false;
    }
}

/// How long the coordinator waits before checking again, for a JWT due for renewal at `due`
/// (none held: at once) at `now` (both unix ms); None: renew now.
fn renew_in(due: Option<i64>, now: i64) -> Option<Duration> {
    let left = due? - now;
    (left > 0).then(|| Duration::from_millis(left.unsigned_abs()).min(KEEP_CHECK))
}

/// The core's server state ([`Shared::backend`]).
#[derive(Default)]
pub(crate) struct State {
    backend: Option<Arc<dyn Backend>>,
    auth: AuthMachine,
    renewal: Renewal,
    /// The calls in [`Core::wait_server_identity`], woken when the identity may have come.
    identity_waiters: Vec<oneshot::Sender<()>>,
}

/// The refresh token `auth:token` handed this install. No `Debug`: it holds the token.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// How long a proof someone started may take, the ticket to the verdict: the verifier may queue
/// it for a minute before it starts (keeping the socket alive, sending nothing).
const PROOF_BUDGET: Duration = Duration::from_secs(150);

/// The last student's refresh token, set aside at a switch ([`Core::set_aside_identity`]).
pub(crate) const LAST_STUDENT: &str = "server_last_student.json";

/// Whether a sign-out failed because there is nothing left to sign out of.
pub(crate) fn signed_out_already(error: &EdgeError) -> bool {
    matches!(error, EdgeError::Status { code, .. } if code.is_signed_out_already())
}

impl Shared {
    fn backend_state(&self) -> std::sync::MutexGuard<'_, State> {
        self.backend.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Wakes the next time [`Shared::identity_may_have_changed`] runs.
    fn identity_change(&self) -> oneshot::Receiver<()> {
        let (wake, woken) = oneshot::channel();
        self.backend_state().identity_waiters.push(wake);
        woken
    }

    /// The identity, the registration number's answer or the signed-in student changed: the
    /// calls waiting for an identity look again.
    pub(crate) fn identity_may_have_changed(&self) {
        let waiters = std::mem::take(&mut self.backend_state().identity_waiters);
        for wake in waiters {
            #[expect(
                clippy::let_underscore_must_use,
                reason = "a waiter that went away needs no wake-up"
            )]
            let _ = wake.send(());
        }
    }

    /// The Convex socket itself got `AuthError` and already cleared its own auth token: drop the
    /// ready JWT in the state machine so the next call refreshes.
    fn on_socket_auth_error(&self) {
        let (_, effects) = self.backend_state().auth.on_jwt_rejected(None);
        if effects.contains(&Effect::ResetUdpJwt) {
            crate::remote::udp::forget_jwt_sent();
        }
    }
}

/// `{ code, error }` from what a function threw, in the edge's words.
pub(crate) fn edge_error(path: &str, error: CallError) -> EdgeError {
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
        let started = Instant::now();
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

    /// The signed-in student's registration number, upper case; None signed out.
    pub(crate) async fn signed_in_regno(&self) -> Option<String> {
        let slot = self.current().ok()?;
        let session = slot.read().await;
        // A restored or restarted session that hasn't loaded a page yet knows it from the saved
        // account (in memory or on disk).
        let regno = session.client.registration_number().ok();
        regno
            .or_else(|| session.account.registration_number.clone())
            .or_else(|| {
                self.read_account()
                    .filter(|saved| {
                        saved
                            .username
                            .eq_ignore_ascii_case(&session.account.username)
                    })
                    .and_then(|saved| saved.registration_number)
            })
            .or_else(|| crate::demo::is(&session.account).then(|| session.account.username.clone()))
            .map(|regno| regno.to_ascii_uppercase())
    }

    /// Runs the side effects emitted by [`AuthMachine`] outside the state lock.
    fn run_effects(&self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::SetSocketAuth(jwt) => {
                    let backend = if jwt.is_some() {
                        self.backend().ok()
                    } else {
                        self.shared.backend_state().backend.clone()
                    };
                    if let Some(backend) = backend {
                        backend.set_auth(jwt);
                    }
                }
                Effect::ResetUdpJwt => crate::remote::udp::forget_jwt_sent(),
                Effect::SaveRefresh(refresh) => self.keep_refresh(refresh),
                Effect::SetAsideRefresh(refresh) => self.write(LAST_STUDENT, &refresh),
                Effect::ForgetV2Token => self.forget_account_token(),
                Effect::ForgetForwarding => self.forget_any_forwarding(),
            }
        }
        self.shared.identity_may_have_changed();
    }

    /// Makes sure the client goes with a live JWT for the signed-in student, returning the
    /// [`Stamp`] it ran under so callers can ignore stale errors from an earlier identity.
    async fn ensure_identity(
        &self,
        flow: &str,
        action: &str,
        ask: bool,
    ) -> Result<Stamp, CoreError> {
        self.identity_within(flow, action, ask.then_some(PROOF_BUDGET), false)
            .await
            .map(|(stamp, _)| stamp)
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
        self.ensure_identity(flow, action, ask).await.map(drop)
    }

    /// [`Self::identity`], proving within `prove` when it must (None: never); returns
    /// `(stamp, got_new_jwt)`. With `background_only`, returns `Ok((stamp, false))` without
    /// network traffic whenever a usable identity or grant already exists.
    async fn identity_within(
        &self,
        flow: &str,
        action: &str,
        prove: Option<Duration>,
        background_only: bool,
    ) -> Result<(Stamp, bool), CoreError> {
        let (stamp, taken) = self
            .take_identity(flow, action, prove, background_only)
            .await?;
        if taken {
            self.leave_queued(flow).await;
            self.semester_to_server(flow).await;
        }
        Ok((stamp, taken))
    }

    /// The kept v2 account token for `regno`, if any.
    fn kept_v2(&self, regno: Option<&str>) -> Option<account_token::Saved> {
        self.read::<account_token::Saved>(account_token::FILE)
            .filter(|saved| {
                regno.is_some_and(|regno| regno.eq_ignore_ascii_case(&saved.registration_number))
            })
    }

    /// Unix ms on the server's clock, as the JWT's expiry is: a phone set minutes off would
    /// otherwise renew on every call, or hold on to an expired JWT.
    fn server_now_ms(&self) -> i64 {
        unix_now()
            .saturating_add(self.device().clock_offset)
            .saturating_mul(1000)
    }

    async fn take_identity(
        &self,
        flow: &str,
        action: &str,
        prove: Option<Duration>,
        background_only: bool,
    ) -> Result<(Stamp, bool), CoreError> {
        // Before the turn: the sheet may wait on the student, and nothing names them until
        // they said Share. A background run or a call nobody started never asks.
        let may_ask = prove.is_some() && !background_only;
        let asked_for = self.signed_in_regno().await;
        self.regno_allowed(flow, asked_for.as_deref(), may_ask)
            .await?;
        let _turn = self.shared.identity_turn.lock().await;
        // Another student may have signed in while this one waited.
        let regno = self.signed_in_regno().await;
        if regno != asked_for {
            self.regno_allowed(flow, regno.as_deref(), false).await?;
        }
        let server = self.forwarding_server();
        if background_only {
            let Some(regno_str) = regno.as_deref() else {
                return Ok((self.shared.backend_state().auth.stamp(), false));
            };
            let refresh = self.device().refresh;
            let has_v2 = self.kept_v2(Some(regno_str)).is_some();
            let state = self.shared.backend_state();
            if regno_str.eq_ignore_ascii_case(crate::demo::USERNAME)
                || state.auth.has_usable_identity(
                    &server,
                    Some(regno_str),
                    refresh.as_ref(),
                    has_v2,
                )
            {
                return Ok((state.auth.stamp(), false));
            }
        }
        for _ in 0..3 {
            let now_ms = self.server_now_ms();
            let refresh = self.device().refresh;
            let v2 = self.kept_v2(regno.as_deref());
            let (step, effects) = {
                let mut state = self.shared.backend_state();
                state.auth.next_step(&StepInput {
                    server: &server,
                    regno: regno.as_deref(),
                    refresh: refresh.as_ref(),
                    v2_token: v2.as_ref().map(|saved| saved.token.as_str()),
                    allow_proof: prove.is_some(),
                    now_ms,
                })
            };
            self.run_effects(effects);
            let grant = match step {
                Step::Ready(stamp) => return Ok((stamp, false)),
                Step::Blocked(Blocked::NotSignedIn) => {
                    info!(action, signed_in = false, "no identity: not proving");
                    return Err(CoreError::NotSignedIn);
                }
                Step::Blocked(Blocked::ConsentDenied) => {
                    info!(
                        action,
                        signed_in = regno.is_some(),
                        "no identity: not proving"
                    );
                    return Err(CoreError::ConsentDenied);
                }
                Step::FetchToken(grant) => grant,
            };
            let source = grant.kind();
            let payload = match grant {
                Grant::Demo => json!({ "demo": true, "device": self.device_info() }),
                Grant::Refresh(refresh) => json!({ "refreshToken": refresh.token }),
                Grant::V2(token) => {
                    json!({ "v2AccountToken": token, "device": self.device_info() })
                }
                Grant::Proof => {
                    let Some(budget) = prove else {
                        return Err(CoreError::ConsentDenied);
                    };
                    info!(action, "no identity: proving");
                    let ticket = self.prove_account(flow, budget).await?;
                    json!({ "ticket": ticket, "device": self.device_info() })
                }
            };
            let asked = Instant::now();
            let issued = self
                .signed_call(flow, Kind::Action, "auth:token", &payload)
                .await;
            let token_ms = asked.elapsed().as_millis() as u64;
            let issued = match issued {
                Ok(issued) => issued,
                Err(error) => {
                    let (retry, effects) = self
                        .shared
                        .backend_state()
                        .auth
                        .on_token_refused(source, &error);
                    if retry {
                        warn!(
                            source = source.as_str(),
                            code = error.code(),
                            "token refused"
                        );
                        self.run_effects(effects);
                        continue;
                    }
                    return Err(error.into());
                }
            };
            let issued: Issued = serde_json::from_value(issued).map_err(|_| CoreError::Other {
                reason: "The server sent an unexpected answer.".into(),
            })?;
            // ms; seconds from a server that counts them.
            let expires_at = match issued.jwt_expires_at {
                at if at < 10_000_000_000 => at.saturating_mul(1000),
                at => at,
            };
            let (stamp, effects) = self.shared.backend_state().auth.on_issued(
                source,
                &server,
                &issued.regno,
                issued.jwt,
                expires_at,
                issued.refresh_token,
            );
            self.run_effects(effects);
            // Signed out (leaving an old address), the lines still name the student.
            if regno.is_none() {
                crate::obs::set_regno(Some(issued.regno.to_ascii_uppercase()));
            }
            info!(
                source = source.as_str(),
                token_ms,
                expires_in_s = (expires_at - now_ms) / 1000,
                "server identity"
            );
            return Ok((stamp, true));
        }
        Err(CoreError::Other {
            reason: "The server turned down this phone's sign-in.".into(),
        })
    }

    /// Marks the current JWT stale (if `jwt_epoch` matches) and clears socket auth.
    pub(crate) fn on_jwt_rejected(&self, jwt_epoch: Option<u64>) {
        let (_, effects) = self.shared.backend_state().auth.on_jwt_rejected(jwt_epoch);
        self.run_effects(effects);
    }

    /// Handles a `device_removed` response from `generation`; returns true when it applied.
    pub(crate) fn on_device_removed(&self, generation: u64) -> bool {
        let (applied, effects) = self
            .shared
            .backend_state()
            .auth
            .on_device_removed(generation);
        self.run_effects(effects);
        applied
    }

    /// Coalesces an immediate watch renewal onto the single coordinator thread.
    fn request_watch_renewal(self: &Arc<Self>, flow: &str, listener: Arc<dyn LiveListener>) {
        let start = self
            .shared
            .backend_state()
            .renewal
            .request_now(flow, listener);
        if start {
            run_renewal_coordinator(Arc::downgrade(self), self.shared.clone());
        }
    }

    /// Developer changed the server URL: drops the old client and active JWT while leaving the
    /// single renewal coordinator intact.
    pub(crate) fn on_server_changed(&self) {
        let (old, effects) = {
            let mut state = self.shared.backend_state();
            let old = state.backend.take();
            state.renewal.poke();
            (old, state.auth.on_server_changed())
        };
        if let Some(old) = old {
            old.close();
        }
        self.run_effects(effects);
    }

    /// Another student signed in over the last one: the last one's refresh token is kept aside
    /// for [`Core::sign_out_last_student`], and this install has no identity until the new
    /// student's.
    pub(crate) fn set_aside_identity(&self) {
        let current = self.device().refresh;
        let effects = self.shared.backend_state().auth.on_set_aside(current);
        self.run_effects(effects);
    }

    /// Signs this install out of the last student's account with their refresh token, kept
    /// aside at a switch, so it stops getting their OTPs and session requests before the new
    /// student ever proves. Tried again on the next call while the server can't be reached.
    pub(crate) async fn sign_out_last_student(&self, flow: &str) {
        let Some(last) = self.read::<Refresh>(LAST_STUDENT) else {
            return;
        };
        if auth::normalize_server(&last.server) != auth::normalize_server(&self.forwarding_server())
        {
            self.remove_file(LAST_STUDENT);
            return;
        }
        // The socket goes with their JWT for this one call: no other identity can be taken
        // meanwhile, and the current student's JWT (if any) is restored afterwards.
        let _turn = self.shared.identity_turn.lock().await;
        let body = json!({ "refreshToken": last.token });
        let issued = match self
            .signed_call(flow, Kind::Action, "auth:token", &body)
            .await
        {
            Ok(issued) => issued,
            Err(error) if signed_out_already(&error) => {
                info!(code = error.code(), "last student already signed out here");
                self.remove_file(LAST_STUDENT);
                return;
            }
            Err(error) => {
                info!(code = error.code(), "last student not signed out yet");
                return;
            }
        };
        // Persist the rotated refresh token before calling `auth:signOut`, so an offline failure
        // retries with the newest token instead of reusing the replaced one.
        if let Some(rotated) = issued.get("refreshToken").and_then(Value::as_str) {
            self.write(
                LAST_STUDENT,
                &Refresh {
                    server: last.server,
                    token: rotated.to_owned(),
                    regno: last.regno,
                },
            );
        }
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
        let restore = self.shared.backend_state().auth.socket_jwt();
        backend.set_auth(restore);
        match signed_out {
            Ok(_) => {
                info!("signed out of the last student's account");
                self.remove_file(LAST_STUDENT);
            }
            Err(error) => {
                let code = edge_error("auth:signOut", error).code().to_owned();
                info!(code, "last student not signed out yet");
            }
        }
    }

    /// The JWT the client goes with, while it has a minute left: the UDP gateway signs in with
    /// it when the session it holds expires.
    pub(crate) fn live_jwt(&self) -> Option<String> {
        let now = self.server_now_ms();
        self.shared.backend_state().auth.live_jwt(now)
    }

    /// Drops the JWT and the refresh token (signed out of the server; another student; this
    /// install removed there). The v2 token goes with the student's files.
    pub(crate) fn forget_identity(&self) {
        let effects = self.shared.backend_state().auth.on_forget("explicit");
        self.run_effects(effects);
    }
}

#[uniffi::export]
impl Core {
    /// Proves who the student is in the background, so the refresh token is there before
    /// Connected apps, forwarding or a server route needs it: right after a sign-in, and again
    /// at each app open until it worked. Only when this install has no identity to use (no
    /// refresh or v2 token, no live JWT) and is signed in; it gives up after `timeout_ms` and
    /// the next open tries again. It takes the identity's turn, so a call someone starts
    /// meanwhile waits for it and uses its JWT instead of proving again. No session leaves the
    /// phone in a proof, so no cookie consent is asked; it proves only once the student said
    /// Share to their registration number ([`crate::remote::regno`]), and never asks. Kotlin runs it in an unattended flow, on an
    /// unmetered or validated non-cellular network unless Developer allows mobile data. True
    /// when it proved.
    pub async fn prove_in_background(
        &self,
        flow: String,
        timeout_ms: u32,
    ) -> Result<bool, CoreError> {
        crate::traced("prove_in_background", &flow, async {
            let budget = Duration::from_millis(timeout_ms.into());
            match self
                .identity_within(&flow, "Background proof", Some(budget), true)
                .await
            {
                // Not until the student said Share; regno_allowed logged it.
                Err(CoreError::RegnoNotShared) => Ok(false),
                proved => proved.map(|(_, taken)| taken),
            }
        })
        .await
    }
}

#[cfg(not(feature = "tlsn"))]
impl Core {
    async fn prove_account(&self, _: &str, _: Duration) -> Result<String, CoreError> {
        Err(CoreError::Other {
            reason: "This build cannot prove who you are to the server.".into(),
        })
    }
}

#[cfg(test)]
mod tests;

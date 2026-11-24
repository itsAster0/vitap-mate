//! Drives `BaseConvexClient` with no async runtime. The socket's callbacks (the app's socket
//! threads) run under one mutex; one timer thread per client reconnects with backoff and drops a
//! silent socket. Socket calls (open, send, disconnect) never run under the mutex: they queue as
//! [`Io`], with the subscribers' updates, and one thread at a time runs the queue in order, so a
//! socket that calls its listener from inside them cannot deadlock the client and every
//! subscriber hears its results in the order they came.
//!
//! An open socket with no call or subscription for [`IDLE_MS`] closes, and the next call opens
//! another, so a phone in the background isn't woken by pings.
//!
//! Auth is handed in ([`ConvexClient::set_auth`]); the client keeps it across reconnects and,
//! when the server rejects it, drops it, reconnects without it and tells [`ConvexClient::new`]'s
//! `on_auth_error`, so the owner fetches a new one.

use std::collections::hash_map::RandomState;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::hash::{BuildHasher, Hasher};
use std::mem;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use convex_sync_types::{AuthenticationToken, ClientMessage, SessionId, UdfPath};
use futures::channel::oneshot;
use serde_json::Value;
use tracing::{Span, info, info_span, warn};

use super::base_client::{BaseConvexClient, FunctionResult, ServerMessage, SubscriberId};
use super::value::ConvexValue;
use super::{ConvexNetwork, ConvexSocket, ConvexSocketListener};

const INITIAL_BACKOFF_MS: u64 = 500;
const MAX_BACKOFF_MS: u64 = 30_000;
/// Convex pings every ~15 s; a socket quiet for this long is dead.
const SILENCE_MS: u64 = 30_000;
/// An open socket with nothing to do closes after this.
const IDLE_MS: u64 = 60_000;
/// The timer thread looks again at least this often (and lets go of a dropped client).
const MAX_TIMER_WAIT: Duration = Duration::from_secs(5);

/// Why a call has no value. Never holds arguments; `Convex` holds what the function threw.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CallError {
    /// The client closed before the answer came.
    Closed,
    /// The function failed without a `ConvexError` (a bug, or a bad path or arguments).
    Message(String),
    /// `ConvexError(data)`: `{ code, error, ... }` by the contract.
    Convex { message: String, data: Value },
}

/// Monotonic milliseconds and backoff jitter, so tests can drive time.
pub(crate) trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
    /// A delay of about `ms`, never more.
    fn jitter(&self, ms: u64) -> u64;
}

pub(crate) struct SystemClock(Instant);

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        self.0.elapsed().as_millis() as u64
    }

    /// Between half of `ms` and all of it.
    fn jitter(&self, ms: u64) -> u64 {
        ms / 2 + random_u64() % (ms / 2 + 1)
    }
}

/// One subscriber's handle, for [`ConvexClient::unsubscribe`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SubscriptionId(pub(crate) u64);

type Update = Result<Value, CallError>;
type Callback = Arc<dyn Fn(Update) + Send + Sync>;

struct Subscription {
    subscriber: SubscriberId,
    on_update: Callback,
    /// The last result handed over, so only changes are.
    last: Option<FunctionResult>,
}

/// A socket call or an update, run outside the lock.
enum Io {
    Open {
        epoch: u64,
    },
    /// A frame for socket `epoch`; dropped if that socket is gone by then.
    Send {
        epoch: u64,
        text: String,
    },
    Close(Arc<dyn ConvexSocket>),
    Update(Callback, Update),
    AuthError,
}

/// Connection lifecycle of the single Convex WebSocket.
enum Link {
    /// No socket and nothing scheduled; the next call wakes it via `Backoff { at_ms: now }`.
    Idle,
    /// Waiting until `at_ms` before opening the next socket.
    Backoff { at_ms: u64 },
    /// Opening socket `epoch`; `socket` arrives when `ConvexNetwork::open` returns, and `opened`
    /// records a synchronous `on_open` callback that arrived before `open` returned.
    Opening {
        epoch: u64,
        socket: Option<Arc<dyn ConvexSocket>>,
        opened: bool,
    },
    /// Socket `epoch` is open and carrying Convex sync frames.
    Open {
        epoch: u64,
        socket: Arc<dyn ConvexSocket>,
    },
    /// Permanently closed; no further calls or reconnects.
    Closed,
}

impl Link {
    fn is_live(&self) -> bool {
        matches!(self, Self::Opening { .. } | Self::Open { .. })
    }

    fn is_open(&self) -> bool {
        matches!(self, Self::Open { .. })
    }

    fn is_closed(&self) -> bool {
        matches!(self, Self::Closed)
    }

    fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    fn reconnect_at(&self) -> Option<u64> {
        match self {
            Self::Backoff { at_ms } => Some(*at_ms),
            _ => None,
        }
    }

    fn epoch(&self) -> Option<u64> {
        match self {
            Self::Opening { epoch, .. } | Self::Open { epoch, .. } => Some(*epoch),
            _ => None,
        }
    }

    fn has_epoch(&self, epoch: u64) -> bool {
        self.epoch() == Some(epoch)
    }

    fn socket_for(&self, epoch: u64) -> Option<Arc<dyn ConvexSocket>> {
        match self {
            Self::Opening {
                epoch: current,
                socket: Some(socket),
                ..
            }
            | Self::Open {
                epoch: current,
                socket,
            } if *current == epoch => Some(socket.clone()),
            _ => None,
        }
    }

    fn take_socket(&mut self) -> Option<Arc<dyn ConvexSocket>> {
        match self {
            Self::Opening { socket, .. } => socket.take(),
            Self::Open { .. } => {
                let prev = mem::replace(self, Self::Idle);
                match prev {
                    Self::Open { socket, .. } => Some(socket),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

struct State {
    base: BaseConvexClient,
    session_id: SessionId,
    link: Link,
    /// Monotonic socket epoch; a listener's calls count only while its epoch matches `link`.
    next_epoch: u64,
    connection_count: u32,
    last_close_reason: String,
    backoff_ms: u64,
    last_server_ms: u64,
    /// When a call was last made or a socket last opened.
    last_used_ms: u64,
    /// Calls waiting for their answer.
    pending: usize,
    next_subscription: u64,
    subscriptions: HashMap<SubscriptionId, Subscription>,
    /// Socket calls and updates waiting to run, in order.
    io: VecDeque<Io>,
    /// A thread is running `io`; others leave theirs to it.
    draining: bool,
    /// Something changed that the timer thread should look at.
    woken: bool,
    /// A new identity was sent and no transition has come since: a cached failure may be the
    /// old identity's (an expired JWT's `unauthenticated`), so it isn't handed out.
    auth_changed: bool,
}

pub(crate) struct ConvexClient {
    url: String,
    network: Arc<dyn ConvexNetwork>,
    on_auth_error: Box<dyn Fn() + Send + Sync>,
    clock: Arc<dyn Clock>,
    state: Mutex<State>,
    wake: Condvar,
    span: Span,
    me: Weak<ConvexClient>,
}

impl ConvexClient {
    /// A client for the deployment at `url`, connecting at once on its own timer thread.
    pub(crate) fn new(
        url: &str,
        network: Arc<dyn ConvexNetwork>,
        on_auth_error: impl Fn() + Send + Sync + 'static,
    ) -> Arc<Self> {
        let clock = Arc::new(SystemClock(Instant::now()));
        Self::with_clock(url, network, Box::new(on_auth_error), clock, true)
    }

    /// Without `timer`, nothing happens on time until [`Self::tick`] is called.
    pub(crate) fn with_clock(
        url: &str,
        network: Arc<dyn ConvexNetwork>,
        on_auth_error: Box<dyn Fn() + Send + Sync>,
        clock: Arc<dyn Clock>,
        timer: bool,
    ) -> Arc<Self> {
        let now = clock.now_ms();
        let client = Arc::new_cyclic(|me| Self {
            url: sync_url(url),
            network,
            on_auth_error,
            clock,
            state: Mutex::new(State {
                base: BaseConvexClient::new(),
                session_id: random_session_id(),
                link: Link::Backoff { at_ms: now },
                next_epoch: 0,
                connection_count: 0,
                last_close_reason: "InitialConnect".into(),
                backoff_ms: INITIAL_BACKOFF_MS,
                last_server_ms: now,
                last_used_ms: now,
                pending: 0,
                next_subscription: 0,
                subscriptions: HashMap::new(),
                io: VecDeque::new(),
                draining: false,
                woken: false,
                auth_changed: false,
            }),
            wake: Condvar::new(),
            // One client serves every flow: its own lines belong to none of them (each call's
            // line is logged in its flow by `remote::backend`).
            span: info_span!(parent: None, "convex", src = "convex"),
            me: me.clone(),
        });
        if timer {
            let me = Arc::downgrade(&client);
            let spawned = std::thread::Builder::new()
                .name("convex-timer".into())
                .spawn(move || run_timer(me));
            if let Err(error) = spawned {
                client
                    .span
                    .in_scope(|| warn!(%error, "convex timer not started"));
            }
        }
        client
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Lets go of the lock, wakes the timer thread, and runs queued socket calls and updates.
    fn finish(&self, state: MutexGuard<'_, State>) {
        self.finish_quietly(state, true);
    }

    fn finish_quietly(&self, mut state: MutexGuard<'_, State>, wake: bool) {
        state.woken |= wake;
        let drain = !state.draining && !state.io.is_empty();
        state.draining |= drain;
        drop(state);
        if wake {
            self.wake.notify_all();
        }
        if drain {
            self.drain();
        }
    }

    /// Runs queued socket calls and updates in order, without the lock, until none are left.
    fn drain(&self) {
        /// A foreign call that panics must not leave the queue to a thread that is gone.
        /// Only under `panic = "unwind"` (tests): the release build aborts on a panic.
        struct Draining<'a>(&'a ConvexClient);
        impl Drop for Draining<'_> {
            fn drop(&mut self) {
                if std::thread::panicking() {
                    self.0.state().draining = false;
                }
            }
        }
        let _draining = Draining(self);
        loop {
            let mut state = self.state();
            let Some(io) = state.io.pop_front() else {
                state.draining = false;
                return;
            };
            match io {
                Io::Open { epoch } => {
                    if !matches!(state.link, Link::Opening { epoch: e, .. } if e == epoch) {
                        continue;
                    }
                    drop(state);
                    let listener = Arc::new(Listener {
                        client: self.me.clone(),
                        epoch,
                    });
                    let socket = self.network.open(self.url.clone(), listener);
                    let mut state = self.state();
                    match &mut state.link {
                        Link::Opening {
                            epoch: current,
                            socket: slot,
                            opened,
                        } if *current == epoch => {
                            if *opened {
                                // `on_open` fired synchronously inside `network.open`: complete
                                // the transition now that the socket handle is stored.
                                state.link = Link::Open { epoch, socket };
                                self.enter_open(&mut state);
                            } else {
                                *slot = Some(socket);
                            }
                        }
                        _ => {
                            drop(state);
                            socket.disconnect();
                        }
                    }
                }
                Io::Send { epoch, text } => {
                    let Some(socket) = state.link.socket_for(epoch) else {
                        continue;
                    };
                    drop(state);
                    if !socket.send(text) {
                        let mut state = self.state();
                        if state.link.has_epoch(epoch) {
                            let now = self.clock.now_ms();
                            self.drop_socket(&mut state, now, "SendFailed".into());
                            state.woken = true;
                            drop(state);
                            self.wake.notify_all();
                        }
                    }
                }
                Io::Close(socket) => {
                    drop(state);
                    socket.disconnect();
                }
                Io::Update(on_update, update) => {
                    drop(state);
                    on_update(update);
                }
                Io::AuthError => {
                    drop(state);
                    (self.on_auth_error)();
                }
            }
        }
    }

    /// Does whatever is due and returns when to call again (None: nothing will be due).
    pub(crate) fn tick(&self) -> Option<u64> {
        let _entered = self.span.enter();
        let now = self.clock.now_ms();
        let mut state = self.state();
        if state.link.is_closed() {
            return None;
        }
        if state.link.is_live() && now >= state.last_server_ms + SILENCE_MS {
            self.drop_socket(&mut state, now, "InactiveServer".into());
        }
        let busy = state.pending > 0 || !state.subscriptions.is_empty();
        if state.link.is_open() && !busy && now >= state.last_used_ms + IDLE_MS {
            info!("convex socket idle");
            Self::close_socket(&mut state, Link::Idle);
            state.last_close_reason = "Idle".into();
        }
        if matches!(state.link, Link::Backoff { at_ms } if now >= at_ms) {
            self.open_socket(&mut state, now);
        }
        let silence = state
            .link
            .is_live()
            .then_some(state.last_server_ms + SILENCE_MS);
        let reconnect = state.link.reconnect_at();
        let idle = (state.link.is_open() && !busy).then_some(state.last_used_ms + IDLE_MS);
        let next = silence.into_iter().chain(reconnect).chain(idle).min();
        self.finish_quietly(state, false);
        next
    }

    /// Sends `jwt` as this client's identity from now on, on this socket and every later one;
    /// None signs out.
    pub(crate) fn set_auth(&self, jwt: Option<String>) {
        let _entered = self.span.enter();
        let mut state = self.state();
        let token = jwt.map_or(AuthenticationToken::None, AuthenticationToken::User);
        state.base.set_auth(token);
        state.auth_changed = true;
        self.flush(&mut state);
        self.finish(state);
    }

    /// A call is made: an idle client opens a socket again.
    fn use_now(&self, state: &mut State) {
        let now = self.clock.now_ms();
        state.last_used_ms = now;
        if state.link.is_idle() {
            state.link = Link::Backoff { at_ms: now };
        }
    }

    fn next_backoff(&self, state: &mut State) -> u64 {
        let delay = self.clock.jitter(state.backoff_ms);
        state.backoff_ms = (state.backoff_ms * 2).min(MAX_BACKOFF_MS);
        delay
    }

    /// Queues the next socket's opening.
    fn open_socket(&self, state: &mut State, now: u64) {
        state.next_epoch += 1;
        let epoch = state.next_epoch;
        info!(connection = state.connection_count, "convex socket opening");
        state.io.push_back(Io::Open { epoch });
        state.link = Link::Opening {
            epoch,
            socket: None,
            opened: false,
        };
        state.last_server_ms = now;
    }

    /// Closes the socket, if any, and plans the next one after the backoff.
    fn drop_socket(&self, state: &mut State, now: u64, reason: String) {
        info!(%reason, "convex socket closed");
        let delay = self.next_backoff(state);
        Self::close_socket(state, Link::Backoff { at_ms: now + delay });
        state.last_close_reason = reason;
    }

    /// Closes the socket and schedules an immediate reconnect without inflating `backoff_ms`
    /// (for `AuthError`, where the server dropped the socket over a stale JWT, not a link fault).
    fn reconnect_now(&self, state: &mut State, now: u64, reason: String) {
        info!(%reason, "convex socket closed");
        Self::close_socket(state, Link::Backoff { at_ms: now });
        state.last_close_reason = reason;
    }

    fn close_socket(state: &mut State, next: Link) {
        if let Some(socket) = state.link.take_socket() {
            state.io.push_back(Io::Close(socket));
        }
        state.next_epoch += 1;
        state.link = next;
        // Whatever was queued is sent again after the next Connect.
        while state.base.pop_next_message().is_some() {}
    }

    /// Sends what the base client queued, while the socket is open.
    fn flush(&self, state: &mut State) {
        if !state.link.is_open() {
            return;
        }
        while let Some(message) = state.base.pop_next_message() {
            Self::send(state, message);
        }
    }

    /// Queues `message` for the current socket.
    fn send(state: &mut State, message: ClientMessage) {
        let Some(epoch) = state.link.epoch() else {
            return;
        };
        let kind: &str = message.as_ref();
        let kind = kind.to_owned();
        let text = match Value::try_from(message) {
            Ok(json) => json.to_string(),
            Err(error) => {
                warn!(%kind, %error, "convex frame not written");
                return;
            }
        };
        state.io.push_back(Io::Send { epoch, text });
    }

    /// Runs the Convex `Connect` handshake and flushes ongoing work once `state.link` is `Open`.
    fn enter_open(&self, state: &mut State) {
        info!("convex socket open");
        state.last_used_ms = self.clock.now_ms();
        while state.base.pop_next_message().is_some() {}
        let connect = ClientMessage::Connect {
            session_id: state.session_id,
            connection_count: state.connection_count,
            last_close_reason: state.last_close_reason.clone(),
            max_observed_timestamp: state.base.max_observed_timestamp(),
            client_ts: Some(unix_ms()),
        };
        state.connection_count += 1;
        // A watch that failed (its JWT ran out while offline) hears the same error again on
        // this socket, so its owner renews instead of waiting on a change that never comes.
        for subscription in state.subscriptions.values_mut() {
            if !matches!(subscription.last, Some(FunctionResult::Value(_))) {
                subscription.last = None;
            }
        }
        Self::send(state, connect);
        state.base.resend_ongoing_queries_mutations();
        self.flush(state);
    }

    fn on_open(&self, epoch: u64) {
        let _entered = self.span.enter();
        let mut state = self.state();
        match &mut state.link {
            Link::Opening {
                epoch: current,
                socket,
                opened,
            } if *current == epoch => {
                if let Some(socket) = socket.take() {
                    state.link = Link::Open { epoch, socket };
                    self.enter_open(&mut state);
                    self.finish(state);
                } else {
                    *opened = true;
                }
            }
            _ => {}
        }
    }

    fn on_text(&self, epoch: u64, text: String) {
        let _entered = self.span.enter();
        let mut state = self.state();
        if !state.link.has_epoch(epoch) {
            return;
        }
        let now = self.clock.now_ms();
        state.last_server_ms = now;
        let message = serde_json::from_str::<Value>(&text)
            .map_err(anyhow::Error::from)
            .and_then(ServerMessage::try_from);
        match message {
            // The error can quote the frame: only that it failed is logged.
            Err(_) => {
                warn!("convex frame unreadable");
                self.drop_socket(&mut state, now, "ProtocolError".into());
            }
            Ok(ServerMessage::AuthError { .. }) => {
                // The server closes after it; the next socket goes without the token, and the
                // owner hands in a new one. Reconnect at once without inflating backoff_ms.
                warn!("convex token rejected");
                state.base.set_auth(AuthenticationToken::None);
                self.reconnect_now(&mut state, now, "AuthError".into());
                state.io.push_back(Io::AuthError);
            }
            Ok(message) => {
                let transition = matches!(message, ServerMessage::Transition { .. });
                match state.base.receive_message(message) {
                    Ok(updated) => {
                        // A server that only fails or pings keeps the backoff growing.
                        if transition {
                            state.backoff_ms = INITIAL_BACKOFF_MS;
                            state.auth_changed = false;
                        }
                        if updated.is_some() {
                            queue_updates(&mut state);
                        }
                    }
                    Err(reason) => {
                        warn!(%reason, "convex protocol restarts");
                        self.drop_socket(&mut state, now, reason);
                    }
                }
            }
        }
        self.finish(state);
    }

    fn on_closed(&self, epoch: u64, reason: String) {
        let _entered = self.span.enter();
        let mut state = self.state();
        if !state.link.has_epoch(epoch) {
            return;
        }
        let now = self.clock.now_ms();
        self.drop_socket(&mut state, now, reason);
        self.finish(state);
    }

    /// Follows the query `path(args)`; `on_update` gets each new result, on the socket's thread
    /// (or this one, for a result already known).
    pub(crate) fn subscribe(
        &self,
        path: &str,
        args: Value,
        on_update: impl Fn(Update) + Send + Sync + 'static,
    ) -> SubscriptionId {
        let _entered = self.span.enter();
        let mut state = self.state();
        let id = SubscriptionId(state.next_subscription);
        state.next_subscription += 1;
        let on_update: Callback = Arc::new(on_update);
        let request = match state.link.is_closed() {
            true => Err(CallError::Closed),
            false => parse_path(path).and_then(|path| Ok((path, parse_args(args)?))),
        };
        let (path, args) = match request {
            Ok(request) => request,
            Err(error) => {
                state.io.push_back(Io::Update(on_update, Err(error)));
                self.finish(state);
                return id;
            }
        };
        self.use_now(&mut state);
        let subscriber = state.base.subscribe(path, args);
        let last = state
            .base
            .latest_results()
            .get(&subscriber)
            .filter(|result| !state.auth_changed || matches!(result, FunctionResult::Value(_)))
            .cloned();
        if let Some(result) = &last {
            let update = Io::Update(on_update.clone(), outcome_of(result.clone()));
            state.io.push_back(update);
        }
        state.subscriptions.insert(
            id,
            Subscription {
                subscriber,
                on_update,
                last,
            },
        );
        self.flush(&mut state);
        self.finish(state);
        id
    }

    pub(crate) fn unsubscribe(&self, id: SubscriptionId) {
        let _entered = self.span.enter();
        let mut state = self.state();
        if let Some(subscription) = state.subscriptions.remove(&id) {
            state.base.unsubscribe(subscription.subscriber);
            self.flush(&mut state);
        }
        self.finish(state);
    }

    /// Closed: it makes no more calls.
    #[cfg(test)]
    pub(crate) fn closed(&self) -> bool {
        self.state().link.is_closed()
    }

    /// A query's current result: followed until its first result, then dropped.
    pub(crate) async fn query(&self, path: &str, args: Value) -> Update {
        let (sender, receiver) = oneshot::channel();
        let sender = Mutex::new(Some(sender));
        let id = self.subscribe(path, args, move |update| {
            if let Some(sender) = sender.lock().unwrap_or_else(PoisonError::into_inner).take() {
                // The caller gave up (timed out): nobody wants the answer.
                #[expect(clippy::let_underscore_must_use, reason = "see above")]
                let _ = sender.send(update);
            }
        });
        /// Unsubscribes however the read ends (a caller that times out drops it).
        struct Unsubscribe<'a>(&'a ConvexClient, SubscriptionId);
        impl Drop for Unsubscribe<'_> {
            fn drop(&mut self) {
                self.0.unsubscribe(self.1);
            }
        }
        let _unsubscribe = Unsubscribe(self, id);
        receiver.await.unwrap_or(Err(CallError::Closed))
    }

    /// Runs a mutation; it resolves once the server's state includes it.
    pub(crate) async fn mutation(&self, path: &str, args: Value) -> Update {
        self.request(path, args, false).await
    }

    /// Runs an action; it resolves on its response.
    pub(crate) async fn action(&self, path: &str, args: Value) -> Update {
        self.request(path, args, true).await
    }

    async fn request(&self, path: &str, args: Value, action: bool) -> Update {
        let path = parse_path(path)?;
        let args = parse_args(args)?;
        let receiver = {
            let _entered = self.span.enter();
            let mut state = self.state();
            if state.link.is_closed() {
                return Err(CallError::Closed);
            }
            let receiver = if action {
                state.base.action(path, args)
            } else {
                state.base.mutation(path, args)
            };
            self.use_now(&mut state);
            state.pending += 1;
            self.flush(&mut state);
            self.finish(state);
            receiver
        };
        /// Counts the call as answered however it ends (a caller that times out drops it).
        struct Pending<'a>(&'a ConvexClient);
        impl Drop for Pending<'_> {
            fn drop(&mut self) {
                let mut state = self.0.state();
                state.pending = state.pending.saturating_sub(1);
                state.last_used_ms = self.0.clock.now_ms();
            }
        }
        let _pending = Pending(self);
        match receiver.await {
            Ok(result) => outcome_of(result),
            Err(_canceled) => Err(CallError::Closed),
        }
    }

    /// Closes the socket, fails pending calls and stops the timer thread.
    pub(crate) fn close(&self) {
        let _entered = self.span.enter();
        let mut state = self.state();
        if state.link.is_closed() {
            return;
        }
        info!("convex client closed");
        Self::close_socket(&mut state, Link::Closed);
        state.base.cancel_requests();
        for subscription in mem::take(&mut state.subscriptions).into_values() {
            let update = Io::Update(subscription.on_update, Err(CallError::Closed));
            state.io.push_back(update);
        }
        self.finish(state);
    }
}

impl Drop for ConvexClient {
    fn drop(&mut self) {
        let state = self.state.get_mut().unwrap_or_else(PoisonError::into_inner);
        if let Some(socket) = state.link.take_socket() {
            socket.disconnect();
        }
    }
}

/// Ticks on time until the client closes or is dropped. A foreign callback that panics is
/// logged and tried again after the backoff.
/// The catch only acts under `panic = "unwind"` (tests): the release build aborts on a panic.
fn run_timer(client: Weak<ConvexClient>) {
    loop {
        let Some(client) = client.upgrade() else {
            return;
        };
        let next = panic::catch_unwind(AssertUnwindSafe(|| client.tick())).unwrap_or_else(|_| {
            client
                .span
                .in_scope(|| warn!("convex timer: a callback panicked"));
            let mut state = client.state();
            Some(client.clock.now_ms() + client.next_backoff(&mut state))
        });
        let mut state = client.state();
        if state.link.is_closed() {
            return;
        }
        if mem::take(&mut state.woken) {
            continue;
        }
        let wait = next.map_or(MAX_TIMER_WAIT, |at| {
            Duration::from_millis(at.saturating_sub(client.clock.now_ms())).min(MAX_TIMER_WAIT)
        });
        let (mut state, _) = client
            .wake
            .wait_timeout(state, wait)
            .unwrap_or_else(PoisonError::into_inner);
        state.woken = false;
    }
}

/// Queues an update for each subscription whose result changed.
fn queue_updates(state: &mut State) {
    let State {
        base,
        subscriptions,
        io,
        ..
    } = state;
    let results = base.latest_results();
    for subscription in subscriptions.values_mut() {
        let Some(result) = results.get(&subscription.subscriber) else {
            continue;
        };
        if subscription.last.as_ref() != Some(result) {
            io.push_back(Io::Update(
                subscription.on_update.clone(),
                outcome_of(result.clone()),
            ));
            subscription.last = Some(result.clone());
        }
    }
}

struct Listener {
    client: Weak<ConvexClient>,
    epoch: u64,
}

impl ConvexSocketListener for Listener {
    fn on_open(&self) {
        if let Some(client) = self.client.upgrade() {
            client.on_open(self.epoch);
        }
    }

    fn on_text(&self, text: String) {
        if let Some(client) = self.client.upgrade() {
            client.on_text(self.epoch, text);
        }
    }

    fn on_closed(&self, reason: String) {
        if let Some(client) = self.client.upgrade() {
            client.on_closed(self.epoch, reason);
        }
    }
}

/// `https://host` → `wss://host/api/sync` (as convex-rs's `deployment_to_ws_url`).
fn sync_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    let url = if let Some(rest) = url.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = url.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        url.to_owned()
    };
    if url.ends_with("/api/sync") {
        url
    } else {
        format!("{url}/api/sync")
    }
}

fn parse_path(path: &str) -> Result<UdfPath, CallError> {
    path.parse()
        .map_err(|error| CallError::Message(format!("bad function path {path}: {error}")))
}

fn parse_args(args: Value) -> Result<BTreeMap<String, ConvexValue>, CallError> {
    let Value::Object(fields) = args else {
        return Err(CallError::Message("arguments must be an object".into()));
    };
    fields
        .into_iter()
        .map(|(key, value)| {
            let value = ConvexValue::try_from(value)
                .map_err(|error| CallError::Message(error.to_string()))?;
            Ok((key, value))
        })
        .collect()
}

fn outcome_of(result: FunctionResult) -> Update {
    match result {
        FunctionResult::Value(value) => Ok(value.0),
        FunctionResult::ErrorMessage(message) => Err(CallError::Message(message)),
        FunctionResult::ConvexError(error) => Err(CallError::Convex {
            message: error.message,
            data: error.data.0,
        }),
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64)
}

/// Random bits from std's per-process hash keys and the time (enough for jitter and a session
/// id; nothing secret).
fn random_u64() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u128(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos()),
    );
    hasher.finish()
}

fn random_session_id() -> SessionId {
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&random_u64().to_le_bytes());
    bytes[8..].copy_from_slice(&random_u64().to_le_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    #[expect(clippy::string_slice, reason = "ASCII hex of a fixed length")]
    let uuid = format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    );
    #[expect(
        clippy::expect_used,
        reason = "the UUID is well-formed by construction"
    )]
    uuid.parse().expect("a well-formed UUID")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_jitter_stays_between_half_and_all() {
        let clock = SystemClock(Instant::now());
        for _ in 0..100 {
            let delay = clock.jitter(30_000);
            assert!((15_000..=30_000).contains(&delay), "{delay}");
        }
    }

    #[test]
    fn sync_url_points_at_api_sync() {
        assert_eq!(sync_url("https://a.example/"), "wss://a.example/api/sync");
        assert_eq!(
            sync_url("http://127.0.0.1:3210"),
            "ws://127.0.0.1:3210/api/sync"
        );
        assert_eq!(
            sync_url("wss://a.example/api/sync"),
            "wss://a.example/api/sync"
        );
    }

    #[test]
    fn session_ids_differ() {
        assert_ne!(*random_session_id(), *random_session_id());
    }
}

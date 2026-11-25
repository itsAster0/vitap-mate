//! Live subscriptions ([`Core::live_watch`]) and the single `Condvar`-driven JWT renewal
//! coordinator that keeps watched queries authenticated.

use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tracing::{info, warn};

use super::Backend;
use crate::device::EdgeError;
use crate::remote::server_code::ServerCode;
use crate::{Core, CoreError, Shared};

/// What a screen follows live ([`Core::live_watch`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum LiveTopic {
    /// `account:view`: the account, its phones and keys.
    Account,
    /// `mail:inbox`: this phone's forwarded mail.
    MailInbox,
    /// `sessions:pending`: connected apps' requests for this phone's VTOP session.
    SessionRequests,
    /// `account:signIns`: the account's last VTOP sign-ins.
    SignIns,
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
    pub(super) fn of(error: EdgeError) -> Self {
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
    pub(super) backend: Arc<dyn Backend>,
    pub(super) id: u64,
    /// While any is held, the JWT is renewed before it runs out ([`Renewal`]).
    pub(super) held: Mutex<Option<Arc<()>>>,
    pub(super) wake: Arc<Condvar>,
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

impl Core {
    /// Coalesces an immediate watch renewal onto the single coordinator thread.
    pub(super) fn request_watch_renewal(
        self: &Arc<Self>,
        flow: &str,
        listener: Arc<dyn LiveListener>,
    ) {
        let start = self
            .shared
            .backend_state()
            .renewal
            .request_now(flow, listener);
        if start {
            run_renewal_coordinator(Arc::downgrade(self), self.shared.clone());
        }
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
pub(super) struct Renewal {
    pub(super) wake: Arc<Condvar>,
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
    pub(super) fn watch(&mut self, held: &Arc<()>, flow: &str) -> bool {
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
    pub(super) fn poke(&mut self) {
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
pub(super) fn run_renewal_coordinator(core: std::sync::Weak<Core>, shared: Arc<Shared>) {
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

#[cfg(test)]
mod tests {
    use super::super::identity::RENEW_AHEAD_MS;
    use super::*;

    #[derive(Default)]
    struct Heard(Mutex<Vec<String>>);

    impl LiveListener for Heard {
        fn on_change(&self) {
            self.0.lock().unwrap().push("change".into());
        }
        fn on_error(&self, error: LiveError) {
            self.0.lock().unwrap().push(format!("{error:?}"));
        }
    }

    #[test]
    fn held_watches_renew_the_jwt_a_minute_early() {
        let expires = 1_000_000;
        let due = Some(expires - RENEW_AHEAD_MS);
        // Far off: it looks again within the check, to notice the watches went.
        assert_eq!(renew_in(due, 0), Some(KEEP_CHECK));
        assert_eq!(
            renew_in(due, expires - RENEW_AHEAD_MS - 5_000),
            Some(Duration::from_secs(5))
        );
        // Due, past due, or no JWT yet: renewed now.
        assert_eq!(renew_in(due, expires - RENEW_AHEAD_MS), None);
        assert_eq!(renew_in(due, expires), None);
        assert_eq!(renew_in(None, 0), None);
    }

    #[test]
    fn renewal_coordinator_coalesces_requests_and_stops_when_watches_drop() {
        let mut renewal = Renewal::default();
        let now = Instant::now();
        let held_a = Arc::new(());
        let held_b = Arc::new(());
        assert!(renewal.watch(&held_a, "f1"), "first watch starts thread");
        assert!(!renewal.watch(&held_b, "f2"), "second watch reuses thread");

        // Multiple unauthenticated watch failures coalesce into one immediate run and dedupe listeners.
        let listener_a: Arc<dyn LiveListener> = Arc::new(Heard::default());
        let listener_b: Arc<dyn LiveListener> = Arc::new(Heard::default());
        assert!(!renewal.request_now("f1", listener_a.clone()));
        assert!(!renewal.request_now("f1", listener_a.clone()));
        assert!(!renewal.request_now("f2", listener_b.clone()));
        assert_eq!(renewal.pending.len(), 2);

        assert_eq!(
            renewal.decide(Some(1_000_000), 0, now),
            RenewDecision::Run { flow: "f2".into() }
        );
        assert_eq!(renewal.phase, RenewPhase::Renewing);

        // Server change pokes the coordinator without spawning a second thread.
        renewal.poke();
        assert!(renewal.running);

        // No identity stops the coordinator and returns both pending listeners to notify.
        let (stop, notify) = renewal.finish(&Err(CoreError::ConsentDenied), now);
        assert!(stop);
        assert_eq!(notify.len(), 2);
        assert!(!renewal.running);

        // Restarting then dropping all watches cleanly transitions to Stop.
        assert!(renewal.watch(&held_a, "f3"));
        drop(held_a);
        drop(held_b);
        assert_eq!(renewal.decide(Some(1_000_000), 0, now), RenewDecision::Stop);
        assert!(!renewal.running);
    }

    #[test]
    fn live_errors_say_what_the_app_does_next() {
        let status = |code, message: &str| EdgeError::Status {
            code,
            message: message.into(),
        };
        for code in [
            ServerCode::Unauthenticated,
            ServerCode::ServerError,
            ServerCode::NotFound,
            ServerCode::BadRequest,
        ] {
            assert_eq!(LiveError::of(status(code, "x")), LiveError::Transient);
        }
        assert_eq!(
            LiveError::of(status(ServerCode::DeviceRemoved, "Removed.")),
            LiveError::NoIdentity {
                message: "Removed.".into()
            }
        );
        assert_eq!(
            LiveError::of(status(ServerCode::Unknown("mailbox_full".into()), "Full.")),
            LiveError::Stopped {
                message: "Full.".into()
            }
        );
        assert_eq!(
            LiveError::of(EdgeError::Offline),
            LiveError::Stopped {
                message: "The server can't be reached.".into()
            }
        );
    }
}

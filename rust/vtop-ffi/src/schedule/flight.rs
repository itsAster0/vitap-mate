//! One run per key at a time on the core's executor: a port of Kotlin's `SingleFlight`.
//!
//! A caller that goes away doesn't cancel its run, and one that comes back for the same key gets
//! the running (or waiting) one, or learns that one just succeeded ([`Flight::succeeded_within`]).
//! At most `fetch_lanes` runs go at once, 1 on a slow link ([`Flight::slow_link`]). The rest
//! wait, and when a lane frees the next is picked then: pages on screen first, then pages the
//! student left, then background work; within one, runs the student asked for, then the oldest.
//! A waiting run is never cancelled by navigation, only moved; one that is automatic, has a
//! cached copy, is off screen and has waited over `queue_drop_ms` is dropped instead
//! ([`FlightError::Dropped`]). A running one always finishes; only [`Flight::cancel_all`]
//! (sign-out) stops it. The decisions are [`FetchQueue`]'s; this runs them.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use futures::FutureExt;
use futures::channel::oneshot;
use futures::future::{BoxFuture, Shared};

use super::queue::{FetchQueue, QueueConfig, QueueEffect, QueueEvent, Ticket};
use crate::CoreError;
use crate::runtime::Runtime;

/// Why a run gave no value. `Dropped` and `Cancelled` are the queue's, not VTOP's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FlightError {
    /// The work failed.
    Failed(CoreError),
    /// It waited off screen too long and never ran.
    Dropped,
    /// Sign-out cancelled it.
    Cancelled,
}

/// How a run is queued. `user`: the student asked (goes first within its priority, never
/// dropped); `has_cache`: the page has a copy to keep showing if this one is dropped;
/// `background`: the open-app sweep (lowest priority).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RunOpts {
    pub(crate) user: bool,
    pub(crate) has_cache: bool,
    pub(crate) background: bool,
}

impl Default for RunOpts {
    fn default() -> Self {
        Self {
            user: true,
            has_cache: false,
            background: false,
        }
    }
}

pub(crate) type Outcome<T> = Shared<BoxFuture<'static, Result<T, FlightError>>>;

/// The run for a key, started in `flow`; `joined` when it was already running or waiting;
/// `started` ends true when it gets a lane, false when it is dropped or cancelled first.
pub(crate) struct Run<T: Clone> {
    pub(crate) result: Outcome<T>,
    pub(crate) flow: String,
    pub(crate) joined: bool,
    pub(crate) started: Shared<BoxFuture<'static, bool>>,
}

/// Writes one `fetch.queue` line: message, flow and fields.
pub(crate) type QueueLog = Arc<dyn Fn(&str, &str, &[(&'static str, String)]) + Send + Sync>;

struct Handle {
    go: Option<oneshot::Sender<bool>>,
    task: Option<smol::Task<()>>,
    /// The run's [`Outcome`], typed by its caller.
    result: Box<dyn Any + Send>,
    started: Shared<BoxFuture<'static, bool>>,
}

#[derive(Default)]
struct State {
    queue: FetchQueue,
    /// Told when the pages holding a lane change ([`Flight::watch_running`]).
    watchers: Vec<smol::channel::Sender<()>>,
    /// The runs by ticket id, until they end.
    handles: HashMap<u64, Handle>,
}

struct Inner {
    state: Mutex<State>,
    runtime: Arc<Runtime>,
    config: Arc<dyn Fn() -> QueueConfig + Send + Sync>,
    log: QueueLog,
}

#[derive(Clone)]
pub(crate) struct Flight {
    inner: Arc<Inner>,
}

/// What [`Inner::step`] decided, with the handle parts each effect needs, done outside the lock.
enum Act {
    Go(oneshot::Sender<bool>),
    Drop(Option<oneshot::Sender<bool>>, Option<smol::Task<()>>),
    Cancel(Option<oneshot::Sender<bool>>, Option<smol::Task<()>>),
    Log(&'static str, String, Vec<(&'static str, String)>),
}

impl Flight {
    pub(crate) fn new(
        runtime: Arc<Runtime>,
        config: Arc<dyn Fn() -> QueueConfig + Send + Sync>,
        log: QueueLog,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                state: Mutex::default(),
                runtime,
                config,
                log,
            }),
        }
    }

    /// The running or waiting run of `key`, else `work` queued now in `flow`.
    pub(crate) fn start<T: Clone + Send + Sync + 'static>(
        &self,
        key: &str,
        flow: &str,
        opts: RunOpts,
        work: impl Future<Output = Result<T, CoreError>> + Send + 'static,
    ) -> Run<T> {
        let inner = &self.inner;
        let (run, acts) = {
            let mut state = inner.lock();
            if let Some(t) = state.queue.active(key).cloned() {
                let joined = state.handles.get(&t.id).map(|handle| {
                    let result = handle
                        .result
                        .downcast_ref::<Outcome<T>>()
                        .cloned()
                        // A key always runs one type of work; another type is a bug: it fails.
                        .unwrap_or_else(|| {
                            futures::future::ready(Err(FlightError::Cancelled))
                                .boxed()
                                .shared()
                        });
                    (result, handle.started.clone())
                });
                if let Some((result, started)) = joined {
                    let acts = if opts.user && !t.user {
                        inner.step(&mut state, QueueEvent::Ask { key: key.into() })
                    } else {
                        Vec::new()
                    };
                    drop(state);
                    inner.perform(acts);
                    return Run {
                        result,
                        flow: t.flow,
                        joined: true,
                        started,
                    };
                }
            }
            let ticket = Ticket {
                id: state.queue.next_id,
                key: key.into(),
                flow: flow.into(),
                user: opts.user,
                has_cache: opts.has_cache,
                background: opts.background,
                at: inner.runtime.clock().now_ms(),
                logged: false,
            };
            let (go, go_rx) = oneshot::channel::<bool>();
            let (done, done_rx) = oneshot::channel::<Result<T, FlightError>>();
            // Some(true): a lane; Some(false): dropped; None: cancelled (the sender went).
            let go_rx = go_rx.map(Result::ok).boxed().shared();
            let started: Shared<BoxFuture<'static, bool>> =
                go_rx.clone().map(|go| go == Some(true)).boxed().shared();
            let result: Outcome<T> = done_rx
                .map(|outcome| outcome.unwrap_or(Err(FlightError::Cancelled)))
                .boxed()
                .shared();
            let task = inner.runtime.spawn(flow, {
                let inner = inner.clone();
                let (id, key, flow) = (ticket.id, ticket.key.clone(), ticket.flow.clone());
                async move {
                    // A cancel can land while this poll runs, so it is told apart from a drop.
                    let outcome = match go_rx.await {
                        Some(true) => work.await.map_err(FlightError::Failed),
                        Some(false) => Err(FlightError::Dropped),
                        None => Err(FlightError::Cancelled),
                    };
                    // The queue first, so a caller that has the outcome sees the key settled.
                    inner.finished(id, key, flow, outcome.is_ok());
                    #[expect(
                        clippy::let_underscore_must_use,
                        reason = "every caller may have gone; the run still counts"
                    )]
                    let _ = done.send(outcome);
                }
            });
            state.handles.insert(
                ticket.id,
                Handle {
                    go: Some(go),
                    task: Some(task),
                    result: Box::new(result.clone()),
                    started: started.clone(),
                },
            );
            let acts = inner.step(&mut state, QueueEvent::Start(ticket));
            let run = Run {
                result,
                flow: flow.into(),
                joined: false,
                started,
            };
            (run, acts)
        };
        inner.perform(acts);
        run
    }

    /// The student asked for `key`'s run too (joined it): it goes ahead and is never dropped.
    pub(crate) fn ask(&self, key: &str) {
        self.inner.dispatch(QueueEvent::Ask { key: key.into() });
    }

    /// A screen showing `page` came (`on`) or went; its waiting runs move up or down.
    pub(crate) fn shown(&self, page: &str, on: bool) {
        self.inner.dispatch(QueueEvent::Shown {
            page: page.into(),
            on,
        });
    }

    /// A page fetch in `flow` took `ms` and came over UDP or not: sets the lanes.
    pub(crate) fn fetched(&self, flow: &str, ms: i64, udp: bool) {
        self.inner.dispatch(QueueEvent::Fetched {
            flow: flow.into(),
            ms,
            udp,
        });
    }

    /// Whether a run of `key` is going on or waiting for a lane.
    pub(crate) fn is_running(&self, key: &str) -> bool {
        self.inner.lock().queue.active(key).is_some()
    }

    /// Whether a screen shows `page`.
    pub(crate) fn is_shown(&self, page: &str) -> bool {
        self.inner.lock().queue.is_shown(page)
    }

    /// Whether `key`'s last run succeeded at most `ms` ago.
    pub(crate) fn succeeded_within(&self, key: &str, ms: i64) -> bool {
        let now = self.inner.runtime.clock().now_ms();
        self.inner.lock().queue.succeeded_within(key, ms, now)
    }

    /// Most of the last few fetches took over `slow_fetch_ms`, or the last came over UDP.
    pub(crate) fn slow_link(&self) -> bool {
        let config = (self.inner.config)();
        self.inner.lock().queue.slow(config)
    }

    /// Pages of the runs holding a lane, oldest first ("Waiting for Attendance to finish").
    pub(crate) fn running_pages(&self) -> Vec<String> {
        self.inner.lock().queue.running_pages()
    }

    /// Told (with nothing) each time [`Flight::running_pages`] changes, until dropped.
    pub(crate) fn watch_running(&self) -> smol::channel::Receiver<()> {
        let (watcher, changes) = smol::channel::unbounded();
        self.inner.lock().watchers.push(watcher);
        changes
    }

    /// Cancels every run, waiting ones too, and forgets which succeeded (sign-out).
    pub(crate) fn cancel_all(&self) {
        self.inner.dispatch(QueueEvent::CancelAll);
    }
}

impl Inner {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn dispatch(&self, event: QueueEvent) {
        let acts = {
            let mut state = self.lock();
            self.step(&mut state, event)
        };
        self.perform(acts);
    }

    /// Run `id` ended by itself: its handle goes (the task, already ending, is let go).
    fn finished(&self, id: u64, key: String, flow: String, succeeded: bool) {
        let acts = {
            let mut state = self.lock();
            if let Some(task) = state
                .handles
                .remove(&id)
                .and_then(|mut handle| handle.task.take())
            {
                task.detach();
            }
            self.step(
                &mut state,
                QueueEvent::Finished {
                    ticket_id: id,
                    key,
                    flow,
                    succeeded,
                },
            )
        };
        self.perform(acts);
    }

    /// `event` applied to the queue; its effects with their runs' parts, for [`Inner::perform`].
    fn step(&self, state: &mut State, event: QueueEvent) -> Vec<Act> {
        let now = self.runtime.clock().now_ms();
        let before = state.queue.running_pages();
        let next = std::mem::take(&mut state.queue).next(event, now, (self.config)());
        state.queue = next.queue;
        if state.queue.running_pages() != before {
            // Unbounded: a send fails only for a watcher that went, which is dropped here.
            state
                .watchers
                .retain(|watcher| watcher.try_send(()).is_ok());
        }
        next.effects
            .into_iter()
            .filter_map(|effect| match effect {
                QueueEffect::Go(id) => state
                    .handles
                    .get_mut(&id)
                    .and_then(|handle| handle.go.take())
                    .map(Act::Go),
                QueueEffect::Drop(id) => state
                    .handles
                    .remove(&id)
                    .map(|mut handle| Act::Drop(handle.go.take(), handle.task.take())),
                QueueEffect::Cancel(id) => state
                    .handles
                    .remove(&id)
                    .map(|mut handle| Act::Cancel(handle.go.take(), handle.task.take())),
                QueueEffect::Log { msg, flow, fields } => Some(Act::Log(msg, flow, fields)),
            })
            .collect()
    }

    /// Outside the lock: a run resumed here may start work that comes back to the queue.
    fn perform(&self, acts: Vec<Act>) {
        for act in acts {
            match act {
                Act::Go(go) => {
                    #[expect(
                        clippy::let_underscore_must_use,
                        reason = "a run whose task is gone has nobody to start"
                    )]
                    let _ = go.send(true);
                }
                Act::Drop(go, task) => {
                    if let Some(go) = go {
                        #[expect(
                            clippy::let_underscore_must_use,
                            reason = "a run whose task is gone has nobody to tell"
                        )]
                        let _ = go.send(false);
                    }
                    // It ends by itself with Dropped.
                    if let Some(task) = task {
                        task.detach();
                    }
                }
                // Dropping the task (and its go) cancels it; its callers get Cancelled.
                Act::Cancel(go, task) => {
                    drop(task);
                    drop(go);
                }
                Act::Log(msg, flow, fields) => (self.log)(msg, &flow, &fields),
            }
        }
    }
}

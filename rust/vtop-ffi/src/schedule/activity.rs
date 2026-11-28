//! Work that talks to VTOP, for the app bar ("Refreshing attendance…") and its recent-activity
//! sheet: a port of Kotlin's `Activity`. Each entry is one flow, so "Details" opens it in the
//! Inspector.

use std::sync::{Arc, Mutex, PoisonError};

use futures::channel::oneshot;

use crate::problem::{Problem, ProblemKind};
use crate::runtime::Clock;
use crate::{Core, CoreError};

/// Finished runs kept, newest first.
pub(crate) const KEEP: usize = 30;

/// How a run is going or how it ended; times in ms.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ActivityRun {
    Running,
    Done { ms: i64 },
    Failed { ms: i64, problem: Problem },
    Cancelled { ms: i64 },
}

/// One tracked run: `label` ("Refreshing attendance") in `flow`, started at `started_at`
/// (unix ms). `id` tells two runs of one flow apart.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ActivityEntry {
    pub id: u64,
    pub flow: String,
    pub label: String,
    pub started_at: i64,
    pub run: ActivityRun,
}

/// What runs now, oldest first, and what finished, newest first (at most [`KEEP`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ActivityState {
    pub(crate) running: Vec<ActivityEntry>,
    pub(crate) recent: Vec<ActivityEntry>,
}

impl ActivityState {
    pub(crate) fn started(mut self, entry: ActivityEntry) -> Self {
        self.running.push(entry);
        self
    }

    /// Run `id` left `running` as `finished`, at the top of `recent`.
    pub(crate) fn finished(mut self, id: u64, finished: ActivityEntry) -> Self {
        self.running.retain(|entry| entry.id != id);
        self.recent.insert(0, finished);
        self.recent.truncate(KEEP);
        self
    }

    /// The latest finished run failed because the phone is offline.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the Connection screen reads it once Activity mirrors Rust"
        )
    )]
    pub(crate) fn offline(&self) -> bool {
        matches!(
            self.recent.first(),
            Some(ActivityEntry {
                run: ActivityRun::Failed { problem, .. },
                ..
            }) if problem.kind == ProblemKind::Offline
        )
    }

    /// Running first, then recent: what [`crate::AppEvents::on_activity`] gets.
    fn entries(&self) -> Vec<ActivityEntry> {
        self.running.iter().chain(&self.recent).cloned().collect()
    }
}

pub(crate) type ActivityPush = Arc<dyn Fn(Vec<ActivityEntry>) + Send + Sync>;

#[derive(Default)]
struct Tracker {
    next_id: u64,
    state: ActivityState,
    /// Waiters woken once `state.running` becomes empty (the open-app sweep).
    idle_waiters: Vec<oneshot::Sender<()>>,
}

pub(crate) struct Activity {
    state: Mutex<Tracker>,
    clock: Arc<dyn Clock>,
    push: ActivityPush,
}

/// How a run Kotlin tracks ended ([`Core::track_end`]).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TrackEnd {
    Done,
    Failed { problem: Problem },
    Cancelled,
}

impl Activity {
    pub(crate) fn new(clock: Arc<dyn Clock>, push: ActivityPush) -> Self {
        Self {
            state: Mutex::default(),
            clock,
            push,
        }
    }

    #[cfg(test)]
    pub(crate) fn state(&self) -> ActivityState {
        self.lock().state.clone()
    }

    /// Waits until no tracked work is running.
    pub(crate) async fn wait_idle(&self) {
        let rx = {
            let mut tracker = self.lock();
            if tracker.state.running.is_empty() {
                return;
            }
            let (tx, rx) = oneshot::channel();
            tracker.idle_waiters.push(tx);
            rx
        };
        #[expect(
            clippy::let_underscore_must_use,
            reason = "sender drops only if the activity tracker is torn down"
        )]
        let _ = rx.await;
    }

    /// `label` in `flow` starts running; its id ends it ([`Activity::end`]).
    pub(crate) fn start(&self, flow: &str, label: &str) -> u64 {
        let started_at = self.clock.now_ms();
        let entry = {
            let mut tracker = self.lock();
            tracker.next_id += 1;
            ActivityEntry {
                id: tracker.next_id,
                flow: flow.into(),
                label: label.into(),
                started_at,
                run: ActivityRun::Running,
            }
        };
        let id = entry.id;
        self.change(|state| state.started(entry));
        id
    }

    /// Run `id` ended as `how`; an unknown or ended one changes nothing.
    pub(crate) fn end(&self, id: u64, how: TrackEnd) {
        let now = self.clock.now_ms();
        self.change(|state| {
            let Some(entry) = state.running.iter().find(|entry| entry.id == id).cloned() else {
                return state;
            };
            let ms = now - entry.started_at;
            let run = match how {
                TrackEnd::Done => ActivityRun::Done { ms },
                TrackEnd::Failed { problem } => ActivityRun::Failed { ms, problem },
                TrackEnd::Cancelled => ActivityRun::Cancelled { ms },
            };
            state.finished(id, ActivityEntry { run, ..entry })
        });
    }

    /// Runs `work` as `label` in `flow`; a failure is recorded and passed on, and a run dropped
    /// before it ends (cancelled) is recorded as such.
    pub(crate) async fn track<T>(
        &self,
        label: &str,
        flow: &str,
        work: impl Future<Output = Result<T, CoreError>>,
    ) -> Result<T, CoreError> {
        let mut guard = Ending {
            activity: self,
            id: Some(self.start(flow, label)),
        };
        let result = work.await;
        guard.end(match &result {
            Ok(_) => TrackEnd::Done,
            Err(error) => TrackEnd::Failed {
                problem: Problem::of(error, flow, false),
            },
        });
        result
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Tracker> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn change(&self, next: impl FnOnce(ActivityState) -> ActivityState) {
        let (entries, waiters) = {
            let mut tracker = self.lock();
            tracker.state = next(std::mem::take(&mut tracker.state));
            let waiters = if tracker.state.running.is_empty() {
                std::mem::take(&mut tracker.idle_waiters)
            } else {
                Vec::new()
            };
            (tracker.state.entries(), waiters)
        };
        for tx in waiters {
            #[expect(
                clippy::let_underscore_must_use,
                reason = "waiter may have been cancelled before activity became idle"
            )]
            let _ = tx.send(());
        }
        (self.push)(entries);
    }
}

/// Ends a tracked run once: as it finished, or as cancelled when its future is dropped first.
struct Ending<'a> {
    activity: &'a Activity,
    id: Option<u64>,
}

impl Ending<'_> {
    fn end(&mut self, how: TrackEnd) {
        if let Some(id) = self.id.take() {
            self.activity.end(id, how);
        }
    }
}

impl Drop for Ending<'_> {
    fn drop(&mut self) {
        self.end(TrackEnd::Cancelled);
    }
}

/// Work Kotlin runs itself (a calendar refresh, a sync page), in the same list as the core's.
#[uniffi::export]
impl Core {
    /// `label` ("Refreshing calendar") starts in `flow`; give the id to [`Core::track_end`].
    pub fn track_start(&self, flow: String, label: String) -> u64 {
        self.schedule.activity.start(&flow, &label)
    }

    /// The entry already names its flow; `_flow` keeps the flow-first call shape.
    pub fn track_end(&self, _flow: String, id: u64, how: TrackEnd) {
        self.schedule.activity.end(id, how);
    }
}

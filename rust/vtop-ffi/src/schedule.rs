//! When VTOP work runs: the fetch queue ([`Flight`], deciding with [`queue::FetchQueue`]) and
//! the activity tracker ([`Activity`]), on the core's [`Runtime`].

mod activity;
mod flight;
mod freshness;
mod queue;
mod sweep;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};

pub(crate) use activity::{Activity, ActivityPush};
pub use activity::{ActivityEntry, ActivityRun, TrackEnd};
pub(crate) use flight::{Flight, FlightError, QueueLog, RunOpts};
pub(crate) use freshness::Freshness;
pub(crate) use queue::QueueConfig;
pub(crate) use sweep::Sweep;

use crate::runtime::Runtime;
use crate::settings::{SettingsStore, TunableKey};
use crate::{Core, CoreError};

/// The queue's limits, read from their tunables at each event (Settings → Developer).
fn queue_config(settings: &SettingsStore) -> QueueConfig {
    let now = settings.get();
    QueueConfig {
        fetch_lanes: now.tunable(TunableKey::FetchLanes),
        queue_drop_ms: i64::from(now.tunable(TunableKey::QueueDrop)) * 1000,
        slow_fetch_ms: i64::from(now.tunable(TunableKey::SlowFetch)) * 1000,
    }
}

/// `fetch.queue` lines through the app's log, as Kotlin's `SingleFlight` wrote them.
fn queue_log() -> QueueLog {
    Arc::new(|msg, flow, fields| {
        let fields: HashMap<String, String> = fields
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.clone()))
            .collect();
        crate::obs::log_event(
            "info".into(),
            "fetch.queue".into(),
            Some(flow.into()),
            msg.into(),
            fields,
        );
    })
}

/// The core's executor, fetch queue, activity tracker, freshness refreshes and open-app sweep.
pub(crate) struct Schedule {
    pub(crate) runtime: Arc<Runtime>,
    pub(crate) flight: Flight,
    pub(crate) activity: Activity,
    pub(crate) freshness: Freshness,
    pub(crate) sweep: Sweep,
    /// Page keys a watch fetched in this process ("this screen fetched this semester"), so
    /// opening the page again doesn't fetch again with Auto Refresh on.
    fetched: Mutex<HashSet<String>>,
}

impl Schedule {
    /// On `runtime`; the tracker pushes to `push`.
    pub(crate) fn on(
        runtime: Arc<Runtime>,
        push: ActivityPush,
        settings: Arc<SettingsStore>,
    ) -> Self {
        let config = Arc::new(move || queue_config(&settings));
        Self {
            flight: Flight::new(runtime.clone(), config, queue_log()),
            activity: Activity::new(runtime.clock().clone(), push),
            freshness: Freshness::default(),
            sweep: Sweep::default(),
            runtime,
            fetched: Mutex::default(),
        }
    }

    pub(crate) fn fetched(&self, key: &str) -> bool {
        self.lock_fetched().contains(key)
    }

    pub(crate) fn mark_fetched(&self, key: &str) {
        self.lock_fetched().insert(key.to_owned());
    }

    /// Sign-out or another account: every run goes, and what was fetched is forgotten.
    pub(crate) fn cancel_all(&self) {
        self.sweep.cancel();
        self.freshness.clear();
        self.flight.cancel_all();
        self.lock_fetched().clear();
    }

    fn lock_fetched(&self) -> std::sync::MutexGuard<'_, HashSet<String>> {
        self.fetched.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Work Kotlin queues itself (the open-app sweep, sync), run when it gets a lane. True when it
/// succeeded; Kotlin keeps what it threw.
#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait QueuedWork: Send + Sync {
    async fn run(&self) -> bool;
}

/// How a [`Core::queue_run`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum QueueEnd {
    /// The work ran (it may have failed).
    Ran,
    /// A run of the key was going already; this work didn't run.
    Joined,
    /// It waited off screen too long and never ran.
    Dropped,
    /// Sign-out cancelled it.
    Cancelled,
}

/// The fetch queue for Kotlin's own runs, until the sweep and sync move here.
#[uniffi::export]
impl Core {
    /// Runs `work` under `key` when it gets a lane, as page fetches do. `user`: the student
    /// asked; `has_cache`: there is a copy to keep if it is dropped; `background`: lowest
    /// priority.
    pub async fn queue_run(
        &self,
        flow: String,
        key: String,
        user: bool,
        has_cache: bool,
        background: bool,
        work: Arc<dyn QueuedWork>,
    ) -> QueueEnd {
        let opts = RunOpts {
            user,
            has_cache,
            background,
        };
        let run = self.schedule.flight.start(&key, &flow, opts, async move {
            if work.run().await {
                Ok(())
            } else {
                Err(CoreError::Other {
                    reason: "failed".into(),
                })
            }
        });
        let joined = run.joined;
        match run.result.await {
            _ if joined => QueueEnd::Joined,
            Ok(()) | Err(FlightError::Failed(_)) => QueueEnd::Ran,
            Err(FlightError::Dropped) => QueueEnd::Dropped,
            Err(FlightError::Cancelled) => QueueEnd::Cancelled,
        }
    }

    /// Cancels every queued and running fetch (Developer, the account teardown).
    pub fn cancel_fetches(&self, flow: String) {
        crate::span("cancel_fetches", &flow).in_scope(|| self.schedule.cancel_all());
    }

    /// Most recent fetches were slow, or came over UDP: one page at a time.
    pub fn slow_link(&self) -> bool {
        self.schedule.flight.slow_link()
    }

    /// Whether a screen shows page `page` (`attendance`).
    pub fn page_shown(&self, page: String) -> bool {
        self.schedule.flight.is_shown(&page)
    }
}

//! A watched page's status, with no I/O: ports of Kotlin's `PageEvent.next`, `PageActivity`
//! and `refreshTrigger`.

use crate::problem::Problem;

/// What a screen shows of a page (the data itself is read by the screen when `updated_at`
/// changes). `count` and `flow` are for logs and the state dump: how many rows, and the flow
/// the data came in.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum PageStatus {
    /// Nothing to show yet (no cache; the first fetch runs). `waiting`: the fetch is queued
    /// behind other pages ("Waiting for Attendance to finish"), set after a moment.
    Loading { waiting: Option<String> },
    /// Cached or fresh data from `updated_at` (unix seconds), and what its refresh is doing.
    Ready {
        updated_at: i64,
        count: Option<i32>,
        flow: String,
        activity: PageActivity,
    },
    /// Nothing cached and the fetch failed.
    Failed { problem: Problem },
}

/// What a ready page's refresh is doing.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum PageActivity {
    Idle,
    /// A fetch runs; `waiting` as in [`PageStatus::Loading`] while it is queued.
    Refreshing {
        waiting: Option<String>,
    },
    /// The last refresh failed with `problem`; the data was kept.
    Stale {
        problem: Problem,
    },
}

impl PageActivity {
    /// After a fetch that ended without an answer (cancelled, or dropped from the queue).
    pub(crate) fn settled(self) -> Self {
        match self {
            Self::Refreshing { .. } => Self::Idle,
            other => other,
        }
    }

    /// `text` as the refresh's `waiting`; only a running fetch waits.
    pub(crate) fn waiting(self, text: Option<String>) -> Self {
        match self {
            Self::Refreshing { .. } => Self::Refreshing { waiting: text },
            other => other,
        }
    }
}

/// A page read or fetched: when, and how many rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PageData {
    pub(crate) updated_at: i64,
    pub(crate) count: Option<i32>,
}

/// What happens to a watched page; [`next`] is the status after each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PageEvent {
    /// The saved copy, read before any fetch of the watch starts, in `flow`.
    CacheLoaded {
        data: PageData,
        flow: String,
    },
    FetchStarted,
    /// The fetch waits for a lane behind `text`, or no longer (None).
    WaitingChanged(Option<String>),
    FetchSucceeded {
        data: PageData,
        flow: String,
    },
    FetchFailed(Problem),
    /// Waited off screen too long and never ran, in `flow`.
    FetchDropped {
        flow: String,
    },
    /// Cancelled before it answered (sign-out, an account switch), in `flow`.
    FetchCancelled {
        flow: String,
    },
}

fn ready(data: PageData, flow: String, activity: PageActivity) -> PageStatus {
    PageStatus::Ready {
        updated_at: data.updated_at,
        count: data.count,
        flow,
        activity,
    }
}

/// The status after `event`. A loading page always has its cache read or a fetch going, so a
/// fetch that ends without an answer and nothing to show fails rather than spinning on. Fresh
/// data always wins; the cache only fills a page that has none.
pub(crate) fn next(status: PageStatus, event: PageEvent) -> PageStatus {
    use PageStatus as S;
    match (status, event) {
        (S::Loading { .. }, PageEvent::CacheLoaded { data, flow }) => {
            ready(data, flow, PageActivity::Idle)
        }
        (S::Failed { problem }, PageEvent::CacheLoaded { data, flow }) => {
            ready(data, flow, PageActivity::Stale { problem })
        }
        (ready @ S::Ready { .. }, PageEvent::CacheLoaded { .. }) => ready,
        (
            S::Ready {
                updated_at,
                count,
                flow,
                ..
            },
            PageEvent::FetchStarted,
        ) => S::Ready {
            updated_at,
            count,
            flow,
            activity: PageActivity::Refreshing { waiting: None },
        },
        (_, PageEvent::FetchStarted) => S::Loading { waiting: None },
        (S::Loading { .. }, PageEvent::WaitingChanged(waiting)) => S::Loading { waiting },
        (
            S::Ready {
                updated_at,
                count,
                flow,
                activity,
            },
            PageEvent::WaitingChanged(text),
        ) => S::Ready {
            updated_at,
            count,
            flow,
            activity: activity.waiting(text),
        },
        (failed @ S::Failed { .. }, PageEvent::WaitingChanged(_)) => failed,
        (_, PageEvent::FetchSucceeded { data, flow }) => ready(data, flow, PageActivity::Idle),
        (
            S::Ready {
                updated_at,
                count,
                flow,
                ..
            },
            PageEvent::FetchFailed(problem),
        ) => S::Ready {
            updated_at,
            count,
            flow,
            activity: PageActivity::Stale { problem },
        },
        (_, PageEvent::FetchFailed(problem)) => S::Failed { problem },
        (status, PageEvent::FetchDropped { flow } | PageEvent::FetchCancelled { flow }) => {
            settled(status, &flow)
        }
    }
}

/// A fetch ended without an answer: a ready page keeps its data, one with nothing shows "Not
/// Loaded" (not the student's doing, so no toast by default).
fn settled(status: PageStatus, flow: &str) -> PageStatus {
    match status {
        PageStatus::Ready {
            updated_at,
            count,
            flow,
            activity,
        } => PageStatus::Ready {
            updated_at,
            count,
            flow,
            activity: activity.settled(),
        },
        PageStatus::Loading { .. } => PageStatus::Failed {
            problem: Problem::not_loaded(flow),
        },
        failed @ PageStatus::Failed { .. } => failed,
    }
}

/// What wakes a watch's refresh: the screen opened, or the app came back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wake {
    Open,
    Resume,
}

/// What a wake decides from. `ready`: something is shown; `running`: its fetch still runs (the
/// screen left mid-fetch); `fetched`: a watch fetched the selected semester already; `stale`:
/// older than `PageMaxAge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Seen {
    pub(crate) ready: bool,
    pub(crate) running: bool,
    pub(crate) auto_refresh: bool,
    pub(crate) fetched: bool,
    pub(crate) stale: bool,
}

impl Default for Seen {
    fn default() -> Self {
        Self {
            ready: true,
            running: false,
            auto_refresh: true,
            fetched: true,
            stale: false,
        }
    }
}

/// Why a watch fetches on `wake`, or None for no fetch.
pub(crate) fn refresh_trigger(wake: Wake, seen: Seen) -> Option<&'static str> {
    match wake {
        Wake::Resume => (seen.auto_refresh && seen.stale).then_some("resume"),
        // Nothing to show: always fetch.
        Wake::Open if !seen.ready => Some("open"),
        // Join it (no request) to show its result.
        Wake::Open if seen.running => Some("rejoin"),
        Wake::Open if !seen.auto_refresh => None,
        Wake::Open if !seen.fetched => Some("open"),
        Wake::Open if seen.stale => Some("stale"),
        Wake::Open => None,
    }
}

/// "Waiting for Attendance to finish" behind the page ids holding the lanes; None with none.
pub(crate) fn waiting_for(pages: &[String]) -> Option<String> {
    match pages {
        [] => None,
        [page] => {
            let name = page.replace('_', " ");
            let mut chars = name.chars();
            let name: String = chars
                .next()
                .map_or_default(|first| first.to_uppercase().chain(chars).collect());
            Some(format!("Waiting for {name} to finish"))
        }
        _ => Some("Waiting for other pages to finish".into()),
    }
}

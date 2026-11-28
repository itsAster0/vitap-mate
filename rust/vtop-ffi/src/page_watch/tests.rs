//! Ports of Kotlin's `PageEventTest`, `PageActivityTest` and `RefreshTriggerTest`, and watches
//! over a fake page on a manual clock.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures::FutureExt;
use futures::future::BoxFuture;
use smol::future::block_on;

use super::state::{PageData, PageEvent, Seen, Wake, next, refresh_trigger, waiting_for};
use super::{PageActivity, PageId, PageListener, PageSource, PageStatus, PageWatch, line_fields};
use crate::problem::{Problem, ProblemKind};
use crate::runtime::{ManualClock, Runtime};
use crate::schedule::RunOpts;
use crate::settings::{SettingChange, TunableKey};
use crate::{Core, CoreError};

// The status machine.

fn offline() -> Problem {
    Problem {
        kind: ProblemKind::Offline,
        title: "Offline".into(),
        message: "Connect and try again.".into(),
        flow: "f1".into(),
        retryable: true,
        user_initiated: true,
    }
}

fn at(updated_at: i64) -> PageData {
    PageData {
        updated_at,
        count: None,
    }
}

fn ready(updated_at: i64, activity: PageActivity) -> PageStatus {
    PageStatus::Ready {
        updated_at,
        count: None,
        flow: "f".into(),
        activity,
    }
}

fn loading() -> PageStatus {
    PageStatus::Loading { waiting: None }
}

fn cache(updated_at: i64) -> PageEvent {
    PageEvent::CacheLoaded {
        data: at(updated_at),
        flow: "f".into(),
    }
}

fn fresh(updated_at: i64) -> PageEvent {
    PageEvent::FetchSucceeded {
        data: at(updated_at),
        flow: "f".into(),
    }
}

fn after(status: PageStatus, events: impl IntoIterator<Item = PageEvent>) -> PageStatus {
    events.into_iter().fold(status, next)
}

fn refreshing(waiting: Option<&str>) -> PageActivity {
    PageActivity::Refreshing {
        waiting: waiting.map(str::to_owned),
    }
}

fn stale() -> PageActivity {
    PageActivity::Stale { problem: offline() }
}

fn waiting(text: Option<&str>) -> PageEvent {
    PageEvent::WaitingChanged(text.map(str::to_owned))
}

#[test]
fn the_cache_shows_first_then_the_fetch_refreshes_it() {
    let cached = after(loading(), [cache(10)]);
    assert_eq!(cached, ready(10, PageActivity::Idle));
    let busy = after(
        cached,
        [
            PageEvent::FetchStarted,
            waiting(Some("Waiting for Marks to finish")),
        ],
    );
    assert_eq!(
        busy,
        ready(10, refreshing(Some("Waiting for Marks to finish")))
    );
    assert_eq!(
        after(busy, [waiting(None), fresh(20)]),
        ready(20, PageActivity::Idle)
    );
}

#[test]
fn a_failure_keeps_the_cache_or_fails_with_none() {
    let failed = PageEvent::FetchFailed(offline());
    assert_eq!(
        after(
            loading(),
            [cache(10), PageEvent::FetchStarted, failed.clone()]
        ),
        ready(10, stale())
    );
    assert_eq!(
        after(loading(), [PageEvent::FetchStarted, failed]),
        PageStatus::Failed { problem: offline() }
    );
    // A retry runs: the old problem is not shown again while it does.
    assert_eq!(
        next(
            PageStatus::Failed { problem: offline() },
            PageEvent::FetchStarted
        ),
        loading()
    );
}

#[test]
fn fresh_data_is_never_overwritten_by_the_cache() {
    // A cache read that comes back late finds the fresh copy and leaves it.
    let got = after(loading(), [PageEvent::FetchStarted, fresh(20)]);
    assert_eq!(next(got.clone(), cache(10)), got);
    // Over a failed fetch, the cache shows with the problem.
    assert_eq!(
        next(PageStatus::Failed { problem: offline() }, cache(10)),
        ready(10, stale())
    );
}

#[test]
fn a_fetch_ending_without_an_answer_never_leaves_the_spinner() {
    let cancelled = PageEvent::FetchCancelled { flow: "f2".into() };
    let dropped = PageEvent::FetchDropped { flow: "f3".into() };
    // Cancelled (sign-out) or dropped with nothing shown: a problem to retry from.
    let PageStatus::Failed { problem } =
        after(loading(), [PageEvent::FetchStarted, cancelled.clone()])
    else {
        panic!("not failed");
    };
    assert_eq!(problem.flow, "f2");
    assert!(problem.retryable && !problem.user_initiated);
    assert!(matches!(
        after(loading(), [PageEvent::FetchStarted, dropped.clone()]),
        PageStatus::Failed { .. }
    ));
    // With a copy shown it settles back to it.
    let busy = ready(10, refreshing(Some("Waiting")));
    assert_eq!(
        next(busy.clone(), cancelled.clone()),
        ready(10, PageActivity::Idle)
    );
    assert_eq!(next(busy, dropped), ready(10, PageActivity::Idle));
    // A problem already shown stays.
    assert_eq!(next(ready(10, stale()), cancelled), ready(10, stale()));
}

#[test]
fn only_a_fetch_waits_for_a_lane() {
    assert_eq!(
        next(loading(), waiting(Some("Waiting"))),
        PageStatus::Loading {
            waiting: Some("Waiting".into())
        }
    );
    let failed = PageStatus::Failed { problem: offline() };
    assert_eq!(next(failed.clone(), waiting(Some("Waiting"))), failed);
    let idle = ready(10, PageActivity::Idle);
    assert_eq!(next(idle.clone(), waiting(Some("Waiting"))), idle);
}

#[test]
fn only_a_running_fetch_waits_and_settles_without_an_answer() {
    assert_eq!(
        refreshing(None).waiting(Some("Waiting".into())),
        refreshing(Some("Waiting"))
    );
    assert_eq!(refreshing(Some("Waiting")).waiting(None), refreshing(None));
    assert_eq!(
        PageActivity::Idle.waiting(Some("Waiting".into())),
        PageActivity::Idle
    );
    assert_eq!(stale().waiting(Some("Waiting".into())), stale());
    assert_eq!(refreshing(Some("Waiting")).settled(), PageActivity::Idle);
    assert_eq!(stale().settled(), stale());
    assert_eq!(PageActivity::Idle.settled(), PageActivity::Idle);
}

#[test]
fn waiting_names_one_page_or_others() {
    assert_eq!(waiting_for(&[]), None);
    assert_eq!(
        waiting_for(&["grade_history".into()]).as_deref(),
        Some("Waiting for Grade history to finish")
    );
    assert_eq!(
        waiting_for(&["marks".into(), "exams".into()]).as_deref(),
        Some("Waiting for other pages to finish")
    );
}

// refreshTrigger.

#[test]
fn nothing_shown_always_fetches_on_open() {
    let seen = Seen {
        ready: false,
        running: true,
        auto_refresh: false,
        ..Seen::default()
    };
    assert_eq!(refresh_trigger(Wake::Open, seen), Some("open"));
}

#[test]
fn a_fetch_still_running_is_rejoined_even_with_auto_refresh_off() {
    let seen = Seen {
        running: true,
        auto_refresh: false,
        stale: true,
        ..Seen::default()
    };
    assert_eq!(refresh_trigger(Wake::Open, seen), Some("rejoin"));
}

#[test]
fn auto_refresh_off_leaves_the_cache_on_open() {
    let seen = Seen {
        auto_refresh: false,
        fetched: false,
        stale: true,
        ..Seen::default()
    };
    assert_eq!(refresh_trigger(Wake::Open, seen), None);
}

#[test]
fn a_semester_not_fetched_yet_fetches_on_open() {
    let seen = Seen {
        fetched: false,
        stale: true,
        ..Seen::default()
    };
    assert_eq!(refresh_trigger(Wake::Open, seen), Some("open"));
}

#[test]
fn old_data_fetches_on_open_and_fresh_data_does_not() {
    let old = Seen {
        stale: true,
        ..Seen::default()
    };
    assert_eq!(refresh_trigger(Wake::Open, old), Some("stale"));
    assert_eq!(refresh_trigger(Wake::Open, Seen::default()), None);
}

#[test]
fn coming_back_fetches_only_old_data_with_auto_refresh_on() {
    let old = Seen {
        stale: true,
        ..Seen::default()
    };
    assert_eq!(refresh_trigger(Wake::Resume, old), Some("resume"));
    assert_eq!(refresh_trigger(Wake::Resume, Seen::default()), None);
    let off = Seen {
        auto_refresh: false,
        ..old
    };
    assert_eq!(refresh_trigger(Wake::Resume, off), None);
}

#[test]
fn log_fields_match_kotlin() {
    let shown = PageStatus::Ready {
        updated_at: 100,
        count: Some(7),
        flow: "f".into(),
        activity: stale(),
    };
    let fields = line_fields(
        "stale_kept",
        &shown,
        42,
        160,
        &[("error", "Offline".into())],
    );
    let keys: Vec<&str> = fields.iter().map(|(key, _)| *key).collect();
    assert_eq!(
        keys,
        ["state", "kind", "ms", "count", "age_s", "problem", "error"]
    );
    let values: Vec<&str> = fields.iter().map(|(_, value)| value.as_str()).collect();
    assert_eq!(
        values,
        ["stale_kept", "Ready", "42", "7", "60", "Offline", "Offline"]
    );
    let none = line_fields("no_cache", &loading(), 3, 0, &[]);
    assert_eq!(
        none.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
        ["state", "kind", "ms"]
    );
}

// Watches over a fake page.

/// A page whose cache and fetch the test sets; a fetch waits for `gate` (when set) and saves
/// what it got, as the real ones do.
#[derive(Clone, Default)]
struct Fake(Arc<FakeState>);

#[derive(Default)]
struct FakeState {
    saved: Mutex<Option<PageData>>,
    answer: Mutex<Option<PageData>>,
    gate: Mutex<Option<smol::channel::Receiver<()>>>,
    fetches: AtomicUsize,
}

impl Fake {
    fn saved(&self, data: Option<PageData>) {
        *self.0.saved.lock().unwrap() = data;
    }

    fn answer(&self, updated_at: i64) {
        *self.0.answer.lock().unwrap() = Some(PageData {
            updated_at,
            count: Some(1),
        });
    }

    /// Fetches wait until the returned sender closes.
    fn gated(&self) -> smol::channel::Sender<()> {
        let (open, wait) = smol::channel::bounded(1);
        *self.0.gate.lock().unwrap() = Some(wait);
        open
    }

    fn fetches(&self) -> usize {
        self.0.fetches.load(Ordering::SeqCst)
    }
}

impl PageSource for Fake {
    fn cached(&self, _flow: &str) -> Option<PageData> {
        *self.0.saved.lock().unwrap()
    }

    fn fetch(&self, _flow: String) -> BoxFuture<'static, Result<PageData, CoreError>> {
        let state = self.0.clone();
        state.fetches.fetch_add(1, Ordering::SeqCst);
        async move {
            let gate = state.gate.lock().unwrap().clone();
            if let Some(gate) = gate {
                gate.recv().await.ok();
            }
            let answer = state.answer.lock().unwrap().ok_or(CoreError::Offline)?;
            *state.saved.lock().unwrap() = Some(answer);
            Ok(answer)
        }
        .boxed()
    }
}

/// Every status, in order.
struct Heard(smol::channel::Sender<PageStatus>);

impl PageListener for Heard {
    fn on_status(&self, status: PageStatus) {
        self.0.try_send(status).unwrap();
    }
}

struct Lab {
    core: Arc<Core>,
    clock: ManualClock,
    page: Fake,
}

fn lab(name: &str) -> Lab {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-watch-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let clock = ManualClock::default();
    let core = Core::for_test(&dir);
    let runtime = Arc::new(Runtime::new(Arc::new(clock.clone())));
    let (events, schedule) = crate::events::wired_on(runtime, core.settings.clone());
    Lab {
        core: Arc::new(Core {
            events,
            schedule,
            ..core
        }),
        clock,
        page: Fake::default(),
    }
}

impl Lab {
    fn open(&self) -> (Arc<PageWatch>, smol::channel::Receiver<PageStatus>) {
        let (heard, statuses) = smol::channel::unbounded();
        let watch = PageWatch::open(
            self.core.clone(),
            "f.open",
            PageId::Marks,
            "",
            Arc::new(self.page.clone()),
            Arc::new(Heard(heard)),
        );
        (watch, statuses)
    }

    /// [`Lab::open`], once its fetch holds a lane (it shows Loading a moment before).
    fn open_fetching(&self) -> (Arc<PageWatch>, smol::channel::Receiver<PageStatus>) {
        let flight = &self.core.schedule.flight;
        let changes = flight.watch_running();
        let opened = self.open();
        while !flight.is_running("marks//null") {
            block_on(changes.recv()).unwrap();
        }
        opened
    }

    /// Waits until nothing runs under `key`.
    fn settle(&self, key: &str) {
        let flight = &self.core.schedule.flight;
        let changes = flight.watch_running();
        while flight.is_running(key) {
            block_on(changes.recv()).unwrap();
        }
    }
}

/// The next status that matches.
fn until(
    statuses: &smol::channel::Receiver<PageStatus>,
    wanted: impl Fn(&PageStatus) -> bool,
) -> PageStatus {
    loop {
        let status = block_on(statuses.recv()).unwrap();
        if wanted(&status) {
            return status;
        }
    }
}

fn is_ready_at(updated_at: i64) -> impl Fn(&PageStatus) -> bool {
    move |status| matches!(status, PageStatus::Ready { updated_at: at, activity: PageActivity::Idle, .. } if *at == updated_at)
}

fn is_loading(status: &PageStatus) -> bool {
    matches!(status, PageStatus::Loading { waiting: None })
}

#[test]
fn open_reads_cache_then_fetches_when_none() {
    let lab = lab("open");
    lab.page.answer(100);
    let (watch, statuses) = lab.open();
    assert_eq!(watch.key(), "marks//null");
    until(&statuses, is_loading);
    until(&statuses, is_ready_at(100));
    block_on(watch.opened.clone());
    assert_eq!(lab.page.fetches(), 1);
    assert!(lab.core.schedule.fetched("marks//null"));
}

#[test]
fn open_rejoins_running_fetch_without_second_request() {
    let lab = lab("rejoin");
    lab.page.answer(100);
    let gate = lab.page.gated();
    let (first, statuses) = lab.open_fetching();
    until(&statuses, is_loading);
    // The tab goes mid-fetch; the fetch runs on.
    first.stop();
    lab.page.saved(Some(at(50)));
    let (again, statuses) = lab.open();
    until(&statuses, |s| {
        matches!(
            s,
            PageStatus::Ready {
                activity: PageActivity::Refreshing { .. },
                ..
            }
        )
    });
    gate.close();
    until(&statuses, is_ready_at(100));
    assert_eq!(lab.page.fetches(), 1);
    block_on(again.opened.clone());
}

#[test]
fn open_within_just_fetched_skips_request() {
    let lab = lab("just");
    lab.page.answer(100);
    let gate = lab.page.gated();
    let (first, statuses) = lab.open_fetching();
    until(&statuses, is_loading);
    first.stop();
    gate.close();
    lab.settle("marks//null");
    // Shown again: the cache is that fetch's result, so no second request.
    let (again, statuses) = lab.open();
    block_on(again.opened.clone());
    assert_eq!(
        block_on(statuses.recv()).unwrap(),
        PageStatus::Ready {
            updated_at: 100,
            count: Some(1),
            flow: "f.open".into(),
            activity: PageActivity::Idle,
        }
    );
    assert!(statuses.is_empty());
    assert_eq!(lab.page.fetches(), 1);
    assert!(lab.core.schedule.fetched("marks//null"));
}

#[test]
fn resume_fetches_only_when_stale_and_auto_refresh() {
    let lab = lab("resume");
    lab.page.saved(Some(at(0)));
    lab.core.schedule.mark_fetched("marks//null");
    // The app was shown and left.
    lab.core.app_shown("f".into(), true);
    lab.core.app_shown("f".into(), false);
    let (watch, statuses) = lab.open();
    block_on(watch.opened.clone());
    // Fresh at open: nothing fetched. Back 7 h later (PageMaxAge is 6): fetched.
    lab.clock.advance(7 * 3_600_000);
    lab.page.answer(25_200);
    lab.core.app_shown("f".into(), true);
    until(&statuses, is_ready_at(25_200));
    assert_eq!(lab.page.fetches(), 1);
}

#[test]
fn waiting_text_after_300ms_names_running_page() {
    let lab = lab("waiting");
    lab.core.settings.set(SettingChange::Tunable {
        key: TunableKey::FetchLanes,
        value: Some(1),
    });
    let (open, hold) = smol::channel::bounded::<()>(1);
    let blocker =
        lab.core
            .schedule
            .flight
            .start("attendance//null", "f.b", RunOpts::default(), async move {
                hold.recv().await.ok();
                Ok(at(1))
            });
    lab.page.answer(100);
    let (_watch, statuses) = lab.open();
    until(&statuses, is_loading);
    lab.clock.wait_for_sleepers(1);
    lab.clock.advance(300);
    assert_eq!(
        until(&statuses, |s| matches!(
            s,
            PageStatus::Loading { waiting: Some(_) }
        )),
        PageStatus::Loading {
            waiting: Some("Waiting for Attendance to finish".into())
        }
    );
    open.close();
    block_on(blocker.result).unwrap();
    until(&statuses, is_loading);
    until(&statuses, is_ready_at(100));
}

#[test]
fn close_stops_all_listener_calls() {
    let lab = lab("close");
    lab.page.answer(100);
    let gate = lab.page.gated();
    let (watch, statuses) = lab.open_fetching();
    until(&statuses, is_loading);
    watch.stop();
    gate.close();
    lab.settle("marks//null");
    assert!(statuses.is_empty());
    // The fetch still counted: it ran to the end in the queue.
    assert!(
        lab.core
            .schedule
            .flight
            .succeeded_within("marks//null", 60_000)
    );
}

#[test]
fn cancel_all_turns_loading_into_not_loaded_without_toast() {
    let lab = lab("cancel");
    lab.page.answer(100);
    let _gate = lab.page.gated();
    let (_watch, statuses) = lab.open_fetching();
    until(&statuses, is_loading);
    lab.core.schedule.cancel_all();
    let PageStatus::Failed { problem } =
        until(&statuses, |s| matches!(s, PageStatus::Failed { .. }))
    else {
        unreachable!();
    };
    assert_eq!(problem.title, "Not Loaded");
    assert!(!problem.toasts(false));
}

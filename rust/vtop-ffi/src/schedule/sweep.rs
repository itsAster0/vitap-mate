//! The open-app stale refresher: a port of Kotlin's `StaleSweep`.
//!
//! After a quick page fetch (the session is live and the network good), at most once every
//! `SweepCooldown` minutes, while the app is shown and Auto Refresh is on, pages not on screen
//! that are older than `SweepMaxAge` hours are fetched: attendance, timetable, marks, exams and
//! the semester list as one batch (one round trip on the Server and UDP routes,
//! [`Core::fetch_pages`]), then the calendar when a day old, then mismatched attendance histories
//! one at a time. It doesn't start on a slow link, waits while anything else talks to VTOP plus
//! [`QUIET_MS`], takes [`super::Flight`]'s lanes at the lowest priority and stops at the first
//! failure, slow or dropped page so it never competes with the student.
//! Flow `sync.stale`, log src `sync.stale`.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};

use futures::FutureExt;
use futures::future::BoxFuture;

use super::flight::{FlightError, RunOpts};
use super::freshness::{FlowRun, flow_id, log, report_background_problem};
use crate::settings::TunableKey;
use crate::{Core, CoreError};

/// After the student's requests finish, wait this long in case more follow.
pub(crate) const QUIET_MS: u64 = 2_000;

/// `(page id, engine name)` in the order the sweep fetches them.
const BATCH_CANDIDATES: [(&str, &str); 5] = [
    ("attendance", "attendance"),
    ("timetable", "timetable"),
    ("marks", "marks"),
    ("exams", "exam_schedule"),
    ("semesters", "semesters"),
];

/// How far a sweep got: on to the next page, or stopped for a reason.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Continue,
    Stop(String),
}

/// Cache reads and VTOP fetches for [`Sweep`]; mocked in tests.
pub(crate) trait SweepSource: Send + Sync {
    /// Engine names [`Core::fetch_pages`] accepts (`batched_pages()`).
    fn batched_pages(&self) -> Vec<String>;
    /// Cached `updated_at` (unix seconds) of a batch page (`attendance`, `timetable`, `marks`,
    /// `exams`, `semesters`).
    fn cached_at(&self, flow: &str, id: &str) -> Option<i64>;
    /// Fetches the given engine page names in one batch; fails if the call or any page fails.
    fn fetch_pages(
        &self,
        flow: String,
        names: Vec<String>,
    ) -> BoxFuture<'static, Result<(), CoreError>>;
    /// The picked semester id from `semester.json`, if valid.
    fn selected_semester(&self) -> Option<String>;
    /// Refreshes `semester`'s academic calendar if old (`anyway = true`, src `"sync"`).
    fn refresh_calendar(&self, semester: String) -> BoxFuture<'static, bool>;
    /// Course keys whose attendance history is missing or behind the summary.
    fn mismatched_attendance_details(&self, flow: &str) -> Vec<String>;
    /// Fetches one course's attendance history.
    fn fetch_attendance_detail(
        &self,
        flow: String,
        key: String,
    ) -> BoxFuture<'static, Result<(), CoreError>>;
}

struct CoreSweep(Arc<Core>);

impl SweepSource for CoreSweep {
    fn batched_pages(&self) -> Vec<String> {
        crate::remote::route::batch::batched_pages()
    }

    fn cached_at(&self, flow: &str, id: &str) -> Option<i64> {
        let f = flow.to_owned();
        match id {
            "attendance" => self.0.cached_attendance(f).map(|v| v.updated_at),
            "timetable" => self.0.cached_timetable(f).map(|v| v.updated_at),
            "marks" => self.0.cached_marks(f).map(|v| v.updated_at),
            "exams" => self.0.cached_exams(f).map(|v| v.updated_at),
            "semesters" => self.0.cached_semesters(f).map(|v| v.updated_at),
            _ => None,
        }
    }

    fn fetch_pages(
        &self,
        flow: String,
        names: Vec<String>,
    ) -> BoxFuture<'static, Result<(), CoreError>> {
        let core = self.0.clone();
        async move {
            let pages = core.fetch_pages(flow, names).await?;
            if let Some(error) = pages.into_iter().find_map(|page| page.error) {
                return Err(error);
            }
            Ok(())
        }
        .boxed()
    }

    fn selected_semester(&self) -> Option<String> {
        self.0.selected_semester()
    }

    fn refresh_calendar(&self, semester: String) -> BoxFuture<'static, bool> {
        let core = self.0.clone();
        async move {
            core.schedule
                .freshness
                .calendar_if_old(&core, &semester, "sync", true)
                .await
        }
        .boxed()
    }

    fn mismatched_attendance_details(&self, flow: &str) -> Vec<String> {
        self.0.mismatched_attendance_details(flow.to_owned())
    }

    fn fetch_attendance_detail(
        &self,
        flow: String,
        key: String,
    ) -> BoxFuture<'static, Result<(), CoreError>> {
        let core = self.0.clone();
        async move {
            core.fetch_attendance_detail(flow, key).await?;
            Ok(())
        }
        .boxed()
    }
}

#[derive(Default)]
struct SweepState {
    running: bool,
    last_run: Option<i64>,
    task: Option<smol::Task<()>>,
    #[cfg(test)]
    done: Vec<futures::channel::oneshot::Sender<Step>>,
}

/// Cooldown and single-run coordinator for the open-app stale sweep.
#[derive(Default)]
pub(crate) struct Sweep {
    state: Mutex<SweepState>,
}

impl Sweep {
    fn lock(&self) -> std::sync::MutexGuard<'_, SweepState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A page fetch of `ms` succeeded over a link worth judging.
    pub(crate) fn after_fetch(&self, core: &Arc<Core>, ms: i64) {
        self.after_fetch_with(core, ms, Arc::new(CoreSweep(core.clone())));
    }

    pub(crate) fn after_fetch_with(
        &self,
        core: &Arc<Core>,
        ms: i64,
        source: Arc<dyn SweepSource>,
    ) -> bool {
        let settings = core.settings();
        let quick_ms = i64::from(settings.tunable(TunableKey::SweepQuick)) * 1000;
        if ms > quick_ms
            || core.schedule.flight.slow_link()
            || !settings.auto_refresh
            || !core.events.on_screen()
        {
            return false;
        }
        let now = core.schedule.runtime.clock().now_ms();
        let cooldown_ms = i64::from(settings.tunable(TunableKey::SweepCooldown)) * 60_000;
        let mut state = self.lock();
        if state.running {
            return false;
        }
        if let Some(last) = state.last_run
            && now - last < cooldown_ms
        {
            return false;
        }
        state.running = true;
        state.last_run = Some(now);
        let core_run = core.clone();
        let id = flow_id();
        let task = core.schedule.runtime.spawn(&id.clone(), async move {
            let mut guard = SweepRunning {
                core: &core_run,
                end: None,
            };
            let end = run_sweep(&core_run, source.as_ref(), id).await;
            guard.end = Some(end);
        });
        state.task = Some(task);
        true
    }

    /// Sign-out or account switch: cancels any running sweep and resets the cooldown.
    pub(crate) fn cancel(&self) {
        let task = {
            let mut state = self.lock();
            state.running = false;
            state.last_run = None;
            state.task.take()
        };
        drop(task);
    }

    #[cfg(test)]
    fn wait_done(&self) -> futures::channel::oneshot::Receiver<Step> {
        let (tx, rx) = futures::channel::oneshot::channel();
        let mut state = self.lock();
        if !state.running {
            let _ = tx.send(Step::Continue);
        } else {
            state.done.push(tx);
        }
        rx
    }
}

struct SweepRunning<'a> {
    core: &'a Core,
    end: Option<Step>,
}

impl Drop for SweepRunning<'_> {
    fn drop(&mut self) {
        let mut state = self.core.schedule.sweep.lock();
        state.running = false;
        #[cfg(test)]
        {
            let outcome = self
                .end
                .take()
                .unwrap_or_else(|| Step::Stop("cancelled".into()));
            for tx in state.done.drain(..) {
                let _ = tx.send(outcome.clone());
            }
        }
    }
}

async fn run_sweep(core: &Arc<Core>, source: &dyn SweepSource, id: String) -> Step {
    let flow = FlowRun::start_with_id(core, "sync.stale", id, true);
    let mut fetched = 0usize;
    let end = sweep_pages(core, source, flow.id(), &mut fetched).await;
    match &end {
        Step::Continue => log(
            "info",
            "sync.stale",
            flow.id(),
            "done",
            vec![("fetched", fetched.to_string())],
        ),
        Step::Stop(reason) => log(
            "info",
            "sync.stale",
            flow.id(),
            &format!("stopped: {reason}"),
            vec![("fetched", fetched.to_string())],
        ),
    }
    flow.done();
    end
}

/// Page `id`'s saved time (`0` for none), or None when it is on screen or fresh enough.
fn due(core: &Core, id: &str, updated_at: Option<i64>) -> Option<i64> {
    // Pages on screen refresh themselves.
    if core.schedule.flight.is_shown(id) {
        return None;
    }
    let Some(at) = updated_at else {
        return Some(0);
    };
    let now_s = core.schedule.runtime.clock().now_ms() / 1000;
    let max_age_s = i64::from(core.settings().tunable(TunableKey::SweepMaxAge)) * 3600;
    (now_s - at >= max_age_s).then_some(at)
}

/// Waits for idle activity and [`QUIET_MS`], then runs `fetch` through [`super::Flight`] at
/// background priority. Stops on app leave, auto-refresh off, drop, cancel, error, or slow page.
async fn run_step(
    core: &Arc<Core>,
    flow: &str,
    id: &str,
    label: &str,
    ats: &[i64],
    fetched: &mut usize,
    fetch: BoxFuture<'static, Result<(), CoreError>>,
) -> Step {
    core.schedule.activity.wait_idle().await;
    core.schedule.runtime.clock().sleep(QUIET_MS).await;
    if !core.settings().auto_refresh || !core.events.on_screen() {
        return Step::Stop("app left or auto refresh off".into());
    }
    let key = format!("{id}/stale/");
    let opts = RunOpts {
        user: false,
        has_cache: ats.iter().all(|&at| at != 0),
        background: true,
    };
    let core_run = core.clone();
    let label_owned = label.to_owned();
    let flow_owned = flow.to_owned();
    let run = core.schedule.flight.start(&key, flow, opts, async move {
        let clock = core_run.schedule.runtime.clock();
        let start = clock.now_ms();
        core_run
            .schedule
            .activity
            .track(&label_owned, &flow_owned, fetch)
            .await?;
        Ok(clock.now_ms() - start)
    });
    let ms = match run.result.await {
        Ok(ms) => ms,
        Err(FlightError::Dropped) => return Step::Stop(format!("{id} waited too long")),
        Err(FlightError::Cancelled) => return Step::Stop(format!("{id} cancelled")),
        Err(FlightError::Failed(error)) => {
            report_background_problem(core, &error, flow);
            return Step::Stop(format!("{id} failed: {}", error.kind()));
        }
    };
    *fetched += ats.len();
    let now_s = core.schedule.runtime.clock().now_ms() / 1000;
    let ages = ats
        .iter()
        .map(|&at| {
            if at == 0 {
                "null".to_owned()
            } else {
                (now_s - at).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    log(
        "info",
        "sync.stale",
        flow,
        &format!("fetched {id}"),
        vec![
            ("page", id.to_owned()),
            ("ms", ms.to_string()),
            ("age_s", ages),
        ],
    );
    let slow_limit =
        i64::from(core.settings().tunable(TunableKey::SlowPage)) * 1000 * (ats.len() as i64);
    if ms > slow_limit {
        Step::Stop(format!("network is slow ({ms} ms for {} pages)", ats.len()))
    } else {
        Step::Continue
    }
}

async fn sweep_pages(
    core: &Arc<Core>,
    source: &dyn SweepSource,
    flow: &str,
    fetched: &mut usize,
) -> Step {
    // Only pages Rust batches (route/batch.rs); the calendar refreshes on its own below.
    let batched: HashSet<String> = source.batched_pages().into_iter().collect();
    let batch: Vec<(&str, &str, i64)> = BATCH_CANDIDATES
        .iter()
        .copied()
        .filter(|&(id, name)| {
            let ok = batched.contains(name);
            if !ok {
                log(
                    "warn",
                    "sync.batch",
                    flow,
                    &format!("{name} is not batched in Rust"),
                    vec![("page", id.to_owned())],
                );
            }
            ok
        })
        .filter_map(|(id, name)| due(core, id, source.cached_at(flow, id)).map(|at| (id, name, at)))
        .collect();
    if !batch.is_empty() {
        let ids: Vec<&str> = batch.iter().map(|(id, _, _)| *id).collect();
        let id = ids.join("+");
        let label = format!("Refreshing {}", ids.join(", "));
        let ats: Vec<i64> = batch.iter().map(|(_, _, at)| *at).collect();
        let names: Vec<String> = batch
            .iter()
            .map(|(_, name, _)| (*name).to_owned())
            .collect();
        let step = run_step(
            core,
            flow,
            &id,
            &label,
            &ats,
            fetched,
            source.fetch_pages(flow.to_owned(), names),
        )
        .await;
        if let Step::Stop(reason) = step {
            return Step::Stop(reason);
        }
    }
    if let Some(semester) = source.selected_semester() {
        source.refresh_calendar(semester).await;
    }
    if !core.schedule.flight.is_shown("attendance") {
        for key in source.mismatched_attendance_details(flow) {
            let id = format!("history.{key}");
            let Some(at) = due(core, &id, None) else {
                continue;
            };
            let step = run_step(
                core,
                flow,
                &id,
                "Refreshing class history",
                &[at],
                fetched,
                source.fetch_attendance_detail(flow.to_owned(), key),
            )
            .await;
            if let Step::Stop(reason) = step {
                return Step::Stop(reason);
            }
        }
    }
    Step::Continue
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use futures::FutureExt;
    use futures::future::BoxFuture;
    use smol::future::{block_on, poll_once};

    use super::*;
    use crate::events::AppEvents;
    use crate::problem::Problem;
    use crate::remote::consent::ConsentAsk;
    use crate::remote::handover::HandoverStatus;
    use crate::runtime::{Clock, ManualClock, Runtime};
    use crate::schedule::{ActivityEntry, TrackEnd};
    use crate::settings::SettingChange;

    struct FakeSweepState {
        batched: Mutex<Vec<String>>,
        cached: Mutex<HashMap<String, i64>>,
        batch_calls: Mutex<Vec<Vec<String>>>,
        batch_result: Mutex<Result<(), CoreError>>,
        batch_advance_ms: Mutex<u64>,
        clock: ManualClock,
        semester: Mutex<Option<String>>,
        calendar_calls: AtomicUsize,
        mismatched: Mutex<Vec<String>>,
        history_calls: Mutex<Vec<String>>,
        history_result: Mutex<Result<(), CoreError>>,
    }

    #[derive(Clone)]
    struct FakeSweep(Arc<FakeSweepState>);

    impl FakeSweep {
        fn new(clock: ManualClock) -> Self {
            Self(Arc::new(FakeSweepState {
                batched: Mutex::new(crate::remote::route::batch::batched_pages()),
                cached: Mutex::default(),
                batch_calls: Mutex::default(),
                batch_result: Mutex::new(Ok(())),
                batch_advance_ms: Mutex::new(0),
                clock,
                semester: Mutex::new(Some("SEM1".into())),
                calendar_calls: AtomicUsize::new(0),
                mismatched: Mutex::default(),
                history_calls: Mutex::default(),
                history_result: Mutex::new(Ok(())),
            }))
        }

        fn set_batched(&self, names: Vec<&str>) {
            *self.0.batched.lock().unwrap() = names.into_iter().map(str::to_owned).collect();
        }

        fn set_cached(&self, id: &str, updated_at: i64) {
            self.0.cached.lock().unwrap().insert(id.into(), updated_at);
        }

        fn set_batch_result(&self, res: Result<(), CoreError>) {
            *self.0.batch_result.lock().unwrap() = res;
        }

        fn set_batch_duration_ms(&self, ms: u64) {
            *self.0.batch_advance_ms.lock().unwrap() = ms;
        }

        fn set_mismatched(&self, keys: Vec<&str>) {
            *self.0.mismatched.lock().unwrap() = keys.into_iter().map(str::to_owned).collect();
        }

        fn batch_calls(&self) -> Vec<Vec<String>> {
            self.0.batch_calls.lock().unwrap().clone()
        }

        fn calendar_calls(&self) -> usize {
            self.0.calendar_calls.load(Ordering::SeqCst)
        }

        fn history_calls(&self) -> Vec<String> {
            self.0.history_calls.lock().unwrap().clone()
        }
    }

    impl SweepSource for FakeSweep {
        fn batched_pages(&self) -> Vec<String> {
            self.0.batched.lock().unwrap().clone()
        }

        fn cached_at(&self, _flow: &str, id: &str) -> Option<i64> {
            self.0.cached.lock().unwrap().get(id).copied()
        }

        fn fetch_pages(
            &self,
            _flow: String,
            names: Vec<String>,
        ) -> BoxFuture<'static, Result<(), CoreError>> {
            let state = self.0.clone();
            async move {
                state.batch_calls.lock().unwrap().push(names);
                let advance = *state.batch_advance_ms.lock().unwrap();
                if advance > 0 {
                    state.clock.advance(advance);
                }
                state.batch_result.lock().unwrap().clone()
            }
            .boxed()
        }

        fn selected_semester(&self) -> Option<String> {
            self.0.semester.lock().unwrap().clone()
        }

        fn refresh_calendar(&self, _semester: String) -> BoxFuture<'static, bool> {
            let state = self.0.clone();
            async move {
                state.calendar_calls.fetch_add(1, Ordering::SeqCst);
                true
            }
            .boxed()
        }

        fn mismatched_attendance_details(&self, _flow: &str) -> Vec<String> {
            self.0.mismatched.lock().unwrap().clone()
        }

        fn fetch_attendance_detail(
            &self,
            _flow: String,
            key: String,
        ) -> BoxFuture<'static, Result<(), CoreError>> {
            let state = self.0.clone();
            async move {
                state.history_calls.lock().unwrap().push(key);
                state.history_result.lock().unwrap().clone()
            }
            .boxed()
        }
    }

    #[derive(Default)]
    struct HeardProblems(Mutex<Vec<Problem>>);

    impl AppEvents for HeardProblems {
        fn on_background_problem(&self, problem: Problem) {
            self.0.lock().unwrap().push(problem);
        }
        fn on_activity(&self, _entries: Vec<ActivityEntry>) {}
        fn on_semesters(&self, _state: crate::SemesterState) {}
        fn on_sync_health(&self, _health: crate::SyncHealth, _paused: Option<String>) {}
        fn on_consent(&self, _ask: ConsentAsk) {}
        fn on_handover_status(&self, _flow: String, _request_id: String, _status: HandoverStatus) {}
        fn is_flow_unattended(&self, _flow: String) -> bool {
            false
        }
    }

    struct Lab {
        core: Arc<Core>,
        clock: ManualClock,
        fake: FakeSweep,
        problems: Arc<HeardProblems>,
    }

    fn lab(name: &str) -> Lab {
        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-sweep-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let clock = ManualClock::default();
        // Start well past epoch so cached updated_at values are positive.
        clock.advance(100 * 3_600_000);
        let core = Core::for_test(&dir);
        let runtime = Arc::new(Runtime::new(Arc::new(clock.clone())));
        let (events, schedule) = crate::events::wired_on(runtime, core.settings.clone());
        let core = Arc::new(Core {
            events,
            schedule,
            ..core
        });
        let problems = Arc::new(HeardProblems::default());
        core.set_events(problems.clone());
        core.events.set_shown_for_test(true);
        let fake = FakeSweep::new(clock.clone());
        Lab {
            core,
            clock,
            fake,
            problems,
        }
    }

    #[test]
    fn sweep_waits_for_quiet_and_idle_activity() {
        let lab = lab("quiet");
        let active = lab
            .core
            .schedule
            .activity
            .start("f.user", "Refreshing attendance");
        assert!(lab.core.schedule.sweep.after_fetch_with(
            &lab.core,
            100,
            Arc::new(lab.fake.clone())
        ));
        let mut done = lab.core.schedule.sweep.wait_done();

        // While user activity is running, advancing past QUIET_MS does not trigger the batch.
        lab.clock.advance(QUIET_MS);
        assert!(block_on(poll_once(&mut done)).is_none());
        assert!(lab.fake.batch_calls().is_empty());

        // Once activity ends, the sweep enters the QUIET_MS sleep.
        lab.core.schedule.activity.end(active, TrackEnd::Done);
        lab.clock.wait_for_sleepers(1);
        lab.clock.advance(QUIET_MS - 1);
        assert!(block_on(poll_once(&mut done)).is_none());
        assert!(lab.fake.batch_calls().is_empty());

        // Reaching QUIET_MS runs the batch and calendar refresh.
        lab.clock.advance(1);
        assert_eq!(block_on(done).unwrap(), Step::Continue);
        assert_eq!(lab.fake.batch_calls().len(), 1);
        assert_eq!(lab.fake.calendar_calls(), 1);
    }

    #[test]
    fn sweep_stops_at_first_failure_slow_or_dropped() {
        // 1) Failure: stops immediately; calendar and histories don't run; toasts only when enabled.
        {
            let lab = lab("fail");
            lab.fake.set_batch_result(Err(CoreError::Offline));
            lab.fake.set_mismatched(vec!["CSE1001.theory"]);
            lab.core
                .settings
                .set(SettingChange::ShowBackgroundErrors { value: true });
            assert!(lab.core.schedule.sweep.after_fetch_with(
                &lab.core,
                100,
                Arc::new(lab.fake.clone())
            ));
            let done = lab.core.schedule.sweep.wait_done();
            lab.clock.wait_for_sleepers(1);
            lab.clock.advance(QUIET_MS);
            let end = block_on(done).unwrap();
            assert!(
                matches!(&end, Step::Stop(reason) if reason.contains("failed: Offline")),
                "{end:?}"
            );
            assert_eq!(lab.fake.calendar_calls(), 0);
            assert!(lab.fake.history_calls().is_empty());
            assert_eq!(lab.problems.0.lock().unwrap().len(), 1);
            assert!(!lab.problems.0.lock().unwrap()[0].user_initiated);
        }

        // 2) Slow page: batch succeeds but exceeds SlowPage * count; stops before calendar/histories.
        {
            let lab = lab("slow");
            let slow_page_s =
                u64::try_from(lab.core.settings().tunable(TunableKey::SlowPage)).unwrap();
            // 5 pages in batch -> limit is slow_page_s * 1000 * 5.
            lab.fake.set_batch_duration_ms(slow_page_s * 1000 * 5 + 1);
            lab.fake.set_mismatched(vec!["CSE1001.theory"]);
            assert!(lab.core.schedule.sweep.after_fetch_with(
                &lab.core,
                100,
                Arc::new(lab.fake.clone())
            ));
            let done = lab.core.schedule.sweep.wait_done();
            lab.clock.wait_for_sleepers(1);
            lab.clock.advance(QUIET_MS);
            let end = block_on(done).unwrap();
            assert!(
                matches!(&end, Step::Stop(reason) if reason.starts_with("network is slow")),
                "{end:?}"
            );
            assert_eq!(lab.fake.calendar_calls(), 0);
            assert!(lab.fake.history_calls().is_empty());
        }

        // 3) Dropped: lane held past QueueDrop for a cached batch -> stops without background toast.
        {
            let lab = lab("drop");
            lab.core
                .settings
                .set(SettingChange::ShowBackgroundErrors { value: true });
            lab.core.settings.set(SettingChange::Tunable {
                key: TunableKey::FetchLanes,
                value: Some(1),
            });
            let stale_at = lab.clock.now_ms() / 1000 - 20 * 3600;
            for (id, _) in BATCH_CANDIDATES {
                lab.fake.set_cached(id, stale_at);
            }
            let (release, hold) = smol::channel::bounded::<()>(1);
            let blocker = lab.core.schedule.flight.start(
                "profile//SEM1",
                "f.hold",
                RunOpts::default(),
                async move {
                    let _ = hold.recv().await;
                    Ok(())
                },
            );
            assert!(lab.core.schedule.sweep.after_fetch_with(
                &lab.core,
                100,
                Arc::new(lab.fake.clone())
            ));
            let done = lab.core.schedule.sweep.wait_done();
            lab.clock.wait_for_sleepers(1);
            lab.clock.advance(QUIET_MS);
            // Wait until the sweep ticket is queued in Flight.
            let batch_key = "attendance+timetable+marks+exams+semesters/stale/";
            while !lab.core.schedule.flight.is_running(batch_key) {
                std::thread::yield_now();
            }
            // Advance past QueueDrop (default 60 s), then release blocker so the queue drops the sweep.
            let drop_ms =
                u64::try_from(lab.core.settings().tunable(TunableKey::QueueDrop)).unwrap() * 1000
                    + 1;
            lab.clock.advance(drop_ms);
            release.close();
            block_on(blocker.result).unwrap();
            let end = block_on(done).unwrap();
            assert!(
                matches!(&end, Step::Stop(reason) if reason.ends_with("waited too long")),
                "{end:?}"
            );
            assert!(lab.fake.batch_calls().is_empty());
            assert!(lab.problems.0.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn sweep_skips_pages_on_screen() {
        let lab = lab("shown");
        lab.core.schedule.flight.shown("attendance", true);
        lab.core.schedule.flight.shown("marks", true);
        lab.fake.set_mismatched(vec!["CSE1001.theory"]);
        assert!(lab.core.schedule.sweep.after_fetch_with(
            &lab.core,
            100,
            Arc::new(lab.fake.clone())
        ));
        let done = lab.core.schedule.sweep.wait_done();
        lab.clock.wait_for_sleepers(1);
        lab.clock.advance(QUIET_MS);
        assert_eq!(block_on(done).unwrap(), Step::Continue);
        // Batch omitted attendance and marks; histories skipped because attendance is shown.
        assert_eq!(
            lab.fake.batch_calls(),
            vec![vec![
                "timetable".to_owned(),
                "exam_schedule".to_owned(),
                "semesters".to_owned(),
            ]]
        );
        assert!(lab.fake.history_calls().is_empty());

        // Unshow attendance, mark one history as shown, advance past cooldown: only the other runs.
        lab.core.schedule.flight.shown("attendance", false);
        lab.core.schedule.flight.shown("timetable", true);
        lab.core.schedule.flight.shown("exams", true);
        lab.core.schedule.flight.shown("semesters", true);
        let fresh_at = lab.clock.now_ms() / 1000;
        lab.fake.set_cached("attendance", fresh_at);
        lab.core
            .schedule
            .flight
            .shown("history.CSE1001.theory", true);
        lab.fake
            .set_mismatched(vec!["CSE1001.theory", "CSE2001.theory"]);
        let cooldown_ms =
            u64::try_from(lab.core.settings().tunable(TunableKey::SweepCooldown)).unwrap() * 60_000;
        lab.clock.advance(cooldown_ms);
        assert!(lab.core.schedule.sweep.after_fetch_with(
            &lab.core,
            100,
            Arc::new(lab.fake.clone())
        ));
        let done = lab.core.schedule.sweep.wait_done();
        lab.clock.wait_for_sleepers(1);
        lab.clock.advance(QUIET_MS);
        assert_eq!(block_on(done).unwrap(), Step::Continue);
        assert_eq!(lab.fake.history_calls(), vec!["CSE2001.theory".to_owned()]);
    }

    #[test]
    fn sweep_needs_quick_fetch_fast_link_auto_refresh_and_shown() {
        let lab = lab("gates");
        let quick_ms = i64::from(lab.core.settings().tunable(TunableKey::SweepQuick)) * 1000;

        // 1) Fetch slower than SweepQuick -> refused.
        assert!(!lab.core.schedule.sweep.after_fetch_with(
            &lab.core,
            quick_ms + 1,
            Arc::new(lab.fake.clone()),
        ));

        // 2) Slow link (UDP) -> refused; recover with 3 fast non-UDP fetches.
        lab.core.schedule.flight.fetched("f", 100, true);
        assert!(lab.core.schedule.flight.slow_link());
        assert!(!lab.core.schedule.sweep.after_fetch_with(
            &lab.core,
            100,
            Arc::new(lab.fake.clone()),
        ));
        for _ in 0..3 {
            lab.core.schedule.flight.fetched("f", 100, false);
        }
        assert!(!lab.core.schedule.flight.slow_link());

        // 3) Auto Refresh off -> refused.
        lab.core
            .settings
            .set(SettingChange::AutoRefresh { value: false });
        assert!(!lab.core.schedule.sweep.after_fetch_with(
            &lab.core,
            100,
            Arc::new(lab.fake.clone()),
        ));
        lab.core
            .settings
            .set(SettingChange::AutoRefresh { value: true });

        // 4) App not on screen -> refused.
        lab.core.events.set_shown_for_test(false);
        assert!(!lab.core.schedule.sweep.after_fetch_with(
            &lab.core,
            100,
            Arc::new(lab.fake.clone()),
        ));
        lab.core.events.set_shown_for_test(true);

        // 5) All 4 met -> starts, and cooldown blocks an immediate second run.
        assert!(lab.core.schedule.sweep.after_fetch_with(
            &lab.core,
            quick_ms,
            Arc::new(lab.fake.clone())
        ));
        let done = lab.core.schedule.sweep.wait_done();
        lab.clock.wait_for_sleepers(1);
        // App leaves during QUIET_MS -> stops without fetching.
        lab.core.events.set_shown_for_test(false);
        lab.clock.advance(QUIET_MS);
        assert_eq!(
            block_on(done).unwrap(),
            Step::Stop("app left or auto refresh off".into())
        );
        assert!(lab.fake.batch_calls().is_empty());

        // Still within cooldown -> refused even after returning to screen.
        lab.core.events.set_shown_for_test(true);
        assert!(!lab.core.schedule.sweep.after_fetch_with(
            &lab.core,
            100,
            Arc::new(lab.fake.clone()),
        ));

        // Past cooldown -> starts again.
        let cooldown_ms =
            u64::try_from(lab.core.settings().tunable(TunableKey::SweepCooldown)).unwrap() * 60_000;
        lab.clock.advance(cooldown_ms);
        assert!(lab.core.schedule.sweep.after_fetch_with(
            &lab.core,
            100,
            Arc::new(lab.fake.clone())
        ));
        let done = lab.core.schedule.sweep.wait_done();
        lab.clock.wait_for_sleepers(1);
        lab.clock.advance(QUIET_MS);
        assert_eq!(block_on(done).unwrap(), Step::Continue);
        assert_eq!(lab.fake.batch_calls().len(), 1);
    }

    #[test]
    fn batch_uses_only_batched_pages() {
        // Every real sweep candidate is in batched_pages().
        let real_batched: HashSet<String> = crate::remote::route::batch::batched_pages()
            .into_iter()
            .collect();
        for (_, name) in BATCH_CANDIDATES {
            assert!(
                real_batched.contains(name),
                "{name} missing from batched_pages()"
            );
        }

        // When only a subset is batched, the sweep sends only those in BATCH_CANDIDATES order.
        let lab = lab("batched");
        lab.fake.set_batched(vec!["marks", "attendance"]);
        let ran_unattended = Arc::new(AtomicBool::new(false));
        assert!(lab.core.schedule.sweep.after_fetch_with(
            &lab.core,
            100,
            Arc::new(lab.fake.clone())
        ));
        let done = lab.core.schedule.sweep.wait_done();
        lab.clock.wait_for_sleepers(1);
        // While sweep runs, its flow is marked unattended in consent.
        if !lab.core.shared.consent().unattended.is_empty() {
            ran_unattended.store(true, Ordering::SeqCst);
        }
        lab.clock.advance(QUIET_MS);
        assert_eq!(block_on(done).unwrap(), Step::Continue);
        assert!(ran_unattended.load(Ordering::SeqCst));
        assert!(lab.core.shared.consent().unattended.is_empty());
        assert_eq!(
            lab.fake.batch_calls(),
            vec![vec!["attendance".to_owned(), "marks".to_owned()]]
        );
    }
}

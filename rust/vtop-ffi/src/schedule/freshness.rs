//! Academic calendar and exam-day freshness refreshes: ports of Kotlin's `CalendarRefresh` and
//! `ExamDayRefresh`.
//!
//! - [`Freshness::calendar_if_old`]: fetches the academic calendar again when none is saved or the
//!   saved one is older than `CalendarMaxAge` (catches holidays announced mid-semester). With Auto
//!   Refresh off only a missing calendar is fetched, unless `anyway` (after an attendance refresh
//!   or during the open-app sweep).
//! - [`Freshness::exams_if_exam_today`]: fetches the exam schedule again on a day with one of the
//!   student's papers, so a moved venue, seat or time shows. Only with Auto Refresh on and a saved
//!   schedule.
//!
//! Each serialises its own fetches and tries at most once per semester in [`RETRY_MS`]; cancelling
//! a running fetch forgets its try so the next open tries again.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use futures::FutureExt;
use futures::future::BoxFuture;

use crate::problem::Problem;
use crate::settings::TunableKey;
use crate::{Core, CoreError};

/// One fetch at a time, at most one try per semester in this long (offline, VTOP down).
pub(crate) const RETRY_MS: i64 = 15 * 60_000;

/// Short 8-hex flow id, matching Kotlin's `flow()`.
pub(crate) fn flow_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u64(NEXT.fetch_add(1, Ordering::Relaxed));
    format!("{:08x}", hasher.finish() as u32)
}

/// Writes one structured log line into the app's log pipeline.
pub(crate) fn log(level: &str, src: &str, flow: &str, msg: &str, fields: Vec<(&str, String)>) {
    let fields: HashMap<String, String> = fields
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect();
    crate::obs::log_event(
        level.into(),
        src.into(),
        Some(flow.into()),
        msg.into(),
        fields,
    );
}

/// Reports a failure of automatic work (calendar refresh, exam-day refresh, open-app sweep) through
/// [`crate::AppEvents::on_background_problem`], only when [`Problem::toasts`] allows it.
pub(crate) fn report_background_problem(core: &Core, error: &CoreError, flow: &str) {
    let problem = Problem {
        user_initiated: false,
        ..Problem::of(error, flow, false)
    };
    if problem.toasts(core.settings().show_background_errors) {
        core.events.tell("background_problem", |listener| {
            listener.on_background_problem(problem);
        });
    }
}

/// IST epoch day (days since 1970-01-01 in India, UTC+05:30) for wall time `now_ms`.
pub(crate) fn ist_day_of(now_ms: i64) -> i32 {
    (now_ms.div_euclid(1000) + 19_800).div_euclid(86_400) as i32
}

/// One Rust-initiated flow (`<src>.calendar`, `<src>.exams`, `sync.stale`): logs `start`, `done`,
/// or `cancelled` (on drop) as Kotlin's `flow()` does, and registers unattended flows in
/// [`crate::remote::consent::State::unattended`] until it ends or is dropped.
pub(crate) struct FlowRun<'a> {
    core: &'a Core,
    name: String,
    id: String,
    started_ms: i64,
    unattended: bool,
    armed: bool,
}

impl<'a> FlowRun<'a> {
    pub(crate) fn start(core: &'a Core, name: impl Into<String>, unattended: bool) -> Self {
        Self::start_with_id(core, name, flow_id(), unattended)
    }

    pub(crate) fn start_with_id(
        core: &'a Core,
        name: impl Into<String>,
        id: String,
        unattended: bool,
    ) -> Self {
        let name = name.into();
        let started_ms = core.schedule.runtime.clock().now_ms();
        if unattended {
            core.shared.consent().unattended.insert(id.clone());
        }
        log(
            "info",
            "flow",
            &id,
            &format!("{name} start"),
            vec![("step", name.clone()), ("ev", "start".into())],
        );
        Self {
            core,
            name,
            id,
            started_ms,
            unattended,
            armed: true,
        }
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn done(mut self) {
        self.armed = false;
        if self.unattended {
            self.core.shared.consent().unattended.remove(&self.id);
        }
        let ms = (self.core.schedule.runtime.clock().now_ms() - self.started_ms).to_string();
        log(
            "info",
            "flow",
            &self.id,
            &format!("{} done", self.name),
            vec![
                ("step", self.name.clone()),
                ("ev", "done".into()),
                ("ms", ms),
                ("outcome", "ok".into()),
            ],
        );
    }
}

impl Drop for FlowRun<'_> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if self.unattended {
            self.core.shared.consent().unattended.remove(&self.id);
        }
        let ms = (self.core.schedule.runtime.clock().now_ms() - self.started_ms).to_string();
        log(
            "info",
            "flow",
            &self.id,
            &format!("{} cancelled", self.name),
            vec![
                ("step", self.name.clone()),
                ("ev", "cancelled".into()),
                ("ms", ms),
                ("outcome", "cancelled".into()),
            ],
        );
    }
}

/// Forgets a semester's retry timestamp if the fetch is cancelled (dropped) mid-flight, so the
/// next screen open tries again.
struct RetryGuard<'a> {
    tries: &'a Mutex<HashMap<String, i64>>,
    semester: String,
    armed: bool,
}

impl<'a> RetryGuard<'a> {
    fn new(tries: &'a Mutex<HashMap<String, i64>>, semester: &str) -> Self {
        Self {
            tries,
            semester: semester.to_owned(),
            armed: true,
        }
    }

    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for RetryGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.tries
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&self.semester);
        }
    }
}

/// Cache reads and VTOP fetches for [`Freshness`]; mocked in tests.
pub(crate) trait FreshnessSource: Send + Sync {
    /// Cached calendar's `updated_at` (unix seconds), or None when missing.
    fn cached_calendar_at(&self, flow: &str) -> Option<i64>;
    /// Fetches the academic calendar; returns the number of months on success.
    fn fetch_calendar(&self, flow: String) -> BoxFuture<'static, Result<usize, CoreError>>;
    /// Whether the cached exam schedule has any paper scheduled on `day` (IST epoch day).
    fn has_exam_on(&self, flow: &str, day: i32) -> bool;
    /// Fetches the exam schedule; returns the total number of papers on success.
    fn fetch_exams(&self, flow: String) -> BoxFuture<'static, Result<usize, CoreError>>;
}

struct CoreFreshness(Arc<Core>);

impl FreshnessSource for CoreFreshness {
    fn cached_calendar_at(&self, flow: &str) -> Option<i64> {
        self.0
            .cached_calendar(flow.to_owned())
            .map(|cal| cal.updated_at)
    }

    fn fetch_calendar(&self, flow: String) -> BoxFuture<'static, Result<usize, CoreError>> {
        let core = self.0.clone();
        async move { Ok(core.fetch_calendar(flow).await?.months.len()) }.boxed()
    }

    fn has_exam_on(&self, flow: &str, day: i32) -> bool {
        self.0.cached_exams(flow.to_owned()).is_some_and(|exams| {
            exams
                .groups
                .iter()
                .any(|group| group.papers.iter().any(|paper| paper.day == Some(day)))
        })
    }

    fn fetch_exams(&self, flow: String) -> BoxFuture<'static, Result<usize, CoreError>> {
        let core = self.0.clone();
        async move {
            let exams = core.fetch_exams(flow).await?;
            Ok(exams.groups.iter().map(|group| group.papers.len()).sum())
        }
        .boxed()
    }
}

/// Per-semester retry state and single-flight locks for calendar and exam-day refreshes.
#[derive(Default)]
pub(crate) struct Freshness {
    calendar_lock: futures::lock::Mutex<()>,
    calendar_tries: Mutex<HashMap<String, i64>>,
    exams_lock: futures::lock::Mutex<()>,
    exams_tries: Mutex<HashMap<String, i64>>,
}

impl Freshness {
    /// Fetches `semester`'s calendar if missing or older than `CalendarMaxAge`, in flow
    /// `<src>.calendar` (log src `ui.<src>`); returns true when a fresh one arrived. With Auto
    /// Refresh off only a missing calendar is fetched, unless `anyway`.
    pub(crate) async fn calendar_if_old(
        &self,
        core: &Arc<Core>,
        semester: &str,
        src: &str,
        anyway: bool,
    ) -> bool {
        self.calendar_if_old_with(core, &CoreFreshness(core.clone()), semester, src, anyway)
            .await
    }

    pub(crate) async fn calendar_if_old_with(
        &self,
        core: &Core,
        source: &dyn FreshnessSource,
        semester: &str,
        src: &str,
        anyway: bool,
    ) -> bool {
        let _one = self.calendar_lock.lock().await;
        let flow = FlowRun::start(core, format!("{src}.calendar"), src == "sync");
        let id = flow.id().to_owned();
        let now_ms = core.schedule.runtime.clock().now_ms();
        let age = source
            .cached_calendar_at(&id)
            .map(|updated_at| now_ms / 1000 - updated_at);
        let settings = core.settings();
        let max_age_s = i64::from(settings.tunable(TunableKey::CalendarMaxAge)) * 3600;
        if let Some(age) = age
            && (age < max_age_s || !(anyway || settings.auto_refresh))
        {
            flow.done();
            return false;
        }
        let tried = self
            .calendar_tries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(semester)
            .copied();
        if let Some(tried) = tried
            && now_ms - tried < RETRY_MS
        {
            flow.done();
            return false;
        }
        self.calendar_tries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(semester.to_owned(), now_ms);
        let retry = RetryGuard::new(&self.calendar_tries, semester);
        let age_s = age.map_or_else(|| "null".to_owned(), |value| value.to_string());
        let log_src = format!("ui.{src}");
        let result = core
            .schedule
            .activity
            .track(
                "Refreshing calendar",
                &id,
                source.fetch_calendar(id.clone()),
            )
            .await;
        retry.disarm();
        let fetched = match result {
            Ok(months) => {
                log(
                    "info",
                    &log_src,
                    &id,
                    "calendar fetched",
                    vec![("months", months.to_string()), ("age_s", age_s)],
                );
                true
            }
            Err(error) => {
                log(
                    "warn",
                    &log_src,
                    &id,
                    "calendar not fetched",
                    vec![("error", error.kind().to_owned()), ("age_s", age_s)],
                );
                report_background_problem(core, &error, &id);
                false
            }
        };
        flow.done();
        fetched
    }

    /// Fetches `semester`'s exam schedule when Auto Refresh is on and the saved schedule has a
    /// paper today (IST), in flow `<src>.exams` (log src `ui.<src>`); returns true when a fresh
    /// schedule arrived.
    pub(crate) async fn exams_if_exam_today(
        &self,
        core: &Arc<Core>,
        semester: &str,
        src: &str,
    ) -> bool {
        self.exams_if_exam_today_with(core, &CoreFreshness(core.clone()), semester, src)
            .await
    }

    pub(crate) async fn exams_if_exam_today_with(
        &self,
        core: &Core,
        source: &dyn FreshnessSource,
        semester: &str,
        src: &str,
    ) -> bool {
        let _one = self.exams_lock.lock().await;
        if !core.settings().auto_refresh {
            return false;
        }
        let flow = FlowRun::start(core, format!("{src}.exams"), false);
        let id = flow.id().to_owned();
        let now_ms = core.schedule.runtime.clock().now_ms();
        let today = ist_day_of(now_ms);
        if !source.has_exam_on(&id, today) {
            flow.done();
            return false;
        }
        let tried = self
            .exams_tries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(semester)
            .copied();
        if let Some(tried) = tried
            && now_ms - tried < RETRY_MS
        {
            flow.done();
            return false;
        }
        self.exams_tries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(semester.to_owned(), now_ms);
        let retry = RetryGuard::new(&self.exams_tries, semester);
        let log_src = format!("ui.{src}");
        let result = core
            .schedule
            .activity
            .track("Refreshing exams", &id, source.fetch_exams(id.clone()))
            .await;
        retry.disarm();
        let fetched = match result {
            Ok(papers) => {
                log(
                    "info",
                    &log_src,
                    &id,
                    "exams fetched (exam day)",
                    vec![("papers", papers.to_string())],
                );
                true
            }
            Err(error) => {
                log(
                    "warn",
                    &log_src,
                    &id,
                    "exams not fetched",
                    vec![("error", error.kind().to_owned())],
                );
                report_background_problem(core, &error, &id);
                false
            }
        };
        flow.done();
        fetched
    }

    /// Sign-out or account switch: forgets retry cooldowns so the next account starts fresh.
    pub(crate) fn clear(&self) {
        self.calendar_tries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        self.exams_tries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use futures::FutureExt;
    use futures::future::BoxFuture;
    use smol::future::block_on;

    use super::*;
    use crate::events::AppEvents;
    use crate::problem::Problem;
    use crate::remote::consent::ConsentAsk;
    use crate::remote::handover::HandoverStatus;
    use crate::runtime::{Clock, ManualClock, Runtime};
    use crate::schedule::ActivityEntry;
    use crate::settings::SettingChange;

    #[derive(Default)]
    struct FakeState {
        calendar_at: Mutex<Option<i64>>,
        calendar_result: Mutex<Option<Result<usize, CoreError>>>,
        calendar_gate: Mutex<Option<(smol::channel::Sender<()>, smol::channel::Receiver<()>)>>,
        calendar_fetches: AtomicUsize,
        exam_days: Mutex<Vec<i32>>,
        exams_result: Mutex<Option<Result<usize, CoreError>>>,
        exams_fetches: AtomicUsize,
    }

    #[derive(Clone, Default)]
    struct Fake(Arc<FakeState>);

    impl Fake {
        fn calendar_at(&self, at: Option<i64>) {
            *self.0.calendar_at.lock().unwrap() = at;
        }

        fn calendar_answer(&self, res: Result<usize, CoreError>) {
            *self.0.calendar_result.lock().unwrap() = Some(res);
        }

        /// Next calendar fetch signals `entered` and waits until `release` closes.
        fn gate_calendar(&self) -> (smol::channel::Receiver<()>, smol::channel::Sender<()>) {
            let (entered_tx, entered_rx) = smol::channel::bounded(1);
            let (release_tx, release_rx) = smol::channel::bounded(1);
            *self.0.calendar_gate.lock().unwrap() = Some((entered_tx, release_rx));
            (entered_rx, release_tx)
        }

        fn calendar_fetches(&self) -> usize {
            self.0.calendar_fetches.load(Ordering::SeqCst)
        }

        fn set_exam_days(&self, days: Vec<i32>) {
            *self.0.exam_days.lock().unwrap() = days;
        }

        fn exams_answer(&self, res: Result<usize, CoreError>) {
            *self.0.exams_result.lock().unwrap() = Some(res);
        }

        fn exams_fetches(&self) -> usize {
            self.0.exams_fetches.load(Ordering::SeqCst)
        }
    }

    impl FreshnessSource for Fake {
        fn cached_calendar_at(&self, _flow: &str) -> Option<i64> {
            *self.0.calendar_at.lock().unwrap()
        }

        fn fetch_calendar(&self, _flow: String) -> BoxFuture<'static, Result<usize, CoreError>> {
            let state = self.0.clone();
            state.calendar_fetches.fetch_add(1, Ordering::SeqCst);
            async move {
                let gate = state.calendar_gate.lock().unwrap().take();
                if let Some((entered, release)) = gate {
                    let _ = entered.send(()).await;
                    let _ = release.recv().await;
                }
                state
                    .calendar_result
                    .lock()
                    .unwrap()
                    .clone()
                    .unwrap_or(Ok(4))
            }
            .boxed()
        }

        fn has_exam_on(&self, _flow: &str, day: i32) -> bool {
            self.0.exam_days.lock().unwrap().contains(&day)
        }

        fn fetch_exams(&self, _flow: String) -> BoxFuture<'static, Result<usize, CoreError>> {
            let state = self.0.clone();
            state.exams_fetches.fetch_add(1, Ordering::SeqCst);
            async move { state.exams_result.lock().unwrap().clone().unwrap_or(Ok(6)) }.boxed()
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
        fake: Fake,
        problems: Arc<HeardProblems>,
    }

    fn lab(name: &str) -> Lab {
        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-fresh-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let clock = ManualClock::default();
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
        Lab {
            core,
            clock,
            fake: Fake::default(),
            problems,
        }
    }

    #[test]
    fn calendar_retry_once_per_15_min_per_semester() {
        let lab = lab("retry");
        // Missing calendar: first try fails, no toast while show_background_errors is off.
        lab.fake.calendar_answer(Err(CoreError::Offline));
        assert!(!block_on(lab.core.schedule.freshness.calendar_if_old_with(
            &lab.core,
            &lab.fake,
            "SEM1",
            "timetable",
            false,
        )));
        assert_eq!(lab.fake.calendar_fetches(), 1);
        assert!(lab.problems.0.lock().unwrap().is_empty());

        // Within 15 min for the same semester: skipped.
        lab.clock.advance((RETRY_MS - 1) as u64);
        assert!(!block_on(lab.core.schedule.freshness.calendar_if_old_with(
            &lab.core,
            &lab.fake,
            "SEM1",
            "timetable",
            false,
        )));
        assert_eq!(lab.fake.calendar_fetches(), 1);

        // Another semester has its own retry window.
        lab.fake.calendar_answer(Ok(5));
        assert!(block_on(lab.core.schedule.freshness.calendar_if_old_with(
            &lab.core,
            &lab.fake,
            "SEM2",
            "timetable",
            false,
        )));
        assert_eq!(lab.fake.calendar_fetches(), 2);

        // Once 15 min pass for SEM1, it retries and succeeds.
        lab.clock.advance(1);
        assert!(block_on(lab.core.schedule.freshness.calendar_if_old_with(
            &lab.core,
            &lab.fake,
            "SEM1",
            "timetable",
            false,
        )));
        assert_eq!(lab.fake.calendar_fetches(), 3);

        // Fresh calendar is never refetched, even with anyway = true.
        lab.clock.advance(RETRY_MS as u64);
        let now_s = lab.clock.now_ms() / 1000;
        lab.fake.calendar_at(Some(now_s - 3600));
        assert!(!block_on(lab.core.schedule.freshness.calendar_if_old_with(
            &lab.core,
            &lab.fake,
            "SEM1",
            "attendance",
            true,
        )));
        assert_eq!(lab.fake.calendar_fetches(), 3);

        // Old calendar with Auto Refresh off: skipped unless anyway = true.
        lab.core
            .settings
            .set(SettingChange::AutoRefresh { value: false });
        lab.clock.advance(25 * 3_600_000);
        assert!(!block_on(lab.core.schedule.freshness.calendar_if_old_with(
            &lab.core,
            &lab.fake,
            "SEM1",
            "timetable",
            false,
        )));
        assert_eq!(lab.fake.calendar_fetches(), 3);
        assert!(block_on(lab.core.schedule.freshness.calendar_if_old_with(
            &lab.core,
            &lab.fake,
            "SEM1",
            "attendance",
            true,
        )));
        assert_eq!(lab.fake.calendar_fetches(), 4);

        // With show_background_errors on, a failure pushes on_background_problem.
        lab.core
            .settings
            .set(SettingChange::ShowBackgroundErrors { value: true });
        lab.clock.advance(RETRY_MS as u64);
        lab.fake.calendar_answer(Err(CoreError::Offline));
        assert!(!block_on(lab.core.schedule.freshness.calendar_if_old_with(
            &lab.core,
            &lab.fake,
            "SEM1",
            "attendance",
            true,
        )));
        assert_eq!(lab.problems.0.lock().unwrap().len(), 1);
        assert!(!lab.problems.0.lock().unwrap()[0].user_initiated);
    }

    #[test]
    fn calendar_cancel_forgets_try() {
        let lab = lab("cancel");
        let (entered, _release) = lab.fake.gate_calendar();
        let core = lab.core.clone();
        let fake = lab.fake.clone();
        let task = lab.core.schedule.runtime.spawn("test.cal", async move {
            core.schedule
                .freshness
                .calendar_if_old_with(&core, &fake, "SEM1", "timetable", false)
                .await
        });
        block_on(entered.recv()).unwrap();
        // Cancel mid-fetch: drops RetryGuard and forgets SEM1's try timestamp.
        assert_eq!(block_on(task.cancel()), None);
        assert_eq!(lab.fake.calendar_fetches(), 1);

        // Immediately trying again (0 ms elapsed) runs and succeeds.
        lab.fake.calendar_answer(Ok(4));
        assert!(block_on(lab.core.schedule.freshness.calendar_if_old_with(
            &lab.core,
            &lab.fake,
            "SEM1",
            "timetable",
            false,
        )));
        assert_eq!(lab.fake.calendar_fetches(), 2);
    }

    #[test]
    fn exam_day_only_with_paper_today_and_auto_refresh() {
        let lab = lab("exams");
        // Set clock to an exact IST day.
        lab.clock.advance(20_000 * 86_400_000);
        let today = ist_day_of(lab.clock.now_ms());

        // No paper today -> no fetch.
        lab.fake.set_exam_days(vec![today + 1]);
        assert!(!block_on(
            lab.core.schedule.freshness.exams_if_exam_today_with(
                &lab.core,
                &lab.fake,
                "SEM1",
                "timetable",
            )
        ));
        assert_eq!(lab.fake.exams_fetches(), 0);

        // Paper today, but Auto Refresh off -> no fetch.
        lab.fake.set_exam_days(vec![today]);
        lab.core
            .settings
            .set(SettingChange::AutoRefresh { value: false });
        assert!(!block_on(
            lab.core.schedule.freshness.exams_if_exam_today_with(
                &lab.core,
                &lab.fake,
                "SEM1",
                "timetable",
            )
        ));
        assert_eq!(lab.fake.exams_fetches(), 0);

        // Auto Refresh on and paper today -> fetches once, then respects RETRY_MS.
        lab.core
            .settings
            .set(SettingChange::AutoRefresh { value: true });
        lab.fake.exams_answer(Ok(3));
        assert!(block_on(
            lab.core.schedule.freshness.exams_if_exam_today_with(
                &lab.core,
                &lab.fake,
                "SEM1",
                "timetable",
            )
        ));
        assert_eq!(lab.fake.exams_fetches(), 1);

        lab.clock.advance((RETRY_MS - 1) as u64);
        assert!(!block_on(
            lab.core.schedule.freshness.exams_if_exam_today_with(
                &lab.core,
                &lab.fake,
                "SEM1",
                "timetable",
            )
        ));
        assert_eq!(lab.fake.exams_fetches(), 1);

        lab.clock.advance(1);
        assert!(block_on(
            lab.core.schedule.freshness.exams_if_exam_today_with(
                &lab.core,
                &lab.fake,
                "SEM1",
                "timetable",
            )
        ));
        assert_eq!(lab.fake.exams_fetches(), 2);
    }
}

// The timetable asks for these itself: only with calculated insights on, and it reads its days
// again when one returns true. Each opens its own flow (`timetable.calendar`, `timetable.exams`).
#[uniffi::export]
impl Core {
    /// The semester's calendar fetched again when old; true when a fresh one arrived.
    pub async fn timetable_calendar_if_old(self: Arc<Self>, semester: String) -> bool {
        let freshness = &self.schedule.freshness;
        freshness
            .calendar_if_old(&self, &semester, "timetable", false)
            .await
    }

    /// The exam schedule fetched again on a day with a paper; true when a fresh one arrived.
    pub async fn timetable_exams_if_exam_today(self: Arc<Self>, semester: String) -> bool {
        let freshness = &self.schedule.freshness;
        freshness
            .exams_if_exam_today(&self, &semester, "timetable")
            .await
    }
}

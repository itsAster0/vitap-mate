//! A VTOP page on a screen: the cache first, then a fetch when nothing is saved, the page was
//! not fetched yet this process (with Auto Refresh on) or its data is older than `PageMaxAge`,
//! on open or when the app comes back. Fetches run through the queue ([`crate::schedule`]), so
//! they outlive the screen: one shown again while its fetch runs or waits joins it. A port of
//! Kotlin's `rememberPage` and `PageHandle`; the screen reads the data itself when `updated_at`
//! changes. Each change is logged as `ui.<id>`, in the same fields as before.

mod state;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use futures::FutureExt;
use futures::channel::oneshot;
use futures::future::{BoxFuture, Shared};
use tracing::warn;

pub use state::{PageActivity, PageStatus};
use state::{PageData, PageEvent, Seen, Wake, next, refresh_trigger, waiting_for};

use crate::problem::Problem;
use crate::remote::route::Route;
use crate::schedule::{FlightError, RunOpts};
use crate::settings::TunableKey;
use crate::{Core, CoreError};

/// Every page a screen watches. [`page_id_name`] gives its id in logs, flows and queue keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum PageId {
    Attendance,
    /// `item`: the course key.
    AttendanceDetail,
    Timetable,
    Calendar,
    Marks,
    Exams,
    /// `item`: the semester id.
    Grades,
    GradeHistory,
    Profile,
    GeneralOuting,
    WeekendOuting,
    /// `item`: the day (IST epoch day).
    Biometric,
    CoursePage,
    /// `item`: `<course id>/<class id or empty>/<1 to match the timetable, else 0>`.
    CoursePageCourse,
}

impl PageId {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Attendance => "attendance",
            Self::AttendanceDetail => "attendance_detail",
            Self::Timetable => "timetable",
            Self::Calendar => "calendar",
            Self::Marks => "marks",
            Self::Exams => "exams",
            Self::Grades => "grades",
            Self::GradeHistory => "grade_history",
            Self::Profile => "profile",
            Self::GeneralOuting | Self::WeekendOuting => "outing",
            Self::Biometric => "biometric",
            Self::CoursePage => "coursepage",
            Self::CoursePageCourse => "coursepage_course",
        }
    }
}

/// `page`'s id: `attendance`, `attendance_detail`, ...
#[uniffi::export]
pub fn page_id_name(page: PageId) -> String {
    page.name().into()
}

/// Hears a watched page's status; Kotlin implements it. Called on the core's threads, in
/// order, and never after [`PageWatch::stop`].
#[uniffi::export(with_foreign)]
pub trait PageListener: Send + Sync {
    fn on_status(&self, status: PageStatus);
}

/// Where a watch reads and fetches its page.
pub(crate) trait PageSource: Send + Sync {
    /// The saved copy; None when there is none or it doesn't read.
    fn cached(&self, flow: &str) -> Option<PageData>;
    fn fetch(&self, flow: String) -> BoxFuture<'static, Result<PageData, CoreError>>;
}

fn rows(count: usize) -> Option<i32> {
    i32::try_from(count).ok()
}

fn data(updated_at: i64, count: usize) -> PageData {
    PageData {
        updated_at,
        count: rows(count),
    }
}

/// The page's own `cached_x` and `fetch_x` calls.
struct CorePage {
    core: Arc<Core>,
    page: PageId,
    item: String,
}

/// A course view's item: the course, the picked class and whether to match the timetable.
fn course_view(item: &str) -> (String, Option<String>, bool) {
    let mut parts = item.rsplitn(3, '/');
    let insights = parts.next() == Some("1");
    let picked = parts.next().filter(|id| !id.is_empty()).map(str::to_owned);
    (
        parts.next().unwrap_or_default().to_owned(),
        picked,
        insights,
    )
}

fn view_rows(view: &crate::pages::course_page::CourseView) -> usize {
    view.plan
        .as_ref()
        .map_or(view.sections.len(), |plan| plan.lectures.len())
}

fn exam_rows(exams: &crate::pages::exams::Exams) -> usize {
    exams.groups.iter().map(|group| group.papers.len()).sum()
}

impl PageSource for CorePage {
    fn cached(&self, flow: &str) -> Option<PageData> {
        let (core, flow, item) = (&self.core, flow.to_owned(), self.item.clone());
        match self.page {
            PageId::Attendance => core
                .cached_attendance(flow)
                .map(|v| data(v.updated_at, v.courses.len())),
            PageId::AttendanceDetail => core
                .cached_attendance_detail(flow, item)
                .map(|v| data(v.updated_at, v.classes.len())),
            PageId::Timetable => core
                .cached_timetable(flow)
                .map(|v| data(v.updated_at, v.slots.len())),
            PageId::Calendar => core
                .cached_calendar(flow)
                .map(|v| data(v.updated_at, v.months.len())),
            PageId::Marks => core
                .cached_marks(flow)
                .map(|v| data(v.updated_at, v.courses.len())),
            PageId::Exams => core
                .cached_exams(flow)
                .map(|v| data(v.updated_at, exam_rows(&v))),
            PageId::Grades => core
                .cached_grades(flow, item)
                .map(|v| data(v.updated_at, v.courses.len())),
            PageId::GradeHistory => core.cached_grade_history(flow).map(|v| PageData {
                updated_at: v.updated_at,
                count: Some(v.course_count),
            }),
            PageId::Profile => core
                .cached_profile(flow)
                .map(|v| data(v.updated_at, v.details.len())),
            PageId::GeneralOuting => core
                .cached_general_outing(flow)
                .map(|v| data(v.updated_at, v.requests.len())),
            PageId::WeekendOuting => core
                .cached_weekend_outing(flow)
                .map(|v| data(v.updated_at, v.requests.len())),
            PageId::Biometric => item
                .parse()
                .ok()
                .and_then(|day| core.cached_biometric(flow, day))
                .map(|v| data(v.updated_at, v.punches.len())),
            PageId::CoursePage => core
                .cached_course_page(flow)
                .map(|v| data(v.updated_at, v.groups.len())),
            PageId::CoursePageCourse => {
                let (course, picked, insights) = course_view(&item);
                core.cached_course_view(flow, course, picked, insights)
                    .map(|v| data(v.updated_at, view_rows(&v)))
            }
        }
    }

    fn fetch(&self, flow: String) -> BoxFuture<'static, Result<PageData, CoreError>> {
        let (core, page, item) = (self.core.clone(), self.page, self.item.clone());
        async move {
            Ok(match page {
                PageId::Attendance => {
                    let v = core.fetch_attendance(flow).await?;
                    data(v.updated_at, v.courses.len())
                }
                PageId::AttendanceDetail => {
                    let v = core.fetch_attendance_detail(flow, item).await?;
                    data(v.updated_at, v.classes.len())
                }
                PageId::Timetable => {
                    let v = core.fetch_timetable(flow).await?;
                    data(v.updated_at, v.slots.len())
                }
                PageId::Calendar => {
                    let v = core.fetch_calendar(flow).await?;
                    data(v.updated_at, v.months.len())
                }
                PageId::Marks => {
                    let v = core.fetch_marks(flow).await?;
                    data(v.updated_at, v.courses.len())
                }
                PageId::Exams => {
                    let v = core.fetch_exams(flow).await?;
                    data(v.updated_at, exam_rows(&v))
                }
                PageId::Grades => {
                    let v = core.fetch_grades(flow, item).await?;
                    data(v.updated_at, v.courses.len())
                }
                PageId::GradeHistory => {
                    let v = core.fetch_grade_history(flow).await?;
                    PageData {
                        updated_at: v.updated_at,
                        count: Some(v.course_count),
                    }
                }
                PageId::Profile => {
                    let v = core.fetch_profile(flow).await?;
                    data(v.updated_at, v.details.len())
                }
                PageId::GeneralOuting => {
                    let v = core.fetch_general_outing(flow).await?;
                    data(v.updated_at, v.requests.len())
                }
                PageId::WeekendOuting => {
                    let v = core.fetch_weekend_outing(flow).await?;
                    data(v.updated_at, v.requests.len())
                }
                PageId::Biometric => {
                    let day = item.parse().map_err(|_| CoreError::Other {
                        reason: format!("not a day: {item}"),
                    })?;
                    let v = core.fetch_biometric(flow, day).await?;
                    data(v.updated_at, v.punches.len())
                }
                PageId::CoursePage => {
                    let v = core.fetch_course_page(flow).await?;
                    data(v.updated_at, v.groups.len())
                }
                PageId::CoursePageCourse => {
                    let (course, picked, insights) = course_view(&item);
                    let v = core
                        .fetch_course_view(flow, course, picked, insights)
                        .await?;
                    data(v.updated_at, view_rows(&v))
                }
            })
        }
        .boxed()
    }
}

/// A short flow id, as Kotlin's `flow()` makes them.
fn flow_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u64(NEXT.fetch_add(1, Ordering::Relaxed));
    format!("{:08x}", hasher.finish() as u32)
}

fn log(level: &str, src: &str, flow: &str, msg: &str, fields: Vec<(&str, String)>) {
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

/// The `ui.<id>` line's fields for `status` after `what`, as Kotlin's `PageHandle.log` wrote
/// them: `state`, `kind`, `ms` since open, `count` and `age_s` when ready, `problem` when one
/// shows, then `extra`.
fn line_fields(
    what: &str,
    status: &PageStatus,
    ms: i64,
    now_s: i64,
    extra: &[(&'static str, String)],
) -> Vec<(&'static str, String)> {
    let mut fields = vec![("state", what.to_owned())];
    let (kind, problem) = match status {
        PageStatus::Loading { .. } => ("Loading", None),
        PageStatus::Ready { activity, .. } => (
            "Ready",
            match activity {
                PageActivity::Stale { problem } => Some(problem),
                _ => None,
            },
        ),
        PageStatus::Failed { problem } => ("Failed", Some(problem)),
    };
    fields.push(("kind", kind.into()));
    fields.push(("ms", ms.to_string()));
    if let PageStatus::Ready {
        updated_at, count, ..
    } = status
    {
        if let Some(count) = count {
            fields.push(("count", count.to_string()));
        }
        fields.push(("age_s", (now_s - updated_at).to_string()));
    }
    if let Some(problem) = problem {
        fields.push(("problem", format!("{:?}", problem.kind)));
    }
    fields.extend(extra.iter().cloned());
    fields
}

type Outcome = Shared<BoxFuture<'static, Result<(), CoreError>>>;

/// This watch's fetch while it runs: its outcome, and whether the student asked for it.
struct Running {
    id: u64,
    outcome: Outcome,
    user: Arc<AtomicBool>,
}

struct WatchState {
    status: PageStatus,
    running: Option<Running>,
    next_run: u64,
    tasks: Vec<smol::Task<()>>,
    shown: bool,
    closed: bool,
}

/// One screen's page ([`Core::page_watch`]).
#[derive(uniffi::Object)]
pub struct PageWatch {
    core: Arc<Core>,
    page: PageId,
    /// The page, its item and the semester: fetches with the same key are one.
    key: String,
    source: Arc<dyn PageSource>,
    listener: Arc<dyn PageListener>,
    opened_ms: i64,
    /// Held while the listener is told, so statuses arrive in order and none after close.
    state: Mutex<WatchState>,
    /// Ends when the open sequence did (the cache, the just-fetched check, the open fetch).
    #[cfg_attr(not(test), expect(dead_code, reason = "tests wait on it"))]
    opened: Shared<BoxFuture<'static, ()>>,
}

#[uniffi::export]
impl Core {
    /// Watches `page` (with `item`, see [`PageId`]) for a screen: reads the cache now in
    /// `flow` (`<id>.open`), then fetches as the open rules say and follows the app coming
    /// back. Stop it when the screen goes.
    pub fn page_watch(
        self: Arc<Self>,
        flow: String,
        page: PageId,
        item: String,
        listener: Arc<dyn PageListener>,
    ) -> Arc<PageWatch> {
        let source = Arc::new(CorePage {
            core: self.clone(),
            page,
            item: item.clone(),
        });
        PageWatch::open(self, &flow, page, &item, source, listener)
    }
}

impl Core {
    /// A page fetch of `ms` succeeded over a link worth judging (no OTP wait in it). Starts the
    /// open-app stale sweep when its preconditions hold.
    pub(crate) fn after_page_fetch(self: &Arc<Self>, _flow: &str, ms: i64) {
        self.schedule.sweep.after_fetch(self, ms);
    }

    /// The picked semester, as page keys name it (`null` with none, as Kotlin's keys did).
    fn key_semester(&self) -> String {
        self.selected_semester().unwrap_or_else(|| "null".into())
    }
}

#[uniffi::export]
impl PageWatch {
    /// Fetches fresh data in `flow` (`<id>.refresh`); `trigger` says why (`pull`, `button`,
    /// `retry`, ...); `user` false for automatic ones (their failure doesn't toast by default).
    /// A refresh while one runs joins it. Err only for a fetch that failed (the status shows
    /// it); a dropped or cancelled one is Ok.
    pub async fn refresh(
        self: Arc<Self>,
        flow: String,
        trigger: String,
        user: bool,
    ) -> Result<(), CoreError> {
        self.fetch_in(&flow, &trigger, user).await
    }

    /// The screen shows the page (`on`) or not: its waiting fetch moves up or down.
    pub fn shown(&self, on: bool) {
        {
            let mut state = self.lock();
            if state.closed || state.shown == on {
                return;
            }
            state.shown = on;
        }
        self.core.schedule.flight.shown(self.page.name(), on);
    }

    /// The screen went: nothing more is told, and its fetch runs on in the queue alone. (Not
    /// `close`: UniFFI's Kotlin objects have one that frees the handle.)
    pub fn stop(&self) {
        let (tasks, shown) = {
            let mut state = self.lock();
            if state.closed {
                return;
            }
            state.closed = true;
            (std::mem::take(&mut state.tasks), state.shown)
        };
        if shown {
            self.core.schedule.flight.shown(self.page.name(), false);
        }
        drop(tasks);
    }

    /// The queue key: `<id>/<item>/<semester>`.
    pub fn key(&self) -> String {
        self.key.clone()
    }
}

impl PageWatch {
    pub(crate) fn open(
        core: Arc<Core>,
        flow: &str,
        page: PageId,
        item: &str,
        source: Arc<dyn PageSource>,
        listener: Arc<dyn PageListener>,
    ) -> Arc<Self> {
        let key = format!("{}/{item}/{}", page.name(), core.key_semester());
        let (done, opened) = oneshot::channel::<()>();
        let watch = Arc::new(Self {
            opened_ms: core.schedule.runtime.clock().now_ms(),
            core,
            page,
            key,
            source,
            listener,
            state: Mutex::new(WatchState {
                status: PageStatus::Loading { waiting: None },
                running: None,
                next_run: 0,
                tasks: Vec::new(),
                shown: false,
                closed: false,
            }),
            opened: opened.map(|_| ()).boxed().shared(),
        });
        watch.read_cache(flow);
        let weak = Arc::downgrade(&watch);
        let resumed = watch.core.events.resumed();
        let task = watch.core.schedule.runtime.spawn(flow, async move {
            if let Some(watch) = weak.upgrade() {
                watch.on_open().await;
            }
            #[expect(
                clippy::let_underscore_must_use,
                reason = "only tests wait for the open sequence"
            )]
            let _ = done.send(());
            while resumed.recv().await.is_ok() {
                let Some(watch) = weak.upgrade() else { break };
                watch.on_resume().await;
            }
        });
        watch.lock().tasks.push(task);
        watch
    }

    fn lock(&self) -> MutexGuard<'_, WatchState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn now_ms(&self) -> i64 {
        self.core.schedule.runtime.clock().now_ms()
    }

    fn is_ready(&self) -> bool {
        matches!(self.lock().status, PageStatus::Ready { .. })
    }

    /// Ready data older than `PageMaxAge` is fetched again on open and on return.
    fn is_stale(&self) -> bool {
        let max_s = i64::from(self.core.settings().tunable(TunableKey::PageMaxAge)) * 3600;
        let now_s = self.now_ms() / 1000;
        matches!(self.lock().status, PageStatus::Ready { updated_at, .. } if now_s - updated_at > max_s)
    }

    /// Shows the cached copy, before any fetch.
    fn read_cache(&self, flow: &str) {
        match self.source.cached(flow) {
            Some(data) => self.on(
                PageEvent::CacheLoaded {
                    data,
                    flow: flow.into(),
                },
                "cached",
                flow,
                &[],
            ),
            None => self.log("no_cache", flow, &[]),
        }
    }

    /// The open fetch: a fetch that finished while the screen was away counts as this
    /// screen's, so no second request; then today's open rules.
    async fn on_open(self: &Arc<Self>) {
        let schedule = &self.core.schedule;
        let settings = self.core.settings();
        let ready = self.is_ready();
        let just_ms = i64::from(settings.tunable(TunableKey::JustFetched)) * 60_000;
        if !schedule.fetched(&self.key)
            && ready
            && schedule.flight.succeeded_within(&self.key, just_ms)
        {
            schedule.mark_fetched(&self.key);
        }
        let seen = Seen {
            ready,
            running: ready && schedule.flight.is_running(&self.key),
            auto_refresh: settings.auto_refresh,
            fetched: schedule.fetched(&self.key),
            stale: self.is_stale(),
        };
        if let Some(trigger) = refresh_trigger(Wake::Open, seen) {
            self.refresh_flow(trigger).await;
            let fresh = matches!(
                self.lock().status,
                PageStatus::Ready { ref activity, .. } if !matches!(activity, PageActivity::Stale { .. })
            );
            if fresh {
                schedule.mark_fetched(&self.key);
            }
        }
    }

    /// Back in the app after a while: old data is fetched again.
    async fn on_resume(self: &Arc<Self>) {
        let seen = Seen {
            auto_refresh: self.core.settings().auto_refresh,
            stale: self.is_stale(),
            ..Seen::default()
        };
        if let Some(trigger) = refresh_trigger(Wake::Resume, seen) {
            self.refresh_flow(trigger).await;
        }
    }

    /// An automatic refresh in a new `<id>.refresh` flow, logged as Kotlin's `flow()` does.
    async fn refresh_flow(self: &Arc<Self>, trigger: &str) {
        let name = format!("{}.refresh", self.page.name());
        let flow = flow_id();
        let started = self.now_ms();
        let step = |ev: &str| vec![("step", name.clone()), ("ev", ev.to_owned())];
        log(
            "info",
            "flow",
            &flow,
            &format!("{name} start"),
            step("start"),
        );
        // A watch stopped meanwhile drops this future: say so, as Kotlin's `flow()` does.
        let cancelled = Cancelled {
            watch: self,
            flow: &flow,
            name: &name,
            started,
            armed: true,
        };
        let result = self.fetch_in(&flow, trigger, false).await;
        cancelled.disarm();
        let ms = (self.now_ms() - started).to_string();
        match result {
            Ok(()) => {
                let mut fields = step("done");
                fields.extend([("ms", ms), ("outcome", "ok".into())]);
                log("info", "flow", &flow, &format!("{name} done"), fields);
            }
            Err(error) => {
                let mut fields = step("failed");
                fields.extend([("ms", ms), ("outcome", error.kind().into())]);
                let msg = format!("{name} failed: {}", error.kind());
                log("warn", "flow", &flow, &msg, fields);
            }
        }
    }

    /// One fetch at a time: joins the running one, else starts one; its failure, if any.
    async fn fetch_in(
        self: &Arc<Self>,
        flow: &str,
        trigger: &str,
        user: bool,
    ) -> Result<(), CoreError> {
        let (outcome, joined) = {
            let mut state = self.lock();
            if state.closed {
                return Ok(());
            }
            match &state.running {
                Some(running) => {
                    running.user.fetch_or(user, Ordering::SeqCst);
                    (running.outcome.clone(), true)
                }
                None => {
                    state.next_run += 1;
                    let id = state.next_run;
                    let asked = Arc::new(AtomicBool::new(user));
                    let (sent, outcome) = oneshot::channel();
                    let outcome: Outcome =
                        outcome.map(|got| got.unwrap_or(Ok(()))).boxed().shared();
                    let watch = self.clone();
                    let (flow, trigger, user_flag) =
                        (flow.to_owned(), trigger.to_owned(), asked.clone());
                    let task = self.core.schedule.runtime.spawn(&flow.clone(), async move {
                        let result = watch.fetch_now(&flow, &trigger, &user_flag).await;
                        {
                            let mut state = watch.lock();
                            if state
                                .running
                                .as_ref()
                                .is_some_and(|running| running.id == id)
                            {
                                state.running = None;
                            }
                        }
                        #[expect(
                            clippy::let_underscore_must_use,
                            reason = "every caller may have gone"
                        )]
                        let _ = sent.send(result);
                    });
                    state.tasks.retain(|task| !task.is_finished());
                    state.tasks.push(task);
                    state.running = Some(Running {
                        id,
                        outcome: outcome.clone(),
                        user: asked,
                    });
                    (outcome, false)
                }
            }
        };
        if joined {
            if user {
                self.core.schedule.flight.ask(&self.key);
            }
            self.log("join", flow, &[("trigger", trigger.to_owned())]);
        }
        outcome.await
    }

    async fn fetch_now(
        self: &Arc<Self>,
        flow: &str,
        trigger: &str,
        user: &AtomicBool,
    ) -> Result<(), CoreError> {
        let has_cache = self.is_ready();
        let what = if has_cache { "refreshing" } else { "fetching" };
        self.on(
            PageEvent::FetchStarted,
            what,
            flow,
            &[("trigger", trigger.to_owned())],
        );
        let opts = RunOpts {
            user: user.load(Ordering::SeqCst),
            has_cache,
            background: false,
        };
        let flight = &self.core.schedule.flight;
        let run = flight.start(&self.key, flow, opts, self.download(flow.to_owned()));
        if run.joined {
            let extra = [
                ("trigger", trigger.to_owned()),
                ("running", run.flow.clone()),
            ];
            self.log("join", flow, &extra);
        }
        let (_, outcome) =
            futures::join!(self.note_waiting(run.started.clone()), run.result.clone());
        match outcome {
            Ok(data) => {
                let event = PageEvent::FetchSucceeded {
                    data,
                    flow: run.flow.clone(),
                };
                self.on(event, "fresh", flow, &[]);
                Ok(())
            }
            Err(FlightError::Cancelled) => {
                self.on(
                    PageEvent::FetchCancelled { flow: run.flow },
                    "cancelled",
                    flow,
                    &[],
                );
                Ok(())
            }
            // Waited off screen past QueueDrop: the cached copy stays, no problem.
            Err(FlightError::Dropped) => {
                self.on(
                    PageEvent::FetchDropped { flow: run.flow },
                    "dropped",
                    flow,
                    &[],
                );
                Ok(())
            }
            Err(FlightError::Failed(error)) => {
                let asked = user.load(Ordering::SeqCst);
                let problem = Problem {
                    user_initiated: asked,
                    ..Problem::of(&error, &run.flow, false)
                };
                let what = if self.is_ready() {
                    "stale_kept"
                } else {
                    "failed"
                };
                let extra = [
                    ("error", error.kind().to_owned()),
                    ("user", asked.to_string()),
                ];
                self.on(PageEvent::FetchFailed(problem), what, flow, &extra);
                Err(error)
            }
        }
    }

    /// One fetch of the page, with what follows from it once (however many watches wait for
    /// it). A re-login that wants the OTP waits for it in Rust (the prompt opens), then goes on.
    fn download(
        &self,
        flow: String,
    ) -> impl Future<Output = Result<PageData, CoreError>> + Send + 'static {
        let core = self.core.clone();
        let page = self.page;
        let source = self.source.clone();
        let label = format!("Refreshing {}", self.page.name().replace('_', " "));
        async move {
            let clock = core.schedule.runtime.clock().clone();
            let start = clock.now_ms();
            let prompts = core.shared.otp.mark();
            let data = core
                .schedule
                .activity
                .track(&label, &flow, source.fetch(flow.clone()))
                .await?;
            let ms = clock.now_ms() - start;
            // A fetch that waited for the student's code says nothing about the link.
            if !core.shared.otp.opened_since(prompts) {
                let udp = core
                    .route_status(flow.clone())
                    .last_use
                    .is_some_and(|used| used.route == Route::Udp);
                core.schedule.flight.fetched(&flow, ms, udp);
                core.after_page_fetch(&flow, ms);
            }
            if page == PageId::Attendance
                && let Some(semester) = core.selected_semester()
            {
                let core_cal = core.clone();
                core.schedule
                    .runtime
                    .spawn("attendance.calendar", async move {
                        core_cal
                            .schedule
                            .freshness
                            .calendar_if_old(&core_cal, &semester, "attendance", true)
                            .await;
                    })
                    .detach();
            }
            Ok(data)
        }
    }

    /// Still waiting for a lane after a moment: says which pages hold them until `started`.
    async fn note_waiting(&self, started: Shared<BoxFuture<'static, bool>>) {
        let noted = AtomicBool::new(false);
        let flight = &self.core.schedule.flight;
        let note = async {
            self.core.schedule.runtime.clock().sleep(300).await;
            noted.store(true, Ordering::SeqCst);
            let changes = flight.watch_running();
            loop {
                self.waiting(waiting_for(&flight.running_pages()));
                if changes.recv().await.is_err() {
                    futures::future::pending::<()>().await;
                }
            }
        };
        smol::future::or(
            async {
                started.await;
            },
            note,
        )
        .await;
        if noted.load(Ordering::SeqCst) {
            self.waiting(None);
        }
    }

    fn waiting(&self, text: Option<String>) {
        let mut state = self.lock();
        if state.closed {
            return;
        }
        let status = next(state.status.clone(), PageEvent::WaitingChanged(text));
        if status != state.status {
            state.status = status.clone();
            self.tell(status);
        }
    }

    /// `event` applied, logged as `what` and told; nothing after close.
    fn on(&self, event: PageEvent, what: &str, flow: &str, extra: &[(&'static str, String)]) {
        let mut state = self.lock();
        if state.closed {
            return;
        }
        state.status = next(state.status.clone(), event);
        let status = state.status.clone();
        self.write_line(what, flow, &status, extra);
        self.tell(status);
    }

    fn log(&self, what: &str, flow: &str, extra: &[(&'static str, String)]) {
        let status = self.lock().status.clone();
        self.write_line(what, flow, &status, extra);
    }

    fn write_line(
        &self,
        what: &str,
        flow: &str,
        status: &PageStatus,
        extra: &[(&'static str, String)],
    ) {
        let now = self.now_ms();
        let fields = line_fields(what, status, now - self.opened_ms, now / 1000, extra);
        log(
            "info",
            &format!("ui.{}", self.page.name()),
            flow,
            what,
            fields,
        );
    }

    /// Tells the listener, under the state lock; a listener that panics is logged.
    fn tell(&self, status: PageStatus) {
        if catch_unwind(AssertUnwindSafe(|| self.listener.on_status(status))).is_err() {
            warn!(page = self.page.name(), "page listener failed");
        }
    }
}

/// Logs `<name> cancelled` for an automatic refresh flow dropped before it ended.
struct Cancelled<'a> {
    watch: &'a PageWatch,
    flow: &'a str,
    name: &'a str,
    started: i64,
    armed: bool,
}

impl Cancelled<'_> {
    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for Cancelled<'_> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let fields = vec![
            ("step", self.name.to_owned()),
            ("ev", "cancelled".to_owned()),
            ("ms", (self.watch.now_ms() - self.started).to_string()),
            ("outcome", "cancelled".to_owned()),
        ];
        let msg = format!("{} cancelled", self.name);
        log("info", "flow", self.flow, &msg, fields);
    }
}

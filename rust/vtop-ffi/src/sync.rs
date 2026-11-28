//! Fetches VTOP's pages for background sync and More → Fetch all ([`Core::sync`]), and class
//! histories in the background ([`Core::histories_refresh`]). Log src `sync`.
//!
//! Semesters and attendance go first, then the rest through the fetch queue's lanes (several at
//! once on a fast link; Fetch all as the student's, background sync at the lowest priority). A
//! page that fails is logged and skipped; the others still run. When VTOP asks for the OTP,
//! [`SyncOtp`] must get it accepted, asked once however many pages need it, or the run stops.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use futures::FutureExt;
use futures::future::BoxFuture;

use crate::pages::outing::OutingStanding;
use crate::runtime::TimedOut;
use crate::schedule::{FlightError, RunOpts};
use crate::settings::SettingChange;
use crate::sync_health::{PauseReason, SyncHealth};
use crate::{Core, CoreError};

/// Which run: background sync leaves out the profile, the course page and (unless a trip is
/// pending) outings, and checks one course's files a run instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SyncKind {
    Background,
    FetchAll,
}

/// Gets the OTP VTOP asked for accepted (background: auto-fill alone; Fetch all: the prompt).
#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait SyncOtp: Send + Sync {
    /// VTOP mailed the code at `issued_at` (unix seconds; None when unknown); true once VTOP
    /// accepted one, false to stop the run.
    async fn get(&self, issued_at: Option<u64>, flow: String) -> bool;
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SyncOutcome {
    Done,
    StoppedAtOtp,
    Rejected,
    Failed,
    /// Sign-out cancelled it.
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct SyncResult {
    pub outcome: SyncOutcome,
    pub fetched: u32,
    pub failed: u32,
}

/// One page a run fetches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SyncPage {
    Semesters,
    Attendance,
    History(String),
    Timetable,
    Marks,
    Exams,
    Calendar,
    Grades(String),
    GradeHistory,
    Biometric(i32),
    Profile,
    CoursePage,
    CourseFiles(String),
    GeneralOuting,
    WeekendOuting,
}

impl SyncPage {
    /// The name in labels and logs (`biometric`).
    fn name(&self) -> &'static str {
        match self {
            Self::Semesters => "semesters",
            Self::Attendance => "attendance",
            Self::History(_) => "history",
            Self::Timetable => "timetable",
            Self::Marks => "marks",
            Self::Exams => "exams",
            Self::Calendar => "calendar",
            Self::Grades(_) => "grades",
            Self::GradeHistory => "grade_history",
            Self::Biometric(_) => "biometric",
            Self::Profile => "profile",
            Self::CoursePage => "course_page",
            Self::CourseFiles(_) => "course_files",
            Self::GeneralOuting => "general_outing",
            Self::WeekendOuting => "weekend_outing",
        }
    }

    /// What tells apart pages of one name (a course's key, a day).
    fn item(&self) -> String {
        match self {
            Self::History(key) => key.clone(),
            Self::Biometric(day) => day.to_string(),
            _ => String::new(),
        }
    }

    /// The id in `page` fields (`history.<key>`).
    fn id(&self) -> String {
        match self {
            Self::History(_) | Self::Biometric(_) => format!("{}.{}", self.name(), self.item()),
            _ => self.name().into(),
        }
    }

    /// The queue key: `<page>/sync/<item>`, a history as `attendance_detail`.
    fn key(&self) -> String {
        let name = match self {
            Self::History(_) => "attendance_detail",
            other => other.name(),
        };
        format!("{name}/sync/{}", self.item())
    }
}

/// What a run reads and fetches; the core's pages, or a test's.
pub(crate) trait SyncSource: Send + Sync {
    fn fetch(&self, flow: String, page: SyncPage) -> BoxFuture<'static, Result<(), CoreError>>;
    /// Reads the cached semester list into the choice; the pick.
    fn read_semesters(&self, flow: &str) -> Option<String>;
    /// Course keys whose history no longer matches the attendance summary.
    fn mismatched(&self, flow: &str) -> Vec<String>;
    /// Biometric days from `from` to `to` cached after they ended.
    fn complete_days(&self, flow: &str, from: i32, to: i32) -> HashSet<i32>;
    /// The course parts in the cached course page.
    fn course_ids(&self, flow: &str) -> Vec<String>;
    /// A trip ahead is pending in the cached outings.
    fn pending_outing(&self, flow: &str) -> bool;
    /// Drops the half-done sign-in waiting on a code nobody gives.
    fn cancel_otp(&self, flow: String) -> BoxFuture<'static, ()>;
}

struct CoreSource(Arc<Core>);

impl SyncSource for CoreSource {
    fn fetch(&self, flow: String, page: SyncPage) -> BoxFuture<'static, Result<(), CoreError>> {
        let core = self.0.clone();
        async move {
            match page {
                SyncPage::Semesters => core.fetch_semesters(flow).await.map(drop),
                SyncPage::Attendance => core.fetch_attendance(flow).await.map(drop),
                SyncPage::History(key) => core.fetch_attendance_detail(flow, key).await.map(drop),
                SyncPage::Timetable => core.fetch_timetable(flow).await.map(drop),
                SyncPage::Marks => core.fetch_marks(flow).await.map(drop),
                SyncPage::Exams => core.fetch_exams(flow).await.map(drop),
                SyncPage::Calendar => core.fetch_calendar(flow).await.map(drop),
                SyncPage::Grades(semester) => core.fetch_grades(flow, semester).await.map(drop),
                SyncPage::GradeHistory => core.fetch_grade_history(flow).await.map(drop),
                SyncPage::Biometric(day) => {
                    core.fetch_biometric_range(flow, day, day).await.map(drop)
                }
                SyncPage::Profile => core.fetch_profile(flow).await.map(drop),
                SyncPage::CoursePage => core.fetch_course_page(flow).await.map(drop),
                SyncPage::CourseFiles(id) => {
                    let insights = core.settings().calculated_insights;
                    core.fetch_course_view(flow, id, None, insights)
                        .await
                        .map(drop)
                }
                SyncPage::GeneralOuting => core.fetch_general_outing(flow).await.map(drop),
                SyncPage::WeekendOuting => core.fetch_weekend_outing(flow).await.map(drop),
            }
        }
        .boxed()
    }

    fn read_semesters(&self, flow: &str) -> Option<String> {
        self.0.semester_read_cache(flow.into());
        self.0.selected_semester()
    }

    fn mismatched(&self, flow: &str) -> Vec<String> {
        self.0.mismatched_attendance_details(flow.into())
    }

    fn complete_days(&self, flow: &str, from: i32, to: i32) -> HashSet<i32> {
        self.0
            .cached_biometric_range(flow.into(), from, to)
            .into_iter()
            .filter(|day| day.complete)
            .map(|day| day.day)
            .collect()
    }

    fn course_ids(&self, flow: &str) -> Vec<String> {
        self.0
            .cached_course_page(flow.into())
            .map_or_default(|page| page.groups)
            .into_iter()
            .flat_map(|group| group.parts.into_iter().map(|part| part.id))
            .collect()
    }

    fn pending_outing(&self, flow: &str) -> bool {
        let general = self
            .0
            .cached_general_outing(flow.into())
            .map(|o| o.requests);
        let weekend = self
            .0
            .cached_weekend_outing(flow.into())
            .map(|o| o.requests);
        general
            .into_iter()
            .chain(weekend)
            .flatten()
            .any(|r| r.standing == OutingStanding::Pending && r.highlight.is_some())
    }

    fn cancel_otp(&self, flow: String) -> BoxFuture<'static, ()> {
        let core = self.0.clone();
        async move {
            #[expect(
                clippy::let_underscore_must_use,
                reason = "nothing pending, or signed out meanwhile, is fine: the call logged it"
            )]
            let _ = core.cancel_otp(flow).await;
        }
        .boxed()
    }
}

/// The newest `n` of the 14 days to `today` not in `complete`, newest first; today always
/// counts.
pub(crate) fn biometric_days(today: i32, complete: &HashSet<i32>, n: usize) -> Vec<i32> {
    (today - 13..=today)
        .rev()
        .filter(|day| *day == today || !complete.contains(day))
        .take(n)
        .collect()
}

/// The id after `last` in `ids`, wrapping round; the first when `last` is gone.
pub(crate) fn next_after(ids: &[String], last: Option<&str>) -> Option<String> {
    if ids.is_empty() {
        return None;
    }
    let at = last
        .and_then(|last| ids.iter().position(|id| id == last))
        .map_or(0, |at| (at + 1) % ids.len());
    ids.get(at).cloned()
}

/// Today in India, days since 1970-01-01.
fn ist_day(now_ms: i64) -> i32 {
    i32::try_from((now_ms.div_euclid(1000) + 19_800).div_euclid(86_400)).unwrap_or(i32::MAX)
}

/// Why a run stopped before its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// VTOP turned the password down: every other page would be too.
    Rejected,
    /// The OTP wasn't given.
    Otp,
    Cancelled,
}

/// The OTP rounds of one run: pages that run into it together share one ask.
#[derive(Default)]
struct OtpRounds {
    /// Codes accepted.
    rounds: u32,
    /// One wasn't given; the run stops.
    stopped: bool,
}

/// One run's state.
struct Run {
    core: Arc<Core>,
    source: Arc<dyn SyncSource>,
    otp: Arc<dyn SyncOtp>,
    flow: String,
    label: &'static str,
    kind: SyncKind,
    page_timeout_ms: Option<u64>,
    fetched: AtomicU32,
    failed: AtomicU32,
    rounds: smol::lock::Mutex<OtpRounds>,
}

/// How a queued page ended.
#[derive(Clone)]
enum Fetched {
    Ok,
    TimedOut,
}

impl Run {
    fn counts(&self) -> [(&'static str, String); 2] {
        [
            ("fetched", self.fetched.load(Ordering::SeqCst).to_string()),
            ("failed", self.failed.load(Ordering::SeqCst).to_string()),
        ]
    }

    /// `page` through the queue; one OTP round at most, after which it is fetched again.
    async fn page(&self, page: SyncPage) -> Result<(), Stop> {
        for attempt in 0..2 {
            let round = self.rounds.lock().await.rounds;
            match self.queued(&page).await {
                Ok(Fetched::Ok) => {
                    self.fetched.fetch_add(1, Ordering::SeqCst);
                    return Ok(());
                }
                Ok(Fetched::TimedOut) => {
                    self.failed.fetch_add(1, Ordering::SeqCst);
                    log(
                        "warn",
                        "sync",
                        &self.flow,
                        "page timed out",
                        [("page", page.id())],
                    );
                    return Ok(());
                }
                Err(FlightError::Failed(CoreError::InvalidCredentials)) => {
                    return Err(Stop::Rejected);
                }
                Err(FlightError::Failed(CoreError::OtpRequired { issued_at, .. })) => {
                    if attempt > 0 {
                        return Err(Stop::Otp);
                    }
                    let mut rounds = self.rounds.lock().await;
                    if rounds.stopped {
                        return Err(Stop::Otp);
                    }
                    if rounds.rounds == round {
                        if !self.otp.get(issued_at, self.flow.clone()).await {
                            rounds.stopped = true;
                            return Err(Stop::Otp);
                        }
                        rounds.rounds += 1;
                    }
                }
                Err(FlightError::Cancelled) => return Err(Stop::Cancelled),
                Err(error) => {
                    self.failed.fetch_add(1, Ordering::SeqCst);
                    let kind = match &error {
                        FlightError::Failed(error) => error.kind(),
                        FlightError::Dropped => "Dropped",
                        FlightError::Cancelled => "Cancelled",
                    };
                    log(
                        "warn",
                        "sync",
                        &self.flow,
                        &format!("page failed: {kind}"),
                        [("page", page.id())],
                    );
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// One try of `page` once it has a lane, tracked as `<label>: <name>`, within the page
    /// timeout.
    async fn queued(&self, page: &SyncPage) -> Result<Fetched, FlightError> {
        let core = self.core.clone();
        let fetch = self.source.fetch(self.flow.clone(), page.clone());
        let (flow, timeout) = (self.flow.clone(), self.page_timeout_ms);
        let label = format!("{}: {}", self.label, page.name());
        let everything = self.kind == SyncKind::FetchAll;
        let opts = RunOpts {
            user: everything,
            has_cache: false,
            background: !everything,
        };
        let work = async move {
            let mut timed_out = false;
            let tracked = core
                .schedule
                .activity
                .track(&label, &flow, async {
                    let Some(ms) = timeout else {
                        return fetch.await;
                    };
                    match core.schedule.runtime.timeout(ms, fetch).await {
                        Ok(fetched) => fetched,
                        Err(TimedOut) => {
                            timed_out = true;
                            Err(CoreError::VtopUnavailable {
                                reason: "VTOP took too long to answer".into(),
                            })
                        }
                    }
                })
                .await;
            match tracked {
                Ok(()) => Ok(Fetched::Ok),
                Err(_) if timed_out => Ok(Fetched::TimedOut),
                Err(error) => Err(error),
            }
        };
        let run = self
            .core
            .schedule
            .flight
            .start(&page.key(), &self.flow, opts, work);
        run.result.await
    }

    /// Every page in order; Err when the run stopped.
    async fn pages(&self) -> Result<(), Stop> {
        // Semesters and attendance first: the pick and the histories to fetch come from them.
        self.page(SyncPage::Semesters).await?;
        let selected = self.source.read_semesters(&self.flow);
        self.page(SyncPage::Attendance).await?;
        let everything = self.kind == SyncKind::FetchAll;
        let flow = self.flow.as_str();
        let mut pages: Vec<SyncPage> = self
            .source
            .mismatched(flow)
            .into_iter()
            .map(SyncPage::History)
            .collect();
        pages.extend([
            SyncPage::Timetable,
            SyncPage::Marks,
            SyncPage::Exams,
            SyncPage::Calendar,
        ]);
        pages.extend(selected.map(SyncPage::Grades));
        pages.push(SyncPage::GradeHistory);
        // Recent biometric days not final yet, so campus times build up over the semester.
        let today = ist_day(self.core.schedule.runtime.clock().now_ms());
        let complete = self.source.complete_days(flow, today - 13, today);
        let days = self
            .core
            .settings
            .get()
            .tunable(crate::settings::TunableKey::SyncBiometricDays);
        let days = biometric_days(today, &complete, usize::try_from(days).unwrap_or(0));
        pages.extend(days.into_iter().map(SyncPage::Biometric));
        if everything {
            pages.extend([SyncPage::Profile, SyncPage::CoursePage]);
        } else if let Some(id) = self.next_course() {
            // One course's lecture plan a run, in turn, so new files reach the inbox.
            pages.push(SyncPage::CourseFiles(id));
        }
        // Outings while a trip ahead is pending, so a decision shows in the inbox.
        if everything || self.source.pending_outing(flow) {
            pages.extend([SyncPage::GeneralOuting, SyncPage::WeekendOuting]);
        }
        // The rest at once; the lanes keep them in this order.
        futures::future::try_join_all(pages.into_iter().map(|page| self.page(page))).await?;
        Ok(())
    }

    /// The course part after the one checked last ([`crate::Settings::sync_course`]).
    fn next_course(&self) -> Option<String> {
        let ids = self.source.course_ids(&self.flow);
        let last = self.core.settings().sync_course;
        let next = next_after(&ids, last.as_deref())?;
        self.core.settings.set(SettingChange::SyncCourse {
            value: Some(next.clone()),
        });
        Some(next)
    }
}

/// Marks a flow unattended while it lives.
struct Unattended<'a> {
    core: &'a Core,
    flow: String,
}

impl Drop for Unattended<'_> {
    fn drop(&mut self) {
        self.core.events.set_unattended(&self.flow, false);
    }
}

impl Core {
    /// [`Core::sync`] over `source`.
    pub(crate) async fn sync_with(
        self: Arc<Self>,
        flow: String,
        kind: SyncKind,
        page_timeout_ms: Option<u64>,
        otp: Arc<dyn SyncOtp>,
        source: Arc<dyn SyncSource>,
    ) -> SyncResult {
        // Background runs never ask: consent uses the chosen route only with its "Don't ask
        // again" ticked, else Direct.
        let _unattended = (kind == SyncKind::Background).then(|| {
            self.events.set_unattended(&flow, true);
            Unattended {
                core: &self,
                flow: flow.clone(),
            }
        });
        let run = Run {
            core: self.clone(),
            source,
            otp,
            flow: flow.clone(),
            label: match kind {
                SyncKind::Background => "Background sync",
                SyncKind::FetchAll => "Fetch all",
            },
            kind,
            page_timeout_ms,
            fetched: AtomicU32::new(0),
            failed: AtomicU32::new(0),
            rounds: smol::lock::Mutex::default(),
        };
        let outcome = match run.pages().await {
            Ok(()) => {
                log("info", "sync", &flow, "done", run.counts());
                SyncOutcome::Done
            }
            Err(Stop::Rejected) => {
                log(
                    "info",
                    "sync",
                    &flow,
                    "stopped: password rejected",
                    run.counts(),
                );
                SyncOutcome::Rejected
            }
            Err(Stop::Otp) => {
                run.source.cancel_otp(flow.clone()).await;
                log("info", "sync", &flow, "stopped at the otp", run.counts());
                SyncOutcome::StoppedAtOtp
            }
            Err(Stop::Cancelled) => {
                log("info", "sync", &flow, "cancelled", run.counts());
                SyncOutcome::Cancelled
            }
        };
        let fetched = run.fetched.load(Ordering::SeqCst);
        if kind == SyncKind::Background {
            match outcome {
                SyncOutcome::Rejected => {
                    self.change_sync_health(&flow, |_| SyncHealth::Paused {
                        reason: PauseReason::PasswordRejected,
                    });
                }
                // A run that fetched nothing (offline, VTOP down) says nothing about auto-fill.
                SyncOutcome::Done if fetched > 0 => {
                    self.change_sync_health(&flow, |_| SyncHealth::OK);
                }
                _ => {}
            }
        }
        SyncResult {
            outcome,
            fetched,
            failed: run.failed.load(Ordering::SeqCst),
        }
    }
}

/// Class histories fetched in the background, one batch at a time.
#[derive(Default)]
pub(crate) struct Histories {
    /// `<semester>/<course key>` → when it was last tried (ms); held through a batch.
    tries: smol::lock::Mutex<HashMap<String, i64>>,
}

/// A course is tried at most once in this long per semester.
const HISTORY_RETRY_MS: i64 = 15 * 60_000;

/// What Task 7 adds to the core: the semester choice, the health lock and the histories.
#[derive(Default)]
pub(crate) struct SyncState {
    pub(crate) choice: crate::semester_choice::Choice,
    pub(crate) health: Mutex<()>,
    histories: Histories,
}

/// Forgets a history's try when the batch stops before it ends (the screen moved on).
struct Trying<'a> {
    tries: &'a mut HashMap<String, i64>,
    key: Option<String>,
}

impl Drop for Trying<'_> {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            self.tries.remove(&key);
        }
    }
}

#[uniffi::export]
impl Core {
    /// Fetches VTOP's pages in `flow`: semesters and attendance first, then the rest through
    /// the queue. Each page gets `page_timeout_ms` once it has a lane (None: the transports'
    /// own timeouts). A background run is unattended.
    pub async fn sync(
        self: Arc<Self>,
        flow: String,
        kind: SyncKind,
        page_timeout_ms: Option<u64>,
        otp: Arc<dyn SyncOtp>,
    ) -> SyncResult {
        let source = Arc::new(CoreSource(self.clone()));
        self.sync_with(flow, kind, page_timeout_ms, otp, source)
            .await
    }

    /// Fetches the class histories of `keys` in `flow` (`<src>.histories`, log src `ui.<src>`);
    /// with `semester`, each course at most once per 15 minutes in it. The keys that arrived.
    pub async fn histories_refresh(
        &self,
        flow: String,
        src: String,
        semester: Option<String>,
        keys: Vec<String>,
    ) -> Vec<String> {
        let mut tries = self.sync.histories.tries.lock().await;
        let clock = self.schedule.runtime.clock().clone();
        let now = clock.now_ms();
        let tried = |key: &String| semester.as_ref().map(|s| format!("{s}/{key}"));
        let due: Vec<String> = keys
            .into_iter()
            .filter(|key| {
                tried(key)
                    .and_then(|at| tries.get(&at).copied())
                    .is_none_or(|at| now - at >= HISTORY_RETRY_MS)
            })
            .collect();
        if due.is_empty() {
            return Vec::new();
        }
        let ui = format!("ui.{src}");
        log(
            "info",
            &ui,
            &flow,
            "histories to fetch",
            [("courses", due.len().to_string())],
        );
        let count = due.len().to_string();
        let arrived = self
            .schedule
            .activity
            .track("Refreshing class history", &flow, async {
                let mut arrived = Vec::new();
                for key in due {
                    let mut trying = Trying {
                        tries: &mut tries,
                        key: tried(&key),
                    };
                    if let Some(at) = &trying.key {
                        trying.tries.insert(at.clone(), clock.now_ms());
                    }
                    let fetched = self
                        .fetch_attendance_detail(flow.clone(), key.clone())
                        .await;
                    trying.key = None;
                    match fetched {
                        Ok(_) => arrived.push(key),
                        Err(error) => log(
                            "warn",
                            &ui,
                            &flow,
                            "history not fetched",
                            [("course", key), ("error", error.kind().into())],
                        ),
                    }
                }
                Ok::<_, CoreError>(arrived)
            })
            .await
            .unwrap_or_default();
        log(
            "info",
            &ui,
            &flow,
            "histories fetched",
            [("fetched", arrived.len().to_string()), ("due", count)],
        );
        arrived
    }
}

/// A line through the app's log, as Kotlin's `Log` wrote it.
fn log<const N: usize>(level: &str, src: &str, flow: &str, msg: &str, fields: [(&str, String); N]) {
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

#[cfg(test)]
mod tests;

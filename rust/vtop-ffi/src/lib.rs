//! The Compose prototype's core: one signed-in VTOP client, its session and
//! a JSON cache in the app's data directory, and ready-to-draw records.

mod calendar;
mod changes;
mod cookies;
mod dev;
mod exams;
mod marks;
mod obs;
mod records;
mod transport;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::field::Empty;
use tracing::{Instrument, Span, info, info_span, warn};
use vtop_core::inputs::{CourseId, CourseType, Password, SemesterId, Username};
use vtop_core::session::CookieStore;
use vtop_core::sync::RwLock;
use vtop_core::types::{
    AcademicCalendarData, AttendanceData, FullAttendanceData, PersistedVtopSession, SemesterData,
    TimetableData,
};
use vtop_core::{VtopClient, VtopError, VtopResult};

pub use biometric::{BiometricDay, Punch, PunchKind};
pub use calendar::{
    AcademicCalendar, CalendarDay, CalendarEvent, CalendarEventPart, CalendarKind, CalendarMonth,
};
pub use cookies::{SharedCookies, set_shared_cookies};
pub use course_page::{
    CourseGroup, CoursePage, CoursePart, CoursePlan, CourseSection, CourseView, Lecture, Material,
    SavedFile,
};
pub use dev::{CacheScope, DevConfig, dev_configure};
pub use exams::{ExamGroup, Exams, ScheduledPaper};
pub use forwarding::{Forwarding, ForwardingState, GmailConfirmation};
pub use grades::{
    GpaPlan, GradeComponent, GradeCount, GradeCourse, GradeCutoff, GradeDetails, GradeHistory,
    GradeMark, GradeSession, Grades, HistoryCourse, MarkSection, PlanCourse, PlannedCourse,
    SemesterSummary, TargetOutlook, plan_gpa, semester_summary,
};
pub use marks::{Assessment, Marks, MarksCourse, ScoreBand};
pub use obs::{log_event, obs_init};
pub use outing::{
    GeneralOuting, GeneralOutingCheck, GeneralOutingForm, GeneralOutingRules, GeneralOutingTimes,
    OutingChoice, OutingFact, OutingKind, OutingMoment, OutingRequest, OutingResult,
    OutingStanding, OutingWhen, WeekendOuting, WeekendOutingCheck, WeekendOutingForm,
    WeekendOutingRules, check_general_outing, check_weekend_outing, general_outing_times,
    general_return_last_day,
};
pub use profile::{
    Address, Contact, ContactKind, DetailSection, Field, FieldGroup, Hostel, Mentor, MentorMessage,
    Parent, Profile,
};
pub use records::{
    Attendance, AttendanceCourse, AttendanceDetail, AttendancePlan, AttendedClass, CalendarMark,
    ClassDay, ClassKind, ClassSlot, ClassStatus, Course, CourseOutlook, DayClass, Debar, ExamPaper,
    ExamStart, NextClass, Semester, Semesters, Timetable, TimetableDay,
};
pub use transport::{
    Header, HttpRequest, HttpResponse, HttpTransport, TransportError, set_http_transport,
    vtop_intermediate_ca,
};
pub use web::WebSession;

uniffi::setup_scaffolding!();

const ACCOUNT: &str = "account.json";
const SESSION: &str = "session.json";
const SEMESTERS: &str = "semesters.json";
/// The semester the student picked; pages wait for it, as in the Flutter app.
const SELECTED: &str = "semester.json";
/// Pages are cached per semester: `pages/<semester id>/<page>.json`.
const PAGES: &str = "pages";
const ATTENDANCE: &str = "attendance.json";
const TIMETABLE: &str = "timetable.json";
const CALENDAR: &str = "calendar.json";
/// One course's class-by-class history: `attendance_detail/<course key>.json`.
const ATTENDANCE_DETAIL: &str = "attendance_detail";

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SignIn {
    Done,
    NeedsOtp { message: String },
}

/// Why a call failed. The text field is `reason`, not `message`: Kotlin
/// exceptions already have a `message` and UniFFI would declare it twice.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum CoreError {
    Offline,
    VtopUnavailable {
        reason: String,
    },
    InvalidCredentials,
    /// A re-login during a fetch wants the emailed OTP.
    OtpRequired {
        reason: String,
    },
    /// `submit_otp` got a wrong or expired code; VTOP's text.
    OtpRejected {
        reason: String,
    },
    NotSignedIn,
    /// No semester picked yet (or VTOP no longer lists the pick): the
    /// student picks one from [`Core::fetch_semesters`] first.
    SemesterNeeded,
    Other {
        reason: String,
    },
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Offline => formatter.write_str("offline"),
            Self::InvalidCredentials => formatter.write_str("invalid credentials"),
            Self::NotSignedIn => formatter.write_str("not signed in"),
            Self::SemesterNeeded => formatter.write_str("no semester picked"),
            Self::VtopUnavailable { reason }
            | Self::OtpRequired { reason }
            | Self::OtpRejected { reason }
            | Self::Other { reason } => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for CoreError {}

impl CoreError {
    /// The variant's name, for logs.
    fn kind(&self) -> &'static str {
        match self {
            Self::Offline => "Offline",
            Self::VtopUnavailable { .. } => "VtopUnavailable",
            Self::InvalidCredentials => "InvalidCredentials",
            Self::OtpRequired { .. } => "OtpRequired",
            Self::OtpRejected { .. } => "OtpRejected",
            Self::NotSignedIn => "NotSignedIn",
            Self::SemesterNeeded => "SemesterNeeded",
            Self::Other { .. } => "Other",
        }
    }
}

impl From<VtopError> for CoreError {
    fn from(error: VtopError) -> Self {
        match error {
            VtopError::NetworkError => Self::Offline,
            VtopError::VtopUnavailable(reason) => Self::VtopUnavailable { reason },
            VtopError::InvalidCredentials => Self::InvalidCredentials,
            VtopError::OTPRequired(reason, _) => Self::OtpRequired { reason },
            error => Self::Other {
                reason: error.to_string(),
            },
        }
    }
}

impl From<std::io::Error> for CoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Other {
            reason: error.to_string(),
        }
    }
}

/// Saved sign-in. TODO: move to the Android Keystore.
#[derive(Clone, Serialize, Deserialize)]
struct Account {
    username: String,
    password: String,
    /// VTOP rejected this password: no sign-in is tried with it again (repeated failures can
    /// lock the VTOP account) until the student signs in with a new one.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    rejected: bool,
    /// VTOP's registration number, saved at the first sign-in (the college email ends with it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    registration_number: Option<String>,
}

/// [`Core::saved_account`]'s answer.
#[derive(Clone, PartialEq, uniffi::Record)]
pub struct SavedAccount {
    pub username: String,
    pub password: String,
    pub rejected: bool,
    /// Known once signed in.
    pub registration_number: Option<String>,
}

struct Session {
    client: VtopClient,
    /// The client's cookies: the shared store's (the web view's) once this session is installed.
    cookies: Arc<cookies::SessionCookies>,
    account: Account,
    /// Bumped on every sign-in, so concurrent fetches re-login once.
    logins: u64,
    /// The OTP prompt while VTOP waits for the emailed code.
    otp: Option<String>,
}

impl Session {
    fn new(account: Account) -> Result<Self, CoreError> {
        let username =
            Username::parse(&account.username).map_err(|_| CoreError::InvalidCredentials)?;
        let password =
            Password::parse(&account.password).map_err(|_| CoreError::InvalidCredentials)?;
        let cookies = cookies::SessionCookies::new();
        let client = VtopClient::builder()
            .config(dev::vtop_config())
            .transport(transport::current()?)
            .cookie_store(cookies.clone());
        Ok(Self {
            client: client.with_credentials(username, password)?,
            cookies,
            account,
            logins: 0,
            otp: None,
        })
    }

    /// Starts over on a fresh client with no cookies (the shared store's go too): no
    /// half-finished login, cookies, CSRF token or OTP wait.
    fn restart(&mut self) -> Result<(), CoreError> {
        let fresh = Self::new(self.account.clone())?;
        fresh.cookies.activate(true);
        (self.client, self.cookies, self.otp) = (fresh.client, fresh.cookies, None);
        Ok(())
    }
}

/// A fetch of one page for a semester.
trait PageOp<T>:
    for<'a> Fn(
        &'a VtopClient,
        &'a SemesterId,
    ) -> Pin<Box<dyn Future<Output = VtopResult<T>> + Send + 'a>>
    + Sync
{
}

impl<T, F> PageOp<T> for F where
    F: for<'a> Fn(
            &'a VtopClient,
            &'a SemesterId,
        ) -> Pin<Box<dyn Future<Output = VtopResult<T>> + Send + 'a>>
        + Sync
{
}

type Slot = RwLock<Session>;

/// Runs `$call` with `$client` bound to the signed-in client. When the
/// session has expired, signs in again with the saved credentials and runs it
/// once more. Nothing is returned for a client signed out meanwhile.
macro_rules! signed_in {
    ($core:expr, |$client:ident| $call:expr) => {{
        let core = $core;
        let slot = core.current()?;
        let first = {
            let session = slot.read().await;
            let $client = &session.client;
            match $call.await {
                Err(VtopError::SessionExpired) => Err(session.logins),
                result => Ok(result?),
            }
        };
        let data = match first {
            Ok(data) => data,
            Err(seen) => {
                core.relogin(&slot, seen).await?;
                let session = slot.read().await;
                let $client = &session.client;
                $call.await.map_err(|error| match error {
                    VtopError::SessionExpired => CoreError::Other {
                        reason: "VTOP ended the session right after signing in".to_string(),
                    },
                    error => error.into(),
                })?
            }
        };
        if core.is_current(&slot) {
            Ok(data)
        } else {
            Err(CoreError::NotSignedIn)
        }
    }};
}

// After `signed_in!`, which it uses.
mod biometric;
mod course_page;
mod forwarding;
mod grades;
mod outing;
mod profile;
mod web;

#[derive(uniffi::Object)]
pub struct Core {
    dir: PathBuf,
    slot: Mutex<Option<Arc<Slot>>>,
}

/// Every call takes the caller's flow id first; its span logs `ms` and
/// `outcome` on close.
#[uniffi::export]
impl Core {
    /// Opens the cache in `data_dir` and restores the saved session.
    #[uniffi::constructor]
    pub fn new(flow: String, data_dir: String) -> Result<Arc<Self>, CoreError> {
        obs::obs_init(data_dir.clone());
        let core = Self {
            dir: PathBuf::from(data_dir),
            slot: Mutex::new(None),
        };
        traced_sync("new", &flow, || {
            fs::create_dir_all(&core.dir)?;
            let Some(account) = core.read::<Account>(ACCOUNT) else {
                return Ok(());
            };
            let session = Session::new(account)?;
            let saved = core
                .read::<PersistedVtopSession>(SESSION)
                .filter(dev::reusable);
            core.restore(&session, saved);
            core.install(Some(Arc::new(RwLock::new(session))));
            Ok(())
        })?;
        Ok(Arc::new(core))
    }

    /// The saved account's registration number.
    pub fn signed_in_user(&self, flow: String) -> Option<String> {
        traced_sync(
            "signed_in_user",
            &flow,
            || Ok(self.read::<Account>(ACCOUNT)),
        )
        .ok()
        .flatten()
        .map(|account| account.username)
    }

    /// The saved username and password, and whether VTOP rejected the password, for the
    /// Account page (opened behind the phone's screen lock) and Settings' "Needs attention".
    /// Never logged.
    pub fn saved_account(&self, flow: String) -> Option<SavedAccount> {
        traced_sync("saved_account", &flow, || Ok(self.read::<Account>(ACCOUNT)))
            .ok()
            .flatten()
            .map(|account| SavedAccount {
                username: account.username,
                password: account.password,
                rejected: account.rejected,
                // An account saved before this field: the saved session has it.
                registration_number: account.registration_number.or_else(|| {
                    self.read::<PersistedVtopSession>(SESSION)
                        .and_then(|saved| saved.registration_number)
                }),
            })
    }

    pub async fn sign_in(
        &self,
        flow: String,
        username: String,
        password: String,
    ) -> Result<SignIn, CoreError> {
        let account = Account {
            username: username.trim().to_uppercase(),
            password,
            rejected: false,
            registration_number: None,
        };
        traced("sign_in", &flow, async {
            let mut session = Session::new(account)?;
            let outcome = match session.client.login().await {
                Ok(()) => SignIn::Done,
                Err(VtopError::OTPRequired(message, _)) => {
                    session.otp = Some(message.clone());
                    SignIn::NeedsOtp { message }
                }
                Err(error) => return Err(error.into()),
            };
            // VTOP asks for the OTP only after it accepted the password. For the saved
            // account (the Account page fixing a password), keep the new password now, so
            // a cancelled OTP doesn't leave the old, rejected one on disk.
            if outcome != SignIn::Done {
                self.keep_accepted_password(&session.account);
            }
            // Accepted: its cookies become the app's (and the web view's).
            session.cookies.activate(true);
            let slot = Arc::new(RwLock::new(session));
            self.install(Some(slot.clone()));
            if outcome == SignIn::Done {
                let session = slot.read().await;
                obs::set_regno(session.client.registration_number().ok());
                self.save(&slot, &session);
            }
            Ok(outcome)
        })
        .await
    }

    pub async fn submit_otp(&self, flow: String, code: String) -> Result<(), CoreError> {
        traced("submit_otp", &flow, async {
            let slot = self.current()?;
            let mut session = slot.write().await;
            session
                .client
                .submit_security_otp(code.trim())
                .await
                .map_err(|error| match error {
                    VtopError::AuthenticationFailed(reason) => CoreError::OtpRejected { reason },
                    error => error.into(),
                })?;
            session.otp = None;
            session.logins += 1;
            obs::set_regno(session.client.registration_number().ok());
            self.save(&slot, &session);
            Ok(())
        })
        .await
    }

    pub async fn resend_otp(&self, flow: String) -> Result<(), CoreError> {
        traced("resend_otp", &flow, async {
            let slot = self.current()?;
            let mut session = slot.write().await;
            Ok(session.client.resend_security_otp().await?)
        })
        .await
    }

    /// Drops a re-login's pending OTP challenge (the student cancelled the prompt), so the next
    /// fetch that needs a session signs in afresh and VTOP mails a new code, instead of reopening
    /// a challenge VTOP may have dropped meanwhile. Nothing pending: nothing changes.
    pub async fn cancel_otp(&self, flow: String) -> Result<(), CoreError> {
        traced("cancel_otp", &flow, async {
            let slot = self.current()?;
            let mut session = slot.write().await;
            if session.otp.is_some() {
                session.restart()?;
                info!("pending otp dropped");
            }
            Ok(())
        })
        .await
    }

    /// Forgets the account, session, cache, change inbox and mail-forwarding address (a sign-in can join the
    /// address again through the edge). Logs stay.
    pub fn sign_out(&self, flow: String) {
        let _ = traced_sync("sign_out", &flow, || {
            self.install(None);
            cookies::sign_out();
            self.forget(&[
                ACCOUNT,
                SESSION,
                SEMESTERS,
                SELECTED,
                forwarding::FILE,
                changes::FILE,
            ]);
            obs::set_regno(None);
            Ok(())
        });
    }

    /// The cached semester list with the selected one, if a list was fetched.
    pub fn cached_semesters(&self, flow: String) -> Option<Semesters> {
        traced_sync("cached_semesters", &flow, || {
            Ok(self
                .read::<SemesterData>(SEMESTERS)
                .map(|list| self.semesters(list)))
        })
        .ok()
        .flatten()
    }

    /// Fetches the semester list from VTOP (newest first) and caches it.
    pub async fn fetch_semesters(&self, flow: String) -> Result<Semesters, CoreError> {
        traced("fetch_semesters", &flow, async {
            let list = signed_in!(self, |client| client.semesters())?;
            if list.semesters.is_empty() {
                return Err(VtopError::ParseError("VTOP lists no semesters".into()).into());
            }
            self.write(SEMESTERS, &list);
            info!(semesters = list.semesters.len(), "semesters");
            Ok(self.semesters(list))
        })
        .await
    }

    /// Picks the semester every page is fetched and cached for, until the
    /// student picks another. It must be in the cached list.
    pub fn select_semester(&self, flow: String, id: String) -> Result<(), CoreError> {
        traced_sync("select_semester", &flow, || {
            if !self
                .read::<SemesterData>(SEMESTERS)
                .is_some_and(|list| list.semesters.iter().any(|semester| semester.id == id))
            {
                return Err(CoreError::Other {
                    reason: format!("unknown semester {id}"),
                });
            }
            self.write(SELECTED, &id);
            info!(semester = %id, "semester selected");
            Ok(())
        })
    }

    pub fn cached_attendance(&self, flow: String) -> Option<Attendance> {
        traced_sync("cached_attendance", &flow, || {
            Ok(self
                .read_page::<AttendanceData>(ATTENDANCE)
                .map(Attendance::from))
        })
        .ok()
        .flatten()
    }

    pub async fn fetch_attendance(&self, flow: String) -> Result<Attendance, CoreError> {
        traced("fetch_attendance", &flow, async {
            let data = self
                .fetch(ATTENDANCE, |client, semester| {
                    Box::pin(client.attendance(semester))
                })
                .await?;
            info!(courses = data.records.len(), "attendance");
            Ok(data.into())
        })
        .await
    }

    /// The saved class-by-class history of `course_key` (an
    /// [`AttendanceCourse::key`] in the saved attendance).
    pub fn cached_attendance_detail(
        &self,
        flow: String,
        course_key: String,
    ) -> Option<AttendanceDetail> {
        traced_sync("cached_attendance_detail", &flow, || {
            let Some(course) = self.cached_course(&course_key) else {
                return Ok(None);
            };
            Ok(self
                .read_page::<FullAttendanceData>(&detail_file(&course))
                .map(|data| {
                    let plans = self.plans(&course);
                    AttendanceDetail::new(course, data, plans)
                }))
        })
        .ok()
        .flatten()
    }

    /// Fetches `course_key`'s class-by-class history and saves it. The course
    /// comes from the saved attendance, fetched first when it lacks the course.
    pub async fn fetch_attendance_detail(
        &self,
        flow: String,
        course_key: String,
    ) -> Result<AttendanceDetail, CoreError> {
        traced("fetch_attendance_detail", &flow, async {
            // One fetch per course at a time (the attendance list and the
            // timetable both sync histories); a caller that waited, or one
            // right after, gets the history just fetched.
            let lock = detail_lock(&course_key);
            let _one = lock.write().await;
            if let Some(course) = self.cached_course(&course_key)
                && let Some(data) = self.read_page::<FullAttendanceData>(&detail_file(&course))
                && unix_now() - data.update_time as i64 <= DETAIL_FRESH_S
            {
                info!(course = %course.key, "attendance detail just fetched");
                let plans = self.plans(&course);
                return Ok(AttendanceDetail::new(course, data, plans));
            }
            let course = match self.cached_course(&course_key) {
                Some(course) => course,
                None => Attendance::from(
                    self.fetch(ATTENDANCE, |client, semester| {
                        Box::pin(client.attendance(semester))
                    })
                    .await?,
                )
                .courses
                .into_iter()
                .find(|course| course.key == course_key)
                .ok_or_else(|| CoreError::Other {
                    reason: format!("no course {course_key} in this semester's attendance"),
                })?,
            };
            let (id, kind) = (
                CourseId::parse(&course.course_id)?,
                CourseType::parse(&course.course_type)?,
            );
            let data = self
                .fetch(&detail_file(&course), |client, semester| {
                    let (id, kind) = (id.clone(), kind.clone());
                    Box::pin(async move { client.full_attendance(semester, &id, &kind).await })
                })
                .await?;
            info!(course = %course.key, classes = data.records.len(), "attendance detail");
            let plans = self.plans(&course);
            Ok(AttendanceDetail::new(course, data, plans))
        })
        .await
    }

    /// Keys of the courses whose saved history is missing or no longer
    /// matches the summary (VTOP says 10/11, the history 9/10), for the list
    /// to fetch after each refresh (like Flutter's `pullMismatchedHistory`,
    /// which skips missing ones; here they are fetched too). Courses without
    /// classes yet are left out. No network.
    pub fn mismatched_attendance_details(&self, flow: String) -> Vec<String> {
        traced_sync("mismatched_attendance_details", &flow, || {
            let keys: Vec<String> = self
                .read_page::<AttendanceData>(ATTENDANCE)
                .map(|data| Attendance::from(data).courses)
                .unwrap_or_default()
                .into_iter()
                .filter(|course| match self.cached_history(course) {
                    Some(classes) => !records::history_in_sync(course, &classes),
                    // Not fetched yet: worth fetching once VTOP lists a class.
                    None => course.total > 0,
                })
                .map(|course| course.key)
                .collect();
            info!(mismatched = keys.len(), "mismatched attendance details");
            Ok(keys)
        })
        .unwrap_or_default()
    }

    /// Each saved course's plan, until the next exam when one comes before
    /// the last class day, else until the semester's end; from the saved
    /// attendance, timetable and academic calendar (empty without a
    /// calendar). No network.
    pub fn attendance_outlook(&self, flow: String) -> Vec<CourseOutlook> {
        traced_sync("attendance_outlook", &flow, || {
            let (Some(attendance), Some((calendar, slots))) = (
                self.read_page::<AttendanceData>(ATTENDANCE),
                self.plan_inputs(),
            ) else {
                return Ok(Vec::new());
            };
            let (today, minute) = records::ist_now();
            let outlook: Vec<CourseOutlook> = Attendance::from(attendance)
                .courses
                .iter()
                .filter_map(|course| {
                    let through = self
                        .cached_history(course)
                        .and_then(|classes| records::counted_through(course, &classes));
                    let plan = records::attendance_plans(
                        course, &slots, &calendar, today, minute, through,
                    )
                    .into_iter()
                    .next()?;
                    Some(CourseOutlook {
                        key: course.key.clone(),
                        plan,
                    })
                })
                .collect();
            info!(courses = outlook.len(), "attendance outlook");
            Ok(outlook)
        })
        .unwrap_or_default()
    }

    pub fn cached_timetable(&self, flow: String) -> Option<Timetable> {
        traced_sync("cached_timetable", &flow, || {
            Ok(self
                .read_page::<TimetableData>(TIMETABLE)
                .map(Timetable::from))
        })
        .ok()
        .flatten()
    }

    /// The next class of each course in the cached timetable, within a week
    /// of `now_unix` at the device's `utc_offset_minutes`.
    pub fn next_classes(
        &self,
        flow: String,
        now_unix: i64,
        utc_offset_minutes: i32,
    ) -> Vec<NextClass> {
        traced_sync("next_classes", &flow, || {
            let slots = self
                .read_page::<TimetableData>(TIMETABLE)
                .map_or_else(Vec::new, |data| Timetable::from(data).slots);
            let calendar = self
                .read_page::<AcademicCalendarData>(CALENDAR)
                .map(|data| calendar::SemesterCalendar::of(&data));
            Ok(records::next_classes(
                &slots,
                now_unix,
                utc_offset_minutes,
                calendar.as_ref(),
            ))
        })
        .unwrap_or_default()
    }

    /// `count` agenda days from epoch day `from_day` (local), from the
    /// cached timetable and academic calendar (without a calendar every
    /// class is held).
    pub fn timetable_days(&self, flow: String, from_day: i32, count: i32) -> Vec<TimetableDay> {
        traced_sync("timetable_days", &flow, || {
            let slots = self
                .read_page::<TimetableData>(TIMETABLE)
                .map_or_else(Vec::new, |data| Timetable::from(data).slots);
            let calendar = self
                .read_page::<AcademicCalendarData>(CALENDAR)
                .map(|data| calendar::SemesterCalendar::of(&data));
            let mut days = records::timetable_days(&slots, from_day, count, calendar.as_ref());
            let histories: HashMap<(String, ClassKind), Vec<AttendedClass>> = self
                .read_page::<AttendanceData>(ATTENDANCE)
                .map(|data| Attendance::from(data).courses)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|course| {
                    let classes = self.cached_history(&course)?;
                    Some(((course.code, course.kind), classes))
                })
                .collect();
            records::fill_posted(&mut days, &histories);
            exams::fill_papers(&mut days, self.read_page(exams::EXAMS));
            info!(
                histories = histories.len(),
                from_day,
                days = days.len(),
                slots = slots.len(),
                calendar = calendar.is_some(),
                marked = days.iter().filter(|day| day.mark.is_some()).count(),
                "timetable days"
            );
            Ok(days)
        })
        .unwrap_or_default()
    }

    /// Keys of the saved courses with a class held in `count` days from
    /// `from_day` (up to today) whose saved history is missing or behind
    /// the summary, for the timetable to fetch in the background. No network.
    pub fn stale_attendance_details(&self, flow: String, from_day: i32, count: i32) -> Vec<String> {
        traced_sync("stale_attendance_details", &flow, || {
            let Some(attendance) = self.read_page::<AttendanceData>(ATTENDANCE) else {
                return Ok(Vec::new());
            };
            let slots = self
                .read_page::<TimetableData>(TIMETABLE)
                .map_or_else(Vec::new, |data| Timetable::from(data).slots);
            let calendar = self
                .read_page::<AcademicCalendarData>(CALENDAR)
                .map(|data| calendar::SemesterCalendar::of(&data));
            let (today, _) = records::ist_now();
            let held: Vec<(String, ClassKind)> =
                records::timetable_days(&slots, from_day, count, calendar.as_ref())
                    .iter()
                    .filter(|day| day.day <= today)
                    .flat_map(|day| &day.classes)
                    .filter(|class| class.state == ClassDay::Held)
                    .map(|class| (class.slot.code.clone(), class.slot.kind))
                    .collect();
            let stale: Vec<String> = Attendance::from(attendance)
                .courses
                .into_iter()
                .filter(|course| held.contains(&(course.code.clone(), course.kind)))
                .filter(|course| {
                    self.cached_history(course)
                        .is_none_or(|classes| !records::history_in_sync(course, &classes))
                })
                .map(|course| course.key)
                .collect();
            info!(stale = stale.len(), "stale attendance details");
            Ok(stale)
        })
        .unwrap_or_default()
    }

    /// The selected semester's cached academic calendar as months, as of
    /// today in India.
    pub fn cached_calendar(&self, flow: String) -> Option<AcademicCalendar> {
        traced_sync("cached_calendar", &flow, || {
            Ok(self
                .read_page::<AcademicCalendarData>(CALENDAR)
                .map(|data| calendar_view(&data)))
        })
        .ok()
        .flatten()
    }

    /// Fetches the selected semester's academic calendar (one request for
    /// the month list, then one per month) and caches it.
    pub async fn fetch_calendar(&self, flow: String) -> Result<AcademicCalendar, CoreError> {
        traced("fetch_calendar", &flow, async {
            let data = self
                .fetch(CALENDAR, |client, semester| {
                    Box::pin(client.academic_calendar(semester))
                })
                .await?;
            Ok(calendar_view(&data))
        })
        .await
    }

    pub async fn fetch_timetable(&self, flow: String) -> Result<Timetable, CoreError> {
        traced("fetch_timetable", &flow, async {
            let data = self
                .fetch(TIMETABLE, |client, semester| {
                    Box::pin(client.timetable(semester))
                })
                .await?;
            info!(slots = data.slots.len(), "timetable");
            Ok(data.into())
        })
        .await
    }
}

impl Core {
    fn install(&self, slot: Option<Arc<Slot>>) {
        *self.slot.lock().unwrap_or_else(PoisonError::into_inner) = slot;
    }

    fn current(&self) -> Result<Arc<Slot>, CoreError> {
        let slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        slot.clone().ok_or(CoreError::NotSignedIn)
    }

    /// Whether `slot` is still the signed-in one (not signed out meanwhile).
    fn is_current(&self, slot: &Arc<Slot>) -> bool {
        self.current()
            .is_ok_and(|current| Arc::ptr_eq(&current, slot))
    }

    /// Fetches a page of the selected semester and caches it as `file` for
    /// that semester.
    async fn fetch<T: Serialize + Send>(
        &self,
        file: &str,
        op: impl PageOp<T>,
    ) -> Result<T, CoreError> {
        self.current()?; // Signed out wins over no pick.
        let semester = self
            .read::<SemesterData>(SEMESTERS)
            .and_then(|list| self.pick(&list))
            .ok_or(CoreError::SemesterNeeded)?;
        let id = SemesterId::parse(&semester).map_err(CoreError::from)?;
        let data = signed_in!(self, |client| op(client, &id))?;
        let path = format!("{PAGES}/{semester}/{file}");
        if let Some(parent) = self.dir.join(&path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        self.write(&path, &data);
        Ok(data)
    }

    /// The student's pick, if `list` still has it.
    fn pick(&self, list: &SemesterData) -> Option<String> {
        let selected = self.read::<String>(SELECTED)?;
        list.semesters
            .iter()
            .any(|semester| semester.id == selected)
            .then_some(selected)
    }

    fn semesters(&self, list: SemesterData) -> Semesters {
        Semesters {
            selected: self.pick(&list),
            updated_at: list.update_time as i64,
            items: list
                .semesters
                .into_iter()
                .map(|semester| Semester::new(semester.id, semester.name))
                .collect(),
        }
    }

    /// `course`'s plans from now, by the saved timetable and academic
    /// calendar; none without a calendar (its last class day is unknown).
    fn plans(&self, course: &AttendanceCourse) -> Vec<AttendancePlan> {
        self.plan_inputs()
            .map(|(calendar, slots)| {
                let (today, minute) = records::ist_now();
                let through = self
                    .cached_history(course)
                    .and_then(|classes| records::counted_through(course, &classes));
                records::attendance_plans(course, &slots, &calendar, today, minute, through)
            })
            .unwrap_or_default()
    }

    /// `course`'s saved class history, newest first.
    fn cached_history(&self, course: &AttendanceCourse) -> Option<Vec<AttendedClass>> {
        let data = self.read_page::<FullAttendanceData>(&detail_file(course))?;
        Some(AttendanceDetail::new(course.clone(), data, Vec::new()).classes)
    }

    /// The saved academic calendar and timetable slots plans count from.
    fn plan_inputs(&self) -> Option<(calendar::SemesterCalendar, Vec<ClassSlot>)> {
        let Some(calendar) = self
            .read_page::<AcademicCalendarData>(CALENDAR)
            .map(|data| calendar::SemesterCalendar::of(&data))
        else {
            info!("no calendar: no plan");
            return None;
        };
        let slots = self
            .read_page::<TimetableData>(TIMETABLE)
            .map_or_else(Vec::new, |data| Timetable::from(data).slots);
        Some((calendar, slots))
    }

    /// `key`'s course in the saved attendance of the selected semester.
    fn cached_course(&self, key: &str) -> Option<AttendanceCourse> {
        Attendance::from(self.read_page::<AttendanceData>(ATTENDANCE)?)
            .courses
            .into_iter()
            .find(|course| course.key == key)
    }

    /// A cached page of the selected semester.
    fn read_page<T: DeserializeOwned>(&self, file: &str) -> Option<T> {
        let list = self.read::<SemesterData>(SEMESTERS)?;
        self.read(&format!("{PAGES}/{}/{file}", self.pick(&list)?))
    }

    async fn relogin(&self, slot: &Arc<Slot>, seen: u64) -> Result<(), CoreError> {
        let mut session = slot.write().await;
        if session.logins != seen {
            return Ok(()); // Another call already signed in again.
        }
        if !self.is_current(slot) {
            // Replaced or signed out meanwhile: its cookies are gone; never sign it in.
            return Err(CoreError::NotSignedIn);
        }
        if let Some(message) = &session.otp {
            return Err(CoreError::OtpRequired {
                reason: message.clone(),
            });
        }
        if session.account.rejected {
            info!("session expired; the saved password was rejected, not signing in again");
            return Err(CoreError::InvalidCredentials);
        }
        info!("session expired, signing in again");
        if let Err(error) = session.client.login().await {
            match &error {
                VtopError::OTPRequired(message, _) => session.otp = Some(message.clone()),
                VtopError::InvalidCredentials => {
                    warn!("VTOP rejected the saved password; no more sign-ins with it");
                    CookieStore::clear(session.cookies.as_ref());
                    session.account.rejected = true;
                    if self.is_current(slot) {
                        self.write_account(&session.account);
                    }
                }
                _ => {}
            }
            return Err(error.into());
        }
        session.logins += 1;
        self.save(slot, &session);
        Ok(())
    }

    fn save(&self, slot: &Arc<Slot>, session: &Session) {
        if !self.is_current(slot) {
            return;
        }
        // Another student signed in over the saved one ("Sign in again"): the last one's
        // semesters, pick and pages are not theirs.
        if self
            .read::<Account>(ACCOUNT)
            .is_some_and(|saved| saved.username != session.account.username)
        {
            info!("another account signed in: cached pages and forwarding address dropped");
            self.forget(&[SEMESTERS, SELECTED, forwarding::FILE, changes::FILE]);
        }
        let saved_at_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis() as u64);
        let mut account = session.account.clone();
        if let Ok(regno) = session.client.registration_number() {
            account.registration_number = Some(regno);
        }
        self.write_account(&account);
        let mut saved = session.client.export_persisted_session(saved_at_ms);
        if session.cookies.is_shared() {
            // The shared store keeps (and saves) them.
            saved.cookies = None;
            session.cookies.flush();
        }
        self.write(SESSION, &saved);
    }

    /// Restores `saved` into `session` (not installed yet) and makes it the shared store's
    /// session. Without cookies in `saved`, the store's are this session's and stay; a
    /// `session.json` from before the shared store (cookies in it) moves them there once and is
    /// saved again without; no `saved`: the store starts empty.
    fn restore(&self, session: &Session, saved: Option<PersistedVtopSession>) {
        let carry = saved.as_ref().is_none_or(|saved| saved.cookies.is_some());
        if let Some(saved) = &saved {
            obs::set_regno(saved.registration_number.clone());
            match session.client.restore_persisted_session(saved) {
                Ok(()) => info!(
                    cookies_in_file = saved.cookies.is_some(),
                    "session restored"
                ),
                Err(error) => warn!(%error, "saved session not restored"),
            }
        }
        session.cookies.activate(carry);
        if let Some(mut saved) = saved
            && saved.cookies.is_some()
            && session.cookies.is_shared()
        {
            saved.cookies = None;
            self.write(SESSION, &saved);
            info!("saved cookies moved to the shared store");
        }
    }

    /// Writes `account`, keeping the saved registration number when `account` has none (it is
    /// only known once signed in).
    fn write_account(&self, account: &Account) {
        let mut account = account.clone();
        if account.registration_number.is_none() {
            account.registration_number = self
                .read::<Account>(ACCOUNT)
                .filter(|saved| saved.username == account.username)
                .and_then(|saved| saved.registration_number);
        }
        self.write(ACCOUNT, &account);
    }

    /// Saves `account`'s password over the saved account's when it's the same username
    /// (another username is another account, saved only once signed in).
    fn keep_accepted_password(&self, account: &Account) {
        if let Some(saved) = self.read::<Account>(ACCOUNT)
            && saved.username == account.username
            && (saved.password != account.password || saved.rejected)
        {
            info!("password accepted before the otp: saved");
            self.write_account(account);
        }
    }

    /// Removes `files` and every cached page.
    fn forget(&self, files: &[&str]) {
        // Not while a pull records mail into forwarding.json (it would write it back).
        let _forwarding = forwarding::FILE_LOCK
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for file in files {
            match fs::remove_file(self.dir.join(file)) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    warn!(file, %error, "not removed");
                }
                _ => {}
            }
        }
        match fs::remove_dir_all(self.dir.join(PAGES)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                warn!(%error, "cached pages not removed");
            }
            _ => {}
        }
    }

    fn read<T: DeserializeOwned>(&self, file: &str) -> Option<T> {
        let bytes = fs::read(self.dir.join(file)).ok()?;
        serde_json::from_slice(&bytes)
            .inspect_err(|error| warn!(file, %error, "unreadable"))
            .ok()
    }

    /// Writes `value` as JSON through a temporary file, so a crash never
    /// leaves half a file. A page compared for the change inbox records
    /// what changed against the copy it replaces.
    fn write(&self, file: &str, value: &impl Serialize) {
        match serde_json::to_vec(value) {
            Ok(bytes) => {
                self.record_changes(file, &bytes);
                self.write_bytes(file, &bytes);
            }
            Err(error) => warn!(file, %error, "not saved"),
        }
    }

    /// [`Core::write`] without the change inbox (the inbox's own file).
    fn write_plain(&self, file: &str, value: &impl Serialize) {
        match serde_json::to_vec(value) {
            Ok(bytes) => self.write_bytes(file, &bytes),
            Err(error) => warn!(file, %error, "not saved"),
        }
    }

    fn write_bytes(&self, file: &str, bytes: &[u8]) {
        let path = self.dir.join(file);
        let temp = path.with_extension("tmp");
        let result = fs::write(&temp, bytes).and_then(|()| fs::rename(&temp, &path));
        if let Err(error) = result {
            warn!(file, %error, "not saved");
        }
    }
}

/// A history this young (seconds) is served again instead of fetched.
const DETAIL_FRESH_S: i64 = 20;

/// The lock that keeps one history fetch per course key at a time.
fn detail_lock(key: &str) -> Arc<RwLock<()>> {
    static LOCKS: std::sync::LazyLock<Mutex<HashMap<String, Arc<RwLock<()>>>>> =
        std::sync::LazyLock::new(Mutex::default);
    LOCKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .entry(key.to_string())
        .or_default()
        .clone()
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

/// Named from the course found, never from the caller's key, so a key can't
/// reach outside the cache.
fn detail_file(course: &AttendanceCourse) -> String {
    format!(
        "{ATTENDANCE_DETAIL}/{}.json",
        course.key.replace(['/', '\\'], "_")
    )
}

/// The month view of `data`, with what it holds logged.
fn calendar_view(data: &AcademicCalendarData) -> AcademicCalendar {
    let view = calendar::SemesterCalendar::of(data).view(data, records::today_ist_day());
    info!(
        entries = data.entries.len(),
        months = view.months.len(),
        days_left = view.days_left,
        "calendar"
    );
    view
}

fn span(call: &str, flow: &str) -> Span {
    info_span!("core", src = %format_args!("core.{call}"), flow, outcome = Empty, error = Empty)
}

/// Puts how a call ended on its span, for the close line.
fn close<T>(span: &Span, result: &Result<T, CoreError>) {
    match result {
        Ok(_) => span.record("outcome", "ok"),
        Err(error) => span
            .record("outcome", error.kind())
            .record("error", tracing::field::display(error)),
    };
}

async fn traced<T>(
    call: &str,
    flow: &str,
    work: impl Future<Output = Result<T, CoreError>>,
) -> Result<T, CoreError> {
    let span = span(call, flow);
    let result = transport::with_flow(flow.to_string(), work.instrument(span.clone())).await;
    close(&span, &result);
    result
}

fn traced_sync<T>(
    call: &str,
    flow: &str,
    work: impl FnOnce() -> Result<T, CoreError>,
) -> Result<T, CoreError> {
    let span = span(call, flow);
    let result = span.in_scope(work);
    close(&span, &result);
    result
}

#[cfg(test)]
mod tests;

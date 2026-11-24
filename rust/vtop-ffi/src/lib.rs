//! The app's core: one signed-in VTOP client, its session and
//! a JSON cache in the app's data directory, and ready-to-draw records.

// The uniffi::export futures (fetch_pages through traced and with_flow) nest
// deeper than the default 128 when the compiler proves them Send.
#![recursion_limit = "256"]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::string_slice,
        clippy::panic
    )
)]
#![cfg_attr(
    test,
    allow(
        clippy::let_underscore_must_use,
        reason = "test setup and cleanup ignore results on purpose"
    )
)]

#[macro_use]
mod macros;

mod convex;
mod demo;
mod dev;
mod device;
mod obs;
mod pages;
mod remote;
mod session;
mod store;
#[cfg(test)]
mod test_edge;
mod transport;
mod web;

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::field::Empty;
use tracing::{Instrument, Span, info, info_span};
use vtop_core::inputs::{Password, Username};
use vtop_core::spec::{self, PageSpec, Query};
use vtop_core::sync::RwLock;
use vtop_core::types::{PersistedVtopSession, SemesterData};
use vtop_core::{VtopClient, VtopError};

pub use obs::log_event;
pub use pages::calendar::CalendarKind;
pub use pages::records::{
    Attendance, AttendanceCourse, AttendanceDetail, AttendancePlan, AttendedClass, ClassDay,
    ClassKind, ClassSlot, ClassStatus, CourseOutlook, NextClass, Semester, Semesters, Timetable,
    TimetableDay,
};
pub use store::cookies::SharedCookies;

uniffi::setup_scaffolding!();

const ACCOUNT: &str = "account.json";
const SESSION: &str = "session.json";
const SEMESTERS: &str = spec::SEMESTERS.file;
/// The semester the student picked; pages wait for it, as in the Flutter app.
const SELECTED: &str = "semester.json";
/// Pages are cached per semester: `pages/<semester id>/<page>.json`.
const PAGES: &str = "pages";
/// The signed-in student's own files, forgotten (with every cached page) on sign-out and when
/// another account signs in over the saved one.
const STUDENT_FILES: [&str; 7] = [
    SEMESTERS,
    SELECTED,
    remote::forwarding::FILE,
    pages::changes::FILE,
    remote::consent::FILE,
    remote::account_token::FILE,
    demo::FILE,
];
const ATTENDANCE: &str = spec::ATTENDANCE.file;
const TIMETABLE: &str = spec::TIMETABLE.file;
const CALENDAR: &str = spec::ACADEMIC_CALENDAR.file;
/// One course's class-by-class history: `attendance_detail/<course key>.json`.
const ATTENDANCE_DETAIL: &str = spec::FULL_ATTENDANCE.file;

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
    /// A re-login during a fetch wants the emailed OTP, mailed at `issued_at` (unix seconds;
    /// None when unknown).
    OtpRequired {
        reason: String,
        issued_at: Option<u64>,
    },
    /// `submit_otp` got a wrong or expired code; VTOP's text.
    OtpRejected {
        reason: String,
    },
    NotSignedIn,
    /// No semester picked yet (or VTOP no longer lists the pick): the
    /// student picks one from [`Core::fetch_semesters`] first.
    SemesterNeeded,
    /// The student said no (or wasn't there to ask): the VTOP session stayed on the phone.
    ConsentDenied,
    /// This phone has no forwarding address, or the server deleted it: an OTP wait on it ends.
    ForwardingOff {
        reason: String,
    },
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
            Self::ConsentDenied => {
                formatter.write_str("Not sent: your VTOP cookie stays on the phone.")
            }
            Self::VtopUnavailable { reason }
            | Self::OtpRequired { reason, .. }
            | Self::OtpRejected { reason }
            | Self::ForwardingOff { reason }
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
            Self::ConsentDenied => "ConsentDenied",
            Self::ForwardingOff { .. } => "ForwardingOff",
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
            VtopError::OTPRequired(reason, issued_at) => Self::OtpRequired {
                reason,
                issued_at: Some(issued_at),
            },
            error => Self::Other {
                reason: error.to_string(),
            },
        }
    }
}

impl From<uniffi::UnexpectedUniFFICallbackError> for CoreError {
    fn from(error: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Other {
            reason: error.reason,
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
    cookies: Arc<store::cookies::SessionCookies>,
    account: Account,
    /// Bumped on every sign-in, so concurrent fetches re-login once.
    logins: u64,
    /// The OTP prompt while VTOP waits for the emailed code.
    otp: Option<String>,
    /// That sign-in was the server's ([`Core::server_otp`] sends the code).
    server_otp: bool,
}

impl Session {
    /// A client for `account` whose hops go through `shared`'s routes.
    fn new(account: Account, shared: &Arc<Shared>) -> Result<Self, CoreError> {
        let username =
            Username::parse(&account.username).map_err(|_| CoreError::InvalidCredentials)?;
        let password =
            Password::parse(&account.password).map_err(|_| CoreError::InvalidCredentials)?;
        let cookies = store::cookies::SessionCookies::new();
        let client = VtopClient::builder()
            .config(dev::vtop_config(shared))
            .transport(transport::current(shared)?)
            .cookie_store(cookies.clone());
        Ok(Self {
            client: client.with_credentials(username, password)?,
            cookies,
            account,
            logins: 0,
            otp: None,
            server_otp: false,
        })
    }

    /// Starts over on a fresh client with no cookies (the shared store's go too): no
    /// half-finished login, cookies, CSRF token or OTP wait.
    fn restart(&mut self, shared: &Arc<Shared>) -> Result<(), CoreError> {
        let fresh = Self::new(self.account.clone(), shared)?;
        fresh.cookies.activate(true);
        (self.client, self.cookies, self.otp) = (fresh.client, fresh.cookies, None);
        self.server_otp = false;
        Ok(())
    }
}

type Slot = RwLock<Session>;

/// The core's state that every VTOP client shares: the route plan, cookie consent and the
/// Developer config. Each client's transport ([`remote::route::Routed`]) holds the same `Arc`.
#[derive(Default)]
struct Shared {
    route: Mutex<remote::route::State>,
    /// The server's client and its JWT.
    backend: Mutex<remote::backend::State>,
    /// One identity fetch at a time ([`remote::backend`]): a refresh token works once.
    identity_turn: futures::lock::Mutex<()>,
    consent: Mutex<remote::consent::State>,
    /// What [`dev::dev_configure`] set before the core was made.
    dev: Option<dev::DevConfig>,
    /// The server account proofs are held to, and its roots, in place of VTOP's.
    #[cfg(all(test, feature = "tlsn"))]
    proof_target: Mutex<Option<(String, vtop_tlsn::RootCertStore)>>,
}

impl Shared {
    /// With what Kotlin set before making the core: the cookie gate and the Developer config.
    fn handed() -> Self {
        let shared = Self {
            dev: dev::handed(),
            ..Self::default()
        };
        shared.consent().gate = remote::consent::handed();
        shared
    }
}

#[derive(uniffi::Object)]
pub struct Core {
    dir: PathBuf,
    /// The change inbox, which Android may clear (see [`Core::at`]).
    cache: PathBuf,
    slot: Mutex<Option<Arc<Slot>>>,
    shared: Arc<Shared>,
}

/// Every call takes the caller's flow id first; its span logs `ms` and
/// `outcome` on close.
#[uniffi::export]
impl Core {
    /// Opens the cache in `data_dir`, the change inbox in `cache_dir`, and
    /// restores the saved session.
    #[uniffi::constructor]
    pub fn new(flow: String, data_dir: String, cache_dir: String) -> Result<Arc<Self>, CoreError> {
        obs::obs_init(cache_dir.clone());
        let core = Self {
            dir: PathBuf::from(data_dir),
            cache: PathBuf::from(cache_dir),
            slot: Mutex::new(None),
            shared: Arc::new(Shared::handed()),
        };
        traced_sync("new", &flow, || {
            fs::create_dir_all(&core.dir)?;
            fs::create_dir_all(&core.cache)?;
            remote::route::load(&core);
            remote::consent::load(&core);
            let Some(account) = core.read::<Account>(ACCOUNT) else {
                return Ok(());
            };
            let session = Session::new(account, &core.shared)?;
            let saved = core
                .read::<PersistedVtopSession>(SESSION)
                .filter(|saved| dev::reusable(&core.shared, saved));
            core.restore(&session, saved);
            core.install(Some(Arc::new(RwLock::new(session))));
            Ok(())
        })?;
        Ok(Arc::new(core))
    }

    /// The cached semester list with the selected one, if a list was fetched.
    pub fn cached_semesters(&self, flow: String) -> Option<Semesters> {
        traced_sync("cached_semesters", &flow, || {
            Ok(self
                .read_kept::<SemesterData>(SEMESTERS)
                .map(|list| self.semesters(list)))
        })
        .ok()
        .flatten()
    }

    /// Fetches the semester list from VTOP (newest first) and caches it.
    pub async fn fetch_semesters(&self, flow: String) -> Result<Semesters, CoreError> {
        traced("fetch_semesters", &flow, async {
            let list: SemesterData = self.page(&spec::SEMESTERS, None, &[]).await?;
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
                .read_kept::<SemesterData>(SEMESTERS)
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
}

impl Core {
    /// A core on `dir` (its cache folder too), signed out, with nothing set by Kotlin.
    #[cfg(test)]
    fn for_test(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self {
            cache: dir.clone(),
            dir,
            slot: Mutex::new(None),
            shared: Arc::default(),
        }
    }

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

    /// Page `spec` of the selected semester, fetched and cached at `file` (by default the
    /// spec's) under that semester.
    async fn fetch<T: Serialize + DeserializeOwned + Send + 'static>(
        &self,
        spec: &PageSpec<T>,
        file: Option<&str>,
        args: &[(&str, &str)],
    ) -> Result<T, CoreError> {
        self.current()?; // Signed out wins over no pick.
        let semester = self
            .read_kept::<SemesterData>(SEMESTERS)
            .and_then(|list| self.pick(&list))
            .ok_or(CoreError::SemesterNeeded)?;
        let data = self.page(spec, Some(&semester), args).await?;
        let path = format!("{PAGES}/{semester}/{}", file.unwrap_or(spec.file));
        if let Some(parent) = self.at(&path).parent() {
            #[expect(
                clippy::let_underscore_must_use,
                reason = "a failed mkdir shows up in the write that follows"
            )]
            let _ = fs::create_dir_all(parent);
        }
        self.write(&path, &data);
        Ok(data)
    }

    /// Page `spec` from the server when the route says so (`Core::server_page`), else from
    /// VTOP here; not cached.
    async fn page<T: Serialize + DeserializeOwned + Send>(
        &self,
        spec: &PageSpec<T>,
        semester: Option<&str>,
        args: &[(&str, &str)],
    ) -> Result<T, CoreError> {
        let data = match self.server_page(spec.name, semester, args).await {
            Some(result) => result?,
            None => signed_in!(self, |client| client.fetch(spec, &Query { semester, args }))?,
        };
        self.record_demo(spec.name, semester, args, &data);
        Ok(data)
    }

    /// The student's pick, if `list` still has it.
    fn pick(&self, list: &SemesterData) -> Option<String> {
        let selected = self.read_kept::<String>(SELECTED)?;
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

    /// A cached page of the selected semester.
    fn read_page<T: DeserializeOwned + Clone + Send + Sync + 'static>(
        &self,
        file: &str,
    ) -> Option<T> {
        let list = self.read_kept::<SemesterData>(SEMESTERS)?;
        self.read_kept(&format!("{PAGES}/{}/{file}", self.pick(&list)?))
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
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
    // A Server or UDP call logs nothing until its answer is back: this line shows it in flight
    // (Activity). The OTP prompt's forwarded-mail poll stays quiet.
    if (call.starts_with("fetch_") || call.starts_with("download_") || call == "sign_in")
        && !call.contains("forward")
    {
        span.in_scope(|| info!("start"));
    }
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

/// `value[key] = item`, without the panic `IndexMut` has when `value` is not an object.
pub(crate) fn json_set(value: &mut serde_json::Value, key: &str, item: serde_json::Value) {
    if value.is_null() {
        *value = serde_json::Value::Object(serde_json::Map::new());
    }
    if let Some(map) = value.as_object_mut() {
        map.insert(key.to_owned(), item);
    }
}

#[cfg(test)]
mod tests;

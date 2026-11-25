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

mod app_files;
mod convex;
mod demo;
mod dev;
mod device;
mod obs;
mod otp;
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

use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::field::Empty;
use tracing::{Instrument, Span, info, info_span, warn};
use vtop_core::inputs::{Password, Username};
use vtop_core::spec::{self, PageSpec, Query};
use vtop_core::sync::RwLock;
use vtop_core::types::{PersistedVtopSession, SemesterData};
use vtop_core::{VtopClient, VtopError};

pub use obs::log_event;
pub use pages::calendar::CalendarKind;
pub use pages::records::{
    Attendance, AttendanceCourse, AttendanceDetail, AttendancePlan, AttendanceProjection,
    AttendedClass, ClassDay, ClassKind, ClassSlot, ClassStatus, CourseOutlook, GoalSplit,
    NextClass, Semester, Semesters, Timetable, TimetableDay, attendance_goal_split,
    project_attendance,
};
pub use store::account::{
    AccountListener, AccountStatus, Opening, PasswordCipher, RejectedBy, Sealing, Standing,
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

/// Why VTOP rejected an OTP submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum OtpRejection {
    /// Wrong or unreadable code; the challenge stays open.
    Wrong,
    /// VTOP said this code has expired; ask for a new one.
    Expired,
    /// The sign-in ended; VTOP mailed a fresh code.
    SignInEnded,
}

/// Why a forwarding call found mail forwarding off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ForwardingOffKind {
    /// No forwarding address is saved on this phone.
    NotSetUp,
    /// Kept from v2 or missing an identity: reconnects once signed in.
    NeedsSignIn,
    /// The address was deleted on the server.
    AddressDeleted,
    /// This phone was removed from the account on the server.
    DeviceRemoved,
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
    /// `submit_otp` got a wrong or expired code, or the sign-in ended; VTOP's text.
    OtpRejected {
        kind: OtpRejection,
        reason: String,
    },
    NotSignedIn,
    /// Signed in, but the phone's Keystore didn't open the saved password just now; the next
    /// call tries again.
    PasswordLocked,
    /// No semester picked yet (or VTOP no longer lists the pick): the
    /// student picks one from [`Core::fetch_semesters`] first.
    SemesterNeeded,
    /// The student said no (or wasn't there to ask): the VTOP session stayed on the phone.
    ConsentDenied,
    /// The student hasn't let this install share their registration number with the server
    /// (`remote/regno.rs`): no server identity is taken.
    RegnoNotShared,
    /// This phone has no forwarding address, or the server deleted it: an OTP wait on it ends.
    ForwardingOff {
        kind: ForwardingOffKind,
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
            Self::PasswordLocked => formatter.write_str("the saved password didn't open"),
            Self::SemesterNeeded => formatter.write_str("no semester picked"),
            Self::ConsentDenied => {
                formatter.write_str("Not sent: your VTOP cookie stays on the phone.")
            }
            Self::RegnoNotShared => {
                formatter.write_str("Not shared: your registration number stays on the phone.")
            }
            Self::VtopUnavailable { reason }
            | Self::OtpRequired { reason, .. }
            | Self::OtpRejected { reason, .. }
            | Self::ForwardingOff { reason, .. }
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
            Self::PasswordLocked => "PasswordLocked",
            Self::SemesterNeeded => "SemesterNeeded",
            Self::ConsentDenied => "ConsentDenied",
            Self::RegnoNotShared => "RegnoNotShared",
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
            VtopError::OtpExpired(reason) => Self::OtpRejected {
                kind: OtpRejection::Expired,
                reason,
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

/// Saved sign-in; on disk its password is sealed and whether VTOP rejected it is kept beside it
/// ([`store::account`]). In memory a rejected password is the session's
/// [`SessionPhase::PasswordRejected`].
#[derive(Clone)]
struct Account {
    username: String,
    /// Always there: a saved account without one (another phone's) is a
    /// [`store::account::Saved`], never an `Account`.
    password: String,
    /// VTOP's registration number, saved at the first sign-in (the college email ends with it).
    registration_number: Option<String>,
}

/// [`Core::saved_account`]'s answer.
#[derive(Clone, PartialEq, uniffi::Record)]
pub struct SavedAccount {
    pub username: String,
    /// Known once signed in.
    pub registration_number: Option<String>,
}

/// Where an active security OTP challenge started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OtpVia {
    Phone,
    Server,
}

/// Lifecycle of an installed VTOP session slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionPhase {
    /// Normal session (signed in or restored cookies ready to validate/relogin).
    Active,
    /// VTOP accepted the password and is waiting for the emailed security OTP.
    AwaitingOtp {
        via: OtpVia,
        message: String,
        challenge_id: u64,
    },
    /// VTOP rejected the saved password; no automatic sign-in is tried again (repeated failures
    /// can lock the VTOP account) until the student signs in with a new one.
    PasswordRejected,
}

struct Session {
    client: VtopClient,
    /// The client's cookies: the shared store's (the web view's) once this session is installed.
    cookies: Arc<store::cookies::SessionCookies>,
    account: Account,
    /// Bumped on every sign-in, so concurrent fetches re-login once.
    logins: u64,
    phase: SessionPhase,
    /// Monotonic id for OTP challenges on this slot so stale submissions are rejected.
    next_challenge: u64,
    /// The OTP prompt every session shares ([`Shared::otp`]) and this session's id on it.
    otp: Arc<otp::Board>,
    otp_session: u64,
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
            phase: SessionPhase::Active,
            next_challenge: 0,
            otp: shared.otp.clone(),
            otp_session: shared.otp.session(),
        })
    }

    #[cfg(test)]
    pub(crate) fn for_test(client: VtopClient, account: Account) -> Self {
        let otp = Arc::new(otp::Board::default());
        Self {
            client,
            cookies: store::cookies::SessionCookies::new(),
            account,
            logins: 0,
            phase: SessionPhase::Active,
            next_challenge: 0,
            otp_session: otp.session(),
            otp,
        }
    }

    /// The session as saved: `rejected` when VTOP had rejected its password.
    fn rejected(mut self, rejected: bool) -> Self {
        if rejected {
            self.phase = SessionPhase::PasswordRejected;
        }
        self
    }

    pub(crate) fn is_awaiting_otp(&self) -> bool {
        matches!(self.phase, SessionPhase::AwaitingOtp { .. })
    }

    pub(crate) fn otp_message(&self) -> Option<&str> {
        match &self.phase {
            SessionPhase::AwaitingOtp { message, .. } => Some(message.as_str()),
            _ => None,
        }
    }

    pub(crate) fn server_otp_challenge(&self) -> Option<u64> {
        match self.phase {
            SessionPhase::AwaitingOtp {
                via: OtpVia::Server,
                challenge_id,
                ..
            } => Some(challenge_id),
            _ => None,
        }
    }

    pub(crate) fn is_password_rejected(&self) -> bool {
        matches!(self.phase, SessionPhase::PasswordRejected)
    }

    pub(crate) fn begin_otp(&mut self, via: OtpVia, message: String) -> u64 {
        self.next_challenge = self.next_challenge.saturating_add(1);
        let challenge_id = self.next_challenge;
        self.otp
            .challenged(self.otp_session, &message, self.client.otp_issued_at());
        self.phase = SessionPhase::AwaitingOtp {
            via,
            message,
            challenge_id,
        };
        challenge_id
    }

    /// VTOP mailed a new code for the same challenge: an open prompt shows when, so auto-fill
    /// waits for that mail.
    pub(crate) fn otp_resent(&self) {
        if let Some(message) = self.otp_message() {
            self.otp
                .challenged(self.otp_session, message, self.client.otp_issued_at());
        }
    }

    pub(crate) fn mark_signed_in(&mut self) {
        self.phase = SessionPhase::Active;
        self.logins = self.logins.saturating_add(1);
        self.otp.close(self.otp_session);
    }

    pub(crate) fn mark_password_rejected(&mut self) {
        vtop_core::session::CookieStore::clear(self.cookies.as_ref());
        self.phase = SessionPhase::PasswordRejected;
        self.otp.close(self.otp_session);
    }

    /// Starts over on a fresh client with no cookies (the shared store's go too): no
    /// half-finished login, cookies, CSRF token or OTP wait. A rejected password stays rejected.
    fn restart(&mut self, shared: &Arc<Shared>) -> Result<(), CoreError> {
        let fresh = Self::new(self.account.clone(), shared)?.rejected(self.is_password_rejected());
        fresh.cookies.activate(&shared.cookies, true);
        self.client = fresh.client;
        self.cookies = fresh.cookies;
        self.phase = fresh.phase;
        // An open prompt stays: a fresh sign-in under it follows, or the caller closes it.
        self.otp.touch();
        Ok(())
    }
}

type Slot = RwLock<Session>;

/// Who [`Core`] is signed in as. `username` is the session's account's (fixed for a slot but
/// for Developer's in-place change, [`Core::rename_student`]), so the [`AccountStatus`] names
/// the installed student without asking the session's lock.
enum Signed {
    Out,
    /// An account is saved but the Keystore didn't open its password when the core opened:
    /// [`Core::current`] opens it again, so a Keystore hiccup doesn't sign the student out.
    Locked {
        username: String,
    },
    In {
        slot: Arc<Slot>,
        username: String,
    },
}

/// What the app hands [`Core::new`]: its HTTP stack, cookie store, sockets, consent prompt,
/// Developer config and this phone's name. One record, so the app can't leave a piece out.
#[derive(uniffi::Record)]
pub struct Platform {
    /// Carries VTOP's (and the Proxy route's) hops.
    pub http: Arc<dyn transport::HttpTransport>,
    /// Android's `CookieManager`, shared with the web view.
    pub cookies: Arc<dyn SharedCookies>,
    /// The UDP route's sockets.
    pub udp: Arc<dyn remote::udp::UdpNetwork>,
    /// The server's WebSocket.
    pub convex: Arc<dyn convex::ConvexNetwork>,
    /// Account proofs' WebSockets.
    pub proof: Arc<dyn remote::account_proof::ProofNetwork>,
    /// Asked before any VTOP session leaves the phone.
    pub cookie_gate: Arc<dyn remote::consent::CookieGate>,
    /// Seals the saved password (the Android Keystore).
    pub cipher: Arc<dyn PasswordCipher>,
    /// Shows the OTP prompt ([`otp`]).
    pub otp: Arc<dyn otp::OtpListener>,
    pub dev: dev::DevConfig,
    /// The name the server shows for this phone ("OnePlus CPH2573").
    pub device_label: String,
}

/// The core's state that every VTOP client shares: the route plan, cookie consent, the
/// Developer config and what the app handed over ([`Platform`]). Each client's transport
/// ([`remote::route::Routed`]) holds the same `Arc`. The platform's pieces are optional only so
/// host tests can leave out what they don't use; the app always sets every one.
#[derive(Default)]
struct Shared {
    route: Mutex<remote::route::State>,
    /// The server's client and its JWT.
    backend: Mutex<remote::backend::State>,
    /// One identity fetch at a time ([`remote::backend`]): a refresh token works once.
    identity_turn: futures::lock::Mutex<()>,
    consent: Mutex<remote::consent::State>,
    /// One registration-number sheet at a time, and the flow that last said "Not now"
    /// ([`remote::regno`]).
    regno_ask: futures::lock::Mutex<Option<String>>,
    dev: Option<dev::DevConfig>,
    /// Without one (host tests), vtop-core's own.
    http: Option<Arc<dyn vtop_core::client::Transport>>,
    /// The shared cookie store and which session's cookies it holds.
    cookies: store::cookies::Shared,
    /// Without one, the UDP route fails and falls back to HTTPS.
    udp: Option<Arc<dyn remote::udp::UdpNetwork>>,
    convex: Option<Arc<dyn convex::ConvexNetwork>>,
    proof: Option<Arc<dyn remote::account_proof::ProofNetwork>>,
    /// This phone's name on the server; `vitapmate` when none was given.
    label: Option<String>,
    /// Without one (host tests), the password is saved as it is.
    cipher: Option<Arc<dyn PasswordCipher>>,
    /// The OTP prompt and the calls waiting on it.
    otp: Arc<otp::Board>,
    /// This core's file locks and its copy of `device.json`.
    locks: store::Locks,
    /// A hash of the JWT last sent to the UDP gateway (0: none), which it signs in with when the
    /// session it holds expires. Each new JWT goes once, whatever its lifetime.
    udp_jwt_sent: std::sync::atomic::AtomicU64,
    /// The semester list, the pick and pages, kept decoded ([`store::decoded`]).
    decoded: store::decoded::Decoded,
    /// The password last sealed or opened ([`store::account`]).
    sealed: store::account::Sealed,
    /// Who hears [`AccountStatus`] changes, and what it heard last.
    account_watch: Mutex<store::account::Watch>,
    /// The server account proofs are held to, and its roots, in place of VTOP's.
    #[cfg(all(test, feature = "tlsn"))]
    proof_target: Mutex<Option<(String, vtop_tlsn::RootCertStore)>>,
}

impl From<Platform> for Shared {
    fn from(platform: Platform) -> Self {
        let dev = &platform.dev;
        info!(
            session_reuse_minutes = dev.session_reuse_minutes,
            request_timeout_s = dev.request_timeout_s,
            debuggable = dev.debuggable,
            "platform: foreign"
        );
        let shared = Self {
            dev: Some(platform.dev),
            http: Some(Arc::new(transport::Foreign(platform.http))),
            cookies: store::cookies::Shared::new(platform.cookies),
            udp: Some(platform.udp),
            convex: Some(platform.convex),
            proof: Some(platform.proof),
            label: Some(platform.device_label.trim().chars().take(64).collect()),
            cipher: Some(platform.cipher),
            ..Self::default()
        };
        shared.consent().gate = Some(platform.cookie_gate);
        shared.otp.set_listener(platform.otp);
        shared
    }
}

#[derive(uniffi::Object)]
pub struct Core {
    dir: PathBuf,
    /// The change inbox, which Android may clear (see [`Core::at`]).
    cache: PathBuf,
    slot: Mutex<Signed>,
    shared: Arc<Shared>,
}

/// Every call takes the caller's flow id first; its span logs `ms` and
/// `outcome` on close.
#[uniffi::export]
impl Core {
    /// Opens the cache in `data_dir`, the change inbox in `cache_dir`, and
    /// restores the saved session, reaching the world through `platform`.
    #[uniffi::constructor]
    pub fn new(
        flow: String,
        data_dir: String,
        cache_dir: String,
        platform: Platform,
    ) -> Result<Arc<Self>, CoreError> {
        Self::open(flow, data_dir, cache_dir, platform.into())
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
    /// [`Core::new`] on `shared` (host tests leave the platform out).
    fn open(
        flow: String,
        data_dir: String,
        cache_dir: String,
        shared: Shared,
    ) -> Result<Arc<Self>, CoreError> {
        obs::obs_init(cache_dir.clone());
        let core = Self {
            dir: PathBuf::from(data_dir),
            cache: PathBuf::from(cache_dir),
            slot: Mutex::new(Signed::Out),
            shared: Arc::new(shared),
        };
        traced_sync("new", &flow, || {
            fs::create_dir_all(&core.dir)?;
            fs::create_dir_all(&core.cache)?;
            core.sweep_temporary();
            remote::route::load(&core);
            remote::consent::load(&core);
            core.upgrade_account();
            let signed = core.restore_saved()?;
            *core.slot.lock().unwrap_or_else(PoisonError::into_inner) = signed;
            Ok(())
        })?;
        Ok(Arc::new(core))
    }

    /// The saved account's session, restored; no password (another phone's, or the key is
    /// gone): signed out.
    fn restore_saved(&self) -> Result<Signed, CoreError> {
        let Some(saved) = self.read_account() else {
            return Ok(Signed::Out);
        };
        let rejected = saved.rejected;
        let account = match saved.password {
            store::account::SavedPassword::Locked => {
                return Ok(Signed::Locked {
                    username: saved.username,
                });
            }
            store::account::SavedPassword::Gone => return Ok(Signed::Out),
            store::account::SavedPassword::Open(_) => saved.usable(),
        };
        let Some(account) = account else {
            return Ok(Signed::Out);
        };
        let session = Session::new(account, &self.shared)?.rejected(rejected);
        let saved = self
            .read::<PersistedVtopSession>(SESSION)
            .filter(|saved| dev::reusable(&self.shared, saved));
        self.restore(&session, saved);
        Ok(Signed::In {
            username: session.account.username.clone(),
            slot: Arc::new(RwLock::new(session)),
        })
    }

    /// A core on `dir` (its cache folder too), signed out, with no platform.
    #[cfg(test)]
    fn for_test(dir: impl Into<PathBuf>) -> Self {
        Self::for_test_on(dir, Shared::default())
    }

    /// [`Core::for_test`] with the platform pieces set in `shared`.
    #[cfg(test)]
    fn for_test_on(dir: impl Into<PathBuf>, shared: Shared) -> Self {
        let dir = dir.into();
        Self {
            cache: dir.clone(),
            dir,
            slot: Mutex::new(Signed::Out),
            shared: Arc::new(shared),
        }
    }

    /// Makes `session` the signed-in one, in its own slot.
    fn install(&self, session: Session) -> Arc<Slot> {
        let username = session.account.username.clone();
        let slot = Arc::new(RwLock::new(session));
        self.put(Signed::In {
            slot: slot.clone(),
            username,
        });
        slot
    }

    /// Drops the signed-in session (a sign-out).
    fn uninstall(&self) {
        self.put(Signed::Out);
    }

    fn put(&self, signed: Signed) {
        *self.slot.lock().unwrap_or_else(PoisonError::into_inner) = signed;
        // The last session's prompt (and the calls waiting on it) end with it.
        self.shared.otp.close_all();
        self.shared.identity_may_have_changed();
        self.account_changed();
    }

    /// Whether an account is signed in (its password perhaps not opened yet).
    fn has_account(&self) -> bool {
        !matches!(
            *self.slot.lock().unwrap_or_else(PoisonError::into_inner),
            Signed::Out
        )
    }

    /// The signed-in session. A password the Keystore didn't open at start is opened again here;
    /// when it turns out gone, the student hears they are signed out.
    fn current(&self) -> Result<Arc<Slot>, CoreError> {
        let (current, reopened) = {
            let mut signed = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
            let reopened = matches!(*signed, Signed::Locked { .. });
            if reopened {
                *signed = self.restore_saved().unwrap_or_else(|error| {
                    warn!(%error, "saved account not restored");
                    Signed::Out
                });
                if let Signed::In { .. } = *signed {
                    info!("saved password opened on a later try");
                }
            }
            let current = match &*signed {
                Signed::In { slot, .. } => Ok(slot.clone()),
                Signed::Locked { .. } => Err(CoreError::PasswordLocked),
                Signed::Out => Err(CoreError::NotSignedIn),
            };
            (current, reopened)
        };
        // Outside the slot's lock: the status reads it.
        if reopened {
            self.account_changed();
        }
        current
    }

    /// The signed-in student's username (their password perhaps not opened yet); None signed out.
    fn student(&self) -> Option<String> {
        match &*self.slot.lock().unwrap_or_else(PoisonError::into_inner) {
            Signed::Out => None,
            Signed::Locked { username } | Signed::In { username, .. } => Some(username.clone()),
        }
    }

    /// `slot`'s account took `username` in place (Developer → Set credentials).
    fn rename_student(&self, slot: &Arc<Slot>, username: &str) {
        if let Signed::In {
            slot: current,
            username: named,
        } = &mut *self.slot.lock().unwrap_or_else(PoisonError::into_inner)
            && Arc::ptr_eq(current, slot)
        {
            username.clone_into(named);
        }
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

//! App settings persisted in `<files>/settings.json` (not a student file: survives sign-out).
//!
//! `SettingsStore` opens synchronously before the first frame so theme and display size never
//! flash. Installs updating from SharedPreferences pass every key from `"settings"` (plus legacy
//! `"sync"`'s `otp_misses`) to [`SettingsStore::import`]; Kotlin clears those keys only after
//! `settings.json` lands on disk.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::CoreError;
use crate::pages::changes::ChangeKind;
use crate::store::write_atomic;

const FILE: &str = "settings.json";
const DISPLAY_SIZES: [i32; 4] = [0, 90, 100, 110];
const DISPLAY_SIZE_DEFAULT: i32 = 100;
const SYNC_HOURS: [i32; 4] = [3, 6, 12, 24];
const SYNC_HOURS_DEFAULT: i32 = 3;
const NAV_TABS_MAX: usize = 5;
const NAV_TABS_MORE: &str = "more";
const WALLPAPER: &str = "wallpaper";

/// Every SharedPreferences key (in `"settings"`) owned by [`SettingsStore`], for post-import cleanup.
const BASE_PREF_KEYS: [&str; 31] = [
    "settings_show_skip_advice",
    "settings_calculated_insights",
    "settings_storage_note_seen",
    "settings_otp_source",
    "settings_haptics",
    "settings_auto_refresh",
    "settings_show_background_errors",
    "settings_timetable_view_mode",
    "settings_theme_mode",
    "settings_colors",
    "settings_dynamic_color",
    "settings_pure_black",
    "settings_display_size",
    "settings_avatar_path",
    "settings_nav_tabs",
    "dev_always_show",
    "dev_inspect_traffic",
    "dev_http_engine",
    "settings_background_sync",
    "settings_live_updates",
    "settings_class_mode",
    "settings_assistant",
    "settings_calendar_sync",
    "settings_onboarded",
    "settings_onboarding_pending",
    "settings_student",
    "settings_wrapped_seen",
    "settings_wrapped_hide_grades",
    "settings_wrapped_hide_name",
    "settings_wrapped_hide_hostel",
    "settings_sync_hours",
];

const EXTRA_PREF_KEYS: [&str; 3] = [
    "settings_sync_paused",
    "settings_sync_otp_misses",
    "settings_sync_course",
];

const ALL_CHANGE_KINDS: [ChangeKind; 8] = [
    ChangeKind::Attendance,
    ChangeKind::Marks,
    ChangeKind::Timetable,
    ChangeKind::Exams,
    ChangeKind::Grades,
    ChangeKind::Outing,
    ChangeKind::Files,
    ChangeKind::Calendar,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum TimetableViewMode {
    Agenda,
    Weekly,
}

/// Where the OTP prompt reads VTOP's code from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum OtpSource {
    Forwarding,
    Gmail,
}

/// A number tuned in Settings → Developer (the Flutter app's `Tunable`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, uniffi::Enum,
)]
pub enum TunableKey {
    PageMaxAge,
    JustFetched,
    CalendarMaxAge,
    SweepMaxAge,
    SweepCooldown,
    SweepQuick,
    SlowPage,
    FetchLanes,
    SlowFetch,
    QueueDrop,
    VtopTimeout,
    SessionReuse,
    OtpPoll,
    SessionAnswer,
    CookieAsk,
    SyncBiometricDays,
    CampusFetchGap,
    CampusFetchAtOnce,
    WrappedRefetch,
    ProofTimeout,
    ProofOnCellular,
}

struct TunableSpec {
    key: TunableKey,
    pref_key: &'static str,
    title: &'static str,
    unit: &'static str,
    default: i32,
    min: i32,
    max: i32,
    note: &'static str,
}

const TUNABLE_SPECS: [TunableSpec; 21] = [
    TunableSpec {
        key: TunableKey::PageMaxAge,
        pref_key: "dev_page_max_age_h",
        title: "Page max age",
        unit: "h",
        default: 6,
        min: 1,
        max: 168,
        note: "Older pages fetch again on open and on return",
    },
    TunableSpec {
        key: TunableKey::JustFetched,
        pref_key: "dev_just_fetched_min",
        title: "Just fetched",
        unit: "min",
        default: 10,
        min: 0,
        max: 120,
        note: "A page that finished this recently while away isn't fetched again",
    },
    TunableSpec {
        key: TunableKey::CalendarMaxAge,
        pref_key: "dev_calendar_max_age_h",
        title: "Calendar max age",
        unit: "h",
        default: 24,
        min: 1,
        max: 720,
        note: "The academic calendar fetches again when this old",
    },
    TunableSpec {
        key: TunableKey::SweepMaxAge,
        pref_key: "dev_sweep_max_age_h",
        title: "Background refresh age",
        unit: "h",
        default: 12,
        min: 1,
        max: 168,
        note: "Pages not on screen this old refresh in the open-app sweep",
    },
    TunableSpec {
        key: TunableKey::SweepCooldown,
        pref_key: "dev_sweep_cooldown_min",
        title: "Background refresh gap",
        unit: "min",
        default: 15,
        min: 0,
        max: 1440,
        note: "At most one sweep in this long",
    },
    TunableSpec {
        key: TunableKey::SweepQuick,
        pref_key: "dev_sweep_quick_s",
        title: "Quick fetch",
        unit: "s",
        default: 2,
        min: 1,
        max: 60,
        note: "A sweep starts only after a page fetch this fast",
    },
    TunableSpec {
        key: TunableKey::SlowPage,
        pref_key: "dev_slow_page_s",
        title: "Background page timeout",
        unit: "s",
        default: 5,
        min: 1,
        max: 120,
        note: "A slower page stops the sweep",
    },
    TunableSpec {
        key: TunableKey::FetchLanes,
        pref_key: "dev_fetch_lanes",
        title: "Pages at once",
        unit: "pages",
        default: 2,
        min: 1,
        max: 4,
        note: "Page fetches that run at the same time; the rest wait, the page on screen first",
    },
    TunableSpec {
        key: TunableKey::SlowFetch,
        pref_key: "dev_slow_fetch_s",
        title: "Slow link",
        unit: "s",
        default: 4,
        min: 1,
        max: 60,
        note: "When recent page fetches take this long, one runs at a time and the sweep waits",
    },
    TunableSpec {
        key: TunableKey::QueueDrop,
        pref_key: "dev_queue_drop_s",
        title: "Waiting fetch limit",
        unit: "s",
        default: 60,
        min: 5,
        max: 600,
        note: "An automatic fetch for a page left behind is dropped after waiting this long",
    },
    TunableSpec {
        key: TunableKey::VtopTimeout,
        pref_key: "dev_vtop_timeout_s",
        title: "VTOP request timeout",
        unit: "s",
        default: 30,
        min: 5,
        max: 180,
        note: "One VTOP request, from the next launch",
    },
    TunableSpec {
        key: TunableKey::SessionReuse,
        pref_key: "dev_session_reuse_min",
        title: "VTOP session reuse",
        unit: "min",
        default: 0,
        min: 0,
        max: 10_080,
        note: "A saved session older than this signs in again at launch; 0: any age",
    },
    TunableSpec {
        key: TunableKey::OtpPoll,
        pref_key: "dev_otp_poll_s",
        title: "OTP check without the live socket",
        unit: "s",
        default: 2,
        min: 1,
        max: 30,
        note: "How often the OTP prompt checks Gmail, or forwarded mail when the server can't push it",
    },
    TunableSpec {
        key: TunableKey::SessionAnswer,
        pref_key: "dev_session_answer_s",
        title: "Session request timeout",
        unit: "s",
        default: 15,
        min: 3,
        max: 60,
        note: "How long the phone tries to answer a browser's or AI app's session request",
    },
    TunableSpec {
        key: TunableKey::CookieAsk,
        pref_key: "dev_cookie_ask_s",
        title: "Cookie consent wait",
        unit: "s",
        default: 60,
        min: 10,
        max: 300,
        note: "How long the 'send your VTOP cookie?' dialog waits before it counts as Cancel",
    },
    TunableSpec {
        key: TunableKey::SyncBiometricDays,
        pref_key: "dev_sync_bio_days",
        title: "Biometric days per sync",
        unit: "days",
        default: 3,
        min: 0,
        max: 7,
        note: "Recent days not final in the cache, newest first; 0 turns it off",
    },
    TunableSpec {
        key: TunableKey::CampusFetchGap,
        pref_key: "dev_campus_gap_s",
        title: "Campus fetch gap",
        unit: "s",
        default: 3,
        min: 0,
        max: 30,
        note: "Pause between biometric days in the Wrapped fetch",
    },
    TunableSpec {
        key: TunableKey::CampusFetchAtOnce,
        pref_key: "dev_campus_at_once",
        title: "Campus fetch at once",
        unit: "days",
        default: 2,
        min: 1,
        max: 3,
        note: "Biometric days the Wrapped fetch asks VTOP for at the same time",
    },
    TunableSpec {
        key: TunableKey::WrappedRefetch,
        pref_key: "dev_wrapped_refetch_h",
        title: "Wrapped refetch age",
        unit: "h",
        default: 12,
        min: 1,
        max: 168,
        note: "Pages with nothing posted yet fetch again for Wrapped when this old",
    },
    TunableSpec {
        key: TunableKey::ProofTimeout,
        pref_key: "dev_proof_bg_timeout_s",
        title: "Background proof timeout",
        unit: "s",
        default: 90,
        min: 20,
        max: 300,
        note: "The account proof after sign-in gives up after this long and tries again at the next open",
    },
    TunableSpec {
        key: TunableKey::ProofOnCellular,
        pref_key: "dev_proof_cellular",
        title: "Background proof on mobile data",
        unit: "on",
        default: 0,
        min: 0,
        max: 1,
        note: "1: the account proof after sign-in (about 1.5 MB) may run on mobile data too",
    },
];

impl TunableKey {
    fn spec(self) -> &'static TunableSpec {
        match self {
            Self::PageMaxAge => &TUNABLE_SPECS[0],
            Self::JustFetched => &TUNABLE_SPECS[1],
            Self::CalendarMaxAge => &TUNABLE_SPECS[2],
            Self::SweepMaxAge => &TUNABLE_SPECS[3],
            Self::SweepCooldown => &TUNABLE_SPECS[4],
            Self::SweepQuick => &TUNABLE_SPECS[5],
            Self::SlowPage => &TUNABLE_SPECS[6],
            Self::FetchLanes => &TUNABLE_SPECS[7],
            Self::SlowFetch => &TUNABLE_SPECS[8],
            Self::QueueDrop => &TUNABLE_SPECS[9],
            Self::VtopTimeout => &TUNABLE_SPECS[10],
            Self::SessionReuse => &TUNABLE_SPECS[11],
            Self::OtpPoll => &TUNABLE_SPECS[12],
            Self::SessionAnswer => &TUNABLE_SPECS[13],
            Self::CookieAsk => &TUNABLE_SPECS[14],
            Self::SyncBiometricDays => &TUNABLE_SPECS[15],
            Self::CampusFetchGap => &TUNABLE_SPECS[16],
            Self::CampusFetchAtOnce => &TUNABLE_SPECS[17],
            Self::WrappedRefetch => &TUNABLE_SPECS[18],
            Self::ProofTimeout => &TUNABLE_SPECS[19],
            Self::ProofOnCellular => &TUNABLE_SPECS[20],
        }
    }

    fn from_pref_key(pref_key: &str) -> Option<Self> {
        TUNABLE_SPECS
            .iter()
            .find(|spec| spec.pref_key == pref_key)
            .map(|spec| spec.key)
    }
}

/// Metadata of one tunable, for Settings → Developer.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TunableInfo {
    pub key: TunableKey,
    pub pref_key: String,
    pub title: String,
    pub unit: String,
    pub default: i32,
    pub min: i32,
    pub max: i32,
    pub note: String,
}

/// Every tunable in Developer screen order.
#[uniffi::export]
pub fn tunables() -> Vec<TunableInfo> {
    TUNABLE_SPECS
        .iter()
        .map(|spec| TunableInfo {
            key: spec.key,
            pref_key: spec.pref_key.to_owned(),
            title: spec.title.to_owned(),
            unit: spec.unit.to_owned(),
            default: spec.default,
            min: spec.min,
            max: spec.max,
            note: spec.note.to_owned(),
        })
        .collect()
}

/// Every key in Android's `"settings"` SharedPreferences that [`SettingsStore::import`] consumes,
/// so Kotlin can remove only those keys after import and keep keys owned by other screens.
#[uniffi::export]
pub fn settings_pref_keys() -> Vec<String> {
    let mut keys: Vec<String> = BASE_PREF_KEYS
        .iter()
        .chain(EXTRA_PREF_KEYS.iter())
        .map(|k| (*k).to_owned())
        .collect();
    for spec in &TUNABLE_SPECS {
        keys.push(spec.pref_key.to_owned());
    }
    for kind in ALL_CHANGE_KINDS {
        keys.push(notify_pref_key(kind).to_owned());
    }
    keys
}

/// One typed value from Android's SharedPreferences, for one-time import.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum PrefValue {
    Bool { value: bool },
    Int { value: i32 },
    Long { value: i64 },
    Float { value: f32 },
    Str { value: String },
    StrSet { values: Vec<String> },
}

/// Snapshot of every app setting.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Settings {
    pub show_skip_advice: bool,
    pub calculated_insights: bool,
    pub storage_note_seen: bool,
    pub otp_source: Option<OtpSource>,
    pub haptics: bool,
    pub auto_refresh: bool,
    pub show_background_errors: bool,
    pub timetable_view: TimetableViewMode,
    pub theme_mode: ThemeMode,
    pub color_choice: Option<String>,
    pub pure_black: bool,
    pub display_size: i32,
    pub avatar_path: Option<String>,
    pub nav_tabs: Vec<String>,
    pub developer_enabled: bool,
    pub inspect_traffic: bool,
    pub http_engine: bool,
    pub background_sync: bool,
    pub live_updates: bool,
    pub class_mode: bool,
    pub assistant: bool,
    pub calendar_sync: bool,
    pub onboarded: bool,
    pub onboarding_pending: bool,
    pub student: Option<String>,
    pub wrapped_seen: Vec<String>,
    pub wrapped_hide_grades: Vec<String>,
    pub wrapped_hide_name: bool,
    pub wrapped_hide_hostel: bool,
    pub sync_hours: i32,
    pub sync_paused: Option<String>,
    pub sync_otp_misses: i32,
    pub sync_course: Option<String>,
    pub notify_off: Vec<ChangeKind>,
    pub tuned: HashMap<String, i32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_skip_advice: false,
            calculated_insights: false,
            storage_note_seen: false,
            otp_source: None,
            haptics: true,
            auto_refresh: true,
            show_background_errors: false,
            timetable_view: TimetableViewMode::Agenda,
            theme_mode: ThemeMode::System,
            color_choice: None,
            pure_black: false,
            display_size: DISPLAY_SIZE_DEFAULT,
            avatar_path: None,
            nav_tabs: default_nav_tabs(),
            developer_enabled: false,
            inspect_traffic: false,
            http_engine: true,
            background_sync: false,
            live_updates: false,
            class_mode: false,
            assistant: false,
            calendar_sync: false,
            onboarded: false,
            onboarding_pending: false,
            student: None,
            wrapped_seen: Vec::new(),
            wrapped_hide_grades: Vec::new(),
            wrapped_hide_name: false,
            wrapped_hide_hostel: false,
            sync_hours: SYNC_HOURS_DEFAULT,
            sync_paused: None,
            sync_otp_misses: 0,
            sync_course: None,
            notify_off: Vec::new(),
            tuned: HashMap::new(),
        }
    }
}

impl Settings {
    /// Current value of `key`, in its unit, clamped to its valid range.
    pub fn tunable(&self, key: TunableKey) -> i32 {
        let spec = key.spec();
        self.tuned
            .get(spec.pref_key)
            .copied()
            .unwrap_or(spec.default)
            .clamp(spec.min, spec.max)
    }

    fn apply(&mut self, change: SettingChange) {
        match change {
            SettingChange::ShowSkipAdvice { value } => self.show_skip_advice = value,
            SettingChange::CalculatedInsights { value } => self.calculated_insights = value,
            SettingChange::StorageNoteSeen { value } => self.storage_note_seen = value,
            SettingChange::OtpSource { value } => self.otp_source = value,
            SettingChange::Haptics { value } => self.haptics = value,
            SettingChange::AutoRefresh { value } => self.auto_refresh = value,
            SettingChange::ShowBackgroundErrors { value } => self.show_background_errors = value,
            SettingChange::TimetableView { value } => self.timetable_view = value,
            SettingChange::ThemeMode { value } => self.theme_mode = value,
            SettingChange::ColorChoice { value } => self.color_choice = value,
            SettingChange::PureBlack { value } => self.pure_black = value,
            SettingChange::DisplaySize { value } => {
                self.display_size = valid_display_size(value);
            }
            SettingChange::AvatarPath { value } => self.avatar_path = value,
            SettingChange::NavTabs { ids } => self.nav_tabs = normalize_nav_tabs(ids),
            SettingChange::DeveloperEnabled { value } => self.developer_enabled = value,
            SettingChange::InspectTraffic { value } => self.inspect_traffic = value,
            SettingChange::HttpEngine { value } => self.http_engine = value,
            SettingChange::BackgroundSync { value } => self.background_sync = value,
            SettingChange::LiveUpdates { value } => self.live_updates = value,
            SettingChange::ClassMode { value } => self.class_mode = value,
            SettingChange::Assistant { value } => self.assistant = value,
            SettingChange::CalendarSync { value } => self.calendar_sync = value,
            SettingChange::Onboarded { value } => self.onboarded = value,
            SettingChange::OnboardingPending { value } => self.onboarding_pending = value,
            SettingChange::Student { value } => self.student = value,
            SettingChange::WrappedSeen { ids } => self.wrapped_seen = dedup_sorted(ids),
            SettingChange::WrappedHideGrades { ids } => {
                self.wrapped_hide_grades = dedup_sorted(ids);
            }
            SettingChange::WrappedHideName { value } => self.wrapped_hide_name = value,
            SettingChange::WrappedHideHostel { value } => self.wrapped_hide_hostel = value,
            SettingChange::SyncHours { value } => self.sync_hours = valid_sync_hours(value),
            SettingChange::SyncHealth { paused, otp_misses } => {
                let paused = paused.filter(|s| !s.is_empty());
                self.sync_otp_misses = if paused.is_some() {
                    0
                } else {
                    otp_misses.max(0)
                };
                self.sync_paused = paused;
            }
            SettingChange::SyncCourse { value } => self.sync_course = value,
            SettingChange::Notifies { kind, on } => {
                if on {
                    self.notify_off.retain(|&k| k != kind);
                } else if !self.notify_off.contains(&kind) {
                    self.notify_off.push(kind);
                    self.notify_off.sort();
                }
            }
            SettingChange::Tunable { key, value } => {
                let spec = key.spec();
                match value.map(|v| v.clamp(spec.min, spec.max)) {
                    Some(v) if v != spec.default => {
                        self.tuned.insert(spec.pref_key.to_owned(), v);
                    }
                    _ => {
                        self.tuned.remove(spec.pref_key);
                    }
                }
            }
            SettingChange::ForgetStudent => {
                self.otp_source = None;
                self.background_sync = false;
                self.sync_paused = None;
                self.sync_otp_misses = 0;
                self.sync_course = None;
                self.wrapped_seen.clear();
                self.wrapped_hide_grades.clear();
                self.wrapped_hide_name = false;
                self.wrapped_hide_hostel = false;
                self.storage_note_seen = false;
                self.student = None;
            }
        }
    }
}

/// One mutation applied by [`SettingsStore::set`].
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SettingChange {
    ShowSkipAdvice {
        value: bool,
    },
    CalculatedInsights {
        value: bool,
    },
    StorageNoteSeen {
        value: bool,
    },
    OtpSource {
        value: Option<OtpSource>,
    },
    Haptics {
        value: bool,
    },
    AutoRefresh {
        value: bool,
    },
    ShowBackgroundErrors {
        value: bool,
    },
    TimetableView {
        value: TimetableViewMode,
    },
    ThemeMode {
        value: ThemeMode,
    },
    ColorChoice {
        value: Option<String>,
    },
    PureBlack {
        value: bool,
    },
    DisplaySize {
        value: i32,
    },
    AvatarPath {
        value: Option<String>,
    },
    NavTabs {
        ids: Vec<String>,
    },
    DeveloperEnabled {
        value: bool,
    },
    InspectTraffic {
        value: bool,
    },
    HttpEngine {
        value: bool,
    },
    BackgroundSync {
        value: bool,
    },
    LiveUpdates {
        value: bool,
    },
    ClassMode {
        value: bool,
    },
    Assistant {
        value: bool,
    },
    CalendarSync {
        value: bool,
    },
    Onboarded {
        value: bool,
    },
    OnboardingPending {
        value: bool,
    },
    Student {
        value: Option<String>,
    },
    WrappedSeen {
        ids: Vec<String>,
    },
    WrappedHideGrades {
        ids: Vec<String>,
    },
    WrappedHideName {
        value: bool,
    },
    WrappedHideHostel {
        value: bool,
    },
    SyncHours {
        value: i32,
    },
    SyncHealth {
        paused: Option<String>,
        otp_misses: i32,
    },
    SyncCourse {
        value: Option<String>,
    },
    Notifies {
        kind: ChangeKind,
        on: bool,
    },
    Tunable {
        key: TunableKey,
        value: Option<i32>,
    },
    ForgetStudent,
}

/// Hears every persisted change from [`SettingsStore`].
#[uniffi::export(with_foreign)]
pub trait SettingsListener: Send + Sync {
    fn on_settings(&self, settings: Settings);
}

/// On-disk JSON shape of `settings.json`, with explicit defaults for every field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct StoredSettings {
    show_skip_advice: bool,
    calculated_insights: bool,
    storage_note_seen: bool,
    otp_source: Option<OtpSource>,
    haptics: bool,
    auto_refresh: bool,
    show_background_errors: bool,
    timetable_view: TimetableViewMode,
    theme_mode: ThemeMode,
    color_choice: Option<String>,
    pure_black: bool,
    display_size: i32,
    avatar_path: Option<String>,
    nav_tabs: Vec<String>,
    developer_enabled: bool,
    inspect_traffic: bool,
    http_engine: bool,
    background_sync: bool,
    live_updates: bool,
    class_mode: bool,
    assistant: bool,
    calendar_sync: bool,
    onboarded: bool,
    onboarding_pending: bool,
    student: Option<String>,
    wrapped_seen: Vec<String>,
    wrapped_hide_grades: Vec<String>,
    wrapped_hide_name: bool,
    wrapped_hide_hostel: bool,
    sync_hours: i32,
    sync_paused: Option<String>,
    sync_otp_misses: i32,
    sync_course: Option<String>,
    notify_off: Vec<ChangeKind>,
    tuned: BTreeMap<String, i32>,
}

impl Default for StoredSettings {
    fn default() -> Self {
        Self::from(&Settings::default())
    }
}

impl From<&Settings> for StoredSettings {
    fn from(s: &Settings) -> Self {
        let mut notify_off = s.notify_off.clone();
        notify_off.sort();
        notify_off.dedup();
        let tuned = s
            .tuned
            .iter()
            .filter_map(|(k, &v)| {
                let key = TunableKey::from_pref_key(k)?;
                let spec = key.spec();
                let clamped = v.clamp(spec.min, spec.max);
                (clamped != spec.default).then(|| (spec.pref_key.to_owned(), clamped))
            })
            .collect();
        Self {
            show_skip_advice: s.show_skip_advice,
            calculated_insights: s.calculated_insights,
            storage_note_seen: s.storage_note_seen,
            otp_source: s.otp_source,
            haptics: s.haptics,
            auto_refresh: s.auto_refresh,
            show_background_errors: s.show_background_errors,
            timetable_view: s.timetable_view,
            theme_mode: s.theme_mode,
            color_choice: s.color_choice.clone(),
            pure_black: s.pure_black,
            display_size: valid_display_size(s.display_size),
            avatar_path: s.avatar_path.clone(),
            nav_tabs: normalize_nav_tabs(s.nav_tabs.clone()),
            developer_enabled: s.developer_enabled,
            inspect_traffic: s.inspect_traffic,
            http_engine: s.http_engine,
            background_sync: s.background_sync,
            live_updates: s.live_updates,
            class_mode: s.class_mode,
            assistant: s.assistant,
            calendar_sync: s.calendar_sync,
            onboarded: s.onboarded,
            onboarding_pending: s.onboarding_pending,
            student: s.student.clone(),
            wrapped_seen: dedup_sorted(s.wrapped_seen.clone()),
            wrapped_hide_grades: dedup_sorted(s.wrapped_hide_grades.clone()),
            wrapped_hide_name: s.wrapped_hide_name,
            wrapped_hide_hostel: s.wrapped_hide_hostel,
            sync_hours: valid_sync_hours(s.sync_hours),
            sync_paused: s.sync_paused.clone().filter(|p| !p.is_empty()),
            sync_otp_misses: if s.sync_paused.as_deref().is_some_and(|p| !p.is_empty()) {
                0
            } else {
                s.sync_otp_misses.max(0)
            },
            sync_course: s.sync_course.clone(),
            notify_off,
            tuned,
        }
    }
}

impl From<StoredSettings> for Settings {
    fn from(stored: StoredSettings) -> Self {
        let sync_paused = stored.sync_paused.filter(|p| !p.is_empty());
        let sync_otp_misses = if sync_paused.is_some() {
            0
        } else {
            stored.sync_otp_misses.max(0)
        };
        let mut notify_off = stored.notify_off;
        notify_off.sort();
        notify_off.dedup();
        let tuned = stored
            .tuned
            .into_iter()
            .filter_map(|(k, v)| {
                let key = TunableKey::from_pref_key(&k)?;
                let spec = key.spec();
                let clamped = v.clamp(spec.min, spec.max);
                (clamped != spec.default).then(|| (spec.pref_key.to_owned(), clamped))
            })
            .collect();
        Self {
            show_skip_advice: stored.show_skip_advice,
            calculated_insights: stored.calculated_insights,
            storage_note_seen: stored.storage_note_seen,
            otp_source: stored.otp_source,
            haptics: stored.haptics,
            auto_refresh: stored.auto_refresh,
            show_background_errors: stored.show_background_errors,
            timetable_view: stored.timetable_view,
            theme_mode: stored.theme_mode,
            color_choice: stored.color_choice,
            pure_black: stored.pure_black,
            display_size: valid_display_size(stored.display_size),
            avatar_path: stored.avatar_path,
            nav_tabs: normalize_nav_tabs(stored.nav_tabs),
            developer_enabled: stored.developer_enabled,
            inspect_traffic: stored.inspect_traffic,
            http_engine: stored.http_engine,
            background_sync: stored.background_sync,
            live_updates: stored.live_updates,
            class_mode: stored.class_mode,
            assistant: stored.assistant,
            calendar_sync: stored.calendar_sync,
            onboarded: stored.onboarded,
            onboarding_pending: stored.onboarding_pending,
            student: stored.student,
            wrapped_seen: dedup_sorted(stored.wrapped_seen),
            wrapped_hide_grades: dedup_sorted(stored.wrapped_hide_grades),
            wrapped_hide_name: stored.wrapped_hide_name,
            wrapped_hide_hostel: stored.wrapped_hide_hostel,
            sync_hours: valid_sync_hours(stored.sync_hours),
            sync_paused,
            sync_otp_misses,
            sync_course: stored.sync_course,
            notify_off,
            tuned,
        }
    }
}

struct State {
    current: Settings,
    has_file: bool,
    listener: Option<Arc<dyn SettingsListener>>,
}

/// Process-wide cache of live stores keyed by directory, so `Settings.load` and `Core::open`
/// share one in-memory store for the same files directory while tests drop and reopen cleanly.
fn live_stores() -> &'static Mutex<HashMap<PathBuf, Weak<SettingsStore>>> {
    static STORES: OnceLock<Mutex<HashMap<PathBuf, Weak<SettingsStore>>>> = OnceLock::new();
    STORES.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(uniffi::Object)]
pub struct SettingsStore {
    dir: PathBuf,
    path: PathBuf,
    state: Mutex<State>,
}

#[uniffi::export]
impl SettingsStore {
    /// Opens `<dir>/settings.json` synchronously; starts at defaults if the file is missing.
    #[uniffi::constructor]
    pub fn open(dir: String) -> Arc<Self> {
        let dir = PathBuf::from(dir);
        let mut stores = live_stores().lock().unwrap_or_else(PoisonError::into_inner);
        stores.retain(|_, weak| weak.strong_count() > 0);
        if let Some(existing) = stores.get(&dir).and_then(Weak::upgrade) {
            return existing;
        }
        let path = dir.join(FILE);
        let (current, has_file) = if path.is_file() {
            match fs::read(&path) {
                Ok(bytes) => match serde_json::from_slice::<StoredSettings>(&bytes) {
                    Ok(stored) => (Settings::from(stored), true),
                    Err(error) => {
                        warn!(file = FILE, %error, "unreadable");
                        (Settings::default(), true)
                    }
                },
                Err(error) => {
                    warn!(file = FILE, %error, "unreadable");
                    (Settings::default(), false)
                }
            }
        } else {
            (Settings::default(), false)
        };
        let store = Arc::new(Self {
            dir: dir.clone(),
            path,
            state: Mutex::new(State {
                current,
                has_file,
                listener: None,
            }),
        });
        stores.insert(dir, Arc::downgrade(&store));
        store
    }

    pub fn get(&self) -> Settings {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .current
            .clone()
    }

    /// True while `<dir>/settings.json` has not been written yet (one-time prefs import needed).
    pub fn needs_import(&self) -> bool {
        !self
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .has_file
    }

    pub fn set_listener(&self, listener: Arc<dyn SettingsListener>) {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .listener = Some(listener);
    }

    /// Applies `change`, persists `settings.json` atomically, notifies the listener and returns
    /// the updated snapshot.
    pub fn set(&self, change: SettingChange) -> Settings {
        let (snapshot, listener) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.current.apply(change);
            let snapshot = state.current.clone();
            if let Err(error) = self.persist(&snapshot) {
                warn!(file = FILE, %error, "not saved");
            } else {
                state.has_file = true;
            }
            (snapshot, state.listener.clone())
        };
        notify(listener.as_deref(), snapshot.clone());
        snapshot
    }

    /// Parses Flutter-era SharedPreferences values, writes `settings.json` atomically, and only
    /// updates in-memory state if the disk write succeeds.
    pub fn import(&self, prefs: HashMap<String, PrefValue>) -> Result<Settings, CoreError> {
        let imported = from_prefs(&prefs);
        let (snapshot, listener) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            self.persist(&imported)?;
            state.current = imported.clone();
            state.has_file = true;
            (imported, state.listener.clone())
        };
        notify(listener.as_deref(), snapshot.clone());
        Ok(snapshot)
    }
}

impl SettingsStore {
    fn persist(&self, settings: &Settings) -> Result<(), CoreError> {
        fs::create_dir_all(&self.dir)?;
        let stored = StoredSettings::from(settings);
        let bytes = serde_json::to_vec(&stored).map_err(|error| CoreError::Other {
            reason: error.to_string(),
        })?;
        write_atomic(&self.path, &bytes)?;
        Ok(())
    }
}

fn notify(listener: Option<&dyn SettingsListener>, settings: Settings) {
    let Some(listener) = listener else {
        return;
    };
    if catch_unwind(AssertUnwindSafe(|| listener.on_settings(settings))).is_err() {
        warn!("settings listener panicked");
    }
}

fn from_prefs(prefs: &HashMap<String, PrefValue>) -> Settings {
    let mut s = Settings::default();

    let bool_of = |key: &str| match prefs.get(key) {
        Some(PrefValue::Bool { value }) => Some(*value),
        _ => None,
    };
    let int_of = |key: &str| match prefs.get(key) {
        Some(PrefValue::Int { value }) => Some(*value),
        _ => None,
    };
    let str_of = |key: &str| match prefs.get(key) {
        Some(PrefValue::Str { value }) => Some(value.as_str()),
        _ => None,
    };
    let str_set_of = |key: &str| match prefs.get(key) {
        Some(PrefValue::StrSet { values }) => Some(dedup_sorted(values.clone())),
        _ => None,
    };

    if let Some(v) = bool_of("settings_show_skip_advice") {
        s.show_skip_advice = v;
    }
    if let Some(v) = bool_of("settings_calculated_insights") {
        s.calculated_insights = v;
    }
    if let Some(v) = bool_of("settings_storage_note_seen") {
        s.storage_note_seen = v;
    }
    if let Some(raw) = str_of("settings_otp_source") {
        s.otp_source = match raw {
            "forwarding" => Some(OtpSource::Forwarding),
            "gmail" => Some(OtpSource::Gmail),
            _ => None,
        };
    }
    if let Some(v) = bool_of("settings_haptics") {
        s.haptics = v;
    }
    if let Some(v) = bool_of("settings_auto_refresh") {
        s.auto_refresh = v;
    }
    if let Some(v) = bool_of("settings_show_background_errors") {
        s.show_background_errors = v;
    }
    if let Some(raw) = str_of("settings_timetable_view_mode") {
        s.timetable_view = match raw {
            "agenda" => TimetableViewMode::Agenda,
            "weekly" => TimetableViewMode::Weekly,
            _ => TimetableViewMode::Agenda,
        };
    }
    if let Some(raw) = str_of("settings_theme_mode") {
        s.theme_mode = match raw {
            "system" => ThemeMode::System,
            "light" => ThemeMode::Light,
            "dark" => ThemeMode::Dark,
            _ => ThemeMode::System,
        };
    }
    // The old wallpaper switch carries over when settings_colors wasn't set.
    s.color_choice = str_of("settings_colors").map(str::to_owned).or_else(|| {
        bool_of("settings_dynamic_color")
            .filter(|&on| on)
            .map(|_| WALLPAPER.to_owned())
    });
    if let Some(v) = bool_of("settings_pure_black") {
        s.pure_black = v;
    }
    if let Some(v) = int_of("settings_display_size") {
        s.display_size = valid_display_size(v);
    }
    s.avatar_path = str_of("settings_avatar_path").map(str::to_owned);
    if let Some(raw) = str_of("settings_nav_tabs") {
        s.nav_tabs = normalize_nav_tabs(raw.split(',').map(str::to_owned));
    }
    if let Some(v) = bool_of("dev_always_show") {
        s.developer_enabled = v;
    }
    if let Some(v) = bool_of("dev_inspect_traffic") {
        s.inspect_traffic = v;
    }
    if let Some(v) = bool_of("dev_http_engine") {
        s.http_engine = v;
    }
    if let Some(v) = bool_of("settings_background_sync") {
        s.background_sync = v;
    }
    if let Some(v) = bool_of("settings_live_updates") {
        s.live_updates = v;
    }
    if let Some(v) = bool_of("settings_class_mode") {
        s.class_mode = v;
    }
    if let Some(v) = bool_of("settings_assistant") {
        s.assistant = v;
    }
    if let Some(v) = bool_of("settings_calendar_sync") {
        s.calendar_sync = v;
    }
    if let Some(v) = bool_of("settings_onboarded") {
        s.onboarded = v;
    }
    if let Some(v) = bool_of("settings_onboarding_pending") {
        s.onboarding_pending = v;
    }
    s.student = str_of("settings_student").map(str::to_owned);
    if let Some(v) = str_set_of("settings_wrapped_seen") {
        s.wrapped_seen = v;
    }
    if let Some(v) = str_set_of("settings_wrapped_hide_grades") {
        s.wrapped_hide_grades = v;
    }
    if let Some(v) = bool_of("settings_wrapped_hide_name") {
        s.wrapped_hide_name = v;
    }
    if let Some(v) = bool_of("settings_wrapped_hide_hostel") {
        s.wrapped_hide_hostel = v;
    }
    if let Some(v) = int_of("settings_sync_hours") {
        s.sync_hours = valid_sync_hours(v);
    }
    let paused = str_of("settings_sync_paused")
        .filter(|p| !p.is_empty())
        .map(str::to_owned);
    let misses = int_of("settings_sync_otp_misses").unwrap_or(0).max(0);
    s.sync_otp_misses = if paused.is_some() { 0 } else { misses };
    s.sync_paused = paused;
    s.sync_course = str_of("settings_sync_course").map(str::to_owned);

    for kind in ALL_CHANGE_KINDS {
        if bool_of(notify_pref_key(kind)) == Some(false) {
            s.notify_off.push(kind);
        }
    }
    s.notify_off.sort();

    for spec in &TUNABLE_SPECS {
        if let Some(v) = int_of(spec.pref_key) {
            let clamped = v.clamp(spec.min, spec.max);
            if clamped != spec.default {
                s.tuned.insert(spec.pref_key.to_owned(), clamped);
            }
        }
    }

    s
}

fn notify_pref_key(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Attendance => "settings_notify_attendance",
        ChangeKind::Marks => "settings_notify_marks",
        ChangeKind::Timetable => "settings_notify_timetable",
        ChangeKind::Exams => "settings_notify_exams",
        ChangeKind::Grades => "settings_notify_grades",
        ChangeKind::Outing => "settings_notify_outing",
        ChangeKind::Files => "settings_notify_files",
        ChangeKind::Calendar => "settings_notify_calendar",
    }
}

fn default_nav_tabs() -> Vec<String> {
    vec![
        "timetable".to_owned(),
        "attendance".to_owned(),
        NAV_TABS_MORE.to_owned(),
    ]
}

fn normalize_nav_tabs(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut clean = Vec::new();
    for raw in ids {
        let trimmed = raw.trim();
        if trimmed.is_empty() || clean.iter().any(|existing: &String| existing == trimmed) {
            continue;
        }
        clean.push(trimmed.to_owned());
    }
    if clean
        .iter()
        .take(NAV_TABS_MAX)
        .any(|id| id == NAV_TABS_MORE)
    {
        clean.truncate(NAV_TABS_MAX);
        return clean;
    }
    let mut out: Vec<String> = clean
        .into_iter()
        .filter(|id| id != NAV_TABS_MORE)
        .take(NAV_TABS_MAX - 1)
        .collect();
    out.push(NAV_TABS_MORE.to_owned());
    out
}

fn valid_display_size(value: i32) -> i32 {
    if DISPLAY_SIZES.contains(&value) {
        value
    } else {
        DISPLAY_SIZE_DEFAULT
    }
}

fn valid_sync_hours(value: i32) -> i32 {
    if SYNC_HOURS.contains(&value) {
        value
    } else {
        SYNC_HOURS_DEFAULT
    }
}

fn dedup_sorted(items: Vec<String>) -> Vec<String> {
    items
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests;

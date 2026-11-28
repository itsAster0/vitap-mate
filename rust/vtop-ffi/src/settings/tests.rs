use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::*;
use crate::pages::changes::ChangeKind;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-settings-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn defaults_match_kotlin() {
    let s = Settings::default();
    assert!(!s.show_skip_advice);
    assert!(!s.calculated_insights);
    assert!(!s.storage_note_seen);
    assert_eq!(s.otp_source, None);
    assert!(s.haptics);
    assert!(s.auto_refresh);
    assert!(!s.show_background_errors);
    assert_eq!(s.timetable_view, TimetableViewMode::Agenda);
    assert_eq!(s.theme_mode, ThemeMode::System);
    assert_eq!(s.color_choice, None);
    assert!(!s.pure_black);
    assert_eq!(s.display_size, 100);
    assert_eq!(s.avatar_path, None);
    assert_eq!(s.nav_tabs, vec!["timetable", "attendance", "more"]);
    assert!(!s.developer_enabled);
    assert!(!s.inspect_traffic);
    assert!(s.http_engine);
    assert!(!s.background_sync);
    assert!(!s.live_updates);
    assert!(!s.class_mode);
    assert!(!s.assistant);
    assert!(!s.calendar_sync);
    assert!(!s.onboarded);
    assert!(!s.onboarding_pending);
    assert_eq!(s.student, None);
    assert!(s.wrapped_seen.is_empty());
    assert!(s.wrapped_hide_grades.is_empty());
    assert!(!s.wrapped_hide_name);
    assert!(!s.wrapped_hide_hostel);
    assert_eq!(s.sync_hours, 3);
    assert_eq!(s.sync_paused, None);
    assert_eq!(s.sync_otp_misses, 0);
    assert_eq!(s.sync_course, None);
    assert!(s.notify_off.is_empty());
    assert!(s.tuned.is_empty());

    type ExpectedTunable = (
        TunableKey,
        &'static str,
        &'static str,
        &'static str,
        i32,
        i32,
        i32,
        &'static str,
    );
    let expected_tunables: &[ExpectedTunable] = &[
        (
            TunableKey::PageMaxAge,
            "dev_page_max_age_h",
            "Page max age",
            "h",
            6,
            1,
            168,
            "Older pages fetch again on open and on return",
        ),
        (
            TunableKey::JustFetched,
            "dev_just_fetched_min",
            "Just fetched",
            "min",
            10,
            0,
            120,
            "A page that finished this recently while away isn't fetched again",
        ),
        (
            TunableKey::CalendarMaxAge,
            "dev_calendar_max_age_h",
            "Calendar max age",
            "h",
            24,
            1,
            720,
            "The academic calendar fetches again when this old",
        ),
        (
            TunableKey::SweepMaxAge,
            "dev_sweep_max_age_h",
            "Background refresh age",
            "h",
            12,
            1,
            168,
            "Pages not on screen this old refresh in the open-app sweep",
        ),
        (
            TunableKey::SweepCooldown,
            "dev_sweep_cooldown_min",
            "Background refresh gap",
            "min",
            15,
            0,
            1440,
            "At most one sweep in this long",
        ),
        (
            TunableKey::SweepQuick,
            "dev_sweep_quick_s",
            "Quick fetch",
            "s",
            2,
            1,
            60,
            "A sweep starts only after a page fetch this fast",
        ),
        (
            TunableKey::SlowPage,
            "dev_slow_page_s",
            "Background page timeout",
            "s",
            5,
            1,
            120,
            "A slower page stops the sweep",
        ),
        (
            TunableKey::FetchLanes,
            "dev_fetch_lanes",
            "Pages at once",
            "pages",
            2,
            1,
            4,
            "Page fetches that run at the same time; the rest wait, the page on screen first",
        ),
        (
            TunableKey::SlowFetch,
            "dev_slow_fetch_s",
            "Slow link",
            "s",
            4,
            1,
            60,
            "When recent page fetches take this long, one runs at a time and the sweep waits",
        ),
        (
            TunableKey::QueueDrop,
            "dev_queue_drop_s",
            "Waiting fetch limit",
            "s",
            60,
            5,
            600,
            "An automatic fetch for a page left behind is dropped after waiting this long",
        ),
        (
            TunableKey::VtopTimeout,
            "dev_vtop_timeout_s",
            "VTOP request timeout",
            "s",
            30,
            5,
            180,
            "One VTOP request, from the next launch",
        ),
        (
            TunableKey::SessionReuse,
            "dev_session_reuse_min",
            "VTOP session reuse",
            "min",
            0,
            0,
            10_080,
            "A saved session older than this signs in again at launch; 0: any age",
        ),
        (
            TunableKey::OtpPoll,
            "dev_otp_poll_s",
            "OTP check without the live socket",
            "s",
            2,
            1,
            30,
            "How often the OTP prompt checks Gmail, or forwarded mail when the server can't push it",
        ),
        (
            TunableKey::SessionAnswer,
            "dev_session_answer_s",
            "Session request timeout",
            "s",
            15,
            3,
            60,
            "How long the phone tries to answer a browser's or AI app's session request",
        ),
        (
            TunableKey::CookieAsk,
            "dev_cookie_ask_s",
            "Cookie consent wait",
            "s",
            60,
            10,
            300,
            "How long the 'send your VTOP cookie?' dialog waits before it counts as Cancel",
        ),
        (
            TunableKey::SyncBiometricDays,
            "dev_sync_bio_days",
            "Biometric days per sync",
            "days",
            3,
            0,
            7,
            "Recent days not final in the cache, newest first; 0 turns it off",
        ),
        (
            TunableKey::CampusFetchGap,
            "dev_campus_gap_s",
            "Campus fetch gap",
            "s",
            3,
            0,
            30,
            "Pause between biometric days in the Wrapped fetch",
        ),
        (
            TunableKey::CampusFetchAtOnce,
            "dev_campus_at_once",
            "Campus fetch at once",
            "days",
            2,
            1,
            3,
            "Biometric days the Wrapped fetch asks VTOP for at the same time",
        ),
        (
            TunableKey::WrappedRefetch,
            "dev_wrapped_refetch_h",
            "Wrapped refetch age",
            "h",
            12,
            1,
            168,
            "Pages with nothing posted yet fetch again for Wrapped when this old",
        ),
        (
            TunableKey::ProofTimeout,
            "dev_proof_bg_timeout_s",
            "Background proof timeout",
            "s",
            90,
            20,
            300,
            "The account proof after sign-in gives up after this long and tries again at the next open",
        ),
        (
            TunableKey::ProofOnCellular,
            "dev_proof_cellular",
            "Background proof on mobile data",
            "on",
            0,
            0,
            1,
            "1: the account proof after sign-in (about 1.5 MB) may run on mobile data too",
        ),
    ];

    let all = tunables();
    assert_eq!(all.len(), expected_tunables.len());
    for (info, &(key, pref_key, title, unit, default, min, max, note)) in
        all.iter().zip(expected_tunables)
    {
        assert_eq!(info.key, key);
        assert_eq!(info.pref_key, pref_key);
        assert_eq!(info.title, title);
        assert_eq!(info.unit, unit);
        assert_eq!(info.default, default);
        assert_eq!(info.min, min);
        assert_eq!(info.max, max);
        assert_eq!(info.note, note);
        assert_eq!(s.tunable(key), default);
    }
}

#[test]
fn import_reads_flutter_keys() {
    let dir = scratch("import-all");
    let store = SettingsStore::open(dir.to_string_lossy().into_owned());
    assert!(store.needs_import());

    let mut prefs = HashMap::from([
        (
            "settings_show_skip_advice".into(),
            PrefValue::Bool { value: true },
        ),
        (
            "settings_calculated_insights".into(),
            PrefValue::Bool { value: true },
        ),
        (
            "settings_storage_note_seen".into(),
            PrefValue::Bool { value: true },
        ),
        (
            "settings_otp_source".into(),
            PrefValue::Str {
                value: "gmail".into(),
            },
        ),
        ("settings_haptics".into(), PrefValue::Bool { value: false }),
        (
            "settings_auto_refresh".into(),
            PrefValue::Bool { value: false },
        ),
        (
            "settings_show_background_errors".into(),
            PrefValue::Bool { value: true },
        ),
        (
            "settings_timetable_view_mode".into(),
            PrefValue::Str {
                value: "weekly".into(),
            },
        ),
        (
            "settings_theme_mode".into(),
            PrefValue::Str {
                value: "dark".into(),
            },
        ),
        (
            "settings_colors".into(),
            PrefValue::Str {
                value: "Ocean".into(),
            },
        ),
        (
            "settings_pure_black".into(),
            PrefValue::Bool { value: true },
        ),
        ("settings_display_size".into(), PrefValue::Int { value: 90 }),
        (
            "settings_avatar_path".into(),
            PrefValue::Str {
                value: "/data/avatar.png".into(),
            },
        ),
        (
            "settings_nav_tabs".into(),
            PrefValue::Str {
                value: "marks,timetable,more".into(),
            },
        ),
        ("dev_always_show".into(), PrefValue::Bool { value: true }),
        (
            "dev_inspect_traffic".into(),
            PrefValue::Bool { value: true },
        ),
        ("dev_http_engine".into(), PrefValue::Bool { value: false }),
        (
            "settings_background_sync".into(),
            PrefValue::Bool { value: true },
        ),
        (
            "settings_live_updates".into(),
            PrefValue::Bool { value: true },
        ),
        (
            "settings_class_mode".into(),
            PrefValue::Bool { value: true },
        ),
        ("settings_assistant".into(), PrefValue::Bool { value: true }),
        (
            "settings_calendar_sync".into(),
            PrefValue::Bool { value: true },
        ),
        ("settings_onboarded".into(), PrefValue::Bool { value: true }),
        (
            "settings_onboarding_pending".into(),
            PrefValue::Bool { value: true },
        ),
        (
            "settings_student".into(),
            PrefValue::Str {
                value: "22BCE0001".into(),
            },
        ),
        (
            "settings_wrapped_seen".into(),
            PrefValue::StrSet {
                values: vec!["AP1".into(), "AP2".into()],
            },
        ),
        (
            "settings_wrapped_hide_grades".into(),
            PrefValue::StrSet {
                values: vec!["AP1".into()],
            },
        ),
        (
            "settings_wrapped_hide_name".into(),
            PrefValue::Bool { value: true },
        ),
        (
            "settings_wrapped_hide_hostel".into(),
            PrefValue::Bool { value: true },
        ),
        ("settings_sync_hours".into(), PrefValue::Int { value: 12 }),
        (
            "settings_sync_paused".into(),
            PrefValue::Str {
                value: "no_code".into(),
            },
        ),
        (
            "settings_sync_otp_misses".into(),
            PrefValue::Int { value: 1 },
        ),
        (
            "settings_sync_course".into(),
            PrefValue::Str {
                value: "CSE1001".into(),
            },
        ),
        (
            "settings_notify_attendance".into(),
            PrefValue::Bool { value: false },
        ),
        (
            "settings_notify_marks".into(),
            PrefValue::Bool { value: true },
        ),
        (
            "settings_notify_exams".into(),
            PrefValue::Bool { value: false },
        ),
        ("dev_page_max_age_h".into(), PrefValue::Int { value: 12 }),
        ("dev_fetch_lanes".into(), PrefValue::Int { value: 4 }),
        ("dev_proof_cellular".into(), PrefValue::Int { value: 1 }),
    ]);

    let s = store.import(prefs.clone()).unwrap();
    assert!(!store.needs_import());
    assert!(s.show_skip_advice);
    assert!(s.calculated_insights);
    assert!(s.storage_note_seen);
    assert_eq!(s.otp_source, Some(OtpSource::Gmail));
    assert!(!s.haptics);
    assert!(!s.auto_refresh);
    assert!(s.show_background_errors);
    assert_eq!(s.timetable_view, TimetableViewMode::Weekly);
    assert_eq!(s.theme_mode, ThemeMode::Dark);
    assert_eq!(s.color_choice.as_deref(), Some("Ocean"));
    assert!(s.pure_black);
    assert_eq!(s.display_size, 90);
    assert_eq!(s.avatar_path.as_deref(), Some("/data/avatar.png"));
    assert_eq!(s.nav_tabs, vec!["marks", "timetable", "more"]);
    assert!(s.developer_enabled);
    assert!(s.inspect_traffic);
    assert!(!s.http_engine);
    assert!(s.background_sync);
    assert!(s.live_updates);
    assert!(s.class_mode);
    assert!(s.assistant);
    assert!(s.calendar_sync);
    assert!(s.onboarded);
    assert!(s.onboarding_pending);
    assert_eq!(s.student.as_deref(), Some("22BCE0001"));
    assert_eq!(s.wrapped_seen, vec!["AP1", "AP2"]);
    assert_eq!(s.wrapped_hide_grades, vec!["AP1"]);
    assert!(s.wrapped_hide_name);
    assert!(s.wrapped_hide_hostel);
    assert_eq!(s.sync_hours, 12);
    assert_eq!(s.sync_paused.as_deref(), Some("no_code"));
    assert_eq!(s.sync_otp_misses, 0);
    assert_eq!(s.sync_course.as_deref(), Some("CSE1001"));
    assert_eq!(
        s.notify_off,
        vec![ChangeKind::Attendance, ChangeKind::Exams]
    );
    assert_eq!(s.tunable(TunableKey::PageMaxAge), 12);
    assert_eq!(s.tunable(TunableKey::FetchLanes), 4);
    assert_eq!(s.tunable(TunableKey::ProofOnCellular), 1);

    // Dynamic color migration: when settings_colors is absent, settings_dynamic_color=true becomes "wallpaper".
    let dir2 = scratch("import-dynamic");
    let store2 = SettingsStore::open(dir2.to_string_lossy().into_owned());
    prefs.remove("settings_colors");
    prefs.insert(
        "settings_dynamic_color".into(),
        PrefValue::Bool { value: true },
    );
    prefs.remove("settings_sync_paused");
    let s2 = store2.import(prefs).unwrap();
    assert_eq!(s2.color_choice.as_deref(), Some("wallpaper"));
    assert_eq!(s2.sync_paused, None);
    assert_eq!(s2.sync_otp_misses, 1);

    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&dir2);
}

#[test]
fn import_ignores_unknown_and_wrong_typed_keys() {
    let dir = scratch("import-wrong");
    let store = SettingsStore::open(dir.to_string_lossy().into_owned());
    let prefs = HashMap::from([
        ("unknown_key".into(), PrefValue::Str { value: "x".into() }),
        ("settings_haptics".into(), PrefValue::Int { value: 0 }),
        (
            "settings_display_size".into(),
            PrefValue::Str { value: "90".into() },
        ),
        ("settings_sync_hours".into(), PrefValue::Int { value: 999 }),
        (
            "settings_timetable_view_mode".into(),
            PrefValue::Str {
                value: "daily".into(),
            },
        ),
        (
            "settings_theme_mode".into(),
            PrefValue::Bool { value: true },
        ),
        (
            "settings_otp_source".into(),
            PrefValue::Str {
                value: "sms".into(),
            },
        ),
        ("dev_fetch_lanes".into(), PrefValue::Float { value: 2.0 }),
        (
            "settings_wrapped_seen".into(),
            PrefValue::Long { value: 42 },
        ),
    ]);
    let s = store.import(prefs).unwrap();
    assert_eq!(s, Settings::default());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn stored_roundtrip() {
    let stored = StoredSettings {
        show_skip_advice: true,
        calculated_insights: true,
        storage_note_seen: true,
        otp_source: Some(OtpSource::Forwarding),
        haptics: false,
        auto_refresh: false,
        show_background_errors: true,
        timetable_view: TimetableViewMode::Weekly,
        theme_mode: ThemeMode::Light,
        color_choice: Some("wallpaper".into()),
        pure_black: true,
        display_size: 110,
        avatar_path: Some("/a/b.jpg".into()),
        nav_tabs: vec!["attendance".into(), "more".into()],
        developer_enabled: true,
        inspect_traffic: true,
        http_engine: false,
        background_sync: true,
        live_updates: true,
        class_mode: true,
        assistant: true,
        calendar_sync: true,
        onboarded: true,
        onboarding_pending: false,
        student: Some("22BCE1234".into()),
        wrapped_seen: vec!["AP1".into()],
        wrapped_hide_grades: vec!["AP2".into()],
        wrapped_hide_name: true,
        wrapped_hide_hostel: true,
        sync_hours: 24,
        sync_paused: Some("gmail_reconnect".into()),
        sync_otp_misses: 0,
        sync_course: Some("MAT1001".into()),
        notify_off: vec![ChangeKind::Calendar],
        tuned: BTreeMap::from([("dev_fetch_lanes".into(), 3)]),
    };
    let json = serde_json::to_string(&stored).unwrap();
    let parsed: StoredSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, stored);

    // An empty JSON object reads back as defaults (forward/backward compatibility).
    let empty: StoredSettings = serde_json::from_str("{}").unwrap();
    assert_eq!(Settings::from(empty), Settings::default());
}

#[test]
fn tunable_clamps_to_range() {
    let dir = scratch("tunable-clamp");
    let store = SettingsStore::open(dir.to_string_lossy().into_owned());
    let s = store.set(SettingChange::Tunable {
        key: TunableKey::FetchLanes,
        value: Some(99),
    });
    assert_eq!(s.tunable(TunableKey::FetchLanes), 4);
    let s = store.set(SettingChange::Tunable {
        key: TunableKey::FetchLanes,
        value: Some(-5),
    });
    assert_eq!(s.tunable(TunableKey::FetchLanes), 1);
    // Setting to default drops the override.
    let s = store.set(SettingChange::Tunable {
        key: TunableKey::FetchLanes,
        value: Some(2),
    });
    assert_eq!(s.tunable(TunableKey::FetchLanes), 2);
    assert!(!s.tuned.contains_key("dev_fetch_lanes"));
    // Setting to None resets to default.
    store.set(SettingChange::Tunable {
        key: TunableKey::SlowPage,
        value: Some(20),
    });
    let s = store.set(SettingChange::Tunable {
        key: TunableKey::SlowPage,
        value: None,
    });
    assert_eq!(s.tunable(TunableKey::SlowPage), 5);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn set_persists_then_reopen_reads_it() {
    let dir = scratch("persist");
    let path = dir.to_string_lossy().into_owned();
    let heard = Arc::new(Mutex::new(Vec::<Settings>::new()));
    struct Spy(Arc<Mutex<Vec<Settings>>>);
    impl SettingsListener for Spy {
        fn on_settings(&self, settings: Settings) {
            self.0.lock().unwrap().push(settings);
        }
    }
    {
        let store = SettingsStore::open(path.clone());
        store.set_listener(Arc::new(Spy(heard.clone())));
        store.set(SettingChange::ThemeMode {
            value: ThemeMode::Dark,
        });
        store.set(SettingChange::NavTabs {
            ids: vec!["marks".into(), "exams".into()],
        });
        store.set(SettingChange::Notifies {
            kind: ChangeKind::Marks,
            on: false,
        });
        store.set(SettingChange::Tunable {
            key: TunableKey::PageMaxAge,
            value: Some(24),
        });
    }
    assert_eq!(heard.lock().unwrap().len(), 4);

    let reopened = SettingsStore::open(path);
    assert!(!reopened.needs_import());
    let s = reopened.get();
    assert_eq!(s.theme_mode, ThemeMode::Dark);
    assert_eq!(s.nav_tabs, vec!["marks", "exams", "more"]);
    assert_eq!(s.notify_off, vec![ChangeKind::Marks]);
    assert_eq!(s.tunable(TunableKey::PageMaxAge), 24);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn import_failure_leaves_no_file() {
    let dir = scratch("import-fail");
    // Block settings.json with a non-empty directory so atomic rename fails.
    let blocked = dir.join("settings.json");
    fs::create_dir(&blocked).unwrap();
    fs::write(blocked.join("keep"), "x").unwrap();

    let store = SettingsStore::open(dir.to_string_lossy().into_owned());
    assert!(store.needs_import());
    let res = store.import(HashMap::from([(
        "settings_theme_mode".into(),
        PrefValue::Str {
            value: "dark".into(),
        },
    )]));
    assert!(res.is_err());
    assert!(store.needs_import());
    assert_eq!(store.get().theme_mode, ThemeMode::System);
    // No temporary files left behind.
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn forget_student_clears_student_fields_and_keeps_device_prefs() {
    let dir = scratch("forget-student");
    let store = SettingsStore::open(dir.to_string_lossy().into_owned());
    store.set(SettingChange::ThemeMode {
        value: ThemeMode::Dark,
    });
    store.set(SettingChange::OtpSource {
        value: Some(OtpSource::Gmail),
    });
    store.set(SettingChange::BackgroundSync { value: true });
    store.set(SettingChange::SyncHealth {
        paused: Some("no_code".into()),
        otp_misses: 0,
    });
    store.set(SettingChange::SyncCourse {
        value: Some("CSE1001".into()),
    });
    store.set(SettingChange::WrappedSeen {
        ids: vec!["AP1".into()],
    });
    store.set(SettingChange::WrappedHideGrades {
        ids: vec!["AP1".into()],
    });
    store.set(SettingChange::WrappedHideName { value: true });
    store.set(SettingChange::WrappedHideHostel { value: true });
    store.set(SettingChange::StorageNoteSeen { value: true });
    store.set(SettingChange::Student {
        value: Some("22BCE0001".into()),
    });

    let s = store.set(SettingChange::ForgetStudent);
    assert_eq!(s.theme_mode, ThemeMode::Dark);
    assert_eq!(s.otp_source, None);
    assert!(!s.background_sync);
    assert_eq!(s.sync_paused, None);
    assert_eq!(s.sync_otp_misses, 0);
    assert_eq!(s.sync_course, None);
    assert!(s.wrapped_seen.is_empty());
    assert!(s.wrapped_hide_grades.is_empty());
    assert!(!s.wrapped_hide_name);
    assert!(!s.wrapped_hide_hostel);
    assert!(!s.storage_note_seen);
    assert_eq!(s.student, None);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn open_shares_live_instance_so_core_sees_updates() {
    let dir = scratch("shared-live");
    let path = dir.to_string_lossy().into_owned();
    let store_a = SettingsStore::open(path.clone());
    let store_b = SettingsStore::open(path);
    store_a.set(SettingChange::AutoRefresh { value: false });
    assert!(!store_b.get().auto_refresh);
    let _ = fs::remove_dir_all(&dir);
}

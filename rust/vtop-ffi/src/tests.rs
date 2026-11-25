#![expect(
    clippy::disallowed_methods,
    reason = "tests read the session back to check it"
)]

use std::collections::HashMap;

use serde_json::Value;
use vtop_core::types::AttendanceData;

use super::*;
use crate::pages::{changes, records};
use crate::remote::handover::HandoverAsk;
use crate::remote::{account_token, consent, edge, forwarding, route};
use crate::store::cookies;

/// The cache folder of every test that opens a real [`Core::new`]. The log pipeline is
/// process-wide ([`obs::obs_init`]): the first core's cache folder gets the one log file, so
/// these tests share it, and never remove it.
pub(crate) fn shared_cache() -> String {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-cache-{}", std::process::id()));
    dir.to_string_lossy().into()
}

#[tokio::test]
async fn calls_log_ms_and_outcome_without_secrets() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-obs-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let account = r#"{"username":"STUDENT","password":"pw-secret-1"}"#;
    fs::write(dir.join(ACCOUNT), account).unwrap();
    let session = r#"{"username":"STUDENT","saved_at_epoch_ms":0,"cookies":"JSESSIONID=cookie-secret","registration_number":"22BCE0001"}"#;
    fs::write(dir.join(SESSION), session).unwrap();
    fs::create_dir_all(dir.join(PAGES).join("AP1")).unwrap();
    fs::write(dir.join(SELECTED), r#""AP1""#).unwrap();
    let dir_s = dir.to_string_lossy().into();
    let core = Core::open("f-new".into(), dir_s, shared_cache(), Shared::default()).unwrap();

    // Both fail before any network call.
    let signed_in = core
        .sign_in("f-sign".into(), " ".into(), "hunter2-secret".into())
        .await;
    assert_eq!(signed_in, Err(CoreError::InvalidCredentials));
    core.sign_out_here("f-out");
    assert_eq!(core.signed_in_user("f-user".into()), None);
    // Signing out forgets the semester pick and every cached page.
    assert!(!dir.join(PAGES).exists() && !dir.join(SELECTED).exists());
    let otp = core.submit_otp("f-otp".into(), "493817".into()).await;
    assert_eq!(otp, Err(CoreError::NotSignedIn));
    log_event(
        "info".into(),
        "ui.test".into(),
        Some("f-ui".into()),
        "shown".into(),
        HashMap::from([("count".into(), "3".into())]),
    );

    let log = fs::read_to_string(PathBuf::from(shared_cache()).join("logs/app.jsonl")).unwrap();
    for secret in ["hunter2-secret", "493817", "pw-secret-1", "cookie-secret"] {
        assert!(!log.contains(secret), "{secret} logged");
    }
    // Other tests' threads may still be appending: a half-written last line is skipped.
    let lines: Vec<Value> = log
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let line = |flow: &str| lines.iter().rfind(|line| line["flow"] == flow).unwrap();
    assert_eq!(line("f-new")["outcome"], "ok");
    // vtop-core's `log` lines carry the flow and the registration number (not the username).
    let restore = lines
        .iter()
        .find(|line| line["src"] == "rust.auth" && line["flow"] == "f-new")
        .unwrap();
    assert_eq!(
        (&restore["flow"], &restore["regno"]),
        (&"f-new".into(), &"22BCE0001".into())
    );
    assert_eq!(line("f-sign")["src"], "core.sign_in");
    assert_eq!(line("f-sign")["outcome"], "InvalidCredentials");
    assert_eq!(line("f-otp")["outcome"], "NotSignedIn");
    assert_eq!(line("f-otp")["l"], "warn");
    assert!(line("f-otp")["ms"].is_u64());
    assert_eq!(line("f-ui")["count"], "3");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn pages_follow_the_selected_semester() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-sem-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let list = r#"{"semesters":[{"id":"AP2026","name":"Fall 2026"},{"id":"AP2025","name":"Winter 2025"}],"update_time":7}"#;
    fs::write(dir.join(SEMESTERS), list).unwrap();
    for (id, code) in [("AP2026", "NEW1001"), ("AP2025", "OLD1001")] {
        fs::create_dir_all(dir.join(PAGES).join(id)).unwrap();
        let page = format!(
            r#"{{"records":[{{"serial":"1","category":"","course_name":"{code} - Course - Embedded Theory","course_code":"{code}","course_type":"ETH","faculty_detail":"","classes_attended":"3","total_classes":"4","attendance_percentage":"75","attendence_fat_cat":"","debar_status":"","course_id":"x"}}],"semester_id":"{id}","update_time":1}}"#
        );
        fs::write(dir.join(PAGES).join(id).join(ATTENDANCE), page).unwrap();
    }
    // Not Core::new: logging is process-wide and the other test reads its own log file.
    let core = Core::for_test(&dir);
    let code = |core: &Core| {
        core.cached_attendance("f".into()).unwrap().courses[0]
            .code
            .clone()
    };

    // Nothing picked: no page, no guess; the picker comes first.
    let semesters = core.cached_semesters("f".into()).unwrap();
    assert_eq!(semesters.selected, None);
    assert_eq!(semesters.items.len(), 2);
    assert!(core.cached_attendance("f".into()).is_none());

    core.select_semester("f".into(), "AP2026".into()).unwrap();
    assert_eq!(code(&core), "NEW1001");
    core.select_semester("f".into(), "AP2025".into()).unwrap();
    assert_eq!(
        core.cached_semesters("f".into())
            .unwrap()
            .selected
            .as_deref(),
        Some("AP2025")
    );
    assert_eq!(code(&core), "OLD1001");

    // Unknown ids are refused and keep the pick.
    assert!(core.select_semester("f".into(), "NOPE".into()).is_err());
    assert_eq!(code(&core), "OLD1001");

    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn pages_wait_for_a_picked_semester() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-need-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let list = r#"{"semesters":[{"id":"AP2026","name":"Fall Semester 2026-27"}],"update_time":7}"#;
    fs::write(dir.join(SEMESTERS), list).unwrap();
    let account = Account {
        username: "STUDENT".into(),
        password: "pw".into(),
        registration_number: None,
    };
    let core = Core::for_test(&dir);
    let session = Session::new(account, &core.shared).unwrap();
    core.install(session);
    // Before any network call: no pick, no request.
    assert_eq!(
        core.fetch_attendance("f".into()).await,
        Err(CoreError::SemesterNeeded)
    );
    // A pick VTOP no longer lists counts as none.
    fs::write(dir.join(SELECTED), r#""AP2019""#).unwrap();
    assert_eq!(
        core.fetch_timetable("f".into()).await.map(|_| ()),
        Err(CoreError::SemesterNeeded)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn civil_dates() {
    assert_eq!(records::civil_date(0), "1970-01-01");
    assert_eq!(records::civil_date(20_762), "2026-11-05");
    assert_eq!(records::civil_date(-1), "1969-12-31");
}

#[test]
fn attendance_detail_reads_the_course_history() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-detail-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let pages = dir.join(PAGES).join("AP2025262");
    fs::create_dir_all(pages.join(ATTENDANCE_DETAIL)).unwrap();
    fs::write(
        dir.join(SEMESTERS),
        r#"{"semesters":[{"id":"AP2025262","name":"Fall Semester 2025-26"}],"update_time":7}"#,
    )
    .unwrap();
    fs::write(dir.join(SELECTED), r#""AP2025262""#).unwrap();
    fs::write(
        pages.join(ATTENDANCE),
        r#"{"records":[{"serial":"5","category":"","course_name":"SWE1002 - Principles of Software Engineering - Embedded Lab","course_code":"AP2025262000671 - L10+L11 - 409","course_type":"ELA","faculty_detail":"","classes_attended":"22","total_classes":"32","attendance_percentage":"79%","attendence_fat_cat":"-","debar_status":"-","course_id":"AM_SWE1002_00200"}],"semester_id":"AP2025262","update_time":1}"#,
    )
    .unwrap();
    let core = Core::for_test(&dir);
    // Course known, history not fetched yet.
    assert_eq!(
        core.cached_attendance_detail("f".into(), "SWE1002.lab".into()),
        None
    );
    fs::write(
        pages.join(ATTENDANCE_DETAIL).join("SWE1002.lab.json"),
        r#"{"records":[{"serial":"1","date":"19-11-2025","slot":"L10+L11","day_time":"WED / 11:00-12:40","status":"Not Posted","remark":""},{"serial":"2","date":"12-11-2025","slot":"L10+L11","day_time":"WED / 11:00-12:40","status":"Present","remark":""}],"semester_id":"AP2025262","update_time":9,"course_id":"AM_SWE1002_00200","course_type":"ELA"}"#,
    )
    .unwrap();
    let detail = core
        .cached_attendance_detail("f".into(), "SWE1002.lab".into())
        .unwrap();
    assert_eq!(
        (detail.course.key.as_str(), detail.course.safe),
        ("SWE1002.lab", true)
    );
    assert_eq!(detail.updated_at, 9);
    assert_eq!(
        detail
            .classes
            .iter()
            .map(|class| class.status)
            .collect::<Vec<_>>(),
        [None, Some(ClassStatus::Present)]
    );
    // Unknown keys (and paths) find nothing.
    for key in ["SWE1002.theory", "../attendance"] {
        assert_eq!(core.cached_attendance_detail("f".into(), key.into()), None);
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_cached_calendar_shapes_the_agenda() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-cal-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(PAGES).join("AP1")).unwrap();
    fs::write(
        dir.join(SEMESTERS),
        r#"{"semesters":[{"id":"AP1","name":"Fall Semester 2026-27"}],"update_time":1}"#,
    )
    .unwrap();
    fs::write(dir.join(SELECTED), r#""AP1""#).unwrap();
    let timetable = r#"{"slots":[{"serial":"1","day":"SAT","slot":"A1","course_code":"MAT1001","course_type":"ETH","room_no":"1","block":"AB1","start_time":"08:00","end_time":"08:50","name":"Calculus","kind":"Theory","faculty":"","credits":"3"}],"courses":[],"semester_id":"AP1","update_time":1}"#;
    fs::write(dir.join(PAGES).join("AP1").join(TIMETABLE), timetable).unwrap();
    let core = Core::for_test(&dir);
    // Sat 2026-11-07 (epoch day 20 764): a plain class day without a calendar.
    let saturday = 20_764;
    let plain = core.timetable_days("f".into(), saturday, 1);
    assert_eq!(plain[0].classes[0].state, ClassDay::Held);
    assert!(core.cached_calendar("f".into()).is_none());

    let calendar = r#"{"entries":[{"date":"2026-11-06","kind":"Instructional Day","group":"General (Semester)","note":"WorkingDay"},{"date":"2026-11-07","kind":"Holiday","group":"General (Semester)","note":"Deepavali"}],"semester_id":"AP1","update_time":5}"#;
    fs::write(dir.join(PAGES).join("AP1").join(CALENDAR), calendar).unwrap();
    let days = core.timetable_days("f".into(), saturday, 1);
    assert_eq!(days[0].classes[0].state, ClassDay::Off);
    assert_eq!(days[0].title.as_deref(), Some("Deepavali"));
    let view = core.cached_calendar("f".into()).unwrap();
    assert_eq!((view.semester_id.as_str(), view.updated_at), ("AP1", 5));
    assert_eq!(view.months.len(), 1);
    assert_eq!(view.months[0].days[6].kind, Some(CalendarKind::Holiday));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn outlook_plans_every_saved_course() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-outlook-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let pages = dir.join(PAGES).join("AP1");
    fs::create_dir_all(&pages).unwrap();
    fs::write(
        dir.join(SEMESTERS),
        r#"{"semesters":[{"id":"AP1","name":"Fall Semester 2998-99"}],"update_time":1}"#,
    )
    .unwrap();
    fs::write(dir.join(SELECTED), r#""AP1""#).unwrap();
    fs::write(
        pages.join(ATTENDANCE),
        r#"{"records":[{"serial":"1","category":"","course_name":"MAT1001 - Calculus - Theory Only","course_code":"","course_type":"TH","faculty_detail":"","classes_attended":"30","total_classes":"40","attendance_percentage":"75%","attendence_fat_cat":"-","debar_status":"-","course_id":"AM_MAT1001"}],"semester_id":"AP1","update_time":1}"#,
    )
    .unwrap();
    let core = Core::for_test(&dir);
    // No calendar yet: no outlook.
    assert!(core.attendance_outlook("f".into()).is_empty());
    fs::write(
        pages.join(TIMETABLE),
        r#"{"slots":[{"serial":"1","day":"MON","slot":"A1","course_code":"MAT1001","course_type":"TH","room_no":"1","block":"AB1","start_time":"08:00","end_time":"08:50","name":"Calculus","kind":"Theory","faculty":"","credits":"3"}],"courses":[],"semester_id":"AP1","update_time":1}"#,
    )
    .unwrap();
    // Two Mondays of classes in 2999, a CAT between them.
    fs::write(
        pages.join(CALENDAR),
        r#"{"entries":[{"date":"2999-01-07","kind":"Instructional Day","group":"General (Semester)","note":""},{"date":"2999-01-10","kind":"CAT - I","group":"General (Semester)","note":"Exam Days"},{"date":"2999-01-14","kind":"Instructional Day","group":"General (Semester)","note":""}],"semester_id":"AP1","update_time":1}"#,
    )
    .unwrap();
    let outlook = core.attendance_outlook("f".into());
    assert_eq!(outlook.len(), 1);
    assert_eq!(outlook[0].key, "MAT1001.theory");
    let plan = &outlook[0].plan;
    assert_eq!((plan.until.as_deref(), plan.upcoming), (Some("CAT-I"), 1));
    assert_eq!((plan.must_attend, plan.can_miss), (Some(1), 0));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn missing_or_behind_histories_are_mismatched() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-mismatch-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let pages = dir.join(PAGES).join("AP1");
    fs::create_dir_all(pages.join(ATTENDANCE_DETAIL)).unwrap();
    fs::write(
        dir.join(SEMESTERS),
        r#"{"semesters":[{"id":"AP1","name":"Fall Semester 2026-27"}],"update_time":1}"#,
    )
    .unwrap();
    fs::write(dir.join(SELECTED), r#""AP1""#).unwrap();
    let record = |code: &str, attended: u32, total: u32| {
        format!(
            r#"{{"serial":"1","category":"","course_name":"{code} - Course - Theory Only","course_code":"","course_type":"TH","faculty_detail":"","classes_attended":"{attended}","total_classes":"{total}","attendance_percentage":"","attendence_fat_cat":"-","debar_status":"-","course_id":"AM_{code}"}}"#
        )
    };
    fs::write(
        pages.join(ATTENDANCE),
        format!(
            r#"{{"records":[{},{},{},{}],"semester_id":"AP1","update_time":1}}"#,
            record("SYNC1", 1, 2),
            record("BEHIND1", 2, 3),
            record("NEVER1", 1, 1),
            record("EMPTY1", 0, 0),
        ),
    )
    .unwrap();
    let row = |date: &str, status: &str| {
        format!(
            r#"{{"serial":"1","date":"{date}","slot":"A1","day_time":"MON / 08:00-08:50","status":"{status}","remark":""}}"#
        )
    };
    let history = |rows: Vec<String>| {
        format!(
            r#"{{"records":[{}],"semester_id":"AP1","update_time":1,"course_id":"x","course_type":"TH"}}"#,
            rows.join(",")
        )
    };
    // In sync: 1 attended of 2 listed. Behind: VTOP counts a third class.
    let two = vec![row("02-11-2026", "Present"), row("03-11-2026", "Absent")];
    fs::write(
        pages.join(ATTENDANCE_DETAIL).join("SYNC1.theory.json"),
        history(two.clone()),
    )
    .unwrap();
    fs::write(
        pages.join(ATTENDANCE_DETAIL).join("BEHIND1.theory.json"),
        history(two),
    )
    .unwrap();
    let core = Core::for_test(&dir);
    assert_eq!(
        core.mismatched_attendance_details("f".into()),
        ["BEHIND1.theory", "NEVER1.theory"]
    );
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn another_student_signing_in_drops_the_last_ones_pages() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-switch-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(PAGES).join("AP2026")).unwrap();
    let cached = |dir: &PathBuf| {
        fs::write(
            dir.join(SEMESTERS),
            r#"{"semesters":[{"id":"AP2026","name":"Fall 2026"}],"update_time":7}"#,
        )
        .unwrap();
        fs::write(dir.join(SELECTED), r#""AP2026""#).unwrap();
        fs::create_dir_all(dir.join(PAGES).join("AP2026")).unwrap();
        fs::write(dir.join(PAGES).join("AP2026").join(ATTENDANCE), "{}").unwrap();
        fs::write(dir.join(forwarding::FILE), "{}").unwrap();
    };
    cached(&dir);
    fs::write(
        dir.join(ACCOUNT),
        r#"{"username":"21BCE0001","password":"x"}"#,
    )
    .unwrap();
    let account = |username: &str| Account {
        username: username.into(),
        password: "pw".into(),
        registration_number: None,
    };
    let core = Core::for_test(&dir);
    let slot = core.install(Session::new(account("21BCE0002"), &core.shared).unwrap());

    // "Sign in again" with another account: the last one's semesters, pick, pages and
    // forwarding address go.
    core.save(&slot, &*slot.read().await);
    assert_eq!(
        core.signed_in_user("f".into()).as_deref(),
        Some("21BCE0002")
    );
    assert!(
        !dir.join(PAGES).exists() && !dir.join(SELECTED).exists() && !dir.join(SEMESTERS).exists()
    );
    assert!(!dir.join(forwarding::FILE).exists());

    // The same student again (a new password, a relogin) keeps them.
    cached(&dir);
    core.save(&slot, &*slot.read().await);
    assert!(
        dir.join(PAGES).join("AP2026").join(ATTENDANCE).exists() && dir.join(SELECTED).exists()
    );
    assert!(dir.join(forwarding::FILE).exists());

    // Signing out forgets the forwarding address too (the next sign-in can join it again).
    core.sign_out_here("f");
    assert!(!dir.join(forwarding::FILE).exists() && !dir.join(PAGES).exists());
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn a_cancelled_otp_makes_the_next_relogin_start_fresh() {
    use vtop_core::SessionState;

    let account = Account {
        username: "21BCE0001".into(),
        password: "pw".into(),
        registration_number: None,
    };
    let core =
        Core::for_test(std::env::temp_dir().join(format!("vtop-ffi-otp-{}", std::process::id())));
    let slot = core.install(Session::new(account, &core.shared).unwrap());
    let waiting = |cookies: &str, otp: Option<u64>| SessionState {
        cookies: cookies.into(),
        csrf_token: Some("csrf".into()),
        registration_number: None,
        otp_issued_at: otp,
        logged_in_at: None,
    };

    // Nothing pending: the signed-in client is kept.
    slot.read()
        .await
        .client
        .resume_session(&waiting("JSESSIONID=live", None))
        .unwrap();
    core.cancel_otp("f".into()).await.unwrap();
    assert_eq!(
        slot.read().await.client.session_state().cookies,
        "JSESSIONID=live"
    );

    // A re-login waiting for the OTP: the challenge and its half-finished login go, so the
    // next re-login signs in again instead of answering OtpRequired with the old challenge.
    {
        let mut session = slot.write().await;
        session
            .client
            .resume_session(&waiting("JSESSIONID=pre", Some(7)))
            .unwrap();
        session.begin_otp(
            OtpVia::Phone,
            "Additional verification is required.".into(),
            7,
        );
        session.logins = 3;
    }
    // Calls that need VTOP's session (Connected apps) answer with the open challenge too,
    // without checking the session with VTOP, which would wipe the challenge.
    let _gate = consent::tests::yes(&core);
    assert_eq!(
        core.handover_cookies(
            "f",
            HandoverAsk {
                request_id: "r1".into(),
                asker: None,
                asked_at: None,
                expires_at: None,
                server: "vmate.aster0.dev".into(),
            },
        )
        .await,
        Err(CoreError::OtpRequired {
            reason: "Additional verification is required.".into(),
            issued_at: Some(7),
        })
    );
    assert_eq!(
        slot.read().await.client.session_state().cookies,
        "JSESSIONID=pre"
    );
    // Until then a re-login answers with the open challenge and when VTOP mailed its code.
    assert_eq!(
        core.relogin(&slot, 3).await,
        Err(CoreError::OtpRequired {
            reason: "Additional verification is required.".into(),
            issued_at: Some(7),
        })
    );
    core.cancel_otp("f".into()).await.unwrap();
    let session = slot.read().await;
    assert_eq!(session.otp_message(), None);
    assert_eq!(session.logins, 3);
    let state = session.client.session_state();
    assert_eq!((state.otp_issued_at, state.cookies.as_str()), (None, ""));
    drop(session);

    core.sign_out_here("f");
    assert_eq!(
        core.cancel_otp("f".into()).await,
        Err(CoreError::NotSignedIn)
    );
}

#[tokio::test]
async fn saved_cookies_move_to_the_shared_store_once_and_stay_there() {
    let (memory, cookies) = cookies::tests::with_shared();
    let vtop = format!("{}/vtop", vtop_core::config::DEFAULT_BASE_URL);
    let dir = std::env::temp_dir().join(format!("vtop-ffi-jar-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // As an install before the shared store saved it: the cookies in the file.
    let before = r#"{"username":"22BCE0001","saved_at_epoch_ms":0,"cookies":"JSESSIONID=old; SERVERID=s1","csrf_token":"csrf-1","registration_number":"22BCE0001"}"#;
    fs::write(dir.join(SESSION), before).unwrap();
    // The web view's cookies from the old copying go.
    memory.set_cookies(
        vtop.clone(),
        vec!["JSESSIONID=web; Domain=.vitap.ac.in".into()],
    );
    let account = Account {
        username: "22BCE0001".into(),
        password: "pw".into(),
        registration_number: None,
    };
    let core = Core::for_test_on(
        &dir,
        Shared {
            cookies,
            ..Shared::default()
        },
    );
    let launch = || {
        let session = Session::new(account.clone(), &core.shared).unwrap();
        core.restore(&session, core.read(SESSION));
        session
    };

    let first = launch();
    assert!(first.client.is_authenticated() && first.cookies.is_shared());
    assert_eq!(
        memory.cookie(vtop.clone()).as_deref(),
        Some("JSESSIONID=old; SERVERID=s1")
    );
    let saved: PersistedVtopSession = core.read(SESSION).unwrap();
    assert_eq!(saved.cookies, None, "rewritten without cookies");
    assert_eq!(saved.csrf_token.as_deref(), Some("csrf-1"));
    let clears = *memory.clears.lock().unwrap();

    // The next launch: the store's cookies (VTOP's latest) are the session's, untouched.
    memory.set_cookies(vtop.clone(), vec!["SERVERID=s2; Path=/".into()]);
    let second = launch();
    assert!(second.client.is_authenticated() && second.cookies.is_shared());
    assert!(!first.cookies.is_shared(), "the session before lets go");
    assert_eq!(*memory.clears.lock().unwrap(), clears);
    assert_eq!(
        second.client.cookie_header().as_deref(),
        Some("JSESSIONID=old; SERVERID=s2")
    );

    // A sign-in saves without cookies; signing out empties the store.
    let slot = core.install(second);
    core.save(&slot, &*slot.read().await);
    assert_eq!(
        core.read::<PersistedVtopSession>(SESSION).unwrap().cookies,
        None
    );
    core.sign_out_here("f");
    assert_eq!(memory.cookie(vtop), None);
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn clearing_the_session_empties_the_shared_store() {
    let (memory, cookies) = cookies::tests::with_shared();
    let vtop = format!("{}/vtop", vtop_core::config::DEFAULT_BASE_URL);
    let dir = std::env::temp_dir().join(format!("vtop-ffi-jar-clear-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let account = Account {
        username: "22BCE0001".into(),
        password: "pw".into(),
        registration_number: None,
    };
    let core = Core::for_test_on(
        &dir,
        Shared {
            cookies,
            ..Shared::default()
        },
    );
    let session = Session::new(account, &core.shared).unwrap();
    session.cookies.activate(&core.shared.cookies, false);
    memory.set_cookies(vtop.clone(), vec!["JSESSIONID=live; Path=/vtop".into()]);
    core.install(session);
    assert_eq!(
        core.dev_cookie_editor_json("f".into())
            .await
            .unwrap()
            .map(|json| json.contains("live")),
        Some(true),
        "Copy saved cookies reads the shared store"
    );

    core.dev_clear_session("f".into()).await.unwrap();

    assert_eq!(memory.cookie(vtop), None);
    let session = core.current().unwrap();
    assert!(
        session.read().await.cookies.is_shared(),
        "the fresh client uses the store"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// Answers VTOP's sign-in pages (landing, CAPTCHA, credentials) and counts the credential
/// posts; with `otp`, the post lands on VTOP's OTP page.
async fn login_vtop(otp: bool) -> (String, Arc<std::sync::atomic::AtomicUsize>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let logins = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let captcha = format!(
        r#"<img class="form-control img-fluid bg-light border-0" src="data:image/jpeg;base64,{}">"#,
        base64(include_bytes!("../../vtop-proto/tests/captchas/1.jpg"))
    );
    let count = logins.clone();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let (count, captcha) = (count.clone(), captcha.clone());
            tokio::spawn(async move {
                let mut buffer = vec![0u8; 16 * 1024];
                while let Ok(read) = socket.read(&mut buffer).await {
                    if read == 0 {
                        return;
                    }
                    let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                    let mut line = request.split_whitespace();
                    let (method, path) = (line.next().unwrap_or(""), line.next().unwrap_or("/"));
                    let (status, location, body) = match (method, path) {
                        (_, "/vtop/open/page") => (
                            "200 OK",
                            "",
                            r#"<input name="_csrf" value="c1">"#.to_string(),
                        ),
                        (_, "/vtop/prelogin/setup") => ("200 OK", "", captcha.clone()),
                        ("POST", "/vtop/login") => {
                            count.fetch_add(1, Ordering::SeqCst);
                            if otp {
                                ("302 Found", "/vtop/login/error", String::new())
                            } else {
                                ("200 OK", "", r#"<input name="_csrf" value="c2"><input type="hidden" name="authorizedIDX" value="22BCE0001">"#.to_string())
                            }
                        }
                        (_, "/vtop/login/error") => (
                            "200 OK",
                            "",
                            r#"<input name="_csrf" value="c3">OTP sent to your registered email."#
                                .to_string(),
                        ),
                        _ => ("404 Not Found", "", String::new()),
                    };
                    let location = if location.is_empty() {
                        String::new()
                    } else {
                        format!("Location: {location}\r\n")
                    };
                    let response = format!(
                        "HTTP/1.1 {status}\r\n{location}Content-Type: text/html\r\nContent-Length: {}\r\n\r\n{body}",
                        body.len()
                    );
                    if socket.write_all(response.as_bytes()).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    (base, logins)
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().fold(0u32, |n, &b| n << 8 | b as u32) << (8 * (3 - chunk.len()));
        for i in 0..=chunk.len() {
            out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    out
}

/// A core on `base`'s VTOP whose session signed in `age` seconds ago.
fn core_signed_in_ago(base: String, name: &str, age: u64) -> (Core, Arc<Slot>, PathBuf) {
    use vtop_core::{SessionState, VtopClient, VtopConfig};

    let dir = std::env::temp_dir().join(format!("vtop-ffi-fresh-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let client = VtopClient::builder()
        .config(VtopConfig {
            base_url: base,
            ..VtopConfig::default()
        })
        .with_credentials(
            Username::parse("22BCE0001").unwrap(),
            Password::parse("pw").unwrap(),
        )
        .unwrap();
    client
        .resume_session(&SessionState {
            cookies: String::new(),
            csrf_token: Some("csrf".into()),
            registration_number: Some("22BCE0001".into()),
            otp_issued_at: None,
            logged_in_at: Some(unix_now() as u64 - age),
        })
        .unwrap();
    let core = Core::for_test(&dir);
    let slot = core.install(Session::for_test(
        client,
        Account {
            username: "22BCE0001".into(),
            password: "pw".into(),
            registration_number: None,
        },
    ));
    (core, slot, dir)
}

#[tokio::test]
async fn fresh_session_skips_sign_in() {
    let (base, logins) = login_vtop(false).await;
    let (core, slot, dir) = core_signed_in_ago(base, "skip", 60);
    core.ensure_fresh_session("f".into(), 900).await.unwrap();
    assert_eq!(logins.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(slot.read().await.logins, 0);
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn stale_session_signs_in_again() {
    let (base, logins) = login_vtop(false).await;
    let (core, slot, dir) = core_signed_in_ago(base, "stale", 1000);
    let before = unix_now() as u64;
    core.ensure_fresh_session("f".into(), 900).await.unwrap();
    assert_eq!(logins.load(std::sync::atomic::Ordering::SeqCst), 1);
    let session = slot.read().await;
    assert_eq!(session.logins, 1);
    assert!(session.client.session_state().logged_in_at.unwrap() >= before);
    drop(session);
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn stale_session_needing_otp_returns_otp_required() {
    let (base, logins) = login_vtop(true).await;
    let (core, slot, dir) = core_signed_in_ago(base, "otp", 1000);
    let result = core.ensure_fresh_session("f".into(), 900).await;
    assert!(
        matches!(result, Err(CoreError::OtpRequired { .. })),
        "{result:?}"
    );
    assert_eq!(logins.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(slot.read().await.is_awaiting_otp());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn sign_out_forgets_the_connection_choice() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-route-out-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let dir_s = dir.to_string_lossy().into();
    let core = Core::open("route-new".into(), dir_s, shared_cache(), Shared::default()).unwrap();
    core.set_route("route-set".into(), route::Route::Server)
        .unwrap();
    assert!(dir.join(route::FILE).exists());
    core.sign_out_here("route-out");
    assert!(!dir.join(route::FILE).exists());
    assert_eq!(
        core.route_status("route-status".into()).chosen,
        route::Route::Direct
    );
}

#[test]
fn sign_out_leaves_only_the_phone_files() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-sign-out-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(PAGES).join("AP1")).unwrap();
    fs::write(dir.join(PAGES).join("AP1").join(ATTENDANCE), "{}").unwrap();
    // The phone's, not the student's: they stay.
    let kept = [
        "device.json",
        "udp-gateway.json",
        "demo_record.json",
        edge::SERVER_FILE,
    ];
    // Listed here, not taken from STUDENT_FILES, so a file left out of it fails.
    let student = [
        ACCOUNT,
        SESSION,
        route::FILE,
        SEMESTERS,
        SELECTED,
        forwarding::FILE,
        changes::FILE,
        consent::FILE,
        account_token::FILE,
        demo::FILE,
    ];
    for file in student.iter().chain(&kept) {
        fs::write(dir.join(file), "{}").unwrap();
    }
    // Not Core::new: its log setup is global (see calls_log_ms_and_outcome_without_secrets).
    let core = Core::for_test(&dir);
    core.sign_out_here("sign-out");
    let mut left: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    left.sort_unstable();
    let mut kept = kept.map(str::to_string).to_vec();
    kept.sort_unstable();
    assert_eq!(left, kept);
    let _ = fs::remove_dir_all(&dir);
}

/// A core on `name`'s folder with semesters AP1 (picked) and AP0, each with `courses`
/// attendance courses coded `<semester>-<n>`.
fn kept_fixture(name: &str, courses: usize) -> (Core, PathBuf) {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(SEMESTERS),
        r#"{"semesters":[{"id":"AP1","name":"Fall"},{"id":"AP0","name":"Winter"}],"update_time":7}"#,
    )
    .unwrap();
    fs::write(dir.join(SELECTED), r#""AP1""#).unwrap();
    for semester in ["AP1", "AP0"] {
        fs::create_dir_all(dir.join(PAGES).join(semester)).unwrap();
        fs::write(
            dir.join(PAGES).join(semester).join(ATTENDANCE),
            attendance_json(semester, courses, "30"),
        )
        .unwrap();
    }
    let core = Core::for_test(&dir);
    (core, dir)
}

fn attendance_json(semester: &str, courses: usize, attended: &str) -> String {
    let records: Vec<String> = (0..courses)
        .map(|n| {
            format!(
                r#"{{"serial":"{n}","category":"","course_name":"{semester}-{n} - Course {n} - Embedded Theory","course_code":"{semester}-{n}","course_type":"ETH","faculty_detail":"70001 - A FACULTY - SCOPE","classes_attended":"{attended}","total_classes":"34","attendance_percentage":"89","attendence_fat_cat":"","debar_status":"","course_id":"AP2026{n}"}}"#
            )
        })
        .collect();
    format!(
        r#"{{"records":[{}],"semester_id":"{semester}","update_time":1}}"#,
        records.join(",")
    )
}

fn first_course(core: &Core) -> Option<(String, i32)> {
    let course = core.cached_attendance("f".into())?.courses.remove(0);
    Some((course.code, course.attended))
}

#[test]
fn kept_pages_follow_writes() {
    let (core, dir) = kept_fixture("kept-write", 1);
    let page = dir.join(PAGES).join("AP1").join(ATTENDANCE);
    assert_eq!(first_course(&core), Some(("AP1-0".into(), 30)));
    assert_eq!(
        core.shared.decoded.kept(&page),
        1,
        "kept after the first read"
    );
    assert_eq!(first_course(&core), Some(("AP1-0".into(), 30)));

    let mut data: AttendanceData = serde_json::from_str(&attendance_json("AP1", 1, "31")).unwrap();
    core.write(&format!("{PAGES}/AP1/{ATTENDANCE}"), &data);
    assert_eq!(core.shared.decoded.kept(&page), 0, "a write drops it");
    assert_eq!(first_course(&core), Some(("AP1-0".into(), 31)));

    // Replaced on disk without `write` (same length): the stamp catches it.
    fs::write(&page, attendance_json("AP1", 1, "32")).unwrap();
    assert_eq!(first_course(&core), Some(("AP1-0".into(), 32)));
    // Gone from disk: gone here too.
    fs::remove_file(&page).unwrap();
    assert_eq!(first_course(&core), None);
    assert_eq!(core.shared.decoded.kept(&page), 0);

    // Written with another type at the same path: each type is kept apart.
    data.records.clear();
    core.write(&format!("{PAGES}/AP1/{ATTENDANCE}"), &data);
    assert!(
        core.cached_attendance("f".into())
            .unwrap()
            .courses
            .is_empty()
    );
    let raw = core.read_kept::<Value>(&format!("{PAGES}/AP1/{ATTENDANCE}"));
    assert_eq!(raw.unwrap()["records"], serde_json::json!([]));
    assert_eq!(core.shared.decoded.kept(&page), 2);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn kept_pages_follow_the_semester() {
    let (core, dir) = kept_fixture("kept-sem", 1);
    assert_eq!(first_course(&core).unwrap().0, "AP1-0");
    assert_eq!(
        core.cached_semesters("f".into())
            .unwrap()
            .selected
            .as_deref(),
        Some("AP1")
    );
    core.select_semester("f".into(), "AP0".into()).unwrap();
    assert_eq!(first_course(&core).unwrap().0, "AP0-0");
    assert_eq!(
        core.cached_semesters("f".into())
            .unwrap()
            .selected
            .as_deref(),
        Some("AP0")
    );

    // A new list without the pick: no page.
    let list: SemesterData =
        serde_json::from_str(r#"{"semesters":[{"id":"AP1","name":"Fall"}],"update_time":8}"#)
            .unwrap();
    core.write(SEMESTERS, &list);
    assert_eq!(first_course(&core), None);
    assert_eq!(core.cached_semesters("f".into()).unwrap().selected, None);

    // Developer's reset forgets the pick.
    core.select_semester("f".into(), "AP1".into()).unwrap();
    assert_eq!(first_course(&core).unwrap().0, "AP1-0");
    core.dev_reset_semester("f".into()).unwrap();
    assert_eq!(first_course(&core), None);
    assert_eq!(core.shared.decoded.kept(&dir.join(SELECTED)), 0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn kept_pages_go_with_clears_and_sign_out() {
    let (core, dir) = kept_fixture("kept-clear", 1);
    let page = dir.join(PAGES).join("AP1").join(ATTENDANCE);
    assert!(first_course(&core).is_some());
    core.dev_clear_cache("f".into(), dev::CacheScope::Semester)
        .unwrap();
    assert_eq!(
        (first_course(&core), core.shared.decoded.kept(&page)),
        (None, 0)
    );

    fs::create_dir_all(page.parent().unwrap()).unwrap();
    fs::write(&page, attendance_json("AP1", 1, "30")).unwrap();
    assert!(first_course(&core).is_some());
    core.dev_clear_cache("f".into(), dev::CacheScope::All)
        .unwrap();
    assert_eq!(
        (first_course(&core), core.shared.decoded.kept(&page)),
        (None, 0)
    );

    fs::create_dir_all(page.parent().unwrap()).unwrap();
    fs::write(&page, attendance_json("AP1", 1, "30")).unwrap();
    assert!(first_course(&core).is_some());
    assert!(core.cached_semesters("f".into()).is_some());
    core.sign_out_here("f");
    assert_eq!(first_course(&core), None);
    assert!(core.cached_semesters("f".into()).is_none());
    assert_eq!(
        core.shared.decoded.kept(&page) + core.shared.decoded.kept(&dir.join(SEMESTERS)),
        0
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn kept_pages_are_bounded() {
    let (core, dir) = kept_fixture("kept-bound", 1);
    // Six pages just under a quarter of the budget each: the first read goes first.
    let big = (1 << 20) - 200;
    let files: Vec<String> = (0..6).map(|n| format!("{PAGES}/AP1/big{n}.json")).collect();
    for file in &files {
        fs::write(core.at(file), serde_json::to_vec(&"x".repeat(big)).unwrap()).unwrap();
        assert!(core.read_kept::<String>(file).is_some());
    }
    assert_eq!(
        core.shared.decoded.kept(&core.at(&files[0])),
        0,
        "least recently used goes"
    );
    assert_eq!(core.shared.decoded.kept(&core.at(&files[5])), 1);
    // Over a quarter of the budget: never kept.
    let huge = format!("{PAGES}/AP1/huge.json");
    fs::write(
        core.at(&huge),
        serde_json::to_vec(&"x".repeat(2 << 20)).unwrap(),
    )
    .unwrap();
    assert!(core.read_kept::<String>(&huge).is_some());
    assert_eq!(core.shared.decoded.kept(&core.at(&huge)), 0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn each_core_keeps_its_own_copies() {
    let (core, dir) = kept_fixture("kept-own-a", 1);
    let (other, other_dir) = kept_fixture("kept-own-b", 1);
    // Four pages just under a quarter of the budget each fill this core's budget.
    let big = (1 << 20) - 200;
    let files: Vec<String> = (0..4).map(|n| format!("{PAGES}/AP1/big{n}.json")).collect();
    for file in &files {
        fs::write(core.at(file), serde_json::to_vec(&"x".repeat(big)).unwrap()).unwrap();
        assert!(core.read_kept::<String>(file).is_some());
    }
    // Another core's reads don't spend this one's budget, and its clears don't reach here.
    assert!(first_course(&other).is_some());
    other.forget(&[]);
    for file in &files {
        assert_eq!(core.shared.decoded.kept(&core.at(file)), 1, "{file}");
    }
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&other_dir);
}

/// `cargo test --release -p vtop-ffi cached_page_bench -- --ignored --nocapture`
#[test]
#[ignore = "timing, run by hand"]
fn cached_page_bench() {
    let (core, dir) = kept_fixture("bench", 12);
    assert_eq!(
        core.cached_attendance("f".into()).unwrap().courses.len(),
        12
    );
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        std::hint::black_box(core.cached_attendance("f".into()));
    }
    let attendance = start.elapsed();
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        std::hint::black_box(core.read_page::<AttendanceData>(ATTENDANCE));
    }
    let read_page = start.elapsed();
    println!("1000 cached_attendance: {attendance:?}; 1000 read_page: {read_page:?}");
    let _ = fs::remove_dir_all(&dir);
}

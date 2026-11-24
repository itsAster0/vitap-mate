use std::fs;

use vtop_core::spec;

use super::*;

#[test]
fn keys_name_semester_and_args() {
    assert_eq!(key("profile", None, &[]), "profile");
    assert_eq!(key("grades", Some("AP1"), &[]), "grades@AP1");
    assert_eq!(
        key(
            "full_attendance",
            Some("AP1"),
            &[("course_id", "C1"), ("type", "TH")]
        ),
        "full_attendance@AP1?course_id=C1&type=TH"
    );
}

#[tokio::test]
async fn demo_reads_sample_pages_and_never_calls_vtop() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-demo-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let bundle = r#"{"schema":1,"pages":{
        "semesters":{"semesters":[{"id":"AP1","name":"Fall 2026"}],"update_time":1},
        "biometric?date=01-Jul-2026":[]
    }}"#;
    fs::write(dir.join(FILE), bundle).unwrap();
    // Signed in without the sign-in, so the bundle written above stays (see edge_demo_signs_in_and_every_page_reads for Core::new).
    let account = Account {
        username: USERNAME.into(),
        password: PASSWORD.into(),
        rejected: false,
        registration_number: None,
    };
    let core = Core::for_test(&dir);
    let session = crate::Session::new(account.clone(), &core.shared).unwrap();
    core.install(Some(Arc::new(RwLock::new(session))));
    core.write_account(&account);

    assert!(core.is_demo("demo-is".into()));
    let semesters = core.fetch_semesters("demo-semesters".into()).await.unwrap();
    assert_eq!(semesters.items.len(), 1);
    // A page recorded for another day stands in for this one.
    assert!(
        core.demo_page::<Vec<Value>>("biometric", None, &[("date", "02-Jul-2026")])
            .await
            .is_some_and(|page| page.is_ok())
    );
    // Not recorded at all, and anything that needs a real session: never VTOP.
    assert_eq!(
        core.fetch_profile("demo-profile".into()).await.err(),
        Some(unavailable())
    );
    assert_eq!(
        core.web_session("demo-web".into(), false).await.err(),
        Some(unavailable())
    );
    core.ensure_fresh_session("demo-fresh".into(), 0)
        .await
        .unwrap();
}

/// The edge's demo (`/guides/demo.json`), as the app downloads it.
const EDGE_DEMO: &str = include_str!("../../../../backend/guides/demo.json");

/// `key`'s parts: the page's name, its semester and its arguments.
fn parts(key: &str) -> (&str, Option<&str>, Vec<(&str, &str)>) {
    let (page, query) = key.split_once('?').unwrap_or((key, ""));
    let (name, semester) = page
        .split_once('@')
        .map_or((page, None), |(name, semester)| (name, Some(semester)));
    let args = query
        .split('&')
        .filter(|arg| !arg.is_empty())
        .map(|arg| arg.split_once('=').unwrap_or((arg, "")))
        .collect();
    (name, semester, args)
}

/// Page `key` of the signed-in demo, as `spec`'s type: from the bundle as recorded, and as the
/// app reads it ([`Core::demo_page`]).
async fn read<T: DeserializeOwned>(core: &Core, spec: &spec::PageSpec<T>, key: &str, page: &Value) {
    if let Err(error) = serde_json::from_value::<T>(page.clone()) {
        panic!("{key} is not a {}: {error}", spec.name);
    }
    let (name, semester, args) = parts(key);
    assert_eq!(
        key,
        super::key(name, semester, &args),
        "{key} is not a demo key"
    );
    let read = core.demo_page::<T>(name, semester, &args).await;
    assert!(read.is_some_and(|page| page.is_ok()), "{key} not read");
}

#[tokio::test]
async fn edge_demo_signs_in_and_every_page_reads() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-edge-demo-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let data = dir.to_string_lossy().to_string();
    let cache = crate::tests::shared_cache();
    let core = Core::open("demo-new".into(), data, cache, crate::Shared::default()).unwrap();
    let account = Account {
        username: USERNAME.into(),
        password: PASSWORD.into(),
        rejected: false,
        registration_number: None,
    };
    let signed = core.open_demo(account, EDGE_DEMO.as_bytes().to_vec()).await;
    assert_eq!(signed, Ok(SignIn::Done));
    assert!(core.is_demo("demo-is".into()));

    let bundle: Bundle = serde_json::from_str(EDGE_DEMO).unwrap();
    assert!(bundle.schema <= SCHEMA, "the edge's demo needs a newer app");
    assert!(!bundle.pages.is_empty());
    for (key, page) in &bundle.pages {
        // One arm per page in `spec::each`; a name the app doesn't know fails.
        macro_rules! pages {
            ($($spec:ident),* $(,)?) => {
                match parts(key).0 {
                    $(name if name == spec::$spec.name => read(&core, &spec::$spec, key, page).await,)*
                    name => panic!("{key}: no page {name}"),
                }
            };
        }
        pages!(
            SEMESTERS,
            ATTENDANCE,
            FULL_ATTENDANCE,
            TIMETABLE,
            MARKS,
            EXAM_SCHEDULE,
            GRADES,
            ACADEMIC_CALENDAR,
            GRADE_DETAILS,
            GRADE_HISTORY,
            BIOMETRIC,
            PROFILE,
            MENTOR,
            GENERAL_OUTING,
            WEEKEND_OUTING,
            COURSE_PAGE_COURSES,
            COURSE_PAGE_CLASSES,
            COURSE_PAGE_DETAIL,
        );
    }
    // The app's own readers, end to end.
    let semesters = core.fetch_semesters("demo-semesters".into()).await.unwrap();
    assert!(!semesters.items.is_empty());
    let _ = fs::remove_dir_all(&dir);
}

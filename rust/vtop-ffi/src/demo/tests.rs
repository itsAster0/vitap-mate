use std::fs;

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
    // Not Core::new: its log setup is global, and calls_log_ms_and_outcome_without_secrets owns it.
    let account = Account {
        username: USERNAME.into(),
        password: PASSWORD.into(),
        rejected: false,
        registration_number: None,
    };
    let core = Core {
        dir: dir.clone(),
        cache: dir.clone(),
        slot: Mutex::new(Some(Arc::new(RwLock::new(
            crate::Session::new(account.clone()).unwrap(),
        )))),
    };
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

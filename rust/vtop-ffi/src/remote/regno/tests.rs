use std::fs;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use super::*;
use crate::remote::backend::Kind;
use crate::remote::consent::tests::{TestGate, install};
use crate::test_edge::{Edge, keep_token, paths, serve, signed_in};

const REGNO: &str = "22BCE0001";

/// Saves Share for `regno`, as the sheet would: for tests of what comes after it.
pub(crate) fn shared(core: &Core, regno: &str) {
    core.run_regno_effects(regno, vec![Effect::Save(StoredAnswer::Shared)]);
}

/// A signed-in core that was never asked, on a fake server.
async fn setup(name: &str) -> (Core, Arc<Mutex<Edge>>, String) {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-regno-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = signed_in(&dir, server.clone());
    // Installed with this version: the file is there, with no answer yet.
    core.write_plain(FILE, &Stored::default());
    (core, edge, server)
}

fn answers(gate: &TestGate, script: &[RegnoAnswer]) {
    gate.regno_answers.lock().unwrap().extend(script);
}

async fn view(core: &Core, flow: &str, ask: bool) -> Result<Result<Value, EdgeErr>, CoreError> {
    core.account_call(flow, Kind::Query, "account:view", "Load", ask, json!({}))
        .await
}

type EdgeErr = crate::device::EdgeError;

#[test]
fn stored_format_roundtrips() {
    let stored = Stored {
        answers: [
            ("22BCE0001".to_string(), StoredAnswer::Shared),
            ("22BCE0002".to_string(), StoredAnswer::Declined),
        ]
        .into(),
    };
    let text = r#"{"answers":{"22BCE0001":"shared","22BCE0002":"declined"}}"#;
    assert_eq!(serde_json::to_string(&stored).unwrap(), text);
    assert_eq!(serde_json::from_str::<Stored>(text).unwrap(), stored);
    assert_eq!(
        serde_json::to_string(&Stored::default()).unwrap(),
        r#"{"answers":{}}"#
    );
    assert_eq!(
        serde_json::from_str::<Stored>("{}").unwrap(),
        Stored::default()
    );
    // Unasked is never written, so a file that says it doesn't read.
    assert!(serde_json::from_str::<Stored>(r#"{"answers":{"x":"unasked"}}"#).is_err());
}

#[test]
fn resolve_reads_the_answer() {
    let stored = Stored {
        answers: [(REGNO.to_string(), StoredAnswer::Declined)].into(),
    };
    let cases = [
        ("answered", REGNO, RegnoSharing::Declined),
        ("lower case", "22bce0001", RegnoSharing::Declined),
        ("another student", "22BCE9999", RegnoSharing::Unasked),
        ("demo", crate::demo::USERNAME, RegnoSharing::Shared),
    ];
    for (name, regno, want) in cases {
        assert_eq!(resolve(&stored, regno), want, "{name}");
    }
    assert_eq!(resolve(&Stored::default(), REGNO), RegnoSharing::Unasked);
}

#[test]
fn an_old_install_shares_for_whom_it_holds_an_identity() {
    let stored = migrated(["22bce0001".to_string(), "22BCE0002".to_string()]);
    assert_eq!(resolve(&stored, REGNO), RegnoSharing::Shared);
    assert_eq!(resolve(&stored, "22BCE0002"), RegnoSharing::Shared);
    assert_eq!(resolve(&stored, "22BCE0003"), RegnoSharing::Unasked);
    assert_eq!(migrated([]), Stored::default());
}

#[test]
fn answers_move_the_state() {
    use RegnoAnswer::*;
    use RegnoSharing::*;
    let save = |answer| Effect::Save(answer);
    let cases = [
        (Unasked, Share, Shared, vec![save(StoredAnswer::Shared)]),
        (Declined, Share, Shared, vec![save(StoredAnswer::Shared)]),
        (
            Unasked,
            NotNow,
            Declined,
            vec![save(StoredAnswer::Declined), Effect::ForgetIdentity],
        ),
        (
            Shared,
            NotNow,
            Declined,
            vec![save(StoredAnswer::Declined), Effect::ForgetIdentity],
        ),
        (Unasked, Dismissed, Unasked, vec![]),
        (Declined, Dismissed, Declined, vec![]),
        (Shared, Dismissed, Shared, vec![]),
    ];
    for (from, answer, to, effects) in cases {
        assert_eq!(next(from, answer), (to, effects), "{from:?} {answer:?}");
    }
}

#[test]
fn only_first_use_skips_a_declined_student() {
    use RegnoAsk::*;
    use RegnoSharing::*;
    let cases = [
        (Unasked, FirstTime, true),
        (Unasked, ForFeature, true),
        (Declined, FirstTime, false),
        (Declined, ForFeature, true),
        (Shared, FirstTime, false),
        (Shared, ForFeature, false),
    ];
    for (sharing, ask, want) in cases {
        assert_eq!(should_ask(sharing, ask), want, "{sharing:?} {ask:?}");
    }
}

#[tokio::test]
async fn unasked_proves_nothing_in_the_background_and_sends_nothing() {
    let (core, edge, _) = setup("unasked").await;
    let gate = install(&core, false, false);
    assert_eq!(
        core.prove_in_background("bg".into(), 1_000).await,
        Ok(false)
    );
    assert!(matches!(
        view(&core, "quiet", false).await,
        Err(CoreError::RegnoNotShared)
    ));
    core.set_push_token("push".into(), "fcm-1".into())
        .await
        .unwrap();
    assert_eq!(
        gate.regno_asks.load(Ordering::SeqCst),
        0,
        "unattended: no sheet"
    );
    assert!(paths(&edge).is_empty(), "nothing reached the server");
    assert_eq!(core.regno_sharing("s".into()).await, RegnoSharing::Unasked);
    assert_eq!(
        core.read::<Stored>(FILE),
        Some(Stored::default()),
        "nothing saved"
    );
}

#[tokio::test]
async fn declined_never_sends_the_kept_refresh_token() {
    let (core, edge, server) = setup("declined").await;
    keep_token(&core, &server);
    core.run_regno_effects(REGNO, vec![Effect::Save(StoredAnswer::Declined)]);
    // Attended, but a call nobody started: no sheet.
    let gate = install(&core, true, false);
    assert!(matches!(
        view(&core, "quiet", false).await,
        Err(CoreError::RegnoNotShared)
    ));
    assert_eq!(
        core.prove_in_background("bg".into(), 1_000).await,
        Ok(false)
    );
    assert!(
        !core.has_server_identity("h".into()).await,
        "held, yet unused"
    );
    assert_eq!(gate.regno_asks.load(Ordering::SeqCst), 0);
    assert!(paths(&edge).is_empty());
}

#[tokio::test]
async fn a_feature_asks_once_per_flow_and_goes_on_after_share() {
    let (core, edge, server) = setup("feature").await;
    keep_token(&core, &server);
    core.run_regno_effects(REGNO, vec![Effect::Save(StoredAnswer::Declined)]);
    let gate = install(&core, true, false);
    answers(&gate, &[RegnoAnswer::NotNow, RegnoAnswer::Share]);
    assert!(matches!(
        view(&core, "open", true).await,
        Err(CoreError::RegnoNotShared)
    ));
    // The same screen's next call doesn't ask again.
    assert!(matches!(
        view(&core, "open", true).await,
        Err(CoreError::RegnoNotShared)
    ));
    assert_eq!(gate.regno_asks.load(Ordering::SeqCst), 1);
    assert!(paths(&edge).is_empty());
    assert!(
        core.device().refresh.is_none(),
        "Not now dropped the identity"
    );
    // Opened again: asked, Share, and the call goes on (a refresh token stands in for the
    // proof, which needs the fake VTOP).
    keep_token(&core, &server);
    assert!(view(&core, "again", true).await.is_ok());
    assert_eq!(gate.regno_asks.load(Ordering::SeqCst), 2);
    assert_eq!(paths(&edge), ["auth:token", "account:view"]);
    assert_eq!(core.regno_sharing("s".into()).await, RegnoSharing::Shared);
    // Shared: no more sheets.
    assert!(view(&core, "later", true).await.is_ok());
    assert_eq!(gate.regno_asks.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_dismissed_sheet_leaves_it_unasked() {
    let (core, edge, _) = setup("dismissed").await;
    let gate = install(&core, true, false);
    let asked = core
        .ask_regno_sharing("first".into(), RegnoAsk::FirstTime)
        .await;
    assert_eq!(asked, Ok(RegnoSharing::Unasked));
    assert_eq!(gate.regno_asks.load(Ordering::SeqCst), 1);
    assert_eq!(core.read::<Stored>(FILE), Some(Stored::default()));
    assert!(paths(&edge).is_empty());
}

#[tokio::test]
async fn first_time_skips_a_declined_student_but_a_route_asks() {
    let (core, _edge, _) = setup("first").await;
    let gate = install(&core, true, false);
    answers(&gate, &[RegnoAnswer::NotNow, RegnoAnswer::Share]);
    let ask = |flow: &str, ask| core.ask_regno_sharing(flow.into(), ask);
    assert_eq!(
        ask("a", RegnoAsk::FirstTime).await,
        Ok(RegnoSharing::Declined)
    );
    assert_eq!(
        ask("b", RegnoAsk::FirstTime).await,
        Ok(RegnoSharing::Declined)
    );
    assert_eq!(gate.regno_asks.load(Ordering::SeqCst), 1);
    assert_eq!(
        ask("c", RegnoAsk::ForFeature).await,
        Ok(RegnoSharing::Shared)
    );
    assert_eq!(gate.regno_asks.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn an_old_install_with_an_identity_counts_as_shared() {
    let (core, edge, server) = setup("migrate").await;
    core.remove_file(FILE);
    keep_token(&core, &server);
    let gate = install(&core, false, false);
    assert_eq!(core.regno_sharing("s".into()).await, RegnoSharing::Shared);
    assert_eq!(
        core.read::<Stored>(FILE),
        Some(migrated([REGNO.to_string()]))
    );
    // And it keeps working unasked, as before.
    view(&core, "quiet", false)
        .await
        .unwrap()
        .unwrap_or_default();
    assert_eq!(paths(&edge)[0], "auth:token");
    assert_eq!(gate.regno_asks.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn an_old_forwarding_address_counts_as_shared() {
    let (core, _edge, _) = setup("migrate-mail").await;
    core.remove_file(FILE);
    core.write_account(&crate::Account {
        username: REGNO.into(),
        password: "pw".into(),
        rejected: false,
        registration_number: None,
    });
    fs::write(core.at(forwarding::FILE), "{}").unwrap();
    assert_eq!(core.regno_sharing("s".into()).await, RegnoSharing::Shared);
}

#[tokio::test]
async fn stopping_drops_the_identity_and_the_server_route() {
    let (core, edge, server) = setup("stop").await;
    keep_token(&core, &server);
    install(&core, false, false);
    core.set_route("r".into(), Route::Server).unwrap();
    let to = core
        .set_regno_sharing("stop".into(), RegnoAnswer::NotNow)
        .await;
    assert_eq!(to, Ok(RegnoSharing::Declined));
    assert!(core.device().refresh.is_none(), "refresh token dropped");
    assert_eq!(core.route_status("r".into()).chosen, Route::Direct);
    assert!(matches!(
        view(&core, "quiet", false).await,
        Err(CoreError::RegnoNotShared)
    ));
    assert!(paths(&edge).is_empty());
    // And back on.
    let to = core
        .set_regno_sharing("on".into(), RegnoAnswer::Share)
        .await;
    assert_eq!(to, Ok(RegnoSharing::Shared));
}

#[tokio::test]
async fn a_fresh_sign_in_after_a_sign_out_is_asked_again() {
    let (core, _edge, server) = setup("out").await;
    keep_token(&core, &server);
    core.run_regno_effects(REGNO, vec![Effect::Save(StoredAnswer::Shared)]);
    core.sign_out_here("out");
    // The refresh token stays to leave the old address, but it is no answer.
    assert!(core.device().refresh.is_some());
    assert_eq!(core.read::<Stored>(FILE), Some(Stored::default()));
    assert_eq!(core.sharing_of(REGNO), RegnoSharing::Unasked);
}

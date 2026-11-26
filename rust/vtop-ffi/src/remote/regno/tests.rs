use std::fs;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use super::*;
use crate::remote::backend::Kind;
use crate::remote::backend::identity::{KEPT_SIGN_OUT, KeptSignOut};
use crate::remote::consent::tests::{TestGate, install};
use crate::test_edge::{Edge, REFRESH, keep_token, paths, serve, signed_in};

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
            vec![save(StoredAnswer::Declined), Effect::SignOut],
        ),
        (
            Shared,
            NotNow,
            Declined,
            vec![save(StoredAnswer::Declined), Effect::SignOut],
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
    // proof, which needs the fake VTOP), once the identity Not now set aside is signed out.
    keep_token(&core, &server);
    can(&edge, "auth:signOut", Value::Null);
    assert!(view(&core, "again", true).await.is_ok());
    assert_eq!(gate.regno_asks.load(Ordering::SeqCst), 2);
    assert_eq!(
        paths(&edge),
        ["auth:token", "auth:signOut", "auth:token", "account:view"]
    );
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
    fs::write(core.at(forwarding::FILE), "{}").unwrap();
    // No regno saved from VTOP: the username is never taken for it.
    core.write_account(
        &crate::Account {
            username: "STUDENT314".into(),
            password: "pw".into(),
            registration_number: None,
        },
        false,
    );
    core.regno_sharing("s".into()).await;
    let kept = fs::read_to_string(core.at(FILE)).unwrap_or_default();
    assert!(!kept.contains("STUDENT314"), "{kept}");
    // The regno VTOP gave, saved with a username that isn't it, counts.
    core.remove_file(FILE);
    core.write_account(
        &crate::Account {
            username: "STUDENT314".into(),
            password: "pw".into(),
            registration_number: Some(REGNO.into()),
        },
        false,
    );
    assert_eq!(core.regno_sharing("s".into()).await, RegnoSharing::Shared);
}

fn can(edge: &Arc<Mutex<Edge>>, path: &str, body: Value) {
    let mut edge = edge.lock().unwrap();
    edge.canned
        .entry(path.into())
        .or_default()
        .push_back((200, body));
}

/// A forwarding address this install is on.
fn joined(core: &Core, server: &str) {
    let stored = json!({
        "server": server, "id": "k3f9x2ab7q", "address": "otp+k3f9x2ab7q@vmate.aster0.dev",
        "joined": true, "ready": true,
    });
    fs::write(core.at(forwarding::FILE), stored.to_string()).unwrap();
}

fn forwarding_joined(core: &Core) -> Option<bool> {
    let text = fs::read_to_string(core.at(forwarding::FILE)).ok()?;
    serde_json::from_str::<Value>(&text).ok()?["joined"].as_bool()
}

fn body(edge: &Arc<Mutex<Edge>>, index: usize) -> Value {
    edge.lock().unwrap().requests[index].body.clone()
}

#[tokio::test]
async fn stopping_signs_out_of_the_server_and_drops_the_server_route() {
    let (core, edge, server) = setup("stop").await;
    keep_token(&core, &server);
    joined(&core, &server);
    install(&core, false, false);
    core.set_route("r".into(), Route::Server).unwrap();
    can(&edge, "auth:signOut", Value::Null);
    let to = core
        .set_regno_sharing("stop".into(), RegnoAnswer::NotNow)
        .await;
    assert_eq!(to, Ok(RegnoSharing::Declined));
    assert!(core.device().refresh.is_none(), "refresh token dropped");
    assert_eq!(core.route_status("r".into()).chosen, Route::Direct);
    // This install's refresh chain is revoked on the server; the address stays, to rejoin.
    assert_eq!(paths(&edge), ["auth:token", "auth:signOut"]);
    assert_eq!(body(&edge, 0), json!({ "refreshToken": REFRESH }));
    assert_eq!(body(&edge, 1), json!({ "everywhere": false }));
    assert!(!core.at(KEPT_SIGN_OUT).exists());
    assert_eq!(forwarding_joined(&core), Some(false));
    assert!(matches!(
        view(&core, "quiet", false).await,
        Err(CoreError::RegnoNotShared)
    ));
    assert_eq!(paths(&edge).len(), 2);
    // And back on.
    let to = core
        .set_regno_sharing("on".into(), RegnoAnswer::Share)
        .await;
    assert_eq!(to, Ok(RegnoSharing::Shared));
}

#[tokio::test]
async fn stopping_offline_signs_out_at_the_next_connection() {
    let (core, edge, server) = setup("stop-offline").await;
    keep_token(&core, &server);
    edge.lock().unwrap().offline = true;
    let to = core
        .set_regno_sharing("stop".into(), RegnoAnswer::NotNow)
        .await;
    assert_eq!(to, Ok(RegnoSharing::Declined));
    // Dropped here at once; the revoke waits with the refresh token.
    assert!(core.device().refresh.is_none());
    let kept = core.read::<KeptSignOut>(KEPT_SIGN_OUT).unwrap();
    assert_eq!(kept.refresh.token, REFRESH);
    assert!(!kept.everywhere);
    assert!(paths(&edge).is_empty());
    edge.lock().unwrap().offline = false;
    can(&edge, "auth:signOut", Value::Null);
    core.leave_old_forwarding("open".into()).await.unwrap();
    assert_eq!(paths(&edge), ["auth:token", "auth:signOut"]);
    assert_eq!(body(&edge, 0), json!({ "refreshToken": REFRESH }));
    assert!(!core.at(KEPT_SIGN_OUT).exists());
}

#[tokio::test]
async fn sharing_again_signs_out_first_and_never_proves_offline() {
    let (core, edge, server) = setup("again").await;
    keep_token(&core, &server);
    edge.lock().unwrap().offline = true;
    core.set_regno_sharing("stop".into(), RegnoAnswer::NotNow)
        .await
        .unwrap();
    core.set_regno_sharing("on".into(), RegnoAnswer::Share)
        .await
        .unwrap();
    // A new identity now would be taken by the revoke sent after it: none while it waits.
    assert_eq!(
        view(&core, "use", true).await.map(drop),
        Err(CoreError::Offline)
    );
    assert!(core.at(KEPT_SIGN_OUT).exists());
    // Back online: the revoke goes before the proof.
    edge.lock().unwrap().offline = false;
    can(&edge, "auth:signOut", Value::Null);
    let _proof_fails_in_tests = view(&core, "use", true).await;
    assert_eq!(paths(&edge)[..2], ["auth:token", "auth:signOut"]);
    assert!(!core.at(KEPT_SIGN_OUT).exists());
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

#[tokio::test]
async fn waiting_for_an_identity_ends_at_share() {
    let (core, _edge, server) = setup("wait").await;
    keep_token(&core, &server);
    core.run_regno_effects(REGNO, vec![Effect::Save(StoredAnswer::Declined)]);
    let wait = core.wait_server_identity("w".into());
    futures::pin_mut!(wait);
    // Held but not shared: nothing to follow with yet.
    assert!(futures::poll!(wait.as_mut()).is_pending());
    assert!(futures::poll!(wait.as_mut()).is_pending());
    core.set_regno_sharing("s".into(), RegnoAnswer::Share)
        .await
        .unwrap();
    assert!(futures::poll!(wait.as_mut()).is_ready());
}

#[tokio::test]
async fn signed_in_without_known_regno_fails_closed_while_signed_out_stays_open() {
    let (core, edge, server) = setup("no-regno").await;
    keep_token(&core, &server);
    // Signed in with no regno known from VTOP (neither session nor account.json has it).
    let slot = core.current().unwrap();
    {
        let mut session = slot.write().await;
        session.account.registration_number = None;
        let state = vtop_core::SessionState {
            cookies: String::new(),
            csrf_token: None,
            registration_number: None,
            otp_issued_at: None,
            logged_in_at: None,
        };
        session.client.resume_session(&state).unwrap();
    }
    core.write_account(
        &crate::Account {
            username: "STUDENT314".into(),
            password: "pw".into(),
            registration_number: None,
        },
        false,
    );
    core.rename_student(&slot, "STUDENT314");
    assert_eq!(core.signed_in_regno().await, None);
    let gate = install(&core, true, false);
    assert!(!core.identity_shared().await);
    assert_eq!(
        core.regno_allowed("f", None, false).await,
        Err(CoreError::RegnoNotShared)
    );
    assert_eq!(
        core.regno_allowed("f", None, true).await,
        Err(CoreError::RegnoNotShared)
    );
    assert_eq!(gate.regno_asks.load(Ordering::SeqCst), 0);
    assert!(!core.has_server_identity("f".into()).await);
    assert!(matches!(
        view(&core, "f", false).await,
        Err(CoreError::RegnoNotShared)
    ));
    assert!(paths(&edge).is_empty(), "nothing reached the server");

    // Signed out keeps today's behaviour so a kept token can still leave an old address.
    core.sign_out_here("out");
    assert!(core.identity_shared().await);
    assert_eq!(core.regno_allowed("f", None, false).await, Ok(()));
    can(&edge, "account:view", json!({ "thisPhone": true }));
    assert!(view(&core, "f", false).await.unwrap().is_ok());
    assert_eq!(paths(&edge), ["auth:token", "account:view"]);
}

struct CountingCipher(std::sync::atomic::AtomicUsize);

impl crate::PasswordCipher for CountingCipher {
    fn seal(&self, plain: String) -> crate::Sealing {
        crate::Sealing::Sealed {
            sealed: format!("s:{plain}"),
        }
    }
    fn open(&self, sealed: String) -> crate::Opening {
        self.0.fetch_add(1, Ordering::SeqCst);
        crate::Opening::Opened {
            plain: sealed.trim_start_matches("s:").to_string(),
        }
    }
}

#[test]
fn migration_uses_session_regno_for_same_username_without_opening_keystore() {
    let dir = std::env::temp_dir().join(format!(
        "vtop-ffi-regno-migrate-session-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let cipher = Arc::new(CountingCipher(std::sync::atomic::AtomicUsize::new(0)));
    let core = Core::for_test_on(
        &dir,
        crate::Shared {
            cipher: Some(cipher.clone()),
            ..crate::Shared::default()
        },
    );
    fs::write(core.at(forwarding::FILE), "{}").unwrap();
    fs::write(
        core.at(crate::ACCOUNT),
        json!({ "username": "STUDENT314", "sealed": "s:secret" }).to_string(),
    )
    .unwrap();
    // Different username in session.json: never migrated.
    fs::write(
        core.at(crate::SESSION),
        json!({
            "username": "OTHER",
            "saved_at_epoch_ms": 0,
            "cookies": null,
            "csrf_token": null,
            "registration_number": "22BCE9999",
        })
        .to_string(),
    )
    .unwrap();
    assert_eq!(core.sharing_of("22BCE9999"), RegnoSharing::Unasked);
    assert_eq!(cipher.0.load(Ordering::SeqCst), 0, "keystore never opened");

    // Matching username (case-insensitive) in session.json: migrated without opening keystore.
    core.remove_file(FILE);
    fs::write(
        core.at(crate::SESSION),
        json!({
            "username": "student314",
            "saved_at_epoch_ms": 0,
            "cookies": null,
            "csrf_token": null,
            "registration_number": REGNO,
        })
        .to_string(),
    )
    .unwrap();
    assert_eq!(core.sharing_of(REGNO), RegnoSharing::Shared);
    assert_eq!(cipher.0.load(Ordering::SeqCst), 0, "keystore never opened");
    let _ = fs::remove_dir_all(&dir);
}

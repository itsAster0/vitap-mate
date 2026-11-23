use std::sync::Arc;

use vtop_shared::device_sig;

use super::*;
use crate::remote::consent;
use crate::remote::edge::SERVER_FILE;
use crate::test_edge::{Edge, ID, keep_token, now, paths, serve, signed_in};

/// Seals `payload` the way the server does: to this install's seal key, for mailbox [`ID`].
fn seal(core: &Core, edge: &Arc<Mutex<Edge>>, payload: Value) {
    let key = core.device().seal_key;
    edge.lock().unwrap().sealed =
        Some(mail_seal::seal(&key, ID, payload.to_string().as_bytes()).unwrap());
}

#[tokio::test]
async fn sets_up_checks_fetches_and_turns_off() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    fs::write(dir.join(SERVER_FILE), url.clone() + "\n").unwrap();
    // The server makes the student's address, or joins theirs.
    let core = signed_in(&dir, url.clone());
    keep_token(&core, &url);
    assert_eq!(
        core.forwarding_status("f".into()).state,
        ForwardingState::Off
    );
    let fetched = core.fetch_forwarded_otp("f".into(), 0).await;
    assert!(matches!(fetched, Err(CoreError::ForwardingOff { .. })));

    let set_up = core.set_up_forwarding("f-setup".into()).await.unwrap();
    assert_eq!(set_up.state, ForwardingState::Waiting);
    assert_eq!(
        set_up.address.as_deref(),
        Some("otp+k3f9x2ab7q@vmate.aster0.dev")
    );
    assert_eq!(set_up.sender, VTOP_SENDER);
    {
        let edge = edge.lock().unwrap();
        let paths: Vec<&str> = edge.requests.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(paths, ["auth:token", "mail:register"]);
        assert!(edge.requests.iter().all(|r| r.flow == "f-setup"));
        assert_eq!(edge.requests[1].body, json!({}));
    }
    // Set up already: nothing is sent.
    core.set_up_forwarding("f".into()).await.unwrap();
    assert_eq!(edge.lock().unwrap().requests.len(), 2);

    // Nothing forwarded yet.
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(checked.state, ForwardingState::Waiting);
    assert_eq!(
        edge.lock().unwrap().requests.last().unwrap().body,
        json!({ "id": ID })
    );

    // Gmail's confirmation shows the address was added, not that forwarding works yet.
    seal(
        &core,
        &edge,
        json!({ "kind": "gmail_confirm", "code": "981234567", "link": null, "requester": "me@vitapstudent.ac.in", "sentAt": 1_700_000_000 }),
    );
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(checked.state, ForwardingState::Waiting);
    let confirmation = checked.confirmation.unwrap();
    assert_eq!(confirmation.code.as_deref(), Some("981234567"));
    assert_eq!(checked.last_mail_at, Some(1_700_000_000));
    assert_eq!(checked.last_otp_at, None);
    // Not an OTP.
    assert_eq!(core.fetch_forwarded_otp("f".into(), 0).await, Ok(None));

    let sent = now();
    seal(
        &core,
        &edge,
        json!({ "kind": "otp", "code": "401295", "sentAt": sent }),
    );
    let code = core.fetch_forwarded_otp("f".into(), sent - 30).await;
    assert_eq!(code, Ok(Some("401295".into())));
    // An OTP from before this prompt.
    let code = core.fetch_forwarded_otp("f".into(), sent + 120).await;
    assert_eq!(code, Ok(None));
    let status = core.forwarding_status("f".into());
    assert_eq!(
        (status.state, status.last_otp_at),
        (ForwardingState::On, Some(sent))
    );
    // No key or secret in forwarding.json.
    let saved = fs::read_to_string(dir.join(FILE)).unwrap();
    assert!(!saved.contains("secret") && !saved.contains("seed"));

    core.turn_off_forwarding("f-off".into()).await.unwrap();
    {
        let edge = edge.lock().unwrap();
        let last = edge.requests.last().unwrap();
        assert_eq!(
            (last.path.as_str(), last.flow.as_str()),
            ("mail:remove", "f-off")
        );
        assert_eq!(last.body, json!({ "id": ID, "everywhere": false }));
    }
    assert_eq!(
        core.forwarding_status("f".into()).state,
        ForwardingState::Off
    );
    assert!(!dir.join(FILE).exists());

    // An address the server lost is forgotten here.
    core.set_up_forwarding("f".into()).await.unwrap();
    edge.lock().unwrap().gone = true;
    let checked = core.check_forwarding("f".into()).await;
    assert!(matches!(checked, Err(CoreError::ForwardingOff { .. })));
    assert_eq!(
        core.forwarding_status("f".into()).state,
        ForwardingState::Off
    );

    // One identity; every call went signed or with its JWT.
    let edge = edge.lock().unwrap();
    let tokens = edge
        .requests
        .iter()
        .filter(|r| r.path == "auth:token")
        .count();
    assert_eq!(tokens, 1);
    assert!(edge.requests.iter().all(|r| r.signed));
}

#[tokio::test]
async fn forwarding_setup_without_a_token_or_proof_sends_nothing() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-deny-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    fs::write(dir.join(SERVER_FILE), &url).unwrap();
    // Signed in, but no proof network here: the proof fails before anything is sent.
    let core = signed_in(&dir, url);
    let gate = consent::tests::no(&core);
    let set_up = core.set_up_forwarding("f".into()).await;
    assert!(matches!(set_up, Err(CoreError::Other { .. })), "{set_up:?}");
    assert_eq!(gate.asks(), 0);
    assert!(edge.lock().unwrap().requests.is_empty());
    assert!(!dir.join(FILE).exists());
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn check_shows_the_confirmation_the_edge_kept() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-kept-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    fs::write(dir.join(SERVER_FILE), url.clone() + "\n").unwrap();
    let core = signed_in(&dir, url.clone());
    keep_token(&core, &url);
    core.set_up_forwarding("f".into()).await.unwrap();
    // Gmail's mail came before this install joined: no box, only the kept confirmation.
    let kept = {
        let payload = json!({ "kind": "gmail_confirm", "code": "981234567", "link": null, "requester": null, "sentAt": 1_700_000_000 });
        mail_seal::seal(&core.device().seal_key, ID, payload.to_string().as_bytes()).unwrap()
    };
    let inbox = "mail:inbox".to_string();
    edge.lock().unwrap().canned.insert(
        inbox.clone(),
        [(
            200,
            json!({ "sealed": null, "confirmation": kept, "lastOtpAt": null }),
        )]
        .into(),
    );
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(checked.state, ForwardingState::Waiting);
    assert_eq!(
        checked.confirmation.unwrap().code.as_deref(),
        Some("981234567")
    );
    assert_eq!(checked.last_otp_at, None);

    // An OTP forwarded to the address before shows it works.
    edge.lock().unwrap().canned.insert(
        inbox,
        [(
            200,
            json!({ "sealed": null, "confirmation": null, "lastOtpAt": 1_700_000_100 }),
        )]
        .into(),
    );
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(checked.last_otp_at, Some(1_700_000_100));
    // and Gmail's confirmation goes, here as on the server.
    assert!(checked.confirmation.is_none());
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn signed_calls_are_this_installs() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-sig-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    fs::write(dir.join(SERVER_FILE), &server).unwrap();
    let core = Core::for_test(&dir);
    let kind = Kind::Mutation;
    core.signed_call("f", kind, "auth:proveStart", &json!({}))
        .await
        .unwrap();
    assert!(edge.lock().unwrap().requests[0].signed);
    // Another seed's signature over the same call does not verify against this install's key.
    let key = device_sig::public_key(&core.device().seed).unwrap();
    let other = device_sig::generate_seed();
    let sig = device_sig::sign(&other, "CALL", "/auth:proveStart", now(), b"{}").unwrap();
    assert!(!device_sig::verify(
        &key,
        &sig,
        "CALL",
        "/auth:proveStart",
        now(),
        b"{}"
    ));
}

#[tokio::test]
async fn old_file_reads_as_no_address() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-old-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let old = json!({
        "server": "https://vmate-api.aster0.dev", "id": ID, "address": "a@b",
        "secret": "ms_x", "private_key": "pk", "device_id": "d", "device_secret": "ds",
        "ready": true,
    });
    fs::write(dir.join(FILE), old.to_string()).unwrap();
    let core = Core::for_test(&dir);
    assert_eq!(
        core.forwarding_status("f".into()).state,
        ForwardingState::Off
    );
}

#[tokio::test]
async fn set_up_needs_a_vtop_sign_in() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-nosign-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let core = Core::for_test(&dir);
    assert_eq!(
        core.set_up_forwarding("f".into()).await,
        Err(CoreError::NotSignedIn)
    );
    assert!(!dir.join(FILE).exists());
}

#[tokio::test]
async fn offline_keeps_the_address() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-off-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let core = Core::for_test(&dir);
    let saved = Saved {
        // Nothing listens on port 9.
        server: "http://127.0.0.1:9".into(),
        id: ID.into(),
        address: "a@b".into(),
        owner_claimed: false,
        joined: false,
        ready: false,
        last_mail_at: None,
        last_otp_at: None,
        confirmation: None,
    };
    core.write(FILE, &saved);
    // With an identity on the default server, which this test can't reach.
    keep_token(&core, crate::remote::edge::DEFAULT_SERVER);
    assert_eq!(
        core.check_forwarding("f".into()).await,
        Err(CoreError::Offline)
    );
    assert_eq!(
        core.turn_off_forwarding("f".into()).await,
        Err(CoreError::Offline)
    );
    assert_eq!(
        core.forwarding_status("f".into()).state,
        ForwardingState::Waiting
    );
}

#[test]
fn otps_are_six_digits_and_recent() {
    let otp = |text: &str| serde_json::from_str::<Payload>(text).unwrap();
    let fresh = otp(r#"{"kind":"otp","code":"401295","sentAt":1000}"#);
    assert_eq!(fresh.otp_since(1000, 0), Some("401295"));
    assert_eq!(fresh.otp_since(1003, VTOP_SLACK_SECS), Some("401295"));
    // An earlier sign-in's code, a few seconds before this challenge.
    assert_eq!(fresh.otp_since(1006, VTOP_SLACK_SECS), None);
    assert_eq!(
        fresh.otp_since(1000 + CLOCK_SKEW_SECS, CLOCK_SKEW_SECS),
        Some("401295")
    );
    assert_eq!(
        fresh.otp_since(1001 + CLOCK_SKEW_SECS, CLOCK_SKEW_SECS),
        None
    );
    assert_eq!(
        otp(r#"{"kind":"otp","code":"40129a","sentAt":1000}"#).otp_since(0, 0),
        None
    );
    let confirm = otp(
        r#"{"kind":"gmail_confirm","code":null,"link":"https://mail-settings.google.com/mail/vf-x","requester":null,"sentAt":5}"#,
    );
    assert_eq!(
        (confirm.kind(), confirm.otp_since(0, 0)),
        ("gmail_confirm", None)
    );
    assert!(serde_json::from_str::<Payload>(r#"{"kind":"other","sentAt":1}"#).is_err());
}

#[tokio::test]
async fn sign_out_leaves_the_address_on_the_edge() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-leave-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    fs::write(dir.join(SERVER_FILE), url.clone() + "\n").unwrap();
    let core = signed_in(&dir, url.clone());
    keep_token(&core, &url);
    core.set_up_forwarding("f".into()).await.unwrap();

    core.sign_out("f-out".into());
    assert!(!dir.join(FILE).exists());
    assert!(dir.join(LEAVE_FILE).exists());
    // A server refusing it for now (or offline): still queued.
    edge.lock().unwrap().canned.insert(
        "mail:leave".into(),
        [(
            500,
            json!({ "code": "server_error", "error": "Try again." }),
        )]
        .into(),
    );
    core.leave_old_forwarding("f".into()).await.unwrap();
    assert!(dir.join(LEAVE_FILE).exists());

    core.leave_old_forwarding("f-leave".into()).await.unwrap();
    {
        let edge = edge.lock().unwrap();
        let last = edge.requests.last().unwrap();
        assert_eq!(
            (last.path.as_str(), last.flow.as_str()),
            ("mail:leave", "f-leave")
        );
        assert_eq!(last.body, json!({ "id": ID }));
    }
    assert!(!dir.join(LEAVE_FILE).exists());
    let sent = edge.lock().unwrap().requests.len();
    core.leave_old_forwarding("f".into()).await.unwrap();
    assert_eq!(edge.lock().unwrap().requests.len(), sent);
    let _ = fs::remove_dir_all(&dir);
}

async fn forwarding_core(name: &str) -> (Core, Arc<Mutex<Edge>>, std::path::PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    fs::write(dir.join(SERVER_FILE), url.clone() + "\n").unwrap();
    let core = signed_in(&dir, url.clone());
    keep_token(&core, &url);
    core.set_up_forwarding("f".into()).await.unwrap();
    (core, edge, dir, url)
}

#[tokio::test]
async fn a_queued_leave_waits_for_the_next_identity() {
    let (core, edge, dir, url) = forwarding_core("later").await;
    core.sign_out("f-out".into());
    core.forget_identity();
    core.leave_old_forwarding("f".into()).await.unwrap();
    assert!(dir.join(LEAVE_FILE).exists());
    assert!(!paths(&edge).contains(&"mail:leave".to_string()));
    // The next student's identity leaves it, whichever call gets it.
    keep_token(&core, &url);
    core.leave_old_forwarding("f-next".into()).await.unwrap();
    assert!(!dir.join(LEAVE_FILE).exists());
    let leaves = paths(&edge).iter().filter(|p| *p == "mail:leave").count();
    assert_eq!(leaves, 1);
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn signing_out_of_the_server_leaves_first_then_forgets() {
    let (core, edge, dir, _url) = forwarding_core("signout").await;
    edge.lock()
        .unwrap()
        .canned
        .insert("auth:signOut".into(), [(200, Value::Null)].into());
    core.sign_out_of_server("f-s".into(), true).await.unwrap();
    let sent = paths(&edge);
    let tail: Vec<&str> = sent
        .iter()
        .rev()
        .take(2)
        .rev()
        .map(String::as_str)
        .collect();
    assert_eq!(tail, ["mail:leave", "auth:signOut"]);
    assert_eq!(
        edge.lock().unwrap().requests.last().unwrap().body,
        json!({ "everywhere": true })
    );
    assert!(!dir.join(FILE).exists());
    assert!(core.device().refresh.is_none());
    // The local sign-out then has nothing left to queue.
    core.sign_out("f-out".into());
    assert!(!dir.join(LEAVE_FILE).exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn opens_only_gmail_confirm_links() {
    let ok = "https://mail-settings.google.com/mail/vf-%5BAN%5D-abc";
    assert_eq!(gmail_link(ok).as_deref(), Some(ok));
    assert!(gmail_link("https://google.com/mail/vf-x").is_some());
    for bad in [
        "http://mail.google.com/mail/vf-x",
        "https://mail.google.com.evil.example/mail/vf-x",
        "https://evilgoogle.com/mail/vf-x",
        "https://user@mail.google.com/mail/vf-x",
        "https://mail.google.com/other",
        "intent://mail.google.com/mail/vf-x",
        "https://www.google.com/mail/vf-/../../url?q=https://evil.example",
        "https://www.google.com/mail/vf-x\\..\\..\\url",
        "https://mail.google.com/mail/vf-",
    ] {
        assert_eq!(gmail_link(bad), None, "{bad}");
    }
}

#[test]
fn knows_a_requester_on_the_profile() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-req-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(PAGES)).unwrap();
    let core = Core::for_test(&dir);
    assert_eq!(core.requester_known(Some("me@gmail.com")), None);
    let profile = json!({
        "name": "A", "registration_number": "22BCE0001", "vit_email": "a.22bce0001@vitapstudent.ac.in",
        "program": "", "school": "", "photo": "", "update_time": 0,
        "sections": [{ "title": "PERSONAL", "groups": [{ "title": "", "fields": [
            { "label": "Email", "value": "Me@Gmail.com, other@x.in" }
        ] }] }],
    });
    fs::write(
        dir.join(PAGES).join(spec::PROFILE.file),
        profile.to_string(),
    )
    .unwrap();
    assert_eq!(core.requester_known(Some("me@gmail.com")), Some(true));
    assert_eq!(
        core.requester_known(Some("a.22bce0001@vitapstudent.ac.in")),
        Some(true)
    );
    assert_eq!(core.requester_known(Some("someone@gmail.com")), Some(false));
    assert_eq!(core.requester_known(None), None);
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn an_address_kept_from_v2_is_rejoined_once() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-keep-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    fs::write(dir.join(SERVER_FILE), url.clone() + "\n").unwrap();
    let core = signed_in(&dir, url.clone());
    keep_token(&core, &url);
    let v2 = Saved {
        server: url.clone(),
        id: ID.into(),
        address: format!("otp+{ID}@vmate.aster0.dev"),
        owner_claimed: true,
        joined: false,
        ready: true,
        last_mail_at: Some(5),
        last_otp_at: Some(5),
        confirmation: None,
    };
    core.write(FILE, &v2);
    // An OTP wait before the rejoin: the server doesn't know this install on the address yet,
    // so it is rejoined there and then, not forgotten.
    edge.lock().unwrap().gone = true;
    let pulled = core.fetch_forwarded_otp("f".into(), 0).await;
    assert!(matches!(pulled, Err(CoreError::Other { .. })), "{pulled:?}");
    assert_eq!(paths(&edge), ["auth:token", "mail:inbox", "mail:register"]);
    edge.lock().unwrap().gone = false;
    let kept = core.read::<Saved>(FILE).unwrap();
    assert!(kept.joined);
    // The same address: its Gmail setup still holds.
    assert_eq!((kept.id.as_str(), kept.ready), (ID, true));
    core.keep_forwarding("f".into()).await.unwrap();
    // Joined: opening the app again asks nothing.
    assert_eq!(paths(&edge).len(), 3);

    // The account holds another address: this phone moves to it and checks Gmail again.
    core.write(
        FILE,
        &Saved {
            id: "zzzzzzzzzz".into(),
            ..v2
        },
    );
    core.keep_forwarding("f".into()).await.unwrap();
    let moved = core.read::<Saved>(FILE).unwrap();
    assert_eq!(
        (moved.id.as_str(), moved.ready, moved.joined),
        (ID, false, true)
    );
    // The old address's "last OTP" isn't shown for the new one.
    assert_eq!((moved.last_otp_at, moved.last_mail_at), (None, None));
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn a_phone_moved_to_the_students_old_address_follows_it() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-moved-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    fs::write(dir.join(SERVER_FILE), url.clone() + "\n").unwrap();
    let core = signed_in(&dir, url.clone());
    keep_token(&core, &url);
    core.set_up_forwarding("f".into()).await.unwrap();
    // The server gave the account's old address the win: this phone is on it now.
    {
        let mut edge = edge.lock().unwrap();
        edge.gone = true;
        edge.moved_to = Some("zzzzzzzzzz".into());
    }
    let pulled = core.fetch_forwarded_otp("f".into(), 0).await;
    assert!(matches!(pulled, Err(CoreError::Other { .. })), "{pulled:?}");
    let kept = core.read::<Saved>(FILE).unwrap();
    assert_eq!(
        (kept.id.as_str(), kept.ready, kept.joined),
        ("zzzzzzzzzz", false, true)
    );
    // Deleted on another phone: no address to follow, so it is forgotten.
    edge.lock().unwrap().moved_to = None;
    let pulled = core.fetch_forwarded_otp("f".into(), 0).await;
    assert!(matches!(pulled, Err(CoreError::ForwardingOff { .. })));
    assert!(core.read::<Saved>(FILE).is_none());
    let _ = fs::remove_dir_all(&dir);
}

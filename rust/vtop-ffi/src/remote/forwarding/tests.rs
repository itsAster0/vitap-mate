use std::sync::{Arc, Mutex};

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

/// The view's variant, for comparing.
fn state(forwarding: &Forwarding) -> &'static str {
    match forwarding {
        Forwarding::Off { .. } => "off",
        Forwarding::Waiting { .. } => "waiting",
        Forwarding::On { .. } => "on",
    }
}

fn confirmation(forwarding: &Forwarding) -> Option<GmailConfirmation> {
    match forwarding {
        Forwarding::Waiting { confirmation, .. } => confirmation.clone(),
        _ => None,
    }
}

fn last_mail_at(forwarding: &Forwarding) -> Option<i64> {
    match forwarding {
        Forwarding::Off { .. } => None,
        Forwarding::Waiting { last_mail_at, .. } | Forwarding::On { last_mail_at, .. } => {
            *last_mail_at
        }
    }
}

fn last_otp_at(forwarding: &Forwarding) -> Option<i64> {
    match forwarding {
        Forwarding::On { last_otp_at, .. } => *last_otp_at,
        _ => None,
    }
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
    assert_eq!(state(&core.forwarding_status("f".into())), "off");
    let fetched = core.fetch_forwarded_otp("f".into(), 0).await;
    assert!(matches!(
        fetched,
        Err(CoreError::ForwardingOff {
            kind: ForwardingOffKind::NotSetUp,
            ..
        })
    ));

    let set_up = core.set_up_forwarding("f-setup".into()).await.unwrap();
    let Forwarding::Waiting {
        address, sender, ..
    } = &set_up
    else {
        panic!("{set_up:?}");
    };
    assert_eq!(address, "otp+k3f9x2ab7q@vmate.aster0.dev");
    assert_eq!(sender, VTOP_SENDER);
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

    // Nothing forwarded yet: cannot mark done before confirmation arrives.
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(state(&checked), "waiting");
    assert_eq!(state(&core.mark_forwarding_done("f".into())), "waiting");
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
    assert_eq!(state(&checked), "waiting");
    let confirm = confirmation(&checked).unwrap();
    assert_eq!(confirm.code.as_deref(), Some("981234567"));
    assert_eq!(last_mail_at(&checked), Some(1_700_000_000));
    assert_eq!(last_otp_at(&checked), None);
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
    assert_eq!((state(&status), last_otp_at(&status)), ("on", Some(sent)));
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
    assert_eq!(state(&core.forwarding_status("f".into())), "off");
    assert!(!dir.join(FILE).exists());

    // An address the server lost is forgotten here.
    core.set_up_forwarding("f".into()).await.unwrap();
    edge.lock().unwrap().gone = true;
    let checked = core.check_forwarding("f".into()).await;
    assert!(matches!(
        checked,
        Err(CoreError::ForwardingOff {
            kind: ForwardingOffKind::AddressDeleted,
            ..
        })
    ));
    assert_eq!(state(&core.forwarding_status("f".into())), "off");

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
    assert_eq!(state(&checked), "waiting");
    assert_eq!(
        confirmation(&checked).unwrap().code.as_deref(),
        Some("981234567")
    );
    assert_eq!(last_otp_at(&checked), None);

    // Once Gmail's confirmation arrived, the student can mark setup done without waiting for an
    // OTP; the server's 24h kept confirmation doesn't re-appear on the next check.
    let done = core.mark_forwarding_done("f".into());
    assert_eq!(state(&done), "on");
    assert_eq!(last_otp_at(&done), None);
    edge.lock().unwrap().canned.insert(
        inbox.clone(),
        [(
            200,
            json!({ "sealed": null, "confirmation": kept, "lastOtpAt": null }),
        )]
        .into(),
    );
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(state(&checked), "on");

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
    assert_eq!(last_otp_at(&checked), Some(1_700_000_100));
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
    assert_eq!(state(&core.forwarding_status("f".into())), "off");
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
    // Nothing listens on port 9.
    let saved = json!({ "server": "http://127.0.0.1:9", "id": ID, "address": "a@b" });
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
    assert_eq!(state(&core.forwarding_status("f".into())), "waiting");
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

    core.sign_out_here("f-out");
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
    core.sign_out_here("f-out");
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
    core.sign_out_here("f-out");
    assert!(!dir.join(LEAVE_FILE).exists());
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn sign_out_ends_signed_out_here_when_the_server_fails() {
    let (core, edge, dir, _url) = forwarding_core("signout-all").await;
    edge.lock().unwrap().canned.insert(
        "auth:signOut".into(),
        [(
            500,
            json!({ "code": "server_error", "error": "Try again." }),
        )]
        .into(),
    );
    core.sign_out("f-all".into(), false).await;
    assert!(core.signed_in_user("f".into()).is_none());
    assert!(!dir.join(FILE).exists());
    // The address went first, while the identity lasted; the server sign-out was tried.
    let sent = paths(&edge);
    let leave = sent.iter().position(|p| p == "mail:leave").unwrap();
    let out = sent.iter().position(|p| p == "auth:signOut").unwrap();
    assert!(leave < out);
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
    let v2 = |id: &str| {
        json!({
            "server": url, "id": id, "address": format!("otp+{ID}@vmate.aster0.dev"),
            "owner_claimed": true, "ready": true, "last_mail_at": 5, "last_otp_at": 5,
        })
    };
    core.write(FILE, &v2(ID));
    // Not on the address yet: not shown as on, so background sync waits for the rejoin.
    assert_eq!(state(&core.forwarding_status("f".into())), "waiting");
    // An OTP wait before the rejoin: the server doesn't know this install on the address yet,
    // so it is rejoined there and then, not forgotten, and the inbox read once joined.
    edge.lock().unwrap().canned.insert(
        "mail:inbox".into(),
        [(
            404,
            json!({ "code": GONE, "error": "No such forwarding address." }),
        )]
        .into(),
    );
    let pulled = core.fetch_forwarded_otp("f".into(), 0).await;
    assert_eq!(pulled, Ok(None));
    assert_eq!(
        paths(&edge),
        ["auth:token", "mail:inbox", "mail:register", "mail:inbox"]
    );
    // The same address: its Gmail setup still holds.
    let kept = core.forwarding_status("f".into());
    assert_eq!((state(&kept), last_otp_at(&kept)), ("on", Some(5)));
    assert!(matches!(core.mailbox_state(), MailboxState::Joined { .. }));
    core.keep_forwarding("f".into()).await.unwrap();
    // Joined: opening the app again asks nothing.
    assert_eq!(paths(&edge).len(), 4);

    // The account holds another address: this phone moves to it and checks Gmail again.
    core.write(FILE, &v2("zzzzzzzzzz"));
    core.keep_forwarding("f".into()).await.unwrap();
    assert_eq!(core.saved_mailbox().unwrap().1, ID);
    let moved = core.forwarding_status("f".into());
    assert_eq!(state(&moved), "waiting");
    // The old address's "last OTP" isn't shown for the new one.
    assert_eq!(last_mail_at(&moved), None);
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
    assert_eq!(core.saved_mailbox().unwrap().1, "zzzzzzzzzz");
    assert_eq!(
        core.mailbox_state(),
        MailboxState::Joined {
            mailbox: Mailbox {
                server: url.clone(),
                id: "zzzzzzzzzz".into(),
                address: "otp+zzzzzzzzzz@vmate.aster0.dev".into(),
            },
            setup: Setup::NEW,
        }
    );
    // Deleted on another phone: no address to follow, so it is forgotten.
    edge.lock().unwrap().moved_to = None;
    let pulled = core.fetch_forwarded_otp("f".into(), 0).await;
    assert!(matches!(
        pulled,
        Err(CoreError::ForwardingOff {
            kind: ForwardingOffKind::AddressDeleted,
            ..
        })
    ));
    assert_eq!(core.mailbox_state(), MailboxState::Off);
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn forwarding_off_reports_device_removed_and_needs_sign_in() {
    let (core, edge, dir, _url) = forwarding_core("off-kinds").await;
    edge.lock().unwrap().canned.insert(
        "mail:inbox".into(),
        [(
            403,
            json!({ "code": REMOVED, "error": "This phone was removed." }),
        )]
        .into(),
    );
    let removed = core.check_forwarding("f".into()).await;
    assert!(matches!(
        removed,
        Err(CoreError::ForwardingOff {
            kind: ForwardingOffKind::DeviceRemoved,
            ..
        })
    ));
    assert_eq!(core.mailbox_state(), MailboxState::Off);

    // Without a server identity (e.g. v2 address before proving), inbox check reports NeedsSignIn.
    keep_token(&core, &_url);
    core.set_up_forwarding("f".into()).await.unwrap();
    core.forget_identity();
    let unauth = core.check_forwarding("f".into()).await;
    assert!(matches!(
        unauth,
        Err(CoreError::ForwardingOff {
            kind: ForwardingOffKind::NeedsSignIn,
            ..
        })
    ));
    let _ = fs::remove_dir_all(&dir);
}

fn mailbox(id: &str) -> Mailbox {
    Mailbox {
        server: "https://vmate.aster0.dev".into(),
        id: id.into(),
        address: format!("otp+{id}@vmate.aster0.dev"),
    }
}

fn confirm(sent_at: i64) -> Payload {
    Payload::GmailConfirm {
        code: Some("981234567".into()),
        link: Some("https://evil.example/mail/vf-x".into()),
        requester: None,
        sent_at,
    }
}

fn parse(stored: Value) -> Result<MailboxState, &'static str> {
    MailboxState::try_from(serde_json::from_value::<Stored>(stored).unwrap())
}

#[test]
fn stored_files_of_every_version_read() {
    let base = json!({ "server": "https://vmate.aster0.dev", "id": ID, "address": "a@b" });
    let with = |extra: Value| {
        let mut file = base.clone();
        file.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        file
    };
    let mailbox = Mailbox {
        server: "https://vmate.aster0.dev".into(),
        id: ID.into(),
        address: "a@b".into(),
    };
    // Imported from v2 before this install rejoined: nothing but where it lives.
    assert_eq!(
        parse(base.clone()),
        Ok(MailboxState::NeedsRejoin {
            mailbox: mailbox.clone(),
            kept: Setup::NEW
        })
    );
    // v2 with OTPs through: rejoining resumes it as working.
    assert_eq!(
        parse(with(
            json!({ "owner_claimed": true, "ready": true, "last_mail_at": 5, "last_otp_at": 5 })
        )),
        Ok(MailboxState::NeedsRejoin {
            mailbox: mailbox.clone(),
            kept: Setup::Active {
                last_mail_at: Some(5),
                last_otp_at: Some(5)
            }
        })
    );
    let confirmation = json!({ "code": "1", "link": null, "requester": null, "sent_at": 7 });
    let waiting = parse(with(
        json!({ "joined": true, "last_mail_at": 7, "confirmation": confirmation }),
    ))
    .unwrap();
    assert!(matches!(
        &waiting,
        MailboxState::Joined {
            setup: Setup::Waiting {
                confirmation: Some(_),
                last_mail_at: Some(7)
            },
            ..
        }
    ));
    // Ready with a confirmation left over (marked done, or an OTP after it): the confirmation
    // goes, as working forwarding has none to show.
    assert_eq!(
        parse(with(
            json!({ "joined": true, "ready": true, "confirmation": confirmation })
        )),
        Ok(MailboxState::Joined {
            mailbox: mailbox.clone(),
            setup: Setup::Active {
                last_mail_at: None,
                last_otp_at: None
            }
        })
    );
    // An OTP came through though `ready` wasn't set: working.
    assert!(matches!(
        parse(with(json!({ "joined": true, "last_otp_at": 9 }))),
        Ok(MailboxState::Joined {
            setup: Setup::Active { .. },
            ..
        })
    ));
    assert!(parse(json!({ "server": "s", "id": "", "address": "a@b" })).is_err());

    // Each state writes the file it reads back as, in the shape older apps read.
    for state in [
        waiting,
        MailboxState::NeedsRejoin {
            mailbox: mailbox.clone(),
            kept: Setup::Active {
                last_mail_at: Some(5),
                last_otp_at: Some(5),
            },
        },
        MailboxState::Joined {
            mailbox,
            setup: Setup::NEW,
        },
    ] {
        let stored = serde_json::to_value(state.stored().unwrap()).unwrap();
        assert!(stored.get("owner_claimed").is_none());
        assert!(stored["joined"].is_boolean() && stored["ready"].is_boolean());
        assert_eq!(parse(stored), Ok(state));
    }
    assert_eq!(MailboxState::Off.stored(), None);
}

fn readiness(forwarding: &Forwarding) -> ForwardingReadiness {
    match forwarding {
        Forwarding::Off { readiness, .. }
        | Forwarding::Waiting { readiness, .. }
        | Forwarding::On { readiness, .. } => *readiness,
    }
}

#[test]
fn readiness_follows_every_stage_of_setup() {
    assert_eq!(
        readiness(&view(&MailboxState::Off)),
        ForwardingReadiness::Off
    );
    let m = mailbox(ID);
    assert_eq!(
        readiness(&view(&MailboxState::Joined {
            mailbox: m.clone(),
            setup: Setup::NEW,
        })),
        ForwardingReadiness::NeedsConfirmation
    );
    // Kept from v2 (even if previously active) waits for rejoin before reporting ready.
    assert_eq!(
        readiness(&view(&MailboxState::NeedsRejoin {
            mailbox: m.clone(),
            kept: Setup::Active {
                last_mail_at: Some(5),
                last_otp_at: Some(5),
            },
        })),
        ForwardingReadiness::NeedsConfirmation
    );
    let confirmed = Setup::NEW.mail(&confirm(10));
    assert_eq!(
        readiness(&view(&MailboxState::Joined {
            mailbox: m.clone(),
            setup: confirmed.clone(),
        })),
        ForwardingReadiness::NeedsFilter
    );
    assert_eq!(
        readiness(&view(&MailboxState::Joined {
            mailbox: m,
            setup: confirmed.mark_done(),
        })),
        ForwardingReadiness::Ready
    );
}

#[test]
fn setup_moves_only_forward() {
    // Gmail's confirmation shows, with only a Gmail link kept.
    let waiting = Setup::NEW.mail(&confirm(10));
    let Setup::Waiting {
        confirmation: Some(shown),
        last_mail_at: Some(10),
    } = &waiting
    else {
        panic!("{waiting:?}");
    };
    assert_eq!(shown.link, None);
    // Nothing reached the address: not done yet.
    assert_eq!(Setup::NEW.mark_done(), Setup::NEW);
    let done = waiting.clone().mark_done();
    assert_eq!(
        done,
        Setup::Active {
            last_mail_at: Some(10),
            last_otp_at: None
        }
    );
    // The server's kept confirmation, read again once working, isn't shown again.
    assert_eq!(done.clone().mail(&confirm(10)), done);
    let otp = Payload::Otp {
        code: "401295".into(),
        sent_at: 20,
    };
    let working = waiting.mail(&otp);
    assert_eq!(
        working,
        Setup::Active {
            last_mail_at: Some(20),
            last_otp_at: Some(20)
        }
    );
    // An older OTP time from the server doesn't move it back.
    assert_eq!(working.clone().otp_at(15), working);
}

#[test]
fn an_address_kept_from_v2_is_not_on_until_rejoined() {
    let active = Setup::Active {
        last_mail_at: Some(5),
        last_otp_at: Some(5),
    };
    let kept = MailboxState::NeedsRejoin {
        mailbox: mailbox(ID),
        kept: active.clone(),
    };
    assert!(matches!(
        view(&kept),
        Forwarding::Waiting {
            last_mail_at: None,
            confirmation: None,
            ..
        }
    ));
    // Rejoined on the same address: the setup it had.
    let rejoined = kept.rejoined(ID, mailbox(ID)).unwrap();
    assert!(matches!(view(&rejoined), Forwarding::On { .. }));
    // On another: set up again.
    assert_eq!(
        kept.rejoined(ID, mailbox("zzzzzzzzzz")),
        Some(MailboxState::Joined {
            mailbox: mailbox("zzzzzzzzzz"),
            setup: Setup::NEW
        })
    );
    // Turned off, or another address, meanwhile: left as it is.
    assert_eq!(MailboxState::Off.rejoined(ID, mailbox(ID)), None);
    assert_eq!(kept.rejoined("other", mailbox(ID)), None);
    // Mail for another address doesn't touch this one.
    assert_eq!(
        kept.clone().with_setup("other", |setup| setup.otp_at(99)),
        kept
    );
}

#[tokio::test]
async fn a_setup_finished_after_a_sign_out_is_queued_to_leave() {
    let (core, _edge, dir, _url) = forwarding_core("signed-out").await;
    core.forget_forwarding(ID);
    // The sign-out came while `mail:register` was answering: the address isn't saved for the
    // next student, and this install leaves it.
    let slot = core.current().unwrap();
    core.install(None);
    assert_eq!(
        core.save_new_mailbox(&slot, mailbox(ID)),
        Err(CoreError::NotSignedIn)
    );
    assert_eq!(core.mailbox_state(), MailboxState::Off);
    let queue = core.read::<Vec<Leave>>(LEAVE_FILE).unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].id, ID);
    // Still signed in, with an address saved meanwhile: that one stays.
    core.install(Some(slot.clone()));
    core.save_new_mailbox(&slot, mailbox("zzzzzzzzzz")).unwrap();
    core.save_new_mailbox(&slot, mailbox(ID)).unwrap();
    assert_eq!(core.saved_mailbox().unwrap().1, "zzzzzzzzzz");
    let _ = fs::remove_dir_all(&dir);
}

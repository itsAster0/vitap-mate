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
        Forwarding::Waiting { confirmation, .. } | Forwarding::On { confirmation, .. } => {
            confirmation.clone()
        }
        Forwarding::Off { .. } => None,
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
        json!({ "kind": "gmail_confirm", "code": "981234567", "link": null, "requester": "a.22bce0001@vitapstudent.ac.in", "sentAt": 1_700_000_000 }),
    );
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(state(&checked), "waiting");
    let confirm = confirmation(&checked).unwrap();
    // Proved server identity holds VTOP's regno even before account.json is saved.
    assert_eq!(confirm.from, ConfirmFrom::CollegeInbox);
    // A VTOP username that isn't the regno: the college mail names the saved regno.
    let slot = core.current().unwrap();
    core.rename_student(&slot, "STUDENT314");
    let account = crate::Account {
        username: "STUDENT314".into(),
        password: "pw".into(),
        registration_number: Some("22BCE0001".into()),
    };
    core.write_account(&account, false);
    let renamed = confirmation(&core.forwarding_status("f".into())).unwrap();
    assert_eq!(renamed.from, ConfirmFrom::CollegeInbox);
    core.rename_student(&slot, "22BCE0001");
    assert_eq!(
        confirm.confirm_by,
        ConfirmBy::Code {
            code: "981234567".into()
        }
    );
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
        confirmation(&checked).unwrap().confirm_by.code(),
        Some("981234567")
    );
    assert_eq!(last_otp_at(&checked), None);

    // While the confirmation hasn't been opened yet, mark_forwarding_done refuses.
    assert_eq!(state(&core.mark_forwarding_done("f".into())), "waiting");
    // Once confirmed in Gmail, the student can mark setup done without waiting for an OTP; the
    // server's 24h kept confirmation doesn't re-appear on the next check.
    assert_eq!(
        step(&core.opened_forwarding_confirm("f".into())),
        Some(SetupStep::Filter)
    );
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
fn tells_who_confirmed() {
    let profile: StudentProfile = serde_json::from_value(json!({
        "name": "A", "registration_number": "22BCE0001", "vit_email": "odd.name@vitapstudent.ac.in",
        "program": "", "school": "", "photo": "", "update_time": 0,
        "sections": [{ "title": "PERSONAL", "groups": [{ "title": "", "fields": [
            { "label": "Email", "value": "Me@Gmail.com, other@x.in" } ] }] }],
    }))
    .unwrap();
    let p = Some(&profile);
    let r = Some("22BCE0001");
    let college = ConfirmFrom::CollegeInbox;
    assert_eq!(
        confirm_from(Some("a.22bce0001@vitapstudent.ac.in"), r, None),
        college
    );
    assert_eq!(
        confirm_from(Some(" Ann <A.22BCE0001@VITAPSTUDENT.AC.IN> "), r, None),
        college
    );
    assert_eq!(
        confirm_from(Some("odd.name@vitapstudent.ac.in"), None, p),
        college
    );
    let mut padded = profile.clone();
    padded.vit_email = "  odd.name@vitapstudent.ac.in \n".into();
    assert_eq!(
        confirm_from(Some("odd.name@vitapstudent.ac.in"), None, Some(&padded)),
        college
    );
    assert_eq!(
        confirm_from(Some("b.22bce0002@vitapstudent.ac.in"), r, p),
        ConfirmFrom::Unknown
    );
    assert_eq!(
        confirm_from(Some("me@gmail.com"), r, p),
        ConfirmFrom::OwnOther
    );
    assert_eq!(
        confirm_from(Some("someone@gmail.com"), r, p),
        ConfirmFrom::Unknown
    );
    assert_eq!(
        confirm_from(Some("someone@gmail.com"), r, None),
        ConfirmFrom::Unknown
    );
    assert_eq!(confirm_from(None, r, p), ConfirmFrom::NotSaid);
    assert_eq!(confirm_from(Some("  "), r, p), ConfirmFrom::NotSaid);
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
    // so it is rejoined there and then via mail:join (never mail:register), not forgotten, and
    // the inbox read once joined.
    let joined_reply = json!({ "id": ID, "address": format!("otp+{ID}@vmate.aster0.dev") });
    {
        let mut edge = edge.lock().unwrap();
        edge.canned.insert(
            "mail:inbox".into(),
            [(
                404,
                json!({ "code": ServerCode::MailboxNotFound.as_str(), "error": "No such forwarding address." }),
            )]
            .into(),
        );
        edge.canned.insert(
            "mail:join".into(),
            [(200, joined_reply.clone()), (200, joined_reply)].into(),
        );
    }
    let pulled = core.fetch_forwarded_otp("f".into(), 0).await;
    assert_eq!(pulled, Ok(None));
    assert_eq!(
        paths(&edge),
        ["auth:token", "mail:inbox", "mail:join", "mail:inbox"]
    );
    assert_eq!(edge.lock().unwrap().requests[2].body, json!({}));
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
            json!({ "code": ServerCode::DeviceRemoved.as_str(), "error": "This phone was removed." }),
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
                last_otp_at: Some(5),
                confirmation: None,
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
                last_otp_at: None,
                confirmation: None,
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
                confirmation: None,
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
                confirmation: None,
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
fn every_stage_says_what_it_is_good_for() {
    use ForwardingReadiness::*;
    let uses = |readiness: ForwardingReadiness| {
        let ForwardingUses {
            usable_for_autofill,
            ready_for_sync,
        } = readiness.uses();
        (usable_for_autofill, ready_for_sync)
    };
    assert_eq!(uses(Off), (false, false));
    assert_eq!(uses(NeedsConfirmation), (true, false));
    assert_eq!(uses(NeedsFilter), (true, true));
    assert_eq!(uses(Ready), (true, true));
    let Forwarding::On { uses: on, .. } = view(&MailboxState::Joined {
        mailbox: mailbox(ID),
        setup: Setup::NEW.mail(&confirm(10)).mark_done(),
    }) else {
        panic!("not on");
    };
    assert_eq!(on, Ready.uses());
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
    assert_eq!(
        shown.confirm_by,
        ConfirmBy::Code {
            code: "981234567".into()
        },
        "not Gmail's link"
    );
    // Nothing reached the address: not done yet.
    assert_eq!(Setup::NEW.mark_done(), Setup::NEW);
    let done = waiting.clone().mark_done();
    assert_eq!(
        done,
        Setup::Active {
            last_mail_at: Some(10),
            last_otp_at: None,
            confirmation: None,
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
            last_otp_at: Some(20),
            confirmation: None,
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
        confirmation: None,
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
            step: SetupStep::Rejoining,
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
    let username = slot.read().await.account.username.clone();
    core.uninstall();
    assert_eq!(
        core.save_new_mailbox(&slot, mailbox(ID)),
        Err(CoreError::NotSignedIn)
    );
    assert_eq!(core.mailbox_state(), MailboxState::Off);
    let queue = core.read::<Vec<Leave>>(LEAVE_FILE).unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].id, ID);
    // Still signed in, with an address saved meanwhile: that one stays.
    core.put(crate::Signed::In {
        slot: slot.clone(),
        username,
    });
    core.save_new_mailbox(&slot, mailbox("zzzzzzzzzz")).unwrap();
    core.save_new_mailbox(&slot, mailbox(ID)).unwrap();
    assert_eq!(core.saved_mailbox().unwrap().1, "zzzzzzzzzz");
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn a_phone_removed_on_the_server_forgets_its_address() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-removed-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    fs::write(dir.join(SERVER_FILE), url.clone() + "\n").unwrap();
    let core = signed_in(&dir, url.clone());
    keep_token(&core, &url);
    core.write(
        FILE,
        &json!({
            "server": url, "id": ID, "address": format!("otp+{ID}@vmate.aster0.dev"),
            "owner_claimed": true, "ready": true,
        }),
    );
    edge.lock().unwrap().canned.insert(
        "mail:inbox".into(),
        [(
            403,
            json!({ "code": ServerCode::DeviceRemoved.as_str(), "error": "Removed." }),
        )]
        .into(),
    );
    let pulled = core.fetch_forwarded_otp("f".into(), 0).await;
    assert!(
        matches!(&pulled, Err(CoreError::ForwardingOff { kind: ForwardingOffKind::DeviceRemoved, reason }) if reason.contains("removed")),
        "{pulled:?}"
    );
    assert!(core.saved_mailbox().is_none());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_kept_confirmation_keeps_its_format_and_drops_a_foreign_link() {
    let file = json!({
        "server": "s", "id": "i", "address": "a@b", "joined": true, "ready": false,
        "last_mail_at": 10, "last_otp_at": null,
        "confirmation": {
            "code": "981234567",
            "link": "https://mail-settings.google.com/mail/vf-abc",
            "requester": "me@vitapstudent.ac.in",
            "sent_at": 10,
        },
    });
    let state = parse(file.clone()).unwrap();
    let MailboxState::Joined {
        setup: Setup::Waiting {
            confirmation: Some(kept),
            ..
        },
        ..
    } = &state
    else {
        panic!("{state:?}");
    };
    assert!(matches!(kept.confirm_by, ConfirmBy::CodeAndLink { .. }));
    assert_eq!(serde_json::to_value(state.stored().unwrap()).unwrap(), file);
    // A link that isn't Gmail's, from a file an older app wrote, is dropped once read.
    let mut foreign = file;
    foreign["confirmation"]["link"] = json!("https://evil.example/mail/vf-x");
    foreign["confirmation"]["code"] = json!(null);
    let MailboxState::Joined {
        setup: Setup::Waiting {
            confirmation: Some(kept),
            ..
        },
        ..
    } = parse(foreign).unwrap()
    else {
        panic!("not waiting");
    };
    assert_eq!(kept.confirm_by, ConfirmBy::Neither);
}

#[test]
fn signed_out_of_the_server_the_address_waits_to_rejoin() {
    let mailbox = Mailbox {
        server: "https://s".into(),
        id: ID.into(),
        address: format!("otp+{ID}@vmate.aster0.dev"),
    };
    let setup = Setup::Active {
        last_mail_at: Some(1),
        last_otp_at: Some(2),
        confirmation: None,
    };
    let joined = MailboxState::Joined {
        mailbox: mailbox.clone(),
        setup: setup.clone(),
    };
    let waiting = MailboxState::NeedsRejoin {
        mailbox,
        kept: setup,
    };
    assert_eq!(joined.unjoined(), waiting);
    assert_eq!(waiting.clone().unjoined(), waiting);
    assert_eq!(MailboxState::Off.unjoined(), MailboxState::Off);
}

fn step(forwarding: &Forwarding) -> Option<SetupStep> {
    match forwarding {
        Forwarding::Waiting { step, .. } => Some(*step),
        _ => None,
    }
}

fn hint(forwarding: &Forwarding) -> Option<ForwardingHint> {
    match forwarding {
        Forwarding::On { hint, .. } => hint.clone(),
        _ => None,
    }
}

#[test]
fn step_follows_gmail_mail_and_the_confirm_tap() {
    let fresh = MailboxState::Joined {
        mailbox: mailbox(ID),
        setup: Setup::NEW,
    };
    let none = Progress::default();
    assert_eq!(step(&view_with(&fresh, &none)), Some(SetupStep::AddAddress));
    let confirmed = fresh.clone().with_setup(ID, |s| s.mail(&confirm(100)));
    assert_eq!(
        step(&view_with(&confirmed, &none)),
        Some(SetupStep::Confirm)
    );
    let opened = Progress {
        id: ID.into(),
        confirm_opened_at: Some(100),
        ..Progress::default()
    };
    assert_eq!(
        step(&view_with(&confirmed, &opened)),
        Some(SetupStep::Filter)
    );
    // A newer confirmation (added again in Gmail) asks again.
    let again = confirmed.with_setup(ID, |s| s.mail(&confirm(200)));
    assert_eq!(step(&view_with(&again, &opened)), Some(SetupStep::Confirm));
    // Another address's progress doesn't count.
    let other = Progress {
        id: "other".into(),
        confirm_opened_at: Some(999),
        ..Progress::default()
    };
    assert_eq!(step(&view_with(&again, &other)), Some(SetupStep::Confirm));
    // A kept address waiting to rejoin isn't a fresh setup.
    let rejoining = MailboxState::NeedsRejoin {
        mailbox: mailbox(ID),
        kept: Setup::NEW,
    };
    assert_eq!(
        step(&view_with(&rejoining, &none)),
        Some(SetupStep::Rejoining)
    );
}

#[test]
fn the_typed_otp_hint_never_changes_the_state() {
    let on = MailboxState::Joined {
        mailbox: mailbox(ID),
        setup: Setup::Active {
            last_mail_at: Some(50),
            last_otp_at: Some(50),
            confirmation: None,
        },
    };
    let typed = Progress {
        id: ID.into(),
        typed_otp_at: Some(60),
        ..Progress::default()
    };
    let v = view_with(&on, &typed);
    assert_eq!(hint(&v), Some(ForwardingHint::TypedOtp { at: 60 }));
    assert_eq!(readiness(&v), ForwardingReadiness::Ready);
    // A forwarded OTP after it, or a dismiss, hides it; a later typed one shows again.
    let forwarded = on.clone().with_setup(ID, |s| s.otp_at(70));
    assert_eq!(hint(&view_with(&forwarded, &typed)), None);
    let dismissed = Progress {
        dismissed_at: Some(60),
        ..typed.clone()
    };
    assert_eq!(hint(&view_with(&on, &dismissed)), None);
    let later = Progress {
        typed_otp_at: Some(80),
        ..dismissed
    };
    assert_eq!(
        hint(&view_with(&on, &later)),
        Some(ForwardingHint::TypedOtp { at: 80 })
    );
}

#[tokio::test]
async fn progress_lives_beside_forwarding_json_and_goes_with_it() {
    let (core, edge, dir, _url) = forwarding_core("progress").await;
    // Not set up yet: a typed OTP is not noted.
    assert_eq!(hint(&core.note_typed_otp("f".into(), 10)), None);
    seal(
        &core,
        &edge,
        json!({ "kind": "gmail_confirm", "code": "1", "link": null, "requester": null, "sentAt": 5 }),
    );
    core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(
        step(&core.opened_forwarding_confirm("f".into())),
        Some(SetupStep::Filter)
    );
    core.mark_forwarding_done("f".into());
    assert_eq!(
        hint(&core.note_typed_otp("f".into(), 20)),
        Some(ForwardingHint::TypedOtp { at: 20 })
    );
    assert_eq!(hint(&core.dismiss_forwarding_hint("f".into())), None);
    // forwarding.json still parses as the old Stored (an older app reads it).
    let stored: Stored = serde_json::from_slice(&fs::read(dir.join(FILE)).unwrap()).unwrap();
    assert!(stored.ready);
    assert!(dir.join(PROGRESS_FILE).exists());
    core.turn_off_forwarding("f".into()).await.unwrap();
    assert!(!dir.join(PROGRESS_FILE).exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_forwarded_copy_of_the_typed_round_clears_the_hint() {
    // The typed OTP is noted at VTOP's time for the round; its forwarded copy may carry a
    // `Date:` a few seconds before that.
    let on = MailboxState::Joined {
        mailbox: mailbox(ID),
        setup: Setup::Active {
            last_mail_at: Some(50),
            last_otp_at: Some(50),
            confirmation: None,
        },
    };
    let typed = Progress {
        id: ID.into(),
        typed_otp_at: Some(100),
        ..Progress::default()
    };
    let copy = on.with_setup(ID, |s| s.otp_at(98));
    assert_eq!(hint(&view_with(&copy, &typed)), None);
}

#[tokio::test]
async fn a_newer_confirmation_while_on_is_shown_until_opened_or_superseded() {
    let (core, edge, dir, _url) = forwarding_core("on-confirm").await;
    let seal_box = |payload: Value| {
        mail_seal::seal(&core.device().seal_key, ID, payload.to_string().as_bytes()).unwrap()
    };
    // Initial OTP puts forwarding On at t = 100.
    edge.lock().unwrap().canned.insert(
        "mail:inbox".into(),
        [(
            200,
            json!({ "sealed": null, "confirmation": null, "confirmedAt": 90, "lastOtpAt": 100 }),
        )]
        .into(),
    );
    let status = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(state(&status), "on");
    assert_eq!(confirmation(&status), None);

    // A newer Gmail confirmation arrives at t = 200 in `sealed`, with `confirmedAt: 200` and
    // older `lastOtpAt: 100`: neither `confirmedAt` nor older `lastOtpAt` swallows it.
    let newer = seal_box(json!({
        "kind": "gmail_confirm",
        "code": "123456789",
        "link": "https://mail-settings.google.com/mail/vf-abc",
        "requester": "a.22bce0001@vitapstudent.ac.in",
        "sentAt": 200,
    }));
    edge.lock().unwrap().canned.insert(
        "mail:inbox".into(),
        [(
            200,
            json!({ "sealed": newer, "confirmation": null, "confirmedAt": 200, "lastOtpAt": 100 }),
        )]
        .into(),
    );
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(state(&checked), "on");
    let shown = confirmation(&checked).unwrap();
    assert_eq!(shown.sent_at, 200);
    assert_eq!(shown.from, ConfirmFrom::CollegeInbox);
    // Persists in progress file while forwarding.json stays compatible with old Stored.
    let stored: Stored = serde_json::from_slice(&fs::read(dir.join(FILE)).unwrap()).unwrap();
    assert!(stored.ready);
    assert!(stored.confirmation.is_none());
    assert_eq!(
        confirmation(&core.forwarding_status("f".into()))
            .unwrap()
            .sent_at,
        200
    );

    // Opening the confirmation hides it, and re-reading the same kept confirmation keeps it hidden.
    let after_open = core.opened_forwarding_confirm("f".into());
    assert_eq!(state(&after_open), "on");
    assert_eq!(confirmation(&after_open), None);
    edge.lock().unwrap().canned.insert(
        "mail:inbox".into(),
        [(
            200,
            json!({ "sealed": null, "confirmation": newer, "confirmedAt": 200, "lastOtpAt": 100 }),
        )]
        .into(),
    );
    assert_eq!(
        confirmation(&core.check_forwarding("f".into()).await.unwrap()),
        None
    );

    // A still-newer confirmation at t = 300 shows again, then an OTP at t = 350 supersedes it.
    let newest = seal_box(json!({
        "kind": "gmail_confirm",
        "code": "222222222",
        "link": null,
        "requester": "other@gmail.com",
        "sentAt": 300,
    }));
    edge.lock().unwrap().canned.insert(
        "mail:inbox".into(),
        [
            (
                200,
                json!({ "sealed": null, "confirmation": newest, "confirmedAt": 300, "lastOtpAt": 100 }),
            ),
            (
                200,
                json!({ "sealed": null, "confirmation": newest, "confirmedAt": 300, "lastOtpAt": 350 }),
            ),
        ]
        .into(),
    );
    assert_eq!(
        confirmation(&core.check_forwarding("f".into()).await.unwrap())
            .unwrap()
            .sent_at,
        300
    );
    assert_eq!(
        confirmation(&core.check_forwarding("f".into()).await.unwrap()),
        None
    );

    // A fresh Waiting phone seeing a newer confirmation alongside an older lastOtpAt lands On
    // with that confirmation still surfaced.
    let mut fresh = Setup::NEW;
    fresh = fresh.mail(&confirm(200));
    fresh = fresh.otp_at(100);
    let shown = confirmation(&view(&MailboxState::Joined {
        mailbox: mailbox(ID),
        setup: fresh,
    }));
    assert_eq!(shown.map(|c| c.sent_at), Some(200));
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn ignoring_a_confirmation_stops_blocking_confirm_until_a_newer_one() {
    let (core, edge, dir, _url) = forwarding_core("ignore").await;
    // College Gmail confirmed and opened at t = 100 -> Filter.
    seal(
        &core,
        &edge,
        json!({ "kind": "gmail_confirm", "code": "111", "link": null, "requester": "a.22bce0001@vitapstudent.ac.in", "sentAt": 100 }),
    );
    core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(
        step(&core.opened_forwarding_confirm("f".into())),
        Some(SetupStep::Filter)
    );
    // Wrong-Gmail confirmation arrives at t = 200 -> pulls step back to Confirm.
    seal(
        &core,
        &edge,
        json!({ "kind": "gmail_confirm", "code": "222", "link": null, "requester": "wrong@gmail.com", "sentAt": 200 }),
    );
    let pulled = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(step(&pulled), Some(SetupStep::Confirm));
    assert_eq!(confirmation(&pulled).unwrap().from, ConfirmFrom::Unknown);
    // Ignoring it is local-only and moves step back to Filter.
    let before = edge.lock().unwrap().requests.len();
    let ignored = core.ignore_forwarding_confirm("f".into());
    assert_eq!(edge.lock().unwrap().requests.len(), before);
    assert_eq!(step(&ignored), Some(SetupStep::Filter));
    assert_eq!(core.progress().ignored_confirm_at, Some(200));
    // A newer confirmation at t = 300 shows again at Confirm.
    seal(
        &core,
        &edge,
        json!({ "kind": "gmail_confirm", "code": "333", "link": null, "requester": "wrong2@gmail.com", "sentAt": 300 }),
    );
    assert_eq!(
        step(&core.check_forwarding("f".into()).await.unwrap()),
        Some(SetupStep::Confirm)
    );
    // Ignore t = 300, mark done -> On; then a newer confirmation at t = 400 while On can also
    // be ignored.
    core.ignore_forwarding_confirm("f".into());
    assert_eq!(state(&core.mark_forwarding_done("f".into())), "on");
    seal(
        &core,
        &edge,
        json!({ "kind": "gmail_confirm", "code": "444", "link": null, "requester": "wrong3@gmail.com", "sentAt": 400 }),
    );
    assert_eq!(
        confirmation(&core.check_forwarding("f".into()).await.unwrap())
            .unwrap()
            .sent_at,
        400
    );
    assert_eq!(
        confirmation(&core.ignore_forwarding_confirm("f".into())),
        None
    );
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn pull_passes_from_id_when_joined_and_forgets_on_null_join() {
    let (core, edge, dir, _url) = forwarding_core("from-id").await;
    // Joined phone removed from its mailbox on purpose: mail:inbox says MailboxNotFound,
    // mail:join receives fromId = ID and answers null -> forgotten as AddressDeleted.
    edge.lock().unwrap().gone = true;
    let err = core.check_forwarding("f".into()).await;
    assert!(matches!(
        err,
        Err(CoreError::ForwardingOff {
            kind: ForwardingOffKind::AddressDeleted,
            ..
        })
    ));
    assert_eq!(core.mailbox_state(), MailboxState::Off);
    {
        let edge = edge.lock().unwrap();
        let join = edge
            .requests
            .iter()
            .rev()
            .find(|r| r.path == "mail:join")
            .unwrap();
        assert_eq!(join.body, json!({ "fromId": ID }));
    }
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn confirmed_at_advances_to_filter_and_missing_field_is_tolerated() {
    let (core, edge, dir, _url) = forwarding_core("confirmed-at").await;
    // Older server without `confirmedAt`: tolerated, stays at AddAddress / NeedsConfirmation.
    edge.lock().unwrap().canned.insert(
        "mail:inbox".into(),
        [(200, json!({ "sealed": null }))].into(),
    );
    let old = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(step(&old), Some(SetupStep::AddAddress));
    assert_eq!(readiness(&old), ForwardingReadiness::NeedsConfirmation);

    // Server dropped the 24h confirmation but kept `confirmedAt`: phone lands on Filter and can
    // mark done without waiting for a VTOP OTP.
    edge.lock().unwrap().canned.insert(
        "mail:inbox".into(),
        [(
            200,
            json!({ "sealed": null, "confirmation": null, "confirmedAt": 1_700_000_000, "lastOtpAt": null }),
        )]
        .into(),
    );
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(step(&checked), Some(SetupStep::Filter));
    assert_eq!(readiness(&checked), ForwardingReadiness::NeedsFilter);
    assert_eq!(last_mail_at(&checked), Some(1_700_000_000));
    assert_eq!(confirmation(&checked), None);
    let done = core.mark_forwarding_done("f".into());
    assert_eq!(state(&done), "on");
    assert_eq!(last_mail_at(&done), Some(1_700_000_000));
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn mail_regno_falls_back_without_keystore_and_unknown_identity_is_not_said() {
    // Pure confirm_from: with nothing to compare against, NotSaid (never Unknown).
    assert_eq!(
        confirm_from(Some("a.22bce0001@vitapstudent.ac.in"), None, None),
        ConfirmFrom::NotSaid
    );
    assert_eq!(
        confirm_from(Some("someone@gmail.com"), Some("   "), None),
        ConfirmFrom::NotSaid
    );

    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-regno-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    let core = signed_in(&dir, url.clone());
    // Seed a joined waiting mailbox with a college-looking requester, and NO regno anywhere.
    core.write(
        FILE,
        &json!({
            "server": url, "id": ID, "address": format!("otp+{ID}@vmate.aster0.dev"),
            "joined": true, "ready": false, "last_mail_at": 10,
            "confirmation": {
                "code": "1", "link": null,
                "requester": "a.22bce0001@vitapstudent.ac.in", "sent_at": 10
            }
        }),
    );
    // Username is "22BCE0001", yet with no VTOP regno saved it is never taken for one -> NotSaid.
    assert_eq!(
        confirmation(&core.forwarding_status("f".into()))
            .unwrap()
            .from,
        ConfirmFrom::NotSaid
    );

    // Fallback 4: device.refresh on another server is ignored; on this server it matches.
    core.keep_refresh(Some(crate::remote::backend::identity::Refresh {
        server: "https://other.example".into(),
        token: "vrt".into(),
        regno: "22BCE0001".into(),
    }));
    assert_eq!(
        confirmation(&core.forwarding_status("f".into()))
            .unwrap()
            .from,
        ConfirmFrom::NotSaid
    );
    keep_token(&core, &url);
    assert_eq!(
        confirmation(&core.forwarding_status("f".into()))
            .unwrap()
            .from,
        ConfirmFrom::CollegeInbox
    );
    core.forget_identity();

    // Fallback 3: cached profile.registration_number.
    let profile_path = dir.join(PAGES).join(spec::PROFILE.file);
    fs::create_dir_all(profile_path.parent().unwrap()).unwrap();
    fs::write(
        &profile_path,
        json!({
            "name": "A", "registration_number": "22BCE0001", "vit_email": "",
            "program": "", "school": "", "photo": "", "update_time": 0, "sections": []
        })
        .to_string(),
    )
    .unwrap();
    assert_eq!(
        confirmation(&core.forwarding_status("f".into()))
            .unwrap()
            .from,
        ConfirmFrom::CollegeInbox
    );
    fs::remove_file(&profile_path).unwrap();

    // Fallback 2: session.json regno matched to student (mismatched username ignored).
    fs::write(
        dir.join(crate::SESSION),
        json!({ "username": "OTHER", "registration_number": "22BCE0001" }).to_string(),
    )
    .unwrap();
    assert_eq!(
        confirmation(&core.forwarding_status("f".into()))
            .unwrap()
            .from,
        ConfirmFrom::NotSaid
    );
    fs::write(
        dir.join(crate::SESSION),
        json!({ "username": "22BCE0001", "registration_number": "22BCE0001" }).to_string(),
    )
    .unwrap();
    assert_eq!(
        confirmation(&core.forwarding_status("f".into()))
            .unwrap()
            .from,
        ConfirmFrom::CollegeInbox
    );
    fs::remove_file(dir.join(crate::SESSION)).unwrap();

    // Fallback 1: account.json with an unreadable sealed password (no Keystore opened) still
    // supplies registration_number for the signed-in student.
    fs::write(
        dir.join(crate::ACCOUNT),
        json!({
            "username": "22BCE0001",
            "sealed": "not-openable-without-keystore",
            "registration_number": "22BCE0001"
        })
        .to_string(),
    )
    .unwrap();
    assert_eq!(
        confirmation(&core.forwarding_status("f".into()))
            .unwrap()
            .from,
        ConfirmFrom::CollegeInbox
    );

    // Another student signing in over the saved one (account.json still belongs to 22BCE0001):
    // neither account.json nor device.refresh leaks 22BCE0001 to the incoming student.
    keep_token(&core, &url);
    let slot = core.current().unwrap();
    core.rename_student(&slot, "22BCE9999");
    assert_eq!(
        confirmation(&core.forwarding_status("f".into()))
            .unwrap()
            .from,
        ConfirmFrom::NotSaid
    );
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn needs_rejoin_promotes_on_inbox_leaves_unauth_locally_and_rejoins_on_check() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-rejoin-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    let core = signed_in(&dir, url.clone());
    keep_token(&core, &url);
    let write_rejoin = || {
        core.write(
            FILE,
            &json!({
                "server": url, "id": ID, "address": format!("otp+{ID}@vmate.aster0.dev"),
                "joined": false, "ready": true, "last_mail_at": 5, "last_otp_at": 5,
            }),
        );
    };

    // (a) Ok(inbox) while NeedsRejoin promotes straight to Joined (keeping setup).
    write_rejoin();
    assert!(matches!(
        core.mailbox_state(),
        MailboxState::NeedsRejoin { .. }
    ));
    core.pull("f").await.unwrap();
    assert!(matches!(
        core.mailbox_state(),
        MailboxState::Joined {
            setup: Setup::Active {
                last_otp_at: Some(5),
                ..
            },
            ..
        }
    ));

    // (b) Check on NeedsRejoin when identity is shared rejoins via mail:join (never register),
    // and when mail:join returns null it forgets and reports AddressDeleted.
    write_rejoin();
    edge.lock().unwrap().canned.insert(
        "mail:join".into(),
        [(
            200,
            json!({ "id": ID, "address": format!("otp+{ID}@vmate.aster0.dev") }),
        )]
        .into(),
    );
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(state(&checked), "on");
    assert!(!paths(&edge).contains(&"mail:register".to_string()));
    write_rejoin();
    edge.lock()
        .unwrap()
        .canned
        .insert("mail:join".into(), [(200, Value::Null)].into());
    let deleted = core.check_forwarding("f".into()).await;
    assert!(matches!(
        deleted,
        Err(CoreError::ForwardingOff {
            kind: ForwardingOffKind::AddressDeleted,
            ..
        })
    ));
    assert_eq!(core.mailbox_state(), MailboxState::Off);

    // (c) Rejoined::Later (no identity / regno not shared) never forgets the address and reports
    // NeedsSignIn; turning off locally succeeds while deleting everywhere fails.
    write_rejoin();
    core.forget_identity();
    let unauth = core.pull("f").await;
    assert!(matches!(
        unauth,
        Err(CoreError::ForwardingOff {
            kind: ForwardingOffKind::NeedsSignIn,
            ..
        })
    ));
    assert!(matches!(
        core.mailbox_state(),
        MailboxState::NeedsRejoin { .. }
    ));
    // Deleting everywhere needs the server and keeps the address.
    assert!(core.leave_forwarding("f", true).await.is_err());
    assert!(matches!(
        core.mailbox_state(),
        MailboxState::NeedsRejoin { .. }
    ));
    // Turning off on this phone forgets locally on Unauthenticated while NeedsRejoin.
    core.turn_off_forwarding("f".into()).await.unwrap();
    assert_eq!(core.mailbox_state(), MailboxState::Off);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn older_confirmation_does_not_overwrite_newer() {
    let newer = Setup::NEW.mail(&confirm(200));
    let after = newer.clone().mail(&confirm(100));
    assert_eq!(after, newer);

    let active = Setup::Active {
        last_mail_at: Some(100),
        last_otp_at: Some(100),
        confirmation: None,
    }
    .mail(&confirm(200));
    let after_old = active.clone().mail(&confirm(150));
    assert_eq!(after_old, active);
}

#[tokio::test]
async fn progress_timestamps_keep_max_ignore_nonpositive_and_saturate() {
    let (core, edge, dir, _url) = forwarding_core("ts").await;
    seal(
        &core,
        &edge,
        json!({ "kind": "gmail_confirm", "code": "1", "link": null, "requester": null, "sentAt": 100 }),
    );
    core.check_forwarding("f".into()).await.unwrap();
    core.opened_forwarding_confirm("f".into());
    core.ignore_forwarding_confirm("f".into());
    core.mark_forwarding_done("f".into());

    // Non-positive typed_otp_at is ignored.
    assert_eq!(hint(&core.note_typed_otp("f".into(), 0)), None);
    assert_eq!(hint(&core.note_typed_otp("f".into(), -42)), None);
    // Max is kept for typed_otp_at and dismissed_at.
    assert_eq!(
        hint(&core.note_typed_otp("f".into(), 200)),
        Some(ForwardingHint::TypedOtp { at: 200 })
    );
    assert_eq!(
        hint(&core.note_typed_otp("f".into(), 150)),
        Some(ForwardingHint::TypedOtp { at: 200 })
    );
    core.dismiss_forwarding_hint("f".into());
    assert_eq!(core.progress().dismissed_at, Some(200));
    // Re-confirming/ignoring an older timestamp never lowers confirm_opened_at or ignored_confirm_at.
    core.write(
        PROGRESS_FILE,
        &Progress {
            id: ID.into(),
            confirm_opened_at: Some(500),
            ignored_confirm_at: Some(500),
            ..core.progress()
        },
    );
    seal(
        &core,
        &edge,
        json!({ "kind": "gmail_confirm", "code": "2", "link": null, "requester": null, "sentAt": 250 }),
    );
    core.check_forwarding("f".into()).await.unwrap();
    core.opened_forwarding_confirm("f".into());
    core.ignore_forwarding_confirm("f".into());
    let progress = core.progress();
    assert_eq!(progress.confirm_opened_at, Some(500));
    assert_eq!(progress.ignored_confirm_at, Some(500));

    // Boundary timestamp math saturates without overflowing.
    let otp = Payload::Otp {
        code: "401295".into(),
        sent_at: i64::MIN,
    };
    assert_eq!(otp.otp_since(i64::MIN, VTOP_SLACK_SECS), Some("401295"));
    let max_on = MailboxState::Joined {
        mailbox: mailbox(ID),
        setup: Setup::Active {
            last_mail_at: Some(i64::MAX),
            last_otp_at: Some(i64::MAX),
            confirmation: None,
        },
    };
    let typed = Progress {
        id: ID.into(),
        typed_otp_at: Some(i64::MAX),
        ..Progress::default()
    };
    assert_eq!(hint(&view_with(&max_on, &typed)), None);
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn save_new_mailbox_clears_progress_and_mark_done_refuses_at_confirm() {
    let (core, edge, dir, _url) = forwarding_core("a10").await;
    let slot = core.current().unwrap();
    // Stale progress for the same id is wiped when a new mailbox is saved.
    core.forget_forwarding(ID);
    core.write(
        PROGRESS_FILE,
        &Progress {
            id: ID.into(),
            confirm_opened_at: Some(999),
            ..Progress::default()
        },
    );
    assert!(dir.join(PROGRESS_FILE).exists());
    core.save_new_mailbox(&slot, mailbox(ID)).unwrap();
    assert!(!dir.join(PROGRESS_FILE).exists());

    // mark_forwarding_done refuses while the step is Confirm, and succeeds after opening.
    seal(
        &core,
        &edge,
        json!({ "kind": "gmail_confirm", "code": "1", "link": null, "requester": null, "sentAt": 100 }),
    );
    let waiting = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(step(&waiting), Some(SetupStep::Confirm));
    assert_eq!(state(&core.mark_forwarding_done("f".into())), "waiting");
    assert_eq!(
        step(&core.opened_forwarding_confirm("f".into())),
        Some(SetupStep::Filter)
    );
    assert_eq!(state(&core.mark_forwarding_done("f".into())), "on");
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn forwarding_phones_keeps_address_on_not_found_while_rejoining() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-b8-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    let core = signed_in(&dir, url.clone());
    keep_token(&core, &url);
    core.write(
        FILE,
        &json!({
            "server": url, "id": ID, "address": format!("otp+{ID}@vmate.aster0.dev"),
            "joined": false, "ready": true, "last_mail_at": 5, "last_otp_at": 5,
        }),
    );
    edge.lock().unwrap().canned.insert(
        "mail:devices".into(),
        [(
            404,
            json!({ "code": ServerCode::MailboxNotFound.as_str(), "error": "No such forwarding address." }),
        )]
        .into(),
    );
    let err = core.forwarding_phones("f".into()).await;
    assert!(matches!(
        err,
        Err(CoreError::ForwardingOff {
            kind: ForwardingOffKind::NeedsSignIn,
            ..
        })
    ));
    assert!(core.saved_mailbox().is_some());
    assert!(dir.join(FILE).exists());
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn demo_account_refuses_set_up_forwarding() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-demo-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    fs::write(dir.join(SERVER_FILE), &url).unwrap();
    let core = Core::for_test(&dir);
    let client = vtop_core::VtopClient::builder()
        .with_session(&vtop_core::SessionState {
            cookies: String::new(),
            csrf_token: None,
            registration_number: None,
            otp_issued_at: None,
            logged_in_at: None,
        })
        .unwrap();
    core.install(crate::Session::for_test(
        client,
        crate::Account {
            username: crate::demo::USERNAME.into(),
            password: crate::demo::PASSWORD.into(),
            registration_number: None,
        },
    ));
    let res = core.set_up_forwarding("f-demo".into()).await;
    assert!(matches!(res, Err(CoreError::Other { .. })), "{res:?}");
    assert!(edge.lock().unwrap().requests.is_empty());
    assert!(!dir.join(FILE).exists());
    let _ = fs::remove_dir_all(&dir);
}

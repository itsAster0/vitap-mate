use std::sync::Arc;

use vtop_shared::device_sig;

use super::*;
use crate::remote::consent;
use crate::remote::edge::SERVER_FILE;
use crate::test_edge::{Edge, ID, keep_token, now, serve, signed_in};

/// Seals `payload` the way the edge does: to the registered seal key, for mailbox [`ID`].
fn seal(edge: &Arc<Mutex<Edge>>, payload: Value) {
    let mut edge = edge.lock().unwrap();
    let key = edge.seal_key.clone().unwrap();
    edge.sealed = Some(mail_seal::seal(&key, ID, payload.to_string().as_bytes()).unwrap());
}

#[tokio::test]
async fn sets_up_checks_fetches_and_turns_off() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let url = serve(edge.clone()).await;
    fs::write(dir.join(SERVER_FILE), url.clone() + "\n").unwrap();
    // No address to join yet: a new one, then claimed for the student.
    let core = signed_in(&dir, url.clone());
    keep_token(&core, &url);
    assert_eq!(
        core.forwarding_status("f".into()).state,
        ForwardingState::Off
    );
    let fetched = core.fetch_forwarded_otp("f".into(), 0).await;
    assert!(matches!(fetched, Err(CoreError::Other { .. })));

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
        assert_eq!(
            paths,
            [
                "/v1/devices/register",
                "/v1/mail-otp/reclaim",
                "/v1/mail-otp/register",
                "/v1/mail-otp/owner"
            ]
        );
        assert!(edge.requests.iter().all(|r| r.flow == "f-setup"));
        let register = &edge.requests[0].body;
        assert_eq!(
            (&register["label"], &register["platform"]),
            (&json!(crate::remote::server::label()), &json!("android"))
        );
        assert!(mail_seal::is_public_key(
            register["sealKey"].as_str().unwrap()
        ));
        assert_eq!(edge.requests[1].body, json!({ "accountToken": "tok" }));
        assert_eq!(edge.requests[2].body, json!({}));
        assert_eq!(edge.requests[3].body["id"], json!(ID));
    }
    // Set up already: nothing is sent.
    core.set_up_forwarding("f".into()).await.unwrap();
    assert_eq!(edge.lock().unwrap().requests.len(), 4);

    // Nothing forwarded yet.
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(checked.state, ForwardingState::Waiting);
    assert_eq!(
        edge.lock().unwrap().requests.last().unwrap().body,
        json!({ "id": ID })
    );

    // Gmail's confirmation proves the address works.
    seal(
        &edge,
        json!({ "kind": "gmail_confirm", "code": "981234567", "link": null, "requester": "me@vitapstudent.ac.in", "sentAt": 1_700_000_000 }),
    );
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(checked.state, ForwardingState::On);
    let confirmation = checked.confirmation.unwrap();
    assert_eq!(confirmation.code.as_deref(), Some("981234567"));
    assert_eq!(checked.last_mail_at, Some(1_700_000_000));
    assert_eq!(checked.last_otp_at, None);
    // Not an OTP.
    assert_eq!(core.fetch_forwarded_otp("f".into(), 0).await, Ok(None));

    let sent = now();
    seal(
        &edge,
        json!({ "kind": "otp", "code": "401295", "sentAt": sent }),
    );
    let code = core.fetch_forwarded_otp("f".into(), sent - 30).await;
    assert_eq!(code, Ok(Some("401295".into())));
    // An OTP from before this prompt.
    let code = core.fetch_forwarded_otp("f".into(), sent + 120).await;
    assert_eq!(code, Ok(None));
    let status = core.forwarding_status("f".into());
    assert_eq!(status.last_otp_at, Some(sent));
    // No key or secret in forwarding.json.
    let saved = fs::read_to_string(dir.join(FILE)).unwrap();
    assert!(!saved.contains("secret") && !saved.contains("seed"));

    core.turn_off_forwarding("f-off".into()).await.unwrap();
    {
        let edge = edge.lock().unwrap();
        let last = edge.requests.last().unwrap();
        assert_eq!(
            (last.path.as_str(), last.flow.as_str()),
            ("/v1/mail-otp/delete", "f-off")
        );
        assert_eq!(last.body, json!({ "id": ID, "everywhere": false }));
    }
    assert_eq!(
        core.forwarding_status("f".into()).state,
        ForwardingState::Off
    );
    assert!(!dir.join(FILE).exists());

    // An address the edge lost is forgotten here.
    core.set_up_forwarding("f".into()).await.unwrap();
    edge.lock().unwrap().gone = true;
    let checked = core.check_forwarding("f".into()).await;
    assert!(matches!(checked, Err(CoreError::Other { .. })));
    assert_eq!(
        core.forwarding_status("f".into()).state,
        ForwardingState::Off
    );

    // Registered once; every request was signed as this install.
    let edge = edge.lock().unwrap();
    let registers = edge
        .requests
        .iter()
        .filter(|r| r.path == "/v1/devices/register")
        .count();
    assert_eq!(registers, 1);
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
        let edge = edge.lock().unwrap();
        let payload = json!({ "kind": "gmail_confirm", "code": "981234567", "link": null, "requester": null, "sentAt": 1_700_000_000 });
        mail_seal::seal(
            edge.seal_key.as_deref().unwrap(),
            ID,
            payload.to_string().as_bytes(),
        )
        .unwrap()
    };
    let inbox = "/v1/mail-otp/inbox".to_string();
    edge.lock().unwrap().canned.insert(
        inbox.clone(),
        [(
            200,
            json!({ "sealed": null, "confirmation": kept, "lastOtpAt": null }),
        )]
        .into(),
    );
    let checked = core.check_forwarding("f".into()).await.unwrap();
    assert_eq!(checked.state, ForwardingState::On);
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
    // and Gmail's confirmation goes, here as on the edge.
    assert!(checked.confirmation.is_none());
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn requests_carry_valid_signatures() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-sig-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = Core::for_test(&dir);
    let body = json!({ "id": ID });
    core.edge_post(&server, "/v1/mail-otp/inbox", "f", &body)
        .await
        .unwrap();
    let edge = edge.lock().unwrap();
    assert_eq!(edge.requests.len(), 2);
    assert!(edge.requests.iter().all(|r| r.signed));
    // The key the edge saw is this install's: a signature by another seed does not verify.
    let other = device_sig::generate_seed();
    let sig = device_sig::sign(&other, "POST", "/v1/mail-otp/inbox", now(), b"{}").unwrap();
    let key = edge.sign_key.as_deref().unwrap();
    assert!(!device_sig::verify(
        key,
        &sig,
        "POST",
        "/v1/mail-otp/inbox",
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
        ready: false,
        last_mail_at: None,
        last_otp_at: None,
        confirmation: None,
    };
    core.write(FILE, &saved);
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
    // An edge without `/leave` (or offline): still queued.
    edge.lock().unwrap().canned.insert(
        "/v1/mail-otp/leave".into(),
        [(
            404,
            json!({ "code": "not_found", "error": "No such route." }),
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
            ("/v1/mail-otp/leave", "f-leave")
        );
        assert_eq!(last.body, json!({ "id": ID }));
    }
    assert!(!dir.join(LEAVE_FILE).exists());
    let sent = edge.lock().unwrap().requests.len();
    core.leave_old_forwarding("f".into()).await.unwrap();
    assert_eq!(edge.lock().unwrap().requests.len(), sent);
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

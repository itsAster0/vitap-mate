use std::fs;
use std::sync::{Arc, Mutex};

use super::*;
use crate::test_edge::{DEVICE_ID, Edge, keep_token, serve, signed_in};

fn core(name: &str) -> Core {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-server-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    Core::for_test(dir)
}

fn can(edge: &Arc<Mutex<Edge>>, path: &str, status: u16, body: Value) {
    let mut edge = edge.lock().unwrap();
    edge.canned
        .entry(path.into())
        .or_default()
        .push_back((status, body));
}

fn paths(edge: &Arc<Mutex<Edge>>) -> Vec<String> {
    let edge = edge.lock().unwrap();
    edge.requests.iter().map(|r| r.path.clone()).collect()
}

/// This phone's forwarding address on `server`, as `forwarding.json` holds it.
fn mailbox(core: &Core, server: &str) {
    let saved = json!({ "server": server, "id": "k3f9x2ab7q", "address": "otp+k3f9x2ab7q@vmate.aster0.dev" });
    fs::write(core.at(crate::remote::forwarding::FILE), saved.to_string()).unwrap();
}

#[tokio::test]
async fn semester_push_is_quiet() {
    let core = core("semester");
    let edge = Arc::new(Mutex::new(Edge::default()));
    fs::write(
        core.at(crate::remote::edge::SERVER_FILE),
        serve(edge.clone()).await,
    )
    .unwrap();
    fs::write(core.at(crate::SELECTED), r#""AP2026""#).unwrap();
    consent::tests::signed_in(&core);
    let gate = consent::tests::yes(&core);
    assert_eq!(core.send_server_semester("f".into()).await, Ok(()));
    let resent = core.resend_server_password("f".into(), None).await;
    assert_eq!(resent, Ok(false));
    assert_eq!(gate.asks(), 0);
    assert!(edge.lock().unwrap().requests.is_empty());
}

#[tokio::test]
async fn status_reads_everything_the_server_keeps() {
    let core = core("status");
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    mailbox(&core, &server);
    let account = json!({
        "registrationNumber": "22BCE0001", "createdAt": 4, "semesterId": "AP2025264", "forwarding": true,
        "settings": { "alwaysUseVault": false, "phoneWaitSecs": 45, "vaultTtlSecs": 86400 },
        "credentialsRejectedAt": 3, "thisPhone": true,
        "keys": [{ "id": "ab12cd34", "label": "MCP", "createdAt": 6, "lastUsedAt": null, "phoneId": "p1", "outings": false }],
        "phones": [
            { "id": "p1", "label": "OnePlus CPH2573", "addedAt": 1, "seenAt": 5, "thisPhone": true },
            { "id": "p2", "label": null, "addedAt": 2, "seenAt": 2, "thisPhone": false },
        ],
    });
    let status = json!({
        "sessionSavedAt": 9, "sessionSource": "phone", "vaultSavedAt": null, "vaultExpiresAt": null,
        "vaultGmail": false, "credentialsRejectedAt": 3, "forwarding": null, "phones": 2, "keys": [],
        "signIns": [{ "at": 8, "by": "server", "ok": false, "reason": "VTOP down" }],
    });
    let info = json!({ "deviceId": DEVICE_ID, "label": "Pixel", "platform": "android", "pushToken": true,
        "createdAt": 7, "events": 2, "latest": 9 });
    let devices = json!({ "devices": [
        { "deviceId": DEVICE_ID, "label": "Pixel", "addedAt": 1, "seenAt": 2, "thisDevice": true },
        { "deviceId": "ff", "label": null, "addedAt": 3, "seenAt": 3, "thisDevice": false },
    ] });
    can(&edge, "/v1/account", 200, account);
    can(&edge, "/v1/devices/info", 200, info);
    can(&edge, "/v1/app/status", 200, status);
    can(&edge, "/v1/mail-otp/devices", 200, devices);
    let core = signed_in(&core.dir, server.clone());
    keep_token(&core, &server);
    let status = core.status_with(&server, "f-s", "Load").await.unwrap();
    assert!(status.linked && status.forwarding);
    assert_eq!(
        (
            status.registration_number.as_str(),
            status.created_at,
            status.semester_id.as_deref()
        ),
        ("22BCE0001", Some(4), Some("AP2025264"))
    );
    assert_eq!(
        status.options,
        ServerOptions {
            always_use_vault: false,
            phone_wait_secs: 45,
            vault_ttl_secs: Some(86400)
        }
    );
    assert_eq!(
        (status.session_saved_at, status.session_source.as_deref()),
        (Some(9), Some("phone"))
    );
    assert_eq!(
        (status.vault_saved_at, status.credentials_rejected_at),
        (None, Some(3))
    );
    assert_eq!(
        status.sign_ins,
        [ServerSignIn {
            at: 8,
            by_server: true,
            ok: false,
            reason: Some("VTOP down".into())
        }]
    );
    assert_eq!(status.phones.len(), 2);
    assert!(status.phones[0].this_phone && !status.phones[1].this_phone);
    assert_eq!(
        (
            status.keys.len(),
            status.keys[0].id.as_str(),
            status.keys[0].created_at
        ),
        (1, "ab12cd34", 6)
    );
    assert_eq!(
        status.device,
        ServerDevice {
            id: Some(DEVICE_ID[..8].into()),
            label: "Pixel".into(),
            push_token: true,
            registered_at: Some(7),
            events: Some(2),
        }
    );
    let mailbox = status.mailbox.unwrap();
    assert_eq!(mailbox.address, "otp+k3f9x2ab7q@vmate.aster0.dev");
    assert!(mailbox.phones[0].this_phone && mailbox.phones[1].id == "ff");
    assert_eq!(
        paths(&edge),
        [
            "/v1/devices/register",
            "/v1/account",
            "/v1/devices/info",
            "/v1/app/status",
            "/v1/mail-otp/devices"
        ]
    );
    let edge = edge.lock().unwrap();
    assert_eq!(
        edge.requests[3].body,
        json!({ "registrationNumber": "22BCE0001" })
    );
    assert_eq!(edge.requests[4].body, json!({ "id": "k3f9x2ab7q" }));
    assert!(edge.requests.iter().all(|r| r.signed && r.flow == "f-s"));
}

#[tokio::test]
async fn an_older_server_and_an_unlinked_phone_still_show_what_applies() {
    let core = core("older");
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    // No createdAt, settings or /v1/devices/info (the fake answers 404 not_found).
    can(
        &edge,
        "/v1/account",
        200,
        json!({ "registrationNumber": "22BCE0001", "thisPhone": false }),
    );
    let core = signed_in(&core.dir, server.clone());
    keep_token(&core, &server);
    let status = core.status_with(&server, "f", "Load").await.unwrap();
    assert!(!status.linked && status.mailbox.is_none());
    assert_eq!(
        (status.created_at, status.options.clone()),
        (None, ServerOptions::default())
    );
    assert_eq!(
        (
            status.device.id.as_deref(),
            status.device.events,
            status.device.label.as_str()
        ),
        (Some(&DEVICE_ID[..8]), None, "vitapmate")
    );
    assert_eq!(
        paths(&edge),
        ["/v1/devices/register", "/v1/account", "/v1/devices/info"]
    );
}

#[tokio::test]
async fn deleting_this_device_forgets_its_id_unless_the_server_lacks_the_route() {
    let core = core("delete");
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    fs::write(core.at(crate::remote::edge::SERVER_FILE), &server).unwrap();
    // Never registered: nothing to delete.
    assert_eq!(core.delete_server_device("f0".into()).await, Ok(true));
    assert!(paths(&edge).is_empty());

    core.device_id(&server, "f").await.unwrap();
    mailbox(&core, &server);
    // An older server: 404 not_found, nothing forgotten.
    assert_eq!(core.delete_server_device("f1".into()).await, Ok(false));
    assert!(core.registration(&server).0.is_some() && core.saved_mailbox().is_some());

    can(&edge, "/v1/devices/delete", 204, Value::Null);
    assert_eq!(core.delete_server_device("f2".into()).await, Ok(true));
    assert!(core.registration(&server).0.is_none() && core.saved_mailbox().is_none());
    let edge = edge.lock().unwrap();
    let last = edge.requests.last().unwrap();
    assert_eq!(
        (last.path.as_str(), &last.body),
        ("/v1/devices/delete", &json!({ "mailbox": "k3f9x2ab7q" }))
    );
}

#[tokio::test]
async fn the_password_never_goes_without_consent() {
    use consent::tests as gates;
    let dir = core("password-consent").dir;
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = signed_in(&dir, server.clone());
    let at = crate::unix_now() * 1000 + 86_400_000;
    let kept = json!({ "server": server, "registration_number": "22BCE0001", "token": "tok", "expires_at": at });
    core.write_plain(crate::remote::account_token::FILE, &kept);
    fs::write(
        core.at(crate::ACCOUNT),
        r#"{"username":"22BCE0001","password":"pw"}"#,
    )
    .unwrap();
    can(
        &edge,
        "/v1/link",
        200,
        json!({ "savedCredentials": true, "forwarding": false }),
    );

    // Unasked without a remembered yes: refused, and nobody is asked.
    let gate = gates::no(&core);
    let unasked = core
        .save_password_with(&server, "f", "save", false, None)
        .await;
    assert!(matches!(unasked, Err(CoreError::ConsentDenied)));
    assert_eq!(gate.asks(), 0);
    assert!(
        !edge
            .lock()
            .unwrap()
            .requests
            .iter()
            .any(|r| r.path == "/v1/link")
    );

    // A remembered yes lets the unasked resend go.
    gates::remember(&core, CookieKind::Account);
    core.save_password_with(&server, "f", "save", false, None)
        .await
        .unwrap();
    assert!(
        edge.lock()
            .unwrap()
            .requests
            .iter()
            .any(|r| r.path == "/v1/link")
    );
}

#[tokio::test]
async fn the_password_goes_sealed_only_when_the_server_can_get_the_code() {
    let dir = core("password").dir;
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    // Signed in, with an account token kept: the password goes with it, not the cookie.
    let core = signed_in(&dir, server.clone());
    let at = crate::unix_now() * 1000 + 86_400_000;
    let kept = json!({ "server": server, "registration_number": "22BCE0001", "token": "tok", "expires_at": at });
    core.write_plain(crate::remote::account_token::FILE, &kept);
    fs::write(
        core.at(crate::ACCOUNT),
        r#"{"username":"22BCE0001","password":"pw"}"#,
    )
    .unwrap();
    can(
        &edge,
        "/v1/link",
        200,
        json!({ "savedCredentials": true, "forwarding": false }),
    );
    can(
        &edge,
        "/v1/link",
        200,
        json!({ "savedCredentials": false, "forwarding": false }),
    );
    let gmail = GmailGrant {
        refresh_token: "r".into(),
        client_id: "c.apps.googleusercontent.com".into(),
        client_secret: None,
        delete_after_reading: true,
    };
    let gate = consent::tests::yes(&core);
    core.save_password_with(&server, "f", "save", true, Some(gmail))
        .await
        .unwrap();
    assert!(
        core.save_password_with(&server, "f", "save", true, None)
            .await
            .is_err()
    );
    // Asked, the Server screen's dialog is the consent: no second prompt.
    assert_eq!(gate.asks(), 0);
    let edge = edge.lock().unwrap();
    let sent = &edge
        .requests
        .iter()
        .find(|r| r.path == "/v1/link")
        .unwrap()
        .body;
    assert_eq!(sent["credentials"]["username"], "22BCE0001");
    assert_eq!(sent["credentials"]["otp"], "gmail");
    assert_eq!(
        sent["credentials"]["gmail"]["clientId"],
        "c.apps.googleusercontent.com"
    );
    assert_eq!(sent["accountToken"], "tok");
    assert!(sent.get("session").is_none());
}

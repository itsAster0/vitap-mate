use std::fs;
use std::sync::{Arc, Mutex};

use super::*;
use crate::test_edge::{Edge, keep_token, paths, serve, signed_in};

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
    let info = json!({ "deviceId": "d1", "label": "Pixel", "platform": "android", "pushToken": true,
        "createdAt": 7, "latest": 9 });
    let devices = json!({ "devices": [
        { "deviceId": "d1", "label": "Pixel", "addedAt": 1, "seenAt": 2, "thisDevice": true },
        { "deviceId": "ff", "label": null, "addedAt": 3, "seenAt": 3, "thisDevice": false },
    ] });
    can(
        &edge,
        "screen:view",
        200,
        json!({ "account": account, "device": info, "status": status, "mailbox": devices }),
    );
    let core = signed_in(&core.dir, server.clone());
    keep_token(&core, &server);
    let status = core.status_with("f-s", "Load").await.unwrap();
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
        status.session,
        ServerSession::Saved {
            at: 9,
            by: SessionSource::Phone
        }
    );
    assert_eq!(
        (status.vault, status.credentials_rejected_at),
        (ServerVault::None, Some(3))
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
            id: Some(core.registration().0[..8].into()),
            label: "Pixel".into(),
            push_token: true,
            registered_at: Some(7),
        }
    );
    let mailbox = status.mailbox.unwrap();
    assert_eq!(mailbox.address, "otp+k3f9x2ab7q@vmate.aster0.dev");
    assert!(mailbox.phones[0].this_phone && mailbox.phones[1].id == "ff");
    // One read, with this install's address.
    assert_eq!(paths(&edge), ["auth:token", "screen:view"]);
    let edge = edge.lock().unwrap();
    assert_eq!(edge.requests[1].body, json!({ "mailbox": "k3f9x2ab7q" }));
    // The reads share the socket's subscriptions, so they carry no flow; `auth:token` does.
    assert!(edge.requests.iter().all(|r| r.signed));
    assert!(
        edge.requests
            .iter()
            .all(|r| (r.path == "auth:token") != r.flow.is_empty())
    );
}

#[tokio::test]
async fn an_older_server_and_an_unlinked_phone_still_show_what_applies() {
    let core = core("older");
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    // No createdAt, settings or device; not linked, so nothing kept is shown.
    can(
        &edge,
        "screen:view",
        200,
        json!({ "account": { "registrationNumber": "22BCE0001", "thisPhone": false }, "status": null }),
    );
    let core = signed_in(&core.dir, server.clone());
    keep_token(&core, &server);
    let status = core.status_with("f", "Load").await.unwrap();
    assert!(!status.linked && status.mailbox.is_none());
    assert_eq!(
        (status.created_at, status.options.clone()),
        (None, ServerOptions::default())
    );
    assert_eq!(
        (status.device.id.as_deref(), status.device.label.as_str()),
        (Some(&core.registration().0[..8]), "vitapmate")
    );
    assert_eq!(paths(&edge), ["auth:token", "screen:view"]);
}

#[tokio::test]
async fn deleting_this_device_forgets_its_identity_unless_the_server_lacks_it() {
    let core = core("delete");
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    fs::write(core.at(crate::remote::edge::SERVER_FILE), &server).unwrap();
    // No identity there: nothing to delete.
    assert_eq!(core.delete_server_device("f0".into()).await, Ok(true));
    assert!(paths(&edge).is_empty());

    keep_token(&core, &server);
    mailbox(&core, &server);
    // An older server: no such function, nothing forgotten.
    assert_eq!(core.delete_server_device("f1".into()).await, Ok(false));
    assert!(core.device().refresh.is_some() && core.saved_mailbox().is_some());

    can(&edge, "devices:remove", 200, Value::Null);
    assert_eq!(core.delete_server_device("f2".into()).await, Ok(true));
    assert!(core.device().refresh.is_none() && core.saved_mailbox().is_none());
    let edge = edge.lock().unwrap();
    let last = edge.requests.last().unwrap();
    assert_eq!(
        (last.path.as_str(), &last.body),
        ("devices:remove", &json!({ "mailbox": "k3f9x2ab7q" }))
    );
}

#[tokio::test]
async fn the_password_never_goes_without_consent() {
    use consent::tests as gates;
    let dir = core("password-consent").dir;
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = signed_in(&dir, server.clone());
    keep_token(&core, &server);
    fs::write(
        core.at(crate::ACCOUNT),
        r#"{"username":"22BCE0001","password":"pw"}"#,
    )
    .unwrap();
    can(
        &edge,
        "account:saveCredentials",
        200,
        json!({ "savedCredentials": true, "forwarding": false }),
    );

    // Unasked without a remembered yes: refused, and nobody is asked.
    let gate = gates::no(&core);
    let unasked = core.save_password_with("f", "save", false, None).await;
    assert!(matches!(unasked, Err(CoreError::ConsentDenied)));
    assert_eq!(gate.asks(), 0);
    assert!(
        !edge
            .lock()
            .unwrap()
            .requests
            .iter()
            .any(|r| r.path == "account:saveCredentials")
    );

    // A remembered yes lets the unasked resend go.
    gates::remember(&core, CookieKind::Account);
    core.save_password_with("f", "save", false, None)
        .await
        .unwrap();
    assert!(
        edge.lock()
            .unwrap()
            .requests
            .iter()
            .any(|r| r.path == "account:saveCredentials")
    );
}

#[tokio::test]
async fn the_password_goes_sealed_only_when_the_server_can_get_the_code() {
    let dir = core("password").dir;
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    // Signed in, with an identity kept: the password goes with it, not the cookie.
    let core = signed_in(&dir, server.clone());
    keep_token(&core, &server);
    fs::write(
        core.at(crate::ACCOUNT),
        r#"{"username":"22BCE0001","password":"pw"}"#,
    )
    .unwrap();
    can(
        &edge,
        "account:saveCredentials",
        200,
        json!({ "savedCredentials": true, "forwarding": false }),
    );
    can(
        &edge,
        "account:saveCredentials",
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
    core.save_password_with("f", "save", true, Some(gmail))
        .await
        .unwrap();
    assert!(
        core.save_password_with("f", "save", true, None)
            .await
            .is_err()
    );
    // Asked, the Server screen's dialog is the consent: no second prompt.
    assert_eq!(gate.asks(), 0);
    let edge = edge.lock().unwrap();
    let sent = &edge
        .requests
        .iter()
        .find(|r| r.path == "account:saveCredentials")
        .unwrap()
        .body;
    assert_eq!(sent["credentials"]["username"], "22BCE0001");
    assert_eq!(sent["credentials"]["otp"], "gmail");
    assert_eq!(
        sent["credentials"]["gmail"]["clientId"],
        "c.apps.googleusercontent.com"
    );
    assert!(sent.get("accountToken").is_none());
    assert!(sent.get("session").is_none());
}

#[test]
fn the_kept_session_and_password_read_once_into_one_state_each() {
    let view = |json: Value| serde_json::from_value::<StatusView>(json).unwrap();
    let none = view(json!({}));
    assert_eq!(
        (none.session(), none.vault()),
        (ServerSession::None, ServerVault::None)
    );
    let saved = view(json!({
        "sessionSavedAt": 7, "sessionSource": "server",
        "vaultSavedAt": 8, "vaultExpiresAt": 9, "vaultGmail": true,
    }));
    assert_eq!(
        saved.session(),
        ServerSession::Saved {
            at: 7,
            by: SessionSource::Server
        }
    );
    assert_eq!(
        saved.vault(),
        ServerVault::Saved {
            at: 8,
            expires_at: Some(9),
            gmail: true
        }
    );
    // A source the server doesn't name (or an older one): a phone's.
    assert_eq!(
        view(json!({ "sessionSavedAt": 7 })).session(),
        ServerSession::Saved {
            at: 7,
            by: SessionSource::Phone
        }
    );
    // An expiry or Gmail flag without a saved password is nothing kept.
    assert_eq!(
        view(json!({ "vaultExpiresAt": 9, "vaultGmail": true })).vault(),
        ServerVault::None
    );
}

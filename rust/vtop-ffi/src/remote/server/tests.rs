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

/// A signed-in core with an identity kept, on a fake server.
async fn linked(name: &str) -> (Core, Arc<Mutex<Edge>>, String) {
    let dir = core(name).dir;
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = signed_in(&dir, server.clone());
    keep_token(&core, &server);
    (core, edge, server)
}

fn wire(view: Value) -> AccountWire {
    serde_json::from_value(view).unwrap()
}

#[tokio::test]
async fn the_account_reads_its_link_and_storage_once() {
    let (core, edge, _) = linked("account").await;
    let view = json!({
        "registrationNumber": "22BCE0001", "createdAt": 4, "semesterId": "AP2025264", "forwarding": true,
        "settings": { "cacheSession": true, "alwaysUseVault": false, "phoneWaitSecs": 45, "vaultTtlSecs": 86400 },
        "savedCredentials": true, "vaultSavedAt": 8, "vaultExpiresAt": 86408, "vaultGmail": true,
        "sessionSavedAt": 9, "sessionSource": "server",
        "credentialsRejectedAt": 3, "thisPhone": true,
        "keys": [{ "id": "ab12cd34", "label": "MCP", "createdAt": 6, "lastUsedAt": null, "phoneId": "p1", "outings": false }],
        "phones": [
            { "id": "p1", "label": "OnePlus CPH2573", "addedAt": 1, "seenAt": 5, "thisPhone": true },
            { "id": "p2", "label": null, "addedAt": 2, "seenAt": 2, "thisPhone": false },
        ],
    });
    can(&edge, "account:view", 200, view);
    let account = core.server_account("f-a".into()).await.unwrap();
    assert_eq!(
        account,
        ServerAccount {
            registration_number: "22BCE0001".into(),
            created_at: Some(4),
            semester_id: Some("AP2025264".into()),
            link: ServerLink::Linked {
                phones: vec![
                    LinkedPhone {
                        id: "p1".into(),
                        label: Some("OnePlus CPH2573".into()),
                        added_at: 1,
                        seen_at: 5,
                        this_phone: true,
                    },
                    LinkedPhone {
                        id: "p2".into(),
                        label: None,
                        added_at: 2,
                        seen_at: 2,
                        this_phone: false,
                    },
                ],
            },
            storage: ServerStorage::OfflineSignIn {
                session: ServerSession::Saved {
                    at: 9,
                    by: SessionSource::Server
                },
                vault: SavedVault {
                    at: 8,
                    expires_at: Some(86408),
                    gmail: true
                },
                options: VaultOptions {
                    always_use_vault: false,
                    phone_wait_secs: 45,
                    vault_ttl_secs: Some(86400)
                },
            },
            forwarding: true,
            credentials_rejected_at: Some(3),
        }
    );
    // One read, after a token; queries share the socket's subscriptions, so they carry no flow.
    // Only these calls count: other background calls may reach the fake server on a slow runner.
    let paths = paths(&edge);
    let token = paths.iter().position(|p| p == "auth:token");
    let views: Vec<usize> = (0..paths.len())
        .filter(|&i| paths[i] == "account:view")
        .collect();
    assert_eq!(views.len(), 1, "{paths:?}");
    assert!(token.is_some_and(|t| t < views[0]), "{paths:?}");
    let edge = edge.lock().unwrap();
    let view = &edge.requests[views[0]];
    assert!(view.signed && edge.requests[token.unwrap()].signed);
    assert!(view.flow.is_empty());
}

#[tokio::test]
async fn an_older_server_and_an_unlinked_phone_still_show_what_applies() {
    let (core, edge, _) = linked("older").await;
    // No createdAt or settings; not linked, so no phones.
    can(
        &edge,
        "account:view",
        200,
        json!({ "registrationNumber": "22BCE0001", "thisPhone": false, "phones": [] }),
    );
    let account = core.server_account("f".into()).await.unwrap();
    assert_eq!(
        (account.link, account.storage, account.created_at),
        (ServerLink::Unlinked, ServerStorage::Off, None)
    );
}

#[test]
fn storage_reads_into_one_state() {
    // Off: whatever else the server sent alongside.
    let off = wire(json!({
        "settings": { "cacheSession": false, "alwaysUseVault": true, "phoneWaitSecs": 20 },
        "sessionSavedAt": 7, "vaultSavedAt": 8,
    }));
    assert_eq!(off.storage(), ServerStorage::Off);
    // On with nothing kept yet, and with a session a phone handed over.
    let on = |extra: Value| {
        let mut view = json!({ "settings": { "cacheSession": true, "alwaysUseVault": true, "phoneWaitSecs": 20 } });
        for (key, value) in extra.as_object().unwrap() {
            view[key] = value.clone();
        }
        wire(view).storage()
    };
    assert_eq!(
        on(json!({})),
        ServerStorage::SessionOnly {
            session: ServerSession::None
        }
    );
    // A source the server doesn't name: a phone's.
    assert_eq!(
        on(json!({ "sessionSavedAt": 7 })),
        ServerStorage::SessionOnly {
            session: ServerSession::Saved {
                at: 7,
                by: SessionSource::Phone
            }
        }
    );
    // An expiry or Gmail flag without a saved password is no password.
    assert_eq!(
        on(json!({ "vaultExpiresAt": 9, "vaultGmail": true })),
        ServerStorage::SessionOnly {
            session: ServerSession::None
        }
    );
    assert_eq!(
        on(json!({ "vaultSavedAt": 8 })),
        ServerStorage::OfflineSignIn {
            session: ServerSession::None,
            vault: SavedVault {
                at: 8,
                expires_at: None,
                gmail: false
            },
            options: VaultOptions {
                always_use_vault: true,
                phone_wait_secs: 20,
                vault_ttl_secs: None
            },
        }
    );
    // A server from before the choice: on when it keeps something, with its defaults.
    assert_eq!(wire(json!({})).storage(), ServerStorage::Off);
    assert!(matches!(
        wire(json!({ "vaultSavedAt": 8 })).storage(),
        ServerStorage::OfflineSignIn {
            options: VaultOptions {
                always_use_vault: true,
                phone_wait_secs: 20,
                ..
            },
            ..
        }
    ));
    assert!(matches!(
        wire(json!({ "sessionSavedAt": 7 })).storage(),
        ServerStorage::SessionOnly { .. }
    ));
}

#[tokio::test]
async fn storage_goes_on_and_off_with_the_options_kept() {
    let (core, edge, _) = linked("storage").await;
    let settings = json!({ "cacheSession": true, "alwaysUseVault": false, "phoneWaitSecs": 45, "vaultTtlSecs": 172800 });
    can(
        &edge,
        "account:view",
        200,
        json!({ "settings": settings, "thisPhone": true }),
    );
    can(&edge, "account:setSettings", 200, Value::Null);
    core.set_server_storage("f-off".into(), false)
        .await
        .unwrap();
    can(&edge, "account:setSettings", 200, Value::Null);
    let options = VaultOptions {
        always_use_vault: true,
        phone_wait_secs: 10,
        vault_ttl_secs: None,
    };
    core.set_vault_options("f-opt".into(), options)
        .await
        .unwrap();
    assert_eq!(
        paths(&edge),
        [
            "auth:token",
            "account:view",
            "account:setSettings",
            "account:setSettings"
        ]
    );
    let edge = edge.lock().unwrap();
    // Off sends the settings whole, the password options as they were.
    assert_eq!(
        edge.requests[2].body,
        json!({ "settings": { "cacheSession": false, "alwaysUseVault": false, "phoneWaitSecs": 45, "vaultTtlSecs": 172800 } })
    );
    // The options leave storage to the server (and an older one takes them).
    assert_eq!(
        edge.requests[3].body,
        json!({ "settings": { "alwaysUseVault": true, "phoneWaitSecs": 10, "vaultTtlSecs": null } })
    );
    assert_eq!(edge.requests[3].flow, "f-opt");
}

#[tokio::test]
async fn sign_ins_read_newest_first_and_an_older_server_has_none() {
    let (core, edge, _) = linked("sign-ins").await;
    can(
        &edge,
        "account:signIns",
        200,
        json!([
            { "at": 9, "by": "phone", "ok": true, "reason": null },
            { "at": 8, "by": "server", "ok": false, "reason": "VTOP down" },
        ]),
    );
    assert_eq!(
        core.server_sign_ins("f".into()).await.unwrap(),
        [
            ServerSignIn {
                at: 9,
                by_server: false,
                outcome: SignInOutcome::SignedIn
            },
            ServerSignIn {
                at: 8,
                by_server: true,
                outcome: SignInOutcome::Failed {
                    reason: Some("VTOP down".into())
                },
            },
        ]
    );
    // Nothing canned: the fake server has no such function.
    assert_eq!(core.server_sign_ins("f".into()).await, Ok(vec![]));
}

#[tokio::test]
async fn this_device_shows_once_registered_and_never_proves() {
    let core = core("device");
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    fs::write(core.at(crate::remote::edge::SERVER_FILE), &server).unwrap();
    assert_eq!(
        core.server_device("f".into()).await,
        Ok(ServerDevice::Unregistered {
            label: "vitapmate".into()
        })
    );
    assert!(paths(&edge).is_empty());
    keep_token(&core, &server);
    let info = json!({ "deviceId": "d1", "label": "Pixel", "platform": "android", "pushToken": true, "createdAt": 7 });
    can(&edge, "devices:info", 200, info);
    assert_eq!(
        core.server_device("f".into()).await,
        Ok(ServerDevice::Registered {
            id: core.registration().0[..8].into(),
            label: "Pixel".into(),
            push_token: true,
            registered_at: Some(7),
        })
    );
}

#[tokio::test]
async fn forwarding_phones_read_and_a_deleted_address_is_forgotten() {
    let (core, edge, server) = linked("mail-phones").await;
    mailbox(&core, &server);
    let devices = json!({ "devices": [
        { "deviceId": "d1", "label": "Pixel", "addedAt": 1, "seenAt": 2, "thisDevice": true },
        { "deviceId": "ff", "label": null, "addedAt": 3, "seenAt": 3, "thisDevice": false },
    ] });
    can(&edge, "mail:devices", 200, devices);
    let phones = core.forwarding_phones("f".into()).await.unwrap();
    assert_eq!(phones.len(), 2);
    assert!(phones[0].this_phone && phones[1].id == "ff");
    assert_eq!(
        edge.lock().unwrap().requests.last().unwrap().body,
        json!({ "id": "k3f9x2ab7q" })
    );
    can(
        &edge,
        "mail:devices",
        404,
        json!({ "code": "mailbox_not_found", "error": "x" }),
    );
    let gone = core.forwarding_phones("f".into()).await;
    assert!(matches!(
        gone,
        Err(CoreError::ForwardingOff {
            kind: ForwardingOffKind::AddressDeleted,
            ..
        })
    ));
    assert!(core.saved_mailbox().is_none());
}

#[tokio::test]
async fn deleting_the_account_stops_sharing_and_server_routes() {
    use crate::remote::regno::RegnoSharing;
    use crate::remote::route::Route;
    let (core, edge, server) = linked("delete-account").await;
    mailbox(&core, &server);
    core.set_route("r".into(), Route::Udp).unwrap();
    can(&edge, "account:remove", 200, Value::Null);
    core.delete_server_account("f".into()).await.unwrap();
    assert_eq!(core.regno_sharing("f".into()).await, RegnoSharing::Declined);
    assert_eq!(core.route_status("f".into()).chosen, Route::Direct);
    assert!(core.device().refresh.is_none() && core.saved_mailbox().is_none());
    // Nothing more goes as the student: the next account call doesn't prove a new account.
    let next = core.server_account("f2".into()).await;
    assert_eq!(next, Err(CoreError::RegnoNotShared));
    assert_eq!(paths(&edge), ["auth:token", "account:remove"]);
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

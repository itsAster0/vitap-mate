//! The account API the app calls with a live VTOP session as proof, and
//! verified phone callbacks.

mod common;

use axum::http::StatusCode;
use common::{json, TestApp, REG};
use serde_json::{json, Value};
use vtop_bridge::fakes::session;
use vtop_bridge::store::Store;

const MINE: &str = "JSESSIONID=a; REG=22BCE0001";
const THEIRS: &str = "JSESSIONID=b; REG=21BCE9999";

fn now() -> u64 {
    vtop_core::now_unix()
}

fn with_session(cookies: &str, extra: Value) -> Value {
    let mut body = extra;
    body["session"] = json!({ "cookies": cookies });
    body
}

fn credentials() -> Value {
    json!({ "username": "yaswanth314", "password": "pw",
            "gmail": { "refresh_token": "r", "client_id": "c", "delete_after_reading": true } })
}

async fn link(
    app: &TestApp,
    cookies: &str,
    extra: Value,
) -> axum::http::Response<axum::body::Body> {
    let mut body = extra;
    if body.get("fcmToken").is_none() {
        body["fcmToken"] = json!("fcm-1");
    }
    app.post("/v1/link", with_session(cookies, body)).await
}

/// Links and returns the app secret.
async fn linked(app: &TestApp, cookies: &str, extra: Value) -> String {
    let response = link(app, cookies, extra).await;
    assert_eq!(response.status(), StatusCode::OK);
    json(response).await["appSecret"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn create_key(app: &TestApp, cookies: &str, secret: &str) -> Value {
    let response = app
        .post(
            "/v1/keys",
            with_session(cookies, json!({ "label": "laptop", "appSecret": secret })),
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    json(response).await
}

#[tokio::test]
async fn link_creates_account_and_caches_session() {
    let app = TestApp::new();
    let response = link(&app, MINE, json!({})).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(json(response).await["appSecret"]
        .as_str()
        .unwrap()
        .starts_with("vta_"));
    let accounts = app.accounts();
    assert_eq!(
        accounts.fcm_token(REG).await.unwrap().as_deref(),
        Some("fcm-1")
    );
    assert_eq!(
        accounts
            .cached_session(REG, now())
            .await
            .unwrap()
            .unwrap()
            .cookies,
        MINE
    );
    assert!(accounts.vault(REG).await.unwrap().is_none());
}

#[tokio::test]
async fn link_requires_an_fcm_token() {
    let app = TestApp::new();
    let response = app.post("/v1/link", with_session(MINE, json!({}))).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn link_with_gmail_credentials_stores_vault() {
    let app = TestApp::new();
    link(&app, MINE, json!({ "credentials": credentials() })).await;
    let vault = app.accounts().vault(REG).await.unwrap().unwrap();
    assert_eq!(vault.username, "yaswanth314");
    let without_gmail = json!({ "username": "u", "password": "pw" });
    let app = TestApp::new();
    link(&app, MINE, json!({ "credentials": without_gmail })).await;
    assert!(app.accounts().vault(REG).await.unwrap().is_none());
}

#[tokio::test]
async fn expired_session_is_401_and_vtop_down_is_503() {
    let app = TestApp::new();
    let expired = link(&app, "JSESSIONID=x", json!({})).await;
    assert_eq!(expired.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json(expired).await["code"], "session_invalid");
    let down = link(&app, "JSESSIONID=x; down", json!({})).await;
    assert_eq!(down.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json(down).await["code"], "vtop_unreachable");
}

#[tokio::test]
async fn keys_need_a_linked_account() {
    let app = TestApp::new();
    let response = app
        .post("/v1/keys", with_session(MINE, json!({ "label": "x" })))
        .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(json(response).await["code"], "not_linked");
}

#[tokio::test]
async fn key_create_list_revoke() {
    let app = TestApp::new();
    let secret = linked(&app, MINE, json!({})).await;
    let created = create_key(&app, MINE, &secret).await;
    let key = created["key"].as_str().unwrap();
    assert!(key.starts_with("vtm_"));
    assert_eq!(
        created["mcpUrl"],
        "https://vtop-mcp-production.up.railway.app/mcp"
    );
    let id = created["id"].as_str().unwrap().to_string();

    let account = json(
        app.post(
            "/v1/account",
            with_session(MINE, json!({ "appSecret": secret })),
        )
        .await,
    )
    .await;
    assert_eq!(account["registrationNumber"], REG);
    assert_eq!(account["thisPhone"], true);
    assert_eq!(account["phoneLinked"], true);
    assert_eq!(account["savedCredentials"], false);
    assert_eq!(account["keys"][0]["id"], id);
    assert_eq!(account["keys"][0]["label"], "laptop");
    assert!(account.to_string().find(key).is_none());

    let revoke = app
        .post(
            "/v1/keys/revoke",
            with_session(MINE, json!({ "id": id, "appSecret": secret })),
        )
        .await;
    assert_eq!(revoke.status(), StatusCode::NO_CONTENT);
    assert_eq!(app.state.keys.resolve(key, now()).await.unwrap(), None);
}

#[tokio::test]
async fn key_limit_is_409() {
    let app = TestApp::new();
    let secret = linked(&app, MINE, json!({})).await;
    for _ in 0..vtop_bridge::keys::MAX_KEYS_PER_ACCOUNT {
        app.state.keys.create(REG, "k", now()).await.unwrap();
    }
    let response = app
        .post(
            "/v1/keys",
            with_session(MINE, json!({ "label": "one more", "appSecret": secret })),
        )
        .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(json(response).await["code"], "key_limit");
}

#[tokio::test]
async fn revoking_another_accounts_key_is_404() {
    let app = TestApp::new();
    let mine = linked(&app, MINE, json!({})).await;
    let their_secret = linked(&app, THEIRS, json!({})).await;
    let theirs = create_key(&app, THEIRS, &their_secret).await;
    let response = app
        .post(
            "/v1/keys/revoke",
            with_session(MINE, json!({ "id": theirs["id"], "appSecret": mine })),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(app
        .state
        .keys
        .resolve(theirs["key"].as_str().unwrap(), now())
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn fcm_token_is_replaced() {
    let app = TestApp::new();
    let secret = linked(&app, MINE, json!({})).await;
    let response = app
        .post(
            "/v1/account/fcm-token",
            with_session(MINE, json!({ "fcmToken": "fcm-2", "appSecret": secret })),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        app.accounts().fcm_token(REG).await.unwrap().as_deref(),
        Some("fcm-2")
    );
}

#[tokio::test]
async fn forget_credentials_clears_only_the_vault() {
    let app = TestApp::new();
    link(&app, MINE, json!({ "credentials": credentials() })).await;
    let response = app
        .post(
            "/v1/account/forget-credentials",
            with_session(MINE, json!({})),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(app.accounts().vault(REG).await.unwrap().is_none());
    assert!(app.accounts().fcm_token(REG).await.unwrap().is_some());
}

#[tokio::test]
async fn delete_removes_everything() {
    let app = TestApp::new();
    let secret = linked(&app, MINE, json!({ "credentials": credentials() })).await;
    let key = create_key(&app, MINE, &secret).await["key"]
        .as_str()
        .unwrap()
        .to_string();
    let response = app
        .post(
            "/v1/account/delete",
            with_session(MINE, json!({ "appSecret": secret })),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(app.store.get_account(REG).await.unwrap().is_none());
    assert_eq!(app.state.keys.resolve(&key, now()).await.unwrap(), None);
}

// ---- verified callbacks ---------------------------------------------------

/// Starts a key-authenticated request that the phone must answer; returns
/// (requestId, responseToken).
async fn phone_request(app: &TestApp) -> (String, String) {
    let secret = linked(app, MINE, json!({})).await;
    app.accounts().clear_session(REG).await.unwrap();
    let key = create_key(app, MINE, &secret).await["key"]
        .as_str()
        .unwrap()
        .to_string();
    let response = app
        .send(
            axum::http::Request::post("/v1/session")
                .header("authorization", format!("Bearer {key}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
    let id = json(response).await["requestId"]
        .as_str()
        .unwrap()
        .to_string();
    for _ in 0..400 {
        if let Some((_, data)) = app.messenger.sends().last() {
            return (id, data["responseToken"].clone());
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("no FCM message");
}

fn cookies_for(header: &str) -> Value {
    serde_json::to_value(vtop_bridge::cookies::from_header(header)).unwrap()
}

#[tokio::test]
async fn callback_match_completes_and_caches() {
    let app = TestApp::new();
    let (id, token) = phone_request(&app).await;
    let fresh = "JSESSIONID=new; REG=22BCE0001";
    let response = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": token, "cookies": cookies_for(fresh) }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        app.accounts()
            .cached_session(REG, now())
            .await
            .unwrap()
            .unwrap()
            .cookies,
        fresh
    );
    let doc = app.store.get_request(&id).await.unwrap().unwrap();
    assert_eq!(doc.status, vtop_bridge::store::RequestStatus::Success);
}

#[tokio::test]
async fn callback_mismatch_is_403_and_fails_the_request() {
    let app = TestApp::new();
    let (id, token) = phone_request(&app).await;
    let response = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": token, "cookies": cookies_for(THEIRS) }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(json(response).await["code"], "account_mismatch");
    let doc = app.store.get_request(&id).await.unwrap().unwrap();
    assert_eq!(doc.status, vtop_bridge::store::RequestStatus::Error);
    assert!(doc.error.unwrap().starts_with("account_mismatch: "));
    assert!(app
        .accounts()
        .cached_session(REG, now())
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn callback_with_vtop_down_is_503_so_the_app_retries() {
    let app = TestApp::new();
    let (id, token) = phone_request(&app).await;
    let response = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": token, "cookies": cookies_for("JSESSIONID=z; down=1") }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let doc = app.store.get_request(&id).await.unwrap().unwrap();
    assert_eq!(doc.status, vtop_bridge::store::RequestStatus::Pending);
}

#[tokio::test]
async fn callback_with_wrong_token_never_reaches_vtop() {
    let app = TestApp::new();
    let (id, _) = phone_request(&app).await;
    let response = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": "wrong", "cookies": cookies_for("A=1; down") }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn callback_with_consented_credentials_refreshes_the_vault() {
    let app = TestApp::new();
    let (id, token) = phone_request(&app).await;
    app.post(
        "/cookie/callback",
        json!({ "requestId": id, "responseToken": token, "cookies": cookies_for(MINE),
                "username": "ignored", "credentials": {
                    "username": "newname", "password": "pw",
                    "gmail": { "refresh_token": "r", "client_id": "c" } } }),
    )
    .await;
    assert_eq!(
        app.accounts().vault(REG).await.unwrap().unwrap().username,
        "newname"
    );
}

#[tokio::test]
async fn legacy_callback_records_nothing() {
    let app = TestApp::new();
    let started = json(app.post("/cookie", json!({ "fcmToken": "abc" })).await).await;
    let id = started["requestId"].as_str().unwrap();
    let token = app.last_fcm()["responseToken"].clone();
    let response = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": token, "cookies": cookies_for(MINE),
                    "username": "u", "credentials": credentials() }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(app.store.get_account(REG).await.unwrap().is_none());
    let _ = session("unused");
}

/// The user typed the OTP after the client gave up: the answer is still
/// cached for the account, so the next call is served at once.
#[tokio::test]
async fn late_phone_answer_is_cached() {
    let app = TestApp::new();
    let (id, token) = phone_request(&app).await;
    let mut doc = app.store.get_request(&id).await.unwrap().unwrap();
    doc.expires_at = now() - 1;
    app.store.delete_request(&id).await.unwrap();
    app.store.create_request(&doc).await.unwrap();

    let late = "JSESSIONID=late; REG=22BCE0001";
    let response = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": token, "cookies": cookies_for(late) }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json(response).await, json!({ "ok": true, "late": true }));
    assert_eq!(
        app.accounts()
            .cached_session(REG, now())
            .await
            .unwrap()
            .unwrap()
            .cookies,
        late
    );
}

#[tokio::test]
async fn late_answer_for_another_student_is_rejected() {
    let app = TestApp::new();
    let (id, token) = phone_request(&app).await;
    let mut doc = app.store.get_request(&id).await.unwrap().unwrap();
    doc.expires_at = now() - 1;
    app.store.delete_request(&id).await.unwrap();
    app.store.create_request(&doc).await.unwrap();
    let response = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": token, "cookies": cookies_for(THEIRS) }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(app
        .accounts()
        .cached_session(REG, now())
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn late_answer_with_wrong_token_is_gone() {
    let app = TestApp::new();
    let (id, _) = phone_request(&app).await;
    let mut doc = app.store.get_request(&id).await.unwrap().unwrap();
    doc.expires_at = now() - 1;
    app.store.delete_request(&id).await.unwrap();
    app.store.create_request(&doc).await.unwrap();
    let response = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": "wrong", "cookies": cookies_for(MINE) }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn callback_with_dead_cookies_is_401_and_fails_the_request() {
    let app = TestApp::new();
    let (id, token) = phone_request(&app).await;
    let response = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": token, "cookies": cookies_for("JSESSIONID=dead") }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json(response).await["code"], "session_invalid");
    let doc = app.store.get_request(&id).await.unwrap().unwrap();
    assert_eq!(doc.status, vtop_bridge::store::RequestStatus::Error);
    assert!(doc.error.unwrap().starts_with("phone_session_invalid: "));
}

// ---- app secret -----------------------------------------------------------

#[tokio::test]
async fn account_changes_need_the_app_secret() {
    let app = TestApp::new();
    let secret = linked(&app, MINE, json!({})).await;
    let key = create_key(&app, MINE, &secret).await;
    for (path, extra) in [
        ("/v1/keys", json!({ "label": "x" })),
        ("/v1/keys/revoke", json!({ "id": key["id"] })),
        ("/v1/account/fcm-token", json!({ "fcmToken": "evil" })),
        ("/v1/account/delete", json!({})),
    ] {
        for bad in [None, Some("vta_wrong")] {
            let mut body = extra.clone();
            if let Some(bad) = bad {
                body["appSecret"] = json!(bad);
            }
            let response = app.post(path, with_session(MINE, body)).await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
            assert_eq!(json(response).await["code"], "app_secret_invalid");
        }
    }
    assert!(app.store.get_account(REG).await.unwrap().is_some());
    assert_eq!(
        app.accounts().fcm_token(REG).await.unwrap().as_deref(),
        Some("fcm-1")
    );
    let key = key["key"].as_str().unwrap();
    assert!(app.state.keys.resolve(key, now()).await.unwrap().is_some());
}

#[tokio::test]
async fn relink_without_the_secret_revokes_keys_and_wipes_the_vault() {
    let app = TestApp::new();
    let old = linked(&app, MINE, json!({ "credentials": credentials() })).await;
    let key = create_key(&app, MINE, &old).await["key"]
        .as_str()
        .unwrap()
        .to_string();

    let new = linked(&app, MINE, json!({ "fcmToken": "fcm-2" })).await;
    assert_ne!(old, new);
    assert_eq!(app.state.keys.resolve(&key, now()).await.unwrap(), None);
    assert!(app.accounts().vault(REG).await.unwrap().is_none());
    assert_eq!(
        app.accounts().fcm_token(REG).await.unwrap().as_deref(),
        Some("fcm-2")
    );
    let stale = app
        .post("/v1/keys", with_session(MINE, json!({ "appSecret": old })))
        .await;
    assert_eq!(stale.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn relink_with_the_secret_keeps_keys_and_vault() {
    let app = TestApp::new();
    let old = linked(&app, MINE, json!({ "credentials": credentials() })).await;
    let key = create_key(&app, MINE, &old).await["key"]
        .as_str()
        .unwrap()
        .to_string();

    let new = linked(&app, MINE, json!({ "appSecret": old })).await;
    assert!(app.state.keys.resolve(&key, now()).await.unwrap().is_some());
    assert!(app.accounts().vault(REG).await.unwrap().is_some());
    create_key(&app, MINE, &new).await;
}

#[tokio::test]
async fn account_status_without_the_secret_hides_keys() {
    let app = TestApp::new();
    let secret = linked(&app, MINE, json!({})).await;
    create_key(&app, MINE, &secret).await;
    let account = json(app.post("/v1/account", with_session(MINE, json!({}))).await).await;
    assert_eq!(account["linked"], true);
    assert_eq!(account["thisPhone"], false);
    assert_eq!(account["keys"], json!([]));
}

#[tokio::test]
async fn forgetting_credentials_needs_no_secret() {
    let app = TestApp::new();
    linked(&app, MINE, json!({ "credentials": credentials() })).await;
    let response = app
        .post(
            "/v1/account/forget-credentials",
            with_session(MINE, json!({})),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(app.accounts().vault(REG).await.unwrap().is_none());
}

// ---- settings and cache ---------------------------------------------------

#[tokio::test]
async fn settings_start_at_the_defaults_and_can_be_changed() {
    let app = TestApp::new();
    let secret = linked(&app, MINE, json!({})).await;
    let account = |secret: String| {
        let app = &app;
        async move {
            json(
                app.post(
                    "/v1/account",
                    with_session(MINE, json!({ "appSecret": secret })),
                )
                .await,
            )
            .await
        }
    };
    assert_eq!(
        account(secret.clone()).await["settings"],
        json!({ "phoneWaitSecs": 20, "vaultTtlSecs": null, "alwaysUseVault": true })
    );
    let wanted = json!({ "phoneWaitSecs": 45, "vaultTtlSecs": 86400, "alwaysUseVault": false });
    let response = app
        .post(
            "/v1/account/settings",
            with_session(MINE, json!({ "appSecret": secret, "settings": wanted })),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(account(secret).await["settings"], wanted);
}

#[tokio::test]
async fn settings_need_the_app_secret_and_an_offered_choice() {
    let app = TestApp::new();
    let secret = linked(&app, MINE, json!({})).await;
    let good = json!({ "phoneWaitSecs": 10, "vaultTtlSecs": null, "alwaysUseVault": true });
    let without = app
        .post(
            "/v1/account/settings",
            with_session(MINE, json!({ "settings": good })),
        )
        .await;
    assert_eq!(without.status(), StatusCode::FORBIDDEN);
    for bad in [
        json!({ "phoneWaitSecs": 0, "vaultTtlSecs": null, "alwaysUseVault": true }),
        json!({ "phoneWaitSecs": 7, "vaultTtlSecs": null, "alwaysUseVault": true }),
        json!({ "phoneWaitSecs": 10, "vaultTtlSecs": 5, "alwaysUseVault": true }),
        json!({ "phoneWaitSecs": 10 }),
    ] {
        let response = app
            .post(
                "/v1/account/settings",
                with_session(MINE, json!({ "appSecret": secret, "settings": bad })),
            )
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{bad}");
    }
}

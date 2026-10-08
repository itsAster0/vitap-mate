//! The bearer-key session API every client uses.

mod common;

use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use common::{json, TestApp, REG};
use serde_json::json;
use vtop_bridge::accounts::Vault;
use vtop_bridge::fakes::session;
use vtop_bridge::gmail::GmailGrant;
use vtop_bridge::store::Store;

fn now() -> u64 {
    vtop_core::now_unix()
}

fn vault() -> Vault {
    Vault {
        username: "yaswanth314".into(),
        password: "pw".into(),
        gmail: GmailGrant {
            refresh_token: "r".into(),
            client_id: "c".into(),
            client_secret: None,
            delete_after_reading: false,
        },
    }
}

async fn key_for(app: &TestApp, registration_number: &str) -> String {
    app.state
        .keys
        .create(registration_number, "test", now())
        .await
        .unwrap()
        .1
}

fn with_key(request: axum::http::request::Builder, key: &str) -> axum::http::request::Builder {
    request.header(header::AUTHORIZATION, format!("Bearer {key}"))
}

async fn post_session(app: &TestApp, key: &str) -> axum::http::Response<Body> {
    app.send(
        with_key(Request::post("/v1/session"), key)
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

async fn poll(app: &TestApp, key: &str, id: &str) -> axum::http::Response<Body> {
    app.send(
        with_key(Request::get(format!("/v1/session/requests/{id}")), key)
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

/// Polls until the request leaves pending; returns (status, body).
async fn settle(app: &TestApp, key: &str, id: &str) -> (StatusCode, serde_json::Value) {
    for _ in 0..400 {
        let response = poll(app, key, id).await;
        let status = response.status();
        let body = json(response).await;
        if body["status"] != "pending" {
            return (status, body);
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("request stayed pending");
}

#[tokio::test]
async fn no_or_unknown_key_is_401() {
    let app = TestApp::new();
    let none = app
        .send(Request::post("/v1/session").body(Body::empty()).unwrap())
        .await;
    assert_eq!(none.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json(none).await["code"], "key_unknown");
    let unknown = post_session(&app, &vtop_bridge::keys::new_key()).await;
    assert_eq!(unknown.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn whoami_names_the_account() {
    let app = TestApp::new();
    let key = key_for(&app, REG).await;
    let response = app
        .send(
            with_key(Request::get("/v1/whoami"), &key)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json(response).await, json!({ "registrationNumber": REG }));
}

#[tokio::test]
async fn ready_from_cache_carries_session_and_cookies() {
    let app = TestApp::new();
    let key = key_for(&app, REG).await;
    app.accounts()
        .record_phone(REG, Some("fcm"), &session("JSESSIONID=abc; REG=22BCE0001"), None, now())
        .await
        .unwrap();
    let response = post_session(&app, &key).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["status"], "ready");
    assert_eq!(body["session"]["cookies"], "JSESSIONID=abc; REG=22BCE0001");
    assert_eq!(body["cookies"][0]["name"], "JSESSIONID");
    assert_eq!(body["cookies"][0]["domain"], "vtop.vitap.ac.in");
}

#[tokio::test]
async fn pending_then_ready_from_vault_login() {
    let app = TestApp::new();
    let key = key_for(&app, REG).await;
    app.accounts()
        .record_phone(REG, Some("fcm"), &session("A=old"), Some(vault()), 1)
        .await
        .unwrap();
    let response = post_session(&app, &key).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert!(response.headers().contains_key(header::RETRY_AFTER));
    let body = json(response).await;
    assert_eq!(body["status"], "pending");
    assert_eq!(body["pollAfterMs"], 1500);
    let id = body["requestId"].as_str().unwrap().to_string();
    let (status, body) = settle(&app, &key, &id).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
    assert_eq!(body["session"]["cookies"], "JSESSIONID=fresh");
}

#[tokio::test]
async fn poll_other_accounts_request_is_404() {
    let app = TestApp::new();
    let mine = key_for(&app, REG).await;
    let theirs = key_for(&app, "21BCE9999").await;
    app.accounts()
        .record_phone(REG, Some("fcm"), &session("A=old"), None, 1)
        .await
        .unwrap();
    let id = json(post_session(&app, &mine).await).await["requestId"]
        .as_str()
        .unwrap()
        .to_string();
    let response = poll(&app, &theirs, &id).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(json(response).await["code"], "request_not_found");
}

#[tokio::test]
async fn phone_unreachable_is_reported() {
    let app = TestApp::new();
    let key = key_for(&app, REG).await;
    app.accounts()
        .record_phone(REG, None, &session("A=old"), None, 1)
        .await
        .unwrap();
    let id = json(post_session(&app, &key).await).await["requestId"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, body) = settle(&app, &key, &id).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "error");
    assert_eq!(body["code"], "phone_unreachable");
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("Reconnect this phone"));
}

#[tokio::test]
async fn expired_request_is_410() {
    let app = TestApp::new();
    let key = key_for(&app, REG).await;
    let doc = vtop_bridge::store::RequestDoc {
        id: uuid::Uuid::new_v4().to_string(),
        response_token: Some("t".into()),
        status: vtop_bridge::store::RequestStatus::Pending,
        cookies_json: None,
        error: None,
        fcm_hash: None,
        want_credentials: false,
        account: Some(REG.into()),
        created_at: 1,
        expires_at: 2,
    };
    app.store.create_request(&doc).await.unwrap();
    let response = poll(&app, &key, &doc.id).await;
    assert_eq!(response.status(), StatusCode::GONE);
    assert_eq!(json(response).await["code"], "request_expired");
}

#[tokio::test]
async fn expire_drops_the_cache() {
    let app = TestApp::new();
    let key = key_for(&app, REG).await;
    app.accounts()
        .record_phone(REG, Some("fcm"), &session("A=1; REG=22BCE0001"), None, now())
        .await
        .unwrap();
    let expire = app
        .send(
            with_key(Request::post("/v1/session/expire"), &key)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(expire.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        post_session(&app, &key).await.status(),
        StatusCode::ACCEPTED
    );
}

#[tokio::test]
async fn rate_limited_per_account() {
    let app = TestApp::new();
    let key = key_for(&app, REG).await;
    let other = key_for(&app, REG).await;
    app.accounts()
        .record_phone(REG, Some("fcm"), &session("A=1; REG=22BCE0001"), None, now())
        .await
        .unwrap();
    for _ in 0..60 {
        assert_eq!(post_session(&app, &key).await.status(), StatusCode::OK);
    }
    // The limit is per account, so a second key of the same account is limited too.
    let limited = post_session(&app, &other).await;
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(json(limited).await["code"], "rate_limited");
}

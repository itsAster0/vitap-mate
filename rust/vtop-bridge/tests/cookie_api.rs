//! Port of fmc-token-go/server_test.go plus the bridge's own cases.

mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use common::{json, TestApp};
use serde_json::json;
use vtop_bridge::fcm::FcmError;
use vtop_bridge::store::{RequestDoc, RequestStatus, Store};

fn cookies() -> serde_json::Value {
    json!([{ "domain": "vtop.vitap.ac.in", "name": "JSESSIONID", "path": "/", "value": "abc123" }])
}

/// Starts a request and returns (requestId, responseToken).
async fn start(app: &TestApp) -> (String, String) {
    let response = app.post("/cookie", json!({ "fcmToken": "abc" })).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let body = json(response).await;
    let id = body["requestId"].as_str().unwrap().to_string();
    (id, app.last_fcm()["responseToken"].clone())
}

fn pending(id: &str, expires_at: u64) -> RequestDoc {
    RequestDoc {
        id: id.into(),
        response_token: Some("secret".into()),
        status: RequestStatus::Pending,
        cookies_json: None,
        error: None,
        fcm_hash: None,
        want_credentials: false,
        account: None,
        created_at: 1,
        expires_at,
    }
}

fn now() -> u64 {
    vtop_core::now_unix()
}

#[tokio::test]
async fn cookie_starts_pollable_request() {
    let app = TestApp::new();
    let response = app.post("/cookie", json!({ "fmcToken": "abc" })).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(response.headers()[header::RETRY_AFTER], "2");
    let started = json(response).await;
    assert_eq!(started["status"], "pending");
    assert_eq!(started["pollAfterMs"], 1500);
    assert!(started["expiresAt"].as_str().unwrap().ends_with('Z'));
    let id = started["requestId"].as_str().unwrap();
    assert!(uuid::Uuid::parse_str(id).is_ok());

    let (token, data) = app.messenger.sends().last().cloned().unwrap();
    assert_eq!(token, "abc");
    assert_eq!(data["type"], "vtop_cookie_request");
    assert_eq!(data["requestId"], id);
    assert_eq!(
        data["callbackUrl"],
        "https://bridge.example.test/cookie/callback"
    );
    assert_eq!(data["responseToken"].len(), 64);

    let status = app.get(&format!("/cookie/status/{id}")).await;
    assert_eq!(status.status(), StatusCode::ACCEPTED);
    assert_eq!(json(status).await, json!({ "status": "pending" }));
}

#[tokio::test]
async fn legacy_cookie_never_asks_for_credentials() {
    // The legacy flow records nothing, so the phone must not send a password.
    let app = TestApp::new();
    start(&app).await;
    assert_eq!(app.last_fcm()["wantCredentials"], "0");
}

#[tokio::test]
async fn callback_completes_request_and_status_keeps_cookies_until_acknowledged() {
    let app = TestApp::new();
    let (id, token) = start(&app).await;
    let callback = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": token, "cookies": cookies() }),
        )
        .await;
    assert_eq!(callback.status(), StatusCode::OK);
    assert_eq!(json(callback).await, json!({ "ok": true }));

    for _ in 0..2 {
        let status = app.get(&format!("/cookie/status/{id}")).await;
        assert_eq!(status.status(), StatusCode::OK);
        let body = json(status).await;
        assert_eq!(body["status"], "success");
        assert_eq!(body["cookies"][0]["value"], "abc123");
    }

    let ack = app.delete(&format!("/cookie/status/{id}")).await;
    assert_eq!(ack.status(), StatusCode::NO_CONTENT);
    let gone = app.get(&format!("/cookie/status/{id}")).await;
    assert_eq!(gone.status(), StatusCode::NOT_FOUND);
    assert_eq!(json(gone).await["code"], "REQUEST_NOT_FOUND");
}

#[tokio::test]
async fn callback_error_is_reported_by_status() {
    let app = TestApp::new();
    let (id, token) = start(&app).await;
    let callback = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": token, "error": "No account" }),
        )
        .await;
    assert_eq!(callback.status(), StatusCode::OK);
    let status = app.get(&format!("/cookie/status/{id}")).await;
    assert_eq!(status.status(), StatusCode::OK);
    assert_eq!(
        json(status).await,
        json!({ "status": "error", "error": "No account" })
    );
}

#[tokio::test]
async fn cookie_rate_limit() {
    let app = TestApp::new();
    for i in 0..10 {
        let response = app.post("/cookie", json!({ "fcmToken": "abc" })).await;
        assert_eq!(response.status(), StatusCode::ACCEPTED, "request {}", i + 1);
    }
    let limited = app.post("/cookie", json!({ "fcmToken": "abc" })).await;
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.headers()[header::RETRY_AFTER], "60");
    assert_eq!(json(limited).await["code"], "RATE_LIMITED");
}

#[tokio::test]
async fn rate_limit_is_per_forwarded_client() {
    let app = TestApp::new();
    let post = |ip: &'static str| {
        Request::post("/cookie")
            .header("content-type", "application/json")
            .header("x-forwarded-for", format!("{ip}, 10.0.0.1"))
            .body(Body::from(json!({ "fcmToken": "abc" }).to_string()))
            .unwrap()
    };
    for _ in 0..10 {
        assert_eq!(
            app.send(post("1.1.1.1")).await.status(),
            StatusCode::ACCEPTED
        );
    }
    assert_eq!(
        app.send(post("1.1.1.1")).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        app.send(post("2.2.2.2")).await.status(),
        StatusCode::ACCEPTED
    );
}

#[tokio::test]
async fn callback_rejects_wrong_token() {
    let app = TestApp::new();
    let (id, _) = start(&app).await;
    let response = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": "wrong", "cookies": cookies() }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(json(response).await["code"], "CALLBACK_NOT_FOUND");
}

#[tokio::test]
async fn cookie_deletes_request_when_fcm_send_fails() {
    for (error, status, code) in [
        (
            FcmError::Unavailable,
            StatusCode::BAD_GATEWAY,
            "FCM_UNAVAILABLE",
        ),
        (FcmError::TokenInvalid, StatusCode::GONE, "TOKEN_INVALID"),
        (
            FcmError::Misconfigured,
            StatusCode::SERVICE_UNAVAILABLE,
            "FCM_MISCONFIGURED",
        ),
    ] {
        let app = TestApp::new();
        *app.messenger.result.lock().unwrap() = Err(error);
        let response = app.post("/cookie", json!({ "fcmToken": "abc" })).await;
        assert_eq!(response.status(), status);
        assert_eq!(json(response).await["code"], code);
        let id = app.last_fcm()["requestId"].clone();
        assert!(app.store.get_request(&id).await.unwrap().is_none());
    }
}

#[tokio::test]
async fn cookie_validates_body() {
    let app = TestApp::new();
    let bad_json = app
        .send(
            Request::post("/cookie")
                .header("content-type", "application/json")
                .body(Body::from("{"))
                .unwrap(),
        )
        .await;
    assert_eq!(bad_json.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json(bad_json).await["code"], "INVALID_REQUEST");

    let missing = app.post("/cookie", json!({ "fcmToken": "  " })).await;
    assert_eq!(json(missing).await["code"], "TOKEN_REQUIRED");

    let long = app
        .post("/cookie", json!({ "fcmToken": "x".repeat(4097) }))
        .await;
    assert_eq!(long.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json(long).await["code"], "TOKEN_INVALID");
}

#[tokio::test]
async fn cookie_store_down_is_503() {
    let app = TestApp::new();
    *app.store.fail.lock().unwrap() = Some("down".into());
    let response = app.post("/cookie", json!({ "fcmToken": "abc" })).await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json(response).await["code"], "STORE_UNAVAILABLE");
    assert!(app.messenger.sends().is_empty());
}

#[tokio::test]
async fn cors_allows_configured_origin_and_delete() {
    let app = TestApp::with_config(common::config_with(&[(
        "ALLOWED_ORIGINS",
        "chrome-extension://allowed",
    )]));
    let response = app
        .send(
            Request::options("/cookie/status/anything")
                .header("origin", "chrome-extension://allowed")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let headers = response.headers();
    assert_eq!(
        headers["access-control-allow-origin"],
        "chrome-extension://allowed"
    );
    assert_eq!(
        headers["access-control-allow-methods"],
        "GET, POST, DELETE, OPTIONS"
    );
    assert_eq!(headers["access-control-allow-headers"], "Content-Type");

    let other = app
        .send(
            Request::get("/healthz")
                .header("origin", "https://evil.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert!(other.headers().get("access-control-allow-origin").is_none());
}

#[tokio::test]
async fn cors_defaults_to_any_origin() {
    let app = TestApp::new();
    let response = app.get("/healthz").await;
    assert_eq!(response.headers()["access-control-allow-origin"], "*");
}

#[tokio::test]
async fn status_expires_and_deletes_request() {
    let app = TestApp::new();
    let id = uuid::Uuid::new_v4().to_string();
    app.store
        .create_request(&pending(&id, now() - 1))
        .await
        .unwrap();
    let response = app.get(&format!("/cookie/status/{id}")).await;
    assert_eq!(response.status(), StatusCode::GONE);
    assert_eq!(json(response).await["code"], "REQUEST_EXPIRED");
    assert!(app.store.get_request(&id).await.unwrap().is_none());
}

#[tokio::test]
async fn status_validates_request_id() {
    let app = TestApp::new();
    let bad = app.get("/cookie/status/not-a-uuid").await;
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json(bad).await["code"], "REQUEST_ID_INVALID");
    let delete = app.delete("/cookie/status/not-a-uuid").await;
    assert_eq!(delete.status(), StatusCode::BAD_REQUEST);
    let unknown = app
        .get(&format!("/cookie/status/{}", uuid::Uuid::new_v4()))
        .await;
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn late_callback_after_expiry_is_gone_and_doc_deleted() {
    let app = TestApp::new();
    let id = uuid::Uuid::new_v4().to_string();
    app.store
        .create_request(&pending(&id, now() - 1))
        .await
        .unwrap();
    let response = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": "secret", "cookies": cookies() }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::GONE);
    assert_eq!(json(response).await["code"], "REQUEST_EXPIRED");
    assert!(app.store.get_request(&id).await.unwrap().is_none());
}

#[tokio::test]
async fn second_callback_conflicts_and_keeps_first_cookies() {
    let app = TestApp::new();
    let (id, token) = start(&app).await;
    app.post(
        "/cookie/callback",
        json!({ "requestId": id, "responseToken": token, "cookies": cookies() }),
    )
    .await;
    let other = json!([{ "domain": "vtop.vitap.ac.in", "name": "JSESSIONID", "value": "evil" }]);
    let second = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": token, "cookies": other }),
        )
        .await;
    assert_eq!(second.status(), StatusCode::CONFLICT);
    assert_eq!(json(second).await["code"], "REQUEST_COMPLETED");
    let status = json(app.get(&format!("/cookie/status/{id}")).await).await;
    assert_eq!(status["cookies"][0]["value"], "abc123");
}

#[tokio::test]
async fn callback_validates_body() {
    let app = TestApp::new();
    let (id, token) = start(&app).await;
    let missing = app
        .post("/cookie/callback", json!({ "requestId": id }))
        .await;
    assert_eq!(json(missing).await["code"], "CALLBACK_INVALID");
    let bad_id = app
        .post(
            "/cookie/callback",
            json!({ "requestId": "x", "responseToken": token }),
        )
        .await;
    assert_eq!(json(bad_id).await["code"], "REQUEST_ID_INVALID");
    let empty = app
        .post(
            "/cookie/callback",
            json!({ "requestId": id, "responseToken": token, "cookies": [] }),
        )
        .await;
    assert_eq!(empty.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json(empty).await["code"], "COOKIES_INVALID");
}

#[tokio::test]
async fn legacy_status_hides_account_requests() {
    let app = TestApp::new();
    let id = uuid::Uuid::new_v4().to_string();
    let mut doc = pending(&id, now() + 60);
    doc.account = Some("22BCE0001".into());
    doc.status = RequestStatus::Success;
    doc.cookies_json = Some(cookies().to_string());
    app.store.create_request(&doc).await.unwrap();

    let read = app.get(&format!("/cookie/status/{id}")).await;
    assert_eq!(read.status(), StatusCode::NOT_FOUND);
    let delete = app.delete(&format!("/cookie/status/{id}")).await;
    assert_eq!(delete.status(), StatusCode::NOT_FOUND);
    assert!(app.store.get_request(&id).await.unwrap().is_some());
}

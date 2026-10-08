mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};

#[tokio::test]
async fn healthz_reports_ok_and_is_not_cached() {
    let app = common::TestApp::new();
    let response = app
        .send(Request::get("/healthz").body(Body::empty()).unwrap())
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        common::json(response).await,
        serde_json::json!({"status": "ok", "service": "vtop-bridge"})
    );
}

#[tokio::test]
async fn healthz_is_503_when_store_fails() {
    let app = common::TestApp::new();
    *app.store.fail.lock().unwrap() = Some("down".into());
    let response = app
        .send(Request::get("/healthz").body(Body::empty()).unwrap())
        .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        common::json(response).await,
        serde_json::json!({"status": "unavailable", "service": "vtop-bridge"})
    );
}

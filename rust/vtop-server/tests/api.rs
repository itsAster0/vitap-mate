//! End-to-end tests of the router against an in-process mock VTOP.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tower::ServiceExt;
use vtop_server::config::ServerConfig;
use vtop_server::{app, AppState};

const FULL_ATTENDANCE_PAGE: &str = "<table><tr><th>a</th></tr><tr><th>b</th></tr><tr><th>c</th></tr><tr><td>1</td><td>01-Jul-2026</td><td>A1</td><td>WED</td><td>Present</td><td></td></tr></table>";
const KEY: &str = "test-key-0123456789abcdef";
const CONTENT_PAGE: &str = r#"<input name="_csrf" value="csrf-1"><input type="hidden" name="authorizedIDX" value="22BCE0001">"#;
const ATTENDANCE_PAGE: &str = r#"<table><tr><th>h</th></tr><tr><td>1</td><td>PC</td><td>Maths</td><td>MAT1001</td><td>Dr X</td><td>10</td><td>12</td><td>83%</td><td>0</td><td>-</td><td><a onclick="javascript:processViewAttendanceDetail('AP2026','22BCE0001','AP2026001','ETH');">v</a></td></tr></table>"#;

#[derive(Clone, Default)]
struct MockVtop {
    hits: Arc<Mutex<HashMap<String, usize>>>,
    expired: Arc<Mutex<bool>>,
}

impl MockVtop {
    fn hits(&self, path: &str) -> usize {
        self.hits.lock().unwrap().get(path).copied().unwrap_or(0)
    }

    async fn start(&self) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mock = self.clone();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mock = mock.clone();
                tokio::spawn(async move {
                    let mut buffer = vec![0u8; 16 * 1024];
                    while let Ok(read) = socket.read(&mut buffer).await {
                        if read == 0 {
                            return;
                        }
                        let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                        let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
                        *mock.hits.lock().unwrap().entry(path.clone()).or_default() += 1;
                        let response = if *mock.expired.lock().unwrap() && path != "/vtop/login" {
                            "HTTP/1.1 302 Found\r\nLocation: /vtop/login\r\nContent-Length: 0\r\n\r\n"
                                .to_string()
                        } else {
                            let body = match path.as_str() {
                                "/vtop/content" => CONTENT_PAGE,
                                "/vtop/processViewStudentAttendance" => ATTENDANCE_PAGE,
                                "/vtop/processViewAttendanceDetail" => FULL_ATTENDANCE_PAGE,
                                _ => "<html>login</html>",
                            };
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                                body.len()
                            )
                        };
                        if socket.write_all(response.as_bytes()).await.is_err() {
                            return;
                        }
                    }
                });
            }
        });
        format!("http://{address}")
    }
}

fn open_config(base_url: &str) -> ServerConfig {
    let base_url = base_url.to_string();
    ServerConfig::from_lookup(move |name| (name == "VTOP_BASE_URL").then(|| base_url.clone()))
        .unwrap()
}

fn config(base_url: &str, extra: &[(&str, &str)]) -> ServerConfig {
    let mut vars: HashMap<String, String> =
        [("VTOP_SERVER_API_KEYS", KEY), ("VTOP_BASE_URL", base_url)]
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
    for (key, value) in extra {
        vars.insert(key.to_string(), value.to_string());
    }
    ServerConfig::from_lookup(|name| vars.get(name).cloned()).unwrap()
}

async fn send(
    router: &axum::Router,
    path: &str,
    key: Option<&str>,
    body: Value,
) -> (StatusCode, Option<String>, Value) {
    let mut request = Request::post(path).header("content-type", "application/json");
    if let Some(key) = key {
        request = request.header("x-api-key", key);
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let session = response
        .headers()
        .get("x-vtop-session")
        .map(|value| value.to_str().unwrap().to_string());
    let retry_after = response.headers().get("retry-after").cloned();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let mut json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if let Some(retry_after) = retry_after {
        json["retry_after"] = json!(retry_after.to_str().unwrap());
    }
    (status, session, json)
}

fn attendance_body() -> Value {
    json!({"session": {"cookies": "JSESSIONID=abc"}, "semester_id": "AP2026"})
}

#[tokio::test]
async fn health_needs_no_key() {
    let router = app(Arc::new(AppState::new(config("http://127.0.0.1:9", &[]))));
    let response = router
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn data_routes_require_a_valid_key() {
    let router = app(Arc::new(AppState::new(config("http://127.0.0.1:9", &[]))));
    for key in [None, Some("wrong-key-0123456789abcdef"), Some("")] {
        let (status, _, body) = send(&router, "/v1/attendance", key, attendance_body()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"]["code"], "unauthorized");
    }
}

#[tokio::test]
async fn cookie_session_is_validated_once_and_cached() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let router = app(Arc::new(AppState::new(config(&base, &[]))));

    let (status, session, body) =
        send(&router, "/v1/attendance", Some(KEY), attendance_body()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["records"][0]["course_id"], "AP2026001");
    assert_eq!(body["semester_id"], "AP2026");

    let session: Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(session.unwrap()).unwrap()).unwrap();
    assert_eq!(session["csrf_token"], "csrf-1");
    assert_eq!(session["registration_number"], "22BCE0001");

    let (status, _, _) = send(&router, "/v1/attendance", Some(KEY), attendance_body()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mock.hits("/vtop/content"), 1);
    assert_eq!(mock.hits("/vtop/processViewStudentAttendance"), 2);
}

#[tokio::test]
async fn expired_session_maps_to_401_and_leaves_the_cache() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let state = Arc::new(AppState::new(config(&base, &[])));
    let router = app(state.clone());
    send(&router, "/v1/attendance", Some(KEY), attendance_body()).await;
    assert_eq!(state.cache.len(), 1);

    *mock.expired.lock().unwrap() = true;
    let (status, _, body) = send(&router, "/v1/attendance", Some(KEY), attendance_body()).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "session_expired");
    assert_eq!(state.cache.len(), 0);
}

#[tokio::test]
async fn rate_limit_applies_per_key() {
    let router = app(Arc::new(AppState::new(config(
        "http://127.0.0.1:9",
        &[("VTOP_SERVER_RATE_LIMIT_PER_MINUTE", "2")],
    ))));
    let bad = json!({"session": {"cookies": ""}, "semester_id": "AP2026"});
    for _ in 0..2 {
        let (status, _, _) = send(&router, "/v1/attendance", Some(KEY), bad.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (status, _, body) = send(&router, "/v1/attendance", Some(KEY), bad).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["error"]["code"], "rate_limited");
    assert_eq!(body["retry_after"], "30");
}

#[tokio::test]
async fn bad_input_is_a_400_without_echoing_values() {
    let router = app(Arc::new(AppState::new(config("http://127.0.0.1:9", &[]))));
    let (status, _, body) = send(
        &router,
        "/v1/attendance",
        Some(KEY),
        json!({"session": {"cookies": 12345}, "semester_id": "AP2026"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!body.to_string().contains("12345"));

    let (status, _, body) = send(
        &router,
        "/v1/biometric",
        Some(KEY),
        json!({"session": {"cookies": "JSESSIONID=abc"}, "date": "31/02/2026"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_input");
}

#[tokio::test]
async fn login_can_be_disabled() {
    let router = app(Arc::new(AppState::new(config(
        "http://127.0.0.1:9",
        &[("VTOP_SERVER_ALLOW_LOGIN", "false")],
    ))));
    let (status, _, body) = send(
        &router,
        "/v1/auth/login",
        Some(KEY),
        json!({"username": "22BCE0001", "password": "secret-password"}),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "login_disabled");
    assert!(!body.to_string().contains("secret-password"));
}

#[tokio::test]
async fn otp_without_a_pending_challenge_is_rejected() {
    let router = app(Arc::new(AppState::new(config("http://127.0.0.1:9", &[]))));
    let (status, _, body) = send(
        &router,
        "/v1/auth/otp",
        Some(KEY),
        json!({"session": {"cookies": "JSESSIONID=abc"}, "otp": "123456"}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "authentication_failed");
    assert!(!body.to_string().contains("123456"));
}

#[tokio::test]
async fn simultaneous_first_requests_validate_once() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let router = app(Arc::new(AppState::new(config(&base, &[]))));

    let (a, b, c) = tokio::join!(
        send(&router, "/v1/attendance", Some(KEY), attendance_body()),
        send(&router, "/v1/attendance", Some(KEY), attendance_body()),
        send(&router, "/v1/attendance", Some(KEY), attendance_body()),
    );

    assert!([a.0, b.0, c.0]
        .iter()
        .all(|status| *status == StatusCode::OK));
    assert_eq!(mock.hits("/vtop/content"), 1);
}

#[tokio::test]
async fn without_configured_keys_the_server_is_open() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let router = app(Arc::new(AppState::new(open_config(&base))));

    let (status, _, body) = send(&router, "/v1/attendance", None, attendance_body()).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // A stray header is ignored rather than rejected.
    let (status, _, _) = send(
        &router,
        "/v1/attendance",
        Some("anything"),
        attendance_body(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn refresh_returns_every_requested_part() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let router = app(Arc::new(AppState::new(config(&base, &[]))));

    let (status, _, body) = send(
        &router,
        "/v1/refresh",
        Some(KEY),
        json!({
            "session": {"cookies": "JSESSIONID=abc"},
            "semester_id": "AP2026",
            "include": ["attendance", "marks", "attendance"],
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["attendance"]["data"]["records"][0]["course_id"],
        "AP2026001"
    );
    // The mock answers marks with a page that has no marks rows.
    assert!(body["marks"]["data"]["records"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(body.get("timetable").is_none());
    assert_eq!(mock.hits("/vtop/processViewStudentAttendance"), 1);
}

#[tokio::test]
async fn refresh_fails_as_a_whole_when_the_session_expired() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    *mock.expired.lock().unwrap() = true;
    let router = app(Arc::new(AppState::new(config(&base, &[]))));

    let (status, _, body) = send(
        &router,
        "/v1/refresh",
        Some(KEY),
        json!({"session": {"cookies": "JSESSIONID=abc"}, "semester_id": "AP2026"}),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "session_expired");
}

#[tokio::test]
async fn responses_are_gzipped_on_request() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let router = app(Arc::new(AppState::new(config(&base, &[]))));
    let response = router
        .oneshot(
            Request::post("/v1/attendance")
                .header("content-type", "application/json")
                .header("x-api-key", KEY)
                .header("accept-encoding", "gzip")
                .body(Body::from(attendance_body().to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-encoding"], "gzip");
}

#[tokio::test]
async fn refresh_can_include_every_course_attendance_detail() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let router = app(Arc::new(AppState::new(config(&base, &[]))));

    let (status, _, body) = send(
        &router,
        "/v1/refresh",
        Some(KEY),
        json!({
            "session": {"cookies": "JSESSIONID=abc"},
            "semester_id": "AP2026",
            "include": ["full_attendance", "attendance"],
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let details = body["full_attendance"]["data"].as_array().unwrap();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0]["course_id"], "AP2026001");
    assert_eq!(details[0]["course_type"], "ETH");
    assert_eq!(details[0]["records"][0]["status"], "Present");
    // The attendance page is fetched once and reused for the details.
    assert_eq!(mock.hits("/vtop/processViewStudentAttendance"), 1);
}

#[tokio::test]
async fn full_attendance_is_not_in_the_default_refresh() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let router = app(Arc::new(AppState::new(config(&base, &[]))));
    let (status, _, body) = send(
        &router,
        "/v1/refresh",
        Some(KEY),
        json!({"session": {"cookies": "JSESSIONID=abc"}, "semester_id": "AP2026"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("full_attendance").is_none());
    assert_eq!(mock.hits("/vtop/processViewAttendanceDetail"), 0);
}

#[tokio::test]
async fn login_accepts_a_gmail_token_without_echoing_it() {
    let router = app(Arc::new(AppState::new(config(
        "http://127.0.0.1:9",
        &[("VTOP_SERVER_ALLOW_LOGIN", "false")],
    ))));
    let (status, _, body) = send(
        &router,
        "/v1/auth/login",
        Some(KEY),
        json!({
            "username": "22BCE0001",
            "password": "secret-password",
            "gmail": {"access_token": "ya29.secret", "expires_at": 1, "delete_after_reading": true}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(!body.to_string().contains("ya29"));
}

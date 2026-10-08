use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use vtop_core::SessionState;
use vtop_mcp::bridge::{Identity, SessionSource};
use vtop_mcp::config::Config;
use vtop_mcp::router;

const KEY: &str = "vtm_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[derive(Default)]
struct RecordingSource {
    usernames: Mutex<Vec<String>>,
    whoami_calls: Mutex<usize>,
    revoked: Mutex<bool>,
}

#[async_trait]
impl SessionSource for RecordingSource {
    async fn whoami(&self, key: &str) -> Result<Option<Identity>, String> {
        *self.whoami_calls.lock().unwrap() += 1;
        Ok(
            (key == KEY && !*self.revoked.lock().unwrap()).then(|| Identity {
                registration_number: "22BCE0001".to_string(),
                key_label: Some("My laptop".to_string()),
                semester_id: Some("AP2026272".to_string()),
            }),
        )
    }

    async fn session(&self, key: &str) -> Result<SessionState, String> {
        self.usernames.lock().unwrap().push(key.to_string());
        Err("phone_unreachable: Open VITAP Mate → Connected apps → Reconnect this phone.".into())
    }

    async fn expire(&self, _username: &str) -> Result<(), String> {
        Ok(())
    }
}

fn config() -> Config {
    Config::from_lookup(|name| match name {
        "BRIDGE_URL" => Some("https://bridge.example".to_string()),
        "BRIDGE_SERVICE_KEY" => Some("0123456789abcdef0123456789abcdef".to_string()),
        "MCP_KEYS" => Some(format!("{KEY}=22bce0001")),
        _ => None,
    })
    .unwrap()
}

fn rpc(method: &str, params: Value) -> String {
    let mut params = params;
    params["_meta"] = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test", "version": "1.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    });
    json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }).to_string()
}

async fn call(
    source: Arc<RecordingSource>,
    bearer: Option<&str>,
    body: String,
) -> (StatusCode, Value) {
    let mut request = Request::post("/mcp")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("host", "mcp.example")
        .header("mcp-protocol-version", "2026-07-28");
    let parsed: Value = serde_json::from_str(&body).unwrap();
    request = request.header("mcp-method", parsed["method"].as_str().unwrap());
    if let Some(name) = parsed["params"]["name"].as_str() {
        request = request.header("mcp-name", name);
    }
    if let Some(bearer) = bearer {
        request = request.header("authorization", format!("Bearer {bearer}"));
    }
    let response = router(&config(), source)
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&bytes).to_string();
    // JSON, or an SSE stream whose data line holds the JSON.
    let json = serde_json::from_str(&text).unwrap_or_else(|_| {
        text.lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .filter_map(|data| serde_json::from_str(data.trim()).ok())
            .next_back()
            .unwrap_or(Value::Null)
    });
    (status, json)
}

#[tokio::test]
async fn health_needs_no_key() {
    let response = router(&config(), Arc::new(RecordingSource::default()))
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn mcp_without_bearer_is_401() {
    let source = Arc::new(RecordingSource::default());
    for bearer in [None, Some("vtm_wrong")] {
        let (status, _) = call(source.clone(), bearer, rpc("tools/list", json!({}))).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn mcp_tools_list_names_every_tool() {
    let (status, body) = call(
        Arc::new(RecordingSource::default()),
        Some(KEY),
        rpc("tools/list", json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let mut names: Vec<String> = body["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("no tools in {body}"))
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "get_academic_calendar",
            "get_attendance",
            "get_biometric",
            "get_course_classes",
            "get_course_detail",
            "get_courses",
            "get_exam_schedule",
            "get_full_attendance",
            "get_general_outing",
            "get_grade_details",
            "get_grade_history",
            "get_grades",
            "get_marks",
            "get_semesters",
            "get_session",
            "get_timetable",
            "get_weekend_outing",
            "whoami",
        ]
    );
}

#[tokio::test]
async fn bad_semester_id_is_tool_error() {
    let source = Arc::new(RecordingSource::default());
    let (status, body) = call(
        source.clone(),
        Some(KEY),
        rpc(
            "tools/call",
            json!({ "name": "get_attendance", "arguments": { "semester_id": " " } }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"]["isError"], true, "{body}");
    assert!(source.usernames.lock().unwrap().is_empty());
}

#[tokio::test]
async fn tools_use_the_callers_account_and_report_bridge_errors() {
    let source = Arc::new(RecordingSource::default());
    let (_, body) = call(
        source.clone(),
        Some(KEY),
        rpc(
            "tools/call",
            json!({ "name": "get_semesters", "arguments": {} }),
        ),
    )
    .await;
    assert_eq!(body["result"]["isError"], true, "{body}");
    assert!(body["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("phone_unreachable"));
    assert_eq!(*source.usernames.lock().unwrap(), vec![KEY.to_string()]);
}

/// Older clients: initialize, then plain requests without a session id.
#[tokio::test]
async fn legacy_protocol_clients_work_without_sessions() {
    let app = router(&config(), Arc::new(RecordingSource::default()));
    let post = |body: Value| {
        Request::post("/mcp")
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", format!("Bearer {KEY}"))
            .header("host", "mcp.example")
            .body(Body::from(body.to_string()))
            .unwrap()
    };
    let init = app
        .clone()
        .oneshot(post(
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": { "name": "claude-code", "version": "1" } } }),
        ))
        .await
        .unwrap();
    let status = init.status();
    let bytes = init.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    let list = app
        .oneshot(post(
            json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {} }),
        ))
        .await
        .unwrap();
    let status = list.status();
    let bytes = list.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        body["result"]["tools"].as_array().unwrap().len(),
        18,
        "{body}"
    );
}

#[tokio::test]
async fn key_checks_are_cached_and_a_revoked_key_fails_after_the_cache_expires() {
    let source = Arc::new(RecordingSource::default());
    let app = vtop_mcp::router_with_cache(
        &config(),
        source.clone(),
        std::time::Duration::from_millis(50),
    );
    let list = || {
        Request::post("/mcp")
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", format!("Bearer {KEY}"))
            .header("host", "mcp.example")
            .body(Body::from(
                json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": { "name": "t", "version": "1" } } })
                .to_string(),
            ))
            .unwrap()
    };
    assert_eq!(
        app.clone().oneshot(list()).await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(
        app.clone().oneshot(list()).await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(*source.whoami_calls.lock().unwrap(), 1);
    *source.revoked.lock().unwrap() = true;
    assert_eq!(
        app.clone().oneshot(list()).await.unwrap().status(),
        StatusCode::OK
    );
    tokio::time::sleep(std::time::Duration::from_millis(60)).await;
    assert_eq!(
        app.oneshot(list()).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
}

/// Agents that take only a URL put the key in the path.
#[tokio::test]
async fn key_in_the_url_path_works_without_a_header() {
    let source = Arc::new(RecordingSource::default());
    let request = |path: String| {
        Request::post(path)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("host", "mcp.example")
            .body(Body::from(
                json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": { "name": "t", "version": "1" } } })
                .to_string(),
            ))
            .unwrap()
    };
    let ok = router(&config(), source.clone())
        .oneshot(request(format!("/mcp/{KEY}")))
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    let wrong = router(&config(), source)
        .oneshot(request("/mcp/vtm_wrong".into()))
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn whoami_answers_from_the_bridge_without_a_vtop_session() {
    let source = Arc::new(RecordingSource::default());
    let (status, body) = call(
        source.clone(),
        Some(KEY),
        rpc("tools/call", json!({ "name": "whoami", "arguments": {} })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"]["isError"], false, "{body}");
    let text: Value =
        serde_json::from_str(body["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        text,
        json!({
            "registrationNumber": "22BCE0001",
            "keyLabel": "My laptop",
            "semesterId": "AP2026272"
        })
    );
    assert!(source.usernames.lock().unwrap().is_empty());
}

#[tokio::test]
async fn course_tools_reject_bad_ids_before_asking_for_a_session() {
    let source = Arc::new(RecordingSource::default());
    for (name, arguments) in [
        ("get_academic_calendar", json!({ "semester_id": " " })),
        ("get_courses", json!({ "semester_id": " " })),
        (
            "get_course_classes",
            json!({ "semester_id": "AP2026272", "course_id": "a/b" }),
        ),
        (
            "get_course_detail",
            json!({ "semester_id": "AP2026272", "erp_id": "1 2", "class_id": "x" }),
        ),
    ] {
        let (status, body) = call(
            source.clone(),
            Some(KEY),
            rpc(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["result"]["isError"], true, "{name}: {body}");
    }
    assert!(source.usernames.lock().unwrap().is_empty());
}

#[tokio::test]
async fn outing_tools_use_the_callers_session() {
    let source = Arc::new(RecordingSource::default());
    for name in ["get_general_outing", "get_weekend_outing", "get_session"] {
        let (_, body) = call(
            source.clone(),
            Some(KEY),
            rpc("tools/call", json!({ "name": name, "arguments": {} })),
        )
        .await;
        assert_eq!(body["result"]["isError"], true, "{body}");
        assert!(
            body["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .starts_with("phone_unreachable"),
            "{body}"
        );
    }
    assert_eq!(*source.usernames.lock().unwrap(), [KEY, KEY, KEY]);
}

#[tokio::test]
async fn discover_returns_the_instructions() {
    let (status, body) = call(
        Arc::new(RecordingSource::default()),
        Some(KEY),
        rpc("server/discover", json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let instructions = body["result"]["instructions"].as_str().unwrap_or_default();
    assert!(instructions.contains("marked current"), "{body}");
}

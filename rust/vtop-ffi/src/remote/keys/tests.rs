use std::fs;
use std::sync::{Arc, Mutex};

use super::*;
use crate::remote::consent;
use crate::test_edge::{Edge, keep_token, paths, serve, signed_in};

fn core(name: &str) -> Core {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-keys-{name}-{}", std::process::id()));
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

#[test]
fn pair_codes_read_like_the_edge() {
    let code = |text: &str| pair_code(text.into());
    assert_eq!(code("abcde-12345").as_deref(), Some("ABCDE12345"));
    assert_eq!(code(" ABCDE 12345 ").as_deref(), Some("ABCDE12345"));
    assert_eq!(code("OILAB-CDEFG").as_deref(), Some("011ABCDEFG"));
    assert_eq!(
        code("https://vmate.aster0.dev/pair#ABCDE-12345").as_deref(),
        Some("ABCDE12345")
    );
    // U is not Crockford; too short; too long.
    assert_eq!(code("ABCDU-12345"), None);
    assert_eq!(code("ABCD-1234"), None);
    assert_eq!(code("ABCDE-123456"), None);
    assert_eq!(code("https://vmate.aster0.dev/pair"), None);
}

#[test]
fn cookies_become_the_answer_list() {
    let list = cookie_list("JSESSIONID=abc; SERVERID=s1;  ; broken");
    assert_eq!(
        list,
        [
            json!({ "name": "JSESSIONID", "value": "abc", "domain": "vtop.vitap.ac.in" }),
            json!({ "name": "SERVERID", "value": "s1", "domain": "vtop.vitap.ac.in" }),
        ]
    );
}

#[test]
fn keys_read_from_the_account_view() {
    let views: Vec<KeyView> = serde_json::from_value(json!([
        { "id": "ab12cd34", "label": "Browser", "createdAt": 10, "lastUsedAt": 20, "phoneId": "p", "outings": false,
          "uses": [{ "at": 20, "via": "extension" }] },
        { "id": "ef56ab78", "label": "MCP", "createdAt": 11, "lastUsedAt": null, "phoneId": "p", "outings": true },
    ]))
    .unwrap();
    let keys: Vec<AccessKey> = views.into_iter().map(AccessKey::from).collect();
    assert_eq!(keys[0].kind, KeyKind::Browser);
    assert_eq!(
        keys[0].uses,
        [KeyUse {
            at: 20,
            via: KeyKind::Browser
        }]
    );
    assert_eq!(
        (keys[1].kind, keys[1].outings, keys[1].last_used_at),
        (KeyKind::Mcp, true, None)
    );
    assert!(keys[1].uses.is_empty());
}

#[test]
fn other_phones_with_keys_are_listed() {
    let key = |phone: &str| json!({ "id": "k", "label": "MCP", "createdAt": 1, "lastUsedAt": null, "phoneId": phone });
    let view: AppsView = serde_json::from_value(json!({
        "thisPhone": true,
        "phones": [
            { "id": "me", "label": "Pixel", "seenAt": 9, "thisPhone": true },
            { "id": "old", "label": "Pixel", "seenAt": 5, "thisPhone": false },
            { "id": "idle", "label": "Tab", "seenAt": 4, "thisPhone": false },
        ],
        "keys": [key("me"), key("old"), key("old"), key("gone")],
    }))
    .unwrap();
    assert!(view.this_phone);
    let found = elsewhere(&view.phones, &view.keys);
    assert_eq!(found.len(), 2);
    assert_eq!(
        found,
        [
            KeyPhone {
                owner: KeyOwner::Phone {
                    id: "old".into(),
                    label: Some("Pixel".into()),
                    seen_at: 5
                },
                keys: 2
            },
            KeyPhone {
                owner: KeyOwner::Unlinked,
                keys: 1
            },
        ]
    );
}

#[tokio::test]
async fn linking_gets_an_identity_and_sends_the_semester() {
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = signed_in(&core("link").dir, server.clone());
    keep_token(&core, &server);
    fs::write(core.at(crate::SELECTED), r#""AP2026""#).unwrap();
    can(&edge, "account:setSemester", 200, Value::Null);
    core.link_with("f-link", "Link").await.unwrap();
    assert_eq!(paths(&edge), ["auth:token", "account:setSemester"]);
    let edge = edge.lock().unwrap();
    assert_eq!(edge.requests[1].body, json!({ "semesterId": "AP2026" }));
}

#[tokio::test]
async fn keys_are_made_as_the_student() {
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = signed_in(&core("create").dir, server.clone());
    keep_token(&core, &server);
    let made =
        json!({ "id": "ab12cd34", "key": "vtm_secret", "mcpUrl": "https://vmate.aster0.dev/mcp" });
    can(&edge, "keys:create", 201, made);
    let key = core
        .create_key_with(
            "f-key",
            "Make an access key",
            true,
            Some("ABCDE12345".into()),
        )
        .await
        .unwrap();
    assert_eq!(
        (key.id.as_str(), key.key.as_str()),
        ("ab12cd34", "vtm_secret")
    );
    assert_eq!(paths(&edge), ["auth:token", "keys:create"]);
    let edge = edge.lock().unwrap();
    assert!(edge.requests.iter().all(|r| r.signed && r.flow == "f-key"));
    assert_eq!(
        edge.requests[1].body,
        json!({ "label": "Browser", "outings": true, "pairCode": "ABCDE12345" })
    );
}

#[test]
fn replies_parse_once_into_their_types() {
    let info = |body: Value| serde_json::from_value::<PairInfo>(body).ok();
    assert_eq!(
        info(json!({ "kind": "mcp", "name": "Claude", "host": "claude.ai" })),
        Some(PairInfo::Mcp {
            name: "Claude".into(),
            host: "claude.ai".into()
        })
    );
    assert_eq!(info(json!({ "kind": "browser" })), Some(PairInfo::Browser));
    assert_eq!(info(json!({ "kind": "watch" })), Some(PairInfo::Other));
    // An MCP client without its name is a broken answer, not the extension.
    assert_eq!(info(json!({ "kind": "mcp" })), None);
    let moved: Moved = serde_json::from_value(json!({ "moved": 3 })).unwrap();
    assert_eq!(moved.moved, 3);
    assert!(serde_json::from_value::<Moved>(json!({})).is_err());
    let made = json!({ "id": "k1", "key": "vk_x", "mcpUrl": "https://s/mcp", "app": null });
    let key: NewKey = serde_json::from_value(made).unwrap();
    assert_eq!(key.mcp_url, "https://s/mcp");
    assert!(serde_json::from_value::<NewKey>(json!({ "id": "k1", "key": "vk_x" })).is_err());
}

#[tokio::test]
async fn pair_codes_name_their_mcp_client() {
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = signed_in(&core("pair-app").dir, server.clone());
    keep_token(&core, &server);
    can(
        &edge,
        "pair:info",
        200,
        json!({ "kind": "mcp", "name": "Claude", "host": "claude.ai" }),
    );
    can(&edge, "pair:info", 200, json!({ "kind": "browser" }));
    let app = core.pair_app_with("f-pair", "ABCDE12345").await.unwrap();
    assert_eq!(
        app,
        Some(PairApp {
            name: "Claude".into(),
            host: "claude.ai".into()
        })
    );
    let browser = core.pair_app_with("f-pair", "ABCDE12345").await.unwrap();
    assert_eq!(browser, None);
    let edge = edge.lock().unwrap();
    // Over the socket with the JWT, as a query (no flow).
    assert_eq!(edge.requests[1].path, "pair:info");
    assert_eq!(edge.requests[1].body, json!({ "code": "ABCDE12345" }));
}

#[tokio::test]
async fn refusals_read_as_what_to_do() {
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = signed_in(&core("refused").dir, server.clone());
    keep_token(&core, &server);
    can(
        &edge,
        "keys:create",
        409,
        json!({ "code": "key_limit", "error": "x" }),
    );
    can(
        &edge,
        "keys:create",
        400,
        json!({ "code": "bad_request", "error": "x" }),
    );
    let reason = |result: Result<NewKey, CoreError>| match result {
        Err(CoreError::Other { reason }) => reason,
        other => panic!("{other:?}"),
    };
    let full = core.create_key_with("f", "Key", false, None).await;
    assert!(reason(full).starts_with("Revoke a key first"));
    let bad = core
        .create_key_with("f", "Key", false, Some("ABCDE12345".into()))
        .await;
    assert!(reason(bad).starts_with("Check the code"));
    assert_eq!(edge.lock().unwrap().requests[1].body["label"], "MCP");
}

#[tokio::test]
async fn account_calls_never_ask_or_send_the_session() {
    // Without an identity they prove; with no proof network (this test's) that fails first.
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = signed_in(&core("consent").dir, server);
    let gate = consent::tests::no(&core);
    assert!(core.connected_apps("f".into()).await.is_err());
    assert!(core.create_key("f2".into(), false, None).await.is_err());
    assert_eq!(gate.asks(), 0);
    assert!(edge.lock().unwrap().requests.is_empty());
}

#[tokio::test]
async fn handover_unattended_answers_with_error_not_cookies() {
    let core = core("handover");
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    fs::write(core.at(crate::remote::edge::SERVER_FILE), &server).unwrap();
    keep_token(&core, &server);
    can(&edge, "sessions:answer", 200, json!({ "ok": true }));
    consent::tests::signed_in(&core);
    // In the pocket, even with "don't ask again" ticked.
    let gate = consent::tests::install(&core, false, false);
    consent::tests::remember(&core, CookieKind::Handover);
    let answered = core
        .answer_session_request("f".into(), "r1".into(), None)
        .await;
    assert_eq!(answered, Err(CoreError::ConsentDenied));
    assert_eq!(gate.asks(), 0);
    let edge = edge.lock().unwrap();
    let answer = &edge
        .requests
        .iter()
        .find(|r| r.path == "sessions:answer")
        .unwrap()
        .body;
    assert_eq!(
        answer,
        &json!({ "requestId": "r1", "error": "The student hasn't allowed it on the phone." })
    );
    assert!(answer.get("cookies").is_none());
}

#[tokio::test]
async fn session_requests_get_cookies_or_why_not() {
    let core = core("answer");
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    fs::write(core.at(crate::remote::edge::SERVER_FILE), &server).unwrap();
    keep_token(&core, &server);
    can(&edge, "sessions:answer", 200, json!({ "ok": true }));
    core.answer_with("f-a", "r1", Ok("JSESSIONID=c".into()))
        .await
        .unwrap();
    // Signed out (no session in this core): the server hears why, the caller gets the error.
    can(&edge, "sessions:answer", 200, json!({ "ok": true }));
    let signed_out = core
        .answer_session_request("f-b".into(), "r2".into(), None)
        .await;
    assert_eq!(signed_out, Err(CoreError::NotSignedIn));

    let edge = edge.lock().unwrap();
    let answers: Vec<&Value> = edge
        .requests
        .iter()
        .filter(|r| r.path == "sessions:answer")
        .map(|r| &r.body)
        .collect();
    assert_eq!(
        answers,
        [
            &json!({ "requestId": "r1", "cookies": [{ "name": "JSESSIONID", "value": "c", "domain": "vtop.vitap.ac.in" }] }),
            &json!({ "requestId": "r2", "error": "The phone is signed out of VTOP." }),
        ]
    );
    assert!(edge.requests.iter().all(|r| r.signed));
}

#[tokio::test]
async fn pending_session_requests_come_from_the_server() {
    let core = core("pending");
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    fs::write(core.at(crate::remote::edge::SERVER_FILE), &server).unwrap();
    // No identity: none, and nothing sent.
    assert!(
        core.pending_session_requests("f".into())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(edge.lock().unwrap().requests.is_empty());
    keep_token(&core, &server);
    let pending =
        json!([{ "requestId": "r1", "expiresAt": 1_700_000_000_000_i64, "wantCredentials": true }]);
    can(&edge, "sessions:pending", 200, pending);
    let requests = core.pending_session_requests("f-p".into()).await.unwrap();
    assert_eq!(
        requests,
        [SessionRequest {
            request_id: "r1".into(),
            expires_at: 1_700_000_000_000,
            want_credentials: true,
        }]
    );
}

use std::fs;
use std::sync::{Arc, Mutex};

use super::*;
use crate::remote::consent;
use crate::test_edge::{Edge, keep_token, serve, signed_in};

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
    let view = json!({
        "phones": [
            { "id": "me", "label": "Pixel", "seenAt": 9, "thisPhone": true },
            { "id": "old", "label": "Pixel", "seenAt": 5, "thisPhone": false },
            { "id": "idle", "label": "Tab", "seenAt": 4, "thisPhone": false },
        ],
        "keys": [
            { "phoneId": "me" }, { "phoneId": "old" }, { "phoneId": "old" }, { "phoneId": "gone" },
        ],
    });
    let found = elsewhere(&view);
    assert_eq!(found.len(), 2);
    assert_eq!(
        (found[0].id.as_deref(), found[0].keys, found[0].seen_at),
        (Some("old"), 2, Some(5))
    );
    assert_eq!((found[1].id.as_deref(), found[1].keys), (None, 1));
}

#[tokio::test]
async fn linking_sends_the_account_token_and_no_key() {
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = signed_in(&core("link").dir, server.clone());
    keep_token(&core, &server);
    can(
        &edge,
        "/v1/link",
        200,
        json!({ "savedCredentials": false, "forwarding": false }),
    );
    core.link_with(&server, "f-link", "Link").await.unwrap();
    let edge = edge.lock().unwrap();
    let paths: Vec<&str> = edge.requests.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(paths, ["/v1/devices/register", "/v1/link"]);
    assert_eq!(
        edge.requests[1].body,
        json!({ "accountToken": "tok", "label": "vitapmate" })
    );
}

#[tokio::test]
async fn a_first_key_links_the_phone_then_is_made() {
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = signed_in(&core("create").dir, server.clone());
    keep_token(&core, &server);
    let not_linked = json!({ "code": "not_linked", "error": "Link first." });
    can(&edge, "/v1/keys", 409, not_linked);
    let made =
        json!({ "id": "ab12cd34", "key": "vtm_secret", "mcpUrl": "https://vmate.aster0.dev/mcp" });
    can(&edge, "/v1/keys", 201, made);
    can(
        &edge,
        "/v1/link",
        200,
        json!({ "savedCredentials": false, "forwarding": false }),
    );
    let key = core
        .create_key_with(
            &server,
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
    let edge = edge.lock().unwrap();
    let paths: Vec<&str> = edge.requests.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(
        paths,
        ["/v1/devices/register", "/v1/keys", "/v1/link", "/v1/keys"]
    );
    assert!(edge.requests.iter().all(|r| r.signed && r.flow == "f-key"));
    assert_eq!(
        edge.requests[1].body,
        json!({ "accountToken": "tok", "label": "Browser", "outings": true, "pairCode": "ABCDE12345" })
    );
    assert_eq!(
        edge.requests[2].body,
        json!({ "accountToken": "tok", "label": "vitapmate" })
    );
}

#[tokio::test]
async fn pair_codes_name_their_mcp_client() {
    let core = core("pair-app");
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    can(
        &edge,
        "/v1/pair/info",
        200,
        json!({ "kind": "mcp", "name": "Claude", "host": "claude.ai" }),
    );
    can(&edge, "/v1/pair/info", 200, json!({ "kind": "browser" }));
    let app = core
        .pair_app_with(&server, "f-pair", "ABCDE12345")
        .await
        .unwrap();
    assert_eq!(
        app,
        Some(PairApp {
            name: "Claude".into(),
            host: "claude.ai".into()
        })
    );
    let browser = core
        .pair_app_with(&server, "f-pair", "ABCDE12345")
        .await
        .unwrap();
    assert_eq!(browser, None);
    let edge = edge.lock().unwrap();
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
        "/v1/keys",
        409,
        json!({ "code": "key_limit", "error": "x" }),
    );
    can(
        &edge,
        "/v1/keys",
        400,
        json!({ "code": "bad_request", "error": "x" }),
    );
    let reason = |result: Result<NewKey, CoreError>| match result {
        Err(CoreError::Other { reason }) => reason,
        other => panic!("{other:?}"),
    };
    let full = core.create_key_with(&server, "f", "Key", false, None).await;
    assert!(reason(full).starts_with("Revoke a key first"));
    let bad = core
        .create_key_with(&server, "f", "Key", false, Some("ABCDE12345".into()))
        .await;
    assert!(reason(bad).starts_with("Check the code"));
    assert_eq!(edge.lock().unwrap().requests[1].body["label"], "MCP");
}

#[tokio::test]
async fn account_calls_never_ask_or_send_the_session() {
    // Without a token they prove; with no proof network (this test's) that fails first.
    let core = core("consent");
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    let core = signed_in(&core.dir, server.clone());
    fs::write(core.at(crate::remote::edge::SERVER_FILE), &server).unwrap();
    let gate = consent::tests::no(&core);
    assert!(core.connected_apps("f".into()).await.is_err());
    assert!(core.create_key("f2".into(), false, None).await.is_err());
    assert_eq!(gate.asks(), 0);
    assert!(edge.lock().unwrap().requests.is_empty());
}

#[tokio::test]
async fn handover_for_another_server_never_asks() {
    let core = core("handover-elsewhere");
    let edge = Arc::new(Mutex::new(Edge::default()));
    fs::write(
        core.at(crate::remote::edge::SERVER_FILE),
        serve(edge.clone()).await,
    )
    .unwrap();
    consent::tests::signed_in(&core);
    let gate = consent::tests::yes(&core);
    let callback = "https://evil.example/v1/session/answer?account=x";
    let answered = core
        .answer_session_request("f".into(), "r1".into(), callback.into(), None)
        .await;
    assert!(matches!(answered, Err(CoreError::Other { .. })));
    assert_eq!(gate.asks(), 0);
    assert!(edge.lock().unwrap().requests.is_empty());
}

#[tokio::test]
async fn handover_unattended_answers_with_error_not_cookies() {
    let core = core("handover");
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    fs::write(core.at(crate::remote::edge::SERVER_FILE), &server).unwrap();
    let path = "/v1/session/answer?account=22bce0001";
    can(&edge, path, 200, json!({ "ok": true }));
    consent::tests::signed_in(&core);
    // In the pocket, even with "don't ask again" ticked.
    let gate = consent::tests::install(&core, false, false);
    consent::tests::remember(&core, CookieKind::Handover);
    let answered = core
        .answer_session_request("f".into(), "r1".into(), format!("{server}{path}"), None)
        .await;
    assert_eq!(answered, Err(CoreError::ConsentDenied));
    assert_eq!(gate.asks(), 0);
    let edge = edge.lock().unwrap();
    let answer = &edge.requests.iter().find(|r| r.path == path).unwrap().body;
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
    let path = "/v1/session/answer?account=22bce0001";
    let callback = format!("{server}{path}");
    can(&edge, path, 200, json!({ "ok": true }));
    core.answer_with(&server, "f-a", "r1", &callback, Ok("JSESSIONID=c".into()))
        .await
        .unwrap();
    // Signed out (no session in this core): the edge hears why, the caller gets the error.
    can(&edge, path, 200, json!({ "ok": true }));
    fs::write(core.at(crate::remote::edge::SERVER_FILE), &server).unwrap();
    let signed_out = core
        .answer_session_request("f-b".into(), "r2".into(), callback.clone(), None)
        .await;
    assert_eq!(signed_out, Err(CoreError::NotSignedIn));
    // Another server's callback is not sent anything.
    let elsewhere = core
        .answer_with(
            &server,
            "f-c",
            "r3",
            "https://evil.example/v1/session/answer?account=x",
            Ok("c=1".into()),
        )
        .await;
    assert!(matches!(elsewhere, Err(CoreError::Other { .. })));

    let edge = edge.lock().unwrap();
    let answers: Vec<&Value> = edge
        .requests
        .iter()
        .filter(|r| r.path == path)
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

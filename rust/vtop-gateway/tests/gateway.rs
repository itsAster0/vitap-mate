use std::time::Duration;

use tokio::net::UdpSocket;
use vtop_gateway::{run, Config};
use vtop_shared::engine::{UDP_KEYS, UDP_KEYS_SEALED_FOR};
use vtop_shared::mail_seal;
use vtop_shared::page_dictionary;
use vtop_shared::wire_page::{
    session_hash, WirePageRequest, WirePageResponse, NEED_SESSION, SIGN_IN_BODY, UNSUPPORTED,
    VERSION,
};
use vtop_wire::client::{fetch, ClientConfig};
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SESSION: &str = r#"{"cookies":"JSESSIONID=abc","csrf":"t"}"#;

async fn gateway(engine: &MockServer) -> (u16, [u8; 32]) {
    let (private, public) = vtop_wire::generate_keypair();
    // The engine passes on the key, sealed by the edge to the gateway.
    let (identity, identity_public) = mail_seal::generate_keypair();
    let keys = serde_json::json!({ "keys": [vtop_wire::hex(&private)] }).to_string();
    let sealed = mail_seal::seal(&identity_public, UDP_KEYS_SEALED_FOR, keys.as_bytes()).unwrap();
    Mock::given(method("GET"))
        .and(path(UDP_KEYS))
        .and(header("authorization", "Bearer relay-secret"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "sealed": sealed })),
        )
        .mount(engine)
        .await;
    let sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let port = sock.local_addr().unwrap().port();
    let config = Config {
        engine_url: engine.uri(),
        edge_url: engine.uri(),
        engine_secret: "relay-secret".into(),
        identity,
    };
    tokio::spawn(run(config, vec![sock]));
    (port, public)
}

fn request(session: Option<&str>, hash_of: &str) -> Vec<u8> {
    WirePageRequest {
        request_id: Some("0f10d00e".into()),
        name: "attendance".into(),
        semester: Some("AP2026275".into()),
        args: Default::default(),
        dictionary: Some("dict".into()),
        session_hash: session_hash(hash_of),
        session: session.map(str::to_string),
        account: None,
    }
    .encode()
    .unwrap()
}

#[tokio::test]
async fn fetches_through_the_engine_and_keeps_the_session() {
    let engine = MockServer::start().await;
    let packed = vec![7u8; 1500];
    Mock::given(method("POST"))
        .and(path("/engine/fetch"))
        .and(header("authorization", "Bearer relay-secret"))
        .and(header("x-request-id", "0f10d00e"))
        .and(body_partial_json(serde_json::json!({
            "name": "attendance",
            "semester": "AP2026275",
            "dictionary": "dict",
            "session": { "cookies": "JSESSIONID=abc" }
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", page_dictionary::CONTENT_TYPE)
                .set_body_bytes(packed.clone()),
        )
        .expect(2)
        .mount(&engine)
        .await;
    let (port, key) = gateway(&engine).await;
    let dir = std::env::temp_dir().join(format!("vtop-gateway-test-{}", std::process::id()));
    let cfg = ClientConfig {
        host: "127.0.0.1".into(),
        ports: vec![port],
        server_key: key,
        state_dir: dir,
        request_in_handshake: true,
    };

    // First fetch carries the session.
    let got = fetch(
        &cfg,
        &request(Some(SESSION), SESSION),
        Duration::from_secs(20),
    )
    .await
    .unwrap();
    let answer = WirePageResponse::decode(&got.body).unwrap();
    assert_eq!(
        (answer.status, answer.packed, answer.body),
        (200, true, packed.clone())
    );

    // Later fetches send only its hash.
    let got = fetch(&cfg, &request(None, SESSION), Duration::from_secs(20))
        .await
        .unwrap();
    assert_eq!(WirePageResponse::decode(&got.body).unwrap().status, 200);

    // A session the gateway does not hold is asked for, without calling the engine.
    let got = fetch(
        &cfg,
        &request(None, "another session"),
        Duration::from_secs(20),
    )
    .await
    .unwrap();
    assert_eq!(
        WirePageResponse::decode(&got.body).unwrap().status,
        NEED_SESSION
    );
}

#[tokio::test]
async fn a_request_newer_than_the_gateway_is_told_to_use_https() {
    let engine = MockServer::start().await;
    let (port, key) = gateway(&engine).await;
    let cfg = ClientConfig {
        host: "127.0.0.1".into(),
        ports: vec![port],
        server_key: key,
        state_dir: std::env::temp_dir().join(format!("vtop-gateway-newer-{}", std::process::id())),
        request_in_handshake: true,
    };
    let got = fetch(&cfg, &[0, VERSION + 1, 0, 0, 0, 0], Duration::from_secs(20))
        .await
        .unwrap();
    assert_eq!(
        WirePageResponse::decode(&got.body).unwrap().status,
        UNSUPPORTED
    );
    let asked = engine.received_requests().await.unwrap();
    assert!(asked.iter().all(|r| r.url.path() == UDP_KEYS));
}

/// A sign-in request named by its edge path, after the handshake.
async fn sign_in(port: u16, key: [u8; 32], path: &str, body: &str) -> WirePageResponse {
    let dir = std::env::temp_dir().join(format!(
        "vtop-gateway-sign-in-{}-{}",
        std::process::id(),
        path.replace('/', "-")
    ));
    let cfg = ClientConfig {
        host: "127.0.0.1".into(),
        ports: vec![port],
        server_key: key,
        state_dir: dir,
        request_in_handshake: false,
    };
    let request = WirePageRequest {
        request_id: None,
        name: path.into(),
        semester: None,
        args: [(SIGN_IN_BODY.to_string(), body.to_string())]
            .into_iter()
            .collect(),
        dictionary: None,
        session_hash: 0,
        session: None,
        account: None,
    }
    .encode()
    .unwrap();
    let got = fetch(&cfg, &request, Duration::from_secs(20))
        .await
        .unwrap();
    assert!(got.handshake);
    WirePageResponse::decode(&got.body).unwrap()
}

#[tokio::test]
async fn sign_ins_go_where_the_edge_would_send_them() {
    // One mock is both the engine and the edge.
    let server = MockServer::start().await;
    let ok = r#"{"status":"ok","session":{"cookies":"JSESSIONID=s"}}"#;
    Mock::given(method("POST"))
        .and(path("/engine/vtop-login"))
        .and(header("authorization", "Bearer relay-secret"))
        .and(body_partial_json(
            serde_json::json!({ "username": "U", "password": "P" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(ok))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/app/login"))
        .and(body_partial_json(
            serde_json::json!({ "appSecret": "secret" }),
        ))
        .respond_with(
            ResponseTemplate::new(401).set_body_string(r#"{"code":"invalid_credentials"}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    let (port, key) = gateway(&server).await;

    let phone = sign_in(
        port,
        key,
        "/v1/vtop/login",
        r#"{"username":"U","password":"P"}"#,
    )
    .await;
    assert_eq!((phone.status, phone.body), (200, ok.as_bytes().to_vec()));

    let by_server = sign_in(
        port,
        key,
        "/v1/app/login",
        r#"{"registrationNumber":"24MIC7076","appSecret":"secret"}"#,
    )
    .await;
    assert_eq!(by_server.status, 401);
}

#[tokio::test]
async fn an_expired_session_is_renewed_in_the_fetch() {
    // One mock is the engine and the edge.
    let server = MockServer::start().await;
    let page = r#"{"data":{"ok":true}}"#;
    Mock::given(method("POST"))
        .and(path("/engine/fetch"))
        .and(body_partial_json(
            serde_json::json!({ "session": { "cookies": "JSESSIONID=old" } }),
        ))
        .respond_with(ResponseTemplate::new(401).set_body_string(r#"{"code":"session_expired"}"#))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/engine/fetch"))
        .and(body_partial_json(
            serde_json::json!({ "session": { "cookies": "JSESSIONID=new" } }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(page))
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/app/login"))
        .and(body_partial_json(serde_json::json!({ "appSecret": "s" })))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"status":"ok","session":{"cookies":"JSESSIONID=new","registration_number":"R"}}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;
    let (port, key) = gateway(&server).await;
    let cfg = ClientConfig {
        host: "127.0.0.1".into(),
        ports: vec![port],
        server_key: key,
        state_dir: std::env::temp_dir().join(format!("vtop-gateway-renew-{}", std::process::id())),
        request_in_handshake: true,
    };
    let old = r#"{"cookies":"JSESSIONID=old"}"#;
    let mut request = WirePageRequest {
        request_id: None,
        name: "attendance".into(),
        semester: None,
        args: Default::default(),
        dictionary: None,
        session_hash: session_hash(old),
        session: Some(old.into()),
        account: Some(r#"{"registrationNumber":"R","appSecret":"s"}"#.into()),
    };

    let got = fetch(&cfg, &request.encode().unwrap(), Duration::from_secs(20))
        .await
        .unwrap();
    let answer = WirePageResponse::decode(&got.body).unwrap();
    assert_eq!(
        (answer.status, answer.body),
        (200, page.as_bytes().to_vec())
    );
    let renewed = answer.session.expect("the new session comes back");
    assert!(renewed.contains("JSESSIONID=new"));

    // The phone's next fetch names the new session by its hash only.
    request.session_hash = session_hash(r#"{"cookies":"JSESSIONID=new","csrf_token":null}"#);
    request.session = None;
    request.account = None;
    let got = fetch(&cfg, &request.encode().unwrap(), Duration::from_secs(20))
        .await
        .unwrap();
    assert_eq!(WirePageResponse::decode(&got.body).unwrap().status, 200);
}

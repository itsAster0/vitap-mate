use std::time::Duration;

use tokio::net::UdpSocket;
use vtop_gateway::{public_key, run, Config};
use vtop_shared::page_dictionary;
use vtop_shared::wire_page::{session_hash, WirePageRequest, WirePageResponse, NEED_SESSION};
use vtop_wire::client::{fetch, ClientConfig};
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SESSION: &str = r#"{"cookies":"JSESSIONID=abc","csrf":"t"}"#;

async fn gateway(engine: &MockServer) -> (u16, [u8; 32]) {
    let (private, _) = vtop_wire::generate_keypair();
    let sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let port = sock.local_addr().unwrap().port();
    let config = Config {
        engine_url: engine.uri(),
        engine_secret: "relay-secret".into(),
        private_key: private,
    };
    tokio::spawn(run(config, vec![sock]));
    (port, public_key(&private))
}

fn request(session: Option<&str>, hash_of: &str) -> Vec<u8> {
    WirePageRequest {
        name: "attendance".into(),
        semester: Some("AP2026275".into()),
        args: Default::default(),
        dictionary: Some("dict".into()),
        session_hash: session_hash(hash_of),
        session: session.map(str::to_string),
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

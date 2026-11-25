use std::net::UdpSocket as StdSocket;
use std::sync::Mutex;

use serde_json::json;
use vtop_shared::engine::BatchPage;
use vtop_shared::wire_page::{BATCH, batch_args, batch_pages};
use vtop_wire::gateway::Gateway as WireGateway;

use super::*;
use crate::test_edge::{Edge, serve};

/// Kotlin's part, on blocking std sockets.
struct StdNetwork;

/// A core on `dir` whose UDP goes through [`StdNetwork`].
fn with_network(dir: &std::path::Path) -> Core {
    let udp = Some(Arc::new(StdNetwork) as Arc<dyn UdpNetwork>);
    Core::for_test_on(
        dir,
        crate::Shared {
            udp,
            ..Default::default()
        },
    )
}

#[async_trait::async_trait]
impl UdpNetwork for StdNetwork {
    fn open(&self) -> Result<Arc<dyn UdpSocket>, TransportError> {
        Ok(Arc::new(Socket(StdSocket::bind("0.0.0.0:0").unwrap())))
    }

    async fn resolve(&self, host: String, _: u32) -> Result<Vec<String>, TransportError> {
        use std::net::ToSocketAddrs;
        let found = (host.as_str(), 0).to_socket_addrs().into_iter().flatten();
        Ok(found.map(|a| a.ip().to_string()).collect())
    }
}

struct Socket(StdSocket);

#[async_trait::async_trait]
impl UdpSocket for Socket {
    async fn send(&self, ip: String, port: u16, data: Vec<u8>) -> Result<(), TransportError> {
        self.0.send_to(&data, (ip.as_str(), port)).unwrap();
        Ok(())
    }

    async fn receive(&self, timeout_ms: u32) -> Result<Option<Datagram>, TransportError> {
        let timeout = Duration::from_millis(timeout_ms.into());
        self.0.set_read_timeout(Some(timeout)).unwrap();
        let mut buf = [0u8; 2048];
        Ok(self.0.recv_from(&mut buf).ok().map(|(n, from)| Datagram {
            ip: from.ip().to_string(),
            port: from.port(),
            data: buf[..n].to_vec(),
        }))
    }

    fn shutdown(&self) {}
}

/// A gateway on its own thread that asks for the session once, then answers each page
/// with its name; its port and public key (base64url).
fn spawn_gateway() -> (u16, String) {
    // Noise's keys are plain X25519 keys.
    let (private, public) = mail_seal::generate_keypair();
    let private = mail_seal::key32(&private).unwrap();
    let socket = StdSocket::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let mut gateway = WireGateway::new(vec![private]);
        let mut session = None;
        let mut buf = [0u8; 1500];
        loop {
            let due = gateway.next_due();
            let wait = due.map_or(Duration::from_millis(50), |t| {
                t.saturating_duration_since(Instant::now())
            });
            let wait = wait.max(Duration::from_millis(1));
            socket.set_read_timeout(Some(wait)).unwrap();
            let received = socket.recv_from(&mut buf).ok();
            let new =
                received.and_then(|(n, from)| gateway.receive(&buf[..n], 0, from, Instant::now()));
            if let Some(new) = new {
                let page = WirePageRequest::decode(&new.body).unwrap();
                assert_eq!(page.dictionary.as_deref(), Some("d1c7"));
                session = page.session.clone().or(session);
                let (status, body) = match &session {
                    None => (NEED_SESSION, json!({ "code": "need_session" })),
                    Some(_) if page.name == BATCH => {
                        let pages = batch_pages(&page.args).unwrap();
                        let answers: Vec<_> = pages
                            .iter()
                            .map(|p| json!({ "data": { "page": p.name } }))
                            .collect();
                        (200, json!({ "data": answers }))
                    }
                    Some(_) => (200, json!({ "data": { "page": page.name } })),
                };
                let answer = WirePageResponse {
                    status,
                    packed: false,
                    body: body.to_string().into_bytes(),
                    session: None,
                };
                gateway.respond(new.token, answer.encode(), Instant::now());
            }
            for out in gateway.poll(Instant::now()) {
                let _ = socket.send_to(&out.packet, out.to);
            }
        }
    });
    (port, public)
}

#[tokio::test]
async fn fetches_a_page_over_udp() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-udp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let core = with_network(&dir);
    let (port, public) = spawn_gateway();
    let now = unix_now();
    core.write_plain(
        GATEWAY,
        &json!({
            "host": "127.0.0.1",
            "ports": [port],
            "keys": [{ "id": "k", "publicKey": public, "from": now - 60, "until": now + 60 }],
            "server": core.forwarding_server(),
            "fetched": now,
        }),
    );
    let request = PageFetch {
        session: json!({ "cookies": "JSESSIONID=x" }),
        name: "attendance".into(),
        semester: Some("AP1".into()),
        args: Default::default(),
        dictionary: Some("d1c7".into()),
    };
    let page = core.udp_page("0f10d00e", &request).await;
    assert_eq!(page.unwrap(), json!({ "page": "attendance" }));
    let page = core.udp_page("0f10d00e", &request).await;
    assert_eq!(page.unwrap(), json!({ "page": "attendance" }));
    // A batch, as `route::batch` sends one.
    let pages = ["attendance", "marks"].map(|name| BatchPage {
        name: name.into(),
        args: Default::default(),
    });
    let batch = WirePageRequest {
        request_id: Some("0f10d00e".into()),
        name: BATCH.into(),
        semester: Some("AP1".into()),
        args: batch_args(&pages),
        dictionary: Some("d1c7".into()),
        session_hash: 0,
        session: None,
        account: None,
    };
    let answers = core
        .udp_fetch("0f10d00e", batch, request.session.to_string())
        .await;
    assert_eq!(
        answers.unwrap(),
        json!([{ "data": { "page": "attendance" } }, { "data": { "page": "marks" } }])
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn gateway_key_uses_server_adjusted_time_when_clock_is_skewed() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-udp-skew-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let core = with_network(&dir);
    // Unreachable edge: proves udp_page stays on the cache using local `fetched` age.
    core.dev_set_forwarding_server("f".into(), Some("http://127.0.0.1:1".into()))
        .unwrap();
    let (port, active_pub) = spawn_gateway();
    let (_, local_pub) = mail_seal::generate_keypair();
    let active_key = mail_seal::key32(&active_pub).unwrap();
    let local_key = mail_seal::key32(&local_pub).unwrap();

    let local_now = unix_now();
    let skew = 7 * 86_400;

    // Phone clock 7 days slow (clock_offset = +7 days).
    core.set_clock_offset(skew);
    let server_now = core.server_now();
    let gateway = Gateway {
        host: "127.0.0.1".into(),
        ports: vec![port],
        keys: vec![
            GatewayKey {
                public_key: local_pub.clone(),
                from: local_now - 60,
                until: local_now + 60,
            },
            GatewayKey {
                public_key: active_pub.clone(),
                from: server_now - 60,
                until: server_now + 60,
            },
        ],
        server: core.forwarding_server(),
        fetched: local_now,
    };
    assert_eq!(gateway.key(local_now), Some(local_key));
    assert_eq!(gateway.key(core.server_now()), Some(active_key));

    core.write_plain(GATEWAY, &gateway);
    let request = PageFetch {
        session: json!({ "cookies": "JSESSIONID=x" }),
        name: "attendance".into(),
        semester: Some("AP1".into()),
        args: Default::default(),
        dictionary: Some("d1c7".into()),
    };
    let page = core.udp_page("skew-slow", &request).await;
    assert_eq!(page.unwrap(), json!({ "page": "attendance" }));

    // Phone clock 7 days fast (clock_offset = -7 days).
    core.set_clock_offset(-skew);
    let behind_now = core.server_now();
    let fast_gateway = Gateway {
        host: "127.0.0.1".into(),
        ports: vec![port],
        keys: vec![GatewayKey {
            public_key: active_pub,
            from: behind_now - 60,
            until: behind_now + 60,
        }],
        server: core.forwarding_server(),
        fetched: local_now,
    };
    assert_eq!(fast_gateway.key(local_now), None);
    assert_eq!(fast_gateway.key(core.server_now()), Some(active_key));

    let _ = std::fs::remove_dir_all(&dir);
}

/// The fake server's `udp:keys`: the gateway at `udp_port` with `public`, current at
/// `server_time`.
fn can_keys(edge: &Arc<Mutex<Edge>>, server_time: i64, udp_port: u16, public: &str) {
    let keys = json!({
        "host": "127.0.0.1",
        "ports": [udp_port],
        "keys": [{
            "id": "k1",
            "publicKey": public,
            "from": server_time - 60,
            "until": server_time + 60,
        }],
    });
    let mut edge = edge.lock().unwrap();
    edge.canned
        .entry("udp:keys".into())
        .or_default()
        .push_back((200, keys));
}

#[tokio::test]
async fn udp_keys_come_from_the_server_and_go_by_its_clock() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-udp-keys-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let core = with_network(&dir);
    let (udp_port, public) = spawn_gateway();
    // The phone's clock an hour slow, as a signed call's `stale_time` taught it.
    let skew = 3600;
    core.set_clock_offset(skew);
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    core.dev_set_forwarding_server("f".into(), Some(server))
        .unwrap();
    can_keys(&edge, unix_now() + skew, udp_port, &public);

    let request = PageFetch {
        session: json!({ "cookies": "JSESSIONID=x" }),
        name: "attendance".into(),
        semester: Some("AP1".into()),
        args: Default::default(),
        dictionary: Some("d1c7".into()),
    };
    let page = core.udp_page("keys-flow", &request).await;
    assert_eq!(page.unwrap(), json!({ "page": "attendance" }));
    {
        let edge = edge.lock().unwrap();
        let asked: Vec<_> = edge
            .requests
            .iter()
            .filter(|r| r.path == "udp:keys")
            .collect();
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].body, json!({}), "udp:keys takes no arguments");
    }
    let saved = core.read::<Gateway>(GATEWAY).unwrap();
    assert!((saved.fetched - unix_now()).abs() <= 5);
    assert!(saved.key(unix_now()).is_none());
    assert!(saved.key(core.server_now()).is_some());

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn udp_keys_the_server_refuses_are_a_clear_error() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-udp-keys-no-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let core = Core::for_test(&dir);
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    core.dev_set_forwarding_server("f".into(), Some(server.clone()))
        .unwrap();
    edge.lock().unwrap().canned.insert(
        "udp:keys".into(),
        [(503, json!({ "code": "gateway_down" }))].into(),
    );

    let err = core.udp_gateway(&server, "keys-down").await.err().unwrap();
    assert_eq!(err, "udp:keys: gateway_down");

    let _ = std::fs::remove_dir_all(&dir);
}

/// Against the deployed edge and gateway: `cargo test -p vtop-ffi live_udp -- --ignored`.
/// Registers a throwaway install; the made-up session makes the engine refuse the page, which
/// still crosses the whole route (edge keys, handshake, gateway, engine).
#[tokio::test]
#[ignore = "needs the network, the live edge and the gateway"]
async fn live_udp() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-live-udp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let core = with_network(&dir);
    let gateway = core.udp_gateway(&core.forwarding_server(), "live").await;
    let gateway = gateway.unwrap();
    println!("gateway {} {:?}", gateway.host, gateway.ports);
    let request = PageFetch {
        session: json!({ "cookies": "JSESSIONID=made-up" }),
        name: "semesters".into(),
        semester: None,
        args: Default::default(),
        dictionary: None,
    };
    let started = Instant::now();
    let page = core.udp_page("live", &request).await;
    println!(
        "{:?} in {:?}",
        page.as_ref().map(|_| "page"),
        started.elapsed()
    );
    let error = page.unwrap_err();
    assert!(
        matches!(
            error,
            UdpFetchError::Gateway { .. } | UdpFetchError::SessionExpired
        ),
        "{error}"
    );
}

#[test]
fn gateway_answers_parse_once_into_errors() {
    let answer = |status: u16, body: &[u8]| WirePageResponse {
        status,
        packed: false,
        body: body.to_vec(),
        session: None,
    };
    assert_eq!(
        page_answer("marks", &answer(200, br#"{"data":{"a":1}}"#)),
        Ok(json!({ "a": 1 }))
    );
    assert_eq!(
        page_answer("marks", &answer(401, br#"{"code":"session_expired"}"#)),
        Err(UdpFetchError::SessionExpired)
    );
    let gone = page_answer("marks", &answer(404, b"<html>"));
    assert_eq!(
        gone,
        Err(UdpFetchError::Gateway {
            status: 404,
            code: ServerCode::ServerError
        })
    );
    assert_eq!(
        gone.unwrap_err().to_string(),
        "the UDP gateway answered 404 server_error"
    );
    assert_eq!(
        page_answer("marks", &answer(502, b"{}")),
        Err(UdpFetchError::Gateway {
            status: 502,
            code: ServerCode::ServerError
        })
    );
    assert_eq!(
        page_answer("marks", &answer(503, br#"{"code":"vtop_unavailable"}"#)),
        Err(UdpFetchError::Gateway {
            status: 503,
            code: ServerCode::VtopUnavailable
        })
    );
    assert!(matches!(
        page_answer("marks", &answer(200, b"not json")),
        Err(UdpFetchError::Transport(_))
    ));
}

#[test]
fn udp_gateway_json_keeps_its_format() {
    let file = r#"{"host":"gw.example","ports":[3478,3479],"keys":[{"publicKey":"a2V5","from":1,"until":2}],"server":"https://vmate.aster0.dev","fetched":9}"#;
    let gateway: Gateway = serde_json::from_str(file).unwrap();
    assert_eq!(serde_json::to_string(&gateway).unwrap(), file);
    // From before the server and the time were kept.
    let old: Gateway = serde_json::from_str(r#"{"host":"h","ports":[1],"keys":[]}"#).unwrap();
    assert!(old.server.is_empty() && old.fetched == 0);
}

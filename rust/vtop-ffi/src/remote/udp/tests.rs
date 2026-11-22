use std::net::UdpSocket as StdSocket;

use serde_json::json;
use vtop_shared::engine::BatchPage;
use vtop_shared::wire_page::{BATCH, batch_args, batch_pages};
use vtop_wire::gateway::Gateway as WireGateway;

use super::*;

/// Kotlin's part, on blocking std sockets.
struct StdNetwork;

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
    let core = Core::for_test(&dir);
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
    set_udp_network(Arc::new(StdNetwork));
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
    let core = Core::for_test(&dir);
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
    set_udp_network(Arc::new(StdNetwork));
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

async fn mock_udp_edge(
    seed: String,
    server_time: i64,
    udp_port: u16,
    public: String,
    always_stale: bool,
) -> (String, Arc<std::sync::atomic::AtomicUsize>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use vtop_shared::device_sig;

    let hits = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let count = hits.clone();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let (count, seed, public) = (count.clone(), seed.clone(), public.clone());
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let mut total = 0;
                loop {
                    let Ok(n) = socket.read(&mut buf[total..]).await else {
                        return;
                    };
                    if n == 0 {
                        break;
                    }
                    total += n;
                    if buf[..total].windows(4).any(|w| w == b"\r\n\r\n") || total == buf.len() {
                        break;
                    }
                }
                let req = String::from_utf8_lossy(&buf[..total]);
                let mut lines = req.lines();
                let first = lines.next().unwrap_or("");
                let mut parts = first.split_whitespace();
                let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
                let header = |name: &str| {
                    req.lines().skip(1).find_map(|line| {
                        let (k, v) = line.split_once(':')?;
                        k.trim().eq_ignore_ascii_case(name).then(|| v.trim())
                    })
                };
                let (status, body) = match (method, path) {
                    ("POST", "/v1/devices/register") => (
                        "201 Created",
                        json!({ "deviceId": "0123456789abcdef0123456789abcdef" }).to_string(),
                    ),
                    ("GET", "/v1/udp") => {
                        count.fetch_add(1, Ordering::SeqCst);
                        let dev = header(device_sig::DEVICE).unwrap_or("");
                        let time: i64 = header(device_sig::TIME)
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0);
                        let sig = header(device_sig::SIG).unwrap_or("");
                        let expected_sig =
                            device_sig::sign(&seed, "GET", "/v1/udp", time, b"").unwrap();
                        if dev != "0123456789abcdef0123456789abcdef" || sig != expected_sig {
                            (
                                "401 Unauthorized",
                                json!({ "code": "bad_signature", "error": "bad sig" }).to_string(),
                            )
                        } else if always_stale || (time - server_time).abs() > 120 {
                            (
                                "401 Unauthorized",
                                json!({
                                    "code": "stale_time",
                                    "error": "clock skewed",
                                    "serverTime": server_time,
                                })
                                .to_string(),
                            )
                        } else {
                            (
                                "200 OK",
                                json!({
                                    "host": "127.0.0.1",
                                    "ports": [udp_port],
                                    "keys": [{
                                        "id": "k1",
                                        "publicKey": public,
                                        "from": server_time - 60,
                                        "until": server_time + 60,
                                    }],
                                })
                                .to_string(),
                            )
                        }
                    }
                    _ => ("404 Not Found", "{}".to_string()),
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });
    (base, hits)
}

#[tokio::test]
async fn udp_gateway_handles_stale_time_and_fetches_page() {
    use std::sync::atomic::Ordering;

    let dir = std::env::temp_dir().join(format!("vtop-ffi-udp-stale-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let core = Core::for_test(&dir);
    let seed = core.device().seed;
    let (udp_port, public) = spawn_gateway();
    let skew = 3600;
    let server_time = unix_now() + skew;
    let (edge, hits) = mock_udp_edge(seed, server_time, udp_port, public, false).await;
    core.dev_set_forwarding_server("f".into(), Some(edge))
        .unwrap();

    let request = PageFetch {
        session: json!({ "cookies": "JSESSIONID=x" }),
        name: "attendance".into(),
        semester: Some("AP1".into()),
        args: Default::default(),
        dictionary: Some("d1c7".into()),
    };
    set_udp_network(Arc::new(StdNetwork));
    let page = core.udp_page("stale-flow", &request).await;
    assert_eq!(page.unwrap(), json!({ "page": "attendance" }));
    assert_eq!(hits.load(Ordering::SeqCst), 2);
    assert!(
        (core.device().clock_offset - skew).abs() <= 5,
        "clock_offset was {}",
        core.device().clock_offset
    );
    let saved = core.read::<Gateway>(GATEWAY).unwrap();
    assert!((saved.fetched - unix_now()).abs() <= 5);
    assert!(saved.key(unix_now()).is_none());
    assert!(saved.key(core.server_now()).is_some());

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn udp_gateway_stops_after_one_stale_time_retry() {
    use std::sync::atomic::Ordering;

    let dir = std::env::temp_dir().join(format!("vtop-ffi-udp-stale-stop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let core = Core::for_test(&dir);
    let seed = core.device().seed;
    let (_, public) = mail_seal::generate_keypair();
    let (edge, hits) = mock_udp_edge(seed, unix_now() + 3600, 3478, public, true).await;

    let err = core.udp_gateway(&edge, "stale-loop").await.err().unwrap();
    assert_eq!(err, "/v1/udp: stale_time");
    assert_eq!(hits.load(Ordering::SeqCst), 2);

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
    let core = Core::for_test(&dir);
    set_udp_network(Arc::new(StdNetwork));
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
    assert!(error.starts_with("the UDP gateway answered"), "{error}");
}

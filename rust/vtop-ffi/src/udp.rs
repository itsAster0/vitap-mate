//! The server route over UDP (vm1): a page request goes to the UDP gateway, which fetches it
//! through the engine's `/engine/fetch` and answers with the page's JSON. The client is
//! vtop-wire's, run on the app's sockets: Kotlin implements [`UdpNetwork`] (`VtopUdp.kt`), so
//! the library needs no tokio. Where the gateway listens and its keys come from the edge's
//! signed `GET /v1/udp`, kept in [`GATEWAY`]; vtop-wire keeps its sessions in [`WIRE_DIR`].

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;
use vtop_shared::engine::PageFetch;
use vtop_shared::mail_seal;
use vtop_shared::wire_page::{NEED_SESSION, WirePageRequest, WirePageResponse, session_hash};
use vtop_wire::client::{self as wire, ClientConfig, WireError};

use crate::route::unpacked;
use crate::transport::TransportError;
use crate::{Core, unix_now};

const GATEWAY: &str = "udp-gateway.json";
const WIRE_DIR: &str = "udp";
/// Asked again after this, or at once when no key is current or the gateway has another.
const GATEWAY_KEEP_SECS: i64 = 6 * 3600;
/// The whole UDP attempt, the engine's VTOP time in it; the route falls back to HTTPS after it.
/// A gateway that never answers is given up on sooner (vtop-wire's `SILENT`, 5 s).
const TIMEOUT: Duration = Duration::from_secs(30);
const EDGE_TIMEOUT: Duration = Duration::from_secs(5);

/// One datagram received; `ip` without a scope.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct Datagram {
    pub ip: String,
    pub port: u16,
    pub data: Vec<u8>,
}

/// The app's UDP: sockets on the network it uses, and DNS on that network.
#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait UdpNetwork: Send + Sync {
    /// A new socket on any local port (quick: binding sends nothing).
    fn open(&self) -> Result<Arc<dyn UdpSocket>, TransportError>;
    /// `host`'s addresses (IPv4 first); empty when the lookup fails or takes over `timeout_ms`.
    async fn resolve(&self, host: String, timeout_ms: u32) -> Result<Vec<String>, TransportError>;
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait UdpSocket: Send + Sync {
    async fn send(&self, ip: String, port: u16, data: Vec<u8>) -> Result<(), TransportError>;
    /// The next datagram, or None after `timeout_ms` (at least 1).
    async fn receive(&self, timeout_ms: u32) -> Result<Option<Datagram>, TransportError>;
    /// Closes the socket (not `close`: UniFFI objects already have one).
    fn shutdown(&self);
}

static NETWORK: Mutex<Option<Arc<dyn UdpNetwork>>> = Mutex::new(None);

/// Sends the UDP route through `network`; without one the route fails and falls back to HTTPS.
#[uniffi::export]
pub fn set_udp_network(network: Arc<dyn UdpNetwork>) {
    info!("udp network: foreign");
    *NETWORK.lock().unwrap_or_else(PoisonError::into_inner) = Some(network);
}

struct Foreign(Arc<dyn UdpNetwork>);

/// Closed when the fetch drops it.
struct Opened(Arc<dyn UdpSocket>);

impl Drop for Opened {
    fn drop(&mut self) {
        self.0.shutdown();
    }
}

fn millis_until(until: Instant) -> u32 {
    let left = until.saturating_duration_since(Instant::now()).as_millis();
    u32::try_from(left).unwrap_or(u32::MAX).max(1)
}

impl wire::Net for Foreign {
    type Socket = Opened;

    async fn bind(&self, _ipv6: bool) -> std::io::Result<Opened> {
        self.0.open().map(Opened).map_err(std::io::Error::other)
    }

    async fn sleep(&self, duration: Duration) {
        vtop_core::sync::sleep(duration).await;
    }

    async fn resolve(&self, host: &str, _port: u16, until: Instant) -> Vec<IpAddr> {
        let found = self.0.resolve(host.into(), millis_until(until)).await;
        let found = found.unwrap_or_default();
        found.iter().filter_map(|ip| ip.parse().ok()).collect()
    }
}

impl wire::Socket for Opened {
    async fn send_to(&self, packet: &[u8], to: SocketAddr) -> std::io::Result<()> {
        let sent = self.0.send(to.ip().to_string(), to.port(), packet.to_vec());
        sent.await.map_err(std::io::Error::other)
    }

    async fn recv_from(&self, buf: &mut [u8], until: Instant) -> Option<(usize, SocketAddr)> {
        let datagram = self.0.receive(millis_until(until)).await.ok()??;
        let n = datagram.data.len().min(buf.len());
        if let (Some(to), Some(from)) = (buf.get_mut(..n), datagram.data.get(..n)) {
            to.copy_from_slice(from);
        }
        Some((n, SocketAddr::new(datagram.ip.parse().ok()?, datagram.port)))
    }
}

/// `GET /v1/udp` as kept, with the server it came from and when.
#[derive(Serialize, Deserialize)]
struct Gateway {
    host: String,
    ports: Vec<u16>,
    keys: Vec<GatewayKey>,
    #[serde(default)]
    server: String,
    #[serde(default)]
    fetched: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GatewayKey {
    public_key: String,
    from: i64,
    until: i64,
}

impl Gateway {
    /// The newest key in use now.
    fn key(&self, now: i64) -> Option<[u8; 32]> {
        let current = self.keys.iter().filter(|k| k.from <= now && now < k.until);
        let newest = current.max_by_key(|k| k.from)?;
        mail_seal::key32(&newest.public_key).ok()
    }
}

impl Core {
    /// Page `request`'s parsed `data`, fetched through the UDP gateway; `Err(reason)` on any
    /// failure, for the route to fall back.
    pub(crate) async fn udp_page(&self, flow: &str, request: &PageFetch) -> Result<Value, String> {
        let page = WirePageRequest {
            request_id: Some(flow.to_string()),
            name: request.name.clone(),
            semester: request.semester.clone(),
            args: request.args.clone(),
            dictionary: request.dictionary.clone(),
            session_hash: 0,
            session: None,
            account: None,
        };
        self.udp_fetch(flow, page, request.session.to_string())
            .await
    }

    /// `page` (one page, or a batch) on `session` through the UDP gateway: the answer's `data`.
    pub(crate) async fn udp_fetch(
        &self,
        flow: &str,
        mut page: WirePageRequest,
        session: String,
    ) -> Result<Value, String> {
        let deadline = Instant::now() + TIMEOUT;
        let network = NETWORK
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let net = Foreign(network.ok_or("no UDP network set")?);
        let server = self.forwarding_server();
        let now = unix_now();
        let server_now = now.saturating_add(self.device().clock_offset);
        let cached = self.read::<Gateway>(GATEWAY).filter(|g| {
            g.server == server
                && (0..GATEWAY_KEEP_SECS).contains(&now.saturating_sub(g.fetched))
                && g.key(server_now).is_some()
        });
        let mut asked = cached.is_none();
        let mut gateway = match cached {
            Some(gateway) => gateway,
            None => self.udp_gateway(&server, flow).await?,
        };
        page.session_hash = session_hash(&session);
        loop {
            let key = gateway
                .key(self.server_now())
                .ok_or("the UDP gateway has no current key")?;
            let config = ClientConfig {
                host: gateway.host.clone(),
                ports: gateway.ports.clone(),
                server_key: key,
                state_dir: self.at(WIRE_DIR),
                // Cookies are secret: not in message 1, which is not forward-secret.
                request_in_handshake: page.session.is_none(),
            };
            let body = page.encode().ok_or("the request is too large for UDP")?;
            let left = deadline.saturating_duration_since(Instant::now());
            let fetched = match wire::fetch_over(&net, &config, &body, left).await {
                Ok(fetched) => fetched,
                Err(WireError::WrongKey) if !asked => {
                    asked = true;
                    gateway = self.udp_gateway(&server, flow).await?;
                    continue;
                }
                Err(error) => return Err(error.to_string()),
            };
            info!(
                page = page.name.as_str(),
                handshake = fetched.handshake,
                ms = fetched.elapsed.as_millis() as u64,
                up = fetched.bytes_up,
                down = fetched.bytes_down,
                retries = fetched.retries,
                "udp fetch"
            );
            let answer =
                WirePageResponse::decode(&fetched.body).ok_or("unreadable answer from UDP")?;
            if answer.status == NEED_SESSION && page.session.is_none() {
                page.session = Some(session.clone());
                continue;
            }
            let unreadable = || format!("unreadable page from UDP (HTTP {})", answer.status);
            if answer.packed && answer.status == 200 {
                return unpacked(&page.name, &answer.body).ok_or_else(unreadable);
            }
            let mut body: Value = serde_json::from_slice(&answer.body).map_err(|_| unreadable())?;
            if answer.status != 200 {
                let code = body
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                return Err(format!("the UDP gateway answered {} {code}", answer.status));
            }
            return Ok(body.get_mut("data").map(Value::take).unwrap_or_default());
        }
    }

    /// Asks the edge where the gateway listens and for its keys, and keeps the answer.
    async fn udp_gateway(&self, server: &str, flow: &str) -> Result<Gateway, String> {
        let path = "/v1/udp";
        let body = self
            .edge_get(server, path, flow, EDGE_TIMEOUT)
            .await
            .map_err(|error| format!("{path}: {}", error.code()))?;
        let mut gateway: Gateway =
            serde_json::from_value(body).map_err(|_| format!("{path}: unexpected answer"))?;
        gateway.server = server.to_string();
        gateway.fetched = unix_now();
        self.write_plain(GATEWAY, &gateway);
        info!(
            host = gateway.host.as_str(),
            keys = gateway.keys.len(),
            "udp gateway"
        );
        Ok(gateway)
    }
}

#[cfg(test)]
mod tests {
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
                let new = received
                    .and_then(|(n, from)| gateway.receive(&buf[..n], 0, from, Instant::now()));
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
        let core = Core {
            cache: dir.clone(),
            dir: dir.clone(),
            slot: Mutex::new(None),
        };
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
        let core = Core {
            cache: dir.clone(),
            dir: dir.clone(),
            slot: Mutex::new(None),
        };
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
                                    json!({ "code": "bad_signature", "error": "bad sig" })
                                        .to_string(),
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
        let core = Core {
            cache: dir.clone(),
            dir: dir.clone(),
            slot: Mutex::new(None),
        };
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

        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-udp-stale-stop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let core = Core {
            cache: dir.clone(),
            dir: dir.clone(),
            slot: Mutex::new(None),
        };
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
        let core = Core {
            cache: dir.clone(),
            dir: dir.clone(),
            slot: Mutex::new(None),
        };
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
}

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
use vtop_core::client::Hop;
use vtop_shared::engine::PageFetch;
use vtop_shared::wire_page::{NEED_SESSION, WirePageRequest, WirePageResponse, session_hash};
use vtop_shared::{device_sig, mail_seal};
use vtop_wire::client::{self as wire, ClientConfig, WireError};

use crate::transport::{self, TransportError};
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
        buf[..n].copy_from_slice(&datagram.data[..n]);
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
        let deadline = Instant::now() + TIMEOUT;
        let network = NETWORK
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let net = Foreign(network.ok_or("no UDP network set")?);
        let server = self.forwarding_server();
        let now = unix_now();
        let cached = self.read::<Gateway>(GATEWAY).filter(|g| {
            g.server == server && now - g.fetched < GATEWAY_KEEP_SECS && g.key(now).is_some()
        });
        let mut asked = cached.is_none();
        let mut gateway = match cached {
            Some(gateway) => gateway,
            None => self.udp_gateway(&server, flow).await?,
        };
        let session = request.session.to_string();
        let mut page = WirePageRequest {
            request_id: Some(flow.to_string()),
            name: request.name.clone(),
            semester: request.semester.clone(),
            args: request.args.clone(),
            // Plain JSON back: the app holds no page dictionary.
            dictionary: None,
            session_hash: session_hash(&session),
            session: None,
            account: None,
        };
        loop {
            let key = gateway
                .key(unix_now())
                .ok_or("the UDP gateway has no current key")?;
            let config = ClientConfig {
                host: gateway.host.clone(),
                ports: gateway.ports.clone(),
                server_key: key,
                state_dir: self.dir.join(WIRE_DIR),
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
            let mut body: Value = serde_json::from_slice(&answer.body)
                .map_err(|_| format!("unreadable page from UDP (HTTP {})", answer.status))?;
            if answer.status != 200 {
                let code = body["code"].as_str().unwrap_or("unknown");
                return Err(format!("the UDP gateway answered {} {code}", answer.status));
            }
            return Ok(body["data"].take());
        }
    }

    /// Asks the edge where the gateway listens and for its keys, and keeps the answer.
    async fn udp_gateway(&self, server: &str, flow: &str) -> Result<Gateway, String> {
        let id = self.device_id(server, flow).await;
        let id = id.map_err(|error| format!("no device id: {}", error.code()))?;
        let device = self.device();
        let path = "/v1/udp";
        let time = unix_now() + device.clock_offset;
        let sig = device_sig::sign(&device.seed, "GET", path, time, b"").unwrap_or_default();
        let hop = Hop {
            method: "GET".into(),
            url: format!("{}{path}", server.trim_end_matches('/')),
            headers: vec![
                ("x-request-id".into(), flow.into()),
                (device_sig::DEVICE.into(), id),
                (device_sig::TIME.into(), time.to_string()),
                (device_sig::SIG.into(), sig),
            ],
            body: Vec::new(),
            timeout: EDGE_TIMEOUT,
            connect_timeout: EDGE_TIMEOUT,
            download_to: None,
        };
        let transport = transport::current().map_err(|error| error.to_string())?;
        let response = transport
            .send(hop)
            .await
            .map_err(|error| error.to_string())?;
        let status = response.status;
        let bytes = response.bytes().await.map_err(|error| error.to_string())?;
        if status != 200 {
            return Err(format!("{path} answered HTTP {status}"));
        }
        let mut gateway: Gateway =
            serde_json::from_slice(&bytes).map_err(|_| format!("{path}: unexpected answer"))?;
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
                    session = page.session.clone().or(session);
                    let (status, body) = match &session {
                        None => (NEED_SESSION, json!({ "code": "need_session" })),
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
            dir,
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
            dictionary: None,
        };
        set_udp_network(Arc::new(StdNetwork));
        let page = core.udp_page("0f10d00e", &request).await;
        assert_eq!(page.unwrap(), json!({ "page": "attendance" }));
        let page = core.udp_page("0f10d00e", &request).await;
        assert_eq!(page.unwrap(), json!({ "page": "attendance" }));
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
            dir,
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

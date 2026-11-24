//! The server route over UDP (vm1): a page request goes to the UDP gateway, which fetches it
//! through the engine's `/engine/fetch` and answers with the page's JSON. The client is
//! vtop-wire's, run on the app's sockets: Kotlin implements [`UdpNetwork`] (`VtopUdp.kt`), so
//! the library needs no tokio. Where the gateway listens and its keys come from the server's
//! `udp:keys`, kept in [`GATEWAY`]; vtop-wire keeps its sessions in [`WIRE_DIR`].

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{info, warn};
use vtop_shared::engine::PageFetch;
use vtop_shared::mail_seal;
use vtop_shared::wire_page::{NEED_SESSION, WirePageRequest, WirePageResponse, session_hash};
use vtop_wire::client::{self as wire, ClientConfig, WireError};

use crate::remote::backend::Kind;
use crate::remote::route::unpacked;
use crate::transport::TransportError;
use crate::{Core, unix_now};

const GATEWAY: &str = "udp-gateway.json";
const WIRE_DIR: &str = "udp";
/// Asked again after this, or at once when no key is current or the gateway has another.
const GATEWAY_KEEP_SECS: i64 = 6 * 3600;
/// The whole UDP attempt, the engine's VTOP time in it; the route falls back to HTTPS after it.
/// A gateway that never answers is given up on sooner (vtop-wire's `SILENT`, 5 s).
const TIMEOUT: Duration = Duration::from_secs(30);
/// How often the gateway gets a fresh JWT, which it signs in with when the session it holds
/// expires: well inside the JWT's 15 minutes.
const JWT_EVERY_MS: i64 = 10 * 60_000;
/// When a JWT last went to the gateway (unix ms).
static JWT_SENT_MS: AtomicI64 = AtomicI64::new(0);

/// The next fetch sends the JWT again: the identity changed (sign-out, another student).
pub(crate) fn forget_jwt_sent() {
    JWT_SENT_MS.store(0, Ordering::Relaxed);
}

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
        // A renewed session belongs to the student signed in now, not to whoever is by the time
        // the answer comes.
        let started = self.current().ok();
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
        let now_ms = unix_now().saturating_mul(1000);
        if now_ms - JWT_SENT_MS.load(Ordering::Relaxed) >= JWT_EVERY_MS {
            page.account = self.live_jwt();
        }
        loop {
            let key = gateway
                .key(self.server_now())
                .ok_or("the UDP gateway has no current key")?;
            let config = ClientConfig {
                host: gateway.host.clone(),
                ports: gateway.ports.clone(),
                server_key: key,
                state_dir: self.at(WIRE_DIR),
                // Cookies and the JWT are secret: not in message 1, which is not forward-secret.
                request_in_handshake: page.session.is_none() && page.account.is_none(),
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
            if page.account.is_some() {
                JWT_SENT_MS.store(now_ms, Ordering::Relaxed);
            }
            if answer.status == NEED_SESSION && page.session.is_none() {
                page.session = Some(session.clone());
                // With it the gateway renews an expired session itself. After a cold start the
                // phone may hold only the refresh token; this is rare, so the socket is asked.
                if self.live_jwt().is_none()
                    && let Err(error) = self.identity(flow, "UDP gateway sign-in", false).await
                {
                    info!(%error, "udp: no jwt for the gateway");
                }
                page.account = self.live_jwt();
                continue;
            }
            if let Some(renewed) = answer.session.as_deref().filter(|_| answer.status == 200) {
                self.adopt_renewed(renewed, started.as_ref()).await;
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

    /// The session the gateway signed in again with, from now on this phone's too: otherwise
    /// the next fetch sends the expired one back and the gateway signs in on every fetch.
    async fn adopt_renewed(&self, renewed: &str, started: Option<&Arc<crate::Slot>>) {
        let Ok(state) = serde_json::from_str::<vtop_core::SessionState>(renewed) else {
            warn!("udp: renewed session unreadable");
            return;
        };
        let Some(slot) = started.filter(|slot| self.is_current(slot)) else {
            info!("udp: renewed session dropped: signed out meanwhile");
            return;
        };
        let mut session = slot.write().await;
        match session.client.resume_session(&state) {
            Ok(()) => {
                session.logins += 1;
                self.save(slot, &session);
                info!("udp: took the session the gateway renewed");
            }
            Err(error) => warn!(%error, "udp: renewed session not taken"),
        }
    }

    /// Asks the server where the gateway listens and for its keys, and keeps the answer.
    async fn udp_gateway(&self, server: &str, flow: &str) -> Result<Gateway, String> {
        let path = "udp:keys";
        let body = self
            .public_call(flow, Kind::Query, path, json!({}))
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
mod tests;

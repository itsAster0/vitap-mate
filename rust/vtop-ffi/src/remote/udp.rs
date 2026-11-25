//! The server route over UDP (vm1): a page request goes to the UDP gateway, which fetches it
//! through the engine's `/engine/fetch` and answers with the page's JSON. The client is
//! vtop-wire's, run on the app's sockets: Kotlin implements [`UdpNetwork`] (`VtopUdp.kt`), so
//! the library needs no tokio. Where the gateway listens and its keys come from the server's
//! `udp:keys`, kept in [`GATEWAY`]; vtop-wire keeps its sessions in [`WIRE_DIR`].

use std::hash::{DefaultHasher, Hash, Hasher};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::Ordering;
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
pub(crate) const WIRE_DIR: &str = "udp";
/// Asked again after this, or at once when no key is current or the gateway has another.
const GATEWAY_KEEP_SECS: i64 = 6 * 3600;
/// The whole UDP attempt, the engine's VTOP time in it; the route falls back to HTTPS after it.
/// A gateway that never answers is given up on sooner (vtop-wire's `SILENT`, 5 s).
const TIMEOUT: Duration = Duration::from_secs(30);

/// Why a UDP fetch gave no page, read once from the gateway's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UdpFetchError {
    /// The link worked; VTOP's session ran out and the gateway couldn't renew it.
    SessionExpired,
    /// The gateway (or the engine behind it) answered `status` with `code` (`unreadable` when
    /// the body wasn't JSON).
    Gateway { status: u16, code: String },
    /// No answer, or none we could use: the route counts it as a failure.
    Transport(String),
}

impl std::fmt::Display for UdpFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionExpired => f.write_str("the UDP gateway answered session_expired"),
            Self::Gateway { status, code } => {
                write!(f, "the UDP gateway answered {status} {code}")
            }
            Self::Transport(reason) => f.write_str(reason),
        }
    }
}

impl From<String> for UdpFetchError {
    fn from(reason: String) -> Self {
        Self::Transport(reason)
    }
}

impl From<&str> for UdpFetchError {
    fn from(reason: &str) -> Self {
        Self::Transport(reason.to_string())
    }
}

/// The next fetch sends the JWT again: the identity changed (sign-out, another student).
pub(crate) fn forget_jwt_sent(shared: &crate::Shared) {
    shared.udp_jwt_sent.store(0, Ordering::Relaxed);
}

fn jwt_hash(jwt: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    jwt.hash(&mut hasher);
    hasher.finish().max(1)
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

/// The gateway's answer to page `name` as its `data`, or why there is none.
fn page_answer(name: &str, answer: &WirePageResponse) -> Result<Value, UdpFetchError> {
    let status = answer.status;
    if status != 200 {
        let body = serde_json::from_slice::<Value>(&answer.body).ok();
        let code = body.as_ref().map_or("unreadable", |body| {
            body.get("code")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        });
        return Err(match code {
            "session_expired" => UdpFetchError::SessionExpired,
            code => UdpFetchError::Gateway {
                status,
                code: code.to_string(),
            },
        });
    }
    let unreadable = || UdpFetchError::Transport("unreadable page from UDP".to_string());
    if answer.packed {
        return unpacked(name, &answer.body).ok_or_else(unreadable);
    }
    let mut body: Value = serde_json::from_slice(&answer.body).map_err(|_| unreadable())?;
    Ok(body.get_mut("data").map(Value::take).unwrap_or_default())
}

impl Core {
    /// Page `request`'s parsed `data`, fetched through the UDP gateway; an error on any failure,
    /// for the route to fall back.
    pub(crate) async fn udp_page(
        &self,
        flow: &str,
        request: &PageFetch,
    ) -> Result<Value, UdpFetchError> {
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
    ) -> Result<Value, UdpFetchError> {
        let deadline = Instant::now() + TIMEOUT;
        // A renewed session belongs to the student signed in now, not to whoever is by the time
        // the answer comes.
        let started = self.current().ok();
        let net = Foreign(self.shared.udp.clone().ok_or("no UDP network set")?);
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
        page.account = self
            .live_jwt()
            .filter(|jwt| jwt_hash(jwt) != self.shared.udp_jwt_sent.load(Ordering::Relaxed));
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
                Err(error) => return Err(UdpFetchError::Transport(error.to_string())),
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
            if let Some(jwt) = &page.account {
                self.shared
                    .udp_jwt_sent
                    .store(jwt_hash(jwt), Ordering::Relaxed);
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
            return page_answer(&page.name, &answer);
        }
    }

    /// The session the gateway signed in again with, from now on this phone's too: otherwise
    /// the next fetch sends the expired one back and the gateway signs in on every fetch.
    pub(crate) async fn adopt_renewed(&self, renewed: &str, started: Option<&Arc<crate::Slot>>) {
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
                session.mark_signed_in();
                crate::obs::set_regno(session.client.registration_number().ok());
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

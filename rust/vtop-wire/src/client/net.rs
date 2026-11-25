//! What the client needs from the platform ([`Net`], [`Socket`], tokio's by default), the
//! gateway's address (`gateway.addr`) and the name of the network the phone is on.

use std::fs;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use super::state::write_atomic;
use super::{ClientConfig, WireError};
use crate::msg::now_ms;

/// What the client needs from the platform.
pub trait Net: Send + Sync {
    type Socket: Socket;
    /// A socket on any local port, for IPv6 or IPv4.
    fn bind(&self, ipv6: bool) -> impl Future<Output = std::io::Result<Self::Socket>> + Send;
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send;
    /// The host's addresses; empty when the lookup fails or `until` passes.
    fn resolve(
        &self,
        host: &str,
        port: u16,
        until: Instant,
    ) -> impl Future<Output = Vec<IpAddr>> + Send;
}

pub trait Socket: Send + Sync {
    fn send_to(
        &self,
        packet: &[u8],
        to: SocketAddr,
    ) -> impl Future<Output = std::io::Result<()>> + Send;
    /// The next datagram, or None once `until` passes (or the socket fails).
    fn recv_from(
        &self,
        buf: &mut [u8],
        until: Instant,
    ) -> impl Future<Output = Option<(usize, SocketAddr)>> + Send;
}

/// tokio's sockets, timers and DNS.
#[cfg(feature = "tokio")]
pub struct Tokio;

#[cfg(feature = "tokio")]
impl Net for Tokio {
    type Socket = tokio::net::UdpSocket;

    async fn bind(&self, ipv6: bool) -> std::io::Result<Self::Socket> {
        tokio::net::UdpSocket::bind(if ipv6 { "[::]:0" } else { "0.0.0.0:0" }).await
    }

    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }

    async fn resolve(&self, host: &str, port: u16, until: Instant) -> Vec<IpAddr> {
        let lookup = tokio::net::lookup_host((host, port));
        match tokio::time::timeout_at(until.into(), lookup).await {
            Ok(Ok(addrs)) => addrs.map(|a| a.ip()).collect(),
            _ => Vec::new(),
        }
    }
}

#[cfg(feature = "tokio")]
impl Socket for tokio::net::UdpSocket {
    async fn send_to(&self, packet: &[u8], to: SocketAddr) -> std::io::Result<()> {
        tokio::net::UdpSocket::send_to(self, packet, to)
            .await
            .map(drop)
    }

    async fn recv_from(&self, buf: &mut [u8], until: Instant) -> Option<(usize, SocketAddr)> {
        let received = tokio::net::UdpSocket::recv_from(self, buf);
        tokio::time::timeout_at(until.into(), received)
            .await
            .ok()?
            .ok()
    }
}

const DNS_KEEP_MS: u64 = 7 * 24 * 3600 * 1000;

/// The gateway's address: cached for a week, refreshed by DNS after that
/// (and still used if the refresh fails).
pub(super) async fn resolve(
    net: &impl Net,
    cfg: &ClientConfig,
    deadline: Instant,
) -> Result<IpAddr, WireError> {
    if let Ok(ip) = cfg.host.parse::<IpAddr>() {
        return Ok(ip);
    }
    let path = cfg.state_dir.join("gateway.addr");
    let cached = fs::read_to_string(&path).ok().and_then(|t| {
        let f: Vec<&str> = t.split_whitespace().collect();
        match f[..] {
            [host, ip, at] if host == cfg.host => {
                Some((ip.parse::<IpAddr>().ok()?, at.parse::<u64>().ok()?))
            }
            _ => None,
        }
    });
    if let Some((ip, at)) = cached
        && now_ms().saturating_sub(at) < DNS_KEEP_MS
    {
        return Ok(ip);
    }
    let all = net
        .resolve(
            &cfg.host,
            cfg.ports.first().copied().unwrap_or_default(),
            deadline,
        )
        .await;
    let found = all.iter().find(|a| a.is_ipv4()).or(all.first()).copied();
    match (found, cached) {
        (Some(ip), _) => {
            #[expect(
                clippy::let_underscore_must_use,
                reason = "the cached address is a fallback: without it the next run resolves again"
            )]
            let _ = write_atomic(
                &path,
                format!("{} {ip} {}\n", cfg.host, now_ms()).as_bytes(),
            );
            Ok(ip)
        }
        (None, Some((ip, _))) => Ok(ip),
        (None, None) => Err(WireError::Unreachable),
    }
}

/// A name for the network this phone is on: the local address prefix the
/// route to the gateway uses (/16 for IPv4, /48 for IPv6).
pub(super) fn network_key(ip: IpAddr, port: u16) -> String {
    let local = std::net::UdpSocket::bind(if ip.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" })
        .and_then(|s| s.connect((ip, port)).map(|_| s))
        .and_then(|s| s.local_addr());
    match local.map(|a| a.ip()) {
        Ok(IpAddr::V4(v4)) => format!("{}.{}", v4.octets()[0], v4.octets()[1]),
        Ok(IpAddr::V6(v6)) => v6.segments()[..3]
            .iter()
            .map(|s| format!("{s:x}"))
            .collect::<Vec<_>>()
            .join("-"),
        Err(_) => "default".into(),
    }
}

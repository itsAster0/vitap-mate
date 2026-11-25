//! The phone side: one request, one response ([`fetch`]).
//!
//! What it keeps in `state_dir` between runs:
//! - `device.key`: the install's Noise static key. Not a credential (the
//!   gateway registers nothing); it ties message 1 timestamps to a device.
//! - `session.N`: a saved session (keys, next counter). Each run locks one
//!   slot for its whole fetch, so the app and a background isolate never
//!   share a counter; before using any counter the run saves the end of a
//!   block of [`COUNTER_BLOCK`], so a crash can skip counters but never
//!   reuse one. Sessions are replaced after [`REKEY`].
//! - `link-<network>`: the [`Link`] profile for each network (keyed by the
//!   local address prefix).
//! - `gateway.addr`: the gateway's resolved address, so a cold start on a
//!   slow link does not wait on DNS.
//!
//! The socket, the timers and DNS come from a [`Net`], so the client needs no
//! async runtime: the app passes its own (Kotlin's sockets), and [`fetch`]
//! uses tokio's (feature `tokio`, on by default).
//!
//! `run` is one fetch's exchange with the gateway; `state` keeps the sessions
//! and the device key; `net` holds the platform's side, the gateway's address
//! and the network's name.

mod net;
mod run;
pub(crate) mod state;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::link::Link;
use crate::msg::{MAX_REQUEST, now_ms};
#[cfg(feature = "tokio")]
pub use net::Tokio;
pub use net::{Net, Socket};
use net::{network_key, resolve};
use run::{Outcome, Run};
use state::{Slot, device_key, write_atomic};

pub const COUNTER_BLOCK: u32 = 1024;
pub const REKEY: Duration = Duration::from_secs(24 * 3600);
/// How long a gateway that never answers gets before the run gives up (UDP blocked or the
/// gateway down), unless this network's measured link needs longer for a handshake.
const SILENT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub host: String,
    pub ports: Vec<u16>,
    pub server_key: [u8; 32],
    pub state_dir: PathBuf,
    /// Whether a new session's request may ride in handshake message 1,
    /// saving a round trip. Message 1 is not forward-secret (a leaked
    /// gateway key would open recorded ones), so a secret goes after it.
    pub request_in_handshake: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum WireError {
    /// Nothing came back from the gateway: UDP is blocked or it is down.
    /// The caller should use HTTPS on this network.
    Unreachable,
    /// The gateway answered but the response did not complete in time.
    Timeout,
    /// The request does not fit one datagram.
    TooLarge,
    /// The gateway cannot read the handshake: the public key held for it is
    /// wrong or old. The caller should fetch it again.
    WrongKey,
    Io(String),
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable => write!(f, "the UDP gateway did not answer"),
            Self::Timeout => write!(f, "the UDP response did not complete in time"),
            Self::TooLarge => write!(f, "the request is too large for UDP"),
            Self::WrongKey => write!(f, "the UDP gateway has another key"),
            Self::Io(e) => write!(f, "UDP transport: {e}"),
        }
    }
}

impl std::error::Error for WireError {}

#[derive(Debug, Default)]
pub struct Fetched {
    pub body: Vec<u8>,
    /// Whether this fetch did a handshake.
    pub handshake: bool,
    pub elapsed: Duration,
    pub bytes_up: usize,
    pub bytes_down: usize,
    pub retries: usize,
    /// What the run learned about this network.
    pub link: Link,
}

/// [`fetch_over`] tokio's sockets.
#[cfg(feature = "tokio")]
pub async fn fetch(
    cfg: &ClientConfig,
    body: &[u8],
    timeout: Duration,
) -> Result<Fetched, WireError> {
    fetch_over(&Tokio, cfg, body, timeout).await
}

/// [`SILENT`], or three handshakes' worth of this network's measured link if longer.
fn silent_window(link: &Link) -> Duration {
    if !link.measured {
        return SILENT;
    }
    let handshake = link.rtt + 2.0 * 1300.0 / link.rate + link.slack();
    SILENT.max(Duration::from_secs_f64((3.0 * handshake).min(60.0)))
}

/// Sends `body` to the gateway and returns its answer; gives up after [`SILENT`] (or longer on a
/// slow measured link) when the gateway never answers.
pub async fn fetch_over<N: Net>(
    net: &N,
    cfg: &ClientConfig,
    body: &[u8],
    timeout: Duration,
) -> Result<Fetched, WireError> {
    if body.len() > MAX_REQUEST {
        return Err(WireError::TooLarge);
    }
    if cfg.ports.is_empty() {
        return Err(WireError::Io("no gateway ports".into()));
    }
    let start = Instant::now();
    let deadline = start + timeout;
    fs::create_dir_all(&cfg.state_dir).map_err(io)?;
    let device = device_key(&cfg.state_dir)?;
    let ip = resolve(net, cfg, deadline).await?;
    let mut slot = Slot::claim(&cfg.state_dir);
    let profile = cfg.state_dir.join(format!(
        "link-{}",
        network_key(ip, cfg.ports.first().copied().unwrap_or_default())
    ));
    let link = fs::read_to_string(&profile)
        .ok()
        .and_then(|t| Link::parse(&t))
        .unwrap_or_default();
    let sock = net.bind(ip.is_ipv6()).await.map_err(io)?;
    let mut run = Run {
        net,
        sock,
        ip,
        ports: &cfg.ports,
        server_key: cfg.server_key,
        device,
        silent_until: start + silent_window(&link),
        link,
        deadline,
        up: 0,
        down: 0,
        retries: 0,
        heard: false,
    };
    let saved = slot
        .as_mut()
        .and_then(Slot::read)
        .filter(|s| s.gateway == cfg.server_key)
        .filter(|s| now_ms().saturating_sub(s.created_ms) < REKEY.as_millis() as u64);
    let in_handshake = cfg.request_in_handshake;
    let mut result = match saved {
        Some(saved) => run.exchange(body, Some(saved), slot.as_mut()).await,
        None => run.fresh(body, in_handshake, slot.as_mut()).await,
    };
    if matches!(result, Err(Outcome::Reset)) {
        // The gateway forgot the session (it restarted): handshake now.
        result = run.fresh(body, in_handshake, slot.as_mut()).await;
    }
    #[expect(
        clippy::let_underscore_must_use,
        reason = "the saved link profile is a hint: without it the next run learns again"
    )]
    let _ = write_atomic(&profile, run.link.format().as_bytes());
    match result {
        Ok((body, saved, handshake)) => {
            if let Some(slot) = slot.as_mut() {
                slot.write(&saved);
            }
            Ok(Fetched {
                body,
                handshake,
                elapsed: start.elapsed(),
                bytes_up: run.up,
                bytes_down: run.down,
                retries: run.retries,
                link: run.link.clone(),
            })
        }
        Err(Outcome::Failed(e)) => Err(e),
        Err(Outcome::Reset) => Err(WireError::Timeout),
    }
}

fn io(e: std::io::Error) -> WireError {
    WireError::Io(e.to_string())
}

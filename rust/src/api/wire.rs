//! The experimental UDP mode: one page through `vtop-gateway` over
//! `vtop-wire`, for "Fetch on the server" on slow, lossy networks. Dart
//! falls back to HTTPS when this reports the gateway unreachable; see
//! `lib/core/vtop_backend/server_vtop.dart`.
//!
//! The VTOP session goes along only when the gateway may not hold it: on the
//! first fetch, after it changes, or after five hours (the gateway keeps it
//! six). Otherwise the request carries its 8-byte hash, and a gateway that
//! no longer has it answers "need session", which costs one more request.

use std::path::{Path, PathBuf};
use std::time::Duration;

use vtop_shared::wire_page::{
    session_hash, WirePageRequest, WirePageResponse, BENCH_PAGE, NEED_SESSION,
};
use vtop_wire::client::{fetch, ClientConfig, WireError};

/// Resend the session after this, a little inside the gateway's six hours.
const SESSION_RESEND_MS: u64 = 5 * 3600 * 1000;

pub struct WireConfig {
    pub host: String,
    pub ports: Vec<u16>,
    /// The gateway's public key, hex.
    pub server_key: String,
    /// Where the transport keeps its keys, sessions and link profiles.
    pub state_dir: String,
}

pub struct WirePage {
    /// The engine's HTTP status.
    pub status: u16,
    /// Packed with the page dictionary; otherwise JSON.
    pub packed: bool,
    pub body: Vec<u8>,
    pub elapsed_ms: u32,
    pub handshake: bool,
    /// Bytes on the wire, both ways, IP headers included.
    pub wire_bytes: u32,
}

pub enum WireFetchError {
    /// Nothing came back: UDP is blocked here or the gateway is down.
    Unreachable,
    /// The gateway answered but the page did not complete in time.
    Timeout,
    /// The request is too large for UDP, or the configuration is invalid.
    Unsupported(String),
    Failed(String),
}

#[allow(clippy::too_many_arguments)]
pub async fn wire_fetch_page(
    config: WireConfig,
    session_json: String,
    name: String,
    semester: Option<String>,
    args: Vec<(String, String)>,
    dictionary: Option<String>,
    timeout_ms: u32,
) -> Result<WirePage, WireFetchError> {
    let server_key: [u8; 32] = vtop_wire::unhex(&config.server_key)
        .and_then(|k| k.try_into().ok())
        .ok_or_else(|| {
            WireFetchError::Unsupported("the gateway key is not 32 bytes of hex".into())
        })?;
    let state_dir = PathBuf::from(&config.state_dir);
    let cfg = ClientConfig {
        host: config.host,
        ports: config.ports,
        server_key,
        state_dir: state_dir.clone(),
    };
    let hash = session_hash(&session_json);
    let mut request = WirePageRequest {
        name,
        semester,
        args: args.into_iter().collect(),
        dictionary,
        session_hash: hash,
        session: (!gateway_has(&state_dir, hash)).then(|| session_json.clone()),
    };
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms as u64);
    let (mut page, mut stats) = send(&cfg, &request, deadline).await?;
    if page.status == NEED_SESSION && request.session.is_none() {
        request.session = Some(session_json);
        let (again, more) = send(&cfg, &request, deadline).await?;
        page = again;
        stats.0 += more.0;
        stats.2 += more.2;
    }
    if request.session.is_some() && page.status < 500 && page.status != NEED_SESSION {
        remember_sent(&state_dir, hash);
    }
    Ok(WirePage {
        status: page.status,
        packed: page.packed,
        body: page.body,
        elapsed_ms: stats.0,
        handshake: stats.1,
        wire_bytes: stats.2,
    })
}

/// One exchange: the answer and (ms, handshake, wire bytes).
async fn send(
    cfg: &ClientConfig,
    request: &WirePageRequest,
    deadline: std::time::Instant,
) -> Result<(WirePageResponse, (u32, bool, u32)), WireFetchError> {
    let body = request.encode().ok_or_else(|| {
        WireFetchError::Unsupported("the request does not fit the wire format".into())
    })?;
    let left = deadline.saturating_duration_since(std::time::Instant::now());
    let fetched = fetch(cfg, &body, left).await.map_err(|e| match e {
        WireError::Unreachable => WireFetchError::Unreachable,
        WireError::Timeout => WireFetchError::Timeout,
        WireError::TooLarge => WireFetchError::Unsupported(e.to_string()),
        WireError::Io(_) => WireFetchError::Failed(e.to_string()),
    })?;
    let page = WirePageResponse::decode(&fetched.body)
        .ok_or_else(|| WireFetchError::Failed("the gateway's answer does not decode".into()))?;
    Ok((
        page,
        (
            fetched.elapsed.as_millis() as u32,
            fetched.handshake,
            (fetched.bytes_up + fetched.bytes_down) as u32,
        ),
    ))
}

/// Whether the gateway was sent the session with `hash` recently enough.
fn gateway_has(dir: &Path, hash: u64) -> bool {
    std::fs::read_to_string(dir.join("page-session"))
        .ok()
        .is_some_and(|t| {
            let f: Vec<&str> = t.split_whitespace().collect();
            matches!(f[..], [h, at] if h == hash.to_string()
            && at.parse::<u64>().is_ok_and(|at| now_ms().saturating_sub(at) < SESSION_RESEND_MS))
        })
}

fn remember_sent(dir: &Path, hash: u64) {
    let _ = std::fs::write(dir.join("page-session"), format!("{hash} {}\n", now_ms()));
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

/// One bench run: a synthetic page from the gateway, and what the transport
/// learned about the network.
pub struct WireBench {
    pub elapsed_ms: u32,
    pub bytes_up: u32,
    pub bytes_down: u32,
    pub retries: u32,
    pub handshake: bool,
    /// Learned link rate, bytes/s (0 until measured).
    pub rate: u32,
    pub rtt_ms: u32,
    /// The cautious loss estimate the transport sizes its spare symbols by.
    pub loss_pct: u32,
}

/// A synthetic page of `size` bytes from the gateway (no VTOP), for the
/// app's network bench. `config.state_dir` should be the bench's own, so it
/// neither uses nor disturbs the app's session; `new_session` drops the
/// saved one first to time a handshake.
pub async fn wire_bench(
    config: WireConfig,
    size: u32,
    new_session: bool,
    timeout_ms: u32,
) -> Result<WireBench, WireFetchError> {
    let server_key: [u8; 32] = vtop_wire::unhex(&config.server_key)
        .and_then(|k| k.try_into().ok())
        .ok_or_else(|| {
            WireFetchError::Unsupported("the gateway key is not 32 bytes of hex".into())
        })?;
    let state_dir = PathBuf::from(&config.state_dir);
    if new_session {
        for i in 0..8 {
            let _ = std::fs::remove_file(state_dir.join(format!("session.{i}")));
        }
    }
    let cfg = ClientConfig {
        host: config.host,
        ports: config.ports,
        server_key,
        state_dir,
    };
    let request = WirePageRequest {
        name: BENCH_PAGE.into(),
        semester: None,
        args: [("size".to_string(), size.to_string())]
            .into_iter()
            .collect(),
        dictionary: None,
        session_hash: 0,
        session: None,
    };
    let body = request.encode().ok_or_else(|| {
        WireFetchError::Unsupported("the request does not fit the wire format".into())
    })?;
    let fetched = fetch(&cfg, &body, Duration::from_millis(timeout_ms as u64))
        .await
        .map_err(|e| match e {
            WireError::Unreachable => WireFetchError::Unreachable,
            WireError::Timeout => WireFetchError::Timeout,
            other => WireFetchError::Failed(other.to_string()),
        })?;
    // Checked, not used: the bench measures the link.
    WirePageResponse::decode(&fetched.body)
        .filter(|p| p.status == 200 && p.body.len() == size as usize)
        .ok_or_else(|| WireFetchError::Failed("the bench page came back wrong".into()))?;
    let link = &fetched.link;
    Ok(WireBench {
        elapsed_ms: fetched.elapsed.as_millis() as u32,
        bytes_up: fetched.bytes_up as u32,
        bytes_down: fetched.bytes_down as u32,
        retries: fetched.retries as u32,
        handshake: fetched.handshake,
        rate: if link.measured { link.rate as u32 } else { 0 },
        rtt_ms: (link.rtt * 1000.0) as u32,
        loss_pct: (link.loss() * 100.0).round() as u32,
    })
}

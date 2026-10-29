//! The experimental UDP mode: one page through `vtop-gateway` over
//! `vtop-wire`, for "Fetch on the server" on slow, lossy networks. Dart
//! falls back to HTTPS when this reports the gateway unreachable; see
//! `lib/vtop/backend/server_vtop.dart`.
//!
//! The VTOP session goes along only when the gateway may not hold it: on the
//! first fetch, after it changes, or after five hours (the gateway keeps it
//! six). Otherwise the request carries its 8-byte hash, and a gateway that
//! no longer has it answers "need session", which costs one more request.

use std::path::{Path, PathBuf};
use std::time::Duration;

use vtop_shared::wire_page::{session_hash, WirePageRequest, WirePageResponse, NEED_SESSION};
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
    /// The session the gateway renewed for this page; use it from now on.
    pub session: Option<String>,
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

/// One request to the gateway: a page on `session_json`'s session, or with
/// `session_json` empty a request that needs none (a server sign-in). A
/// `secret` request goes only on a forward-secret session, after the
/// handshake rather than in it. `account_json` (`/v1/app/login`'s body)
/// goes with the session, so the gateway can renew it in the fetch; it is
/// secret too.
#[allow(clippy::too_many_arguments)]
pub async fn wire_fetch_page(
    config: WireConfig,
    session_json: String,
    name: String,
    semester: Option<String>,
    args: Vec<(String, String)>,
    dictionary: Option<String>,
    account_json: Option<String>,
    secret: bool,
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
        request_in_handshake: !secret && account_json.is_none(),
    };
    let hash = session_hash(&session_json);
    let mut request = WirePageRequest {
        name,
        semester,
        args: args.into_iter().collect(),
        dictionary,
        session_hash: hash,
        session: (!session_json.is_empty() && !gateway_has(&state_dir, hash))
            .then(|| session_json.clone()),
        account: None,
    };
    if request.session.is_some() {
        request.account = account_json.clone();
    }
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms as u64);
    let (mut page, mut stats) = send(&cfg, &request, deadline).await?;
    if page.status == NEED_SESSION && request.session.is_none() && !session_json.is_empty() {
        request.session = Some(session_json);
        request.account = account_json;
        let (again, more) = send(&cfg, &request, deadline).await?;
        page = again;
        stats.0 += more.0;
        stats.2 += more.2;
    }
    if request.session.is_some() && page.status < 500 && page.status != NEED_SESSION {
        remember_sent(&state_dir, hash);
    }
    if let Some(renewed) = &page.session {
        remember_sent(&state_dir, session_hash(renewed));
    }
    Ok(WirePage {
        status: page.status,
        packed: page.packed,
        body: page.body,
        elapsed_ms: stats.0,
        handshake: stats.1,
        wire_bytes: stats.2,
        session: page.session,
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

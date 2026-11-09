//! The UDP gateway: phones speak `vtop-wire` to it, it asks the engine.
//!
//! ```text
//! phone ──UDP (vtop-wire)──► gateway ──HTTPS──► engine /engine/fetch ──► VTOP
//! ```
//!
//! Page fetches and sign-ins ([`SIGN_IN_BODY`]); changes to VTOP stay on
//! HTTPS. It runs on its own host (UDP needs one) and calls the engine
//! exactly as the edge does (`Authorization: Bearer $INTERNAL_TOKEN`, the
//! phone's session in the body) and passes the answer back byte for byte,
//! so it holds no VTOP logic. It makes its own Noise keys ([`keys`]): a new
//! one every Monday 00:00 IST, the previous kept a week. At start and after
//! each rotation it posts the public halves, with where it listens, to the
//! edge's [`UDP_KEYS_INTERNAL`], which hands them to the app. What it keeps
//! in memory: Noise sessions, and each device's VTOP session for six hours,
//! so a fetch can carry an 8-byte hash instead of ~700 bytes of cookies
//! (see `vtop_shared::wire_page`).

pub mod keys;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use vtop_shared::engine::{
    PageFetch, FETCH, UDP_KEYS_INTERNAL, VTOP_LOGIN, VTOP_LOGIN_OTP, VTOP_LOGIN_RESEND,
};
use vtop_shared::page_dictionary;
use vtop_shared::wire_page::{
    session_hash, WirePageRequest, WirePageResponse, BUSY, NEED_SESSION, SIGN_IN_BODY, TOO_LARGE,
    UNSUPPORTED, VERSION,
};
use vtop_wire::fec::MAX_RESPONSE;
use vtop_wire::gateway::{Gateway, NewRequest, Token};

/// Over the engine's own VTOP limit (25 s a page), like the edge's.
const ENGINE_TIMEOUT: Duration = Duration::from_secs(32);
/// Over the edge's wait for a sign-in by the server (90 s), which includes
/// the engine's wait for the OTP mail.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(95);
const SESSION_KEEP: Duration = Duration::from_secs(6 * 3600);
const MAX_SESSIONS: usize = 20_000;
/// Engine calls at once; more are told to use HTTPS. Anyone with the app
/// can make a device key, so this bounds what UDP mode costs the engine
/// and VTOP.
const MAX_ENGINE_CALLS: usize = 256;
const SWEEP_EVERY: Duration = Duration::from_secs(30);
/// The longest sleep between checks for a new week, so a changed clock is
/// noticed within it.
const ROTATE_CHECK: Duration = Duration::from_secs(3600);
/// The first wait after a failed post of the keys to the edge; it doubles
/// up to [`ANNOUNCE_RETRY_MAX`].
const ANNOUNCE_RETRY: Duration = Duration::from_secs(5);
const ANNOUNCE_RETRY_MAX: Duration = Duration::from_secs(600);

/// No `Debug`: it holds the internal token.
pub struct Config {
    pub engine_url: String,
    /// The edge: `/v1/app/login`, and [`UDP_KEYS_INTERNAL`] for the keys.
    pub edge_url: String,
    /// `INTERNAL_TOKEN`, shared with the engine and the edge.
    pub token: String,
    /// The public host name or address the app sends UDP to.
    pub host: String,
    /// The public UDP ports, as the app should try them.
    pub ports: Vec<u16>,
    /// Where the keys are kept (`GATEWAY_DATA_DIR`).
    pub data_dir: PathBuf,
}

/// The public key the app must hold for `private`.
pub fn public_key(private: &[u8; 32]) -> [u8; 32] {
    // Noise's 25519 keys are plain X25519 keys.
    x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from(*private)).to_bytes()
}

/// One socket per port on `bind`.
pub async fn bind(bind: &str, ports: &[u16]) -> Result<Vec<UdpSocket>, String> {
    let mut sockets = Vec::new();
    for port in ports {
        let socket = UdpSocket::bind((bind, *port))
            .await
            .map_err(|e| format!("cannot bind {bind}:{port}: {e}"))?;
        sockets.push(socket);
    }
    tracing::info!(?ports, bind, "UDP gateway listening");
    Ok(sockets)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// This week's keys, newest first; retried while the data dir fails.
async fn current_keys(dir: &std::path::Path) -> Vec<keys::Key> {
    loop {
        match keys::rotate(dir, unix_now()) {
            Ok(keys) => return keys,
            Err(e) => {
                tracing::warn!("keys: {e}");
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

/// Posts the public keys to the edge until it takes them, backing off.
async fn announce(http: reqwest::Client, url: String, token: String, body: keys::Announce) {
    let mut wait = ANNOUNCE_RETRY;
    loop {
        let sent = http
            .post(&url)
            .bearer_auth(&token)
            .timeout(Duration::from_secs(10))
            .json(&body)
            .send()
            .await;
        match sent {
            Ok(answer) if answer.status().is_success() => {
                tracing::info!(keys = body.keys.len(), "keys posted to the edge");
                return;
            }
            Ok(answer) => tracing::warn!(
                "edge took no keys: HTTP {}; again in {}s",
                answer.status().as_u16(),
                wait.as_secs()
            ),
            Err(e) => tracing::warn!(
                "edge took no keys: {}; again in {}s",
                e.without_url(),
                wait.as_secs()
            ),
        }
        tokio::time::sleep(wait).await;
        wait = (wait * 2).min(ANNOUNCE_RETRY_MAX);
    }
}

/// Serves on `sockets` (one per port) until the process ends.
pub async fn run(config: Config, sockets: Vec<UdpSocket>) {
    let engine = Arc::new(Engine::new(
        config.engine_url,
        config.token.clone(),
        config.edge_url.clone(),
    ));
    let keys = current_keys(&config.data_dir).await;
    let privates = |keys: &[keys::Key]| keys.iter().map(keys::Key::private).collect::<Vec<_>>();
    let initial = privates(&keys);
    tracing::info!(key = keys[0].id(), "keys");
    let (keys_tx, mut keys_rx) = mpsc::channel::<Vec<[u8; 32]>>(1);
    let announce_to = format!("{}{UDP_KEYS_INTERNAL}", engine.edge_url);
    tokio::spawn(async move {
        let post = |keys: &[keys::Key]| {
            let body = keys::Announce::new(&config.host, &config.ports, keys);
            tokio::spawn(announce(
                engine_http(),
                announce_to.clone(),
                config.token.clone(),
                body,
            ))
        };
        let mut posting = post(&keys);
        let mut week = keys[0].from;
        loop {
            let next = week + keys::WEEK_SECS;
            let wait = Duration::from_secs(next.saturating_sub(unix_now()) + 1);
            tokio::time::sleep(wait.min(ROTATE_CHECK)).await;
            if keys::week_start(unix_now()) == week {
                continue;
            }
            let keys = current_keys(&config.data_dir).await;
            week = keys[0].from;
            tracing::info!(key = keys[0].id(), "a new key");
            if keys_tx.send(privates(&keys)).await.is_err() {
                return;
            }
            posting.abort();
            posting = post(&keys);
        }
    });
    let sockets: Vec<Arc<UdpSocket>> = sockets.into_iter().map(Arc::new).collect();
    let (in_tx, mut in_rx) = mpsc::channel::<(usize, SocketAddr, Vec<u8>)>(4096);
    for (i, sock) in sockets.iter().enumerate() {
        let (sock, tx) = (sock.clone(), in_tx.clone());
        tokio::spawn(async move {
            let mut buf = vec![0u8; 2048];
            loop {
                match sock.recv_from(&mut buf).await {
                    Ok((n, from)) => {
                        let _ = tx.try_send((i, from, buf[..n].to_vec()));
                    }
                    Err(e) => tracing::debug!("udp receive: {e}"),
                }
            }
        });
    }
    let (done_tx, mut done_rx) = mpsc::unbounded_channel::<(Token, Vec<u8>)>();
    let calls = Arc::new(tokio::sync::Semaphore::new(MAX_ENGINE_CALLS));
    let mut gw = Gateway::new(initial);
    let mut last_sweep = Instant::now();
    loop {
        let wake = gw
            .next_due()
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(1));
        tokio::select! {
            Some((socket, from, packet)) = in_rx.recv() => {
                if let Some(request) = gw.receive(&packet, socket, from, Instant::now()) {
                    let (engine, tx) = (engine.clone(), done_tx.clone());
                    let permit = calls.clone().try_acquire_owned();
                    tokio::spawn(async move {
                        let token = request.token;
                        let body = match permit {
                            Ok(_permit) => engine.answer(request).await,
                            Err(_) => {
                                tracing::warn!("engine calls at the limit; sent BUSY");
                                error(BUSY, "busy", "The server is busy.").encode()
                            }
                        };
                        let _ = tx.send((token, body));
                    });
                }
            }
            Some((token, body)) = done_rx.recv() => gw.respond(token, body, Instant::now()),
            Some(keys) = keys_rx.recv() => gw.set_keys(keys),
            _ = tokio::time::sleep_until(tokio::time::Instant::from_std(wake)) => {}
        }
        for out in gw.poll(Instant::now()) {
            if let Err(e) = sockets[out.socket].send_to(&out.packet, out.to).await {
                tracing::debug!("udp send to {}: {e}", out.to);
            }
        }
        if last_sweep.elapsed() > SWEEP_EVERY {
            gw.sweep(Instant::now());
            engine.sweep();
            last_sweep = Instant::now();
            tracing::debug!(sessions = gw.session_count(), "sweep");
        }
    }
}

/// A device's VTOP session, and its account when the server may sign in
/// for it (`/v1/app/login`'s body).
struct Kept {
    hash: u64,
    session: serde_json::Value,
    account: Option<String>,
    at: Instant,
}

struct Engine {
    http: reqwest::Client,
    url: String,
    token: String,
    edge_url: String,
    sessions: std::sync::Mutex<HashMap<[u8; 32], Kept>>,
}

impl Engine {
    fn new(url: String, token: String, edge_url: String) -> Self {
        Self {
            http: engine_http(),
            url: url.trim_end_matches('/').to_string(),
            token,
            edge_url: edge_url.trim_end_matches('/').to_string(),
            sessions: std::sync::Mutex::default(),
        }
    }

    fn sweep(&self) {
        let mut s = self.sessions.lock().expect("lock");
        s.retain(|_, kept| kept.at.elapsed() < SESSION_KEEP);
    }

    /// The encoded [`WirePageResponse`] for a request.
    /// Logged with the phone's flow id (`request_id`), which also goes to
    /// the engine and the edge as `x-request-id`.
    async fn answer(&self, request: NewRequest) -> Vec<u8> {
        let started = Instant::now();
        let page = WirePageRequest::decode(&request.body);
        let response = match &page {
            Some(page) => self.page(&request.device, page).await,
            None if WirePageRequest::version(&request.body) > VERSION => error(
                UNSUPPORTED,
                "unsupported_version",
                "The gateway is older than the app.",
            ),
            None => error(400, "bad_request", "The request does not decode."),
        };
        let (request_id, name) = page.as_ref().map_or(("", ""), |page| {
            (page.request_id.as_deref().unwrap_or(""), page.name.as_str())
        });
        tracing::info!(
            request_id,
            name,
            status = response.status,
            packed = response.packed,
            bytes = response.body.len(),
            ms = started.elapsed().as_millis() as u64,
            "fetch"
        );
        let encoded = response.encode();
        if encoded.len() > MAX_RESPONSE {
            return error(
                TOO_LARGE,
                "too_large",
                "The page is too large for UDP mode.",
            )
            .encode();
        }
        encoded
    }

    async fn page(&self, device: &[u8; 32], page: &WirePageRequest) -> WirePageResponse {
        if let Some(body) = page
            .args
            .get(SIGN_IN_BODY)
            .filter(|_| page.name.starts_with('/'))
        {
            return self.sign_in(page, &page.name, body.clone()).await;
        }
        let Some((session, account)) = self.session(device, page) else {
            return error(NEED_SESSION, "need_session", "Send the session again.");
        };
        let answer = self.fetch(session, page).await;
        match account {
            Some(account) if expired(&answer) => self.renewed(device, page, account, answer).await,
            _ => answer,
        }
    }

    /// [page] from the engine on [session].
    async fn fetch(&self, session: serde_json::Value, page: &WirePageRequest) -> WirePageResponse {
        let body = PageFetch {
            session,
            name: page.name.clone(),
            semester: page.semester.clone(),
            args: page.args.clone(),
            dictionary: page.dictionary.clone(),
        };
        let sent = self
            .post(page, format!("{}{FETCH}", self.url))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await;
        let answer = match sent {
            Ok(answer) => answer,
            Err(e) if e.is_timeout() => {
                return error(504, "vtop_timeout", "VTOP took too long to answer.")
            }
            Err(e) => {
                tracing::warn!("engine: {e}");
                return error(
                    503,
                    "engine_unavailable",
                    "The server cannot reach the engine.",
                );
            }
        };
        let status = answer.status().as_u16();
        let packed = answer
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .is_some_and(|v| v.as_bytes() == page_dictionary::CONTENT_TYPE.as_bytes());
        match answer.bytes().await {
            Ok(bytes) => WirePageResponse {
                status,
                packed,
                body: bytes.to_vec(),
                session: None,
            },
            Err(_) => error(503, "engine_unavailable", "The engine's answer broke off."),
        }
    }

    /// The session expired: the server signs in for the phone's account (or
    /// hands out the session it keeps for connected apps), and [page] is
    /// fetched again on the new one, which goes back with it. One try. A
    /// rejected password goes back as it is; any other failure, the
    /// [expired] answer, so the phone signs in as usual.
    async fn renewed(
        &self,
        device: &[u8; 32],
        page: &WirePageRequest,
        account: String,
        expired: WirePageResponse,
    ) -> WirePageResponse {
        let login = self.sign_in(page, "/v1/app/login", account.clone()).await;
        if login.status == 401 {
            return login;
        }
        let session = (login.status == 200)
            .then(|| serde_json::from_slice::<serde_json::Value>(&login.body).ok())
            .flatten()
            .and_then(|body| body.get("session").cloned());
        let Some(session) = session else {
            return expired;
        };
        let json = session.to_string();
        self.keep(device, session_hash(&json), session.clone(), Some(account));
        tracing::info!("renewed a session in the fetch");
        let mut answer = self.fetch(session, page).await;
        answer.session = Some(json);
        answer
    }

    /// A sign-in request, named by the edge path the phone would use over
    /// HTTPS, answered as that path answers: the phone's own sign-in goes
    /// to the engine as the edge sends it, a sign-in by the server with its
    /// saved password to the edge (the phone proves it is linked with its
    /// app secret, and the server reads the OTP itself).
    async fn sign_in(&self, page: &WirePageRequest, path: &str, body: String) -> WirePageResponse {
        let to_engine = |route: &str| {
            self.post(page, format!("{}{route}", self.url))
                .bearer_auth(&self.token)
        };
        let request = match path {
            "/v1/vtop/login" => to_engine(VTOP_LOGIN),
            "/v1/vtop/login/otp" => to_engine(VTOP_LOGIN_OTP),
            "/v1/vtop/login/resend" => to_engine(VTOP_LOGIN_RESEND),
            "/v1/app/login" => self
                .post(page, format!("{}{path}", self.edge_url))
                .timeout(LOGIN_TIMEOUT),
            _ => return error(404, "not_found", "No such sign-in."),
        };
        let sent = request
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await;
        let answer = match sent {
            Ok(answer) => answer,
            Err(e) if e.is_timeout() => {
                return error(504, "vtop_timeout", "VTOP took too long to answer.")
            }
            Err(e) => {
                tracing::warn!("sign-in {path}: {e}");
                return error(
                    503,
                    "engine_unavailable",
                    "The server cannot sign in right now.",
                );
            }
        };
        let status = answer.status().as_u16();
        match answer.bytes().await {
            Ok(bytes) => WirePageResponse {
                status,
                packed: false,
                body: bytes.to_vec(),
                session: None,
            },
            Err(_) => error(503, "engine_unavailable", "The answer broke off."),
        }
    }

    /// A POST to [url] for [page], with its flow id, marked as having come
    /// over UDP (`x-via`, which the engine logs).
    fn post(&self, page: &WirePageRequest, url: String) -> reqwest::RequestBuilder {
        let request = self.http.post(url).header("x-via", "udp");
        match &page.request_id {
            Some(id) => request.header("x-request-id", id),
            None => request,
        }
    }

    /// The device's VTOP session and account: from this request (and then
    /// kept), or the kept ones when the hash matches.
    fn session(
        &self,
        device: &[u8; 32],
        page: &WirePageRequest,
    ) -> Option<(serde_json::Value, Option<String>)> {
        if let Some(json) = &page.session {
            if session_hash(json) != page.session_hash {
                return None;
            }
            let value: serde_json::Value = serde_json::from_str(json).ok()?;
            self.keep(
                device,
                page.session_hash,
                value.clone(),
                page.account.clone(),
            );
            return Some((value, page.account.clone()));
        }
        let mut sessions = self.sessions.lock().expect("lock");
        let kept = sessions.get_mut(device)?;
        if kept.hash != page.session_hash || kept.at.elapsed() > SESSION_KEEP {
            return None;
        }
        kept.at = Instant::now();
        Some((kept.session.clone(), kept.account.clone()))
    }

    fn keep(
        &self,
        device: &[u8; 32],
        hash: u64,
        session: serde_json::Value,
        account: Option<String>,
    ) {
        let mut sessions = self.sessions.lock().expect("lock");
        if sessions.len() >= MAX_SESSIONS && !sessions.contains_key(device) {
            // Rare: forget the stalest to make room.
            if let Some(oldest) = sessions
                .iter()
                .min_by_key(|(_, kept)| kept.at)
                .map(|(d, _)| *d)
            {
                sessions.remove(&oldest);
            }
        }
        let at = Instant::now();
        sessions.insert(
            *device,
            Kept {
                hash,
                session,
                account,
                at,
            },
        );
    }
}

fn engine_http() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(ENGINE_TIMEOUT)
        .build()
        .expect("HTTP client")
}

/// Whether the engine said the session expired.
fn expired(answer: &WirePageResponse) -> bool {
    answer.status == 401
        && serde_json::from_slice::<serde_json::Value>(&answer.body)
            .ok()
            .is_some_and(|body| body["code"] == "session_expired")
}

fn error(status: u16, code: &str, message: &str) -> WirePageResponse {
    WirePageResponse {
        status,
        packed: false,
        body: serde_json::json!({ "code": code, "error": message })
            .to_string()
            .into_bytes(),
        session: None,
    }
}

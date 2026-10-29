//! The UDP gateway: phones speak `vtop-wire` to it, it asks the engine.
//!
//! ```text
//! phone ──UDP (vtop-wire)──► gateway ──loopback──► engine /engine/fetch ──► VTOP
//! ```
//!
//! Page fetches and sign-ins ([`SIGN_IN_BODY`]); changes to VTOP stay on
//! HTTPS. It
//! runs in the engine's process and calls it exactly as the edge does (`BRIDGE_SECRET`, the phone's
//! session in the body) and passes the answer back byte for byte, so it
//! holds no VTOP logic. What it keeps, in memory only: Noise sessions, and
//! each device's VTOP session for six hours, so a fetch can carry an 8-byte
//! hash instead of ~700 bytes of cookies (see `vtop_shared::wire_page`).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use vtop_shared::engine::{PageFetch, FETCH, VTOP_LOGIN, VTOP_LOGIN_OTP, VTOP_LOGIN_RESEND};
use vtop_shared::page_dictionary;
use vtop_shared::wire_page::{
    session_hash, WirePageRequest, WirePageResponse, BUSY, NEED_SESSION, SIGN_IN_BODY, TOO_LARGE,
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

pub struct Config {
    pub engine_url: String,
    /// The edge, for `/v1/app/login`.
    pub edge_url: String,
    /// The engine's relay secret (the edge's `BRIDGE_SECRET`).
    pub engine_secret: String,
    pub private_key: [u8; 32],
}

/// The public key the app must hold for `private`.
pub fn public_key(private: &[u8; 32]) -> [u8; 32] {
    // Noise's 25519 keys are plain X25519 keys.
    x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from(*private)).to_bytes()
}

/// The gateway from the environment, its sockets bound, for an engine at
/// `engine_url` with `engine_secret` and the edge at `edge_url`; None when
/// `GATEWAY_KEY` is unset.
/// Reads `GATEWAY_KEY` (Noise private key, hex), `BIND` (default `0.0.0.0`;
/// on Fly UDP must bind `fly-global-services`) and `PORTS` (comma-separated,
/// default 3478,51820,443). The engine (vtop-bridge) calls this to run the
/// gateway in its own process; `vtop-gateway` alone does too.
pub async fn bind_from_env(
    engine_url: String,
    engine_secret: String,
    edge_url: String,
) -> Option<Result<(Config, Vec<UdpSocket>), String>> {
    let key = std::env::var("GATEWAY_KEY")
        .ok()
        .filter(|k| !k.is_empty())?;
    Some(bind(key, engine_url, engine_secret, edge_url).await)
}

async fn bind(
    key: String,
    engine_url: String,
    engine_secret: String,
    edge_url: String,
) -> Result<(Config, Vec<UdpSocket>), String> {
    let private_key = vtop_wire::unhex(&key)
        .and_then(|k| k.try_into().ok())
        .ok_or("GATEWAY_KEY is not 32 bytes of hex")?;
    let bind = std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0".into());
    let ports: Vec<u16> = match std::env::var("PORTS") {
        Ok(p) => p.split(',').filter_map(|s| s.trim().parse().ok()).collect(),
        Err(_) => vtop_wire::PORTS.to_vec(),
    };
    let mut sockets = Vec::new();
    for port in &ports {
        let socket = UdpSocket::bind((bind.as_str(), *port))
            .await
            .map_err(|e| format!("cannot bind {bind}:{port}: {e}"))?;
        sockets.push(socket);
    }
    tracing::info!(
        public_key = vtop_wire::hex(&public_key(&private_key)),
        ?ports,
        bind,
        "UDP gateway listening"
    );
    Ok((
        Config {
            engine_url,
            edge_url,
            engine_secret,
            private_key,
        },
        sockets,
    ))
}

/// Serves on `sockets` (one per port) until the process ends.
pub async fn run(config: Config, sockets: Vec<UdpSocket>) {
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
    let engine = Arc::new(Engine::new(
        config.engine_url,
        config.engine_secret,
        config.edge_url,
    ));
    let calls = Arc::new(tokio::sync::Semaphore::new(MAX_ENGINE_CALLS));
    let mut gw = Gateway::new(config.private_key);
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
    secret: String,
    edge_url: String,
    sessions: std::sync::Mutex<HashMap<[u8; 32], Kept>>,
}

impl Engine {
    fn new(url: String, secret: String, edge_url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(ENGINE_TIMEOUT)
            .build()
            .expect("HTTP client");
        Self {
            http,
            url: url.trim_end_matches('/').to_string(),
            secret,
            edge_url: edge_url.trim_end_matches('/').to_string(),
            sessions: std::sync::Mutex::default(),
        }
    }

    fn sweep(&self) {
        let mut s = self.sessions.lock().expect("lock");
        s.retain(|_, kept| kept.at.elapsed() < SESSION_KEEP);
    }

    /// The encoded [`WirePageResponse`] for a request.
    async fn answer(&self, request: NewRequest) -> Vec<u8> {
        let started = Instant::now();
        let response = self.page(&request).await;
        tracing::info!(
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

    async fn page(&self, request: &NewRequest) -> WirePageResponse {
        let Some(page) = WirePageRequest::decode(&request.body) else {
            return error(400, "bad_request", "The request does not decode.");
        };
        if let Some(body) = page
            .args
            .get(SIGN_IN_BODY)
            .filter(|_| page.name.starts_with('/'))
        {
            return self.sign_in(&page.name, body.clone()).await;
        }
        let Some((session, account)) = self.session(&request.device, &page) else {
            return error(NEED_SESSION, "need_session", "Send the session again.");
        };
        let answer = self.fetch(session, &page).await;
        match account {
            Some(account) if expired(&answer) => {
                self.renewed(&request.device, &page, account, answer).await
            }
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
            .http
            .post(format!("{}{FETCH}", self.url))
            .bearer_auth(&self.secret)
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
        let login = self.sign_in("/v1/app/login", account.clone()).await;
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
    async fn sign_in(&self, path: &str, body: String) -> WirePageResponse {
        let to_engine = |route: &str| {
            self.http
                .post(format!("{}{route}", self.url))
                .bearer_auth(&self.secret)
        };
        let request = match path {
            "/v1/vtop/login" => to_engine(VTOP_LOGIN),
            "/v1/vtop/login/otp" => to_engine(VTOP_LOGIN_OTP),
            "/v1/vtop/login/resend" => to_engine(VTOP_LOGIN_RESEND),
            "/v1/app/login" => self
                .http
                .post(format!("{}{path}", self.edge_url))
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

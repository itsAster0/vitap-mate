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

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;

use crate::fec::{self, DATA_WIRE};
use crate::frame::{
    same_token, Keys, IP_UDP, RESET_TOKEN, T_DATA, T_DONE, T_HS1, T_HS2, T_REQ, T_RESET, T_WORKING,
};
use crate::link::Link;
use crate::msg::{now_ms, now_us, Data, Req, MAX_REQUEST};
use crate::{hex, unhex, NOISE};

pub const COUNTER_BLOCK: u32 = 1024;
pub const REKEY: Duration = Duration::from_secs(24 * 3600);
const SLOTS: usize = 4;
const DNS_KEEP_MS: u64 = 7 * 24 * 3600 * 1000;
/// Gap between a control packet and its duplicate.
const DUPLICATE_GAP: Duration = Duration::from_millis(30);

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub host: String,
    pub ports: Vec<u16>,
    pub server_key: [u8; 32],
    pub state_dir: PathBuf,
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
    Io(String),
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable => write!(f, "the UDP gateway did not answer"),
            Self::Timeout => write!(f, "the UDP response did not complete in time"),
            Self::TooLarge => write!(f, "the request is too large for UDP"),
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

/// Sends `body` to the gateway and returns its answer.
pub async fn fetch(
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
    let ip = resolve(cfg, deadline).await?;
    let mut slot = Slot::claim(&cfg.state_dir);
    let profile = cfg
        .state_dir
        .join(format!("link-{}", network_key(ip, cfg.ports[0])));
    let link = fs::read_to_string(&profile)
        .ok()
        .and_then(|t| Link::parse(&t))
        .unwrap_or_default();
    let bind: SocketAddr = if ip.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" }
        .parse()
        .expect("valid");
    let sock = UdpSocket::bind(bind).await.map_err(io)?;
    let mut run = Run {
        sock,
        ip,
        ports: &cfg.ports,
        server_key: cfg.server_key,
        device,
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
        .filter(|s| now_ms().saturating_sub(s.created_ms) < REKEY.as_millis() as u64);
    let mut result = run.exchange(body, saved, slot.as_mut()).await;
    if matches!(result, Err(Outcome::Reset)) {
        // The gateway forgot the session (it restarted): handshake now.
        result = run.exchange(body, None, slot.as_mut()).await;
    }
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

enum Outcome {
    Reset,
    Failed(WireError),
}

#[derive(Clone, Debug, PartialEq)]
struct Saved {
    sid: u32,
    send: [u8; 32],
    recv: [u8; 32],
    next_ctr: u32,
    port: usize,
    created_ms: u64,
    /// The gateway's proof for a reset of this session.
    reset: [u8; RESET_TOKEN],
}

impl Saved {
    fn parse(text: &str) -> Option<Self> {
        let f: Vec<&str> = text.split_whitespace().collect();
        let [sid, send, recv, next, port, created, reset] = f[..] else {
            return None; // also a session saved before reset tokens: hand-shake again
        };
        Some(Self {
            sid: sid.parse().ok()?,
            send: unhex(send)?.try_into().ok()?,
            recv: unhex(recv)?.try_into().ok()?,
            next_ctr: next.parse().ok()?,
            port: port.parse().ok()?,
            created_ms: created.parse().ok()?,
            reset: unhex(reset)?.try_into().ok()?,
        })
    }

    fn format(&self) -> String {
        format!(
            "{} {} {} {} {} {} {}\n",
            self.sid,
            hex(&self.send),
            hex(&self.recv),
            self.next_ctr,
            self.port,
            self.created_ms,
            hex(&self.reset)
        )
    }
}

/// A session file this run holds the lock on.
struct Slot {
    file: File,
}

impl Slot {
    /// The first free slot; None when all are busy (the run then does a
    /// handshake and keeps nothing).
    fn claim(dir: &Path) -> Option<Self> {
        (0..SLOTS).find_map(|i| {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(dir.join(format!("session.{i}")))
                .ok()?;
            file.try_lock().ok()?;
            Some(Self { file })
        })
    }

    fn read(&mut self) -> Option<Saved> {
        let mut text = String::new();
        self.file.seek(SeekFrom::Start(0)).ok()?;
        self.file.read_to_string(&mut text).ok()?;
        Saved::parse(&text)
    }

    /// Durable before it returns: the counter reservation depends on it.
    fn write(&mut self, saved: &Saved) -> bool {
        let text = saved.format();
        self.file.set_len(0).is_ok()
            && self.file.seek(SeekFrom::Start(0)).is_ok()
            && self.file.write_all(text.as_bytes()).is_ok()
            && self.file.sync_all().is_ok()
    }
}

struct Round {
    len: Option<u8>,
    maxpos: i32,
    requested: usize,
    sent_at: Instant,
    received: usize,
}

struct Run<'a> {
    sock: UdpSocket,
    ip: IpAddr,
    ports: &'a [u16],
    server_key: [u8; 32],
    device: [u8; 32],
    link: Link,
    deadline: Instant,
    up: usize,
    down: usize,
    retries: usize,
    /// Whether the gateway answered anything at all.
    heard: bool,
}

impl Run<'_> {
    async fn send(&mut self, port: usize, packet: &[u8], duplicate: bool) {
        let to = SocketAddr::new(self.ip, self.ports[port % self.ports.len()]);
        for copy in 0..if duplicate { 2 } else { 1 } {
            if copy > 0 {
                tokio::time::sleep(DUPLICATE_GAP).await;
            }
            if self.sock.send_to(packet, to).await.is_ok() {
                self.up += packet.len() + IP_UDP;
            }
        }
    }

    async fn recv(&mut self, until: Instant, buf: &mut [u8]) -> Option<usize> {
        let until = tokio::time::Instant::from_std(until.min(self.deadline));
        loop {
            let Ok(Ok((n, from))) = tokio::time::timeout_at(until, self.sock.recv_from(buf)).await
            else {
                return None;
            };
            if from.ip() == self.ip {
                self.down += n + IP_UDP;
                self.heard = true;
                return Some(n);
            }
        }
    }

    fn failure(&self) -> Outcome {
        Outcome::Failed(if self.heard {
            WireError::Timeout
        } else {
            WireError::Unreachable
        })
    }

    /// One request/response on a saved session, or on a new one.
    async fn exchange(
        &mut self,
        body: &[u8],
        saved: Option<Saved>,
        slot: Option<&mut Slot>,
    ) -> Result<(Vec<u8>, Saved, bool), Outcome> {
        let req_id = crate::rand::u16();
        let mut buf = [0u8; 1500];
        let mut lost_events = 0usize;
        let mut sent_events = 0usize;
        // Packets that arrive before HS2 (they need its keys).
        let mut early: Vec<(Instant, Vec<u8>)> = Vec::new();
        let mut hs2_at = None;
        let new_session = saved.is_none();

        let (keys, mut saved, mut ctr, reserved_end) = if let Some(s) = saved {
            let end = s.next_ctr + COUNTER_BLOCK;
            if let Some(slot) = slot {
                if !slot.write(&Saved {
                    next_ctr: end,
                    ..s.clone()
                }) {
                    return Err(Outcome::Failed(WireError::Io(
                        "cannot save the session".into(),
                    )));
                }
            }
            let keys = Keys::new(&s.send, &s.recv);
            let packet = keys.seal(
                T_REQ,
                s.sid,
                s.next_ctr,
                &request(&self.link, req_id, 0, 0, Some(body), 0).encode(),
            );
            self.send(s.port, &packet, self.link.duplicate()).await;
            let ctr = s.next_ctr + 1;
            (keys, s, ctr, end)
        } else {
            let (keys, s) = self
                .handshake(
                    body,
                    req_id,
                    &mut early,
                    &mut hs2_at,
                    &mut lost_events,
                    &mut sent_events,
                )
                .await?;
            (keys, s, 0, COUNTER_BLOCK)
        };
        let sid = saved.sid;

        let mut got: BTreeMap<u8, Vec<u8>> = BTreeMap::new();
        let mut k = None::<usize>;
        let mut rounds: HashMap<u8, Round> = HashMap::new();
        // The first burst is at least one symbol: expect it before its length is known.
        rounds.insert(
            0,
            Round {
                len: None,
                maxpos: -1,
                requested: 1,
                sent_at: Instant::now(),
                received: 0,
            },
        );
        let mut next_round = 1u8;
        let mut last_ctr = 0u32;
        let mut last: Option<(Instant, u8, i32)> = hs2_at.map(|t| (t, 0, -1));
        let mut last_full: Option<(Instant, u8, i32)> = None;
        let mut working: Option<Instant> = None;
        let mut backoff = 0.0f64;
        let mut acked = false;
        let mut rate_samples = 0;
        let mut first_wire = if self.link.duplicate() {
            2 * (body.len() + 60)
        } else {
            body.len() + 60
        };

        let response = loop {
            let item = if !early.is_empty() {
                Some(early.remove(0))
            } else {
                let until = self.timer(&rounds, last, working, first_wire, backoff);
                self.recv(until, &mut buf)
                    .await
                    .map(|n| (Instant::now(), buf[..n].to_vec()))
            };
            if let Some((t, packet)) = item {
                // A reset counts only with this session's token; anything
                // else unencrypted is ignored.
                if packet.first() == Some(&T_RESET)
                    && packet.len() == 5 + RESET_TOKEN
                    && packet[1..5] == sid.to_be_bytes()
                    && same_token(&packet[5..], &saved.reset)
                    && !new_session
                {
                    return Err(Outcome::Reset);
                }
                let Some(opened) = keys.open(&packet) else {
                    continue;
                };
                last_ctr = last_ctr.max(opened.ctr + 1);
                if opened.ptype == T_WORKING {
                    if opened.plain.get(..2) == Some(&req_id.to_be_bytes()[..]) && k.is_none() {
                        if working.is_none() {
                            sent_events += 1; // the request got through
                        }
                        for r in rounds.values_mut().filter(|r| r.len.is_none()) {
                            r.requested = 0;
                        }
                        working = Some(t);
                        backoff = 0.0;
                    }
                    continue;
                }
                if opened.ptype != T_DATA {
                    continue;
                }
                let Some(d) = Data::decode(&opened.plain) else {
                    continue;
                };
                if d.req_id != req_id {
                    continue;
                }
                working = None;
                backoff = 0.0;
                let wire = packet.len() + IP_UDP;
                // Link rate, from two full-size packets next to each other in
                // one burst: they leave back to back, so their spacing is a
                // packet's transmit time at the bottleneck (or the pacing,
                // if that is slower). Not from HS2 to the first symbol (that
                // gap includes the engine's time on VTOP), and not across a
                // short packet (it waits on the pacing of the one before).
                let full = wire as f64 >= 0.9 * DATA_WIRE;
                if let Some((t0, r0, p0)) = last_full {
                    if full && r0 == d.round && d.pos as i32 == p0 + 1 {
                        let per = t.duration_since(t0).as_secs_f64();
                        if per > 0.002 {
                            self.link.sample_rate(wire, per, rate_samples == 0);
                            rate_samples += 1;
                        }
                    }
                }
                last_full = full.then_some((t, d.round, d.pos as i32));
                last = Some((t, d.round, d.pos as i32));
                let r = rounds.entry(d.round).or_insert(Round {
                    len: None,
                    maxpos: -1,
                    requested: 0,
                    sent_at: t,
                    received: 0,
                });
                r.len = Some(d.len);
                r.maxpos = r.maxpos.max(d.pos as i32);
                r.received += 1;
                k = Some(d.k as usize);
                got.entry(d.sym).or_insert_with(|| d.bytes.to_vec());
                if !acked {
                    // Prove this address, so the gateway stops capping what it sends.
                    let ack = keys.seal(
                        T_REQ,
                        sid,
                        ctr,
                        &request(&self.link, req_id, 0, 0, None, last_ctr).encode(),
                    );
                    ctr += 1;
                    self.send(saved.port, &ack, false).await;
                    acked = true;
                }
                let kk = d.k as usize;
                if got.len() >= kk {
                    break fec::decode(d.total as usize, &got);
                }
                let (known, pending) = outstanding(&rounds);
                if got.len() + known + pending < kk {
                    // Certain shortfall even if everything in flight arrives: ask now.
                    let deficit = kk - got.len() - known - pending;
                    let want = deficit + fec::extra_for(deficit, self.link.loss());
                    let rq = request(&self.link, req_id, next_round, want as u8, None, last_ctr);
                    self.ask(
                        &keys,
                        &mut saved,
                        &mut ctr,
                        &mut rounds,
                        &mut next_round,
                        rq,
                        want,
                    )
                    .await;
                }
                continue;
            }
            if Instant::now() >= self.deadline {
                return Err(self.failure());
            }
            // Timer: what was outstanding is presumed lost.
            for r in rounds.values_mut() {
                if let Some(len) = r.len {
                    r.maxpos = len as i32 - 1;
                } else if r.requested > 0 && r.received == 0 && working.is_none() {
                    lost_events += 1;
                    sent_events += 1;
                }
                r.requested = 0;
            }
            let want = match k {
                // Nothing yet: the first burst may still be queued (or the
                // gateway still working), so probe for one symbol.
                None => 1,
                Some(kk) => {
                    let deficit = kk.saturating_sub(got.len());
                    deficit + fec::extra_for(deficit, self.link.loss())
                }
            };
            let with_body = k.is_none() && working.is_none() && !self.heard;
            let rq = request(
                &self.link,
                req_id,
                next_round,
                want as u8,
                with_body.then_some(body),
                last_ctr,
            );
            first_wire = rq.encode().len() + 25 + IP_UDP;
            if last.is_none() {
                if let Some(r0) = rounds.get_mut(&0) {
                    r0.sent_at = Instant::now();
                }
            }
            if working.is_some() {
                working = Some(Instant::now());
            }
            self.ask(
                &keys,
                &mut saved,
                &mut ctr,
                &mut rounds,
                &mut next_round,
                rq,
                want,
            )
            .await;
            backoff = (backoff * 1.5 + self.link.slack()).min(32.0);
        };

        let Some(response) = response else {
            return Err(Outcome::Failed(WireError::Io(
                "the response did not decode".into(),
            )));
        };
        let done = keys.seal(T_DONE, sid, ctr, &req_id.to_be_bytes());
        ctr += 1;
        self.send(saved.port, &done, self.link.duplicate()).await;

        // Learn the link, per packet. Downlink: positions up to the last one
        // seen in each burst (later ones may not have arrived before we
        // finished), plus the request that caused it.
        for r in rounds.values().filter(|r| r.len.is_some()) {
            let seen = (r.maxpos + 1) as usize;
            sent_events += seen + 1;
            lost_events += seen.saturating_sub(r.received);
        }
        self.link.observe(lost_events, sent_events);
        saved.next_ctr = reserved_end.max(ctr);
        Ok((response, saved, new_session))
    }

    /// Noise IK; message 1 carries a timestamp and the request. Tries the
    /// ports in turn from the one that last answered on this network.
    async fn handshake(
        &mut self,
        body: &[u8],
        req_id: u16,
        early: &mut Vec<(Instant, Vec<u8>)>,
        hs2_at: &mut Option<Instant>,
        lost_events: &mut usize,
        sent_events: &mut usize,
    ) -> Result<(Keys, Saved), Outcome> {
        let fail = |e: snow::Error| Outcome::Failed(WireError::Io(e.to_string()));
        let mut hs = snow::Builder::new(NOISE.parse().map_err(fail)?)
            .local_private_key(&self.device)
            .map_err(fail)?
            .remote_public_key(&self.server_key)
            .map_err(fail)?
            .build_initiator()
            .map_err(fail)?;
        let mut payload = now_us().to_be_bytes().to_vec();
        payload.extend_from_slice(&request(&self.link, req_id, 0, 0, Some(body), 0).encode());
        let mut msg1 = [0u8; 1200];
        let len = hs.write_message(&payload, &mut msg1).map_err(fail)?;
        let mut hs1 = vec![T_HS1];
        hs1.extend_from_slice(&msg1[..len]);
        let duplicate = self.link.duplicate();
        let copies = if duplicate { 2.0 } else { 1.0 };
        let base = 1.5
            * (self.link.rtt + (copies * (hs1.len() + IP_UDP) as f64 + 81.0) / self.link.rate)
            + self.link.slack();
        let mut wait = base;
        let mut buf = [0u8; 1500];
        for attempt in 0.. {
            if Instant::now() >= self.deadline {
                return Err(self.failure());
            }
            let port = (self.link.port + attempt) % self.ports.len();
            if attempt > 0 {
                self.retries += 1;
                *lost_events += 1;
                *sent_events += 1;
            }
            let t0 = Instant::now();
            self.send(port, &hs1, duplicate).await;
            let until = Instant::now() + Duration::from_secs_f64(wait);
            while let Some(n) = self.recv(until, &mut buf).await {
                match buf[0] {
                    T_HS2 if n > 5 && buf[1..5] == hs1[1..5] => {
                        let mut p = [0u8; 64];
                        if let Ok(len) = hs.read_message(&buf[5..n], &mut p) {
                            if len != 4 + RESET_TOKEN {
                                continue;
                            }
                            let now = Instant::now();
                            *hs2_at = Some(now);
                            // Only with a measured rate: subtracting send times
                            // at a guessed rate can drive the sample to zero,
                            // and the minimum filter would keep it.
                            if attempt == 0 && self.link.measured {
                                // If the first copy was lost HS2 answers the second, sent one
                                // transmit time later: subtract both copies' send time.
                                let s = now.duration_since(t0).as_secs_f64()
                                    - (copies * (hs1.len() + IP_UDP) as f64 + (n + IP_UDP) as f64)
                                        / self.link.rate;
                                self.link.sample_rtt(s);
                            }
                            self.link.port = port;
                            let (send, recv) = hs.dangerously_get_raw_split();
                            let sid = u32::from_be_bytes(p[..4].try_into().expect("4 bytes"));
                            return Ok((
                                Keys::new(&send, &recv),
                                Saved {
                                    sid,
                                    send,
                                    recv,
                                    next_ctr: 0,
                                    port,
                                    created_ms: now_ms(),
                                    reset: p[4..4 + RESET_TOKEN].try_into().expect("16 bytes"),
                                },
                            ));
                        }
                    }
                    T_DATA | T_WORKING => early.push((Instant::now(), buf[..n].to_vec())),
                    _ => {}
                }
            }
            wait = (wait * 1.5).min(base + 32.0);
        }
        unreachable!("the loop returns")
    }

    /// When to give up waiting and ask again.
    fn timer(
        &self,
        rounds: &HashMap<u8, Round>,
        last: Option<(Instant, u8, i32)>,
        working: Option<Instant>,
        first_wire: usize,
        backoff: f64,
    ) -> Instant {
        let link = &self.link;
        let now = Instant::now();
        if let Some(w) = working {
            // The gateway has the request: check back now and then, giving
            // a symbol that just left time to arrive.
            return w + Duration::from_secs_f64(
                3.0f64.max(2.0 * (link.rtt + link.slack())) + link.packet_time() + backoff,
            );
        }
        let secs = match last {
            None => {
                let r0 = rounds.get(&0).map_or(now, |r| r.sent_at);
                let s = 1.5 * (link.rtt + (first_wire as f64 + DATA_WIRE) / link.rate)
                    + link.slack()
                    + backoff;
                return r0 + Duration::from_secs_f64(s);
            }
            Some((t, _, _)) => {
                let (known, _) = outstanding(rounds);
                let mut end =
                    -now.duration_since(t).as_secs_f64() + link.packet_time() * known as f64;
                for r in rounds
                    .values()
                    .filter(|r| r.len.is_none() && r.requested > 0)
                {
                    let earliest =
                        -now.duration_since(r.sent_at).as_secs_f64() + link.rtt + 160.0 / link.rate;
                    end = end.max(earliest) + link.packet_time() * r.requested as f64;
                }
                end + link.slack() + backoff
            }
        };
        now + Duration::from_secs_f64(secs.max(0.01))
    }

    #[allow(clippy::too_many_arguments)]
    async fn ask(
        &mut self,
        keys: &Keys,
        saved: &mut Saved,
        ctr: &mut u32,
        rounds: &mut HashMap<u8, Round>,
        next_round: &mut u8,
        rq: Req,
        want: usize,
    ) {
        let packet = keys.seal(T_REQ, saved.sid, *ctr, &rq.encode());
        *ctr += 1;
        let duplicate = self.link.duplicate();
        self.send(saved.port, &packet, duplicate).await;
        self.retries += 1;
        rounds.insert(
            rq.round,
            Round {
                len: None,
                maxpos: -1,
                requested: want,
                sent_at: Instant::now(),
                received: 0,
            },
        );
        *next_round = next_round.wrapping_add(1).max(1);
    }
}

/// (symbols still due in bursts whose length is known, symbols requested
/// in rounds not heard from yet).
fn outstanding(rounds: &HashMap<u8, Round>) -> (usize, usize) {
    rounds
        .values()
        .fold((0, 0), |(known, pending), r| match r.len {
            Some(len) => (known + (len as i32 - 1 - r.maxpos).max(0) as usize, pending),
            None => (known, pending + r.requested),
        })
}

fn request(
    link: &Link,
    req_id: u16,
    round: u8,
    want: u8,
    body: Option<&[u8]>,
    last_ctr: u32,
) -> Req {
    Req {
        req_id,
        round,
        want,
        loss_pct: link.loss_pct(),
        rate: if link.measured {
            link.rate.min(65535.0) as u16
        } else {
            0
        },
        rtt_ms: (link.rtt * 1000.0) as u16,
        last_ctr,
        body: body.map(<[u8]>::to_vec).unwrap_or_default(),
    }
}

fn device_key(dir: &Path) -> Result<[u8; 32], WireError> {
    let path = dir.join("device.key");
    if let Some(key) = fs::read_to_string(&path)
        .ok()
        .and_then(|t| unhex(&t))
        .and_then(|k| k.try_into().ok())
    {
        return Ok(key);
    }
    let (private, _) = crate::generate_keypair();
    write_atomic(&path, hex(&private).as_bytes()).map_err(io)?;
    Ok(private)
}

/// The gateway's address: cached for a week, refreshed by DNS after that
/// (and still used if the refresh fails).
async fn resolve(cfg: &ClientConfig, deadline: Instant) -> Result<IpAddr, WireError> {
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
    if let Some((ip, at)) = cached {
        if now_ms().saturating_sub(at) < DNS_KEEP_MS {
            return Ok(ip);
        }
    }
    let lookup = tokio::net::lookup_host((cfg.host.as_str(), cfg.ports[0]));
    let found = tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), lookup)
        .await
        .ok()
        .and_then(Result::ok)
        .and_then(|mut addrs| {
            let all: Vec<SocketAddr> = addrs.by_ref().collect();
            all.iter()
                .find(|a| a.is_ipv4())
                .or(all.first())
                .map(SocketAddr::ip)
        });
    match (found, cached) {
        (Some(ip), _) => {
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
fn network_key(ip: IpAddr, port: u16) -> String {
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

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    fs::write(&tmp, bytes)?;
    fs::rename(tmp, path)
}

fn io(e: std::io::Error) -> WireError {
    WireError::Io(e.to_string())
}

//! The gateway side, without I/O: feed it packets ([`Gateway::receive`]) and
//! answers ([`Gateway::respond`]), and send what [`Gateway::poll`] returns.
//! `vtop-gateway` owns the sockets and turns requests into engine calls.
//!
//! Protections, all from the measurements and RFC 9000 / WireGuard:
//! - Message 1 must be fresh (±[`FRESH_MS`]) and its timestamp new for the
//!   device; a repeat with the same ephemeral key gets the cached HS2 (after
//!   [`CLIENT_REPEAT`]), and never runs the request again. (Not "newer than
//!   the last", as WireGuard has it: the app and a background isolate may
//!   handshake at once, and their messages may arrive in either order. A
//!   replay repeats its timestamp; a new timestamp needs the device's key.)
//! - Until an address proves it receives (the client echoes our counter),
//!   it gets at most 3x the bytes it sent. A new port on an IP the device
//!   was seen at counts as proven: that is a NAT rebinding.
//! - Sends are paced at 2x the client's measured rate (so a faster link
//!   shows itself in the next measurement), and stop when the
//!   client has been silent for [`BREAKER`].

use std::collections::{HashMap, HashSet, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use crate::fec::{self, DATA_WIRE};
use crate::frame::{
    header, reset_token, Keys, ReplayWindow, IP_UDP, T_DATA, T_DONE, T_HS1, T_HS2, T_REQ, T_RESET,
    T_WORKING,
};
use crate::link::RATE_FLOOR;
use crate::msg::{now_us, Data, Req, LOSS_UNKNOWN};
use crate::NOISE;

pub const FRESH_MS: u64 = 120_000;
/// The client's own duplicate of a packet arrives within this; a later copy
/// is a real retry.
pub const CLIENT_REPEAT: Duration = Duration::from_millis(1500);
pub const BREAKER: Duration = Duration::from_secs(30);
/// Idle sessions are forgotten after this; the client re-handshakes daily.
pub const SESSION_IDLE: Duration = Duration::from_secs(48 * 3600);
/// An answer still being made is waited for this long: over a server
/// sign-in that waits for the OTP mail.
pub const PENDING_KEEP: Duration = Duration::from_secs(150);
/// A finished response is kept this long for late requests for more.
pub const RESPONSE_KEEP: Duration = Duration::from_secs(120);
/// Tell the client "working" if the answer is not ready by half its wait
/// for a first symbol, within these bounds: the notice costs half a second
/// of airtime at 1 kbit/s, so it goes only when the wait is real.
pub const WORKING_MIN: Duration = Duration::from_millis(300);
pub const WORKING_MAX: Duration = Duration::from_secs(2);
/// Pacing before the client reports a measured rate (bytes/s).
pub const PACE_UNKNOWN: u16 = 250;
const MAX_PENDING: usize = 4;
/// Sessions kept at most, so memory stays bounded (device keys cost
/// nothing); when full, the one heard from longest ago makes room.
const MAX_SESSIONS: usize = 50_000;

/// Which request an answer is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    pub sid: u32,
    pub req_id: u16,
}

/// A request for the application to answer with [`Gateway::respond`].
#[derive(Debug)]
pub struct NewRequest {
    pub token: Token,
    /// The client's static key: stable per install.
    pub device: [u8; 32],
    pub body: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub struct Outgoing {
    /// Which of the gateway's sockets to send from (the one the client used).
    pub socket: usize,
    pub to: SocketAddr,
    pub packet: Vec<u8>,
}

enum Resp {
    Pending {
        since: Instant,
        hints: Req,
        rounds: HashSet<u8>,
    },
    Ready {
        at: Instant,
        k: usize,
        total: usize,
        data: Vec<Vec<u8>>,
        repair: Vec<Vec<u8>>,
        next_repair: usize,
        rounds: HashSet<u8>,
    },
}

struct Session {
    keys: Keys,
    window: ReplayWindow,
    ctr: u32,
    peer: SocketAddr,
    socket: usize,
    device: [u8; 32],
    validated: bool,
    credit: usize,
    heard: Instant,
    next_send: Instant,
    /// The client's latest measured rate, bytes/s (0: none yet).
    rate: u16,
    responses: HashMap<u16, Resp>,
}

struct Out {
    due: Instant,
    sid: u32,
    req_id: u16,
    packet: Vec<u8>,
}

pub struct Gateway {
    private: [u8; 32],
    sessions: HashMap<u32, Session>,
    /// Message 1 by the client's ephemeral key: when, which session, HS2.
    handshakes: HashMap<[u8; 32], (Instant, u32, Vec<u8>)>,
    /// Message 1 timestamps (µs) seen per device within the freshness window.
    seen_ts: HashMap<[u8; 32], Vec<u64>>,
    known_ip: HashMap<[u8; 32], IpAddr>,
    queue: VecDeque<Out>,
    /// Handshake replies and resets: not paced, not tied to a session.
    direct: VecDeque<Outgoing>,
    /// "Working" notices due: (when, token).
    working: Vec<(Instant, Token)>,
}

impl Gateway {
    pub fn new(private: [u8; 32]) -> Self {
        Self {
            private,
            sessions: HashMap::new(),
            handshakes: HashMap::new(),
            seen_ts: HashMap::new(),
            known_ip: HashMap::new(),
            queue: VecDeque::new(),
            direct: VecDeque::new(),
            working: Vec::new(),
        }
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Handles one datagram; returns a request when it starts a new one.
    pub fn receive(
        &mut self,
        packet: &[u8],
        socket: usize,
        from: SocketAddr,
        now: Instant,
    ) -> Option<NewRequest> {
        log::debug!(
            "{} bytes from {from} (type {:#04x})",
            packet.len(),
            packet.first()?
        );
        match packet.first()? {
            &T_HS1 => self.handshake(packet, socket, from, now),
            &T_REQ | &T_DONE => self.framed(packet, socket, from, now),
            _ => None,
        }
    }

    /// The answer to `token`. A body over [`fec::MAX_RESPONSE`] is the
    /// caller's bug: it is refused (the client times out and falls back).
    pub fn respond(&mut self, token: Token, body: Vec<u8>, now: Instant) {
        if body.len() > fec::MAX_RESPONSE {
            log::error!("response of {} bytes is over the limit", body.len());
            return;
        }
        let Some(sess) = self.sessions.get_mut(&token.sid) else {
            return;
        };
        let Some(Resp::Pending { hints, rounds, .. }) = sess.responses.remove(&token.req_id) else {
            return;
        };
        let (k, _) = fec::shape(body.len());
        let (data, repair) = fec::encode(&body);
        sess.responses.insert(
            token.req_id,
            Resp::Ready {
                at: now,
                k,
                total: body.len(),
                data,
                repair,
                next_repair: 0,
                rounds,
            },
        );
        self.working.retain(|(_, t)| *t != token);
        self.burst(token.sid, &hints, true, now);
    }

    /// Datagrams due now.
    pub fn poll(&mut self, now: Instant) -> Vec<Outgoing> {
        let mut out: Vec<Outgoing> = self.direct.drain(..).collect();
        let due: Vec<Token> = self
            .working
            .iter()
            .filter(|(t, _)| *t <= now)
            .map(|(_, tok)| *tok)
            .collect();
        self.working.retain(|(t, _)| *t > now);
        for tok in due {
            self.send_working(tok, now);
        }
        let mut keep = VecDeque::with_capacity(self.queue.len());
        while let Some(o) = self.queue.pop_front() {
            let Some(sess) = self.sessions.get_mut(&o.sid) else {
                continue;
            };
            if now.saturating_duration_since(sess.heard) > BREAKER {
                continue; // circuit breaker: the client is gone
            }
            if o.due > now {
                keep.push_back(o);
                continue;
            }
            let wire = o.packet.len() + IP_UDP;
            if !sess.validated {
                if sess.credit < wire {
                    keep.push_back(o); // until the client sends more or proves its address
                    continue;
                }
                sess.credit -= wire;
            }
            out.push(Outgoing {
                socket: sess.socket,
                to: sess.peer,
                packet: o.packet,
            });
        }
        self.queue = keep;
        out
    }

    /// When [`Gateway::poll`] next has something to send.
    pub fn next_due(&self) -> Option<Instant> {
        let queued = self
            .queue
            .iter()
            .filter(|o| {
                self.sessions
                    .get(&o.sid)
                    .is_some_and(|s| s.validated || s.credit >= o.packet.len() + IP_UDP)
            })
            .map(|o| o.due);
        let working = self.working.iter().map(|(t, _)| *t);
        let direct = (!self.direct.is_empty()).then(Instant::now);
        queued.chain(working).chain(direct).min()
    }

    /// Forgets idle sessions and old responses.
    pub fn sweep(&mut self, now: Instant) {
        self.sessions
            .retain(|_, s| now.saturating_duration_since(s.heard) < SESSION_IDLE);
        for s in self.sessions.values_mut() {
            s.responses.retain(|_, r| match r {
                Resp::Pending { since, .. } => now.saturating_duration_since(*since) < PENDING_KEEP,
                Resp::Ready { at, .. } => now.saturating_duration_since(*at) < RESPONSE_KEEP,
            });
        }
        self.handshakes.retain(|_, (at, _, _)| {
            now.saturating_duration_since(*at) < Duration::from_millis(FRESH_MS * 2)
        });
        let live: HashSet<[u8; 32]> = self.sessions.values().map(|s| s.device).collect();
        self.known_ip.retain(|d, _| live.contains(d));
        let oldest = now_us().saturating_sub(FRESH_MS * 2000);
        for list in self.seen_ts.values_mut() {
            list.retain(|&ts| ts >= oldest);
        }
        self.seen_ts
            .retain(|d, list| live.contains(d) || !list.is_empty());
    }

    fn handshake(
        &mut self,
        packet: &[u8],
        socket: usize,
        from: SocketAddr,
        now: Instant,
    ) -> Option<NewRequest> {
        let msg1 = &packet[1..];
        let ephemeral: [u8; 32] = msg1.get(..32)?.try_into().ok()?;
        if let Some((at, sid, hs2)) = self.handshakes.get(&ephemeral).cloned() {
            if let Some(sess) = self.sessions.get_mut(&sid) {
                sess.credit += 3 * (packet.len() + IP_UDP);
            }
            if now.saturating_duration_since(at) > CLIENT_REPEAT {
                self.direct.push_back(Outgoing {
                    socket,
                    to: from,
                    packet: hs2,
                });
            }
            return None;
        }
        let mut hs = snow::Builder::new(NOISE.parse().ok()?)
            .local_private_key(&self.private)
            .ok()?
            .build_responder()
            .ok()?;
        let mut payload = [0u8; 1500];
        let len = match hs.read_message(msg1, &mut payload) {
            Ok(len) => len,
            Err(e) => {
                log::debug!("message 1 from {from} does not decrypt: {e}");
                return None;
            }
        };
        let ts = u64::from_be_bytes(payload.get(..8)?.try_into().ok()?);
        let req = Req::decode(&payload[8..len])?;
        let device: [u8; 32] = hs.get_remote_static()?.try_into().ok()?;
        let seen = self.seen_ts.entry(device).or_default();
        if now_us().abs_diff(ts) > FRESH_MS * 1000 || seen.contains(&ts) {
            log::debug!("message 1 from {from}: stale or replayed timestamp");
            return None;
        }
        if seen.len() >= 64 {
            seen.remove(0);
        }
        seen.push(ts);
        let sid = crate::rand::u32();
        let mut msg2 = [0u8; 128];
        let mut hs2_payload = sid.to_be_bytes().to_vec();
        hs2_payload.extend_from_slice(&reset_token(&self.private, sid));
        let m2 = hs.write_message(&hs2_payload, &mut msg2).ok()?;
        let (k_ir, k_ri) = hs.dangerously_get_raw_split();
        let mut hs2 = vec![T_HS2];
        hs2.extend_from_slice(&ephemeral[..4]);
        hs2.extend_from_slice(&msg2[..m2]);
        // Losing HS2 stalls everything, so it goes twice on lossy links.
        let lossy = req.loss_pct != LOSS_UNKNOWN && req.loss_pct >= 10;
        let copies = if lossy { 2 } else { 1 };
        for _ in 0..copies {
            self.direct.push_back(Outgoing {
                socket,
                to: from,
                packet: hs2.clone(),
            });
        }
        let hs2_wire = copies * (hs2.len() + IP_UDP);
        let session = Session {
            keys: Keys::new(&k_ri, &k_ir),
            window: ReplayWindow::default(),
            ctr: 0,
            peer: from,
            socket,
            device,
            validated: self.known_ip.get(&device) == Some(&from.ip()),
            credit: (3 * (packet.len() + IP_UDP)).saturating_sub(hs2_wire),
            heard: now,
            next_send: now + pace(hs2_wire, req.rate),
            rate: req.rate,
            responses: HashMap::new(),
        };
        if self.sessions.len() >= MAX_SESSIONS {
            // Full: the session heard from longest ago makes room.
            if let Some(oldest) = self
                .sessions
                .iter()
                .min_by_key(|(_, s)| s.heard)
                .map(|(id, _)| *id)
            {
                self.sessions.remove(&oldest);
            }
        }
        self.handshakes.insert(ephemeral, (now, sid, hs2));
        self.sessions.insert(sid, session);
        self.request(sid, req, now)
    }

    fn framed(
        &mut self,
        packet: &[u8],
        socket: usize,
        from: SocketAddr,
        now: Instant,
    ) -> Option<NewRequest> {
        let (_, sid, ctr) = header(packet)?;
        let Some(sess) = self.sessions.get_mut(&sid) else {
            let mut reset = vec![T_RESET];
            reset.extend_from_slice(&sid.to_be_bytes());
            reset.extend_from_slice(&reset_token(&self.private, sid));
            // Only when smaller than the packet that asked: no amplification.
            if reset.len() < packet.len() {
                self.direct.push_back(Outgoing {
                    socket,
                    to: from,
                    packet: reset,
                });
            }
            return None;
        };
        let opened = sess.keys.open(packet)?;
        if !sess.window.accept(ctr) {
            return None; // a replay, or the client's deliberate duplicate
        }
        if from != sess.peer {
            sess.peer = from;
            sess.validated = self.known_ip.get(&sess.device) == Some(&from.ip());
        }
        sess.socket = socket;
        sess.heard = now;
        sess.credit += 3 * (packet.len() + IP_UDP);
        if opened.ptype == T_DONE {
            let id = u16::from_be_bytes(opened.plain.get(..2)?.try_into().ok()?);
            self.queue.retain(|o| !(o.sid == sid && o.req_id == id));
            sess.next_send = now;
            sess.responses.remove(&id);
            return None;
        }
        let req = Req::decode(&opened.plain)?;
        if req.last_ctr > 0 && !sess.validated {
            sess.validated = true;
            self.known_ip.insert(sess.device, from.ip());
        }
        if req.rate > 0 && sess.rate != req.rate {
            // A measured rate: re-space what is still queued for this session.
            sess.rate = req.rate;
            let mut due = now;
            for o in self.queue.iter_mut().filter(|o| o.sid == sid) {
                o.due = due;
                due += pace(o.packet.len() + IP_UDP, req.rate);
            }
            sess.next_send = due;
        }
        self.request(sid, req, now)
    }

    fn request(&mut self, sid: u32, req: Req, now: Instant) -> Option<NewRequest> {
        let sess = self.sessions.get_mut(&sid)?;
        let token = Token {
            sid,
            req_id: req.req_id,
        };
        if let Some(resp) = sess.responses.get_mut(&req.req_id) {
            if req.want == 0 {
                return None; // a pure ack
            }
            let rounds = match resp {
                Resp::Pending { rounds, .. } | Resp::Ready { rounds, .. } => rounds,
            };
            if !rounds.insert(req.round) {
                return None;
            }
            if matches!(resp, Resp::Pending { .. }) {
                self.send_working(token, now);
                return None;
            }
            self.burst(sid, &req, false, now);
            return None;
        }
        if req.body.is_empty() {
            return None; // a follow-up for a request this gateway never saw
        }
        let device = sess.device;
        let pending = sess
            .responses
            .values()
            .filter(|r| matches!(r, Resp::Pending { .. }))
            .count();
        if pending >= MAX_PENDING {
            return None;
        }
        // A new request from this device: whatever is still queued for its
        // older requests (in any session) is for a fetch it finished or gave up on.
        let sessions = &self.sessions;
        self.queue
            .retain(|o| sessions.get(&o.sid).is_none_or(|s| s.device != device));
        let sess = self.sessions.get_mut(&sid)?;
        let body = req.body.clone();
        let rate = if req.rate == 0 {
            RATE_FLOOR
        } else {
            req.rate as f64
        };
        let wait = 0.5 * (req.rtt_ms as f64 / 1000.0 + DATA_WIRE / rate);
        let working_at = now + Duration::from_secs_f64(wait).clamp(WORKING_MIN, WORKING_MAX);
        sess.responses.insert(
            req.req_id,
            Resp::Pending {
                since: now,
                rounds: HashSet::from([req.round]),
                hints: req,
            },
        );
        self.working.push((working_at, token));
        Some(NewRequest {
            token,
            device,
            body,
        })
    }

    /// Queues symbols: on the first burst all data plus proactive repair,
    /// later `want` fresh repair symbols.
    fn burst(&mut self, sid: u32, req: &Req, first: bool, now: Instant) {
        let Some(sess) = self.sessions.get_mut(&sid) else {
            return;
        };
        let Some(Resp::Ready {
            k,
            total,
            data,
            repair,
            next_repair,
            ..
        }) = sess.responses.get_mut(&req.req_id)
        else {
            return;
        };
        let mut syms: Vec<(u8, Vec<u8>)> = Vec::new();
        if first {
            let rate = if req.rate == 0 {
                RATE_FLOOR
            } else {
                req.rate as f64
            };
            let p = if req.loss_pct == LOSS_UNKNOWN {
                0.1
            } else {
                req.loss_pct as f64 / 100.0
            };
            let spare = fec::choose_repair(
                *k,
                p,
                DATA_WIRE / rate,
                160.0 / rate,
                req.rtt_ms as f64 / 1000.0,
            );
            syms.extend(data.iter().enumerate().map(|(i, d)| (i as u8, d.clone())));
            for _ in 0..spare {
                syms.push(next(*k, repair, next_repair));
            }
        } else {
            for _ in 0..req.want {
                syms.push(next(*k, repair, next_repair));
            }
        }
        let (k, total) = (*k as u8, *total as u16);
        let len = syms.len() as u8;
        let rate = sess.rate.max(req.rate);
        let mut due = sess.next_send.max(now);
        for (pos, (sym, bytes)) in syms.into_iter().enumerate() {
            let plain = Data {
                req_id: req.req_id,
                sym,
                k,
                total,
                round: req.round,
                pos: pos as u8,
                len,
                bytes: &bytes,
            }
            .encode();
            let packet = sess.keys.seal(T_DATA, sid, sess.ctr, &plain);
            sess.ctr += 1;
            let wire = packet.len() + IP_UDP;
            self.queue.push_back(Out {
                due,
                sid,
                req_id: req.req_id,
                packet,
            });
            due += pace(wire, rate);
        }
        sess.next_send = due;
    }

    fn send_working(&mut self, token: Token, now: Instant) {
        let Some(sess) = self.sessions.get_mut(&token.sid) else {
            return;
        };
        if !matches!(
            sess.responses.get(&token.req_id),
            Some(Resp::Pending { .. })
        ) {
            return;
        }
        let packet = sess
            .keys
            .seal(T_WORKING, token.sid, sess.ctr, &token.req_id.to_be_bytes());
        sess.ctr += 1;
        let due = sess.next_send.max(now);
        sess.next_send = due + pace(packet.len() + IP_UDP, sess.rate);
        self.queue.push_back(Out {
            due,
            sid: token.sid,
            req_id: token.req_id,
            packet,
        });
    }
}

fn next(k: usize, repair: &[Vec<u8>], next_repair: &mut usize) -> (u8, Vec<u8>) {
    let i = *next_repair % repair.len();
    *next_repair += 1;
    ((k + i) as u8, repair[i].clone())
}

/// Spacing at 2x the client's rate (4 kbit/s before it is known). Faster
/// than the link only queues at the bottleneck, which then sets the spacing
/// the client measures; slower would cap every measurement at the pace.
fn pace(wire: usize, rate: u16) -> Duration {
    let rate = if rate == 0 { PACE_UNKNOWN } else { rate };
    Duration::from_secs_f64(wire as f64 / (2.0 * rate as f64))
}

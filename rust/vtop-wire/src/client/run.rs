//! One fetch's run: the handshake, the request and the transfer of the response's
//! symbols, timed from what this network's [`Link`] has learned.

use std::collections::{BTreeMap, HashMap};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use super::state::{Saved, Slot};
use super::{COUNTER_BLOCK, Net, Socket, WireError};
use crate::fec::{self, DATA_WIRE};
use crate::frame::{
    IP_UDP, Keys, RESET_TOKEN, T_DATA, T_DONE, T_HS1, T_HS2, T_REQ, T_RESET, T_WORKING,
    T_WRONG_KEY, same_token,
};
use crate::link::Link;
use crate::msg::{Data, Req, now_ms, now_us};
use crate::noise::{Initiator, random_key};

/// Gap between a control packet and its duplicate.
const DUPLICATE_GAP: Duration = Duration::from_millis(30);

pub(super) enum Outcome {
    Reset,
    Failed(WireError),
}

struct Round {
    len: Option<u8>,
    maxpos: i32,
    requested: usize,
    sent_at: Instant,
    received: usize,
}

pub(super) struct Run<'a, N: Net> {
    pub(super) net: &'a N,
    pub(super) sock: N::Socket,
    pub(super) ip: IpAddr,
    pub(super) ports: &'a [u16],
    pub(super) server_key: [u8; 32],
    pub(super) device: [u8; 32],
    pub(super) link: Link,
    pub(super) deadline: Instant,
    /// When to give up while the gateway hasn't answered at all.
    pub(super) silent_until: Instant,
    pub(super) up: usize,
    pub(super) down: usize,
    pub(super) retries: usize,
    /// Whether the gateway answered anything at all.
    pub(super) heard: bool,
}

impl<N: Net> Run<'_, N> {
    async fn send(&mut self, port: usize, packet: &[u8], duplicate: bool) {
        let Some(&to) = port
            .checked_rem(self.ports.len())
            .and_then(|i| self.ports.get(i))
        else {
            return;
        };
        let to = SocketAddr::new(self.ip, to);
        for copy in 0..if duplicate { 2 } else { 1 } {
            if copy > 0 {
                self.net.sleep(DUPLICATE_GAP).await;
            }
            if self.sock.send_to(packet, to).await.is_ok() {
                self.up += packet.len() + IP_UDP;
            }
        }
    }

    async fn recv(&mut self, until: Instant, buf: &mut [u8]) -> Option<usize> {
        let until = until.min(self.give_up_at());
        loop {
            let (n, from) = self.sock.recv_from(buf, until).await?;
            if from.ip() == self.ip {
                self.down += n + IP_UDP;
                self.heard = true;
                return Some(n);
            }
        }
    }

    /// The deadline, or sooner while nothing came back.
    fn give_up_at(&self) -> Instant {
        if self.heard {
            self.deadline
        } else {
            self.deadline.min(self.silent_until)
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
    pub(super) async fn exchange(
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
            if let Some(slot) = slot
                && !slot.write(&Saved {
                    next_ctr: end,
                    ..s.clone()
                })
            {
                return Err(Outcome::Failed(WireError::Io(
                    "cannot save the session".into(),
                )));
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
                    .and_then(|n| Some((Instant::now(), buf.get(..n)?.to_vec())))
            };
            if let Some((t, packet)) = item {
                // A reset counts only with this session's token; anything
                // else unencrypted is ignored.
                if packet.first() == Some(&T_RESET)
                    && packet.len() == 5 + RESET_TOKEN
                    && packet.get(1..5) == Some(&sid.to_be_bytes()[..])
                    && same_token(packet.get(5..).unwrap_or_default(), &saved.reset)
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
                if let Some((t0, r0, p0)) = last_full
                    && full
                    && r0 == d.round
                    && d.pos as i32 == p0 + 1
                {
                    let per = t.duration_since(t0).as_secs_f64();
                    if per > 0.002 {
                        self.link.sample_rate(wire, per, rate_samples == 0);
                        rate_samples += 1;
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
            if Instant::now() >= self.give_up_at() {
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
            if last.is_none()
                && let Some(r0) = rounds.get_mut(&0)
            {
                r0.sent_at = Instant::now();
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

    /// [`Self::exchange`] on a new session, the request in message 1 or,
    /// unless `in_handshake`, after the handshake.
    pub(super) async fn fresh(
        &mut self,
        body: &[u8],
        in_handshake: bool,
        slot: Option<&mut Slot>,
    ) -> Result<(Vec<u8>, Saved, bool), Outcome> {
        if in_handshake {
            return self.exchange(body, None, slot).await;
        }
        // An empty request: the gateway sets up the session and runs nothing.
        let (mut early, mut hs2_at, mut lost, mut sent) = (Vec::new(), None, 0, 0);
        let (_, saved) = self
            .handshake(
                &[],
                crate::rand::u16(),
                &mut early,
                &mut hs2_at,
                &mut lost,
                &mut sent,
            )
            .await?;
        let (response, saved, _) = self.exchange(body, Some(saved), slot).await?;
        Ok((response, saved, true))
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
        let fail = |what: &str| Outcome::Failed(WireError::Io(format!("noise: {what}")));
        let mut payload = now_us().to_be_bytes().to_vec();
        payload.extend_from_slice(&request(&self.link, req_id, 0, 0, Some(body), 0).encode());
        let ephemeral = random_key().ok_or_else(|| fail("no random source"))?;
        let (hs, msg1) = Initiator::start(&self.device, &self.server_key, ephemeral, &payload)
            .ok_or_else(|| fail("message 1"))?;
        // snow wrote message 1 into a 1200-byte buffer; keep its limit.
        if msg1.len() > 1200 {
            return Err(fail("message 1 too long"));
        }
        let mut hs1 = vec![T_HS1];
        hs1.extend_from_slice(&msg1);
        let duplicate = self.link.duplicate();
        let copies = if duplicate { 2.0 } else { 1.0 };
        let base = 1.5
            * (self.link.rtt + (copies * (hs1.len() + IP_UDP) as f64 + 81.0) / self.link.rate)
            + self.link.slack();
        let mut wait = base;
        let mut buf = [0u8; 1500];
        for attempt in 0.. {
            if Instant::now() >= self.give_up_at() {
                return Err(self.failure());
            }
            let port = (self.link.port + attempt) % self.ports.len().max(1);
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
                    T_HS2 if n > 5 && buf.get(1..5) == hs1.get(1..5) => {
                        let Some((p, (send, recv))) = buf.get(5..n).and_then(|m| hs.finish(m))
                        else {
                            continue;
                        };
                        let Some((sid, reset)) = p.split_first_chunk::<4>().and_then(|(sid, r)| {
                            Some((
                                u32::from_be_bytes(*sid),
                                <[u8; RESET_TOKEN]>::try_from(r).ok()?,
                            ))
                        }) else {
                            continue;
                        };
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
                        return Ok((
                            Keys::new(&send, &recv),
                            Saved {
                                sid,
                                send,
                                recv,
                                next_ctr: 0,
                                port,
                                created_ms: now_ms(),
                                reset,
                                gateway: self.server_key,
                            },
                        ));
                    }
                    T_WRONG_KEY if n == 5 && buf.get(1..5) == hs1.get(1..5) => {
                        return Err(Outcome::Failed(WireError::WrongKey));
                    }
                    T_DATA | T_WORKING => {
                        early.extend(buf.get(..n).map(|p| (Instant::now(), p.to_vec())))
                    }
                    _ => {}
                }
            }
            wait = (wait * 1.5).min(base + 32.0);
        }
        // `0..` never ends; the deadline above returns.
        Err(self.failure())
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

    #[expect(
        clippy::too_many_arguments,
        reason = "one round of the transfer's state"
    )]
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

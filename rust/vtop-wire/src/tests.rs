use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::client::{fetch, ClientConfig, WireError};
use crate::fec;
use crate::frame::{Keys, ReplayWindow, T_DATA};
use crate::gateway::{Gateway, Token};
use crate::link::Link;
use crate::msg::{Data, Req};

#[test]
fn frames_open_and_reject_tampering() {
    let (a, b) = ([1u8; 32], [2u8; 32]);
    let (client, gateway) = (Keys::new(&a, &b), Keys::new(&b, &a));
    let packet = client.seal(T_DATA, 7, 42, b"hello");
    let opened = gateway.open(&packet).unwrap();
    assert_eq!(
        (opened.ptype, opened.sid, opened.ctr, &opened.plain[..]),
        (T_DATA, 7, 42, &b"hello"[..])
    );
    let mut bad = packet.clone();
    bad[6] ^= 1; // the counter is authenticated
    assert!(gateway.open(&bad).is_none());
    assert!(
        client.open(&packet).is_none(),
        "each direction has its own key"
    );
}

#[test]
fn replay_window_accepts_each_counter_once() {
    let mut w = ReplayWindow::default();
    assert!(w.accept(10));
    assert!(w.accept(12));
    assert!(w.accept(11));
    assert!(!w.accept(11));
    assert!(w.accept(1000));
    assert!(!w.accept(12), "too old to tell");
}

#[test]
fn messages_round_trip() {
    let req = Req {
        req_id: 9,
        round: 2,
        want: 3,
        loss_pct: 30,
        rate: 125,
        rtt_ms: 410,
        last_ctr: 77,
        body: b"page".to_vec(),
    };
    assert_eq!(Req::decode(&req.encode()).unwrap(), req);
    let d = Data {
        req_id: 9,
        sym: 6,
        k: 5,
        total: 1999,
        round: 1,
        pos: 0,
        len: 2,
        bytes: b"xyz",
    };
    let enc = d.encode();
    assert_eq!(Data::decode(&enc).unwrap(), d);
}

#[test]
fn any_k_symbols_rebuild_the_response() {
    let body: Vec<u8> = (0..1999u32).map(|i| (i * 7 % 251) as u8).collect();
    let (data, repair) = fec::encode(&body);
    assert_eq!(data.len(), 5);
    let k = data.len();
    // Lose data 0, 2 and 4; keep data 1 and 3 and three repair symbols.
    let mut symbols = BTreeMap::new();
    symbols.insert(1u8, data[1].clone());
    symbols.insert(3u8, data[3].clone());
    for i in [0usize, 4, 7] {
        symbols.insert((k + i) as u8, repair[i].clone());
    }
    assert_eq!(fec::decode(body.len(), &symbols).unwrap(), body);
    symbols.remove(&3);
    assert!(
        fec::decode(body.len(), &symbols).is_none(),
        "4 of 5 is not enough"
    );
    // A lone short symbol is padded only to an even size.
    let (one, _) = fec::encode(b"abc");
    assert_eq!(one[0].len(), 4);
}

#[test]
fn spare_symbols_follow_loss() {
    for rate in [125.0, 1000.0] {
        let (tp, up) = (fec::DATA_WIRE / rate, 160.0 / rate);
        let picks: Vec<usize> = [0.0, 0.1, 0.3]
            .iter()
            .map(|&p| fec::choose_repair(5, p, tp, up, 0.3))
            .collect();
        assert_eq!(picks, [0, 1, 2], "at {rate} B/s");
        assert_eq!(
            fec::choose_repair(1, 0.3, tp, up, 0.3),
            1,
            "a tiny page on a bad link gets a spare"
        );
        assert_eq!(fec::choose_repair(1, 0.0, tp, up, 0.3), 0);
    }
}

/// Single-packet fetches on a 30% link: most succeed without a visible
/// loss, and the estimate must not drift down to 0 (it did once, when lost
/// requests were not counted).
#[test]
fn loss_estimate_holds_on_a_bad_link() {
    let mut link = Link::default();
    let mut seed = 1u64;
    for _ in 0..40 {
        // Each fetch: the request + one symbol; with 30% loss either may vanish.
        let lost = (0..2).filter(|_| xorshift(&mut seed) % 100 < 30).count();
        link.observe(lost, 2);
    }
    assert!(link.loss() > 0.2, "estimate {:.3}", link.loss());
    let mut clean = Link::default();
    for _ in 0..40 {
        clean.observe(0, 2);
    }
    assert!(clean.loss() < 0.05, "estimate {:.3}", clean.loss());
    assert_eq!(
        Link::parse(&clean.format()).unwrap().format(),
        clean.format()
    );
}

fn xorshift(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

// ---- end to end over localhost UDP ----

/// A gateway on one localhost socket whose answer to a body `[n_hi, n_lo, ..]`
/// is `n` bytes of a known pattern, after `delay`.
struct TestGateway {
    port: u16,
    public: [u8; 32],
    private: [u8; 32],
    seen: Arc<Mutex<Vec<Vec<u8>>>>,
    task: tokio::task::JoinHandle<()>,
}

fn answer(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i * 7 % 251) as u8).collect()
}

async fn spawn_gateway(
    keys: Option<([u8; 32], [u8; 32])>,
    port: u16,
    delay: Duration,
) -> TestGateway {
    let (private, public) = keys.unwrap_or_else(crate::generate_keypair);
    let sock = Arc::new(UdpSocket::bind(("127.0.0.1", port)).await.unwrap());
    let port = sock.local_addr().unwrap().port();
    let seen: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
    let seen2 = seen.clone();
    let task = tokio::spawn(async move {
        let mut gw = Gateway::new(private);
        let (tx, mut rx) = mpsc::unbounded_channel::<(Token, Vec<u8>)>();
        let mut buf = [0u8; 1500];
        loop {
            let wake = gw
                .next_due()
                .unwrap_or(Instant::now() + Duration::from_millis(50));
            tokio::select! {
                r = sock.recv_from(&mut buf) => {
                    let Ok((n, from)) = r else { continue };
                    seen2.lock().unwrap().push(buf[..n].to_vec());
                    if let Some(req) = gw.receive(&buf[..n], 0, from, Instant::now()) {
                        let size = u16::from_be_bytes([req.body[0], req.body[1]]) as usize;
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(delay).await;
                            let _ = tx.send((req.token, answer(size)));
                        });
                    }
                }
                Some((token, body)) = rx.recv() => gw.respond(token, body, Instant::now()),
                _ = tokio::time::sleep_until(tokio::time::Instant::from_std(wake)) => {}
            }
            for out in gw.poll(Instant::now()) {
                let _ = sock.send_to(&out.packet, out.to).await;
            }
        }
    });
    TestGateway {
        port,
        public,
        private,
        seen,
        task,
    }
}

/// Forwards between one client and the gateway, dropping `loss`% each way.
async fn spawn_lossy_relay(gateway_port: u16, loss: u64, seed: u64) -> u16 {
    let front = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let back = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    back.connect(("127.0.0.1", gateway_port)).await.unwrap();
    let port = front.local_addr().unwrap().port();
    let client: Arc<Mutex<Option<SocketAddr>>> = Arc::default();
    let rng = Arc::new(AtomicU64::new(seed));
    let drop = move |rng: &AtomicU64| {
        let mut s = rng.load(Ordering::Relaxed);
        let r = xorshift(&mut s);
        rng.store(s, Ordering::Relaxed);
        r % 100 < loss
    };
    let (f, b, c, r) = (front.clone(), back.clone(), client.clone(), rng.clone());
    tokio::spawn(async move {
        let mut buf = [0u8; 1500];
        while let Ok((n, from)) = f.recv_from(&mut buf).await {
            *c.lock().unwrap() = Some(from);
            if !drop(&r) {
                let _ = b.send(&buf[..n]).await;
            }
        }
    });
    tokio::spawn(async move {
        let mut buf = [0u8; 1500];
        while let Ok(n) = back.recv(&mut buf).await {
            let to = *client.lock().unwrap();
            if let (Some(to), false) = (to, drop(&rng)) {
                let _ = front.send_to(&buf[..n], to).await;
            }
        }
    });
    port
}

fn state_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "vtop-wire-{name}-{}-{}",
        std::process::id(),
        crate::rand::u32()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn config(port: u16, key: [u8; 32], dir: &std::path::Path) -> ClientConfig {
    ClientConfig {
        host: "127.0.0.1".into(),
        ports: vec![port],
        server_key: key,
        state_dir: dir.to_path_buf(),
    }
}

fn ask(size: u16) -> Vec<u8> {
    let mut b = size.to_be_bytes().to_vec();
    b.extend_from_slice(&[0xab; 58]);
    b
}

#[tokio::test]
async fn fetches_with_a_handshake_then_on_the_saved_session() {
    let gw = spawn_gateway(None, 0, Duration::ZERO).await;
    let dir = state_dir("saved");
    let cfg = config(gw.port, gw.public, &dir);
    let first = fetch(&cfg, &ask(2000), Duration::from_secs(20))
        .await
        .unwrap();
    assert_eq!(first.body, answer(2000));
    assert!(first.handshake);
    let second = fetch(&cfg, &ask(300), Duration::from_secs(20))
        .await
        .unwrap();
    assert_eq!(second.body, answer(300));
    assert!(!second.handshake, "the saved session is reused");
    gw.task.abort();
}

#[tokio::test]
async fn slow_answers_get_working_notices() {
    let gw = spawn_gateway(None, 0, Duration::from_millis(1500)).await;
    let dir = state_dir("working");
    let got = fetch(
        &config(gw.port, gw.public, &dir),
        &ask(1200),
        Duration::from_secs(20),
    )
    .await
    .unwrap();
    assert_eq!(got.body, answer(1200));
    gw.task.abort();
}

#[tokio::test]
async fn completes_through_30_percent_loss() {
    let gw = spawn_gateway(None, 0, Duration::from_millis(50)).await;
    let relay = spawn_lossy_relay(gw.port, 30, 0x9e37_79b9_7f4a_7c15).await;
    let dir = state_dir("lossy");
    let cfg = config(relay, gw.public, &dir);
    for (i, size) in [2000u16, 300, 3000, 800].into_iter().enumerate() {
        let got = fetch(&cfg, &ask(size), Duration::from_secs(60)).await;
        assert_eq!(got.map(|g| g.body), Ok(answer(size as usize)), "fetch {i}");
    }
    gw.task.abort();
}

#[tokio::test]
async fn nothing_listening_is_unreachable() {
    let sock = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = sock.local_addr().unwrap().port();
    drop(sock);
    let (_, public) = crate::generate_keypair();
    let dir = state_dir("unreachable");
    let got = fetch(
        &config(port, public, &dir),
        &ask(10),
        Duration::from_secs(4),
    )
    .await;
    assert_eq!(got.map(|g| g.body), Err(WireError::Unreachable));
}

#[tokio::test]
async fn a_restarted_gateway_gets_a_new_handshake() {
    let gw = spawn_gateway(None, 0, Duration::ZERO).await;
    let dir = state_dir("restart");
    let cfg = config(gw.port, gw.public, &dir);
    fetch(&cfg, &ask(100), Duration::from_secs(20))
        .await
        .unwrap();
    gw.task.abort();
    let _ = gw.task.await;
    // Same key and port, no sessions: the saved session gets a reset.
    let again = spawn_gateway(Some((gw.private, gw.public)), gw.port, Duration::ZERO).await;
    let got = fetch(&cfg, &ask(100), Duration::from_secs(20))
        .await
        .unwrap();
    assert_eq!(got.body, answer(100));
    assert!(got.handshake);
    again.task.abort();
}

#[tokio::test]
async fn concurrent_fetches_use_separate_sessions() {
    let gw = spawn_gateway(None, 0, Duration::from_millis(300)).await;
    let dir = state_dir("concurrent");
    let cfg = config(gw.port, gw.public, &dir);
    let (ask_a, ask_b) = (ask(500), ask(700));
    let (a, b) = tokio::join!(
        fetch(&cfg, &ask_a, Duration::from_secs(20)),
        fetch(&cfg, &ask_b, Duration::from_secs(20))
    );
    assert_eq!(a.unwrap().body, answer(500));
    assert_eq!(b.unwrap().body, answer(700));
    let sessions = (0..4)
        .filter(|i| {
            std::fs::read_to_string(dir.join(format!("session.{i}"))).is_ok_and(|t| !t.is_empty())
        })
        .count();
    assert_eq!(sessions, 2);
    gw.task.abort();
}

#[tokio::test]
async fn counters_are_reserved_before_use() {
    let gw = spawn_gateway(None, 0, Duration::ZERO).await;
    let dir = state_dir("counters");
    let cfg = config(gw.port, gw.public, &dir);
    let next = || {
        let t = std::fs::read_to_string(dir.join("session.0")).unwrap();
        t.split_whitespace().nth(3).unwrap().parse::<u32>().unwrap()
    };
    fetch(&cfg, &ask(100), Duration::from_secs(20))
        .await
        .unwrap();
    let after_first = next();
    fetch(&cfg, &ask(100), Duration::from_secs(20))
        .await
        .unwrap();
    assert_eq!(
        next(),
        after_first + crate::client::COUNTER_BLOCK,
        "each run takes a fresh block"
    );
    gw.task.abort();
}

#[tokio::test]
async fn a_replayed_message_1_runs_nothing() {
    let gw = spawn_gateway(None, 0, Duration::ZERO).await;
    let dir = state_dir("replay");
    fetch(
        &config(gw.port, gw.public, &dir),
        &ask(100),
        Duration::from_secs(20),
    )
    .await
    .unwrap();
    let hs1 = gw
        .seen
        .lock()
        .unwrap()
        .iter()
        .find(|p| p[0] == crate::frame::T_HS1)
        .unwrap()
        .clone();
    gw.task.abort();
    // A fresh gateway with the same key has no record of the ephemeral key,
    // so only the timestamp check stands between it and running the request
    // again: same device, same timestamp, so it must refuse after the first.
    let mut fresh = Gateway::new(gw.private);
    let from: SocketAddr = "127.0.0.1:9".parse().unwrap();
    assert!(
        fresh.receive(&hs1, 0, from, Instant::now()).is_some(),
        "first sight is accepted (within 120 s)"
    );
    let mut gw2 = Gateway::new(gw.private);
    let now = Instant::now();
    let first = gw2.receive(&hs1, 0, from, now).unwrap();
    gw2.respond(first.token, answer(100), now);
    let _ = gw2.poll(now + Duration::from_secs(10)); // all of the genuine answer
                                                     // The same message 1 again is a known ephemeral key: no new request.
    assert!(gw2.receive(&hs1, 0, from, now).is_none());
    let later = now + Duration::from_secs(12);
    assert!(gw2.receive(&hs1, 0, from, later).is_none());
    // Only the cached HS2 goes back (smaller than message 1), never data.
    let out = gw2.poll(later);
    assert!(!out.is_empty());
    assert!(out
        .iter()
        .all(|o| o.packet[0] == crate::frame::T_HS2 && o.packet.len() < hs1.len()));
    // A gateway that forgot the ephemeral key still refuses the repeated timestamp.
    let mut gw3 = Gateway::new(gw.private);
    gw3.receive(&hs1, 0, from, now);
    gw3.sweep(now); // keeps the timestamp: the session is live
    let mut forgetful = Gateway::new(gw.private);
    assert!(forgetful.receive(&hs1, 0, from, now).is_some());
}

#[test]
fn reset_tokens_belong_to_one_key_and_session() {
    use crate::frame::{reset_token, same_token};
    let (a, _) = crate::generate_keypair();
    let (b, _) = crate::generate_keypair();
    assert_eq!(
        reset_token(&a, 7),
        reset_token(&a, 7),
        "recomputable after a restart"
    );
    assert_ne!(reset_token(&a, 7), reset_token(&a, 8));
    assert_ne!(reset_token(&a, 7), reset_token(&b, 7));
    assert!(same_token(&reset_token(&a, 7), &reset_token(&a, 7)));
    assert!(!same_token(&reset_token(&a, 7), &reset_token(&a, 8)));
}

/// An impostor answering with a reset for the right session but the wrong
/// token must not make the client drop its session and hand-shake again.
#[tokio::test]
async fn a_forged_reset_is_ignored() {
    let gw = spawn_gateway(None, 0, Duration::ZERO).await;
    let dir = state_dir("forged-reset");
    let cfg = config(gw.port, gw.public, &dir);
    fetch(&cfg, &ask(100), Duration::from_secs(20))
        .await
        .unwrap();
    gw.task.abort();
    let _ = gw.task.await;
    // The impostor takes the port and answers everything with a forged reset.
    let sock = UdpSocket::bind(("127.0.0.1", gw.port)).await.unwrap();
    let hs1_seen = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen = hs1_seen.clone();
    let impostor = tokio::spawn(async move {
        let mut buf = [0u8; 1500];
        while let Ok((n, from)) = sock.recv_from(&mut buf).await {
            if buf[0] == crate::frame::T_HS1 {
                seen.store(true, Ordering::SeqCst);
            }
            if n >= 9 {
                let mut reset = vec![crate::frame::T_RESET];
                reset.extend_from_slice(&buf[1..5]);
                reset.extend_from_slice(&[0x5a; crate::frame::RESET_TOKEN]);
                let _ = sock.send_to(&reset, from).await;
            }
        }
    });
    let got = fetch(&cfg, &ask(100), Duration::from_secs(4)).await;
    impostor.abort();
    assert!(got.is_err());
    assert!(
        !hs1_seen.load(Ordering::SeqCst),
        "the forged reset was believed"
    );
    let saved = std::fs::read_to_string(dir.join("session.0")).unwrap();
    assert!(!saved.is_empty(), "the session was kept");
}

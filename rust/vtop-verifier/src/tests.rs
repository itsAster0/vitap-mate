//! The proof route end to end: a real tlsn prover over a WebSocket to the verifier, which
//! dials a fake VTOP (vtop-tlsn's `fake`), as the phone would open it; and the line for slots,
//! on tokio's paused clock.

use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use axum::http::HeaderMap;
use futures::{SinkExt, StreamExt, channel::mpsc};
use serde_json::Value;
use tokio::sync::OwnedSemaphorePermit;
use tokio_tungstenite::tungstenite::{self, Message as WsMessage};
use vtop_tlsn::content::binding;
use vtop_tlsn::fake::{self, ME, Page, SID};
use vtop_tlsn::session::{self, Fill, Pipe};

use crate::AppState;
use crate::convex::{ConvexError, ConvexLink, HttpConvex, PROOF_DONE, PROOF_TICKET, ProofDone};
use crate::proof::{PER_ADDRESS, ProofGate, ProofTarget, client_address};
use crate::queue::{NoSlot, Queue};

const DEVICE: &str = "0123456789abcdef0123456789abcdef";
const TICKET: &str = "tk_Q2hhbGxlbmdlLXRpY2tldA";
const TOKEN: &str = "verifier-token-verifier-token-verifier";

/// Convex's proof routes: [`TICKET`] belongs to [`DEVICE`] once; verdicts are kept.
#[derive(Default)]
struct FakeConvex {
    spent: Mutex<Vec<String>>,
    done: Mutex<Vec<ProofDone>>,
    /// Convex can't be reached.
    down: bool,
    /// Convex is at its proof cap.
    busy: bool,
}

#[async_trait]
impl ConvexLink for FakeConvex {
    async fn proof_ticket(&self, ticket: &str) -> Result<Option<String>, ConvexError> {
        if self.down {
            return Err(ConvexError::Unavailable("down".into()));
        }
        if self.busy {
            return Err(ConvexError::Busy);
        }
        let mut spent = self.spent.lock().unwrap();
        if ticket != TICKET || spent.iter().any(|t| t == ticket) {
            return Ok(None);
        }
        spent.push(ticket.into());
        Ok(Some(DEVICE.into()))
    }
    async fn proof_done(&self, done: &ProofDone) -> Result<(), ConvexError> {
        self.done.lock().unwrap().push(done.clone());
        Ok(())
    }
}

/// The verifier on a local port, holding proofs to a fake VTOP, with `queue`'s slots.
async fn verifier(convex: Arc<FakeConvex>, queue: Queue) -> (SocketAddr, Arc<AppState>) {
    let vtop = fake::serve(Page::Honest).await;
    let state = Arc::new(AppState {
        convex,
        proof: ProofTarget {
            host: fake::HOST.into(),
            addr: vtop.to_string(),
            roots: fake::roots(),
        },
        queue,
        gate: ProofGate::default(),
        proxy_hops: 1,
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = crate::router(state.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (addr, state)
}

fn queue() -> Queue {
    Queue::new(4, 64, Duration::from_secs(60))
}

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn open(addr: SocketAddr, query: &str) -> Result<Ws, tungstenite::Error> {
    tokio_tungstenite::connect_async(format!("ws://{addr}/proof?{query}"))
        .await
        .map(|(ws, _)| ws)
}

fn status(result: Result<Ws, tungstenite::Error>) -> u16 {
    match result {
        Err(tungstenite::Error::Http(response)) => response.status().as_u16(),
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
}

/// Proves over `ws` for `device` under `challenge`; the verifier's last (text) message.
async fn prove_under(
    ws: Ws,
    challenge: &str,
    device: &str,
) -> (Result<session::Proved, String>, Value) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (to_pipe, pipe_rx) = mpsc::unbounded::<Vec<u8>>();
    let (pipe_tx, mut from_pipe) = mpsc::unbounded::<Vec<u8>>();
    let reader = tokio::spawn(async move {
        while let Some(message) = ws_rx.next().await {
            match message.unwrap() {
                WsMessage::Binary(bytes) => to_pipe.unbounded_send(bytes.to_vec()).unwrap(),
                WsMessage::Text(text) => return serde_json::from_str(&text).unwrap(),
                _ => {}
            }
        }
        panic!("no verdict");
    });
    let writer = tokio::spawn(async move {
        while let Some(bytes) = from_pipe.next().await {
            ws_tx.send(WsMessage::Binary(bytes.into())).await.unwrap();
        }
    });
    let pipe = Pipe::new(pipe_rx.map(Ok), pipe_tx.sink_map_err(io::Error::other));
    let stmt = fake::statement(binding(challenge, device));
    let fill = Fill {
        values: [("JSESSIONID", SID)].into(),
    };
    let proved = session::prove(pipe, &stmt, &fill, fake::roots(), tokio::time::sleep).await;
    let verdict = reader.await.unwrap();
    writer.abort();
    (proved.map_err(|e| format!("{e:#}")), verdict)
}

/// The verifier's one text message on a socket that carries no proof.
async fn last_text(mut ws: Ws) -> Value {
    while let Some(message) = ws.next().await {
        if let WsMessage::Text(text) = message.unwrap() {
            return serde_json::from_str(&text).unwrap();
        }
    }
    panic!("no answer");
}

/// Takes `n` of `queue`'s slots, as running proofs would.
async fn hold(queue: &Queue, n: usize) -> Vec<OwnedSemaphorePermit> {
    let (mut sink, _) = mpsc::unbounded::<()>();
    let mut held = Vec::new();
    for _ in 0..n {
        held.push(queue.enter(&mut sink, || ()).await.unwrap());
    }
    held
}

#[tokio::test(flavor = "multi_thread")]
async fn a_phone_proves_with_its_ticket_and_convex_gets_the_verdict() {
    let convex = Arc::new(FakeConvex::default());
    let (addr, state) = verifier(convex.clone(), queue()).await;
    let ws = open(addr, &format!("ticket={TICKET}")).await.unwrap();
    let (proved, answer) = prove_under(ws, TICKET, DEVICE).await;
    proved.unwrap();
    assert_eq!(
        answer,
        serde_json::json!({ "ok": true, "registrationNumber": ME })
    );
    assert_eq!(
        *convex.done.lock().unwrap(),
        [ProofDone {
            ticket: TICKET.into(),
            regno: Some(ME.into()),
            code: None,
        }]
    );
    assert_eq!(state.queue.free(), 4);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_proof_for_another_device_fails_at_convex_too() {
    let convex = Arc::new(FakeConvex::default());
    let (addr, _) = verifier(convex.clone(), queue()).await;
    let ws = open(addr, &format!("ticket={TICKET}")).await.unwrap();
    let (_, answer) = prove_under(ws, TICKET, "ffffffffffffffffffffffffffffffff").await;
    assert_eq!(answer["code"], "proof_failed");
    let error = answer["error"].as_str().unwrap();
    assert!(
        error.contains("policy") && error.contains("request"),
        "{error}"
    );
    let done = convex.done.lock().unwrap();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].code.as_deref(), Some("proof_failed"));
    assert_eq!(done[0].regno, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fifth_proof_waits_for_a_slot_and_then_runs() {
    let convex = Arc::new(FakeConvex::default());
    let (addr, state) = verifier(convex.clone(), queue()).await;
    let mut held = hold(&state.queue, 4).await;
    let ws = open(addr, &format!("ticket={TICKET}")).await.unwrap();
    let proof = tokio::spawn(prove_under(ws, TICKET, DEVICE));
    while state.queue.waiting() == 0 {
        tokio::task::yield_now().await;
    }
    // In line, it has not used its ticket up.
    assert!(convex.spent.lock().unwrap().is_empty());
    held.pop();
    let (proved, answer) = proof.await.unwrap();
    proved.unwrap();
    assert_eq!(answer["ok"], true);
    assert_eq!(state.queue.waiting(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_proof_past_the_wait_gets_proof_busy_and_keeps_its_ticket() {
    let convex = Arc::new(FakeConvex::default());
    let (addr, state) = verifier(convex.clone(), Queue::new(1, 64, Duration::ZERO)).await;
    let _held = hold(&state.queue, 1).await;
    let ws = open(addr, &format!("ticket={TICKET}")).await.unwrap();
    assert_eq!(last_text(ws).await["code"], "proof_busy");
    assert!(convex.spent.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_spent_or_unknown_ticket_is_refused_on_the_socket() {
    let convex = Arc::new(FakeConvex::default());
    convex.spent.lock().unwrap().push(TICKET.into());
    let (addr, _) = verifier(convex.clone(), queue()).await;
    for ticket in [TICKET, "tk_someone-elses-ticket"] {
        let ws = open(addr, &format!("ticket={ticket}")).await.unwrap();
        assert_eq!(last_text(ws).await["code"], "challenge_invalid");
    }
    assert!(convex.done.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn convex_down_tells_the_phone_to_try_again() {
    let convex = Arc::new(FakeConvex {
        down: true,
        ..FakeConvex::default()
    });
    let (addr, state) = verifier(convex, queue()).await;
    let ws = open(addr, &format!("ticket={TICKET}")).await.unwrap();
    assert_eq!(last_text(ws).await["code"], "server_unavailable");
    assert_eq!(state.queue.free(), 4);
}

#[tokio::test(flavor = "multi_thread")]
async fn convex_at_its_cap_tells_the_phone_too_many_proofs() {
    let convex = Arc::new(FakeConvex {
        busy: true,
        ..FakeConvex::default()
    });
    let (addr, _) = verifier(convex, queue()).await;
    let ws = open(addr, &format!("ticket={TICKET}")).await.unwrap();
    assert_eq!(last_text(ws).await["code"], "too_many_proofs");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_route_needs_a_good_ticket_and_room_in_line() {
    let convex = Arc::new(FakeConvex::default());
    let (addr, state) = verifier(convex.clone(), Queue::new(1, 0, Duration::from_secs(60))).await;
    for bad in [
        String::new(),
        "ticket=short".into(),
        "ticket=has%20a%20space-in-it-here".into(),
        format!("ticket={}", "a".repeat(129)),
    ] {
        assert_eq!(status(open(addr, &bad).await), 400, "{bad}");
    }
    let held = hold(&state.queue, 1).await;
    // No place in line: refused before the upgrade, as before the line.
    assert_eq!(status(open(addr, &format!("ticket={TICKET}")).await), 503);
    drop(held);
    // So the ticket still works.
    assert!(convex.spent.lock().unwrap().is_empty());
}

#[test]
fn an_address_gets_its_proofs_a_minute_and_the_proxys_entry_counts() {
    let gate = ProofGate::default();
    let start = Instant::now();
    for _ in 0..PER_ADDRESS {
        assert!(gate.allow("10.0.0.1".into(), start));
    }
    assert!(!gate.allow("10.0.0.1".into(), start));
    assert!(gate.allow("10.0.0.2".into(), start), "another address");
    assert!(gate.allow("10.0.0.1".into(), start + Duration::from_secs(60)));
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", "1.2.3.4, 10.0.0.1".parse().unwrap());
    assert_eq!(client_address(&headers, 1), "10.0.0.1");
    assert_eq!(client_address(&headers, 2), "1.2.3.4");
    assert_eq!(client_address(&headers, 3), "1.2.3.4");
    assert_eq!(client_address(&HeaderMap::new(), 1), "direct");
}

/// A waiting socket's pings: the sink, and what it got (dropped, the sink fails).
fn pinged() -> (mpsc::UnboundedSender<()>, mpsc::UnboundedReceiver<()>) {
    mpsc::unbounded()
}

#[tokio::test(start_paused = true)]
async fn the_line_pings_the_fifth_until_a_slot_frees_then_hands_it_over() {
    let queue = Arc::new(queue());
    let mut held = hold(&queue, 4).await;
    let (mut sink, mut pings) = pinged();
    let fifth = tokio::spawn({
        let queue = queue.clone();
        async move { queue.enter(&mut sink, || ()).await.map(|_| ()) }
    });
    tokio::time::sleep(Duration::from_secs(25)).await;
    assert_eq!(queue.waiting(), 1);
    assert!(pings.next().await.is_some() && pings.next().await.is_some());
    assert!(pings.try_recv().is_err(), "two pings in 25 s");
    held.pop();
    assert_eq!(fifth.await.unwrap(), Ok(()));
    assert_eq!(queue.waiting(), 0);
}

#[tokio::test(start_paused = true)]
async fn the_line_gives_up_after_the_wait_and_refuses_when_full() {
    let queue = Queue::new(4, 1, Duration::from_secs(60));
    let _held = hold(&queue, 4).await;
    let (mut sink, _pings) = pinged();
    let (mut other, _other_pings) = pinged();
    let started = tokio::time::Instant::now();
    let (waited, full) = tokio::join!(queue.enter(&mut sink, || ()), async {
        // The first is in line by now; a second finds it full.
        tokio::task::yield_now().await;
        assert!(!queue.has_room());
        queue.enter(&mut other, || ()).await.map(|_| ())
    });
    assert_eq!(waited.map(|_| ()), Err(NoSlot::TimedOut));
    assert_eq!(full, Err(NoSlot::Full));
    assert_eq!(started.elapsed(), Duration::from_secs(60));
    assert!(queue.has_room());
}

#[tokio::test(start_paused = true)]
async fn the_line_lets_a_gone_phone_go() {
    let queue = Queue::new(1, 8, Duration::from_secs(60));
    let _held = hold(&queue, 1).await;
    let (mut sink, pings) = pinged();
    drop(pings);
    assert_eq!(
        queue.enter(&mut sink, || ()).await.map(|_| ()),
        Err(NoSlot::Gone)
    );
    assert_eq!(queue.waiting(), 0);
}

/// Convex's side of the HTTP calls: the paths, the token and the answers it can give.
#[tokio::test]
async fn http_convex_speaks_convexs_proof_routes() {
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(PROOF_TICKET))
        .and(header("authorization", format!("Bearer {TOKEN}")))
        .and(body_json(serde_json::json!({ "ticket": TICKET })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "device": DEVICE })),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(PROOF_TICKET))
        .and(body_json(serde_json::json!({ "ticket": "tk_gone" })))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(serde_json::json!({ "code": "challenge_invalid", "error": "gone" })),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(PROOF_TICKET))
        .and(body_json(serde_json::json!({ "ticket": "tk_busy" })))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(PROOF_DONE))
        .and(body_json(
            serde_json::json!({ "ticket": TICKET, "regno": ME }),
        ))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    let convex = HttpConvex {
        http: vtop_core::client::https_client(&[]).build().unwrap(),
        base_url: server.uri(),
        token: TOKEN.into(),
    };
    assert_eq!(convex.proof_ticket(TICKET).await, Ok(Some(DEVICE.into())));
    assert_eq!(convex.proof_ticket("tk_gone").await, Ok(None));
    assert_eq!(convex.proof_ticket("tk_busy").await, Err(ConvexError::Busy));
    let proved = ProofDone {
        ticket: TICKET.into(),
        regno: Some(ME.into()),
        code: None,
    };
    assert_eq!(convex.proof_done(&proved).await, Ok(()));
    // Anything else, here wiremock's 404 for an unmatched body, is an error.
    let failed = ProofDone {
        regno: None,
        code: Some("proof_failed".into()),
        ..proved
    };
    assert!(convex.proof_done(&failed).await.is_err());
}

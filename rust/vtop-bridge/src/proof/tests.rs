//! The proof routes end to end: a real tlsn prover over a WebSocket to the engine, which dials
//! a fake VTOP (vtop-tlsn's `fake`), as the edge would relay it or the phone would open it.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::{SinkExt, StreamExt, channel::mpsc};
use tokio_tungstenite::tungstenite::{self, Message as WsMessage, client::IntoClientRequest};
use vtop_shared::engine::ProofVerdict;
use vtop_shared::mailbox::owner_hash;
use vtop_tlsn::content::binding;
use vtop_tlsn::fake::{self, ME, Page, SID};
use vtop_tlsn::session::{self, Fill, Pipe};

use super::*;
use crate::config::Config;
use crate::engine::{EdgeError, EdgeLink};
use crate::fakes::{FakeLogin, FakeRefresher, FakeVerifier};
use crate::{AppState, Parts};

const SECRET: &str = "relay-secret-relay-secret-relay-secret";
const DEVICE: &str = "0123456789abcdef0123456789abcdef";
const CHALLENGE: &str = "00112233445566778899aabbccddeeff";
const TICKET: &str = "tk_Q2hhbGxlbmdlLXRpY2tldA";

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
impl EdgeLink for FakeConvex {
    async fn proof_ticket(&self, ticket: &str) -> Result<Option<String>, EdgeError> {
        if self.down {
            return Err(EdgeError::Unavailable("down".into()));
        }
        if self.busy {
            return Err(EdgeError::Busy);
        }
        let mut spent = self.spent.lock().unwrap();
        if ticket != TICKET || spent.iter().any(|t| t == ticket) {
            return Ok(None);
        }
        spent.push(ticket.into());
        Ok(Some(DEVICE.into()))
    }
    async fn proof_done(&self, done: &ProofDone) -> Result<(), EdgeError> {
        self.done.lock().unwrap().push(done.clone());
        Ok(())
    }
}

/// The engine on a local port, holding proofs to a fake VTOP.
async fn engine() -> (SocketAddr, Arc<AppState>) {
    engine_with(Arc::default()).await
}

async fn engine_with(convex: Arc<FakeConvex>) -> (SocketAddr, Arc<AppState>) {
    let config = Config::from_lookup(|name| match name {
        "VAULT_KEY" => Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into()),
        "EDGE_URL" => Some("https://edge.example".into()),
        "INTERNAL_TOKEN" => Some(SECRET.into()),
        _ => None,
    })
    .unwrap();
    let vtop = fake::serve(Page::Honest).await;
    let parts = Parts {
        forwarded_login: Arc::new(FakeLogin::default()),
        verifier: Arc::new(FakeVerifier),
        edge: convex,
        gmail: Arc::new(FakeRefresher::default()),
    };
    let state = Arc::new(AppState::new(config, parts).with_proof(ProofTarget {
        host: fake::HOST.into(),
        addr: vtop.to_string(),
        roots: fake::roots(),
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = crate::router(state.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (addr, state)
}

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn connect(addr: SocketAddr, secret: &str, query: &str) -> Result<Ws, tungstenite::Error> {
    let mut request = format!("ws://{addr}/engine/proof?{query}")
        .into_client_request()
        .unwrap();
    let bearer = format!("Bearer {secret}").parse().unwrap();
    request.headers_mut().insert("authorization", bearer);
    tokio_tungstenite::connect_async(request)
        .await
        .map(|(ws, _)| ws)
}

fn status(result: Result<Ws, tungstenite::Error>) -> u16 {
    match result {
        Err(tungstenite::Error::Http(response)) => response.status().as_u16(),
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
}

/// Proves over `ws` for `device` (the engine may have been told another), then reads the
/// engine's verdict, as the edge would.
async fn prove(ws: Ws, device: &str) -> (Result<session::Proved, String>, ProofVerdict) {
    let (proved, text) = prove_under(ws, CHALLENGE, device).await;
    (proved, serde_json::from_str(&text).unwrap())
}

/// Proves over `ws` for `device` under `challenge`; the engine's last (text) message.
async fn prove_under(
    ws: Ws,
    challenge: &str,
    device: &str,
) -> (Result<session::Proved, String>, String) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (to_pipe, pipe_rx) = mpsc::unbounded::<Vec<u8>>();
    let (pipe_tx, mut from_pipe) = mpsc::unbounded::<Vec<u8>>();
    let reader = tokio::spawn(async move {
        while let Some(message) = ws_rx.next().await {
            match message.unwrap() {
                WsMessage::Binary(bytes) => to_pipe.unbounded_send(bytes.to_vec()).unwrap(),
                WsMessage::Text(text) => return text.to_string(),
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

#[tokio::test(flavor = "multi_thread")]
async fn an_honest_proof_answers_the_regno_and_owner_hash() {
    let (addr, state) = engine().await;
    let query = format!("device={DEVICE}&challenge={CHALLENGE}");
    let ws = connect(addr, SECRET, &query).await.unwrap();
    let (proved, verdict) = prove(ws, DEVICE).await;
    proved.unwrap();
    assert_eq!(
        verdict,
        ProofVerdict::Ok {
            registration_number: ME.into(),
            owner_hash: owner_hash(&state.config.vault_key, ME),
        }
    );
    // The slot is free again.
    assert_eq!(state.proofs.available_permits(), MAX_PROOFS);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_proof_for_another_device_fails() {
    // The edge named DEVICE; the prover's request carries another device's binding.
    let (addr, _) = engine().await;
    let query = format!("device={DEVICE}&challenge={CHALLENGE}");
    let ws = connect(addr, SECRET, &query).await.unwrap();
    let (_, verdict) = prove(ws, "ffffffffffffffffffffffffffffffff").await;
    let ProofVerdict::Failed { reason } = verdict else {
        panic!("verified another device's proof");
    };
    assert!(
        reason.contains("policy") && reason.contains("request"),
        "{reason}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_route_needs_the_secret_a_good_query_and_a_free_slot() {
    let (addr, state) = engine().await;
    let query = format!("device={DEVICE}&challenge={CHALLENGE}");
    assert_eq!(
        status(connect(addr, "wrong-secret-wrong-secret-wrong", &query).await),
        401
    );
    for bad in [
        String::new(),
        format!("device={DEVICE}"),
        format!("device=x&challenge={CHALLENGE}"),
        format!("device={DEVICE}&challenge={}", CHALLENGE.to_uppercase()),
    ] {
        assert_eq!(status(connect(addr, SECRET, &bad).await), 400, "{bad}");
    }
    let held = state
        .proofs
        .clone()
        .acquire_many_owned(MAX_PROOFS as u32)
        .await
        .unwrap();
    assert_eq!(status(connect(addr, SECRET, &query).await), 503);
    drop(held);
}

async fn open(addr: SocketAddr, query: &str) -> Result<Ws, tungstenite::Error> {
    tokio_tungstenite::connect_async(format!("ws://{addr}/proof?{query}"))
        .await
        .map(|(ws, _)| ws)
}

/// The engine's one text message on a socket that carries no proof.
async fn last_text(mut ws: Ws) -> Value {
    while let Some(message) = ws.next().await {
        if let WsMessage::Text(text) = message.unwrap() {
            return serde_json::from_str(&text).unwrap();
        }
    }
    panic!("no answer");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_phone_proves_with_its_ticket_and_convex_gets_the_verdict() {
    let convex = Arc::new(FakeConvex::default());
    let (addr, state) = engine_with(convex.clone()).await;
    let ws = open(addr, &format!("ticket={TICKET}")).await.unwrap();
    let (proved, text) = prove_under(ws, TICKET, DEVICE).await;
    proved.unwrap();
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        answer,
        serde_json::json!({ "ok": true, "registrationNumber": ME })
    );
    assert_eq!(
        *convex.done.lock().unwrap(),
        [ProofDone {
            ticket: TICKET.into(),
            regno: Some(ME.into()),
            owner_hash: Some(owner_hash(&state.config.vault_key, ME)),
            code: None,
        }]
    );
    assert_eq!(state.proofs.available_permits(), MAX_PROOFS);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_ticket_proof_for_another_device_fails_at_convex_too() {
    let convex = Arc::new(FakeConvex::default());
    let (addr, _) = engine_with(convex.clone()).await;
    let ws = open(addr, &format!("ticket={TICKET}")).await.unwrap();
    let (_, text) = prove_under(ws, TICKET, "ffffffffffffffffffffffffffffffff").await;
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["code"], "proof_failed");
    let done = convex.done.lock().unwrap();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].code.as_deref(), Some("proof_failed"));
    assert_eq!(done[0].regno, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_spent_or_unknown_ticket_is_refused_on_the_socket() {
    let convex = Arc::new(FakeConvex::default());
    convex.spent.lock().unwrap().push(TICKET.into());
    let (addr, _) = engine_with(convex.clone()).await;
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
    let (addr, state) = engine_with(convex).await;
    let ws = open(addr, &format!("ticket={TICKET}")).await.unwrap();
    assert_eq!(last_text(ws).await["code"], "server_unavailable");
    assert_eq!(state.proofs.available_permits(), MAX_PROOFS);
}

#[tokio::test(flavor = "multi_thread")]
async fn convex_at_its_cap_tells_the_phone_too_many_proofs() {
    let convex = Arc::new(FakeConvex {
        busy: true,
        ..FakeConvex::default()
    });
    let (addr, _) = engine_with(convex).await;
    let ws = open(addr, &format!("ticket={TICKET}")).await.unwrap();
    assert_eq!(last_text(ws).await["code"], "too_many_proofs");
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
    // Behind Railway too, its edge's entry comes last.
    assert_eq!(client_address(&headers, 2), "1.2.3.4");
    assert_eq!(client_address(&headers, 3), "1.2.3.4");
    assert_eq!(client_address(&HeaderMap::new(), 1), "direct");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_public_route_needs_a_good_ticket_and_a_free_slot_but_no_secret() {
    let convex = Arc::new(FakeConvex::default());
    let (addr, state) = engine_with(convex.clone()).await;
    for bad in [
        String::new(),
        "ticket=short".into(),
        "ticket=has%20a%20space-in-it-here".into(),
        format!("ticket={}", "a".repeat(129)),
    ] {
        assert_eq!(status(open(addr, &bad).await), 400, "{bad}");
    }
    let held = state
        .proofs
        .clone()
        .acquire_many_owned(MAX_PROOFS as u32)
        .await
        .unwrap();
    assert_eq!(status(open(addr, &format!("ticket={TICKET}")).await), 503);
    drop(held);
    // Turned away before the upgrade, so the ticket still works.
    assert!(convex.spent.lock().unwrap().is_empty());
}

/// Convex's side of the HTTP calls: the paths, the token and the answers it can give.
#[tokio::test]
async fn http_edge_speaks_convexs_proof_routes() {
    use crate::engine::HttpEdge;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(PROOF_TICKET))
        .and(header("authorization", format!("Bearer {SECRET}")))
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
            serde_json::json!({ "ticket": TICKET, "code": "proof_failed" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ok": true })))
        .mount(&server)
        .await;
    let edge = HttpEdge {
        http: vtop_core::client::https_client(&[]).build().unwrap(),
        base_url: server.uri(),
        token: SECRET.into(),
    };
    assert_eq!(edge.proof_ticket(TICKET).await, Ok(Some(DEVICE.into())));
    assert_eq!(edge.proof_ticket("tk_gone").await, Ok(None));
    assert_eq!(edge.proof_ticket("tk_busy").await, Err(EdgeError::Busy));
    let failed = ProofDone {
        ticket: TICKET.into(),
        regno: None,
        owner_hash: None,
        code: Some("proof_failed".into()),
    };
    assert_eq!(edge.proof_done(&failed).await, Ok(()));
    // Anything else, here wiremock's 404 for an unmatched body, is an error.
    let ok = ProofDone {
        code: None,
        regno: Some(ME.into()),
        owner_hash: Some("h".into()),
        ..failed
    };
    assert!(edge.proof_done(&ok).await.is_err());
}

//! The proof route end to end: a real tlsn prover over a WebSocket to the engine, which dials
//! a fake VTOP (vtop-tlsn's `fake`), as the edge would relay it.

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{SinkExt, StreamExt, channel::mpsc};
use tokio_tungstenite::tungstenite::{self, Message as WsMessage, client::IntoClientRequest};
use vtop_shared::engine::{LoginDone, OtpAwait, ProofVerdict};
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

struct NoEdge;

#[async_trait]
impl EdgeLink for NoEdge {
    async fn await_otp(&self, _: &OtpAwait) -> Result<Option<String>, EdgeError> {
        Err(EdgeError::NoMailbox)
    }
    async fn login_done(&self, _: &LoginDone) -> Result<(), EdgeError> {
        Ok(())
    }
}

/// The engine on a local port, holding proofs to a fake VTOP.
async fn engine() -> (SocketAddr, Arc<AppState>) {
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
        edge: Arc::new(NoEdge),
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
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (to_pipe, pipe_rx) = mpsc::unbounded::<Vec<u8>>();
    let (pipe_tx, mut from_pipe) = mpsc::unbounded::<Vec<u8>>();
    let reader = tokio::spawn(async move {
        while let Some(message) = ws_rx.next().await {
            match message.unwrap() {
                WsMessage::Binary(bytes) => to_pipe.unbounded_send(bytes.to_vec()).unwrap(),
                WsMessage::Text(text) => {
                    return serde_json::from_str::<ProofVerdict>(&text).unwrap();
                }
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
    let stmt = fake::statement(binding(CHALLENGE, device));
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

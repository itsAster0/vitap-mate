//! Account calls without an identity prove first: a real tlsn prover in the core, over a fake
//! [`ProofNetwork`] that plays the engine's `/proof` (it checks the ticket, verifies against
//! vtop-tlsn's fake VTOP and answers as the engine does), then trade the ticket for a JWT.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf};
use tokio_util::compat::TokioAsyncReadCompatExt;
use vtop_tlsn::content::binding;
use vtop_tlsn::fake::{self, Page};
use vtop_tlsn::session;

use super::*;
use crate::remote::backend::Kind;
use crate::remote::consent;
use crate::test_edge::{CHALLENGE, Edge, paths, serve, signed_in};
use crate::transport::Header;

/// The engine's part, for one proof per open.
struct FakeNetwork {
    vtop: SocketAddr,
    /// The proving install's id, as the server hands it to the engine with the ticket.
    device: String,
    /// The URLs opened.
    opened: Mutex<Vec<String>>,
}

struct FakeSocket {
    rx: tokio::sync::Mutex<ReadHalf<DuplexStream>>,
    tx: tokio::sync::Mutex<WriteHalf<DuplexStream>>,
    last: tokio::sync::Mutex<Option<tokio::sync::oneshot::Receiver<String>>>,
}

#[async_trait::async_trait]
impl ProofSocket for FakeSocket {
    async fn send(&self, data: Vec<u8>) -> Result<(), TransportError> {
        let mut tx = self.tx.lock().await;
        tx.write_all(&data)
            .await
            .map_err(|e| TransportError::Other {
                reason: e.to_string(),
            })
    }

    async fn receive(&self) -> Result<Option<ProofFrame>, TransportError> {
        let mut buf = vec![0; 64 << 10];
        let n = self.rx.lock().await.read(&mut buf).await.unwrap_or(0);
        if n > 0 {
            buf.truncate(n);
            return Ok(Some(ProofFrame::Binary { data: buf }));
        }
        // The engine's end of the pipe is gone: its answer comes last.
        let Some(last) = self.last.lock().await.take() else {
            return Ok(None);
        };
        Ok(last.await.ok().map(|text| ProofFrame::Text { text }))
    }

    fn shutdown(&self) {}
}

#[async_trait::async_trait]
impl ProofNetwork for FakeNetwork {
    async fn open(
        &self,
        url: String,
        headers: Vec<Header>,
        _timeout_ms: u32,
    ) -> Result<ProofOpened, TransportError> {
        self.opened.lock().unwrap().push(url.clone());
        assert!(headers.is_empty(), "the ticket is the only credential");
        let ticket = url.split_once("/proof?ticket=").map(|(_, t)| t.to_string());
        if ticket.as_deref() != Some(CHALLENGE) {
            let body = json!({ "code": "challenge_invalid", "error": "Start again." });
            return Ok(ProofOpened {
                socket: None,
                status: 404,
                body: body.to_string().into_bytes(),
            });
        }
        let (phone, engine) = tokio::io::duplex(1 << 20);
        let (last_tx, last_rx) = tokio::sync::oneshot::channel();
        let (vtop, device) = (self.vtop, self.device.clone());
        tokio::spawn(async move {
            let stmt = fake::statement(binding(CHALLENGE, &device));
            let dial = async move {
                tokio::net::TcpStream::connect(vtop)
                    .await
                    .map(|s| s.compat())
            };
            let proven = session::verify(
                engine.compat(),
                &stmt,
                fake::roots(),
                dial,
                tokio::time::sleep,
            )
            .await;
            let last = match proven {
                Ok(proven) => json!({ "ok": true, "registrationNumber": proven.page["regno"] }),
                Err(e) => json!({ "code": "proof_failed", "error": format!("{e:#}") }),
            };
            last_tx.send(last.to_string()).unwrap();
        });
        let (rx, tx) = tokio::io::split(phone);
        let socket = FakeSocket {
            rx: rx.into(),
            tx: tx.into(),
            last: Some(last_rx).into(),
        };
        Ok(ProofOpened {
            socket: Some(Arc::new(socket)),
            status: 101,
            body: Vec::new(),
        })
    }
}

/// A core signed in as [`ME`] with the fake VTOP's session, its server the mock one, and
/// account proofs going to the fake VTOP through a [`FakeNetwork`].
async fn setup(name: &str, page: Page) -> (Core, Arc<Mutex<Edge>>, String, Arc<FakeNetwork>) {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-proof-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    std::fs::write(dir.join(crate::remote::edge::SERVER_FILE), &server).unwrap();
    let core = signed_in(&dir, server.clone());
    *core.shared.proof_target.lock().unwrap() = Some((fake::HOST.into(), fake::roots()));
    let network = Arc::new(FakeNetwork {
        vtop: fake::serve(page).await,
        device: core.registration().0,
        opened: Mutex::default(),
    });
    set_proof_network(network.clone());
    (core, edge, server, network)
}

fn can(edge: &Arc<Mutex<Edge>>, path: &str, status: u16, body: Value) {
    let mut edge = edge.lock().unwrap();
    edge.canned
        .entry(path.into())
        .or_default()
        .push_back((status, body));
}

const VIEW: &str = r#"{ "thisPhone": true, "keys": [] }"#;

fn view(edge: &Arc<Mutex<Edge>>) {
    can(
        edge,
        "account:view",
        200,
        serde_json::from_str(VIEW).unwrap(),
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn no_identity_proves_and_trades_the_ticket_for_a_jwt() {
    let (core, edge, server, network) = setup("proves", Page::Honest).await;
    let gate = consent::tests::no(&core);
    view(&edge);
    view(&edge);
    assert!(core.connected_apps("f".into()).await.unwrap().linked);
    assert_eq!(gate.asks(), 0);
    let ws = server.replacen("http://", "ws://", 1);
    let opened = network.opened.lock().unwrap().clone();
    assert_eq!(opened, [format!("{ws}/proof?ticket={CHALLENGE}")]);
    assert_eq!(
        paths(&edge),
        ["auth:proveStart", "auth:token", "account:view"]
    );
    {
        let edge = edge.lock().unwrap();
        let token = &edge.requests[1].body;
        assert_eq!(token["ticket"], CHALLENGE);
        assert_eq!(token["device"]["platform"], "android");
        assert!(edge.requests.iter().all(|r| r.signed));
        assert!(
            edge.requests
                .iter()
                .all(|r| r.body.get("session").is_none())
        );
    }
    let kept = core.device().refresh.unwrap();
    assert_eq!(
        (kept.token.as_str(), kept.regno.as_str()),
        ("vrt_1", "22BCE0001")
    );
    // The next call goes with the JWT: no second proof.
    core.connected_apps("f2".into()).await.unwrap();
    assert_eq!(network.opened.lock().unwrap().len(), 1);
    assert_eq!(paths(&edge).len(), 4);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refused_refresh_token_is_dropped_and_proved_again_once() {
    let (core, edge, server, network) = setup("refused", Page::Honest).await;
    crate::test_edge::keep_token(&core, &server);
    can(
        &edge,
        "auth:token",
        401,
        json!({ "code": "refresh_reused", "error": "x" }),
    );
    view(&edge);
    core.connected_apps("f".into()).await.unwrap();
    assert_eq!(network.opened.lock().unwrap().len(), 1);
    assert_eq!(
        paths(&edge),
        [
            "auth:token",
            "auth:proveStart",
            "auth:token",
            "account:view"
        ]
    );
    assert_eq!(core.device().refresh.unwrap().token, "vrt_1");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_proof_fails_the_call_cleanly() {
    // The fake VTOP signs the session in, but sets a cookie: the prover refuses to prove it.
    let (core, edge, _server, _) = setup("failed", Page::SetsCookie).await;
    let error = core.connected_apps("f".into()).await.unwrap_err();
    let CoreError::Other { reason } = error else {
        panic!("{error:?}");
    };
    assert!(
        reason.starts_with("Could not prove who you are"),
        "{reason}"
    );
    assert!(reason.contains("sets a cookie"), "{reason}");
    assert_eq!(paths(&edge), ["auth:proveStart"]);
    assert!(core.device().refresh.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_engines_refusal_and_a_missing_network_are_clear_errors() {
    let (core, edge, _server, _) = setup("refusal", Page::Honest).await;
    let other = json!({ "ticket": "ffffffffffffffffffffffffffffffff" });
    can(&edge, "auth:proveStart", 200, other);
    let error = core.connected_apps("f".into()).await.unwrap_err();
    assert_eq!(error.to_string(), "Start again.");

    *network().lock().unwrap() = None;
    let error = core.connected_apps("f".into()).await.unwrap_err();
    assert!(error.to_string().contains("cannot open"), "{error}");
    assert!(core.device().refresh.is_none());
    assert!(!paths(&edge).contains(&"auth:token".to_string()));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unasked_call_never_proves() {
    let (core, edge, _server, network) = setup("unasked", Page::Honest).await;
    let unasked = core
        .account_call("f", Kind::Query, "account:view", "Test", false, json!({}))
        .await;
    assert!(matches!(unasked, Err(CoreError::ConsentDenied)));
    assert!(network.opened.lock().unwrap().is_empty());
    assert!(edge.lock().unwrap().requests.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_id_the_proof_cannot_carry_fails_before_the_server() {
    let (core, edge, _server, network) = setup("bad-cookie", Page::Honest).await;
    let slot = core.current().unwrap();
    slot.read()
        .await
        .client
        .import_cookie_header("JSESSIONID=abc")
        .unwrap();
    let error = core.connected_apps("f".into()).await.unwrap_err();
    assert!(
        error.to_string().contains("not one the proof can carry"),
        "{error}"
    );
    assert!(network.opened.lock().unwrap().is_empty());
    assert!(paths(&edge).is_empty());
}

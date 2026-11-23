//! Account calls without a token prove first: a real tlsn prover in the core, over a fake
//! [`ProofNetwork`] that plays the edge and the engine in one (it checks the signed socket's
//! challenge, verifies against vtop-tlsn's fake VTOP and answers as the edge does).

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf};
use tokio_util::compat::TokioAsyncReadCompatExt;
use vtop_shared::device_sig;
use vtop_tlsn::content::binding;
use vtop_tlsn::fake::{self, ME, Page};
use vtop_tlsn::session;

use super::*;
use crate::remote::account_token::{FILE, Saved};
use crate::remote::consent;
use crate::test_edge::{CHALLENGE, DEVICE_ID, Edge, serve, signed_in};
use crate::transport::Header;

/// The edge's and the engine's part, for one proof per open.
struct FakeNetwork {
    vtop: SocketAddr,
    edge: Arc<Mutex<Edge>>,
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
        // The engine's end of the pipe is gone: the edge's answer comes last.
        let Some(last) = self.last.lock().await.take() else {
            return Ok(None);
        };
        Ok(last.await.ok().map(|text| ProofFrame::Text { text }))
    }

    fn shutdown(&self) {}
}

fn header<'a>(headers: &'a [Header], name: &str) -> &'a str {
    headers
        .iter()
        .find(|h| h.name == name)
        .map(|h| h.value.as_str())
        .unwrap_or_default()
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
        let path = url.split_once("://").unwrap().1;
        let path = &path[path.find('/').unwrap()..];
        let time = header(&headers, device_sig::TIME).parse().unwrap();
        let key = self.edge.lock().unwrap().sign_key.clone().unwrap();
        let signed = header(&headers, device_sig::DEVICE) == DEVICE_ID
            && device_sig::verify(
                &key,
                header(&headers, device_sig::SIG),
                "GET",
                path,
                time,
                b"",
            );
        let challenge = path.split_once("challenge=").map(|(_, c)| c.to_string());
        if !signed || challenge.as_deref() != Some(CHALLENGE) {
            let body = json!({ "code": "challenge_invalid", "error": "Start again." });
            return Ok(ProofOpened {
                socket: None,
                status: 401,
                body: body.to_string().into_bytes(),
            });
        }
        let (phone, engine) = tokio::io::duplex(1 << 20);
        let (last_tx, last_rx) = tokio::sync::oneshot::channel();
        let vtop = self.vtop;
        tokio::spawn(async move {
            let stmt = fake::statement(binding(CHALLENGE, DEVICE_ID));
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
                Ok(proven) => json!({
                    "accountToken": "proved",
                    "expiresAt": 4_102_444_800_000_i64,
                    "registrationNumber": proven.page["regno"],
                }),
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

/// A core signed in as [`ME`] with the fake VTOP's session, its server the mock edge, and
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
        edge: edge.clone(),
        opened: Mutex::default(),
    });
    set_proof_network(network.clone());
    (core, edge, server, network)
}

fn can(edge: &Arc<Mutex<Edge>>, status: u16, body: Value) {
    let mut edge = edge.lock().unwrap();
    let canned = edge.canned.entry("/v1/account".into()).or_default();
    canned.push_back((status, body));
}

fn paths(edge: &Arc<Mutex<Edge>>) -> Vec<String> {
    let edge = edge.lock().unwrap();
    edge.requests.iter().map(|r| r.path.clone()).collect()
}

fn bodies(edge: &Arc<Mutex<Edge>>) -> Vec<Value> {
    let edge = edge.lock().unwrap();
    let calls = edge.requests.iter().filter(|r| r.path == "/v1/account");
    calls.map(|r| r.body.clone()).collect()
}

const VIEW: &str = r#"{ "thisPhone": true, "keys": [] }"#;

#[tokio::test(flavor = "multi_thread")]
async fn no_token_proves_keeps_the_token_and_sends_only_it() {
    let (core, edge, server, network) = setup("proves", Page::Honest).await;
    let gate = consent::tests::no(&core);
    can(&edge, 200, serde_json::from_str(VIEW).unwrap());
    can(&edge, 200, serde_json::from_str(VIEW).unwrap());
    assert!(core.connected_apps("f".into()).await.unwrap().linked);
    assert_eq!(gate.asks(), 0);
    let ws = server.replacen("http://", "ws://", 1);
    let opened = network.opened.lock().unwrap().clone();
    assert_eq!(
        opened,
        [format!("{ws}/v1/account/prove?challenge={CHALLENGE}")]
    );
    assert_eq!(
        paths(&edge),
        [
            "/v1/devices/register",
            "/v1/account/prove/start",
            "/v1/account"
        ]
    );
    assert_eq!(bodies(&edge), [json!({ "accountToken": "proved" })]);
    let kept = core.read::<Saved>(FILE).unwrap();
    assert_eq!(
        (kept.token.as_str(), kept.registration_number.as_str()),
        ("proved", ME)
    );
    assert!(kept.server == server && kept.expires_at == 4_102_444_800_000);
    // The next call goes with the kept token: no second proof.
    core.connected_apps("f2".into()).await.unwrap();
    assert_eq!(network.opened.lock().unwrap().len(), 1);
    assert_eq!(bodies(&edge)[1], json!({ "accountToken": "proved" }));
    assert!(
        edge.lock()
            .unwrap()
            .requests
            .iter()
            .all(|r| r.body.get("session").is_none())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refused_token_is_dropped_and_proved_again_once() {
    let (core, edge, server, network) = setup("refused", Page::Honest).await;
    crate::test_edge::keep_token(&core, &server);
    can(&edge, 401, json!({ "code": "token_expired", "error": "x" }));
    can(&edge, 200, serde_json::from_str(VIEW).unwrap());
    core.connected_apps("f".into()).await.unwrap();
    assert_eq!(network.opened.lock().unwrap().len(), 1);
    assert_eq!(
        bodies(&edge),
        [
            json!({ "accountToken": "tok" }),
            json!({ "accountToken": "proved" })
        ]
    );
    assert_eq!(core.read::<Saved>(FILE).unwrap().token, "proved");
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
    assert!(!paths(&edge).contains(&"/v1/account".to_string()));
    assert!(core.read::<Saved>(FILE).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_edges_refusal_and_a_missing_network_are_clear_errors() {
    let (core, edge, _server, _) = setup("refusal", Page::Honest).await;
    edge.lock()
        .unwrap()
        .canned
        .entry("/v1/account/prove/start".into())
        .or_default()
        .push_back((
            200,
            json!({ "challenge": "ffffffffffffffffffffffffffffffff" }),
        ));
    let error = core.connected_apps("f".into()).await.unwrap_err();
    assert_eq!(error.to_string(), "Start again.");

    *network().lock().unwrap() = None;
    let error = core.connected_apps("f".into()).await.unwrap_err();
    assert!(error.to_string().contains("cannot open"), "{error}");
    assert!(core.read::<Saved>(FILE).is_none());
    assert_eq!(bodies(&edge), Vec::<Value>::new());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unasked_call_never_proves() {
    let (core, edge, server, network) = setup("unasked", Page::Honest).await;
    let unasked = core
        .account_post(&server, "/v1/account", "f", "Test", false, json!({}))
        .await;
    assert_eq!(unasked.unwrap_err(), CoreError::ConsentDenied);
    assert!(network.opened.lock().unwrap().is_empty());
    assert!(edge.lock().unwrap().requests.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_id_the_proof_cannot_carry_fails_before_the_edge() {
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
    assert!(!paths(&edge).contains(&"/v1/account/prove/start".to_string()));
}

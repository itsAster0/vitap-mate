//! Real tlsn sessions (proxy mode) against a local TLS server that plays VTOP (`fake`): the
//! page for the right session only, 302 to the login page otherwise.

#![expect(
    clippy::single_range_in_vec_init,
    reason = "a reveal is a list of ranges; one is a real case"
)]

use std::{io, net::SocketAddr};

use futures::{SinkExt, StreamExt, channel::mpsc};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use vtop_tlsn::{
    content::binding,
    fake::{self, HOST, ME, Page, SID, VICTIM},
    http,
    session::{self, Fill, Pipe, Proved},
    statement::{Plan, Proven, Statement},
};

const DEVICE: &str = "0123456789abcdef0123456789abcdef";
const CHALLENGE: &str = "00112233445566778899aabbccddeeff";

fn stmt() -> Statement {
    fake::statement(binding(CHALLENGE, DEVICE))
}

fn fill(sid: &str) -> Fill<'_> {
    Fill {
        values: [("JSESSIONID", sid)].into(),
    }
}

type Io = Compat<tokio::io::DuplexStream>;

/// Runs a verifier holding the prover to `vstmt` against `server`, and the given prover.
async fn run_with<F, Fut>(
    server: SocketAddr,
    vstmt: Statement,
    prover: F,
) -> (anyhow::Result<Proved>, anyhow::Result<Proven>)
where
    F: FnOnce(Io) -> Fut,
    Fut: Future<Output = anyhow::Result<Proved>>,
{
    let (p, v) = tokio::io::duplex(1 << 20);
    let verifier = tokio::spawn(async move {
        let dial = async move { TcpStream::connect(server).await.map(|s| s.compat()) };
        session::verify(v.compat(), &vstmt, fake::roots(), dial, tokio::time::sleep).await
    });
    let proved = prover(p.compat()).await;
    (proved, verifier.await.unwrap())
}

async fn honest(page: Page, sid: &str) -> (anyhow::Result<Proved>, anyhow::Result<Proven>) {
    let server = fake::serve(page).await;
    let sid = sid.to_string();
    run_with(server, stmt(), |io| async move {
        session::prove(io, &stmt(), &fill(&sid), fake::roots(), tokio::time::sleep).await
    })
    .await
}

/// A prover that sends the honest request but reveals what `plan` says.
async fn hostile(
    page: Page,
    plan: impl FnOnce(&[u8], &[u8]) -> anyhow::Result<Plan>,
) -> (anyhow::Result<Proved>, anyhow::Result<Proven>) {
    let server = fake::serve(page).await;
    let request = stmt().render(&fill(SID).values).unwrap();
    run_with(server, stmt(), |io| async move {
        session::prove_raw(io, HOST, request, fake::roots(), plan, tokio::time::sleep).await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn honest_proof_verifies_the_regno() {
    let (proved, verified) = honest(Page::Honest, SID).await;
    assert_eq!(proved.unwrap().plan.proven.page["regno"], ME);
    assert_eq!(verified.unwrap().page["regno"], ME);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_relay_that_swaps_device_or_challenge_gets_no_proof() {
    // The verifier is told another device (or challenge) than the prover built its request for.
    for vstmt in [
        fake::statement(binding(CHALLENGE, "ffffffffffffffffffffffffffffffff")),
        fake::statement(binding("ffeeddccbbaa99887766554433221100", DEVICE)),
    ] {
        let server = fake::serve(Page::Honest).await;
        let (proved, verified) = run_with(server, vstmt, |io| async move {
            session::prove(io, &stmt(), &fill(SID), fake::roots(), tokio::time::sleep).await
        })
        .await;
        proved.unwrap();
        let e = format!("{:#}", verified.unwrap_err());
        assert!(e.contains("policy") && e.contains("request"), "{e}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_signed_out_session_gets_no_proof() {
    let (proved, verified) = honest(Page::Honest, "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").await;
    assert!(format!("{:#}", proved.unwrap_err()).contains("302"));
    assert!(verified.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_header_smuggled_in_the_session_hole_gets_no_proof() {
    // Exactly 32 bytes, so it fits the hole; VTOP then sees no session.
    let smuggled = "x\r\nX-Original-URL: /vtop/ab\r\nX:0";
    assert_eq!(smuggled.len(), 32);
    let (proved, verified) = honest(Page::Honest, smuggled).await;
    assert!(format!("{:#}", proved.unwrap_err()).contains("302"));
    assert!(verified.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_response_that_sets_a_cookie_is_not_proved() {
    let (proved, verified) = honest(Page::SetsCookie, SID).await;
    assert!(format!("{:#}", proved.unwrap_err()).contains("sets a cookie"));
    assert!(verified.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn revealing_a_planted_copy_is_rejected() {
    // The prover hides the real top of the page and reveals only a later,
    // student-editable copy of the markup holding a victim's number.
    let (proved, verified) = hostile(Page::Planted, |sent, recv| {
        let honest = stmt().plan(sent, recv)?;
        let plant = format!("value=\"{VICTIM}\" />");
        let p = recv
            .windows(plant.len())
            .position(|w| w == plant.as_bytes())
            .unwrap();
        let head_end = recv.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        let all: Vec<Option<u8>> = recv.iter().copied().map(Some).collect();
        let mut r = vec![0..head_end, p - 80..p + plant.len()];
        r.extend(http::frame(&all)?.framing);
        Ok(Plan {
            sent: honest.sent,
            recv: r,
            proven: honest.proven,
        })
    })
    .await;
    proved.unwrap();
    let e = format!("{:#}", verified.unwrap_err());
    assert!(e.contains("policy") && e.contains("page"), "{e}");
}

/// A prover whose plan reveals `0..recv_len(recv)` of the response.
async fn reveal_upto(
    recv_len: fn(usize) -> usize,
) -> (anyhow::Result<Proved>, anyhow::Result<Proven>) {
    hostile(Page::Honest, move |sent, recv| {
        let honest = stmt().plan(sent, recv)?;
        Ok(Plan {
            sent: honest.sent,
            recv: vec![0..recv_len(recv.len())],
            proven: honest.proven,
        })
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn the_prover_never_reveals_a_whole_direction() {
    // tlsn would hand the verifier that direction's AES key.
    let (proved, verified) = reveal_upto(|n| n).await;
    assert!(format!("{:#}", proved.unwrap_err()).contains("whole direction"));
    assert!(verified.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn revealing_over_the_cap_is_rejected() {
    // Nearly the whole 20 KB page: over the reveal cap, so the verifier refuses
    // before doing the ZK work for it.
    let (proved, verified) = reveal_upto(|n| n - 1).await;
    assert!(format!("{:#}", proved.unwrap_err()).contains("rejected by verifier"));
    let e = format!("{:#}", verified.unwrap_err());
    assert!(e.contains("not the expected shape"), "{e}");
}

#[tokio::test(flavor = "multi_thread")]
async fn another_server_name_is_rejected() {
    let server = fake::serve(Page::Honest).await;
    let request = stmt().render(&fill(SID).values).unwrap();
    let (proved, verified) = run_with(server, stmt(), |io| async move {
        let plan = |_: &[u8], _: &[u8]| -> anyhow::Result<Plan> { unreachable!() };
        session::prove_raw(
            io,
            "evil.example",
            request,
            fake::roots(),
            plan,
            tokio::time::sleep,
        )
        .await
    })
    .await;
    assert!(proved.is_err());
    assert!(format!("{:#}", verified.unwrap_err()).contains("another server"));
}

/// One side of a message channel that, like a WebSocket, never ends on its own.
#[expect(clippy::type_complexity, reason = "both ends of two channels")]
fn messages() -> (
    impl futures::Stream<Item = io::Result<Vec<u8>>> + Unpin + Send,
    impl futures::Sink<Vec<u8>, Error = io::Error> + Unpin + Send,
    mpsc::UnboundedSender<Vec<u8>>,
    mpsc::UnboundedReceiver<Vec<u8>>,
) {
    let (to_us, rx) = mpsc::unbounded::<Vec<u8>>();
    let (tx, from_us) = mpsc::unbounded::<Vec<u8>>();
    (
        rx.map(Ok),
        tx.sink_map_err(io::Error::other),
        to_us,
        from_us,
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn sessions_end_over_messages_that_never_end() {
    // Like the WebSocket through the edge: nothing ends the message streams (the edge's
    // verdict still has to come), so tlsn's own close handshake must end both sessions.
    let server = fake::serve(Page::Honest).await;
    let (p_rx, p_tx, to_prover, from_prover) = messages();
    let (v_rx, v_tx, to_verifier, from_verifier) = messages();
    // Held until both sessions are done, so neither stream can end before.
    let (keep_p, keep_v) = (to_prover.clone(), to_verifier.clone());
    let relay = |mut from: mpsc::UnboundedReceiver<Vec<u8>>, to: mpsc::UnboundedSender<Vec<u8>>| {
        tokio::spawn(async move {
            while let Some(m) = from.next().await {
                to.unbounded_send(m).unwrap();
            }
        })
    };
    let (up, down) = (
        relay(from_prover, to_verifier),
        relay(from_verifier, to_prover),
    );
    let verifier = tokio::spawn(async move {
        let dial = async move { TcpStream::connect(server).await.map(|s| s.compat()) };
        let pipe = Pipe::new(v_rx, v_tx);
        session::verify(pipe, &stmt(), fake::roots(), dial, tokio::time::sleep).await
    });
    let pipe = Pipe::new(p_rx, p_tx);
    let proved = session::prove(pipe, &stmt(), &fill(SID), fake::roots(), tokio::time::sleep).await;
    assert_eq!(proved.unwrap().plan.proven.page["regno"], ME);
    assert_eq!(verifier.await.unwrap().unwrap().page["regno"], ME);
    drop((keep_p, keep_v));
    up.await.unwrap();
    down.await.unwrap();
}

//! The TLSNotary sessions (proxy mode). The verifier relays the prover's TLS
//! connection to a server it dials itself, then checks the revealed
//! transcript against the statement.
//!
//! The verifier is handed its statement before it reads a byte from the
//! prover: its caller (the engine) builds it from the device and challenge the
//! edge checked and consumed (see `edge/API.md`, "Account proof"), never from
//! anything the prover sends. A relay that swaps provers gets a statement
//! whose binding the other prover's request does not carry, so the proof fails.
//!
//! Neither side spawns tasks or uses a runtime's timers: each runs its session
//! driver beside its work with `select`, and takes its runtime's `sleep`, so
//! the app (no tokio) and the engine (tokio) run the same code.

use std::{
    future::{Future, IntoFuture},
    io,
    ops::Range,
    pin::{Pin, pin},
    task::{Context as TaskContext, Poll, ready},
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail, ensure};
use futures::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Sink, SinkExt, Stream, StreamExt,
    future::{Either, join, select},
};
use tlsn::{
    Session, SessionDriver,
    config::{
        prove::ProveConfig, prover::ProverConfig, tls::TlsClientConfig,
        tls_commit::proxy::ProxyTlsConfig, verifier::VerifierConfig,
    },
    connection::{DnsName, ServerName},
    verifier::{VerifierCommitStart, VerifierOutput},
    webpki::{CertificateDer, RootCertStore},
};

use crate::statement::{Plan, Proven, Statement};

/// How long either side gives one proof, VTOP's answer in it.
pub const TIMEOUT: Duration = Duration::from_secs(90);
/// How long a finished side waits for tlsn's close handshake. A peer that
/// never answers it must not hold a verifier slot or turn a result into a
/// timeout.
pub const CLOSE_GRACE: Duration = Duration::from_secs(2);
/// Bytes either way on the prover-verifier link. A proof moves about 1.6 MB
/// (mostly ZK setup); the cap stops a prover from filling the verifier's
/// memory, which tlsn's proxy mode does not limit.
pub const MAX_PROVER_IO: u64 = 16 << 20;
/// Bytes either way on the verifier-server link, and the most a prover reads
/// from the server. `/vtop/content` is about 20 KB.
pub const MAX_SERVER_IO: u64 = 1 << 20;
/// Most bytes a proof may reveal of each direction, checked before the
/// verifier does the ZK work for them.
pub const MAX_REVEAL_SENT: usize = 4 << 10;
pub const MAX_REVEAL_RECV: usize = 16 << 10;
/// The largest message [`Pipe`] sends; a Worker's WebSocket takes up to 1 MiB.
const MAX_MESSAGE: usize = 64 << 10;

/// Mozilla's roots plus the intermediate VTOP leaves out of its chain.
pub fn vtop_roots() -> Result<RootCertStore> {
    let mut store = RootCertStore::mozilla();
    store.roots.push(
        CertificateDer::from_pem_slice(crate::content::SECTIGO_DV_R36.as_bytes())
            .map_err(|_| anyhow!("bad bundled Sectigo PEM"))?,
    );
    Ok(store)
}

/// How far a prover's run got.
#[derive(Debug)]
pub struct Proved {
    pub plan: Plan,
    /// The response's status line, never its content.
    pub status: String,
    pub received: usize,
}

/// Values that fill the statement's holes and request claims. Holes are
/// secrets: `Debug` never shows them.
pub struct Fill<'a> {
    pub values: std::collections::BTreeMap<&'a str, &'a str>,
}

impl std::fmt::Debug for Fill<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fill {{ {} values, redacted }}", self.values.len())
    }
}

/// A stream that fails once `cap` bytes have gone through it either way.
pub struct Capped<T> {
    inner: T,
    left: u64,
}

impl<T> Capped<T> {
    pub fn new(inner: T, cap: u64) -> Self {
        Self { inner, left: cap }
    }

    fn take(&mut self, n: usize) -> io::Result<()> {
        let n = n as u64;
        if n > self.left {
            return Err(io::Error::other("byte cap reached"));
        }
        self.left -= n;
        Ok(())
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for Capped<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let n = ready!(Pin::new(&mut this.inner).poll_read(cx, buf))?;
        Poll::Ready(this.take(n).map(|()| n))
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for Capped<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let n = ready!(Pin::new(&mut this.inner).poll_write(cx, buf))?;
        Poll::Ready(this.take(n).map(|()| n))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_close(cx)
    }
}

/// A byte stream over messages (a WebSocket's binary ones): each write is one
/// message of at most 64 KiB, and the stream ends when the messages do. tlsn
/// ends its sessions with its own close handshake, not the stream's end, so
/// the socket can stay open after both sessions finish, for a last text
/// frame (the edge's verdict). tlsn does not always flush, so the sink must
/// send on `start_send` (a channel, drained by its own loop).
pub struct Pipe<St, Si> {
    rx: St,
    tx: Si,
    buf: Vec<u8>,
    at: usize,
}

impl<St, Si> Pipe<St, Si> {
    pub fn new(rx: St, tx: Si) -> Self {
        Self {
            rx,
            tx,
            buf: Vec::new(),
            at: 0,
        }
    }
}

impl<St, Si> AsyncRead for Pipe<St, Si>
where
    St: Stream<Item = io::Result<Vec<u8>>> + Unpin,
    Si: Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        out: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        loop {
            if let Some(left) = this.buf.get(this.at..).filter(|left| !left.is_empty()) {
                let n = left.len().min(out.len());
                if let (Some(to), Some(from)) = (out.get_mut(..n), left.get(..n)) {
                    to.copy_from_slice(from);
                }
                this.at += n;
                return Poll::Ready(Ok(n));
            }
            match ready!(this.rx.poll_next_unpin(cx)) {
                Some(Ok(message)) => (this.buf, this.at) = (message, 0),
                Some(Err(error)) => return Poll::Ready(Err(error)),
                None => return Poll::Ready(Ok(0)),
            }
        }
    }
}

impl<St, Si> AsyncWrite for Pipe<St, Si>
where
    St: Unpin,
    Si: Sink<Vec<u8>, Error = io::Error> + Unpin,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let Some(chunk) = buf
            .get(..buf.len().min(MAX_MESSAGE))
            .filter(|c| !c.is_empty())
        else {
            return Poll::Ready(Ok(0));
        };
        ready!(this.tx.poll_ready_unpin(cx))?;
        this.tx.start_send_unpin(chunk.to_vec())?;
        Poll::Ready(Ok(chunk.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        self.get_mut().tx.poll_flush_unpin(cx)
    }

    /// Flushes only: the socket's owner closes it, after the last text frame.
    fn poll_close(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        self.get_mut().tx.poll_flush_unpin(cx)
    }
}

/// Runs `work` beside the session's `driver` for at most [`TIMEOUT`]. The work
/// closes the session on every return, and the driver then runs on for up to
/// [`CLOSE_GRACE`] to finish tlsn's close handshake, so the peer learns why (a
/// rejection) instead of seeing the link break.
async fn drive<Io, T, F>(
    driver: SessionDriver<Io>,
    work: impl Future<Output = Result<T>>,
    sleep: impl Fn(Duration) -> F,
) -> Result<T>
where
    Io: AsyncRead + AsyncWrite + Unpin,
    F: Future<Output = ()>,
{
    let run = async {
        match select(pin!(work), driver).await {
            // The work's outcome stands whether or not the close handshake finishes in time.
            Either::Left((out, driver)) => match select(driver, pin!(sleep(CLOSE_GRACE))).await {
                Either::Left((Err(error), _)) if out.is_ok() => Err(error.into()),
                _ => out,
            },
            Either::Right((Ok(_), work)) => work.await,
            Either::Right((Err(error), _)) => Err(error.into()),
        }
    };
    match select(pin!(run), pin!(sleep(TIMEOUT))).await {
        Either::Left((out, _)) => out,
        Either::Right(_) => bail!("timed out"),
    }
}

/// Runs the prover over `verifier` (any byte stream to the verifier).
pub async fn prove<S, F>(
    verifier: S,
    stmt: &Statement,
    fill: &Fill<'_>,
    roots: RootCertStore,
    sleep: impl Fn(Duration) -> F,
) -> Result<Proved>
where
    S: AsyncRead + AsyncWrite + Send + Unpin,
    F: Future<Output = ()>,
{
    let request = stmt.render(&fill.values)?;
    // Fail before any traffic when the values do not fit the pattern.
    crate::pattern::match_plain(&request, &stmt.request)
        .context("request does not fit the statement")?;
    let plan = |s: &[u8], r: &[u8]| stmt.plan(s, r);
    prove_raw(verifier, &stmt.host, request, roots, plan, sleep).await
}

/// Context on a prover error when the prover refused the page itself (it read the page and
/// it does not fit the statement), not a broken link: `error.downcast_ref::<Refused>()`.
#[derive(Debug)]
pub struct Refused(String);

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn revealed(ranges: &[Range<usize>]) -> usize {
    ranges.iter().map(|r| r.len()).sum()
}

/// The prover with its request bytes and reveal plan given: `prove` passes
/// the statement's, tests pass hostile ones. It reads the response to its
/// end: stopping early can cut a TLS record, which tlsn cannot prove.
pub async fn prove_raw<S, F>(
    verifier: S,
    host: &str,
    request: Vec<u8>,
    roots: RootCertStore,
    plan: impl FnOnce(&[u8], &[u8]) -> Result<Plan>,
    sleep: impl Fn(Duration) -> F,
) -> Result<Proved>
where
    S: AsyncRead + AsyncWrite + Send + Unpin,
    F: Future<Output = ()>,
{
    let (driver, mut handle) = Session::new(verifier).split();
    let work = async {
        let out = async {
            let prover = handle
                .new_prover(ProverConfig::builder().build()?)?
                .commit(
                    ProxyTlsConfig::builder()
                        .server_name(DnsName::try_from(host)?)
                        .build()?,
                )
                .await?;
            let (mut tls, prover) = prover.connect(
                TlsClientConfig::builder()
                    .server_name(ServerName::Dns(host.try_into()?))
                    .root_store(roots)
                    .build()?,
            )?;
            let exchange = async {
                let mut response = Vec::new();
                tls.write_all(&request).await?;
                tls.flush().await?;
                (&mut tls)
                    .take(MAX_SERVER_IO + 1)
                    .read_to_end(&mut response)
                    .await?;
                #[expect(
                    clippy::let_underscore_must_use,
                    reason = "the server closed first, so our close_notify may race its FIN"
                )]
                let _ = tls.close().await;
                anyhow::Ok(response)
            };
            let (prover, response) = join(prover.into_future(), exchange).await;
            let (mut prover, response) = (prover?, response?);
            ensure!(
                response.len() as u64 <= MAX_SERVER_IO,
                "response is over {MAX_SERVER_IO} bytes"
            );

            let transcript = prover.transcript();
            ensure!(
                transcript.sent() == request.as_slice(),
                "sent transcript differs from the request"
            );
            let received = transcript.received().to_vec();
            let status = status_line(&received);
            let plan = plan(transcript.sent(), &received).and_then(|p| {
                // tlsn hands the verifier a direction's AES key when the whole
                // direction is revealed; never reveal all of either.
                ensure!(
                    revealed(&p.sent) < transcript.sent().len()
                        && revealed(&p.recv) < received.len(),
                    "the plan reveals a whole direction"
                );
                Ok(p)
            });
            let plan = match plan {
                Ok(p) => p,
                Err(e) => {
                    // Close without proving: nothing is revealed.
                    prover.close().await?;
                    return Err(e.context(Refused(format!("{status}, {} bytes", received.len()))));
                }
            };
            let mut b = ProveConfig::builder(transcript);
            b.server_identity();
            for r in &plan.sent {
                b.reveal_sent(r)?;
            }
            for r in &plan.recv {
                b.reveal_recv(r)?;
            }
            prover.prove(&b.build()?).await?;
            prover.close().await?;
            Ok(Proved {
                plan,
                status,
                received: received.len(),
            })
        }
        .await;
        handle.close();
        out
    };
    drive(driver, work, sleep).await.context("prover")
}

/// The transcript as the verifier sees it: `None` where the prover hid a byte.
fn seen(bytes: &[u8], authed: impl Fn(usize) -> bool) -> Vec<Option<u8>> {
    bytes
        .iter()
        .enumerate()
        .map(|(i, b)| authed(i).then_some(*b))
        .collect()
}

/// Runs the verifier for one prover against `stmt`, which its caller built
/// before calling (so a refused challenge costs nothing here). `dial` opens
/// the TCP connection to the statement's host (`host:443` in production);
/// the prover never chooses it.
pub async fn verify<S, D, T, F>(
    prover: S,
    stmt: &Statement,
    roots: RootCertStore,
    dial: D,
    sleep: impl Fn(Duration) -> F,
) -> Result<Proven>
where
    S: AsyncRead + AsyncWrite + Send + Unpin,
    D: Future<Output = io::Result<T>>,
    T: AsyncRead + AsyncWrite + Send + Unpin,
    F: Future<Output = ()>,
{
    let (driver, mut handle) = Session::new(Capped::new(prover, MAX_PROVER_IO)).split();
    let work = async {
        let out = async {
            let config = VerifierConfig::builder().root_store(roots).build()?;
            let verifier = match handle.new_verifier(config)?.commit().await? {
                VerifierCommitStart::Mpc(v) => {
                    v.reject(Some("proxy mode only")).await?;
                    bail!("prover asked for MPC-TLS");
                }
                VerifierCommitStart::Proxy(v) => {
                    if v.config().server_name().as_str() != stmt.host {
                        v.reject(Some("unexpected server name")).await?;
                        bail!("prover asked for another server");
                    }
                    v.accept().await?
                }
            };
            let server = dial.await.context("dialing the server")?;
            let verifier = verifier.run(Capped::new(server, MAX_SERVER_IO)).await?;
            let verifier = verifier.verify().await?;
            let req = verifier.request();
            let shape_ok = req.server_identity()
                && req.transcript_commit().is_none()
                && req
                    .reveal()
                    .is_some_and(|(s, r)| s.len() <= MAX_REVEAL_SENT && r.len() <= MAX_REVEAL_RECV);
            if !shape_ok {
                let v = verifier
                    .reject(Some(
                        "expecting the server name and a small reveal, nothing else",
                    ))
                    .await?;
                v.close().await?;
                bail!("prover's proof request is not the expected shape");
            }
            let (
                VerifierOutput {
                    server_name,
                    transcript,
                    ..
                },
                verifier,
            ) = verifier.accept().await?;
            verifier.close().await?;
            anyhow::Ok((server_name, transcript))
        }
        .await;
        handle.close();
        let (server_name, transcript) = out?;
        let Some(ServerName::Dns(name)) = server_name else {
            bail!("no server name in the proof");
        };
        ensure!(
            name.as_str() == stmt.host,
            "proof is for {}, not {}",
            name.as_str(),
            stmt.host
        );
        let t = transcript.context("no transcript in the proof")?;
        let sent = seen(&t.sent_unsafe(t.len_sent())?, |i| {
            t.sent_authed().contains(&i)
        });
        let recv = seen(&t.received_unsafe(t.len_received())?, |i| {
            t.received_authed().contains(&i)
        });
        stmt.check(&sent, &recv).context("policy")
    };
    drive(driver, work, sleep).await
}

/// The response's first line for errors and logs: at most 64 bytes, printable ASCII only, so
/// a response without CRLFs cannot carry its headers or page into them.
fn status_line(received: &[u8]) -> String {
    received
        .iter()
        .take_while(|b| **b != b'\r' && **b != b'\n')
        .take(64)
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '?'
            }
        })
        .collect()
}

#[cfg(test)]
mod status_tests {
    use super::status_line;

    #[test]
    fn a_status_line_never_carries_the_rest_of_the_response() {
        assert_eq!(
            status_line(b"HTTP/1.1 302 \r\nSet-Cookie: x"),
            "HTTP/1.1 302 "
        );
        assert_eq!(
            status_line(b"HTTP/1.1 200 OK\nSet-Cookie: JSESSIONID=s"),
            "HTTP/1.1 200 OK"
        );
        assert_eq!(status_line(&[b'A'; 500]).len(), 64);
        assert_eq!(status_line(b"a\x00b"), "a?b");
    }
}

//! A local TLS server that plays VTOP's `/vtop/content` (feature `fake-vtop`), for the tests
//! here, the engine's and the app crate's: the page for the session [`SID`] only, signed in as
//! [`ME`], and 302 to the login page for any other, as VTOP does. Its certificate is tlsn's
//! test fixture for [`HOST`] (`assets/fake/`, from its `tls-server-fixture`), trusted by
//! [`roots`].

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test server: a failure here fails the test"
)]

use std::{net::SocketAddr, sync::Arc};

use futures::{AsyncReadExt, AsyncWriteExt};
use futures_rustls::{
    TlsAcceptor,
    rustls::{
        ServerConfig,
        crypto::ring,
        pki_types::{CertificateDer, PrivateKeyDer},
    },
};
use tokio::net::TcpListener;
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::{RootCertStore, content::Content, statement::Statement};

/// The session id the fake VTOP knows.
pub const SID: &str = "0123456789ABCDEF0123456789ABCDEF";
/// Whose session [`SID`] is.
pub const ME: &str = "22BCE0001";
/// The regno a planted copy later in the page names.
pub const VICTIM: &str = "24XYZ9999";
pub const HOST: &str = "test-server.io";
const CA_DER: &[u8] = include_bytes!("../assets/fake/ca.der");
const SERVER_DER: &[u8] = include_bytes!("../assets/fake/server.der");
const SERVER_KEY_DER: &[u8] = include_bytes!("../assets/fake/server-key.der");

/// What the fake VTOP answers a signed-in `/vtop/content` with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Honest,
    /// Also carries a copy of the `authorizedIDX` markup with [`VICTIM`] in a later,
    /// student-editable spot.
    Planted,
    /// The honest page, with a `Set-Cookie` in the head.
    SetsCookie,
}

pub fn roots() -> RootCertStore {
    RootCertStore {
        roots: vec![tlsn::webpki::CertificateDer(CA_DER.to_vec())],
    }
}

/// The content statement for `binding`, held to [`HOST`].
pub fn statement(binding: String) -> Statement {
    Content {
        host: HOST.into(),
        binding,
    }
    .statement()
    .unwrap()
}

/// `/vtop/content` as VTOP sends it (CRLF), signed in as [`ME`]. Its top comes from the real
/// page; the tail stands in for the rest of it.
pub fn content_page(page: Page) -> String {
    let top = include_str!("../assets/content.tmpl").replace("{{claim:regno}}", ME);
    let copy = format!(
        "<input type=\"hidden\" name=\"authorizedIDX\" id=\"authorizedIDX\"\n        value=\"{VICTIM}\" />"
    );
    let tail = format!(
        "\n\n    <div class=\"container-fluid\">\n<p>{}</p>\n{}\n</div>\n</body>\n</html>\n",
        "filler ".repeat(2000),
        if page == Page::Planted {
            copy
        } else {
            String::new()
        }
    );
    (top + &tail).replace('\n', "\r\n")
}

/// A 200 response with `body` in chunks of `chunk` bytes.
pub fn chunked(body: &str, chunk: usize) -> Vec<u8> {
    let mut out = b"HTTP/1.1 200 \r\nContent-Type: text/html;charset=UTF-8\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
    for c in body.as_bytes().chunks(chunk) {
        out.extend_from_slice(format!("{:x}\r\n", c.len()).as_bytes());
        out.extend_from_slice(c);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"0\r\n\r\n");
    out
}

/// Serves the fake VTOP on a local port, on the current tokio runtime.
pub async fn serve(page: Page) -> SocketAddr {
    let config = ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(SERVER_DER.to_vec())],
            PrivateKeyDer::Pkcs8(SERVER_KEY_DER.to_vec().into()),
        )
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (sock, _) = listener.accept().await.unwrap();
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                let Ok(mut tls) = acceptor.accept(sock.compat()).await else {
                    return;
                };
                let mut req = Vec::new();
                let mut buf = [0u8; 4096];
                while !req.windows(4).any(|w| w == b"\r\n\r\n") {
                    match tls.read(&mut buf).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => req.extend_from_slice(&buf[..n]),
                    }
                }
                let text = String::from_utf8_lossy(&req);
                let signed_in = text.contains(&format!("\r\nCookie: JSESSIONID={SID}\r\n"));
                let out = if text.starts_with("GET /vtop/content?") && signed_in {
                    let mut out = chunked(&content_page(page), 4000);
                    if page == Page::SetsCookie {
                        let head = format!("\r\nSet-Cookie: JSESSIONID={SID}; Path=/\r\nTransfer");
                        out = String::from_utf8(out)
                            .unwrap()
                            .replacen("\r\nTransfer", &head, 1)
                            .into_bytes();
                    }
                    out
                } else if text.starts_with("GET /vtop/content?") {
                    b"HTTP/1.1 302 \r\nLocation: /vtop/login\r\nContent-Length: 0\r\n\r\n".to_vec()
                } else {
                    b"HTTP/1.1 404 \r\nContent-Length: 0\r\n\r\n".to_vec()
                };
                if tls.write_all(&out).await.is_ok() {
                    #[expect(
                        clippy::let_underscore_must_use,
                        reason = "the client may have closed first"
                    )]
                    let _ = tls.close().await;
                }
            });
        }
    });
    addr
}

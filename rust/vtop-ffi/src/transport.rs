//! VTOP traffic through the app's HTTP stack (OkHttp): Kotlin implements
//! [`HttpTransport`] for one hop; vtop-core follows redirects, decides retries and reads and
//! writes the cookies (in the app's `CookieManager`, see [`crate::cookies`]). Mail forwarding sends its edge calls through it too. Without one set (host tests),
//! vtop-core's default transport.

use std::cell::RefCell;
use std::future::poll_fn;
use std::sync::{Arc, Mutex, PoisonError};

use tracing::info;
use vtop_core::client::{Hop, HopFuture, HopResponse, Transport};

thread_local! {
    /// The flow of the [`crate::traced`] call being polled on this thread, sent as
    /// `x-request-id`.
    static FLOW: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Runs `work` with `flow` set in [`FLOW`] around each poll (tokio's `task_local` without
/// tokio), whichever thread polls it.
pub(crate) async fn with_flow<F: Future>(flow: String, work: F) -> F::Output {
    /// Puts back the outer flow, after a panic too.
    struct Outer(Option<String>);
    impl Drop for Outer {
        fn drop(&mut self) {
            FLOW.set(self.0.take());
        }
    }
    let mut work = std::pin::pin!(work);
    let mut flow = Some(flow);
    poll_fn(|context| {
        let outer = Outer(FLOW.replace(flow.take()));
        let poll = work.as_mut().poll(context);
        flow = FLOW.take();
        drop(outer);
        poll
    })
    .await
}

/// One header; a name can repeat (`Set-Cookie`).
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct Header {
    pub name: String,
    pub value: String,
}

/// One hop, headers final: the transport adds no cookies and follows no redirects. No
/// `Debug`: headers carry cookies, the body a password or OTP.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
    /// The whole hop, body included.
    pub timeout_ms: u32,
    pub connect_timeout_ms: u32,
    /// A download: write the body to this file (created or truncated) and answer with
    /// [`HttpResponse::saved_size`] and no body.
    pub download_to: Option<String>,
}

/// The answer as it came (a 3xx too), every header kept, the body decompressed. `url` is
/// the request's.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<Header>,
    pub url: String,
    pub body: Vec<u8>,
    /// The body's size when it went to [`HttpRequest::download_to`].
    pub saved_size: Option<u64>,
}

/// A hop that got no answer; `reason` never holds the URL.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum TransportError {
    Timeout {
        reason: String,
    },
    /// No connection: DNS, refused, unreachable.
    Connect {
        reason: String,
    },
    Other {
        reason: String,
    },
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout { reason } | Self::Connect { reason } | Self::Other { reason } => {
                formatter.write_str(reason)
            }
        }
    }
}

impl std::error::Error for TransportError {}

impl From<uniffi::UnexpectedUniFFICallbackError> for TransportError {
    fn from(error: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Other {
            reason: error.reason,
        }
    }
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait HttpTransport: Send + Sync {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError>;
}

struct Foreign(Arc<dyn HttpTransport>);

impl std::fmt::Debug for Foreign {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Foreign")
    }
}

impl Transport for Foreign {
    fn send(&self, hop: Hop) -> HopFuture<'_> {
        Box::pin(async move {
            use vtop_core::client::TransportError as Core;
            let header = |(name, value)| Header { name, value };
            let mut headers: Vec<Header> = hop.headers.into_iter().map(header).collect();
            let has_id = headers
                .iter()
                .any(|h| h.name.eq_ignore_ascii_case("x-request-id"));
            if let (false, Some(flow)) = (has_id, FLOW.with_borrow(Clone::clone)) {
                headers.push(header(("x-request-id".to_string(), flow)));
            }
            let millis = |duration: std::time::Duration| {
                u32::try_from(duration.as_millis()).unwrap_or(u32::MAX)
            };
            let request = HttpRequest {
                method: hop.method,
                url: hop.url,
                headers,
                body: hop.body,
                timeout_ms: millis(hop.timeout),
                connect_timeout_ms: millis(hop.connect_timeout),
                download_to: hop
                    .download_to
                    .map(|path| path.to_string_lossy().into_owned()),
            };
            let response = self.0.send(request).await.map_err(|error| match error {
                TransportError::Timeout { reason } => Core::Timeout(reason),
                TransportError::Connect { reason } => Core::Connect(reason),
                TransportError::Other { reason } => Core::Other(reason),
            })?;
            let headers = response.headers.into_iter();
            let headers = headers.map(|h| (h.name, h.value)).collect();
            Ok(match response.saved_size {
                Some(size) => HopResponse::saved(response.status, headers, response.url, size),
                None => HopResponse::whole(response.status, headers, response.url, response.body),
            })
        })
    }
}

static TRANSPORT: Mutex<Option<Arc<dyn Transport>>> = Mutex::new(None);

/// Sends VTOP clients made after this through `transport`; call before [`crate::Core::new`].
#[uniffi::export]
pub fn set_http_transport(transport: Arc<dyn HttpTransport>) {
    info!("http transport: foreign");
    *TRANSPORT.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(Foreign(transport)));
}

/// The transport set by [`set_http_transport`], else vtop-core's default (none in the app
/// build: vtop-core comes without reqwest).
pub(crate) fn current() -> vtop_core::VtopResult<Arc<dyn Transport>> {
    let set = TRANSPORT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    match set {
        Some(transport) => Ok(transport),
        None => vtop_core::client::default_transport(&crate::dev::vtop_config()),
    }
}

/// The intermediate CA VTOP leaves out of its chain (PEM), for the app's TLS to trust too.
#[uniffi::export]
pub fn vtop_intermediate_ca() -> String {
    vtop_core::client::VITAP_INTERMEDIATE_CA_PEM.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Answers 200 with the `x-request-id` it got as the body.
    struct Echo;

    #[async_trait::async_trait]
    impl HttpTransport for Echo {
        async fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
            vtop_core::sync::sleep(std::time::Duration::from_millis(5)).await;
            let id = request.headers.iter().find(|h| h.name == "x-request-id");
            Ok(HttpResponse {
                status: 200,
                headers: Vec::new(),
                url: request.url,
                body: id.map(|h| h.value.clone().into_bytes()).unwrap_or_default(),
                saved_size: None,
            })
        }
    }

    #[tokio::test]
    async fn each_flow_sends_its_own_request_id() {
        let transport = Foreign(Arc::new(Echo));
        let hop = || Hop {
            method: "GET".into(),
            url: "https://example.invalid/".into(),
            headers: Vec::new(),
            body: Vec::new(),
            timeout: std::time::Duration::from_secs(1),
            connect_timeout: std::time::Duration::from_secs(1),
            download_to: None,
        };
        let sent = |flow: &'static str| {
            with_flow(flow.into(), async {
                let first = transport.send(hop()).await.unwrap().bytes().await.unwrap();
                let again = transport.send(hop()).await.unwrap().bytes().await.unwrap();
                (first, again)
            })
        };
        let (a, b) = tokio::join!(sent("a"), sent("b"));
        assert_eq!(a, (b"a".to_vec(), b"a".to_vec()));
        assert_eq!(b, (b"b".to_vec(), b"b".to_vec()));
        let outside = transport.send(hop()).await.unwrap().bytes().await.unwrap();
        assert!(outside.is_empty());
    }
}

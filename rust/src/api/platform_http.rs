//! Lets vtop-core send its requests through the phone's HTTP stack (Cronet on
//! Android, NSURLSession on iOS), which uses HTTP/3 when the server offers it
//! and falls back on its own. Dart does the sending; see
//! `lib/core/utils/vtop_platform_http.dart`.

use std::sync::Arc;

use flutter_rust_bridge::DartFnFuture;
use vtop_core::client::transport::{
    Transport, TransportError, TransportErrorKind, TransportFuture, TransportRequest,
    TransportResponse,
};

/// A request for Dart to send as it is: no redirects followed, no cookies
/// added or kept.
pub struct PlatformHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub timeout_ms: u32,
}

pub enum PlatformHttpErrorKind {
    Timeout,
    /// Nothing reached the server.
    Connect,
    Other,
}

/// Dart's answer: a response, or `error` when there is none.
pub struct PlatformHttpResponse {
    pub status: u16,
    pub url: String,
    /// One entry per header line, so each `Set-Cookie` stays separate.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub error_kind: Option<PlatformHttpErrorKind>,
    pub error: Option<String>,
}

pub(crate) type PlatformSend =
    dyn Fn(PlatformHttpRequest) -> DartFnFuture<PlatformHttpResponse> + Send + Sync;

pub(crate) struct DartTransport {
    send: Box<PlatformSend>,
}

impl DartTransport {
    pub(crate) fn new(
        send: impl Fn(PlatformHttpRequest) -> DartFnFuture<PlatformHttpResponse> + Send + Sync + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            send: Box::new(send),
        })
    }
}

impl std::fmt::Debug for DartTransport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DartTransport")
    }
}

impl Transport for DartTransport {
    fn send(&self, request: TransportRequest) -> TransportFuture<'_> {
        let future = (self.send)(PlatformHttpRequest {
            method: request.method,
            url: request.url,
            headers: request.headers,
            body: request.body,
            timeout_ms: u32::try_from(request.timeout.as_millis()).unwrap_or(u32::MAX),
        });
        Box::pin(async move {
            let response = future.await;
            if let Some(kind) = response.error_kind {
                return Err(TransportError {
                    kind: match kind {
                        PlatformHttpErrorKind::Timeout => TransportErrorKind::Timeout,
                        PlatformHttpErrorKind::Connect => TransportErrorKind::Connect,
                        PlatformHttpErrorKind::Other => TransportErrorKind::Other,
                    },
                    message: response.error.unwrap_or_default(),
                });
            }
            Ok(TransportResponse {
                status: response.status,
                url: response.url,
                headers: response.headers,
                body: response.body,
            })
        })
    }
}

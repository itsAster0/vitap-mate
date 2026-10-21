//! Sending requests through an HTTP stack other than reqwest, such as the
//! phone's own (Cronet on Android, NSURLSession on iOS). Those switch to
//! HTTP/3 when the server advertises it, fall back to HTTP/2 when QUIC is
//! blocked, and keep connections across a Wi-Fi/mobile switch.
//!
//! Requests are still built with reqwest and responses read as reqwest
//! responses, so cookies, redirects and parsing stay in [`super::VtopClient`].
//! A transport only moves bytes: it must not follow redirects or keep
//! cookies of its own.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use http_body_util::BodyExt;
use reqwest::header::{HeaderMap, CONTENT_ENCODING, CONTENT_LENGTH, HOST};
use reqwest::{ResponseBuilderExt, Url};

/// A request with its body read out.
#[derive(Debug, Clone)]
pub struct TransportRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Whole-request limit, as [`crate::VtopConfig::request_timeout`].
    pub timeout: Duration,
}

/// A response with its body decoded (no `Content-Encoding` left to undo).
#[derive(Debug, Clone)]
pub struct TransportResponse {
    pub status: u16,
    /// Where the response came from; the request URL, as redirects are not
    /// followed.
    pub url: String,
    /// One entry per header line, so each `Set-Cookie` stays separate.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportErrorKind {
    Timeout,
    /// Nothing reached the server, so the request is safe to send again.
    Connect,
    Other,
}

#[derive(Debug, Clone)]
pub struct TransportError {
    pub kind: TransportErrorKind,
    pub message: String,
}

impl TransportError {
    pub fn other(message: impl Into<String>) -> Self {
        Self {
            kind: TransportErrorKind::Other,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

pub type TransportFuture<'a> =
    Pin<Box<dyn Future<Output = Result<TransportResponse, TransportError>> + Send + 'a>>;

pub trait Transport: Send + Sync + std::fmt::Debug {
    fn send(&self, request: TransportRequest) -> TransportFuture<'_>;
}

/// Reads `request` out, adding the client's default headers it lacks (reqwest
/// adds those only when it sends a request itself).
pub(super) async fn to_transport_request(
    mut request: reqwest::Request,
    defaults: &HeaderMap,
    timeout: Duration,
) -> Result<TransportRequest, TransportError> {
    let body = match request.body_mut().take() {
        None => Vec::new(),
        Some(body) => match body.as_bytes() {
            Some(bytes) => bytes.to_vec(),
            // Multipart bodies are streams.
            None => body
                .collect()
                .await
                .map_err(|error| TransportError::other(format!("request body: {error}")))?
                .to_bytes()
                .to_vec(),
        },
    };
    let mut headers = request.headers().clone();
    for (name, value) in defaults {
        if !headers.contains_key(name) {
            headers.insert(name, value.clone());
        }
    }
    // The transport sets these from the URL and the body it sends.
    headers.remove(HOST);
    headers.remove(CONTENT_LENGTH);
    Ok(TransportRequest {
        method: request.method().to_string(),
        url: request.url().to_string(),
        headers: headers
            .iter()
            .map(|(name, value)| {
                let value = String::from_utf8_lossy(value.as_bytes()).into_owned();
                (name.to_string(), value)
            })
            .collect(),
        body,
        timeout,
    })
}

pub(super) fn to_reqwest_response(
    response: TransportResponse,
) -> Result<reqwest::Response, TransportError> {
    let url = Url::parse(&response.url)
        .map_err(|error| TransportError::other(format!("response URL: {error}")))?;
    let mut builder = http::Response::builder().status(response.status).url(url);
    for (name, value) in &response.headers {
        // The transport already decoded the body.
        if name.eq_ignore_ascii_case(CONTENT_ENCODING.as_str())
            || name.eq_ignore_ascii_case(CONTENT_LENGTH.as_str())
        {
            continue;
        }
        builder = builder.header(name.as_str(), value.as_str());
    }
    builder
        .body(response.body)
        .map(reqwest::Response::from)
        .map_err(|error| TransportError::other(format!("response: {error}")))
}

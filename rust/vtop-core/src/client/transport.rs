//! One HTTP hop: what [`super::VtopClient::send_request`] hands to the
//! network. A transport never follows redirects and keeps no cookies; the
//! client adds its `Cookie` header and reads every `Set-Cookie` itself (into
//! its [`crate::session::CookieStore`]). The
//! server uses reqwest (feature `reqwest`); the app passes OkHttp through
//! `vtop-ffi`.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use vtop_proto::http::Response as ProtoResponse;

/// A request with its final headers (the defaults and `Cookie` included).
/// `Debug` shows only the method and the URL without its query: headers
/// carry cookies, the body a password or OTP, a download's query the CSRF
/// token.
pub struct Hop {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// The whole hop, body included.
    pub timeout: Duration,
    pub connect_timeout: Duration,
    /// A download: the transport may write the body to this file instead
    /// of returning it ([`HopResponse::saved`]); one that can't returns it
    /// as usual.
    pub download_to: Option<std::path::PathBuf>,
}

impl fmt::Debug for Hop {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let url = self.url.split('?').next().unwrap_or_default();
        write!(formatter, "Hop({} {url})", self.method)
    }
}

/// A hop that got no answer. The text never carries the URL's query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    Timeout(String),
    /// No connection: DNS, refused, unreachable.
    Connect(String),
    Other(String),
}

impl TransportError {
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout(_))
    }

    pub fn is_connect(&self) -> bool {
        matches!(self, Self::Connect(_))
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout(reason) | Self::Connect(reason) | Self::Other(reason) => {
                formatter.write_str(reason)
            }
        }
    }
}

impl std::error::Error for TransportError {}

/// A hop's answer as it came (a 3xx too). `url` is the URL that answered.
/// The body is in memory, or (reqwest) still on the wire, so a download
/// can stream and a head-only call can drop it unread. `Debug` shows the
/// status and the URL without its query.
pub struct HopResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub url: String,
    body: Body,
}

enum Body {
    Whole(Vec<u8>),
    /// Written to the hop's `download_to`; its size.
    Saved(u64),
    #[cfg(feature = "reqwest")]
    Stream(reqwest::Response),
}

impl fmt::Debug for HopResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let url = self.url.split('?').next().unwrap_or_default();
        write!(formatter, "HopResponse({} {url})", self.status)
    }
}

impl HopResponse {
    /// The answer of a transport that read the whole body.
    pub fn whole(status: u16, headers: Vec<(String, String)>, url: String, body: Vec<u8>) -> Self {
        Self {
            status,
            headers,
            url,
            body: Body::Whole(body),
        }
    }

    /// The answer of a transport that wrote the body, `size` bytes, to the
    /// hop's `download_to`.
    pub fn saved(status: u16, headers: Vec<(String, String)>, url: String, size: u64) -> Self {
        Self {
            status,
            headers,
            url,
            body: Body::Saved(size),
        }
    }

    /// The size of a body the transport wrote to the hop's `download_to`.
    pub fn saved_size(&self) -> Option<u64> {
        match self.body {
            Body::Saved(size) => Some(size),
            _ => None,
        }
    }

    /// The first header with this name, ignoring case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The body's size when known: a whole body's length, or the
    /// `Content-Length` of one still streaming.
    pub fn content_length(&self) -> Option<u64> {
        match &self.body {
            Body::Whole(bytes) => Some(bytes.len() as u64),
            Body::Saved(size) => Some(*size),
            #[cfg(feature = "reqwest")]
            Body::Stream(response) => response.content_length(),
        }
    }

    /// The status, headers and URL, with an empty body.
    pub fn head(&self) -> ProtoResponse {
        ProtoResponse {
            status: self.status,
            headers: self.headers.clone(),
            url: self.url.clone(),
            body: Vec::new(),
        }
    }

    /// The next part of the body; `None` at the end.
    pub async fn chunk(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
        match &mut self.body {
            Body::Whole(bytes) if bytes.is_empty() => Ok(None),
            Body::Whole(bytes) => Ok(Some(std::mem::take(bytes))),
            Body::Saved(_) => Ok(None),
            #[cfg(feature = "reqwest")]
            Body::Stream(response) => Ok(response.chunk().await?.map(|bytes| bytes.to_vec())),
        }
    }

    /// The whole body.
    pub async fn bytes(self) -> Result<Vec<u8>, TransportError> {
        match self.body {
            Body::Whole(bytes) => Ok(bytes),
            Body::Saved(_) => Ok(Vec::new()),
            #[cfg(feature = "reqwest")]
            Body::Stream(response) => Ok(response.bytes().await?.to_vec()),
        }
    }

    /// The whole answer, body read.
    pub async fn read(self) -> Result<ProtoResponse, TransportError> {
        let mut head = self.head();
        head.body = self.bytes().await?;
        Ok(head)
    }
}

pub type HopFuture<'a> =
    Pin<Box<dyn Future<Output = Result<HopResponse, TransportError>> + Send + 'a>>;

/// Sends one hop.
pub trait Transport: Send + Sync + fmt::Debug {
    fn send(&self, hop: Hop) -> HopFuture<'_>;

    /// Waits `duration` before a retry; by default on [`crate::sync::sleep`]'s timer thread.
    fn pause(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(crate::sync::sleep(duration))
    }
}

#[cfg(feature = "reqwest")]
impl From<reqwest::Error> for TransportError {
    fn from(error: reqwest::Error) -> Self {
        let (timeout, connect) = (error.is_timeout(), error.is_connect());
        let reason = error.without_url().to_string();
        if timeout {
            Self::Timeout(reason)
        } else if connect {
            Self::Connect(reason)
        } else {
            Self::Other(reason)
        }
    }
}

/// reqwest with a shared pool ([`super::http::shared_client`]). The body
/// streams.
#[cfg(feature = "reqwest")]
#[derive(Debug)]
pub(super) struct Reqwest(pub(super) reqwest::Client);

#[cfg(feature = "reqwest")]
impl Transport for Reqwest {
    fn pause(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(tokio::time::sleep(duration))
    }

    fn send(&self, hop: Hop) -> HopFuture<'_> {
        Box::pin(async move {
            let method = reqwest::Method::from_bytes(hop.method.as_bytes())
                .map_err(|_| TransportError::Other(format!("bad method {}", hop.method)))?;
            // The connect timeout is the client's (one client per timeout pair).
            let mut builder = self
                .0
                .request(method, &hop.url)
                .timeout(hop.timeout)
                .body(hop.body);
            for (name, value) in &hop.headers {
                builder = builder.header(name, value);
            }
            let response = self.0.execute(builder.build()?).await?;
            Ok(HopResponse {
                status: response.status().as_u16(),
                headers: response
                    .headers()
                    .iter()
                    .map(|(name, value)| {
                        let value = String::from_utf8_lossy(value.as_bytes()).into_owned();
                        (name.as_str().to_string(), value)
                    })
                    .collect(),
                url: response.url().to_string(),
                body: Body::Stream(response),
            })
        })
    }
}

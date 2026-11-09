//! What every VTOP request carries, and (feature `reqwest`) the server's
//! shared HTTP clients.

use std::sync::OnceLock;
#[cfg(feature = "reqwest")]
use std::{sync::Mutex, time::Duration};

#[cfg(feature = "reqwest")]
use reqwest::{Certificate, Client, ClientBuilder, Proxy};

#[cfg(feature = "reqwest")]
use crate::config::VtopConfig;
#[cfg(feature = "reqwest")]
use crate::error::{VtopError, VtopResult};

/// Intermediate CA VTOP sometimes fails to send in its chain.
pub const VITAP_INTERMEDIATE_CA_PEM: &str = include_str!("../../assets/sectigo_dv_r36.pem");

/// A proxy and the CAs it signs with, so a debugging proxy (Proxyman,
/// mitmproxy) can read the phone's traffic: the app's Developer → Inspect
/// Traffic. Never set on the engine.
#[derive(Debug, Default)]
pub struct Inspection {
    /// `host:port`.
    pub proxy: Option<String>,
    pub ca_pems: Vec<String>,
}

static INSPECTION: OnceLock<Inspection> = OnceLock::new();

/// Sends every HTTP client built after this through `inspection`. Set once,
/// before the first request; later calls are ignored.
pub fn set_inspection(inspection: Inspection) {
    let _ = INSPECTION.set(inspection);
}

/// `builder` with the [`Inspection`] proxy and CAs, when one is set.
#[cfg(feature = "reqwest")]
pub(crate) fn inspected(mut builder: ClientBuilder) -> ClientBuilder {
    let Some(inspection) = INSPECTION.get() else {
        return builder;
    };
    if let Some(proxy) = inspection
        .proxy
        .as_ref()
        .and_then(|proxy| Proxy::all(format!("http://{proxy}")).ok())
    {
        builder = builder.proxy(proxy);
    }
    for pem in &inspection.ca_pems {
        if let Ok(certificate) = Certificate::from_pem(pem.as_bytes()) {
            builder = builder.add_root_certificate(certificate);
        }
    }
    builder
}

/// Headers every VTOP request carries, names in lower case. Desktop
/// Firefox on Linux, so VTOP serves the layout the parser expects; bump
/// with Firefox.
pub(super) const DEFAULT_HEADERS: &[(&str, &str)] = &[
    (
        "user-agent",
        "Mozilla/5.0 (X11; Linux x86_64; rv:157.0) Gecko/20100101 Firefox/157.0",
    ),
    (
        "accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    ),
    ("accept-language", "en-US,en;q=0.5"),
    ("content-type", "application/x-www-form-urlencoded"),
    ("upgrade-insecure-requests", "1"),
    ("sec-fetch-dest", "document"),
    ("sec-fetch-mode", "navigate"),
    ("sec-fetch-site", "same-origin"),
    ("sec-fetch-user", "?1"),
    ("priority", "u=0, i"),
];

/// Clients that share a connection pool, one per timeout setting. Cookies
/// are never stored here: each [`super::VtopClient`] adds its own to every
/// request (see [`super::VtopClient::send_request`]), so many sessions can
/// reuse the same warm TLS connections to VTOP without seeing each other's
/// cookies.
#[cfg(feature = "reqwest")]
static SHARED: OnceLock<Mutex<Vec<(TimeoutKey, Client)>>> = OnceLock::new();

#[cfg(feature = "reqwest")]
type TimeoutKey = (Duration, Duration);

/// The shared client for `config`'s timeouts, built on first use.
#[cfg(feature = "reqwest")]
pub(super) fn shared_client(config: &VtopConfig) -> VtopResult<Client> {
    let key = (config.request_timeout, config.connect_timeout);
    let clients = SHARED.get_or_init(|| Mutex::new(Vec::new()));
    let mut clients = clients
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((_, client)) = clients.iter().find(|(existing, _)| *existing == key) {
        return Ok(client.clone());
    }
    let client = build_client(config)?;
    clients.push((key, client.clone()));
    Ok(client)
}

#[cfg(feature = "reqwest")]
fn build_client(config: &VtopConfig) -> VtopResult<Client> {
    let mut builder = Client::builder()
        // Redirects are followed by VtopClient so it can collect the
        // Set-Cookie headers of every hop into its own jar.
        .redirect(reqwest::redirect::Policy::none())
        .pool_max_idle_per_host(32)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(60))
        .tcp_nodelay(true)
        .connect_timeout(config.connect_timeout)
        .timeout(config.request_timeout);

    if let Ok(certificate) = Certificate::from_pem(VITAP_INTERMEDIATE_CA_PEM.as_bytes()) {
        builder = builder.add_root_certificate(certificate);
    }

    inspected(builder).build().map_err(|error| {
        VtopError::ConfigurationError(format!("HTTP client setup failed: {error}"))
    })
}

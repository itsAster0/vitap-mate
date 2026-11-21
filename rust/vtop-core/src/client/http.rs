//! What every VTOP request carries, and (feature `reqwest`) the server's
//! shared HTTP clients.

#[cfg(feature = "reqwest")]
use std::{
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

#[cfg(feature = "reqwest")]
use reqwest::{Client, ClientBuilder};
#[cfg(feature = "reqwest")]
use rustls::pki_types::{pem::PemObject, CertificateDer};

#[cfg(feature = "reqwest")]
use crate::config::VtopConfig;
#[cfg(feature = "reqwest")]
use crate::error::{VtopError, VtopResult};

/// Intermediate CA VTOP sometimes fails to send in its chain.
pub const VITAP_INTERMEDIATE_CA_PEM: &str = include_str!("../../assets/sectigo_dv_r36.pem");

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

/// A client on rustls with ring, trusting the bundled webpki roots and
/// `extra_roots` (PEM), as reqwest 0.12's `rustls-tls-webpki-roots` did. The
/// server images carry no system certificates, which reqwest 0.13's own
/// verifier would read. Every server client starts here.
#[cfg(feature = "reqwest")]
pub fn https_client(extra_roots: &[&str]) -> ClientBuilder {
    let mut roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    roots.add_parsable_certificates(
        extra_roots
            .iter()
            .filter_map(|pem| CertificateDer::from_pem_slice(pem.as_bytes()).ok()),
    );
    let Ok(tls) = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions() else {
        // ring supports TLS 1.2 and 1.3, so this does not happen; were it
        // to, reqwest's own TLS setup is the fallback.
        return Client::builder();
    };
    let mut tls = tls.with_root_certificates(roots).with_no_client_auth();
    // What reqwest offers by itself with `http2` on.
    tls.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Client::builder().tls_backend_preconfigured(tls)
}

#[cfg(feature = "reqwest")]
fn build_client(config: &VtopConfig) -> VtopResult<Client> {
    https_client(&[VITAP_INTERMEDIATE_CA_PEM])
        // Redirects are followed by VtopClient so it can collect the
        // Set-Cookie headers of every hop into its own jar.
        .redirect(reqwest::redirect::Policy::none())
        .pool_max_idle_per_host(32)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(60))
        .tcp_nodelay(true)
        .connect_timeout(config.connect_timeout)
        .timeout(config.request_timeout)
        .build()
        .map_err(|error| {
            VtopError::ConfigurationError(format!("HTTP client setup failed: {error}"))
        })
}

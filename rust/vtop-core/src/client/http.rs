use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, CONTENT_TYPE, USER_AGENT};
use reqwest::redirect::Policy;
use reqwest::{Certificate, Client};

use crate::config::VtopConfig;
use crate::error::{VtopError, VtopResult};

/// Intermediate CA VTOP sometimes fails to send in its chain.
const VITAP_INTERMEDIATE_CA_PEM: &str = include_str!("../../assets/sectigo_dv_r36.pem");

fn default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Linux; U; Linux x86_64; en-US) Gecko/20100101 Firefox/130.5",
        ),
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
    );
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.5"));
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    headers.insert("Upgrade-Insecure-Requests", HeaderValue::from_static("1"));
    headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("document"));
    headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("navigate"));
    headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
    headers.insert("Sec-Fetch-User", HeaderValue::from_static("?1"));
    headers.insert("Priority", HeaderValue::from_static("u=0, i"));
    headers
}

/// Clients that share a connection pool, one per timeout setting. Cookies
/// are never stored here: each [`super::VtopClient`] adds its own to every
/// request (see [`super::VtopClient::send_request`]), so many sessions can
/// reuse the same warm TLS connections to VTOP without seeing each other's
/// cookies.
static SHARED: OnceLock<Mutex<Vec<(TimeoutKey, Client)>>> = OnceLock::new();

type TimeoutKey = (Duration, Duration);

/// The shared client for `config`'s timeouts, built on first use.
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

fn build_client(config: &VtopConfig) -> VtopResult<Client> {
    let mut builder = Client::builder()
        .default_headers(default_headers())
        // Redirects are followed by VtopClient so it can collect the
        // Set-Cookie headers of every hop into its own jar.
        .redirect(Policy::none())
        .pool_max_idle_per_host(32)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(60))
        .tcp_nodelay(true)
        .connect_timeout(config.connect_timeout)
        .timeout(config.request_timeout);

    if let Ok(certificate) = Certificate::from_pem(VITAP_INTERMEDIATE_CA_PEM.as_bytes()) {
        builder = builder.add_root_certificate(certificate);
    }

    builder.build().map_err(|error| {
        VtopError::ConfigurationError(format!("HTTP client setup failed: {error}"))
    })
}

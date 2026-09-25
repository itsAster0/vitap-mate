//! Server settings, read from the environment.

use std::net::SocketAddr;
use std::time::Duration;

use vtop_core::VtopConfig;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind: SocketAddr,
    /// Accepted values of the `X-API-Key` header. Empty means the server is
    /// open and the header is not checked.
    pub api_keys: Vec<String>,
    /// Upper bound for a data request, VTOP round trips included.
    pub request_timeout: Duration,
    /// Upper bound for login and OTP requests (CAPTCHA retries take a while).
    pub login_timeout: Duration,
    /// Requests per minute allowed for each API key.
    pub rate_limit_per_minute: u32,
    /// How long a validated session stays in memory. Zero disables the cache.
    pub session_cache_ttl: Duration,
    pub session_cache_max_entries: usize,
    /// Whether the server may accept VTOP passwords at /v1/auth/*.
    pub allow_login: bool,
    /// How long a login waits for VTOP's OTP email when the caller lent a
    /// Gmail token.
    pub gmail_wait: Duration,
    /// How many VTOP requests one `/v1/refresh` runs at once.
    pub detail_concurrency: usize,
    pub vtop: VtopConfig,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConfigError(pub String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

fn parse<T: std::str::FromStr>(
    lookup: &impl Fn(&str) -> Option<String>,
    name: &str,
    default: T,
) -> Result<T, ConfigError> {
    match lookup(name) {
        None => Ok(default),
        Some(raw) => raw
            .trim()
            .parse()
            .map_err(|_| ConfigError(format!("{name} has an invalid value"))),
    }
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
    }

    /// Reads settings through `lookup`, so tests need not touch the process
    /// environment.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let bind = match lookup("VTOP_SERVER_BIND") {
            Some(bind) => bind
                .parse()
                .map_err(|_| ConfigError("VTOP_SERVER_BIND must be host:port".into()))?,
            None => {
                // Railway and most PaaS hosts hand the port over in $PORT.
                let port: u16 = parse(&lookup, "PORT", 8080)?;
                SocketAddr::from(([0, 0, 0, 0], port))
            }
        };

        let api_keys: Vec<String> = lookup("VTOP_SERVER_API_KEYS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(str::to_string)
            .collect();
        // No keys means an open server: fine on localhost or a private
        // network, risky on a public URL (see README).
        if api_keys.iter().any(|key| key.len() < 16) {
            return Err(ConfigError(
                "every API key must be at least 16 characters".into(),
            ));
        }

        let defaults = VtopConfig::default();
        let vtop = VtopConfig {
            base_url: lookup("VTOP_BASE_URL")
                .map(|url| url.trim_end_matches('/').to_string())
                .unwrap_or(defaults.base_url),
            request_timeout: Duration::from_secs(parse(&lookup, "VTOP_UPSTREAM_TIMEOUT_SECS", 30)?),
            connect_timeout: Duration::from_secs(parse(&lookup, "VTOP_CONNECT_TIMEOUT_SECS", 10)?),
        };

        Ok(Self {
            bind,
            api_keys,
            request_timeout: Duration::from_secs(parse(
                &lookup,
                "VTOP_SERVER_REQUEST_TIMEOUT_SECS",
                45,
            )?),
            login_timeout: Duration::from_secs(parse(
                &lookup,
                "VTOP_SERVER_LOGIN_TIMEOUT_SECS",
                120,
            )?),
            rate_limit_per_minute: parse(&lookup, "VTOP_SERVER_RATE_LIMIT_PER_MINUTE", 120)?,
            session_cache_ttl: Duration::from_secs(parse(
                &lookup,
                "VTOP_SERVER_SESSION_CACHE_TTL_SECS",
                300,
            )?),
            session_cache_max_entries: parse(&lookup, "VTOP_SERVER_SESSION_CACHE_MAX", 5_000)?,
            allow_login: parse(&lookup, "VTOP_SERVER_ALLOW_LOGIN", true)?,
            gmail_wait: Duration::from_secs(parse(&lookup, "VTOP_SERVER_GMAIL_WAIT_SECS", 45)?),
            detail_concurrency: parse::<usize>(&lookup, "VTOP_SERVER_CONCURRENCY", 8)?.max(1),
            vtop,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config(vars: &[(&str, &str)]) -> Result<ServerConfig, ConfigError> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        ServerConfig::from_lookup(|name| vars.get(name).cloned())
    }

    #[test]
    fn api_keys_are_optional_but_must_be_long_when_set() {
        assert!(config(&[]).unwrap().api_keys.is_empty());
        assert!(config(&[("VTOP_SERVER_API_KEYS", " , ")])
            .unwrap()
            .api_keys
            .is_empty());
        assert!(config(&[("VTOP_SERVER_API_KEYS", "short")]).is_err());
    }

    #[test]
    fn reads_port_and_key_list() {
        let config = config(&[
            ("PORT", "9000"),
            (
                "VTOP_SERVER_API_KEYS",
                " aaaaaaaaaaaaaaaa , bbbbbbbbbbbbbbbb ,",
            ),
        ])
        .unwrap();
        assert_eq!(config.bind.port(), 9000);
        assert_eq!(config.api_keys.len(), 2);
        assert!(config.allow_login);
    }

    #[test]
    fn rejects_bad_numbers() {
        assert!(config(&[
            ("VTOP_SERVER_API_KEYS", "aaaaaaaaaaaaaaaa"),
            ("VTOP_SERVER_RATE_LIMIT_PER_MINUTE", "lots"),
        ])
        .is_err());
    }
}

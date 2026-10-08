//! MCP service settings, read from the environment.

use std::net::SocketAddr;

/// Where the bridge is, and where to listen. The service holds no keys:
/// each caller brings their own access key.
#[derive(Clone)]
pub struct Config {
    /// Bridge origin, without a trailing slash.
    pub bridge_url: String,
    pub listen: SocketAddr,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConfigError(pub String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
    }

    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let bridge_url = lookup("BRIDGE_URL")
            .map(|url| url.trim().trim_end_matches('/').to_string())
            .filter(|url| !url.is_empty())
            .ok_or_else(|| ConfigError("BRIDGE_URL is required".into()))?;
        if reqwest::Url::parse(&bridge_url).is_err() {
            return Err(ConfigError("BRIDGE_URL must be a URL".into()));
        }
        let port: u16 = match lookup("PORT") {
            None => 8080,
            Some(raw) => raw
                .trim()
                .parse()
                .map_err(|_| ConfigError("PORT has an invalid value".into()))?,
        };
        Ok(Self {
            bridge_url,
            listen: SocketAddr::from((std::net::Ipv6Addr::UNSPECIFIED, port)),
        })
    }
}

//! Engine settings, read from the environment.

use std::net::SocketAddr;
use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::STANDARD};

/// No `Debug`: it holds the vault key and the shared secret.
#[derive(Clone)]
pub struct Config {
    pub vault_key: [u8; 32],
    /// Where the engine reports and checks keys: Convex (`CONVEX_URL`) for backend v3, else
    /// the Cloudflare edge (`EDGE_URL`). Both serve the same internal routes.
    pub edge_url: String,
    /// The bearer token shared with the edge, both ways (`INTERNAL_TOKEN`).
    pub internal_token: String,
    /// The UDP gateway's own token (`GATEWAY_TOKEN`), taken only on the
    /// routes it calls (page fetches and the phone's sign-in), so the public
    /// VPS can't ask whose a session is or reach the edge's internal routes.
    pub gateway_token: Option<String>,
    /// How long a sign-in waits for the forwarded OTP mail.
    pub otp_wait: Duration,
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

    /// Reads settings through `lookup`, so tests need not touch the process
    /// environment.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let required = |name: &str| {
            lookup(name)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| ConfigError(format!("{name} is required")))
        };

        let vault_key = STANDARD
            .decode(required("VAULT_KEY")?)
            .ok()
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            .ok_or_else(|| ConfigError("VAULT_KEY must be 32 bytes, base64".into()))?;

        let (name, edge_url) = ["CONVEX_URL", "EDGE_URL"]
            .into_iter()
            .find_map(|name| Some((name, required(name).ok()?)))
            .ok_or_else(|| ConfigError("CONVEX_URL or EDGE_URL is required".into()))?;
        let edge_url = edge_url.trim_end_matches('/').to_string();
        if !edge_url.starts_with("https://") && !edge_url.starts_with("http://") {
            return Err(ConfigError(format!("{name} must be an HTTP(S) URL")));
        }

        let internal_token = required("INTERNAL_TOKEN")?;
        if internal_token.len() < 32 {
            return Err(ConfigError(
                "INTERNAL_TOKEN must be at least 32 characters".into(),
            ));
        }

        let gateway_token = lookup("GATEWAY_TOKEN")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if gateway_token.as_ref().is_some_and(|token| token.len() < 32) {
            return Err(ConfigError(
                "GATEWAY_TOKEN must be at least 32 characters".into(),
            ));
        }

        let otp_wait = Duration::from_secs(parse(&lookup, "OTP_WAIT_SECS", 45)?);
        let port: u16 = parse(&lookup, "PORT", 8080)?;
        Ok(Self {
            vault_key,
            edge_url,
            internal_token,
            gateway_token,
            otp_wait,
            listen: SocketAddr::from((std::net::Ipv6Addr::UNSPECIFIED, port)),
        })
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const KEY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    const SECRET: &str = "internal-token-internal-token-internal";

    fn config(extra: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let mut vars: HashMap<&str, &str> = HashMap::from([
            ("VAULT_KEY", KEY),
            ("EDGE_URL", "https://edge.example/"),
            ("INTERNAL_TOKEN", SECRET),
        ]);
        vars.extend(extra.iter().copied());
        Config::from_lookup(|name| vars.get(name).map(|value| value.to_string()))
    }

    #[test]
    fn defaults() {
        let c = config(&[]).unwrap();
        assert_eq!(c.edge_url, "https://edge.example");
        assert_eq!(c.otp_wait, Duration::from_secs(45));
        assert_eq!(c.listen.port(), 8080);
        assert!(c.gateway_token.is_none());
    }

    #[test]
    fn overrides() {
        let c = config(&[("OTP_WAIT_SECS", "90"), ("PORT", "9000")]).unwrap();
        assert_eq!(c.otp_wait, Duration::from_secs(90));
        assert_eq!(c.listen.port(), 9000);
    }

    #[test]
    fn rejects_bad_values() {
        assert!(config(&[("VAULT_KEY", "short")]).is_err());
        assert!(config(&[("EDGE_URL", "edge.example")]).is_err());
        assert!(config(&[("CONVEX_URL", "convex.example")]).is_err());
        assert!(config(&[("INTERNAL_TOKEN", "short")]).is_err());
        assert!(config(&[("GATEWAY_TOKEN", "short")]).is_err());
        assert!(config(&[("OTP_WAIT_SECS", "x")]).is_err());
    }

    #[test]
    fn requires_the_edge() {
        let vars = HashMap::from([("VAULT_KEY", KEY)]);
        let error = Config::from_lookup(|name| vars.get(name).map(|v| v.to_string()))
            .err()
            .unwrap();
        assert_eq!(error.0, "CONVEX_URL or EDGE_URL is required");
    }

    #[test]
    fn convex_comes_before_the_edge() {
        let c = config(&[("CONVEX_URL", "https://convex.example/")]).unwrap();
        assert_eq!(c.edge_url, "https://convex.example");
    }
}

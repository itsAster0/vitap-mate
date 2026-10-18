//! Engine settings, read from the environment.

use std::net::SocketAddr;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};

/// No `Debug`: it holds the vault key and the shared secret.
#[derive(Clone)]
pub struct Config {
    pub vault_key: [u8; 32],
    /// The Cloudflare edge this engine works for.
    pub edge_url: String,
    /// The bearer secret shared with the edge (its `BRIDGE_SECRET`), both
    /// ways.
    pub mail_relay_secret: String,
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

        let edge_url = required("EDGE_URL")?.trim_end_matches('/').to_string();
        if !edge_url.starts_with("https://") && !edge_url.starts_with("http://") {
            return Err(ConfigError("EDGE_URL must be an HTTP(S) URL".into()));
        }

        let mail_relay_secret = required("MAIL_RELAY_SECRET")?;
        if mail_relay_secret.len() < 32 {
            return Err(ConfigError(
                "MAIL_RELAY_SECRET must be at least 32 characters".into(),
            ));
        }

        let otp_wait = Duration::from_secs(parse(&lookup, "OTP_WAIT_SECS", 45)?);
        let port: u16 = parse(&lookup, "PORT", 8080)?;
        Ok(Self {
            vault_key,
            edge_url,
            mail_relay_secret,
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
    const SECRET: &str = "relay-secret-relay-secret-relay-secret";

    fn config(extra: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let mut vars: HashMap<&str, &str> = HashMap::from([
            ("VAULT_KEY", KEY),
            ("EDGE_URL", "https://edge.example/"),
            ("MAIL_RELAY_SECRET", SECRET),
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
        assert!(config(&[("MAIL_RELAY_SECRET", "short")]).is_err());
        assert!(config(&[("OTP_WAIT_SECS", "x")]).is_err());
    }

    #[test]
    fn requires_the_edge() {
        let vars = HashMap::from([("VAULT_KEY", KEY)]);
        let error = Config::from_lookup(|name| vars.get(name).map(|v| v.to_string()))
            .err()
            .unwrap();
        assert_eq!(error.0, "EDGE_URL is required");
    }
}

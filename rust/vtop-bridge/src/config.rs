//! Engine settings, read from the environment.

use std::net::SocketAddr;
use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::STANDARD};

/// No `Debug`: it holds the vault key and the shared secret.
#[derive(Clone)]
pub struct Config {
    pub vault_key: [u8; 32],
    /// Where the engine reports and checks keys: Convex's HTTP actions (`CONVEX_URL`).
    pub edge_url: String,
    /// The bearer token shared with Convex, both ways (`INTERNAL_TOKEN`).
    pub internal_token: String,
    /// The UDP gateway's own token (`GATEWAY_TOKEN`), taken only on the
    /// routes it calls (page fetches and the phone's sign-in), so the public
    /// VPS can't ask whose a session is or reach Convex's internal routes.
    pub gateway_token: String,
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

        let edge_url = required("CONVEX_URL")?.trim_end_matches('/').to_string();
        if !edge_url.starts_with("https://") && !edge_url.starts_with("http://") {
            return Err(ConfigError("CONVEX_URL must be an HTTP(S) URL".into()));
        }

        let internal_token = required("INTERNAL_TOKEN")?;
        if internal_token.len() < 32 {
            return Err(ConfigError(
                "INTERNAL_TOKEN must be at least 32 characters".into(),
            ));
        }

        let gateway_token = required("GATEWAY_TOKEN")?;
        if gateway_token.len() < 32 {
            return Err(ConfigError(
                "GATEWAY_TOKEN must be at least 32 characters".into(),
            ));
        }

        let otp_wait = Duration::from_secs(parse(&lookup, "OTP_WAIT_SECS")?);
        let port: u16 = parse(&lookup, "PORT")?;
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

/// Every setting is required: the deploy's env files hold each value, so there is one place to
/// read what a service runs with.
fn parse<T: std::str::FromStr>(
    lookup: &impl Fn(&str) -> Option<String>,
    name: &str,
) -> Result<T, ConfigError> {
    lookup(name)
        .ok_or_else(|| ConfigError(format!("{name} is required")))?
        .trim()
        .parse()
        .map_err(|_| ConfigError(format!("{name} has an invalid value")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const KEY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    const SECRET: &str = "internal-token-internal-token-internal";
    const GATEWAY: &str = "gateway-token-gateway-token-gateway-tok";
    const ALL: [(&str, &str); 6] = [
        ("VAULT_KEY", KEY),
        ("CONVEX_URL", "https://convex.example/"),
        ("INTERNAL_TOKEN", SECRET),
        ("GATEWAY_TOKEN", GATEWAY),
        ("OTP_WAIT_SECS", "45"),
        ("PORT", "8080"),
    ];

    fn config(extra: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let mut vars: HashMap<&str, &str> = HashMap::from(ALL);
        vars.extend(extra.iter().copied());
        Config::from_lookup(|name| vars.get(name).map(|value| value.to_string()))
    }

    #[test]
    fn reads_every_setting() {
        let c = config(&[]).unwrap();
        assert_eq!(c.edge_url, "https://convex.example");
        assert_eq!(c.otp_wait, Duration::from_secs(45));
        assert_eq!(c.listen.port(), 8080);
        assert_eq!(c.gateway_token, GATEWAY);
    }

    #[test]
    fn rejects_bad_values() {
        assert!(config(&[("VAULT_KEY", "short")]).is_err());
        assert!(config(&[("CONVEX_URL", "convex.example")]).is_err());
        assert!(config(&[("INTERNAL_TOKEN", "short")]).is_err());
        assert!(config(&[("GATEWAY_TOKEN", "short")]).is_err());
        assert!(config(&[("OTP_WAIT_SECS", "x")]).is_err());
    }

    #[test]
    fn requires_every_setting() {
        for (missing, _) in ALL {
            let vars: HashMap<&str, &str> = ALL
                .into_iter()
                .filter(|(name, _)| *name != missing)
                .collect();
            let error = Config::from_lookup(|name| vars.get(name).map(|v| v.to_string()))
                .err()
                .unwrap();
            assert_eq!(error.0, format!("{missing} is required"));
        }
    }
}

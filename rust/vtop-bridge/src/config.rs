//! Bridge settings, read from the environment.

use std::net::SocketAddr;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};

/// No `Debug`: it holds the vault key, the service key and Google
/// credentials.
#[derive(Clone)]
pub struct Config {
    /// HTTPS origin the phone posts its callback to.
    pub public_base_url: String,
    /// Where vtop-mcp is reached; shown with new keys.
    pub public_mcp_url: String,
    /// Firebase service-account JSON.
    pub credentials_json: String,
    pub vault_key: [u8; 32],
    /// Lifetime of a cookie request.
    pub request_timeout: Duration,
    /// Lifetime of a cached VTOP session.
    pub session_cache: Duration,
    /// How long a login waits for the OTP email.
    pub gmail_wait: Duration,
    /// Allowed CORS origins. Empty means any.
    pub allowed_origins: Vec<String>,
    pub listen: SocketAddr,
    /// Poll interval suggested to the extension.
    pub poll_after: Duration,
}

pub const DEFAULT_PUBLIC_MCP_URL: &str = "https://vtop-mcp.aster0.dev/mcp";

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

        // On Railway the service's own domain is known, so PUBLIC_BASE_URL
        // only needs setting for a custom domain.
        let public_base_url = required("PUBLIC_BASE_URL")
            .or_else(|error| {
                lookup("RAILWAY_PUBLIC_DOMAIN")
                    .map(|domain| domain.trim().to_string())
                    .filter(|domain| !domain.is_empty())
                    .map(|domain| format!("https://{domain}"))
                    .ok_or(error)
            })?
            .trim_end_matches('/')
            .to_string();
        let parsed = reqwest::Url::parse(&public_base_url).ok();
        let is_origin = parsed.is_some_and(|url| {
            url.scheme() == "https"
                && url.host_str().is_some_and(|host| !host.is_empty())
                && url.query().is_none()
                && url.fragment().is_none()
        });
        if !is_origin {
            return Err(ConfigError(
                "PUBLIC_BASE_URL must be an HTTPS origin without a query or fragment".into(),
            ));
        }

        let credentials_json = required("FIREBASE_CREDENTIALS_JSON")?;
        if serde_json::from_str::<serde_json::Value>(&credentials_json).is_err() {
            return Err(ConfigError(
                "FIREBASE_CREDENTIALS_JSON must contain valid JSON".into(),
            ));
        }

        let vault_key = STANDARD
            .decode(required("VAULT_KEY")?)
            .ok()
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            .ok_or_else(|| ConfigError("VAULT_KEY must be 32 bytes, base64".into()))?;

        let request_timeout = secs(&lookup, "REQUEST_TIMEOUT_SECS", 120)?;
        if !(30..=600).contains(&request_timeout.as_secs()) {
            return Err(ConfigError(
                "REQUEST_TIMEOUT_SECS must be between 30 and 600".into(),
            ));
        }

        let port: u16 = parse(&lookup, "PORT", 8080)?;
        let allowed_origins = lookup("ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty() && *origin != "*")
            .map(String::from)
            .collect();

        let public_mcp_url = lookup("PUBLIC_MCP_URL")
            .map(|url| url.trim().to_string())
            .filter(|url| !url.is_empty())
            .unwrap_or_else(|| DEFAULT_PUBLIC_MCP_URL.to_string());
        Ok(Self {
            public_mcp_url,
            public_base_url,
            credentials_json,
            vault_key,
            request_timeout,
            session_cache: secs(&lookup, "SESSION_CACHE_SECS", 7 * 24 * 3600)?,
            gmail_wait: secs(&lookup, "GMAIL_WAIT_SECS", 45)?,
            allowed_origins,
            listen: SocketAddr::from((std::net::Ipv6Addr::UNSPECIFIED, port)),
            poll_after: Duration::from_millis(1500),
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

fn secs(
    lookup: &impl Fn(&str) -> Option<String>,
    name: &str,
    default: u64,
) -> Result<Duration, ConfigError> {
    parse(lookup, name, default).map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const KEY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

    fn config(overrides: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let mut vars: HashMap<String, String> = [
            ("PUBLIC_BASE_URL", "https://bridge.example"),
            ("FIREBASE_CREDENTIALS_JSON", "{}"),
            ("VAULT_KEY", KEY),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
        for (name, value) in overrides {
            if value.is_empty() {
                vars.remove(*name);
            } else {
                vars.insert(name.to_string(), value.to_string());
            }
        }
        Config::from_lookup(|name| vars.get(name).cloned())
    }

    #[test]
    fn defaults_apply() {
        let c = config(&[]).unwrap();
        assert_eq!(c.public_base_url, "https://bridge.example");
        assert_eq!(c.request_timeout, Duration::from_secs(120));
        assert_eq!(c.session_cache, Duration::from_secs(7 * 24 * 3600));
        assert_eq!(c.gmail_wait, Duration::from_secs(45));
        assert_eq!(c.poll_after, Duration::from_millis(1500));
        assert_eq!(c.listen.port(), 8080);
        assert_eq!(c.vault_key, [0u8; 32]);
        assert!(c.allowed_origins.is_empty());
    }

    #[test]
    fn public_base_url_falls_back_to_the_railway_domain() {
        let c = config(&[
            ("PUBLIC_BASE_URL", ""),
            (
                "RAILWAY_PUBLIC_DOMAIN",
                "vtop-bridge-production.up.railway.app",
            ),
        ])
        .unwrap();
        assert_eq!(
            c.public_base_url,
            "https://vtop-bridge-production.up.railway.app"
        );
        let explicit = config(&[("RAILWAY_PUBLIC_DOMAIN", "other.up.railway.app")]).unwrap();
        assert_eq!(explicit.public_base_url, "https://bridge.example");
        assert!(config(&[("PUBLIC_BASE_URL", "")]).is_err());
    }

    #[test]
    fn public_base_url_must_be_https_origin() {
        for bad in [
            "http://x.example",
            "https://x.example/?a=1",
            "https://x.example/#f",
            "https://",
            "",
        ] {
            assert!(config(&[("PUBLIC_BASE_URL", bad)]).is_err(), "{bad:?}");
        }
        let c = config(&[("PUBLIC_BASE_URL", "https://x.example/")]).unwrap();
        assert_eq!(c.public_base_url, "https://x.example");
    }

    #[test]
    fn request_timeout_bounds() {
        assert!(config(&[("REQUEST_TIMEOUT_SECS", "29")]).is_err());
        assert!(config(&[("REQUEST_TIMEOUT_SECS", "601")]).is_err());
        assert!(config(&[("REQUEST_TIMEOUT_SECS", "abc")]).is_err());
        let c = config(&[("REQUEST_TIMEOUT_SECS", "30")]).unwrap();
        assert_eq!(c.request_timeout, Duration::from_secs(30));
    }

    #[test]
    fn vault_key_must_be_32_bytes() {
        assert!(config(&[("VAULT_KEY", "AAAAAAAAAAAAAAAAAAAAAA==")]).is_err());
        assert!(config(&[("VAULT_KEY", "not base64!")]).is_err());
        assert!(config(&[("VAULT_KEY", "")]).is_err());
    }

    #[test]
    fn credentials_json_must_parse() {
        assert!(config(&[("FIREBASE_CREDENTIALS_JSON", "{")]).is_err());
        assert!(config(&[("FIREBASE_CREDENTIALS_JSON", "")]).is_err());
    }

    #[test]
    fn listens_on_all_ipv6_and_ipv4_addresses() {
        // Railway's private network needs IPv6; `::` also takes IPv4.
        let listen = config(&[]).unwrap().listen;
        assert_eq!(
            listen.ip(),
            std::net::IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED)
        );
    }

    #[test]
    fn port_env_sets_listen() {
        assert_eq!(config(&[("PORT", "9000")]).unwrap().listen.port(), 9000);
    }

    #[test]
    fn allowed_origins_split_on_commas() {
        let c = config(&[(
            "ALLOWED_ORIGINS",
            " chrome-extension://a , ,chrome-extension://b",
        )])
        .unwrap();
        assert_eq!(
            c.allowed_origins,
            vec!["chrome-extension://a", "chrome-extension://b"]
        );
    }

    #[test]
    fn other_durations_are_read() {
        let c = config(&[("SESSION_CACHE_SECS", "60"), ("GMAIL_WAIT_SECS", "10")]).unwrap();
        assert_eq!(c.session_cache, Duration::from_secs(60));
        assert_eq!(c.gmail_wait, Duration::from_secs(10));
    }
}

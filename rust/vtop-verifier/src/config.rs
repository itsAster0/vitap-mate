//! Verifier settings, read from the environment.

use std::net::SocketAddr;
use std::time::Duration;

/// No `Debug`: it holds the token.
#[derive(Clone)]
pub struct Config {
    /// Convex, which holds the tickets and takes the verdicts (`CONVEX_URL`).
    pub convex_url: String,
    /// The bearer token Convex takes on its two proof routes only (`VERIFIER_TOKEN`).
    pub token: String,
    /// Proofs running at once (`MAX_PROOFS`): each holds a TLS relay to VTOP and a few
    /// seconds of ZK work.
    pub max_proofs: usize,
    /// Sockets that may wait for a slot (`MAX_QUEUE`); past it, `503 proof_busy`.
    pub max_queue: usize,
    /// How long a socket waits for a slot (`QUEUE_WAIT_SECS`) before `proof_busy`.
    pub queue_wait: Duration,
    /// Proxies in front that each append the address they saw to `X-Forwarded-For`
    /// (`PROXY_HOPS`): 1 for hawk's Caddy alone. The phone's address is that many entries from
    /// the end; earlier ones it may have made up.
    pub proxy_hops: usize,
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

    /// Reads settings through `lookup`, so tests need not touch the process environment.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let required = |name: &str| {
            lookup(name)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| ConfigError(format!("{name} is required")))
        };
        let convex_url = required("CONVEX_URL")?.trim_end_matches('/').to_string();
        if !convex_url.starts_with("https://") && !convex_url.starts_with("http://") {
            return Err(ConfigError("CONVEX_URL must be an HTTP(S) URL".into()));
        }
        let token = required("VERIFIER_TOKEN")?;
        if token.len() < 32 {
            return Err(ConfigError(
                "VERIFIER_TOKEN must be at least 32 characters".into(),
            ));
        }
        let at_least_one = |name: &str| match parse(&lookup, name)? {
            0 => Err(ConfigError(format!("{name} must be at least 1"))),
            value => Ok(value),
        };
        Ok(Self {
            convex_url,
            token,
            max_proofs: at_least_one("MAX_PROOFS")?,
            max_queue: parse(&lookup, "MAX_QUEUE")?,
            queue_wait: Duration::from_secs(parse(&lookup, "QUEUE_WAIT_SECS")?),
            proxy_hops: at_least_one("PROXY_HOPS")?,
            listen: SocketAddr::from((std::net::Ipv6Addr::UNSPECIFIED, parse(&lookup, "PORT")?)),
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

    const TOKEN: &str = "verifier-token-verifier-token-verifier";

    const ALL: [(&str, &str); 7] = [
        ("CONVEX_URL", "https://convex.example/"),
        ("VERIFIER_TOKEN", TOKEN),
        ("MAX_PROOFS", "4"),
        ("MAX_QUEUE", "64"),
        ("QUEUE_WAIT_SECS", "60"),
        ("PROXY_HOPS", "1"),
        ("PORT", "8080"),
    ];

    fn config(extra: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let mut vars = HashMap::from(ALL);
        vars.extend(extra.iter().copied());
        Config::from_lookup(|name| vars.get(name).map(|v| (*v).to_string()))
    }

    #[test]
    fn reads_every_setting() {
        let c = config(&[]).unwrap();
        assert_eq!(c.convex_url, "https://convex.example");
        assert_eq!(c.token, TOKEN);
        assert_eq!((c.max_proofs, c.max_queue, c.proxy_hops), (4, 64, 1));
        assert_eq!(c.queue_wait, Duration::from_secs(60));
        assert_eq!(c.listen.port(), 8080);
        let c = config(&[
            ("MAX_PROOFS", "8"),
            ("MAX_QUEUE", "0"),
            ("QUEUE_WAIT_SECS", "5"),
        ])
        .unwrap();
        assert_eq!((c.max_proofs, c.max_queue), (8, 0));
        assert_eq!(c.queue_wait, Duration::from_secs(5));
    }

    #[test]
    fn refuses_bad_values() {
        for (name, value) in [
            ("CONVEX_URL", "convex.example"),
            ("VERIFIER_TOKEN", "short"),
            ("MAX_PROOFS", "0"),
            ("PROXY_HOPS", "0"),
            ("QUEUE_WAIT_SECS", "soon"),
        ] {
            assert!(config(&[(name, value)]).is_err(), "{name}={value}");
        }
        for (missing, _) in ALL {
            let vars: HashMap<&str, &str> = ALL
                .into_iter()
                .filter(|(name, _)| *name != missing)
                .collect();
            let error = Config::from_lookup(|name| vars.get(name).map(|v| (*v).to_string()))
                .err()
                .unwrap();
            assert_eq!(error.0, format!("{missing} is required"));
        }
    }
}

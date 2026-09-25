use std::time::Duration;

pub const DEFAULT_BASE_URL: &str = "https://vtop.vitap.ac.in";

/// Where the client talks to and how patient it is.
#[derive(Debug, Clone)]
pub struct VtopConfig {
    /// VTOP origin, without a trailing slash.
    pub base_url: String,
    /// Upper bound for a single HTTP request to VTOP, redirects included.
    pub request_timeout: Duration,
    pub connect_timeout: Duration,
}

impl Default for VtopConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            request_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
        }
    }
}

impl VtopConfig {
    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

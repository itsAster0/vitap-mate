use super::types::{PersistedCookie, PersistedHeader, PersistedVtopSession};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use reqwest::cookie::{CookieStore, Jar};
use reqwest::Url;

#[derive(Debug)]
pub struct SessionManager {
    csrf_token: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    cookie_store: Arc<Jar>,
    is_authenticated: bool,
    is_cookie_external: bool,
    persisted_headers: Vec<PersistedHeader>,
}

impl SessionManager {
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let cookie_store = Arc::new(Jar::default());

        Self {
            csrf_token: None,
            #[cfg(not(target_arch = "wasm32"))]
            cookie_store,
            is_authenticated: false,
            is_cookie_external: false,
            persisted_headers: Self::default_persisted_headers(),
        }
    }

    pub fn set_csrf_token(&mut self, token: String) {
        self.csrf_token = Some(token);
    }

    pub fn get_csrf_token(&self) -> Option<String> {
        self.csrf_token.clone()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_cookie_store(&self) -> Arc<Jar> {
        self.cookie_store.clone()
    }

    pub fn set_authenticated(&mut self, authenticated: bool) {
        self.is_authenticated = authenticated;
    }

    pub fn is_authenticated(&self) -> bool {
        self.is_authenticated
    }

    pub fn is_cookie_external(&self) -> bool {
        self.is_cookie_external
    }

    pub fn set_cookie_external(&mut self, value: bool) {
        self.is_cookie_external = value;
    }

    pub fn get_persisted_headers(&self) -> Vec<PersistedHeader> {
        if self.persisted_headers.is_empty() {
            Self::default_persisted_headers()
        } else {
            self.persisted_headers.clone()
        }
    }

    pub fn set_persisted_headers(&mut self, headers: Vec<PersistedHeader>) {
        self.persisted_headers = if headers.is_empty() {
            Self::default_persisted_headers()
        } else {
            headers
        };
    }

    pub fn clear(&mut self) {
        self.csrf_token = None;
        self.is_authenticated = false;
        self.is_cookie_external = false;
        self.persisted_headers = Self::default_persisted_headers();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.cookie_store = Arc::new(Jar::default());
        }
    }

    pub fn set_csrf_from_external(&mut self, token: String) {
        self.csrf_token = Some(token);
    }

    pub fn set_cookie_from_external(&mut self, url: String, cookie: String) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(parsed_url) = Url::parse(&url) {
                self.cookie_store.add_cookie_str(&cookie, &parsed_url);
                self.is_cookie_external = true;
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (url, cookie);
            self.is_cookie_external = true;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn import_persisted_session(&mut self, base_url: String, snapshot: PersistedVtopSession) {
        self.clear();
        self.set_persisted_headers(snapshot.headers);

        if let Some(token) = snapshot.csrf_token {
            self.csrf_token = Some(token);
        }

        let Ok(parsed_base_url) = Url::parse(&base_url) else {
            return;
        };

        for cookie in snapshot.cookies {
            self.add_persisted_cookie(&parsed_base_url, &cookie);
        }

        self.is_authenticated = false;
        self.is_cookie_external = true;
    }

    #[cfg(target_arch = "wasm32")]
    pub fn import_persisted_session(&mut self, _base_url: String, snapshot: PersistedVtopSession) {
        self.clear();
        self.set_persisted_headers(snapshot.headers);
        self.csrf_token = snapshot.csrf_token;
        self.is_authenticated = false;
        self.is_cookie_external = true;
    }

    pub fn export_persisted_session(
        &self,
        base_url: String,
        username: String,
        saved_at_epoch_ms: u64,
        expires_at_epoch_ms: u64,
    ) -> PersistedVtopSession {
        let cookies =
            self.collect_persisted_cookies(&format!("{}/vtop", base_url.trim_end_matches('/')));
        PersistedVtopSession {
            username,
            saved_at_epoch_ms,
            expires_at_epoch_ms,
            csrf_token: self.csrf_token.clone(),
            authenticated_hint: self.is_authenticated,
            cookies,
            headers: self.get_persisted_headers(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn add_persisted_cookie(&mut self, base_url: &Url, cookie: &PersistedCookie) {
        let mut cookie_line = format!("{}={}", cookie.name, cookie.value);
        if !cookie.domain.is_empty() {
            cookie_line.push_str(&format!("; Domain={}", cookie.domain));
        }
        if !cookie.path.is_empty() {
            cookie_line.push_str(&format!("; Path={}", cookie.path));
        }
        if cookie.secure {
            cookie_line.push_str("; Secure");
        }
        if cookie.http_only {
            cookie_line.push_str("; HttpOnly");
        }
        self.cookie_store.add_cookie_str(&cookie_line, base_url);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn collect_persisted_cookies(&self, base_url: &str) -> Vec<PersistedCookie> {
        let Ok(parsed_url) = Url::parse(base_url) else {
            return Vec::new();
        };
        let Some(cookie_header) = self.cookie_store.cookies(&parsed_url) else {
            return Vec::new();
        };

        let domain = parsed_url.host_str().unwrap_or_default().to_string();
        let secure = parsed_url.scheme().eq_ignore_ascii_case("https");

        cookie_header
            .to_str()
            .ok()
            .map(|raw_cookie| {
                raw_cookie
                    .split(';')
                    .filter_map(|part| {
                        let trimmed = part.trim();
                        if trimmed.is_empty() {
                            return None;
                        }
                        let (name, value) = trimmed.split_once('=')?;
                        Some(PersistedCookie {
                            name: name.trim().to_string(),
                            value: value.trim().to_string(),
                            domain: domain.clone(),
                            path: "/".to_string(),
                            expires_at_epoch_ms: None,
                            secure,
                            http_only: false,
                            same_site: None,
                            host_only: true,
                            persistent: false,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    #[cfg(target_arch = "wasm32")]
    fn collect_persisted_cookies(&self, _base_url: &str) -> Vec<PersistedCookie> {
        Vec::new()
    }

    fn default_persisted_headers() -> Vec<PersistedHeader> {
        vec![
            PersistedHeader {
                name: "User-Agent".to_string(),
                value: "Mozilla/5.0 (Linux; U; Linux x86_64; en-US) Gecko/20100101 Firefox/130.5"
                    .to_string(),
            },
            PersistedHeader {
                name: "Accept".to_string(),
                value: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
                    .to_string(),
            },
            PersistedHeader {
                name: "Accept-Language".to_string(),
                value: "en-US,en;q=0.5".to_string(),
            },
            PersistedHeader {
                name: "Content-Type".to_string(),
                value: "application/x-www-form-urlencoded".to_string(),
            },
            PersistedHeader {
                name: "Upgrade-Insecure-Requests".to_string(),
                value: "1".to_string(),
            },
            PersistedHeader {
                name: "Sec-Fetch-Dest".to_string(),
                value: "document".to_string(),
            },
            PersistedHeader {
                name: "Sec-Fetch-Mode".to_string(),
                value: "navigate".to_string(),
            },
            PersistedHeader {
                name: "Sec-Fetch-Site".to_string(),
                value: "same-origin".to_string(),
            },
            PersistedHeader {
                name: "Sec-Fetch-User".to_string(),
                value: "?1".to_string(),
            },
            PersistedHeader {
                name: "Priority".to_string(),
                value: "u=0, i".to_string(),
            },
        ]
    }
}

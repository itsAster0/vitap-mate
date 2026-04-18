use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use reqwest::cookie::Jar;
use reqwest::{cookie::CookieStore, Url};

use crate::api::vtop::types::PersistedVtopSession;

fn is_cookie_attribute_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "path"
            | "domain"
            | "expires"
            | "max-age"
            | "secure"
            | "httponly"
            | "samesite"
            | "priority"
            | "partitioned"
    )
}

fn parse_cookie_pairs(cookie_header: &str) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for part in cookie_header.split(';') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Some((raw_name, raw_value)) = trimmed.split_once('=') else {
            continue;
        };
        let name = raw_name.trim();
        let value = raw_value.trim();
        if name.is_empty() || value.is_empty() || is_cookie_attribute_name(name) {
            continue;
        }

        if let Some(index) = pairs
            .iter()
            .position(|(existing_name, _)| existing_name.eq_ignore_ascii_case(name))
        {
            pairs[index] = (name.to_string(), value.to_string());
        } else {
            pairs.push((name.to_string(), value.to_string()));
        }
    }

    pairs
}

#[derive(Debug)]
pub struct SessionManager {
    csrf_token: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    cookie_store: Arc<Jar>,
    is_authenticated: bool,
    is_cookie_external: bool,
    external_cookie_header: Option<String>,
}

impl SessionManager {
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let jar = Jar::default();
        #[cfg(not(target_arch = "wasm32"))]
        let cookie_store = Arc::new(jar);
        Self {
            csrf_token: None,
            #[cfg(not(target_arch = "wasm32"))]
            cookie_store,
            is_authenticated: false,
            is_cookie_external: false,
            external_cookie_header: None,
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
    pub fn set_cookie_external(&mut self, bool: bool) {
        self.is_cookie_external = bool;
    }

    pub fn clear(&mut self) {
        self.csrf_token = None;
        self.is_authenticated = false;
        self.external_cookie_header = None;
    }

    pub fn set_csrf_from_external(&mut self, token: String) {
        self.csrf_token = Some(token);
    }
    pub fn set_cookie_from_external(&mut self, url: String, cookie: String) {
        self.external_cookie_header = Some(cookie.clone());
        let parsed_url = Url::parse(&url).unwrap();
        let pairs = parse_cookie_pairs(&cookie);

        if pairs.is_empty() {
            self.cookie_store.add_cookie_str(&cookie, &parsed_url);
        } else {
            for (name, value) in pairs {
                self.cookie_store
                    .add_cookie_str(&format!("{name}={value}"), &parsed_url);
            }
        }
        self.is_cookie_external = true;
    }

    pub fn get_external_cookie_header(&self) -> Option<String> {
        self.external_cookie_header.clone()
    }
    pub fn get_cookie(&self, url: String) -> Option<String> {
        let k = self.cookie_store.cookies(&Url::parse(&url).unwrap());
        if let Some(cookie) = k {
            let data = cookie.as_bytes();
            return Some(String::from_utf8_lossy(&data).to_string());
        }
        None
    }
    pub fn export_persisted_session(
        &self,
        url: String,
        username: String,
        saved_at_epoch_ms: u64,
    ) -> PersistedVtopSession {
        return PersistedVtopSession {
            username,
            saved_at_epoch_ms,
            cookies: self.get_cookie(url),
        };
    }
    pub fn import_persisted_session(&mut self, url: String, session: PersistedVtopSession) {
        if let Some(cookie) = session.cookies {
            self.set_cookie_from_external(url, cookie);
            self.set_authenticated(false);
            self.set_cookie_external(true);
        }
    }
}

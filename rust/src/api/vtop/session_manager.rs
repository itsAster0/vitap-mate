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
enum SessionState {
    Anonymous { csrf_token: Option<String> },
    Restored { csrf_token: Option<String> },
    Authenticated { csrf_token: String },
}

impl SessionState {
    fn csrf_token(&self) -> Option<&str> {
        match self {
            Self::Anonymous { csrf_token } | Self::Restored { csrf_token } => csrf_token.as_deref(),
            Self::Authenticated { csrf_token } => Some(csrf_token),
        }
    }

    fn with_csrf(self, token: String) -> Self {
        match self {
            Self::Anonymous { .. } => Self::Anonymous {
                csrf_token: Some(token),
            },
            Self::Restored { .. } => Self::Restored {
                csrf_token: Some(token),
            },
            Self::Authenticated { .. } => Self::Authenticated { csrf_token: token },
        }
    }
}

#[derive(Debug)]
pub struct SessionManager {
    state: SessionState,
    #[cfg(not(target_arch = "wasm32"))]
    cookie_store: Arc<Jar>,
    external_cookie_header: Option<String>,
}

impl SessionManager {
    pub(crate) fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let jar = Jar::default();
        #[cfg(not(target_arch = "wasm32"))]
        let cookie_store = Arc::new(jar);
        Self {
            state: SessionState::Anonymous { csrf_token: None },
            #[cfg(not(target_arch = "wasm32"))]
            cookie_store,
            external_cookie_header: None,
        }
    }

    pub(crate) fn set_csrf_token(&mut self, token: String) {
        let state = std::mem::replace(
            &mut self.state,
            SessionState::Anonymous { csrf_token: None },
        );
        self.state = state.with_csrf(token);
    }

    pub(crate) fn get_csrf_token(&self) -> Option<String> {
        self.state.csrf_token().map(str::to_owned)
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn get_cookie_store(&self) -> Arc<Jar> {
        self.cookie_store.clone()
    }

    pub(crate) fn set_authenticated(&mut self, authenticated: bool) {
        let csrf_token = self.state.csrf_token().map(str::to_owned);
        self.state = if authenticated {
            match csrf_token {
                Some(csrf_token) => SessionState::Authenticated { csrf_token },
                None => SessionState::Anonymous { csrf_token: None },
            }
        } else {
            SessionState::Anonymous { csrf_token }
        };
    }

    pub(crate) fn is_authenticated(&self) -> bool {
        matches!(self.state, SessionState::Authenticated { .. })
    }

    pub(crate) fn is_cookie_external(&self) -> bool {
        matches!(self.state, SessionState::Restored { .. })
    }
    pub(crate) fn set_cookie_external(&mut self, external: bool) {
        let csrf_token = self.state.csrf_token().map(str::to_owned);
        self.state = if external {
            SessionState::Restored { csrf_token }
        } else if self.is_authenticated() {
            SessionState::Authenticated {
                csrf_token: csrf_token.expect("authenticated session contains CSRF token"),
            }
        } else {
            SessionState::Anonymous { csrf_token }
        };
    }

    pub(crate) fn clear(&mut self) {
        self.state = SessionState::Anonymous { csrf_token: None };
        self.external_cookie_header = None;
        self.cookie_store = Arc::new(Jar::default());
    }

    pub(crate) fn set_cookie_from_external(&mut self, url: String, cookie: String) {
        if cookie.trim().is_empty() {
            return;
        }
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
        let csrf_token = self.state.csrf_token().map(str::to_owned);
        self.state = SessionState::Restored { csrf_token };
    }

    pub(crate) fn get_external_cookie_header(&self) -> Option<String> {
        self.external_cookie_header.clone()
    }
    pub(crate) fn get_cookie(&self, url: String) -> Option<String> {
        let k = self.cookie_store.cookies(&Url::parse(&url).unwrap());
        if let Some(cookie) = k {
            let data = cookie.as_bytes();
            return Some(String::from_utf8_lossy(&data).to_string());
        }
        None
    }
    pub(crate) fn export_persisted_session(
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
    pub(crate) fn import_persisted_session(&mut self, url: String, session: PersistedVtopSession) {
        if let Some(cookie) = session.cookies {
            self.set_cookie_from_external(url, cookie);
            self.set_authenticated(false);
            self.set_cookie_external(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authentication_requires_a_csrf_token() {
        let mut session = SessionManager::new();
        session.set_authenticated(true);
        assert!(!session.is_authenticated());

        session.set_csrf_token("csrf".to_string());
        session.set_authenticated(true);
        assert!(session.is_authenticated());
    }

    #[test]
    fn restored_and_authenticated_are_distinct_states() {
        let mut session = SessionManager::new();
        session.set_cookie_external(true);
        assert!(session.is_cookie_external());
        assert!(!session.is_authenticated());

        session.set_csrf_token("csrf".to_string());
        session.set_authenticated(true);
        assert!(session.is_authenticated());
        assert!(!session.is_cookie_external());
    }
}

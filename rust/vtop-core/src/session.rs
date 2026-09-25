//! Cookie jar and CSRF/auth state for one VTOP session.

use std::sync::{Arc, RwLock};

use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::HeaderValue;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::inputs::RegistrationNumber;

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

/// Splits a `Cookie`/`Set-Cookie`-ish header into `name=value` pairs, dropping
/// attributes and keeping the last value for a repeated name.
pub(crate) fn parse_cookie_pairs(cookie_header: &str) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for part in cookie_header.split(';') {
        let Some((raw_name, raw_value)) = part.trim().split_once('=') else {
            continue;
        };
        let name = raw_name.trim();
        let value = raw_value.trim();
        if name.is_empty() || value.is_empty() || is_cookie_attribute_name(name) {
            continue;
        }

        if let Some(existing) = pairs
            .iter_mut()
            .find(|(existing_name, _)| existing_name.eq_ignore_ascii_case(name))
        {
            *existing = (name.to_string(), value.to_string());
        } else {
            pairs.push((name.to_string(), value.to_string()));
        }
    }
    pairs
}

/// A cookie jar that can be emptied in place.
///
/// reqwest's `Jar` has no `clear`, and the `Client` holds on to its cookie
/// provider, so without this wrapper every session reset had to build a new
/// `Client` and throw away its warm connection pool.
#[derive(Debug, Default)]
pub struct ResettableJar {
    inner: RwLock<Arc<Jar>>,
}

impl ResettableJar {
    fn current(&self) -> Arc<Jar> {
        match self.inner.read() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub fn clear(&self) {
        let fresh = Arc::new(Jar::default());
        match self.inner.write() {
            Ok(mut guard) => *guard = fresh,
            Err(poisoned) => *poisoned.into_inner() = fresh,
        }
    }

    pub fn add_cookie_str(&self, cookie: &str, url: &Url) {
        self.current().add_cookie_str(cookie, url);
    }

    pub fn header_for(&self, url: &Url) -> Option<String> {
        self.current()
            .cookies(url)
            .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
    }
}

impl CookieStore for ResettableJar {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        self.current().set_cookies(cookie_headers, url);
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        self.current().cookies(url)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AuthState {
    Anonymous {
        csrf_token: Option<String>,
    },
    /// Cookies came from outside (saved session, WebView, server request)
    /// and have not been checked against VTOP yet.
    Restored {
        csrf_token: Option<String>,
    },
    Authenticated {
        csrf_token: String,
    },
}

impl AuthState {
    fn csrf_token(&self) -> Option<&str> {
        match self {
            Self::Anonymous { csrf_token } | Self::Restored { csrf_token } => csrf_token.as_deref(),
            Self::Authenticated { csrf_token } => Some(csrf_token),
        }
    }
}

/// Auth state, CSRF token and registration number for one VTOP session.
/// The cookies themselves live in the shared [`ResettableJar`].
#[derive(Debug)]
pub struct SessionManager {
    state: AuthState,
    external_cookie_header: Option<String>,
    registration_number: Option<RegistrationNumber>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            state: AuthState::Anonymous { csrf_token: None },
            external_cookie_header: None,
            registration_number: None,
        }
    }

    pub fn set_csrf_token(&mut self, token: String) {
        self.state =
            match std::mem::replace(&mut self.state, AuthState::Anonymous { csrf_token: None }) {
                AuthState::Anonymous { .. } => AuthState::Anonymous {
                    csrf_token: Some(token),
                },
                AuthState::Restored { .. } => AuthState::Restored {
                    csrf_token: Some(token),
                },
                AuthState::Authenticated { .. } => AuthState::Authenticated { csrf_token: token },
            };
    }

    pub fn csrf_token(&self) -> Option<String> {
        self.state.csrf_token().map(str::to_owned)
    }

    /// Marks the session signed in. Without a CSRF token that is impossible,
    /// so the session falls back to anonymous.
    pub fn set_authenticated(&mut self, authenticated: bool) {
        let csrf_token = self.csrf_token();
        self.state = match (authenticated, csrf_token) {
            (true, Some(csrf_token)) => AuthState::Authenticated { csrf_token },
            (true, None) => AuthState::Anonymous { csrf_token: None },
            (false, csrf_token) => AuthState::Anonymous { csrf_token },
        };
    }

    pub fn is_authenticated(&self) -> bool {
        matches!(self.state, AuthState::Authenticated { .. })
    }

    pub fn is_cookie_external(&self) -> bool {
        matches!(self.state, AuthState::Restored { .. })
    }

    pub fn set_cookie_external(&mut self, external: bool) {
        let csrf_token = self.csrf_token();
        self.state = match (external, &self.state, csrf_token) {
            (true, _, csrf_token) => AuthState::Restored { csrf_token },
            (false, AuthState::Authenticated { .. }, Some(csrf_token)) => {
                AuthState::Authenticated { csrf_token }
            }
            (false, _, csrf_token) => AuthState::Anonymous { csrf_token },
        };
    }

    pub fn clear(&mut self) {
        self.state = AuthState::Anonymous { csrf_token: None };
        self.external_cookie_header = None;
        self.registration_number = None;
    }

    /// Remembers the raw header and marks the session restored. The caller
    /// adds the cookie pairs to the jar.
    pub fn set_external_cookie_header(&mut self, cookie: String) {
        self.external_cookie_header = Some(cookie);
        let csrf_token = self.csrf_token();
        self.state = AuthState::Restored { csrf_token };
    }

    pub fn external_cookie_header(&self) -> Option<&str> {
        self.external_cookie_header.as_deref()
    }

    pub fn set_registration_number(&mut self, registration_number: RegistrationNumber) {
        self.registration_number = Some(registration_number);
    }

    pub fn registration_number(&self) -> Option<&RegistrationNumber> {
        self.registration_number.as_ref()
    }
}

/// Everything needed to resume a signed-in (or OTP-pending) session in a
/// fresh process. The server hands this to its caller instead of storing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionState {
    /// `Cookie` header value for `{base_url}/vtop`.
    pub cookies: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub csrf_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_number: Option<String>,
    /// Unix seconds at which a security OTP was issued, when a login is
    /// waiting for one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub otp_issued_at: Option<u64>,
    /// Unix seconds of the last real login, for callers that re-login after
    /// a fixed session age.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logged_in_at: Option<u64>,
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print cookie or CSRF values.
        write!(
            formatter,
            "SessionState(cookies={}B, csrf={}, regno={}, otp_pending={})",
            self.cookies.len(),
            self.csrf_token.is_some(),
            self.registration_number.is_some(),
            self.otp_issued_at.is_some()
        )
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

    #[test]
    fn leaving_external_mode_keeps_an_authenticated_session() {
        let mut session = SessionManager::new();
        session.set_csrf_token("csrf".to_string());
        session.set_authenticated(true);
        session.set_cookie_external(false);
        assert!(session.is_authenticated());
    }

    #[test]
    fn cookie_pairs_drop_attributes_and_keep_last_value() {
        let pairs =
            parse_cookie_pairs("JSESSIONID=a; Path=/vtop; SERVERID=x; jsessionid=b; HttpOnly");
        assert_eq!(
            pairs,
            vec![
                ("jsessionid".to_string(), "b".to_string()),
                ("SERVERID".to_string(), "x".to_string()),
            ]
        );
    }

    #[test]
    fn resettable_jar_clears_in_place() {
        let jar = ResettableJar::default();
        let url = Url::parse("https://vtop.example/vtop").unwrap();
        jar.add_cookie_str("JSESSIONID=abc", &url);
        assert_eq!(jar.header_for(&url).as_deref(), Some("JSESSIONID=abc"));
        jar.clear();
        assert_eq!(jar.header_for(&url), None);
    }

    #[test]
    fn session_state_display_hides_secrets() {
        let state = SessionState {
            cookies: "JSESSIONID=secret".into(),
            csrf_token: Some("csrf-secret".into()),
            registration_number: Some("22BCE0001".into()),
            otp_issued_at: None,
            logged_in_at: None,
        };
        let shown = state.to_string();
        assert!(!shown.contains("secret"));
        assert!(!shown.contains("22BCE0001"));
    }
}

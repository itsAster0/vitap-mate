//! Cookie jar and CSRF/auth state for one VTOP session.

use std::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use crate::inputs::RegistrationNumber;
pub(crate) use vtop_proto::http::cookie_pairs as parse_cookie_pairs;
use vtop_proto::http::url_parts;

/// One stored cookie.
#[derive(Debug, Clone)]
struct Cookie {
    name: String,
    value: String,
    /// Lower case, without a leading dot.
    domain: String,
    /// No `Domain` attribute: only `domain` itself gets it.
    host_only: bool,
    path: String,
    secure: bool,
    /// Unix seconds; `None` for a session cookie.
    expires: Option<u64>,
}

impl Cookie {
    /// A `Set-Cookie` value received from `host` at `path`, as RFC 6265
    /// reads it. `None` when it is malformed or names another domain.
    fn parse(set_cookie: &str, host: &str, path: &str, now: u64) -> Option<Self> {
        let mut parts = set_cookie.split(';');
        let (name, value) = parts.next()?.split_once('=')?;
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        let mut cookie = Self {
            name: name.to_string(),
            value: value.trim().to_string(),
            domain: host.to_string(),
            host_only: true,
            path: default_path(path).to_string(),
            secure: false,
            expires: None,
        };
        let mut max_age = None;
        for attribute in parts {
            let (key, value) = attribute.split_once('=').unwrap_or((attribute, ""));
            let value = value.trim();
            match key.trim().to_ascii_lowercase().as_str() {
                "domain" if !value.is_empty() => {
                    let domain = value.trim_start_matches('.').to_ascii_lowercase();
                    if host != domain && !host.ends_with(&format!(".{domain}")) {
                        return None;
                    }
                    cookie.domain = domain;
                    cookie.host_only = false;
                }
                "path" if value.starts_with('/') => cookie.path = value.to_string(),
                "secure" => cookie.secure = true,
                "max-age" => {
                    max_age = value
                        .parse::<i64>()
                        .ok()
                        .map(|seconds| now.saturating_add_signed(seconds.max(0)))
                }
                "expires" => {
                    if let Some(at) = parse_cookie_date(value) {
                        cookie.expires = Some(at);
                    }
                }
                _ => {}
            }
        }
        // Max-Age wins over Expires.
        cookie.expires = max_age.or(cookie.expires);
        Some(cookie)
    }

    fn expired(&self, now: u64) -> bool {
        self.expires.is_some_and(|at| at <= now)
    }

    fn matches(&self, https: bool, host: &str, path: &str, now: u64) -> bool {
        let domain = host == self.domain
            || (!self.host_only && host.ends_with(&format!(".{}", self.domain)));
        let path = path == self.path
            || (path.starts_with(&self.path)
                && (self.path.ends_with('/') || path[self.path.len()..].starts_with('/')));
        domain && path && (https || !self.secure) && !self.expired(now)
    }
}

/// The directory of `path`, as RFC 6265 defaults a cookie's path.
fn default_path(path: &str) -> &str {
    match path.rfind('/') {
        Some(0) | None => "/",
        Some(at) => &path[..at],
    }
}

/// An `Expires` date (`Wed, 21 Oct 2015 07:28:00 GMT`, or with dashes) as
/// unix seconds.
fn parse_cookie_date(text: &str) -> Option<u64> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let (mut day, mut month, mut year, mut time) = (None, None, None, None);
    for token in text
        .split([' ', '-', ','])
        .filter(|token| !token.is_empty())
    {
        let lower = token.to_ascii_lowercase();
        if token.contains(':') && time.is_none() {
            let mut fields = token.split(':').map(|field| field.parse::<u64>().ok());
            time = Some((fields.next()??, fields.next()??, fields.next()??));
        } else if let Some(at) = MONTHS.iter().position(|name| lower.starts_with(name)) {
            month = Some(at as i64 + 1);
        } else if let Ok(number) = token.parse::<i64>() {
            match (token.len(), day) {
                (1 | 2, None) => day = Some(number),
                (2, Some(_)) => year = Some(number + if number < 70 { 2000 } else { 1900 }),
                (4, _) => year = Some(number),
                _ => {}
            }
        }
    }
    let (day, month, year, (hours, minutes, seconds)) = (day?, month?, year?, time?);
    // Days from civil (Howard Hinnant).
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let seconds = days * 86_400 + (hours * 3600 + minutes * 60 + seconds) as i64;
    Some(seconds.max(0) as u64)
}

/// Where a client keeps its session's cookies: a [`ResettableJar`] unless
/// the caller passes another (the app: Android's `CookieManager`, shared
/// with its web view). Values are never logged.
pub trait CookieStore: Send + Sync + std::fmt::Debug {
    /// The `Cookie` header for a request to `url`, if any cookie matches.
    fn header_for(&self, url: &str) -> Option<String>;
    /// Stores the `Set-Cookie` values `url` answered with.
    fn set_cookies(&self, url: &str, set_cookies: &[&str]);
    /// Forgets every cookie.
    fn clear(&self);
}

impl CookieStore for ResettableJar {
    fn header_for(&self, url: &str) -> Option<String> {
        ResettableJar::header_for(self, url)
    }

    fn set_cookies(&self, url: &str, set_cookies: &[&str]) {
        ResettableJar::set_cookies(self, set_cookies.iter().copied(), url);
    }

    fn clear(&self) {
        ResettableJar::clear(self);
    }
}

/// The session's cookies: name and value per domain and path, with
/// `Secure`, `Max-Age` and `Expires` honoured. Emptied in place on a reset.
#[derive(Debug, Default)]
pub struct ResettableJar {
    cookies: Mutex<Vec<Cookie>>,
}

impl ResettableJar {
    fn cookies(&self) -> MutexGuard<'_, Vec<Cookie>> {
        self.cookies
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn clear(&self) {
        self.cookies().clear();
    }

    /// Stores one `Set-Cookie` value as if `url` had sent it.
    pub fn add_cookie_str(&self, cookie: &str, url: &str) {
        self.set_cookies([cookie], url);
    }

    /// Stores the `Set-Cookie` values `url` answered with. A cookie with
    /// the same name, domain and path is replaced in place; an expired one
    /// removes it.
    pub fn set_cookies<'a>(&self, set_cookies: impl IntoIterator<Item = &'a str>, url: &str) {
        let Some((_, host, path)) = url_parts(url) else {
            return;
        };
        let now = crate::now_unix();
        let mut cookies = self.cookies();
        for cookie in set_cookies
            .into_iter()
            .filter_map(|set_cookie| Cookie::parse(set_cookie, &host, path, now))
        {
            let same = cookies.iter().position(|old| {
                old.name == cookie.name && old.domain == cookie.domain && old.path == cookie.path
            });
            match (same, cookie.expired(now)) {
                (Some(at), true) => {
                    cookies.remove(at);
                }
                (Some(at), false) => cookies[at] = cookie,
                (None, true) => {}
                (None, false) => cookies.push(cookie),
            }
        }
    }

    /// The `Cookie` header for a request to `url`, if any cookie matches.
    pub fn header_for(&self, url: &str) -> Option<String> {
        let (scheme, host, path) = url_parts(url)?;
        let https = scheme.eq_ignore_ascii_case("https");
        let now = crate::now_unix();
        let header = self
            .cookies()
            .iter()
            .filter(|cookie| cookie.matches(https, &host, path, now))
            .map(|cookie| format!("{}={}", cookie.name, cookie.value))
            .collect::<Vec<_>>()
            .join("; ");
        (!header.is_empty()).then_some(header)
    }

    /// Every live cookie as a URL and the `Set-Cookie` value that stores it
    /// there again, to move the cookies into another [`CookieStore`].
    pub fn set_cookie_lines(&self) -> Vec<(String, String)> {
        let now = crate::now_unix();
        self.cookies()
            .iter()
            .filter(|cookie| !cookie.expired(now))
            .map(|cookie| {
                let mut line = format!("{}={}; Path={}", cookie.name, cookie.value, cookie.path);
                if !cookie.host_only {
                    line.push_str(&format!("; Domain={}", cookie.domain));
                }
                if cookie.secure {
                    line.push_str("; Secure");
                }
                if let Some(at) = cookie.expires {
                    line.push_str(&format!("; Max-Age={}", at.saturating_sub(now)));
                }
                (format!("https://{}{}", cookie.domain, cookie.path), line)
            })
            .collect()
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
    fn resettable_jar_clears_in_place() {
        let jar = ResettableJar::default();
        let url = "https://vtop.example/vtop";
        jar.add_cookie_str("JSESSIONID=abc", url);
        assert_eq!(jar.header_for(url).as_deref(), Some("JSESSIONID=abc"));
        jar.clear();
        assert_eq!(jar.header_for(url), None);
    }

    #[test]
    fn jar_follows_domain_path_secure_and_expiry() {
        let jar = ResettableJar::default();
        jar.set_cookies(
            [
                "JSESSIONID=a; Path=/vtop; Secure; HttpOnly",
                "SERVERID=s1; Path=/",
                "WIDE=w; Domain=.example.in; Path=/",
                "OTHER=o; Domain=elsewhere.in",
                "GONE=g; Max-Age=0",
                "OLD=o; Expires=Thu, 01-Jan-1970 00:00:01 GMT",
                "LATER=l; Expires=Fri, 31 Dec 2100 23:59:59 GMT; Path=/vtop",
            ],
            "https://vtop.example.in/vtop/login",
        );
        assert_eq!(
            jar.header_for("https://vtop.example.in/vtop/content?x=1")
                .as_deref(),
            Some("JSESSIONID=a; SERVERID=s1; WIDE=w; LATER=l")
        );
        assert_eq!(
            jar.header_for("http://vtop.example.in/vtop").as_deref(),
            Some("SERVERID=s1; WIDE=w; LATER=l")
        );
        assert_eq!(
            jar.header_for("https://vtop.example.in/vtopx").as_deref(),
            Some("SERVERID=s1; WIDE=w")
        );
        assert_eq!(
            jar.header_for("https://mail.example.in/").as_deref(),
            Some("WIDE=w")
        );
        // Replaced in place, then deleted by an expired one.
        jar.add_cookie_str("SERVERID=s2; Path=/", "https://vtop.example.in/");
        assert!(jar
            .header_for("https://vtop.example.in/")
            .unwrap()
            .starts_with("SERVERID=s2"));
        jar.add_cookie_str(
            "SERVERID=x; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
            "https://vtop.example.in/",
        );
        assert_eq!(
            jar.header_for("https://vtop.example.in/").as_deref(),
            Some("WIDE=w")
        );
    }

    #[test]
    fn set_cookie_lines_store_the_same_cookies_again() {
        let jar = ResettableJar::default();
        let url = "https://vtop.example.in/vtop/login";
        jar.set_cookies(
            [
                "JSESSIONID=a; Path=/vtop; Secure; HttpOnly",
                "WIDE=w; Domain=.example.in; Path=/",
                "LATER=l; Expires=Fri, 31 Dec 2100 23:59:59 GMT; Path=/vtop",
                "GONE=g; Max-Age=0",
            ],
            url,
        );
        let copy = ResettableJar::default();
        for (url, line) in jar.set_cookie_lines() {
            copy.add_cookie_str(&line, &url);
        }
        for url in [
            "https://vtop.example.in/vtop/content",
            "http://vtop.example.in/vtop",
            "https://mail.example.in/",
        ] {
            assert_eq!(copy.header_for(url), jar.header_for(url), "{url}");
        }
    }

    #[test]
    fn cookie_dates_parse() {
        assert_eq!(parse_cookie_date("Thu, 01 Jan 1970 00:00:00 GMT"), Some(0));
        assert_eq!(
            parse_cookie_date("Wed, 21-Oct-15 07:28:00 GMT"),
            Some(1_445_412_480)
        );
        assert_eq!(
            parse_cookie_date("Mon, 09 Nov 2026 01:02:03 GMT"),
            Some(1_794_186_123)
        );
        assert_eq!(parse_cookie_date("soon"), None);
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

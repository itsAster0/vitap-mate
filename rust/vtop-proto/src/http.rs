//! The request a machine asks a transport to send and the response it gets
//! back. Transports never follow redirects: a 3xx comes back as is, with its
//! `Location` and every `Set-Cookie`, so the machine sees each step.

use std::fmt;

/// The multipart boundary. The fields VTOP takes (registration numbers,
/// semester ids, CSRF tokens) never contain it.
const BOUNDARY: &str = "vitapmate-7d1f0c9a5b3e";

/// The two methods VTOP uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

/// One request. `Debug` shows only the method and URL: the body can carry a
/// password or OTP and the headers a CSRF token.
#[derive(Clone, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method: Method::Get,
            url: url.into(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// A `POST` of an already encoded `application/x-www-form-urlencoded`
    /// body, sent byte for byte.
    pub fn post(url: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            method: Method::Post,
            url: url.into(),
            headers: vec![(
                "Content-Type".into(),
                "application/x-www-form-urlencoded".into(),
            )],
            body: body.into().into_bytes(),
        }
    }

    /// A `POST` with an `application/x-www-form-urlencoded` body.
    pub fn form(url: impl Into<String>, pairs: &[(&str, &str)]) -> Self {
        Self::post(url, form_encode(pairs))
    }

    /// A `POST` with a `multipart/form-data` body of text fields.
    pub fn multipart(url: impl Into<String>, fields: &[(&str, &str)]) -> Self {
        let mut body = String::new();
        for (name, value) in fields {
            body.push_str(&format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            ));
        }
        body.push_str(&format!("--{BOUNDARY}--\r\n"));
        Self {
            method: Method::Post,
            url: url.into(),
            headers: vec![(
                "Content-Type".into(),
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )],
            body: body.into_bytes(),
        }
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {}", self.method, self.url)
    }
}

/// One response, after no redirects. `url` is the URL that answered.
/// `Debug` shows only the status and URL.
#[derive(Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub url: String,
    pub body: Vec<u8>,
}

impl Response {
    /// The first header with this name, ignoring case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Every `Set-Cookie` value, in order.
    pub fn set_cookies(&self) -> impl Iterator<Item = &str> {
        self.headers
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case("set-cookie"))
            .map(|(_, value)| value.as_str())
    }

    /// Where a 3xx points, if it is one.
    pub fn redirect(&self) -> Option<&str> {
        (300..400)
            .contains(&self.status)
            .then(|| self.header("location"))
            .flatten()
    }

    /// The body as text; invalid UTF-8 becomes U+FFFD.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.status, self.url)
    }
}

/// Percent-encodes everything except `A-Z a-z 0-9 - . _ ~` (as the
/// `urlencoding` crate did, so request bodies stay byte-identical).
pub fn url_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `name=value&...` with both sides [`url_encode`]d.
pub fn form_encode(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(name, value)| format!("{}={}", url_encode(name), url_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

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
pub fn cookie_pairs(cookie_header: &str) -> Vec<(String, String)> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_encode_matches_the_urlencoding_crate() {
        for text in [
            "",
            "abc-._~XYZ09",
            "a b+c&d=e/f?g",
            "pässwörd!@#$%^*()",
            "தமிழ்",
            "\n\t\"'",
        ] {
            assert_eq!(url_encode(text), urlencoding::encode(text), "{text}");
        }
    }

    #[test]
    fn multipart_request_has_each_field_and_the_boundary() {
        let request = Request::multipart("https://vtop.example/m", &[("a", "1"), ("_csrf", "c")]);
        let body = String::from_utf8(request.body).unwrap();
        assert_eq!(
            body,
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\n1\r\n--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"_csrf\"\r\n\r\nc\r\n--{BOUNDARY}--\r\n"
            )
        );
        assert_eq!(
            request.headers[0].1,
            format!("multipart/form-data; boundary={BOUNDARY}")
        );
    }

    #[test]
    fn form_request_encodes_pairs_and_sets_the_type() {
        let request = Request::form(
            "https://vtop.example/vtop/login",
            &[("uname", "a b"), ("x", "1&2")],
        );
        assert_eq!(request.method, Method::Post);
        assert_eq!(request.body, b"uname=a%20b&x=1%262");
        assert_eq!(request.headers[0].1, "application/x-www-form-urlencoded");
    }

    #[test]
    fn debug_hides_bodies_and_headers() {
        let request = Request::form("https://vtop.example/login", &[("passwd", "secret")])
            .header("X-CSRF", "token");
        let response = Response {
            status: 302,
            headers: vec![("Set-Cookie".into(), "JSESSIONID=secret".into())],
            url: "https://vtop.example/login".into(),
            body: b"secret".to_vec(),
        };
        let shown = format!("{request:?} {response:?}");
        assert!(
            !shown.contains("secret") && !shown.contains("token"),
            "{shown}"
        );
    }

    #[test]
    fn response_reads_headers_cookies_and_redirects() {
        let response = Response {
            status: 302,
            headers: vec![
                ("set-cookie".into(), "A=1; Path=/".into()),
                ("Location".into(), "/vtop/open/page".into()),
                ("Set-Cookie".into(), "B=2".into()),
            ],
            url: "https://vtop.example/vtop/login".into(),
            body: Vec::new(),
        };
        assert_eq!(response.redirect(), Some("/vtop/open/page"));
        assert_eq!(
            response.set_cookies().collect::<Vec<_>>(),
            ["A=1; Path=/", "B=2"]
        );
        assert_eq!(
            Response {
                status: 200,
                ..response
            }
            .redirect(),
            None
        );
    }

    #[test]
    fn cookie_pairs_drop_attributes_and_keep_last_value() {
        let pairs = cookie_pairs("JSESSIONID=a; Path=/vtop; SERVERID=x; jsessionid=b; HttpOnly");
        assert_eq!(
            pairs,
            vec![
                ("jsessionid".to_string(), "b".to_string()),
                ("SERVERID".to_string(), "x".to_string()),
            ]
        );
    }
}

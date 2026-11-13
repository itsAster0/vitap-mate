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
    /// A `GET`'s head only: a file's name and type for a downloader
    /// elsewhere.
    Head,
}

/// One request. `Debug` shows only the method and the URL without its
/// query: the body can carry a password or OTP, and the headers or a
/// download's query a CSRF token.
#[derive(Clone, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Whether the transport may send it again after a failed attempt.
    /// False for actions (an outing applied twice is two outings).
    pub retry: bool,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method: Method::Get,
            url: url.into(),
            headers: Vec::new(),
            body: Vec::new(),
            retry: true,
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
            retry: true,
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
            retry: true,
        }
    }

    /// Sent at most once: see [`Request::retry`].
    pub fn once(mut self) -> Self {
        self.retry = false;
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let url = self
            .url
            .split_once('?')
            .map_or(self.url.as_str(), |(url, _)| url);
        write!(f, "{:?} {url}", self.method)
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

/// `name=value&...` as browsers encode a form (and reqwest's `.form()`):
/// `A-Z a-z 0-9 * - . _` kept, space as `+`, everything else `%XX`.
pub fn form_encode(pairs: &[(&str, &str)]) -> String {
    fn encode(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for byte in text.bytes() {
            match byte {
                b' ' => out.push('+'),
                b'*' | b'-' | b'.' | b'_' => out.push(byte as char),
                byte if byte.is_ascii_alphanumeric() => out.push(byte as char),
                byte => out.push_str(&format!("%{byte:02X}")),
            }
        }
        out
    }
    pairs
        .iter()
        .map(|(name, value)| format!("{}={}", encode(name), encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// `unix` seconds as an HTTP date, `Sun, 06 Nov 1994 08:49:37 GMT`.
pub fn http_date(unix: u64) -> String {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let (days, secs) = ((unix / 86_400) as i64, unix % 86_400);
    // Civil from days (Howard Hinnant), days since 1970-01-01.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{}, {day:02} {} {year} {:02}:{:02}:{:02} GMT",
        DAYS[(days % 7) as usize],
        MONTHS[(month - 1) as usize],
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

/// An absolute URL's scheme, host (no port, lower case) and path (no query
/// or fragment, `/` when empty). `None` for anything without `scheme://`.
pub fn url_parts(url: &str) -> Option<(&str, String, &str)> {
    let (scheme, rest) = url.split_once("://")?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..end];
    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let host = match authority.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or_default(),
        None => authority.split(':').next().unwrap_or_default(),
    };
    let rest = &rest[end..];
    let path = &rest[..rest.find(['?', '#']).unwrap_or(rest.len())];
    Some((
        scheme,
        host.to_ascii_lowercase(),
        if path.is_empty() { "/" } else { path },
    ))
}

/// Where a `Location` points from `base`: absolute, scheme-relative,
/// path-absolute, query-only or relative to `base`'s directory. Dot
/// segments are kept as they are (VTOP sends none).
pub fn join_url(base: &str, location: &str) -> Option<String> {
    let location = location.trim();
    if location.contains("://") {
        return Some(location.to_string());
    }
    let (scheme, rest) = base.split_once("://")?;
    let origin_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let origin = format!("{scheme}://{}", &rest[..origin_end]);
    let path = &rest[origin_end..];
    let path = &path[..path.find(['?', '#']).unwrap_or(path.len())];
    Some(if let Some(after) = location.strip_prefix("//") {
        format!("{scheme}://{after}")
    } else if location.starts_with('/') {
        format!("{origin}{location}")
    } else if location.starts_with('?') {
        format!("{origin}{path}{location}")
    } else {
        let dir = &path[..path.rfind('/').map_or(0, |at| at + 1)];
        let dir = if dir.is_empty() { "/" } else { dir };
        format!("{origin}{dir}{location}")
    })
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
    fn form_encode_is_the_browser_form_encoding() {
        assert_eq!(
            form_encode(&[("x", "Mon, 09 Nov 2026 01:02:03 GMT"), ("a*-._~", "ü/+")]),
            "x=Mon%2C+09+Nov+2026+01%3A02%3A03+GMT&a*-._%7E=%C3%BC%2F%2B"
        );
    }

    #[test]
    fn http_dates_match_the_httpdate_crate() {
        for unix in [0, 784_111_777, 951_782_400, 1_794_165_073, 4_102_444_799] {
            let expected = httpdate::fmt_http_date(
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(unix),
            );
            assert_eq!(http_date(unix), expected, "{unix}");
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
        assert_eq!(request.body, b"uname=a+b&x=1%262");
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
    fn urls_split_and_join_like_the_url_crate() {
        assert_eq!(
            url_parts("https://VTOP.example:8443/vtop/content?x=1#f"),
            Some(("https", "vtop.example".to_string(), "/vtop/content"))
        );
        assert_eq!(
            url_parts("http://127.0.0.1:9?a"),
            Some(("http", "127.0.0.1".to_string(), "/"))
        );
        assert_eq!(url_parts("/vtop"), None);
        let base = "https://v.example/vtop/login/x?q=1";
        for (location, joined) in [
            ("/vtop/open", "https://v.example/vtop/open"),
            ("https://o.example/a", "https://o.example/a"),
            ("//o.example/a", "https://o.example/a"),
            ("page?a=b", "https://v.example/vtop/login/page?a=b"),
            ("?z=2", "https://v.example/vtop/login/x?z=2"),
        ] {
            assert_eq!(
                join_url(base, location).as_deref(),
                Some(joined),
                "{location}"
            );
        }
        assert_eq!(join_url("http://h:1", "a").as_deref(), Some("http://h:1/a"));
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

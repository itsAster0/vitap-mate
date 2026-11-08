//! Page machines: each page or action is a value that says which request to
//! send next and, fed the response, either asks for another or is done. A
//! driver (reqwest on the server, OkHttp in the app) moves the bytes:
//!
//! ```text
//! let mut step = machine.start();
//! loop {
//!     step = match step {
//!         Step::Send(request) => machine.feed(transport.send(request)?),
//!         Step::Done(result) => break result,
//!     };
//! }
//! ```
//!
//! A machine reports a dead session as [`VtopError::SessionExpired`]; the
//! driver then signs in again and starts a new machine.

use crate::error::{VtopError, VtopResult};
use crate::http::{Request, Response};

/// What a machine wants next.
#[derive(Debug)]
pub enum Step<T> {
    Send(Request),
    Done(VtopResult<T>),
}

/// One page or action, driven to completion by [`Step`]s.
pub trait Machine {
    type Output;
    /// The first step. Called once.
    fn start(&mut self) -> Step<Self::Output>;
    /// The answer to the last [`Step::Send`].
    fn feed(&mut self, response: Response) -> Step<Self::Output>;
}

/// What a signed-in request needs from the session. `Debug` hides the CSRF
/// token and the registration number.
#[derive(Clone, PartialEq, Eq)]
pub struct Signed {
    /// `https://vtop.vitap.ac.in`, without a trailing slash.
    pub base_url: String,
    pub csrf: String,
    pub registration_number: String,
}

impl Signed {
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

impl std::fmt::Debug for Signed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Signed({})", self.base_url)
    }
}

/// One signed-in request whose page is the result, after [`signed_page`].
pub struct OnePost {
    request: Option<Request>,
}

impl OnePost {
    pub fn new(request: Request) -> Self {
        Self {
            request: Some(request),
        }
    }
}

impl Machine for OnePost {
    type Output = String;

    fn start(&mut self) -> Step<String> {
        match self.request.take() {
            Some(request) => Step::Send(request),
            None => Step::Done(Err(VtopError::ConfigurationError(
                "machine started twice".into(),
            ))),
        }
    }

    fn feed(&mut self, response: Response) -> Step<String> {
        Step::Done(signed_page(&response))
    }
}

/// The page of a response to a signed-in request. VTOP being down or slow
/// (502, 503, 504) is [`VtopError::VtopUnavailable`] and leaves the session
/// alone. A bounce to the login page, or a 4xx (how VTOP answers some dead
/// sessions), is [`VtopError::SessionExpired`]. Any other failure status is
/// [`VtopError::VtopServerError`].
pub fn signed_page(response: &Response) -> VtopResult<String> {
    match response.status {
        504 => {
            return Err(VtopError::VtopUnavailable(
                "VTOP took too long to answer".into(),
            ));
        }
        502 | 503 => return Err(VtopError::VtopUnavailable("VTOP is down right now".into())),
        _ => {}
    }
    if is_login_url(&response.url) || response.redirect().is_some_and(is_login_url) {
        return Err(VtopError::SessionExpired);
    }
    match response.status {
        200..=299 => Ok(response.text()),
        400..=499 => Err(VtopError::SessionExpired),
        status => Err(VtopError::VtopServerError(format!(
            "VTOP returned HTTP {status}"
        ))),
    }
}

/// Whether `url` (absolute or a path) is VTOP's login page.
pub fn is_login_url(url: &str) -> bool {
    let path = url
        .split_once("://")
        .map_or(url, |(_, rest)| rest.find('/').map_or("", |at| &rest[at..]));
    path.to_ascii_lowercase().starts_with("/vtop/login")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(status: u16, url: &str, location: Option<&str>) -> Response {
        Response {
            status,
            headers: location
                .map(|to| vec![("Location".into(), to.into())])
                .unwrap_or_default(),
            url: url.into(),
            body: b"<html>page</html>".to_vec(),
        }
    }

    #[test]
    fn signed_page_sorts_answers() {
        let page = "https://vtop.example/vtop/processViewTimeTable";
        assert_eq!(
            signed_page(&answer(200, page, None)).unwrap(),
            "<html>page</html>"
        );
        for expired in [
            answer(200, "https://vtop.example/vtop/login", None),
            answer(302, page, Some("/vtop/login")),
            answer(302, page, Some("https://vtop.example/vtop/login?error")),
            answer(404, page, None),
        ] {
            assert_eq!(
                signed_page(&expired),
                Err(VtopError::SessionExpired),
                "{expired:?}"
            );
        }
        assert!(matches!(
            signed_page(&answer(503, page, None)),
            Err(VtopError::VtopUnavailable(_))
        ));
        assert!(matches!(
            signed_page(&answer(504, page, None)),
            Err(VtopError::VtopUnavailable(_))
        ));
        assert!(matches!(
            signed_page(&answer(500, page, None)),
            Err(VtopError::VtopServerError(_))
        ));
    }

    #[test]
    fn login_urls() {
        assert!(is_login_url("https://vtop.example/vtop/login"));
        assert!(is_login_url("/vtop/login/error"));
        assert!(!is_login_url("https://vtop.example/vtop/content"));
        assert!(!is_login_url("https://login.example/vtop/content"));
    }

    #[test]
    fn one_post_sends_once_then_reads_the_page() {
        let mut machine = OnePost::new(Request::get("https://vtop.example/vtop/x"));
        assert!(matches!(machine.start(), Step::Send(_)));
        let done = machine.feed(answer(200, "https://vtop.example/vtop/x", None));
        assert!(matches!(done, Step::Done(Ok(ref page)) if page == "<html>page</html>"));
        assert!(matches!(machine.start(), Step::Done(Err(_))));
    }
}

//! Signing in: the [`Login`] machine (landing page, CAPTCHA, credentials)
//! and the session check on `/vtop/content`. Every request here is sent
//! once; the machine itself retries what VTOP expects to be retried (a
//! prelogin page without a CAPTCHA, a rejected CAPTCHA answer).

use crate::error::{VtopError, VtopResult};
use crate::http::{Request, Response, url_encode};
use crate::inputs::RegistrationNumber;
use crate::machine::{Machine, Step, is_login_url};
use crate::parser::page;

/// Login attempts before giving up on CAPTCHAs the solver gets wrong.
pub const MAX_CAPTCHA_ATTEMPTS: usize = 40;
/// Prelogin reloads per attempt while VTOP leaves the CAPTCHA out.
pub const MAX_PRELOGIN_RELOADS: usize = 20;

/// Reads a `data:image/...;base64,` CAPTCHA and returns its text.
pub type Solver = fn(&str) -> Result<String, String>;

/// How a sign-in ended. `Debug` hides the token and registration number.
#[derive(Clone, PartialEq, Eq)]
pub enum SignIn {
    /// Signed in; what signed-in requests need.
    SignedIn {
        csrf: String,
        registration_number: RegistrationNumber,
    },
    /// VTOP wants the emailed security OTP, issued at `issued_at` (unix
    /// seconds); `csrf` is the OTP page's token.
    OtpRequired {
        csrf: Option<String>,
        issued_at: u64,
    },
}

impl std::fmt::Debug for SignIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SignedIn { .. } => f.write_str("SignedIn"),
            Self::OtpRequired { issued_at, .. } => write!(f, "OtpRequired({issued_at})"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Landing,
    Prelogin,
    Credentials,
}

/// Signs in with a username and password. No `Debug`: it holds the
/// password.
pub struct Login {
    base_url: String,
    username: String,
    password: String,
    solve: Solver,
    stage: Stage,
    csrf: String,
    issued_at: u64,
    /// Login attempts sent (each with a fresh CAPTCHA).
    pub attempts: usize,
    /// Prelogin reloads in this attempt.
    pub reloads: usize,
}

impl Login {
    pub fn new(base_url: &str, username: &str, password: &str, solve: Solver) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
            solve,
            stage: Stage::Landing,
            csrf: String::new(),
            issued_at: 0,
            attempts: 0,
            reloads: 0,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    fn prelogin(&mut self) -> Step<SignIn> {
        self.stage = Stage::Prelogin;
        let body = format!("_csrf={}&flag=VTOP", self.csrf);
        Step::Send(Request::post(self.url("/vtop/prelogin/setup"), body).once())
    }

    fn read(&mut self, response: Response) -> VtopResult<Step<SignIn>> {
        match self.stage {
            Stage::Landing => {
                let url = self.url("/vtop/open/page");
                check_status(
                    &response,
                    &format!("VTOP returned HTTP {} at {url}", response.status),
                )?;
                let landing = response.text();
                self.csrf = page::extract_csrf_token(&landing).ok_or_else(|| {
                    VtopError::VtopServerError(format!(
                        "VTOP's sign-in page loaded without a security token ({} characters). This is usually a temporary VTOP or network problem — please try again.",
                        landing.chars().count()
                    ))
                })?;
                Ok(self.prelogin())
            }
            Stage::Prelogin => {
                check_status(
                    &response,
                    &format!(
                        "VTOP returned HTTP {} while preparing captcha",
                        response.status
                    ),
                )?;
                let text = response.text();
                if !text.contains("base64,") {
                    self.reloads += 1;
                    if self.reloads >= MAX_PRELOGIN_RELOADS {
                        return Err(VtopError::CaptchaRequired);
                    }
                    return Ok(self.prelogin());
                }
                let captcha =
                    page::extract_captcha_data(&text).ok_or(VtopError::CaptchaRequired)?;
                let answer = (self.solve)(&captcha).map_err(VtopError::ConfigurationError)?;
                self.stage = Stage::Credentials;
                self.attempts += 1;
                // VTOP's clock just before the POST that makes it mail the OTP, so the code's
                // mail (dated by VTOP too) can be told from an earlier sign-in's.
                self.issued_at = response.server_time();
                let body = format!(
                    "_csrf={}&username={}&password={}&captchaStr={}",
                    self.csrf,
                    url_encode(&self.username),
                    url_encode(&self.password),
                    answer
                );
                Ok(Step::Send(
                    Request::post(self.url("/vtop/login"), body).once(),
                ))
            }
            Stage::Credentials => {
                let text = response.text();
                if !response.url.contains("error") {
                    let (csrf, registration_number) = page::extract_session_fields(&text);
                    let csrf = csrf.ok_or_else(|| {
                        VtopError::ParseError(
                            "VTOP's response after sign-in was missing its session token".into(),
                        )
                    })?;
                    let registration_number = RegistrationNumber::parse(
                        &registration_number.ok_or(VtopError::RegistrationParsingError)?,
                    )?;
                    return Ok(Step::Done(Ok(SignIn::SignedIn {
                        csrf,
                        registration_number,
                    })));
                }
                if text.contains("Invalid Captcha") {
                    if self.attempts >= MAX_CAPTCHA_ATTEMPTS {
                        return Err(VtopError::AuthenticationFailed(
                            "We could not complete sign-in after several captcha attempts. Please try again."
                                .to_string(),
                        ));
                    }
                    self.reloads = 0;
                    return Ok(self.prelogin());
                }
                if page::is_security_otp_required_response(&text) {
                    return Ok(Step::Done(Ok(SignIn::OtpRequired {
                        csrf: page::extract_csrf_token(&text),
                        issued_at: self.issued_at,
                    })));
                }
                if page::is_invalid_credentials_response(&text) {
                    return Err(VtopError::InvalidCredentials);
                }
                Err(VtopError::AuthenticationFailed(page::login_page_error(
                    &text,
                )))
            }
        }
    }
}

impl Machine for Login {
    type Output = SignIn;

    fn start(&mut self) -> Step<SignIn> {
        self.stage = Stage::Landing;
        Step::Send(Request::get(self.url("/vtop/open/page")).once())
    }

    fn feed(&mut self, response: Response) -> Step<SignIn> {
        self.read(response)
            .unwrap_or_else(|error| Step::Done(Err(error)))
    }
}

/// VTOP being down or slow as [`VtopError::VtopUnavailable`], any other
/// failure status as [`VtopError::VtopServerError`] with `detail`.
fn check_status(response: &Response, detail: &str) -> VtopResult<()> {
    match response.status {
        200..=299 => Ok(()),
        504 => Err(VtopError::VtopUnavailable(
            "VTOP took too long to answer".into(),
        )),
        502 | 503 => Err(VtopError::VtopUnavailable("VTOP is down right now".into())),
        _ => Err(VtopError::VtopServerError(detail.to_string())),
    }
}

/// The session check: `/vtop/content`, with `cookie` (a whole `Cookie`
/// header) when the session came from outside the transport's jar.
pub fn content(base_url: &str, cookie: Option<&str>) -> Request {
    let request = Request::get(format!("{}/vtop/content", base_url.trim_end_matches('/'))).once();
    match cookie {
        Some(cookie) => request.header("Cookie", cookie),
        None => request,
    }
}

/// Reads [`content`]'s answer: `Ok(Some)` with the session's token and
/// registration number, `Ok(None)` when the session is dead. A 5xx is an
/// error when `server_error_is_error` (the caller must not drop a session
/// over VTOP being down), otherwise a dead session.
pub fn read_content(
    response: &Response,
    server_error_is_error: bool,
) -> VtopResult<Option<(String, RegistrationNumber)>> {
    let login = is_login_url(&response.url) || response.redirect().is_some_and(is_login_url);
    if server_error_is_error && response.status >= 500 && !login {
        check_status(response, &format!("HTTP {}", response.status))?;
    }
    if login || !(200..300).contains(&response.status) {
        return Ok(None);
    }
    let (csrf, registration_number) = page::extract_session_fields(&response.text());
    let csrf = csrf
        .ok_or_else(|| VtopError::ParseError("VTOP's session page was missing its token".into()))?;
    let registration_number = RegistrationNumber::parse(
        &registration_number.ok_or(VtopError::RegistrationParsingError)?,
    )?;
    Ok(Some((csrf, registration_number)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LANDING: &str = r#"<input type="hidden" name="_csrf" value="csrf-1">"#;
    const CAPTCHA: &str = r#"<img class="form-control img-fluid bg-light border-0" src="data:image/jpeg;base64,AAAA">"#;
    const CONTENT: &str = r#"<html><input name="_csrf" value="csrf-2">
<input type="hidden" name="authorizedIDX" value="22BCE0001"></html>"#;

    fn solve(_: &str) -> Result<String, String> {
        Ok("ABC12".into())
    }

    fn answer(url: &str, html: &str) -> Response {
        Response {
            status: 200,
            headers: Vec::new(),
            url: url.into(),
            body: html.as_bytes().to_vec(),
        }
    }

    fn body(step: &Step<SignIn>) -> String {
        match step {
            Step::Send(request) => String::from_utf8(request.body.clone()).unwrap(),
            Step::Done(done) => panic!("expected a request, got {done:?}"),
        }
    }

    fn login() -> Login {
        Login::new("https://vtop.example/", "user name", "p@ss&word", solve)
    }

    #[test]
    fn signs_in_through_landing_captcha_and_credentials() {
        let mut machine = login();
        assert!(
            matches!(machine.start(), Step::Send(ref r) if r.url == "https://vtop.example/vtop/open/page" && !r.retry)
        );
        let step = machine.feed(answer("https://vtop.example/vtop/open/page", LANDING));
        assert_eq!(body(&step), "_csrf=csrf-1&flag=VTOP");
        let step = machine.feed(answer("https://vtop.example/vtop/prelogin/setup", CAPTCHA));
        assert_eq!(
            body(&step),
            "_csrf=csrf-1&username=user%20name&password=p%40ss%26word&captchaStr=ABC12"
        );
        let done = machine.feed(answer("https://vtop.example/vtop/content", CONTENT));
        let Step::Done(Ok(SignIn::SignedIn {
            csrf,
            registration_number,
        })) = done
        else {
            panic!("{done:?}")
        };
        assert_eq!(
            (csrf.as_str(), registration_number.as_str()),
            ("csrf-2", "22BCE0001")
        );
    }

    #[test]
    fn reloads_prelogin_until_the_captcha_comes() {
        let mut machine = login();
        machine.start();
        machine.feed(answer("https://vtop.example/vtop/open/page", LANDING));
        for _ in 0..MAX_PRELOGIN_RELOADS - 1 {
            let step = machine.feed(answer(
                "https://vtop.example/vtop/prelogin/setup",
                "<html></html>",
            ));
            assert_eq!(body(&step), "_csrf=csrf-1&flag=VTOP");
        }
        let step = machine.feed(answer(
            "https://vtop.example/vtop/prelogin/setup",
            "<html></html>",
        ));
        assert!(matches!(step, Step::Done(Err(VtopError::CaptchaRequired))));
    }

    #[test]
    fn a_rejected_captcha_tries_again_then_gives_up() {
        let mut machine = login();
        machine.start();
        machine.feed(answer("https://vtop.example/vtop/open/page", LANDING));
        machine.feed(answer("https://vtop.example/vtop/prelogin/setup", CAPTCHA));
        for attempt in 1..MAX_CAPTCHA_ATTEMPTS {
            let step = machine.feed(answer(
                "https://vtop.example/vtop/login/error",
                "Invalid Captcha",
            ));
            assert_eq!(body(&step), "_csrf=csrf-1&flag=VTOP", "attempt {attempt}");
            machine.feed(answer("https://vtop.example/vtop/prelogin/setup", CAPTCHA));
        }
        let step = machine.feed(answer(
            "https://vtop.example/vtop/login/error",
            "Invalid Captcha",
        ));
        assert!(matches!(
            step,
            Step::Done(Err(VtopError::AuthenticationFailed(_)))
        ));
        assert_eq!(machine.attempts, MAX_CAPTCHA_ATTEMPTS);
    }

    #[test]
    fn landing_without_a_token_is_a_server_error() {
        let mut machine = login();
        machine.start();
        let step = machine.feed(answer(
            "https://vtop.example/vtop/open/page",
            "<html></html>",
        ));
        assert!(matches!(
            step,
            Step::Done(Err(VtopError::VtopServerError(_)))
        ));
        let mut machine = login();
        machine.start();
        let down = Response {
            status: 503,
            ..answer("https://vtop.example/vtop/open/page", "")
        };
        assert!(matches!(
            machine.feed(down),
            Step::Done(Err(VtopError::VtopUnavailable(_)))
        ));
    }

    #[test]
    fn content_reads_or_expires() {
        let request = content("https://vtop.example", Some("JSESSIONID=x"));
        assert_eq!(
            request.headers.last().unwrap(),
            &("Cookie".to_string(), "JSESSIONID=x".to_string())
        );
        let (csrf, registration_number) =
            read_content(&answer("https://vtop.example/vtop/content", CONTENT), true)
                .unwrap()
                .unwrap();
        assert_eq!(
            (csrf.as_str(), registration_number.as_str()),
            ("csrf-2", "22BCE0001")
        );
        assert_eq!(
            read_content(&answer("https://vtop.example/vtop/login", ""), true),
            Ok(None)
        );
        let down = Response {
            status: 500,
            ..answer("https://vtop.example/vtop/content", "")
        };
        assert!(read_content(&down, true).is_err());
        assert_eq!(read_content(&down, false), Ok(None));
    }

    #[test]
    fn debug_hides_the_session() {
        let shown = format!(
            "{:?}",
            SignIn::SignedIn {
                csrf: "secret".into(),
                registration_number: RegistrationNumber::parse("22BCE0001").unwrap()
            }
        );
        assert_eq!(shown, "SignedIn");
    }
}

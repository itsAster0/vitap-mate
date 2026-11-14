//! The emailed security OTP after [`crate::login::Login`] ends in
//! `OtpRequired`: [`SubmitOtp`] sends the code and confirms the session,
//! [`ResendOtp`] asks VTOP to mail a new one. Every request is sent once.

use crate::error::{VtopError, VtopResult};
use crate::http::{Request, Response};
use crate::inputs::{OtpCode, RegistrationNumber};
use crate::login::{SignIn, content, read_content};
use crate::machine::{Machine, Step, is_login_url};
use crate::parser::page;

/// VTOP has used both spellings.
const RESEND_PATHS: [&str; 2] = ["/vtop/resendSecurityOtp", "/vtop/resendSecurityOTP"];

/// The JSON VTOP answers OTP requests with, when it does.
#[derive(Debug, Default, PartialEq, Eq)]
struct OtpReply {
    status: Option<String>,
    message: Option<String>,
    redirect_url: Option<String>,
}

impl OtpReply {
    fn parse(body: &str) -> Self {
        if !body.trim_start().starts_with('{') {
            return Self::default();
        }
        Self {
            status: json_string(body, "status").map(|status| status.to_uppercase()),
            message: json_string(body, "message"),
            redirect_url: json_string(body, "redirectUrl").filter(|url| !url.trim().is_empty()),
        }
    }

    fn is_json(&self) -> bool {
        self.status.is_some() || self.message.is_some() || self.redirect_url.is_some()
    }
}

/// The string value of `"key": "..."` in a flat JSON object, unescaped.
/// Enough for VTOP's OTP replies; not a general JSON reader.
fn json_string(body: &str, key: &str) -> Option<String> {
    let quoted = format!("\"{key}\"");
    let mut rest = &body[body.find(&quoted)? + quoted.len()..];
    rest = rest
        .trim_start()
        .strip_prefix(':')?
        .trim_start()
        .strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    out.push(
                        char::from_u32(u32::from_str_radix(&hex, 16).ok()?).unwrap_or('\u{fffd}'),
                    );
                }
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

/// An OTP-page request: multipart, with the headers VTOP's own page sends.
fn otp_post(base_url: &str, path: &str, fields: &[(&str, &str)]) -> Request {
    Request::multipart(format!("{base_url}{path}"), fields)
        .header("Accept", "*/*")
        .header("Origin", base_url)
        .header("Referer", format!("{base_url}/vtop/login/error"))
        .header("Sec-Fetch-Dest", "empty")
        .header("Sec-Fetch-Mode", "cors")
        .header("Sec-Fetch-Site", "same-origin")
        .once()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Submit,
    Redirect,
    CheckAfterRedirect,
    Check,
}

/// What the submit answer said, kept for the steps after it.
#[derive(Default)]
struct Answer {
    reply: OtpReply,
    status: u16,
    url: String,
    text: String,
}

impl Answer {
    fn declares_success(&self) -> bool {
        matches!(
            self.reply.status.as_deref(),
            Some("VALID" | "SUCCESS" | "OK")
        ) || matches!(
            self.text.trim().to_uppercase().as_str(),
            "VALID" | "SUCCESS" | "OK"
        )
    }
}

/// Sends the OTP and confirms the signed-in session. No `Debug`: it holds
/// the code and the token.
pub struct SubmitOtp {
    base_url: String,
    csrf: String,
    code: OtpCode,
    stage: Stage,
    answer: Answer,
}

impl SubmitOtp {
    /// `csrf` is the OTP page's token, from [`SignIn::OtpRequired`].
    pub fn new(base_url: &str, csrf: &str, code: OtpCode) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            csrf: csrf.to_string(),
            code,
            stage: Stage::Submit,
            answer: Answer::default(),
        }
    }

    fn check(&mut self, stage: Stage) -> Step<SignIn> {
        self.stage = stage;
        Step::Send(content(&self.base_url, None))
    }

    /// After the submit answer (and its redirect, if any) without a session.
    fn after_answer(&mut self) -> VtopResult<Step<SignIn>> {
        let answer = &self.answer;
        if let Some(status) = answer.reply.status.as_deref()
            && !matches!(status, "VALID" | "SUCCESS" | "OK")
        {
            return Err(failed(
                answer.reply.message.clone(),
                "Verification failed. Please try again.",
            ));
        }
        let landed_on_content = (200..300).contains(&answer.status)
            && answer.url.trim_end_matches('/').ends_with("/vtop/content");
        if landed_on_content {
            let (csrf, registration_number) = page::extract_session_fields(&answer.text);
            let registration_number = RegistrationNumber::parse(
                &registration_number.ok_or(VtopError::RegistrationParsingError)?,
            )?;
            let csrf = csrf.unwrap_or_else(|| self.csrf.clone());
            return Ok(Step::Done(Ok(SignIn::SignedIn {
                csrf,
                registration_number,
            })));
        }
        Ok(self.check(Stage::Check))
    }

    fn read(&mut self, response: Response) -> VtopResult<Step<SignIn>> {
        match self.stage {
            Stage::Submit => {
                let text = response.text();
                let reply = OtpReply::parse(&text);
                let status = reply.status.clone();
                if status.as_deref() == Some("INVALID") || text.to_lowercase().contains("invalid otp") {
                    return Err(failed(reply.message, "Invalid OTP. Please try again."));
                }
                if status.as_deref() == Some("EXPIRED") {
                    return Err(failed(reply.message, "OTP has expired. Please resend."));
                }
                let redirect = reply.redirect_url.clone().filter(|_| status.as_deref() == Some("SUCCESS"));
                self.answer = Answer { reply, status: response.status, url: response.url.clone(), text };
                match redirect {
                    Some(redirect) => {
                        self.stage = Stage::Redirect;
                        let url = if redirect.contains("://") {
                            redirect
                        } else {
                            format!("{}/{}", self.base_url, redirect.trim_start_matches('/'))
                        };
                        Ok(Step::Send(Request::get(url).header("Accept", "*/*").once()))
                    }
                    None => self.after_answer(),
                }
            }
            Stage::Redirect => {
                if is_login_url(&response.url) || response.redirect().is_some_and(is_login_url) {
                    return self.after_answer();
                }
                match response.status {
                    200..=299 => {}
                    504 => return Err(VtopError::VtopUnavailable("VTOP took too long to answer".into())),
                    502 | 503 => return Err(VtopError::VtopUnavailable("VTOP is down right now".into())),
                    status => {
                        return Err(VtopError::VtopServerError(format!(
                            "VTOP returned HTTP {status} at {}",
                            response.url
                        )));
                    }
                }
                let (csrf, registration_number) = page::extract_session_fields(&response.text());
                RegistrationNumber::parse(&registration_number.ok_or(VtopError::RegistrationParsingError)?)?;
                if let Some(csrf) = csrf {
                    self.csrf = csrf;
                }
                Ok(self.check(Stage::CheckAfterRedirect))
            }
            Stage::CheckAfterRedirect => match read_content(&response, false) {
                Ok(Some((csrf, registration_number))) => {
                    Ok(Step::Done(Ok(SignIn::SignedIn { csrf, registration_number })))
                }
                _ => self.after_answer(),
            },
            Stage::Check => match read_content(&response, false) {
                Ok(Some((csrf, registration_number))) => {
                    Ok(Step::Done(Ok(SignIn::SignedIn { csrf, registration_number })))
                }
                _ if self.answer.declares_success() => Err(VtopError::AuthenticationFailed(
                    "VTOP accepted the OTP, but did not open a signed-in session. Please try again."
                        .to_string(),
                )),
                _ => Err(VtopError::AuthenticationFailed(
                    "We could not verify that OTP. Please try again.".to_string(),
                )),
            },
        }
    }
}

impl Machine for SubmitOtp {
    type Output = SignIn;

    fn start(&mut self) -> Step<SignIn> {
        self.stage = Stage::Submit;
        let request = otp_post(
            &self.base_url,
            "/vtop/validateSecurityOtp",
            &[("otpCode", self.code.as_str()), ("_csrf", &self.csrf)],
        );
        Step::Send(request)
    }

    fn feed(&mut self, response: Response) -> Step<SignIn> {
        self.read(response)
            .unwrap_or_else(|error| Step::Done(Err(error)))
    }
}

/// Asks VTOP to mail a new OTP, trying both resend paths.
pub struct ResendOtp {
    base_url: String,
    csrf: String,
    path: usize,
}

impl ResendOtp {
    pub fn new(base_url: &str, csrf: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            csrf: csrf.to_string(),
            path: 0,
        }
    }

    fn send(&self) -> Step<u64> {
        Step::Send(otp_post(
            &self.base_url,
            RESEND_PATHS[self.path],
            &[("_csrf", &self.csrf)],
        ))
    }
}

impl Machine for ResendOtp {
    /// When VTOP mailed the new code, by its clock.
    type Output = u64;

    fn start(&mut self) -> Step<u64> {
        self.path = 0;
        self.send()
    }

    fn feed(&mut self, response: Response) -> Step<u64> {
        const FAILED: &str = "Failed to resend OTP. Please try again.";
        if response.status == 404 {
            self.path += 1;
            if self.path < RESEND_PATHS.len() {
                return self.send();
            }
            return Step::Done(Err(failed(None, FAILED)));
        }
        let text = response.text();
        let reply = OtpReply::parse(&text);
        if reply.is_json() {
            if matches!(
                reply.status.as_deref(),
                Some("SUCCESS" | "SENT" | "VALID" | "OK")
            ) {
                return Step::Done(Ok(response.server_time()));
            }
            return Step::Done(Err(failed(reply.message, FAILED)));
        }
        let lower = text.to_lowercase();
        if (200..300).contains(&response.status) && lower.contains("otp") && lower.contains("sent")
        {
            return Step::Done(Ok(response.server_time()));
        }
        Step::Done(Err(failed(None, FAILED)))
    }
}

fn failed(message: Option<String>, otherwise: &str) -> VtopError {
    VtopError::AuthenticationFailed(message.unwrap_or_else(|| otherwise.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTENT: &str = r#"<html><input name="_csrf" value="csrf-2">
<input type="hidden" name="authorizedIDX" value="22BCE0001"></html>"#;

    fn answer(status: u16, url: &str, body: &str) -> Response {
        Response {
            status,
            headers: Vec::new(),
            url: url.into(),
            body: body.as_bytes().to_vec(),
        }
    }

    fn submit() -> SubmitOtp {
        SubmitOtp::new(
            "https://vtop.example",
            "csrf-1",
            OtpCode::parse("123456").unwrap(),
        )
    }

    fn url(step: &Step<SignIn>) -> String {
        match step {
            Step::Send(request) => request.url.clone(),
            Step::Done(done) => panic!("expected a request, got {done:?}"),
        }
    }

    #[test]
    fn otp_reply_reads_json_and_ignores_html() {
        let reply = OtpReply::parse(
            r#"{"status":"success","message":"o\"k!","redirectUrl":"/vtop/content"}"#,
        );
        assert_eq!(reply.status.as_deref(), Some("SUCCESS"));
        assert_eq!(reply.message.as_deref(), Some("o\"k!"));
        assert_eq!(reply.redirect_url.as_deref(), Some("/vtop/content"));
        assert!(!OtpReply::parse("<html>\"status\":\"x\"</html>").is_json());
        assert_eq!(
            OtpReply::parse(r#"{"redirectUrl" : "  "}"#).redirect_url,
            None
        );
    }

    #[test]
    fn submit_sends_the_code_once_with_the_page_headers() {
        let Step::Send(request) = submit().start() else {
            panic!()
        };
        assert!(!request.retry);
        assert_eq!(request.url, "https://vtop.example/vtop/validateSecurityOtp");
        let body = String::from_utf8(request.body.clone()).unwrap();
        assert!(
            body.contains("name=\"otpCode\"\r\n\r\n123456\r\n")
                && body.contains("name=\"_csrf\"\r\n\r\ncsrf-1\r\n")
        );
        assert!(request.headers.contains(&(
            "Referer".into(),
            "https://vtop.example/vtop/login/error".into()
        )));
    }

    #[test]
    fn success_follows_the_redirect_then_checks_the_session() {
        let mut machine = submit();
        machine.start();
        let step = machine.feed(answer(
            200,
            "https://vtop.example/vtop/validateSecurityOtp",
            r#"{"status":"SUCCESS","redirectUrl":"/vtop/content"}"#,
        ));
        assert_eq!(url(&step), "https://vtop.example/vtop/content");
        let step = machine.feed(answer(200, "https://vtop.example/vtop/content", CONTENT));
        assert_eq!(url(&step), "https://vtop.example/vtop/content");
        let done = machine.feed(answer(200, "https://vtop.example/vtop/content", CONTENT));
        assert!(
            matches!(done, Step::Done(Ok(SignIn::SignedIn { ref csrf, .. })) if csrf == "csrf-2")
        );
    }

    #[test]
    fn wrong_and_expired_codes_say_so() {
        for (body, message) in [
            (
                r#"{"status":"INVALID","message":"Wrong code"}"#,
                "Wrong code",
            ),
            ("Invalid OTP", "Invalid OTP. Please try again."),
            (r#"{"status":"EXPIRED"}"#, "OTP has expired. Please resend."),
            (
                r#"{"status":"LOCKED"}"#,
                "Verification failed. Please try again.",
            ),
        ] {
            let mut machine = submit();
            machine.start();
            let done = machine.feed(answer(
                200,
                "https://vtop.example/vtop/validateSecurityOtp",
                body,
            ));
            assert!(
                matches!(done, Step::Done(Err(VtopError::AuthenticationFailed(ref m))) if m == message),
                "{body}"
            );
        }
    }

    #[test]
    fn an_unconfirmed_success_is_an_error() {
        let mut machine = submit();
        machine.start();
        let step = machine.feed(answer(
            200,
            "https://vtop.example/vtop/validateSecurityOtp",
            "OK",
        ));
        assert_eq!(url(&step), "https://vtop.example/vtop/content");
        let done = machine.feed(answer(200, "https://vtop.example/vtop/login", ""));
        assert!(
            matches!(done, Step::Done(Err(VtopError::AuthenticationFailed(ref m))) if m.starts_with("VTOP accepted"))
        );
    }

    #[test]
    fn landing_on_content_signs_in_without_another_request() {
        let mut machine = submit();
        machine.start();
        let done = machine.feed(answer(200, "https://vtop.example/vtop/content/", CONTENT));
        assert!(matches!(done, Step::Done(Ok(SignIn::SignedIn { .. }))));
    }

    #[test]
    fn resend_tries_the_other_spelling_on_404() {
        let mut machine = ResendOtp::new("https://vtop.example", "c");
        let Step::Send(first) = machine.start() else {
            panic!()
        };
        assert!(first.url.ends_with("/vtop/resendSecurityOtp"));
        let Step::Send(second) = machine.feed(answer(404, &first.url, "")) else {
            panic!()
        };
        assert!(second.url.ends_with("/vtop/resendSecurityOTP"));
        // When VTOP mailed it, by VTOP's clock.
        let mut sent = answer(200, &second.url, r#"{"status":"sent"}"#);
        sent.headers
            .push(("Date".into(), "Sat, 14 Nov 2026 15:24:48 GMT".into()));
        assert!(matches!(machine.feed(sent), Step::Done(Ok(1_794_669_888))));
        let mut machine = ResendOtp::new("https://vtop.example", "c");
        machine.start();
        assert!(matches!(
            machine.feed(answer(200, "u", "<html>error</html>")),
            Step::Done(Err(_))
        ));
    }
}

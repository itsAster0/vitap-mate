//! Small extractors for the login and landing pages: CSRF token,
//! registration number, CAPTCHA image and login error banner.

use std::sync::LazyLock;

use scraper::{Html, Selector};

use super::selector;

static CSRF_INPUT: LazyLock<Selector> = LazyLock::new(|| selector("input[name='_csrf']"));
static REGISTRATION_INPUT: LazyLock<Selector> =
    LazyLock::new(|| selector("input[type=hidden][name=authorizedIDX]"));
static CAPTCHA_IMAGE: LazyLock<Selector> =
    LazyLock::new(|| selector("img.form-control.img-fluid.bg-light.border-0"));
static LOGIN_ALERT: LazyLock<Selector> =
    LazyLock::new(|| selector(r#"span.text-danger.text-center[role="alert"]"#));

/// Reads `var <name> = "<value>"` (either quote style) out of inline script.
pub fn extract_javascript_var(page: &str, name: &str) -> Option<String> {
    let marker = format!("var {name}");
    let after_marker = page.split(&marker).nth(1)?;
    let after_equals = after_marker.split_once('=')?.1.trim_start();
    let quote = after_equals.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    after_equals[quote.len_utf8()..]
        .split_once(quote)
        .map(|(value, _)| value.to_string())
}

pub fn extract_csrf_token(page: &str) -> Option<String> {
    let document = Html::parse_document(page);
    document
        .select(&CSRF_INPUT)
        .next()
        .and_then(|element| element.value().attr("value"))
        .map(str::to_string)
        .or_else(|| extract_javascript_var(page, "csrfValue"))
}

pub fn extract_registration_number(page: &str) -> Option<String> {
    let document = Html::parse_document(page);
    document
        .select(&REGISTRATION_INPUT)
        .next()
        .and_then(|element| element.value().attr("value"))
        .map(str::to_string)
        .or_else(|| extract_javascript_var(page, "id"))
}

/// The CSRF token and registration number from one parse of a signed-in page.
pub fn extract_session_fields(page: &str) -> (Option<String>, Option<String>) {
    let document = Html::parse_document(page);
    let csrf = document
        .select(&CSRF_INPUT)
        .next()
        .and_then(|element| element.value().attr("value"))
        .map(str::to_string)
        .or_else(|| extract_javascript_var(page, "csrfValue"));
    let registration = document
        .select(&REGISTRATION_INPUT)
        .next()
        .and_then(|element| element.value().attr("value"))
        .map(str::to_string)
        .or_else(|| extract_javascript_var(page, "id"));
    (csrf, registration)
}

/// The `data:image/...;base64,...` CAPTCHA source on the prelogin page.
pub fn extract_captcha_data(page: &str) -> Option<String> {
    let document = Html::parse_document(page);
    document
        .select(&CAPTCHA_IMAGE)
        .next()
        .and_then(|element| element.value().attr("src"))
        .filter(|src| src.contains("base64,"))
        .map(str::to_string)
}

/// Text of the red alert banner on the login page, whitespace-collapsed.
pub fn login_alert_message(page: &str) -> Option<String> {
    let document = Html::parse_document(page);
    document.select(&LOGIN_ALERT).next().map(|element| {
        element
            .text()
            .flat_map(str::split_whitespace)
            .collect::<Vec<_>>()
            .join(" ")
    })
}

pub fn login_page_error(page: &str) -> String {
    match login_alert_message(page) {
        Some(message) if message.is_empty() => "We could not sign you in. Please try again.".into(),
        Some(message) => message,
        None => "We could not sign you in because VTOP returned an unexpected response.".into(),
    }
}

pub fn is_invalid_credentials_response(page: &str) -> bool {
    let Some(message) = login_alert_message(page) else {
        return false;
    };
    let message = message.to_lowercase();
    message.contains("invalid username/password") || message.contains("invalid loginid/password")
}

pub fn is_security_otp_required_response(page: &str) -> bool {
    page.to_lowercase()
        .contains("otp sent to your registered email.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csrf_comes_from_input_or_script() {
        assert_eq!(
            extract_csrf_token(r#"<input name="_csrf" value="abc">"#).as_deref(),
            Some("abc")
        );
        assert_eq!(
            extract_csrf_token(r#"<script>var csrfValue = 'xyz';</script>"#).as_deref(),
            Some("xyz")
        );
        assert_eq!(extract_csrf_token("<p>nothing</p>"), None);
    }

    #[test]
    fn registration_number_comes_from_hidden_input_or_script() {
        assert_eq!(
            extract_registration_number(
                r#"<input type="hidden" name="authorizedIDX" value="22BCE0001">"#
            )
            .as_deref(),
            Some("22BCE0001")
        );
        assert_eq!(
            extract_registration_number(r#"<script>var id = "22BCE0002";</script>"#).as_deref(),
            Some("22BCE0002")
        );
    }

    #[test]
    fn session_fields_come_from_one_parse() {
        let page = r#"<input name="_csrf" value="abc"><input type="hidden" name="authorizedIDX" value="22BCE0001">"#;
        assert_eq!(
            extract_session_fields(page),
            (Some("abc".into()), Some("22BCE0001".into()))
        );
    }

    #[test]
    fn captcha_must_be_inline_base64() {
        let page = r#"<img class="form-control img-fluid bg-light border-0" src="data:image/jpeg;base64,AAAA">"#;
        assert_eq!(
            extract_captcha_data(page).as_deref(),
            Some("data:image/jpeg;base64,AAAA")
        );
        let remote = r#"<img class="form-control img-fluid bg-light border-0" src="/captcha.jpg">"#;
        assert_eq!(extract_captcha_data(remote), None);
    }

    #[test]
    fn login_errors_are_classified() {
        let invalid = r#"<span class="text-danger text-center" role="alert">
            Invalid  LoginId/Password </span>"#;
        assert!(is_invalid_credentials_response(invalid));
        assert_eq!(login_page_error(invalid), "Invalid LoginId/Password");
        assert!(!is_invalid_credentials_response("<p></p>"));
        assert!(is_security_otp_required_response(
            "<p>OTP sent to your registered email.</p>"
        ));
        assert_eq!(
            login_page_error("<p></p>"),
            "We could not sign you in because VTOP returned an unexpected response."
        );
    }
}

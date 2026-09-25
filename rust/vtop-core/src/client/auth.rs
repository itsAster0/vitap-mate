//! Password + CAPTCHA login and session validation.

use log::Level;

use super::{
    missing_csrf_error, network_error, server_error, AuthStage, Credentials, SendWith, VtopClient,
};
use crate::error::{VtopError, VtopResult};
use crate::inputs::RegistrationNumber;
use crate::now_unix;
use crate::parser::page;

const MAX_CAPTCHA_ATTEMPTS: usize = 40;
const MAX_PRELOGIN_RELOADS: usize = 20;

impl VtopClient {
    fn begin_auth_flow(&mut self, reason: &str) {
        self.auth_flow_id = self.auth_flow_id.saturating_add(1);
        self.state().auth_stage = AuthStage::LoggingIn;
        self.auth_log(Level::Info, "start", reason);
    }

    fn finish_login(&self, message: &str) {
        let mut state = self.state();
        state.session.set_authenticated(true);
        state.session.set_cookie_external(false);
        state.auth_stage = AuthStage::Idle;
        state.logged_in_at = Some(crate::now_unix());
        drop(state);
        self.auth_log(Level::Info, "finish", message);
    }

    /// Signs in, reusing restored cookies when VTOP still accepts them and
    /// otherwise running the CAPTCHA login. Fails with
    /// [`VtopError::OTPRequired`] when VTOP wants the emailed code; follow up
    /// with [`VtopClient::submit_security_otp`].
    pub async fn login(&mut self) -> VtopResult<()> {
        let credentials = self.credentials.clone().ok_or_else(|| {
            VtopError::ConfigurationError("this client has no credentials to log in with".into())
        })?;
        self.begin_auth_flow("login requested");

        if self.state().session.is_cookie_external() {
            self.auth_log(
                Level::Info,
                "session.restore",
                "validating restored session before fresh login",
            );
            match self.try_restore_existing_session().await {
                Ok(true) => {
                    self.state().auth_stage = AuthStage::Idle;
                    return Ok(());
                }
                Ok(false) => self.auth_log(
                    Level::Warn,
                    "session.restore",
                    "restored session could not be verified, falling back to full login",
                ),
                Err(error) => self.auth_log(
                    Level::Warn,
                    "session.restore",
                    format!(
                        "restored session validation failed, falling back to fresh login: {error}"
                    ),
                ),
            }
        }

        for attempt in 0..MAX_CAPTCHA_ATTEMPTS {
            self.auth_log(
                Level::Info,
                "login.attempt",
                format!(
                    "starting login attempt {} of {MAX_CAPTCHA_ATTEMPTS}",
                    attempt + 1
                ),
            );
            let captcha_data = self.load_login_page(attempt == 0).await?;
            let captcha_answer = self.solve_captcha(captcha_data).await?;
            match self.perform_login(&credentials, &captcha_answer).await {
                Ok(()) => {
                    self.finish_login("login completed successfully");
                    return Ok(());
                }
                Err(VtopError::AuthenticationFailed(message))
                    if message.contains("Invalid Captcha") =>
                {
                    self.auth_log(
                        Level::Warn,
                        "captcha",
                        "captcha answer was rejected, retrying login",
                    );
                }
                Err(error) => return Err(error),
            }
        }

        self.auth_log(
            Level::Error,
            "finish",
            "exhausted captcha attempts without completing sign-in",
        );
        Err(VtopError::AuthenticationFailed(
            "We could not complete sign-in after several captcha attempts. Please try again."
                .to_string(),
        ))
    }

    async fn perform_login(
        &self,
        credentials: &Credentials,
        captcha_answer: &str,
    ) -> VtopResult<()> {
        let csrf = self
            .state()
            .session
            .csrf_token()
            .ok_or_else(|| missing_csrf_error("perform_login"))?;

        let login_data = format!(
            "_csrf={}&username={}&password={}&captchaStr={}",
            csrf,
            urlencoding::encode(credentials.username.as_str()),
            urlencoding::encode(credentials.password.as_str()),
            captcha_answer
        );
        let url = self.config.url("/vtop/login");

        Self::log_request("perform_login.send", "POST", &url);
        let issued_at = now_unix();
        let response = self
            .http
            .post(&url)
            .body(login_data)
            .send_with(self)
            .await
            .map_err(|error| network_error("perform_login.send", error))?;
        let response_url = response.url().to_string();
        let response_text = response
            .text()
            .await
            .map_err(|error| network_error("perform_login.text", error))?;

        if !response_url.contains("error") {
            let (csrf, registration_number) = page::extract_session_fields(&response_text);
            let csrf = csrf.ok_or_else(|| VtopError::ParseError("CSRF token not found".into()))?;
            let registration_number = RegistrationNumber::parse(
                &registration_number.ok_or(VtopError::RegistrationParsingError)?,
            )?;
            let mut state = self.state();
            state.session.set_csrf_token(csrf);
            state.session.set_registration_number(registration_number);
            drop(state);
            self.auth_log(
                Level::Info,
                "login",
                "credentials accepted and session page loaded",
            );
            return Ok(());
        }

        if response_text.contains("Invalid Captcha") {
            return Err(VtopError::AuthenticationFailed(
                "Invalid Captcha".to_string(),
            ));
        }
        if page::is_security_otp_required_response(&response_text) {
            let mut state = self.state();
            if let Some(csrf) = page::extract_csrf_token(&response_text) {
                state.session.set_csrf_token(csrf);
            }
            state.auth_stage = AuthStage::AwaitingOtp {
                issued_at_unix_seconds: issued_at,
            };
            drop(state);
            self.auth_log(Level::Info, "otp", "security OTP verification is required");
            return Err(VtopError::OTPRequired(
                "Additional verification is required.".to_string(),
                issued_at,
            ));
        }
        if page::is_invalid_credentials_response(&response_text) {
            self.auth_log(
                Level::Warn,
                "login",
                "login rejected due to invalid credentials",
            );
            return Err(VtopError::InvalidCredentials);
        }
        Err(VtopError::AuthenticationFailed(page::login_page_error(
            &response_text,
        )))
    }

    /// Loads the prelogin page until it carries a CAPTCHA and returns the
    /// image data URL. The first attempt also opens a fresh VTOP session to
    /// get a CSRF token.
    async fn load_login_page(&self, open_session: bool) -> VtopResult<String> {
        if open_session {
            let landing = self.load_initial_page().await?;
            let csrf = page::extract_csrf_token(&landing)
                .ok_or_else(|| VtopError::ParseError("CSRF token not found".to_string()))?;
            self.state().session.set_csrf_token(csrf);
        }
        let csrf = self
            .state()
            .session
            .csrf_token()
            .ok_or_else(|| missing_csrf_error("load_login_page"))?;
        let url = self.config.url("/vtop/prelogin/setup");
        let body = format!("_csrf={csrf}&flag=VTOP");

        for attempt in 0..MAX_PRELOGIN_RELOADS {
            Self::log_request("load_login_page.send", "POST", &url);
            let response = self
                .http
                .post(&url)
                .body(body.clone())
                .send_with(self)
                .await
                .map_err(|error| network_error("load_login_page.send", error))?;
            let status = response.status();
            if !status.is_success() {
                return Err(server_error(
                    "load_login_page",
                    format!("VTOP returned HTTP {status} while preparing captcha"),
                ));
            }
            let text = response
                .text()
                .await
                .map_err(|error| network_error("load_login_page.text", error))?;
            if text.contains("base64,") {
                return page::extract_captcha_data(&text).ok_or(VtopError::CaptchaRequired);
            }
            self.auth_log(
                Level::Warn,
                "captcha",
                format!(
                    "captcha payload was missing on attempt {} of {MAX_PRELOGIN_RELOADS}, retrying login page load",
                    attempt + 1
                ),
            );
        }

        self.auth_log(
            Level::Error,
            "captcha",
            "captcha payload was missing after all login page reload attempts",
        );
        Err(VtopError::CaptchaRequired)
    }

    async fn load_initial_page(&self) -> VtopResult<String> {
        let url = self.config.url("/vtop/open/page");
        Self::log_request("load_initial_page.send", "GET", &url);
        let response = self
            .http
            .get(&url)
            .send_with(self)
            .await
            .map_err(|error| network_error("load_initial_page.send", error))?;
        let status = response.status();
        if !status.is_success() {
            return Err(server_error(
                "load_initial_page",
                format!("VTOP returned HTTP {status} at {url}"),
            ));
        }
        response
            .text()
            .await
            .map_err(|error| network_error("load_initial_page.text", error))
    }

    /// Loads `/vtop/content` with the current cookies. On success, stores the
    /// page's CSRF token and registration number and marks the session
    /// signed in. A bounce to the login page clears the session and returns
    /// `Ok(false)`.
    pub(super) async fn validate_authenticated_session(&self) -> VtopResult<bool> {
        self.log_cookie_names("validate_authenticated_session.before_request");
        let url = self.config.url("/vtop/content");
        Self::log_request("validate_authenticated_session.send", "GET", &url);
        let mut request = self.http.get(&url);
        let external_header = self
            .state()
            .session
            .external_cookie_header()
            .map(str::to_owned);
        if let Some(raw_cookie_header) = external_header {
            request = request.header(reqwest::header::COOKIE, raw_cookie_header);
        }
        let response = request
            .send_with(self)
            .await
            .map_err(|error| network_error("validate_authenticated_session.send", error))?;
        let final_url = response.url().to_string();
        let status = response.status();
        if Self::is_login_url(&final_url) || !status.is_success() {
            self.mark_session_expired(
                "validate_authenticated_session",
                &format!("VTOP returned HTTP {status} at {final_url}"),
            );
            return Ok(false);
        }
        let content = response
            .text()
            .await
            .map_err(|error| network_error("validate_authenticated_session.text", error))?;

        let (csrf, registration_number) = page::extract_session_fields(&content);
        let csrf = csrf.ok_or_else(|| VtopError::ParseError("CSRF token not found".into()))?;
        let registration_number = RegistrationNumber::parse(
            &registration_number.ok_or(VtopError::RegistrationParsingError)?,
        )?;
        let mut state = self.state();
        state.session.set_csrf_token(csrf);
        state.session.set_registration_number(registration_number);
        state.session.set_authenticated(true);
        Ok(true)
    }

    /// True when the session is signed in, validating restored cookies with
    /// VTOP first if needed.
    pub(super) async fn ensure_authenticated_session(&self) -> VtopResult<bool> {
        if self.has_usable_session() {
            return Ok(true);
        }
        let _validation = self.validation.lock().await;
        // Another request may have validated while this one waited.
        if self.has_usable_session() {
            return Ok(true);
        }
        let (authenticated, external) = {
            let state = self.state();
            (
                state.session.is_authenticated(),
                state.session.is_cookie_external(),
            )
        };
        if authenticated {
            log::info!(
                target: super::AUTH,
                "authenticated session is missing CSRF token or registration number, refreshing from VTOP content"
            );
            return self.validate_authenticated_session().await;
        }
        if !external {
            return Ok(false);
        }
        self.try_restore_existing_session().await
    }

    fn has_usable_session(&self) -> bool {
        let state = self.state();
        state.session.is_authenticated()
            && state.session.csrf_token().is_some()
            && state.session.registration_number().is_some()
    }

    async fn try_restore_existing_session(&self) -> VtopResult<bool> {
        if matches!(self.validate_authenticated_session().await, Ok(true)) {
            self.state().session.set_cookie_external(false);
            self.auth_log(
                Level::Info,
                "session.restore",
                "restored session validation succeeded",
            );
            return Ok(true);
        }

        self.auth_log(
            Level::Warn,
            "session.restore",
            "restored session was not accepted by VTOP",
        );
        self.reset_session_state();
        Ok(false)
    }
}

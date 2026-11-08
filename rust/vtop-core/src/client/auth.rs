//! Password + CAPTCHA login and session validation.

use log::Level;

use super::{network_error, send_error, status_error, AuthStage, SendWith, VtopClient};
use crate::error::{VtopError, VtopResult};
use crate::inputs::RegistrationNumber;
use crate::login::{Login, SignIn};
use crate::parser::page;

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

        let mut machine = Login::new(
            &self.config.base_url,
            credentials.username.as_str(),
            credentials.password.as_str(),
            vtop_proto::captcha::solve_data_url,
        );
        let result = self.run("login", &mut machine).await;
        self.auth_log(
            Level::Info,
            "login.attempt",
            format!(
                "{} login attempt(s), {} prelogin reload(s) in the last",
                machine.attempts, machine.reloads
            ),
        );
        match result? {
            SignIn::SignedIn {
                csrf,
                registration_number,
            } => {
                let mut state = self.state();
                state.session.set_csrf_token(csrf);
                state.session.set_registration_number(registration_number);
                drop(state);
                self.finish_login("login completed successfully");
                Ok(())
            }
            SignIn::OtpRequired { csrf, issued_at } => {
                let mut state = self.state();
                if let Some(csrf) = csrf {
                    state.session.set_csrf_token(csrf);
                }
                state.auth_stage = AuthStage::AwaitingOtp {
                    issued_at_unix_seconds: issued_at,
                };
                drop(state);
                self.auth_log(Level::Info, "otp", "security OTP verification is required");
                Err(VtopError::OTPRequired(
                    "Additional verification is required.".to_string(),
                    issued_at,
                ))
            }
        }
    }

    pub async fn verify_session(&self) -> VtopResult<String> {
        let _validation = self.validation.lock().await;
        if !self.check_authenticated_session(true).await? {
            return Err(VtopError::SessionExpired);
        }
        self.registration_number()
    }

    /// Loads `/vtop/content` with the current cookies. On success, stores the
    /// page's CSRF token and registration number and marks the session
    /// signed in. A bounce to the login page clears the session and returns
    /// `Ok(false)`.
    pub(super) async fn validate_authenticated_session(&self) -> VtopResult<bool> {
        self.check_authenticated_session(false).await
    }

    /// As [`Self::validate_authenticated_session`]; with
    /// `server_error_is_error`, an HTTP 5xx is `VtopServerError` instead of
    /// an expired session, so callers can tell an outage from a dead login.
    async fn check_authenticated_session(&self, server_error_is_error: bool) -> VtopResult<bool> {
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
            .map_err(|error| send_error("validate_authenticated_session.send", error))?;
        let final_url = response.url().to_string();
        let status = response.status();
        if server_error_is_error && status.is_server_error() && !Self::is_login_url(&final_url) {
            return Err(status_error(
                "validate_authenticated_session",
                status,
                format!("HTTP {status}"),
            ));
        }
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
        let csrf = csrf.ok_or_else(|| {
            VtopError::ParseError("VTOP's session page was missing its token".into())
        })?;
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

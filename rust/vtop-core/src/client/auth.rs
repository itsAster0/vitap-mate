//! Password + CAPTCHA login and session validation.

use log::Level;

use super::{AuthStage, VtopClient, network_error, send_error};
use crate::error::{VtopError, VtopResult};
use crate::login::{self, Login, SignIn};

/// Resets `AuthStage::LoggingIn` back to `Idle` if `login` exits via an error or cancellation.
struct LoggingInGuard<'a> {
    client: &'a VtopClient,
    active: bool,
}

impl<'a> LoggingInGuard<'a> {
    fn new(client: &'a VtopClient) -> Self {
        Self {
            client,
            active: true,
        }
    }

    fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for LoggingInGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            let mut state = self.client.state();
            if matches!(state.auth_stage, AuthStage::LoggingIn) {
                state.auth_stage = AuthStage::Idle;
            }
        }
    }
}

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
        let mut guard = LoggingInGuard::new(self);

        if self.state().session.is_cookie_external() {
            self.auth_log(
                Level::Info,
                "session.restore",
                "validating restored session before fresh login",
            );
            match self.try_restore_existing_session().await {
                Ok(true) => {
                    self.state().auth_stage = AuthStage::Idle;
                    guard.disarm();
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
                guard.disarm();
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
                guard.disarm();
                self.auth_log(Level::Info, "otp", "security OTP verification is required");
                Err(VtopError::OTPRequired(
                    "Additional verification is required.".to_string(),
                    issued_at,
                ))
            }
        }
    }

    pub async fn verify_session(&self) -> VtopResult<String> {
        let _validation = self.validation.write().await;
        self.check_authenticated_session(true)
            .await?
            .ok_or(VtopError::SessionExpired)?;
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
        let external_header = self
            .state()
            .session
            .external_cookie_header()
            .map(str::to_owned);
        let request = login::content(&self.config.base_url, external_header.as_deref());
        Self::log_request("validate_authenticated_session.send", "GET", &request.url);
        let response = self
            .send_request(request)
            .await
            .map_err(|error| send_error("validate_authenticated_session.send", error))?
            .read()
            .await
            .map_err(|error| network_error("validate_authenticated_session.text", error))?;
        let checked =
            login::read_content(&response, server_error_is_error).inspect_err(|error| {
                log::warn!(target: super::NETWORK, "validate_authenticated_session: {error}");
            })?;
        let Some((csrf, registration_number)) = checked else {
            self.mark_session_expired(
                "validate_authenticated_session",
                &format!("VTOP returned HTTP {} at {}", response.status, response.url),
            );
            return Ok(false);
        };
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
        let _validation = self.validation.write().await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::transport::{Hop, HopFuture, Transport, TransportError};
    use crate::inputs::{Password, Username};

    #[derive(Debug)]
    struct FailingTransport;

    impl Transport for FailingTransport {
        fn send(&self, _hop: Hop) -> HopFuture<'_> {
            Box::pin(async { Err(TransportError::Connect("offline".into())) })
        }

        fn pause(
            &self,
            _duration: std::time::Duration,
        ) -> std::pin::Pin<Box<dyn Future<Output = ()> + Send + '_>> {
            Box::pin(async {})
        }
    }

    #[tokio::test]
    async fn failed_login_resets_auth_stage_to_idle() {
        let mut client = VtopClient::builder()
            .transport(std::sync::Arc::new(FailingTransport))
            .with_credentials(
                Username::parse("22BCE0001").unwrap(),
                Password::parse("pw").unwrap(),
            )
            .unwrap();
        assert!(client.login().await.is_err());
        assert_eq!(client.state().auth_stage, AuthStage::Idle);
    }
}

//! The emailed security OTP VTOP asks for on a login from a new device,
//! run as the `vtop-proto` OTP machines.

use log::Level;

use super::{missing_csrf_error, AuthStage, VtopClient};
use crate::error::{VtopError, VtopResult};
use crate::inputs::OtpCode;
use crate::login::SignIn;
use crate::otp::{ResendOtp, SubmitOtp};

impl VtopClient {
    fn otp_pending(&self) -> VtopResult<String> {
        let state = self.state();
        if !matches!(state.auth_stage, AuthStage::AwaitingOtp { .. }) {
            return Err(VtopError::AuthenticationFailed(
                "No OTP challenge is active.".to_string(),
            ));
        }
        state
            .session
            .csrf_token()
            .ok_or_else(|| missing_csrf_error("security_otp"))
    }

    fn finish_otp(&self, message: &str) {
        let mut state = self.state();
        state.session.set_authenticated(true);
        state.session.set_cookie_external(false);
        state.auth_stage = AuthStage::Idle;
        state.logged_in_at = Some(crate::now_unix());
        drop(state);
        self.auth_log(Level::Info, "finish", message);
    }

    pub async fn submit_security_otp(&mut self, otp_code: &str) -> VtopResult<()> {
        let csrf = self.otp_pending()?;
        let otp_code = OtpCode::parse(otp_code)?;
        let machine = SubmitOtp::new(&self.config.base_url, &csrf, otp_code);
        match self.run("submit_security_otp", machine).await {
            Ok(SignIn::SignedIn {
                csrf,
                registration_number,
            }) => {
                let mut state = self.state();
                state.session.set_csrf_token(csrf);
                state.session.set_registration_number(registration_number);
                drop(state);
                self.finish_otp("OTP verified and session confirmed");
                Ok(())
            }
            Ok(SignIn::OtpRequired { .. }) => Err(VtopError::AuthenticationFailed(
                "We could not verify that OTP. Please try again.".to_string(),
            )),
            Err(error) => {
                self.auth_log(
                    Level::Warn,
                    "otp",
                    format!("OTP verification failed: {error}"),
                );
                Err(error)
            }
        }
    }

    pub async fn resend_security_otp(&mut self) -> VtopResult<()> {
        let csrf = self.otp_pending()?;
        let machine = ResendOtp::new(&self.config.base_url, &csrf);
        self.run("resend_security_otp", machine).await?;
        log::info!(target: super::AUTH, "OTP resend request completed successfully");
        Ok(())
    }
}

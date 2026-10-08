//! Logs in to VTOP with saved credentials, answering the OTP from Gmail.

use std::time::Duration;

use async_trait::async_trait;
use vtop_core::gmail::{GmailAccess, GmailOtpError};
use vtop_core::inputs::{Password, Username};
use vtop_core::{SessionState, VtopClient, VtopConfig, VtopError, VtopResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginError {
    /// VTOP rejected the username or password.
    InvalidCredentials,
    /// Anything else; the credentials may still be good.
    Failed(String),
}

#[async_trait]
pub trait VtopLogin: Send + Sync {
    async fn login(
        &self,
        username: &str,
        password: &str,
        gmail: GmailAccess,
        wait: Duration,
    ) -> Result<SessionState, LoginError>;
}

/// Maps the result of `login_with_gmail_otp`.
pub fn map_outcome(
    result: VtopResult<()>,
    gmail_error: Option<GmailOtpError>,
) -> Result<(), LoginError> {
    match result {
        Ok(()) => Ok(()),
        Err(VtopError::InvalidCredentials) => Err(LoginError::InvalidCredentials),
        // Gmail had no usable OTP email, so the login is stuck at the OTP.
        Err(VtopError::OTPRequired(..)) => Err(LoginError::Failed(format!(
            "otp_not_found: {}",
            gmail_error.map_or("none", GmailOtpError::code)
        ))),
        Err(error) => Err(LoginError::Failed(error.to_string())),
    }
}

pub struct CoreLogin {
    pub config: VtopConfig,
}

#[async_trait]
impl VtopLogin for CoreLogin {
    async fn login(
        &self,
        username: &str,
        password: &str,
        gmail: GmailAccess,
        wait: Duration,
    ) -> Result<SessionState, LoginError> {
        let invalid_input = |error: VtopError| LoginError::Failed(error.to_string());
        let mut client = VtopClient::builder()
            .config(self.config.clone())
            .with_credentials(
                Username::parse(username).map_err(invalid_input)?,
                Password::parse(password).map_err(invalid_input)?,
            )
            .map_err(invalid_input)?;
        let outcome = client.login_with_gmail_otp(Some(&gmail), wait).await;
        map_outcome(outcome.result, outcome.gmail_error)?;
        Ok(client.session_state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_is_ok() {
        assert_eq!(map_outcome(Ok(()), None), Ok(()));
    }

    #[test]
    fn invalid_credentials_map() {
        assert_eq!(
            map_outcome(Err(VtopError::InvalidCredentials), None),
            Err(LoginError::InvalidCredentials)
        );
    }

    #[test]
    fn otp_still_required_names_the_gmail_reason() {
        let result = map_outcome(
            Err(VtopError::OTPRequired("enter OTP".into(), 5)),
            Some(GmailOtpError::NotFound),
        );
        assert_eq!(
            result,
            Err(LoginError::Failed(
                "otp_not_found: gmail_otp_not_found".into()
            ))
        );
        let result = map_outcome(Err(VtopError::OTPRequired("enter OTP".into(), 5)), None);
        assert_eq!(
            result,
            Err(LoginError::Failed("otp_not_found: none".into()))
        );
    }

    #[test]
    fn other_errors_are_failed() {
        assert_eq!(
            map_outcome(Err(VtopError::NetworkError), None),
            Err(LoginError::Failed("Network connection error".into()))
        );
    }
}

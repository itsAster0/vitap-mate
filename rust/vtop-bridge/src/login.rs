//! Signs in to VTOP with saved credentials, the OTP coming from the
//! student's forwarded mail.

use std::time::Duration;

use async_trait::async_trait;
use vtop_core::inputs::{Password, Username};
use vtop_core::{SessionState, VtopClient, VtopConfig, VtopError, VtopResult};

/// Where a sign-in gets VTOP's emailed OTP from.
#[async_trait]
pub trait OtpCodes: Send + Sync {
    /// A code from mail sent after `issued_at` that is not in `tried`, or
    /// `None` when none comes within `remaining`.
    async fn next(&self, issued_at: u64, remaining: Duration, tried: Vec<String>)
        -> Option<String>;
}

/// Signs in with saved credentials, the OTP from `codes`.
#[async_trait]
pub trait ForwardedLogin: Send + Sync {
    async fn login(
        &self,
        username: &str,
        password: &str,
        codes: &dyn OtpCodes,
        wait: Duration,
    ) -> Result<SessionState, LoginError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginError {
    /// VTOP rejected the username or password.
    InvalidCredentials,
    /// Anything else; the credentials may still be good.
    Failed(String),
}

/// Maps how a sign-in ended.
pub fn map_outcome(result: VtopResult<()>) -> Result<(), LoginError> {
    match result {
        Ok(()) => Ok(()),
        Err(VtopError::InvalidCredentials) => Err(LoginError::InvalidCredentials),
        // No usable code came, so the sign-in is stuck at the OTP.
        Err(VtopError::OTPRequired(..)) => {
            Err(LoginError::Failed("otp_not_found: no forwarded OTP".into()))
        }
        Err(error) => Err(LoginError::Failed(error.to_string())),
    }
}

pub struct CoreLogin {
    pub config: VtopConfig,
}

#[async_trait]
impl ForwardedLogin for CoreLogin {
    async fn login(
        &self,
        username: &str,
        password: &str,
        codes: &dyn OtpCodes,
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
        let result = client
            .login_with_otp_from(wait, |issued_at, remaining, tried| {
                codes.next(issued_at, remaining, tried)
            })
            .await;
        map_outcome(result)?;
        Ok(client.session_state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcomes() {
        assert_eq!(map_outcome(Ok(())), Ok(()));
        assert_eq!(
            map_outcome(Err(VtopError::InvalidCredentials)),
            Err(LoginError::InvalidCredentials)
        );
        assert_eq!(
            map_outcome(Err(VtopError::OTPRequired("enter OTP".into(), 5))),
            Err(LoginError::Failed("otp_not_found: no forwarded OTP".into()))
        );
        assert_eq!(
            map_outcome(Err(VtopError::NetworkError)),
            Err(LoginError::Failed("Network connection error".into()))
        );
    }
}

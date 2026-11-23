//! Signs in to VTOP with saved credentials, the OTP coming from the
//! student's Gmail in one go, or from forwarded mail in a later call.

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

/// Signs in with saved credentials.
#[async_trait]
pub trait ForwardedLogin: Send + Sync {
    /// Signs in, the OTP from `codes`.
    async fn login(
        &self,
        username: &str,
        password: &str,
        codes: &dyn OtpCodes,
        wait: Duration,
    ) -> Result<SessionState, LoginError>;
    /// Signs in as far as VTOP's OTP, which comes later to [`Self::submit`].
    async fn start(&self, username: &str, password: &str) -> Result<Started, LoginError>;
    /// Sends `otp` on a session [`Self::start`] left waiting for one.
    async fn submit(&self, session: &SessionState, otp: &str) -> Result<SessionState, LoginError>;
}

/// No `Debug`: it holds cookies.
pub enum Started {
    SignedIn(SessionState),
    /// VTOP mailed an OTP at `issued_at`.
    OtpNeeded {
        session: SessionState,
        issued_at: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginError {
    /// VTOP rejected the username or password.
    InvalidCredentials,
    /// VTOP did not take the OTP (an older mail's code); it still waits for one.
    OtpRejected,
    /// Anything else; the credentials may still be good.
    Failed(String),
}

/// Maps how a sign-in ended.
pub fn map_outcome(result: VtopResult<()>) -> Result<(), LoginError> {
    result.map_err(login_error)
}

fn login_error(error: VtopError) -> LoginError {
    match error {
        VtopError::InvalidCredentials => LoginError::InvalidCredentials,
        // No usable code came, so the sign-in is stuck at the OTP.
        VtopError::OTPRequired(..) => {
            LoginError::Failed("otp_not_found: the OTP mail did not arrive".into())
        }
        error => LoginError::Failed(error.to_string()),
    }
}

fn failed(error: VtopError) -> LoginError {
    LoginError::Failed(error.to_string())
}

pub struct CoreLogin {
    pub config: VtopConfig,
}

impl CoreLogin {
    fn client(&self, username: &str, password: &str) -> Result<VtopClient, LoginError> {
        VtopClient::builder()
            .config(self.config.clone())
            .with_credentials(
                Username::parse(username).map_err(failed)?,
                Password::parse(password).map_err(failed)?,
            )
            .map_err(failed)
    }
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
        let mut client = self.client(username, password)?;
        let result = client
            .login_with_otp_from(wait, |issued_at, remaining, tried| {
                codes.next(issued_at, remaining, tried)
            })
            .await;
        map_outcome(result)?;
        Ok(client.session_state())
    }

    async fn start(&self, username: &str, password: &str) -> Result<Started, LoginError> {
        let mut client = self.client(username, password)?;
        match client.login().await {
            Ok(()) => Ok(Started::SignedIn(client.session_state())),
            Err(VtopError::OTPRequired(_, issued_at)) => Ok(Started::OtpNeeded {
                session: client.session_state(),
                issued_at,
            }),
            Err(error) => Err(login_error(error)),
        }
    }

    async fn submit(&self, session: &SessionState, otp: &str) -> Result<SessionState, LoginError> {
        let mut client = VtopClient::builder()
            .config(self.config.clone())
            .with_session(session)
            .map_err(failed)?;
        match client.submit_security_otp(otp).await {
            Ok(()) => Ok(client.session_state()),
            Err(VtopError::AuthenticationFailed(_)) => Err(LoginError::OtpRejected),
            Err(error) => Err(login_error(error)),
        }
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
            Err(LoginError::Failed(
                "otp_not_found: the OTP mail did not arrive".into()
            ))
        );
        assert_eq!(
            map_outcome(Err(VtopError::NetworkError)),
            Err(LoginError::Failed("Network connection error".into()))
        );
    }
}

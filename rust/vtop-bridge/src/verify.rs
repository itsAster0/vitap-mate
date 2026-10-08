//! Finds out, from VTOP itself, whose session a set of cookies is.

use async_trait::async_trait;
use vtop_core::{SessionState, VtopClient, VtopConfig, VtopError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// VTOP does not accept the cookies.
    Expired,
    Unavailable(String),
}

#[async_trait]
pub trait SessionVerifier: Send + Sync {
    /// The upper-cased registration number of the session's owner.
    async fn registration_number(&self, session: &SessionState) -> Result<String, VerifyError>;
}

pub struct CoreVerifier {
    pub config: VtopConfig,
}

/// Maps a `verify_session` error.
pub fn map_error(error: VtopError) -> VerifyError {
    match error {
        VtopError::SessionExpired
        | VtopError::AuthenticationFailed(_)
        | VtopError::RegistrationParsingError => VerifyError::Expired,
        other => VerifyError::Unavailable(other.to_string()),
    }
}

#[async_trait]
impl SessionVerifier for CoreVerifier {
    async fn registration_number(&self, session: &SessionState) -> Result<String, VerifyError> {
        // Only the cookies count; anything else the caller sent is ignored.
        let cookies_only = SessionState {
            cookies: session.cookies.clone(),
            csrf_token: None,
            registration_number: None,
            otp_issued_at: None,
            logged_in_at: None,
        };
        let client = VtopClient::builder()
            .config(self.config.clone())
            .with_session(&cookies_only)
            .map_err(|_| VerifyError::Expired)?;
        client
            .verify_session()
            .await
            .map(|registration_number| registration_number.trim().to_uppercase())
            .map_err(map_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_sessions_are_expired() {
        assert_eq!(map_error(VtopError::SessionExpired), VerifyError::Expired);
        assert_eq!(
            map_error(VtopError::AuthenticationFailed("x".into())),
            VerifyError::Expired
        );
        assert_eq!(
            map_error(VtopError::RegistrationParsingError),
            VerifyError::Expired
        );
    }

    #[test]
    fn outages_are_unavailable() {
        assert!(matches!(
            map_error(VtopError::NetworkError),
            VerifyError::Unavailable(_)
        ));
        assert!(matches!(
            map_error(VtopError::VtopServerError("503".into())),
            VerifyError::Unavailable(_)
        ));
    }
}

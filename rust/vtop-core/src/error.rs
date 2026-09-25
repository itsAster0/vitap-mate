use serde::Serialize;

/// Every failure the VTOP client can report.
///
/// The variants and their payloads are part of the Dart API (mirrored by the
/// FFI crate), so add new variants rather than reshaping existing ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
pub enum VtopError {
    #[error("Network connection error")]
    NetworkError,
    #[error("VTOP server error: {0}")]
    VtopServerError(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Failed to parse registration number")]
    RegistrationParsingError,
    #[error("Invalid username or password")]
    InvalidCredentials,
    #[error("Session has expired")]
    SessionExpired,
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    #[error("Captcha verification required")]
    CaptchaRequired,
    #[error("Invalid response from server")]
    InvalidResponse,
    /// VTOP wants the emailed security OTP. Carries a message and the unix
    /// time (seconds) the challenge was issued.
    #[error("OTP required: {0} at {1}")]
    OTPRequired(String, u64),
}

pub type VtopResult<T> = Result<T, VtopError>;

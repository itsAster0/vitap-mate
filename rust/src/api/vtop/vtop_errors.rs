//! Dart-facing mirror of `vtop_core::VtopError`.
#![allow(dead_code)]

use flutter_rust_bridge::frb;
pub use vtop_core::error::{VtopError, VtopResult};

#[frb(mirror(VtopError), non_opaque)]
pub enum _VtopError {
    NetworkError,
    VtopServerError(String),
    AuthenticationFailed(String),
    RegistrationParsingError,
    InvalidCredentials,
    SessionExpired,
    ParseError(String),
    ConfigurationError(String),
    CaptchaRequired,
    InvalidResponse,
    OTPRequired(String, u64),
}

//! JSON error responses. Messages never echo cookies, passwords or OTPs.

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use vtop_core::VtopError;

#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    RateLimited { retry_after_secs: u64 },
    LoginDisabled,
    Timeout,
    BadRequest(String),
    Vtop(VtopError),
}

impl From<VtopError> for ApiError {
    fn from(error: VtopError) -> Self {
        Self::Vtop(error)
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: ErrorDetail<'a>,
}

#[derive(Serialize)]
struct ErrorDetail<'a> {
    code: &'a str,
    message: String,
}

impl ApiError {
    /// The machine-readable code and message, as sent in error bodies.
    pub fn code_and_message(&self) -> (&'static str, String) {
        let (_, code, message) = self.parts();
        (code, message)
    }

    fn parts(&self) -> (StatusCode, &'static str, String) {
        match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "missing or invalid API key".into(),
            ),
            Self::RateLimited { .. } => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "too many requests for this API key".into(),
            ),
            Self::LoginDisabled => (
                StatusCode::FORBIDDEN,
                "login_disabled",
                "this server does not accept VTOP credentials".into(),
            ),
            Self::Timeout => (
                StatusCode::GATEWAY_TIMEOUT,
                "timeout",
                "VTOP did not answer in time".into(),
            ),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message.clone()),
            Self::Vtop(error) => {
                let (status, code) = match error {
                    VtopError::SessionExpired => (StatusCode::UNAUTHORIZED, "session_expired"),
                    VtopError::InvalidCredentials => {
                        (StatusCode::UNAUTHORIZED, "invalid_credentials")
                    }
                    VtopError::AuthenticationFailed(_) => {
                        (StatusCode::UNAUTHORIZED, "authentication_failed")
                    }
                    VtopError::OTPRequired(..) => (StatusCode::UNAUTHORIZED, "otp_required"),
                    VtopError::ConfigurationError(_) => (StatusCode::BAD_REQUEST, "invalid_input"),
                    VtopError::CaptchaRequired => {
                        (StatusCode::SERVICE_UNAVAILABLE, "captcha_unavailable")
                    }
                    VtopError::NetworkError => (StatusCode::BAD_GATEWAY, "vtop_unreachable"),
                    VtopError::VtopServerError(_) => (StatusCode::BAD_GATEWAY, "vtop_error"),
                    VtopError::RegistrationParsingError
                    | VtopError::ParseError(_)
                    | VtopError::InvalidResponse => (StatusCode::BAD_GATEWAY, "vtop_unparseable"),
                };
                (status, code, error.to_string())
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = self.parts();
        let mut response = (
            status,
            Json(ErrorBody {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response();
        if let Self::RateLimited { retry_after_secs } = self {
            if let Ok(value) = HeaderValue::from_str(&retry_after_secs.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, value);
            }
        }
        response
    }
}

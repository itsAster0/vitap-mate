//! JSON error responses, in the Go bridge's shape: `{"code", "error"}`.
//! Messages never echo cookies, passwords or tokens.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "code": self.code, "error": self.message });
        (self.status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn renders_code_and_error() {
        let response = ApiError::new(
            StatusCode::GONE,
            "TOKEN_INVALID",
            "The mobile token is invalid.",
        )
        .into_response();
        assert_eq!(response.status(), StatusCode::GONE);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"code": "TOKEN_INVALID", "error": "The mobile token is invalid."})
        );
    }
}

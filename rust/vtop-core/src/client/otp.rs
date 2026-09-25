//! The emailed security OTP VTOP asks for on a login from a new device.

use log::Level;
use reqwest::multipart;

use super::{missing_csrf_error, network_error, server_error, AuthStage, SendWith, VtopClient};
use crate::error::{VtopError, VtopResult};
use crate::inputs::{OtpCode, RegistrationNumber};
use crate::parser::page;

const RESEND_PATHS: [&str; 2] = ["/vtop/resendSecurityOtp", "/vtop/resendSecurityOTP"];

/// The JSON VTOP answers OTP requests with. Every field is optional because
/// the endpoint also answers with plain text or HTML.
#[derive(Debug, Default)]
struct OtpReply {
    status: Option<String>,
    message: Option<String>,
    redirect_url: Option<String>,
}

impl OtpReply {
    fn parse(body: &str) -> Self {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(body) else {
            return Self::default();
        };
        let text = |key: &str| json.get(key).and_then(|value| value.as_str());
        Self {
            status: text("status").map(str::to_uppercase),
            message: text("message").map(str::to_string),
            redirect_url: text("redirectUrl")
                .filter(|url| !url.trim().is_empty())
                .map(str::to_string),
        }
    }

    fn is_json(&self) -> bool {
        self.status.is_some() || self.message.is_some() || self.redirect_url.is_some()
    }
}

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
        let url = self.config.url("/vtop/validateSecurityOtp");
        let referer = self.config.url("/vtop/login/error");

        let form = multipart::Form::new()
            .text("otpCode", otp_code.as_str().to_string())
            .text("_csrf", csrf);

        Self::log_request("submit_security_otp.send", "POST", &url);
        let response = self
            .http
            .post(&url)
            .header("Accept", "*/*")
            .header("Origin", self.config.base_url.as_str())
            .header("Referer", referer)
            .header("Sec-Fetch-Dest", "empty")
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Site", "same-origin")
            .multipart(form)
            .send_with(self)
            .await
            .map_err(|error| network_error("submit_security_otp.send", error))?;
        let final_url = response.url().to_string();
        let status_code = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|error| network_error("submit_security_otp.text", error))?;

        let reply = OtpReply::parse(&response_text);
        let status = reply.status.as_deref();

        if status == Some("INVALID") || response_text.to_lowercase().contains("invalid otp") {
            self.auth_log(
                Level::Warn,
                "otp",
                "OTP verification failed because the code was invalid",
            );
            return Err(VtopError::AuthenticationFailed(
                reply
                    .message
                    .unwrap_or_else(|| "Invalid OTP. Please try again.".to_string()),
            ));
        }

        if status == Some("EXPIRED") {
            self.auth_log(
                Level::Warn,
                "otp",
                "OTP verification failed because the code expired",
            );
            return Err(VtopError::AuthenticationFailed(
                reply
                    .message
                    .unwrap_or_else(|| "OTP has expired. Please resend.".to_string()),
            ));
        }

        if status == Some("SUCCESS") {
            match reply.redirect_url.as_deref() {
                Some(redirect_url) => {
                    if self.follow_security_otp_redirect(redirect_url).await? {
                        return Ok(());
                    }
                }
                None => self.auth_log(
                    Level::Warn,
                    "otp",
                    "OTP verification returned SUCCESS without redirectUrl",
                ),
            }
        }

        if let Some(status) = status {
            if !matches!(status, "VALID" | "SUCCESS" | "OK") {
                self.auth_log(
                    Level::Warn,
                    "otp",
                    "OTP verification returned a non-success status",
                );
                return Err(VtopError::AuthenticationFailed(
                    reply
                        .message
                        .unwrap_or_else(|| "Verification failed. Please try again.".to_string()),
                ));
            }
        }

        let response_declares_success = matches!(status, Some("VALID" | "SUCCESS" | "OK"))
            || matches!(
                response_text.trim().to_uppercase().as_str(),
                "VALID" | "SUCCESS" | "OK"
            );

        let landed_on_content =
            status_code.is_success() && final_url.trim_end_matches('/').ends_with("/vtop/content");
        if landed_on_content {
            let (csrf, registration_number) = page::extract_session_fields(&response_text);
            let registration_number = RegistrationNumber::parse(
                &registration_number.ok_or(VtopError::RegistrationParsingError)?,
            )?;
            let mut state = self.state();
            if let Some(csrf) = csrf {
                state.session.set_csrf_token(csrf);
            }
            state.session.set_registration_number(registration_number);
            drop(state);
            self.finish_otp("OTP verified and session confirmed");
            return Ok(());
        }

        if matches!(self.validate_authenticated_session().await, Ok(true)) {
            self.finish_otp("OTP verified and session confirmed");
            return Ok(());
        }

        if response_declares_success {
            self.auth_log(
                Level::Warn,
                "otp",
                format!(
                    "OTP endpoint reported success, but session validation failed after landing at {final_url}"
                ),
            );
            return Err(VtopError::AuthenticationFailed(
                "VTOP accepted the OTP, but did not open a signed-in session. Please try again."
                    .to_string(),
            ));
        }

        self.auth_log(
            Level::Warn,
            "otp",
            format!(
                "OTP verification did not return a success response: HTTP {status_code} at {final_url}"
            ),
        );
        Err(VtopError::AuthenticationFailed(
            "We could not verify that OTP. Please try again.".to_string(),
        ))
    }

    async fn follow_security_otp_redirect(&self, redirect_url: &str) -> VtopResult<bool> {
        let url = self.resolve_vtop_url(redirect_url)?;
        Self::log_request("follow_security_otp_redirect.send", "GET", &url);
        let response = self
            .http
            .get(&url)
            .header("Accept", "*/*")
            .send_with(self)
            .await
            .map_err(|error| network_error("follow_security_otp_redirect.send", error))?;
        let final_url = response.url().to_string();
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| network_error("follow_security_otp_redirect.text", error))?;

        if Self::is_login_url(&final_url) {
            self.mark_session_expired(
                "follow_security_otp_redirect",
                &format!("VTOP returned login page after OTP redirect at {final_url}"),
            );
            return Ok(false);
        }
        if !status.is_success() {
            return Err(server_error(
                "follow_security_otp_redirect",
                format!("VTOP returned HTTP {status} at {final_url}"),
            ));
        }

        let (csrf, registration_number) = page::extract_session_fields(&text);
        let registration_number = RegistrationNumber::parse(
            &registration_number.ok_or(VtopError::RegistrationParsingError)?,
        )?;
        {
            let mut state = self.state();
            if let Some(csrf) = csrf {
                state.session.set_csrf_token(csrf);
            }
            state.session.set_registration_number(registration_number);
        }

        if matches!(self.validate_authenticated_session().await, Ok(true)) {
            self.finish_otp("OTP redirect followed and session confirmed");
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn resend_security_otp(&mut self) -> VtopResult<()> {
        let csrf = self.otp_pending()?;
        let referer = self.config.url("/vtop/login/error");
        let failed = || {
            VtopError::AuthenticationFailed("Failed to resend OTP. Please try again.".to_string())
        };

        for path in RESEND_PATHS {
            let url = self.config.url(path);
            let form = multipart::Form::new().text("_csrf", csrf.clone());

            Self::log_request("resend_security_otp.send", "POST", &url);
            let response = self
                .http
                .post(&url)
                .header("Accept", "*/*")
                .header("Origin", self.config.base_url.as_str())
                .header("Referer", &referer)
                .header("Sec-Fetch-Dest", "empty")
                .header("Sec-Fetch-Mode", "cors")
                .header("Sec-Fetch-Site", "same-origin")
                .multipart(form)
                .send_with(self)
                .await
                .map_err(|error| network_error("resend_security_otp.send", error))?;

            let status_code = response.status();
            let response_text = response
                .text()
                .await
                .map_err(|error| network_error("resend_security_otp.text", error))?;

            if status_code.as_u16() == 404 {
                continue;
            }

            let reply = OtpReply::parse(&response_text);
            if reply.is_json() {
                if matches!(
                    reply.status.as_deref(),
                    Some("SUCCESS" | "SENT" | "VALID" | "OK")
                ) {
                    log::info!(target: super::AUTH, "OTP resend request completed successfully");
                    return Ok(());
                }
                return Err(VtopError::AuthenticationFailed(
                    reply
                        .message
                        .unwrap_or_else(|| "Failed to resend OTP. Please try again.".to_string()),
                ));
            }

            let lower = response_text.to_lowercase();
            if status_code.is_success() && lower.contains("otp") && lower.contains("sent") {
                return Ok(());
            }
            return Err(failed());
        }

        Err(failed())
    }
}

#[cfg(test)]
mod tests {
    use super::OtpReply;

    #[test]
    fn otp_reply_reads_json_and_ignores_html() {
        let reply =
            OtpReply::parse(r#"{"status":"success","message":"ok","redirectUrl":"/vtop/content"}"#);
        assert_eq!(reply.status.as_deref(), Some("SUCCESS"));
        assert_eq!(reply.redirect_url.as_deref(), Some("/vtop/content"));
        assert!(!OtpReply::parse("<html></html>").is_json());
        assert_eq!(
            OtpReply::parse(r#"{"redirectUrl":"  "}"#).redirect_url,
            None
        );
    }
}

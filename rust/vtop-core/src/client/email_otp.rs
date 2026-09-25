//! Login that answers VTOP's security OTP from Gmail by itself.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use log::Level;

use super::VtopClient;
use crate::error::{VtopError, VtopResult};
use crate::gmail::{self, GmailAccess, GmailOtpError};

/// What [`VtopClient::login_with_gmail_otp`] did.
#[derive(Debug)]
pub struct EmailOtpLogin {
    /// `Ok` when signed in. `Err(OTPRequired)` when the OTP still has to be
    /// entered (Gmail had no usable email); the session is then waiting for
    /// [`VtopClient::submit_security_otp`] as after a plain login.
    pub result: VtopResult<()>,
    /// Why Gmail could not supply the OTP, if it was asked.
    pub gmail_error: Option<GmailOtpError>,
}

impl VtopClient {
    /// Logs in, and if VTOP asks for the emailed security OTP, reads it from
    /// Gmail with `gmail` (waiting up to `wait` for the email) and submits
    /// it. A code VTOP rejects is skipped and the next email is tried.
    pub async fn login_with_gmail_otp(
        &mut self,
        gmail: Option<&GmailAccess>,
        wait: Duration,
    ) -> EmailOtpLogin {
        let (message, issued_at) = match self.login().await {
            Err(VtopError::OTPRequired(message, issued_at)) => (message, issued_at),
            other => {
                return EmailOtpLogin {
                    result: other,
                    gmail_error: None,
                }
            }
        };
        let otp_required = || Err(VtopError::OTPRequired(message.clone(), issued_at));
        let Some(gmail) = gmail else {
            return EmailOtpLogin {
                result: otp_required(),
                gmail_error: None,
            };
        };

        let deadline = Instant::now() + wait;
        let mut tried = HashSet::new();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let found = match gmail::wait_for_otp(gmail, issued_at, remaining, &tried).await {
                Ok(found) => found,
                Err(error) => {
                    self.auth_log(
                        Level::Info,
                        "otp.gmail",
                        format!("no OTP from Gmail: {}", error.code()),
                    );
                    return EmailOtpLogin {
                        result: otp_required(),
                        gmail_error: Some(error),
                    };
                }
            };
            match self.submit_security_otp(&found.code).await {
                Ok(()) => {
                    gmail::tidy_up(gmail, &found.message_id).await;
                    self.auth_log(Level::Info, "otp.gmail", "OTP read from Gmail and accepted");
                    return EmailOtpLogin {
                        result: Ok(()),
                        gmail_error: None,
                    };
                }
                // An older email's code: VTOP rejects it and the challenge
                // stays open, so look for a newer one.
                Err(VtopError::AuthenticationFailed(_)) if !remaining.is_zero() => {
                    tried.insert(found.message_id);
                }
                Err(error) => {
                    return EmailOtpLogin {
                        result: Err(error),
                        gmail_error: None,
                    }
                }
            }
        }
    }
}

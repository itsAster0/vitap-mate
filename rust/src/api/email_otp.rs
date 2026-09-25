//! Gmail OTP reading for the app's auto-fetch, shared with vtop-server via
//! `vtop_core::gmail`. The app keeps Google sign-in and token refresh; it
//! passes a fresh access token for each call.

use std::collections::HashSet;

use vtop_core::gmail::{self, GmailAccess};

use crate::api::vtop::vtop_errors::VtopError;

/// An OTP found in Gmail.
pub struct GmailOtpCode {
    pub code: String,
    /// Pass back in `skip_message_ids` if VTOP rejects the code.
    pub message_id: String,
}

fn access(
    access_token: String,
    delete_after_reading: bool,
    expires_at_unix: Option<u64>,
) -> GmailAccess {
    GmailAccess {
        access_token,
        delete_after_reading,
        expires_at: expires_at_unix,
    }
}

/// One look at the mailbox for VTOP's OTP email sent after
/// `issued_at_unix`. Errors are `ConfigurationError` with a stable code:
/// `gmail_unauthorized` (refresh the token and retry), `gmail_forbidden`,
/// `gmail_unavailable`.
pub async fn gmail_find_otp(
    access_token: String,
    expires_at_unix: Option<u64>,
    issued_at_unix: u64,
    skip_message_ids: Vec<String>,
) -> Result<Option<GmailOtpCode>, VtopError> {
    let skip: HashSet<String> = skip_message_ids.into_iter().collect();
    let found = gmail::find_otp(
        &access(access_token, false, expires_at_unix),
        issued_at_unix,
        &skip,
    )
    .await
    .map_err(|error| VtopError::ConfigurationError(error.code().to_string()))?;
    Ok(found.map(|otp| GmailOtpCode {
        code: otp.code,
        message_id: otp.message_id,
    }))
}

/// Trashes the OTP email (or only marks it read). Best effort.
pub async fn gmail_tidy_up(access_token: String, message_id: String, delete_after_reading: bool) {
    gmail::tidy_up(
        &access(access_token, delete_after_reading, None),
        &message_id,
    )
    .await;
}

//! Forwarded OTP mail (`vtop-otp-<id>@aster0.dev`): the phone's keypair and
//! opening the boxes the mail Worker seals. See `vtop_core::mail_seal`.

use vtop_core::mail_seal::{self, SealError};

use crate::api::vtop::vtop_errors::VtopError;

/// A new X25519 keypair, base64url. Keep `secret` in secure storage; send
/// `public_key` to vtop-bridge.
pub struct MailOtpKeypair {
    pub secret: String,
    pub public_key: String,
}

#[flutter_rust_bridge::frb(sync)]
pub fn mail_otp_generate_keypair() -> MailOtpKeypair {
    let (secret, public_key) = mail_seal::generate_keypair();
    MailOtpKeypair { secret, public_key }
}

/// The JSON a box carries, e.g. `{"kind":"otp","code":"401295","sentAt":…}`
/// or `{"kind":"gmail_confirm","code":…,"link":…}`. Errors are
/// `ConfigurationError` with code `mail_otp_malformed` or
/// `mail_otp_rejected` (wrong key or mailbox).
#[flutter_rust_bridge::frb(sync)]
pub fn mail_otp_open(
    secret: String,
    mailbox_id: String,
    sealed: String,
) -> Result<String, VtopError> {
    let bytes = mail_seal::open(&secret, &mailbox_id, &sealed).map_err(|error| {
        VtopError::ConfigurationError(
            match error {
                SealError::Malformed => "mail_otp_malformed",
                SealError::Rejected => "mail_otp_rejected",
            }
            .to_string(),
        )
    })?;
    String::from_utf8(bytes)
        .map_err(|_| VtopError::ConfigurationError("mail_otp_malformed".to_string()))
}

//! VTOP's OTP in a Gmail message the app fetched itself (OAuth and HTTP
//! stay in Kotlin, `Gmail.kt`), read by the same parser vtop-core's Gmail
//! poll uses ([`vtop_core::gmail`]). Pure: nothing is logged, least of all
//! the code or the mail.

use vtop_core::gmail;

/// The OTP in a Gmail API message (`users.messages.get`, `format=full`, as
/// JSON) when VTOP sent it at or after `since` (unix seconds); None for an
/// older mail, another sender, no code or JSON that is not a message.
#[uniffi::export]
pub fn gmail_otp(message_json: String, since: i64) -> Option<String> {
    let message = serde_json::from_str::<gmail::Message>(&message_json).ok()?;
    gmail::otp(&message, u64::try_from(since).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_message_json() {
        // "Your OTP is 401295" in URL-safe base64.
        let json = r#"{"id":"m1","internalDate":"1763700000000","snippet":"",
            "payload":{"headers":[{"name":"From","value":"VTOP <noreply.sdc@vitap.ac.in>"}],
            "body":{"size":18,"data":"WW91ciBPVFAgaXMgNDAxMjk1"}}}"#;
        assert_eq!(
            gmail_otp(json.to_string(), 1_763_699_990).as_deref(),
            Some("401295")
        );
        assert_eq!(gmail_otp(json.to_string(), -30).as_deref(), Some("401295"));
        assert_eq!(gmail_otp(json.to_string(), 1_763_700_001), None);
        assert_eq!(gmail_otp("not json".to_string(), 0), None);
    }
}

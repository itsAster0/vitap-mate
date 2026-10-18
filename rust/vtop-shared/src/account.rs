//! Accounts as the edge keeps them and the engine and app see them:
//! settings, access keys and the ids they are found by.

use serde::{Deserialize, Serialize};

use crate::tokens::{random_token, APP_SECRET_PREFIX, KEY_PREFIX};

pub const MAX_KEYS_PER_ACCOUNT: usize = 10;
/// A key's `last_used_at` is written at most this often.
pub const TOUCH_EVERY_SECS: u64 = 60;
pub const MAX_LABEL_CHARS: usize = 60;

pub const PHONE_UNREACHABLE: &str =
    "phone_unreachable: Open VITAP Mate → Connected apps → Reconnect this phone.";

/// How sessions of an account are served; the student picks these in the
/// app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSettings {
    /// Sign in with saved credentials as soon as a session is needed,
    /// without asking the phone first.
    pub always_use_vault: bool,
    /// When not `always_use_vault`: how long the phone gets to answer
    /// before the server signs in with saved credentials.
    pub phone_wait_secs: u64,
    /// How long saved credentials are kept after they were last sent;
    /// `None` keeps them until VTOP rejects them.
    pub vault_ttl_secs: Option<u64>,
}

impl Default for AccountSettings {
    fn default() -> Self {
        Self {
            always_use_vault: true,
            phone_wait_secs: 20,
            vault_ttl_secs: None,
        }
    }
}

impl AccountSettings {
    pub const PHONE_WAIT_CHOICES: [u64; 3] = [10, 20, 45];
    pub const VAULT_TTL_CHOICES: [Option<u64>; 3] = [Some(86_400), Some(172_800), None];

    pub fn is_valid(&self) -> bool {
        Self::PHONE_WAIT_CHOICES.contains(&self.phone_wait_secs)
            && Self::VAULT_TTL_CHOICES.contains(&self.vault_ttl_secs)
    }
}

/// Account ids are upper-cased registration numbers.
pub fn account_id(registration_number: &str) -> String {
    registration_number.trim().to_uppercase()
}

/// What an account id looks like: VTOP registration numbers are short and
/// alphanumeric.
pub fn is_account_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 20
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || b.is_ascii_uppercase())
}

pub fn new_key() -> String {
    random_token(KEY_PREFIX)
}

pub fn new_app_secret() -> String {
    random_token(APP_SECRET_PREFIX)
}

/// The short id users see and revoke a key by.
pub fn key_id(hash: &str) -> &str {
    &hash[..8.min(hash.len())]
}

pub fn key_label(raw: Option<&str>) -> String {
    let label = raw
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .unwrap_or("Access key");
    label.chars().take(MAX_LABEL_CHARS).collect()
}

/// A VTOP semester id as the app sends it: plain alphanumerics.
pub fn semester_id(raw: &str) -> Option<String> {
    let id = raw.trim();
    (!id.is_empty() && id.len() <= 40 && id.chars().all(|c| c.is_ascii_alphanumeric()))
        .then(|| id.to_string())
}

/// The code before the colon of a request error (`code: message`).
pub fn error_code(error: &str) -> String {
    error
        .split_once(':')
        .map_or("login_failed", |(code, _)| code)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_only_take_offered_choices() {
        assert!(AccountSettings::default().is_valid());
        let odd = AccountSettings {
            phone_wait_secs: 11,
            ..AccountSettings::default()
        };
        assert!(!odd.is_valid());
        let json = serde_json::to_value(AccountSettings::default()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"alwaysUseVault": true, "phoneWaitSecs": 20, "vaultTtlSecs": null})
        );
    }

    #[test]
    fn ids_and_labels() {
        assert_eq!(account_id(" 24mic7076 "), "24MIC7076");
        assert!(is_account_id("24MIC7076"));
        assert!(!is_account_id("24mic7076"));
        assert!(!is_account_id("a/b"));
        assert_eq!(key_id("abcdef0123456789"), "abcdef01");
        assert_eq!(key_label(Some("  ")), "Access key");
        assert_eq!(key_label(Some(&"x".repeat(80))).len(), 60);
        assert!(new_key().starts_with("vtm_"));
        assert!(new_app_secret().starts_with("vta_"));
        assert_eq!(semester_id(" VL2025 "), Some("VL2025".into()));
        assert_eq!(semester_id("VL 2025"), None);
        assert_eq!(error_code("phone_unreachable: x"), "phone_unreachable");
        assert_eq!(error_code("plain"), "login_failed");
    }
}

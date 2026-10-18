//! What the edge and the engine say to each other. The engine does VTOP
//! work and holds the vault key; the edge holds everything else. Every
//! call carries `Authorization: Bearer <shared secret>`.

use serde::{Deserialize, Serialize};

/// Engine: whose session this is, or whether a cached one still works.
pub const SESSION: &str = "/engine/session";
/// Engine: seals saved credentials for an account.
pub const SEAL: &str = "/engine/seal";
/// Engine: signs in with saved credentials; answers at once and reports
/// to the edge's [`LOGIN_DONE`] when finished.
pub const LOGIN: &str = "/engine/login";
/// Engine: fetches and parses VTOP pages for a session.
pub const FETCH: &str = "/engine/fetch";
/// Edge: the outcome of a [`LOGIN`].
pub const LOGIN_DONE: &str = "/internal/accounts/login-done";
/// Edge: waits for the next forwarded OTP of a student, sealed to a key.
pub const OTP_AWAIT: &str = "/internal/mail-otp/await";

/// Either fresh cookies from a client, or an account's cached session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCheck {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cookies: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sealed_session: Option<String>,
    /// The account a sealed session belongs to (its seal is bound to it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// VTOP accepts the session.
    Valid,
    /// VTOP rejects it, or a sealed one no longer opens.
    Expired,
    /// VTOP could not be asked.
    Unknown,
}

/// No `Debug`: it holds cookies.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionChecked {
    pub status: SessionStatus,
    /// Whose session it is, when `Valid`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_number: Option<String>,
    /// The `Cookie` header, when `Valid`, or `Unknown` for a cached one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cookies: Option<String>,
    /// The session sealed for caching, when `Valid`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sealed_session: Option<String>,
    /// The student's mailbox owner hash, when `Valid`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_hash: Option<String>,
}

/// No `Debug`: it holds a password.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealVault {
    pub account: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealedVault {
    pub sealed_vault: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStart {
    pub account: String,
    pub sealed_vault: String,
    pub request_id: String,
}

/// No `Debug`: `Ok` holds cookies.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LoginOutcome {
    #[serde(rename_all = "camelCase")]
    Ok {
        cookies: String,
        sealed_session: String,
    },
    /// VTOP rejected the saved username or password.
    InvalidCredentials,
    /// The saved credentials belong to someone else, or no longer open.
    VaultUnusable,
    /// No forwarding address is set up, so the OTP cannot be read.
    ForwardingRequired,
    /// Anything else; the credentials may still be good.
    Failed { reason: String },
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginDone {
    pub account: String,
    pub request_id: String,
    pub outcome: LoginOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtpAwait {
    pub owner_hash: String,
    /// An X25519 public key made for this one sign-in.
    pub public_key: String,
    /// Only mail sent at or after this (unix seconds) counts.
    pub since: u64,
    pub wait_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtpAwaited {
    /// The mail payload sealed to `public_key`; `None` when nothing came.
    pub sealed: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_outcomes_are_tagged() {
        let ok = LoginOutcome::Ok {
            cookies: "A=1".into(),
            sealed_session: "s".into(),
        };
        assert_eq!(
            serde_json::to_value(&ok).unwrap(),
            serde_json::json!({"status": "ok", "cookies": "A=1", "sealedSession": "s"})
        );
        let failed: LoginOutcome =
            serde_json::from_value(serde_json::json!({"status": "failed", "reason": "x"})).unwrap();
        assert!(failed == LoginOutcome::Failed { reason: "x".into() });
        assert_eq!(
            serde_json::to_value(LoginOutcome::ForwardingRequired).unwrap(),
            serde_json::json!({"status": "forwarding_required"})
        );
    }

    #[test]
    fn session_check_omits_absent_fields() {
        let check = SessionCheck {
            cookies: Some("A=1".into()),
            ..SessionCheck::default()
        };
        assert_eq!(
            serde_json::to_value(&check).unwrap(),
            serde_json::json!({"cookies": "A=1"})
        );
    }
}

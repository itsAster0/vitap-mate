//! The codes our server answers with (`{ code, error }`, `backend/convex/lib/errors.ts`), read
//! once into [`ServerCode`] where the answer comes in, so callers match variants, not strings.
//! `vtop-shared/vectors/server_codes.json` lists every code matched here; a cargo test and a
//! vitest keep it, this enum and the backend in step.

/// A code from the server (or the engine behind it), or one the phone gives a failure that
/// carried none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ServerCode {
    /// No JWT, or one the server no longer takes: renew it.
    Unauthenticated,
    /// This install was removed from the account on the server.
    DeviceRemoved,
    RefreshReused,
    RefreshInvalid,
    /// A v2 account token the server won't trade in.
    TokenInvalid,
    TokenExpired,
    /// No such function or route (a server older than the app).
    NotFound,
    BadRequest,
    DemoUnavailable,
    ProxyUnavailable,
    MailboxNotFound,
    KeyLimit,
    PairExpired,
    KeyNotFound,
    PhoneNotFound,
    SessionInvalid,
    VtopUnreachable,
    /// The engine's: VTOP's session ran out.
    SessionExpired,
    /// The engine's: VTOP turned the saved password down.
    InvalidCredentials,
    /// The engine's: VTOP turned the OTP down.
    AuthenticationFailed,
    /// The phone's: a failure with no code (or a server that crashed).
    ServerError,
    /// The phone's: proving who the student is failed.
    IdentityFailed,
    /// Anything else, kept for the logs.
    Unknown(String),
}

impl ServerCode {
    /// Every code but [`Self::Unknown`], for the contract test.
    #[cfg(test)]
    pub(crate) const KNOWN: [Self; 22] = [
        Self::Unauthenticated,
        Self::DeviceRemoved,
        Self::RefreshReused,
        Self::RefreshInvalid,
        Self::TokenInvalid,
        Self::TokenExpired,
        Self::NotFound,
        Self::BadRequest,
        Self::DemoUnavailable,
        Self::ProxyUnavailable,
        Self::MailboxNotFound,
        Self::KeyLimit,
        Self::PairExpired,
        Self::KeyNotFound,
        Self::PhoneNotFound,
        Self::SessionInvalid,
        Self::VtopUnreachable,
        Self::SessionExpired,
        Self::InvalidCredentials,
        Self::AuthenticationFailed,
        Self::ServerError,
        Self::IdentityFailed,
    ];

    pub(crate) fn parse(code: &str) -> Self {
        match code {
            "unauthenticated" => Self::Unauthenticated,
            "device_removed" => Self::DeviceRemoved,
            "refresh_reused" => Self::RefreshReused,
            "refresh_invalid" => Self::RefreshInvalid,
            "token_invalid" => Self::TokenInvalid,
            "token_expired" => Self::TokenExpired,
            "not_found" => Self::NotFound,
            "bad_request" => Self::BadRequest,
            "demo_unavailable" => Self::DemoUnavailable,
            "proxy_unavailable" => Self::ProxyUnavailable,
            "mailbox_not_found" => Self::MailboxNotFound,
            "key_limit" => Self::KeyLimit,
            "pair_expired" => Self::PairExpired,
            "key_not_found" => Self::KeyNotFound,
            "phone_not_found" => Self::PhoneNotFound,
            "session_invalid" => Self::SessionInvalid,
            "vtop_unreachable" => Self::VtopUnreachable,
            "session_expired" => Self::SessionExpired,
            "invalid_credentials" => Self::InvalidCredentials,
            "authentication_failed" => Self::AuthenticationFailed,
            "server_error" => Self::ServerError,
            "identity_failed" => Self::IdentityFailed,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Unauthenticated => "unauthenticated",
            Self::DeviceRemoved => "device_removed",
            Self::RefreshReused => "refresh_reused",
            Self::RefreshInvalid => "refresh_invalid",
            Self::TokenInvalid => "token_invalid",
            Self::TokenExpired => "token_expired",
            Self::NotFound => "not_found",
            Self::BadRequest => "bad_request",
            Self::DemoUnavailable => "demo_unavailable",
            Self::ProxyUnavailable => "proxy_unavailable",
            Self::MailboxNotFound => "mailbox_not_found",
            Self::KeyLimit => "key_limit",
            Self::PairExpired => "pair_expired",
            Self::KeyNotFound => "key_not_found",
            Self::PhoneNotFound => "phone_not_found",
            Self::SessionInvalid => "session_invalid",
            Self::VtopUnreachable => "vtop_unreachable",
            Self::SessionExpired => "session_expired",
            Self::InvalidCredentials => "invalid_credentials",
            Self::AuthenticationFailed => "authentication_failed",
            Self::ServerError => "server_error",
            Self::IdentityFailed => "identity_failed",
            Self::Unknown(code) => code,
        }
    }

    /// `auth:token` won't take the refresh or v2 token it was given: the token is dropped and
    /// the next source is tried. Any other failure (offline, a server fault) keeps it, so a
    /// passing fault never costs a new proof.
    pub(crate) fn is_token_refused(&self) -> bool {
        matches!(
            self,
            Self::DeviceRemoved
                | Self::RefreshReused
                | Self::RefreshInvalid
                | Self::TokenInvalid
                | Self::TokenExpired
        )
    }

    /// A sign-out answered this because there is nothing left to sign out of.
    pub(crate) fn is_signed_out_already(&self) -> bool {
        self.is_token_refused() || *self == Self::Unauthenticated
    }
}

impl std::fmt::Display for ServerCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn codes_roundtrip_and_unknown_ones_are_kept() {
        for code in ServerCode::KNOWN {
            assert_eq!(ServerCode::parse(code.as_str()), code);
        }
        let other = ServerCode::parse("stale_time");
        assert_eq!(other, ServerCode::Unknown("stale_time".into()));
        assert_eq!(other.as_str(), "stale_time");
    }

    #[test]
    fn the_shared_list_names_every_code() {
        let json: serde_json::Value = serde_json::from_str(include_str!(
            "../../../vtop-shared/vectors/server_codes.json"
        ))
        .unwrap();
        let listed: BTreeSet<&str> = ["backend", "engine", "phone"]
            .iter()
            .flat_map(|group| json[group].as_array().unwrap())
            .map(|code| code.as_str().unwrap())
            .collect();
        let known: BTreeSet<&str> = ServerCode::KNOWN.iter().map(ServerCode::as_str).collect();
        assert_eq!(listed, known);
        for code in &listed {
            assert!(
                !matches!(ServerCode::parse(code), ServerCode::Unknown(_)),
                "{code}"
            );
        }
    }

    #[test]
    fn refused_tokens_and_signed_out_already() {
        assert!(ServerCode::RefreshReused.is_token_refused());
        assert!(!ServerCode::ServerError.is_token_refused());
        assert!(!ServerCode::Unauthenticated.is_token_refused());
        assert!(ServerCode::Unauthenticated.is_signed_out_already());
        assert!(ServerCode::DeviceRemoved.is_signed_out_already());
        assert!(!ServerCode::NotFound.is_signed_out_already());
    }
}

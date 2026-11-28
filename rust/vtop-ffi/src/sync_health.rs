//! How background sync stands ([`SyncHealth`]): kept in the settings store as
//! `sync_paused` (a [`PauseReason`] id) and `sync_otp_misses`, and pushed to the app on every
//! change ([`crate::AppEvents::on_sync_health`]).

use std::sync::PoisonError;

use tracing::info;

use crate::settings::SettingChange;
use crate::{Core, ForwardingOffKind, span};

/// Why background sync is paused: [`PauseReason::text`] is shown ("Paused: …"),
/// [`PauseReason::id`] is saved.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum PauseReason {
    NotSetUp,
    GmailReconnect,
    GmailRefused { code: Option<String> },
    NoCode,
    CodeRefused,
    ForwardingOff { kind: Option<ForwardingOffKind> },
    PasswordRejected,
    OtpFailed,
}

const GMAIL_REFUSED: &str = "gmail_refused";
const FORWARDING_OFF: &str = "forwarding_off";

impl PauseReason {
    /// The refusal's code, trimmed; None when blank.
    fn clean(code: Option<&str>) -> Option<&str> {
        code.map(str::trim).filter(|code| !code.is_empty())
    }

    pub(crate) fn id(&self) -> String {
        match self {
            Self::NotSetUp => "not_set_up".into(),
            Self::GmailReconnect => "gmail_reconnect".into(),
            Self::GmailRefused { code } => match Self::clean(code.as_deref()) {
                Some(code) => format!("{GMAIL_REFUSED}:{code}"),
                None => GMAIL_REFUSED.into(),
            },
            Self::NoCode => "no_code".into(),
            Self::CodeRefused => "code_refused".into(),
            Self::ForwardingOff { kind } => match kind {
                None => FORWARDING_OFF.into(),
                Some(kind) => format!("{FORWARDING_OFF}:{}", kind_id(*kind)),
            },
            Self::PasswordRejected => "password_rejected".into(),
            Self::OtpFailed => "otp_failed".into(),
        }
    }

    pub(crate) fn text(&self) -> String {
        match self {
            Self::NotSetUp => "OTP auto-fill isn't set up".into(),
            Self::GmailReconnect => "Gmail needs reconnecting".into(),
            Self::GmailRefused { code } => match Self::clean(code.as_deref()) {
                Some(code) => format!("Gmail refused: {code}"),
                None => "Gmail refused the request".into(),
            },
            Self::NoCode => "No code came in time".into(),
            Self::CodeRefused => "VTOP didn't take the code".into(),
            Self::ForwardingOff { kind } => match kind {
                None | Some(ForwardingOffKind::NotSetUp) => "Mail forwarding is off".into(),
                Some(ForwardingOffKind::NeedsSignIn) => {
                    "Mail forwarding reconnects once you're signed in".into()
                }
                Some(ForwardingOffKind::AddressDeleted) => {
                    "Your forwarding address no longer exists. Set it up again".into()
                }
                Some(ForwardingOffKind::DeviceRemoved) => {
                    "This phone was removed from your account. Sign in again".into()
                }
            },
            Self::PasswordRejected => "VTOP turned down your password".into(),
            Self::OtpFailed => "The OTP couldn't be filled in".into(),
        }
    }

    /// Connecting Gmail again clears it.
    fn gmail(&self) -> bool {
        matches!(self, Self::GmailReconnect | Self::GmailRefused { .. })
    }

    /// Reads a saved id, or migrates the display text an older app saved.
    pub(crate) fn parse(saved: &str) -> Self {
        let simple = [
            Self::NotSetUp,
            Self::GmailReconnect,
            Self::NoCode,
            Self::CodeRefused,
            Self::PasswordRejected,
            Self::OtpFailed,
        ];
        if let Some(reason) = simple
            .into_iter()
            .find(|reason| reason.id() == saved || reason.text() == saved)
        {
            return reason;
        }
        if saved == GMAIL_REFUSED || saved == "Gmail refused the request" {
            return Self::GmailRefused { code: None };
        }
        if let Some(code) = saved.strip_prefix(&format!("{GMAIL_REFUSED}:")) {
            return Self::GmailRefused {
                code: Some(code.into()),
            };
        }
        if saved == FORWARDING_OFF || saved == "Mail forwarding is off" {
            return Self::ForwardingOff { kind: None };
        }
        if let Some(kind) = saved.strip_prefix(&format!("{FORWARDING_OFF}:")) {
            return Self::ForwardingOff {
                kind: parse_kind(kind),
            };
        }
        // One-time migration for older apps that saved free-text failure messages.
        let lower = saved.to_lowercase();
        let kind = |kind| Self::ForwardingOff { kind: Some(kind) };
        if let Some(code) = saved.strip_prefix("Gmail refused:") {
            Self::GmailRefused {
                code: Some(code.trim().into()),
            }
        } else if saved.starts_with("Gmail refused") {
            Self::GmailRefused { code: None }
        } else if lower.contains("removed from your account") {
            kind(ForwardingOffKind::DeviceRemoved)
        } else if lower.contains("no longer exists") {
            kind(ForwardingOffKind::AddressDeleted)
        } else if lower.contains("reconnects once") {
            kind(ForwardingOffKind::NeedsSignIn)
        } else if lower.contains("forwarding") {
            kind(ForwardingOffKind::NotSetUp)
        } else {
            Self::OtpFailed
        }
    }
}

fn kind_id(kind: ForwardingOffKind) -> &'static str {
    match kind {
        ForwardingOffKind::NotSetUp => "not_set_up",
        ForwardingOffKind::NeedsSignIn => "needs_sign_in",
        ForwardingOffKind::AddressDeleted => "address_deleted",
        ForwardingOffKind::DeviceRemoved => "device_removed",
    }
}

fn parse_kind(raw: &str) -> Option<ForwardingOffKind> {
    [
        ForwardingOffKind::NotSetUp,
        ForwardingOffKind::NeedsSignIn,
        ForwardingOffKind::AddressDeleted,
        ForwardingOffKind::DeviceRemoved,
    ]
    .into_iter()
    .find(|kind| kind_id(*kind) == raw)
}

/// How background sync stands: `Active` counts the runs in a row whose OTP code never came;
/// `Paused` says why, until a run goes through (runs still try while paused).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SyncHealth {
    Active { otp_misses: u32 },
    Paused { reason: PauseReason },
}

/// Runs in a row whose code never came before sync pauses.
const MISSES_TO_PAUSE: u32 = 2;

impl SyncHealth {
    pub(crate) const OK: Self = Self::Active { otp_misses: 0 };

    /// After a run's OTP auto-fill: `why` is its failure, None when the code went through
    /// (which says nothing until the run ends). VTOP mails late at times, so one code that
    /// never came is only that run's luck; [`MISSES_TO_PAUSE`] in a row pause.
    pub(crate) fn after_otp(self, why: Option<PauseReason>) -> Self {
        let Some(reason) = why else { return self };
        if reason != PauseReason::NoCode {
            return Self::Paused { reason };
        }
        match self {
            Self::Active { otp_misses } if otp_misses + 1 < MISSES_TO_PAUSE => Self::Active {
                otp_misses: otp_misses + 1,
            },
            _ => Self::Paused {
                reason: PauseReason::NoCode,
            },
        }
    }

    /// Active again when paused for a reason `cleared` covers (the student fixed it).
    pub(crate) fn resumed_if(self, cleared: SyncResume) -> Self {
        match &self {
            Self::Paused { reason } if cleared.covers(reason) => Self::OK,
            _ => self,
        }
    }

    /// From the saved values: `paused` (None or empty when not), `misses` the count.
    pub(crate) fn parse(paused: Option<&str>, misses: i32) -> Self {
        match paused.filter(|paused| !paused.is_empty()) {
            Some(saved) => Self::Paused {
                reason: PauseReason::parse(saved),
            },
            None => Self::Active {
                otp_misses: u32::try_from(misses).unwrap_or(0),
            },
        }
    }

    /// The saved values, as [`SyncHealth::parse`] reads them.
    fn stored(&self) -> (Option<String>, i32) {
        match self {
            Self::Active { otp_misses } => (None, i32::try_from(*otp_misses).unwrap_or(i32::MAX)),
            Self::Paused { reason } => (Some(reason.id()), 0),
        }
    }
}

/// What the student fixed, for [`Core::sync_health_resume_if`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SyncResume {
    /// Connected Gmail again.
    Gmail,
    /// Saved a new password.
    PasswordRejected,
}

impl SyncResume {
    fn covers(self, reason: &PauseReason) -> bool {
        match self {
            Self::Gmail => reason.gmail(),
            Self::PasswordRejected => *reason == PauseReason::PasswordRejected,
        }
    }
}

impl Core {
    /// Changes the health with `next`, saves and pushes it when it changed; the new one.
    pub(crate) fn change_sync_health(
        &self,
        flow: &str,
        next: impl FnOnce(SyncHealth) -> SyncHealth,
    ) -> SyncHealth {
        // One change at a time, so two runs' misses both count.
        let _changing = self
            .sync
            .health
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let was = self.sync_health();
        let now = next(was.clone());
        if now != was {
            let (paused, otp_misses) = now.stored();
            self.settings
                .set(SettingChange::SyncHealth { paused, otp_misses });
            span("sync_health", flow).in_scope(|| info!(health = ?now, "sync health"));
            self.tell_sync_health(now.clone());
        }
        now
    }

    /// Tells the app the current health (when it starts listening).
    pub(crate) fn push_sync_health(&self) {
        self.tell_sync_health(self.sync_health());
    }

    fn tell_sync_health(&self, now: SyncHealth) {
        let paused = match &now {
            SyncHealth::Paused { reason } => Some(reason.text()),
            SyncHealth::Active { .. } => None,
        };
        self.events.tell("on_sync_health", |events| {
            events.on_sync_health(now, paused)
        });
    }
}

#[uniffi::export]
impl Core {
    /// How background sync stands now.
    pub fn sync_health(&self) -> SyncHealth {
        let settings = self.settings();
        SyncHealth::parse(settings.sync_paused.as_deref(), settings.sync_otp_misses)
    }

    /// Sets it (sync turned on, or auto-fill not set up).
    pub fn set_sync_health(&self, flow: String, health: SyncHealth) -> SyncHealth {
        self.change_sync_health(&flow, |_| health)
    }

    /// After a run's OTP auto-fill ([`SyncHealth::after_otp`]).
    pub fn sync_health_after_otp(&self, flow: String, why: Option<PauseReason>) -> SyncHealth {
        if let Some(why) = &why {
            crate::obs::log_event(
                "info".into(),
                "sync".into(),
                Some(flow.clone()),
                format!("otp not solved: {}", why.id()),
                std::collections::HashMap::new(),
            );
        }
        self.change_sync_health(&flow, |health| health.after_otp(why))
    }

    /// Active again when paused for what the student just fixed.
    pub fn sync_health_resume_if(&self, flow: String, cleared: SyncResume) -> SyncHealth {
        self.change_sync_health(&flow, |health| health.resumed_if(cleared))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(code: &str) -> PauseReason {
        PauseReason::GmailRefused {
            code: Some(code.into()),
        }
    }

    fn off(kind: Option<ForwardingOffKind>) -> PauseReason {
        PauseReason::ForwardingOff { kind }
    }

    fn paused(reason: PauseReason) -> SyncHealth {
        SyncHealth::Paused { reason }
    }

    fn active(otp_misses: u32) -> SyncHealth {
        SyncHealth::Active { otp_misses }
    }

    #[test]
    fn parses_saved_values_and_old_texts() {
        assert_eq!(active(0), SyncHealth::parse(None, 0));
        assert_eq!(active(1), SyncHealth::parse(None, 1));
        assert_eq!(active(0), SyncHealth::parse(Some(""), -3));
        assert_eq!(
            paused(PauseReason::NoCode),
            SyncHealth::parse(Some("no_code"), 1)
        );
        assert_eq!(
            paused(PauseReason::GmailRefused { code: None }),
            SyncHealth::parse(Some("gmail_refused"), 0)
        );
        assert_eq!(
            paused(off(None)),
            SyncHealth::parse(Some("forwarding_off"), 0)
        );
        assert_eq!(
            paused(PauseReason::GmailReconnect),
            SyncHealth::parse(Some("Gmail needs reconnecting"), 0)
        );
        assert_eq!(
            paused(PauseReason::PasswordRejected),
            SyncHealth::parse(Some("VTOP turned down your password"), 0)
        );
        assert_eq!(
            paused(PauseReason::NotSetUp),
            SyncHealth::parse(Some("OTP auto-fill isn't set up"), 0)
        );
        assert_eq!(
            paused(refused("invalid_grant")),
            SyncHealth::parse(Some("Gmail refused: invalid_grant"), 0)
        );
        assert_eq!(
            paused(off(Some(ForwardingOffKind::AddressDeleted))),
            SyncHealth::parse(
                Some("Your forwarding address no longer exists. Set it up again."),
                0
            )
        );
        assert_eq!(
            paused(off(Some(ForwardingOffKind::DeviceRemoved))),
            SyncHealth::parse(
                Some("This phone was removed from your account. Sign in again."),
                0
            )
        );
        assert_eq!(
            paused(off(Some(ForwardingOffKind::NeedsSignIn))),
            SyncHealth::parse(Some("Mail forwarding reconnects once you're signed in."), 0)
        );
        assert_eq!(
            paused(PauseReason::OtpFailed),
            SyncHealth::parse(Some("something new"), 0)
        );
    }

    #[test]
    fn pause_reason_stored_roundtrip() {
        let all = [
            PauseReason::NotSetUp,
            PauseReason::GmailReconnect,
            PauseReason::GmailRefused { code: None },
            refused("invalid_grant"),
            PauseReason::NoCode,
            PauseReason::CodeRefused,
            off(None),
            off(Some(ForwardingOffKind::NotSetUp)),
            off(Some(ForwardingOffKind::NeedsSignIn)),
            off(Some(ForwardingOffKind::AddressDeleted)),
            off(Some(ForwardingOffKind::DeviceRemoved)),
            PauseReason::PasswordRejected,
            PauseReason::OtpFailed,
        ];
        for reason in all {
            assert_eq!(reason, PauseReason::parse(&reason.id()));
            let (saved, misses) = paused(reason.clone()).stored();
            assert_eq!(paused(reason), SyncHealth::parse(saved.as_deref(), misses));
        }
        let (saved, misses) = active(1).stored();
        assert_eq!(active(1), SyncHealth::parse(saved.as_deref(), misses));
    }

    #[test]
    fn texts_name_the_reason() {
        assert_eq!(
            "Gmail refused: invalid_grant",
            refused("invalid_grant").text()
        );
        assert_eq!(
            "Gmail refused the request",
            PauseReason::GmailRefused { code: None }.text()
        );
        assert_eq!(
            "This phone was removed from your account. Sign in again",
            off(Some(ForwardingOffKind::DeviceRemoved)).text()
        );
        assert_eq!(
            "Your forwarding address no longer exists. Set it up again",
            off(Some(ForwardingOffKind::AddressDeleted)).text()
        );
        assert_eq!(
            "Mail forwarding reconnects once you're signed in",
            off(Some(ForwardingOffKind::NeedsSignIn)).text()
        );
        assert_eq!(
            "Mail forwarding is off",
            off(Some(ForwardingOffKind::NotSetUp)).text()
        );
    }

    #[test]
    fn two_missed_codes_in_a_row_pause() {
        let once = SyncHealth::OK.after_otp(Some(PauseReason::NoCode));
        assert_eq!(active(1), once);
        assert_eq!(
            paused(PauseReason::NoCode),
            once.clone().after_otp(Some(PauseReason::NoCode))
        );
        assert_eq!(once, once.clone().after_otp(None));
        assert_eq!(
            paused(PauseReason::CodeRefused),
            once.after_otp(Some(PauseReason::CodeRefused))
        );
        assert_eq!(
            paused(PauseReason::NoCode),
            paused(refused("invalid_grant")).after_otp(Some(PauseReason::NoCode))
        );
    }

    #[test]
    fn resumes_only_for_what_was_fixed() {
        let gmail = paused(refused("invalid_grant"));
        assert_eq!(SyncHealth::OK, gmail.resumed_if(SyncResume::Gmail));
        let password = paused(PauseReason::PasswordRejected);
        assert_eq!(password, password.clone().resumed_if(SyncResume::Gmail));
        assert_eq!(
            SyncHealth::OK,
            password.resumed_if(SyncResume::PasswordRejected)
        );
        assert_eq!(active(1), active(1).resumed_if(SyncResume::Gmail));
    }
}

//! What went wrong, in words for the student: the one mapping from a [`CoreError`] to a
//! [`Problem`] (texts from the Flutter app's `app_errors.dart` and `common_toast.dart`).

use crate::CoreError;

/// How a [`Problem`] looks. `SemesterNeeded`: the app shows the picker instead of a toast.
/// `ConsentDenied`: the student said no to sending their VTOP cookie (or their registration
/// number); no toast, only the problem view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ProblemKind {
    Offline,
    VtopDown,
    SignInNeeded,
    OtpNeeded,
    OtpRejected,
    SemesterNeeded,
    ConsentDenied,
    Unexpected,
}

/// `title` and `message` (says what to do), and the `flow` it happened in. `user_initiated`: the
/// student asked for the work (a pull, a button, a retry, a submit); false for automatic work
/// (auto refresh, the sweep), which toasts only with Show background errors
/// ([`Problem::toasts`]).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Problem {
    pub kind: ProblemKind,
    pub title: String,
    pub message: String,
    pub flow: String,
    pub retryable: bool,
    #[uniffi(default = true)]
    pub user_initiated: bool,
}

const UNEXPECTED: &str =
    "Something went wrong. Try again; if it keeps happening, open Details and share the log.";

impl Problem {
    fn new(kind: ProblemKind, title: &str, message: impl Into<String>, flow: &str) -> Self {
        Self {
            kind,
            title: title.into(),
            message: message.into(),
            flow: flow.into(),
            retryable: true,
            user_initiated: true,
        }
    }

    fn not_retryable(mut self) -> Self {
        self.retryable = false;
        self
    }

    /// `error` in flow `flow`. `server`: the call went to our server, not VTOP, so its failures
    /// don't blame VTOP.
    pub(crate) fn of(error: &CoreError, flow: &str, server: bool) -> Self {
        use ProblemKind as K;
        let p = |kind, title, message: &str| Self::new(kind, title, message, flow);
        let or = |text: &str, fallback: &str| {
            let text = text.trim();
            if text.is_empty() { fallback } else { text }.to_owned()
        };
        match error {
            CoreError::Offline => p(
                K::Offline,
                "No Internet Connection",
                &format!(
                    "We couldn't reach {}. Check your internet connection and try again.",
                    if server { "the server" } else { "VTOP" }
                ),
            ),
            CoreError::VtopUnavailable { reason } => {
                let reason = reason.trim().trim_end_matches('.');
                let reason = if reason.is_empty() {
                    "VTOP is not answering"
                } else {
                    reason
                };
                p(
                    K::VtopDown,
                    "VTOP Is Down",
                    &format!(
                        "{reason}. This is on VTOP's side, not your connection; try again in a few minutes."
                    ),
                )
            }
            CoreError::InvalidCredentials => p(
                K::SignInNeeded,
                "VTOP Sign-In Failed",
                "VTOP rejected the saved password. Update it in Settings, under Your account.",
            )
            .not_retryable(),
            CoreError::NotSignedIn => p(
                K::SignInNeeded,
                "Sign-In Needed",
                "Your saved session expired. Please sign in again.",
            )
            .not_retryable(),
            CoreError::PasswordLocked => p(
                K::Unexpected,
                "Couldn't Open Your Password",
                "The phone's keystore didn't open the saved password just now. Try again in a moment.",
            ),
            CoreError::OtpRequired { .. } => p(
                K::OtpNeeded,
                "Verify Your Sign-In",
                "We need to verify this sign-in with the OTP sent to your registered email.",
            ),
            CoreError::OtpRejected { reason, .. } => p(
                K::OtpRejected,
                "OTP Not Accepted",
                &or(
                    reason,
                    "That OTP didn't work. Check the code, or ask for a new one, and try again.",
                ),
            ),
            CoreError::SemesterNeeded => p(
                K::SemesterNeeded,
                "Choose Your Semester",
                "Pick the semester to show in Settings → Account → Semester.",
            )
            .not_retryable(),
            CoreError::ConsentDenied => p(
                K::ConsentDenied,
                "Not Sent",
                "Not sent: your VTOP cookie stays on the phone.",
            ),
            // A "Not now" on the registration number's sheet: as quiet as the cookie's no.
            CoreError::RegnoNotShared => p(
                K::ConsentDenied,
                "Not Shared",
                "This needs your registration number on the vitapmate server. Share it from Settings → Connections.",
            ),
            CoreError::Other { reason } if server => {
                p(K::Unexpected, "Server Problem", &or(reason, UNEXPECTED))
            }
            CoreError::Other { .. } => p(K::Unexpected, "VTOP Problem", UNEXPECTED),
            CoreError::ForwardingOff { .. } => p(K::Unexpected, "Something Went Wrong", UNEXPECTED),
        }
    }

    /// A page that stopped loading before it finished (cancelled, or dropped from the queue).
    pub(crate) fn not_loaded(flow: &str) -> Self {
        Self {
            user_initiated: false,
            ..Self::new(
                ProblemKind::Unexpected,
                "Not Loaded",
                "This page stopped loading before it finished. Try again.",
                flow,
            )
        }
    }

    /// Whether this shows as a toast: never for an OTP, semester or consent prompt (they have
    /// their own UI), and automatic work's only with `show_background_errors`.
    pub(crate) fn toasts(&self, show_background_errors: bool) -> bool {
        !matches!(
            self.kind,
            ProblemKind::SemesterNeeded
                | ProblemKind::OtpNeeded
                | ProblemKind::OtpRejected
                | ProblemKind::ConsentDenied
        ) && (self.user_initiated || show_background_errors)
    }
}

/// [`Problem::of`] for Kotlin, which passes the `CoreException` it caught.
#[uniffi::export]
pub fn problem_of(error: CoreError, flow: String, server: bool) -> Problem {
    Problem::of(&error, &flow, server)
}

/// [`Problem::toasts`] for Kotlin.
#[uniffi::export]
pub fn problem_toasts(problem: Problem, show_background_errors: bool) -> bool {
    problem.toasts(show_background_errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OtpRejection;

    fn other(reason: &str) -> CoreError {
        CoreError::Other {
            reason: reason.into(),
        }
    }

    fn down(reason: &str) -> CoreError {
        CoreError::VtopUnavailable {
            reason: reason.into(),
        }
    }

    fn rejected(reason: &str) -> CoreError {
        CoreError::OtpRejected {
            kind: OtpRejection::Wrong,
            reason: reason.into(),
        }
    }

    fn kind(error: CoreError) -> ProblemKind {
        Problem::of(&error, "f1", false).kind
    }

    #[test]
    fn every_core_error_has_a_kind() {
        use ProblemKind as K;
        assert_eq!(kind(CoreError::Offline), K::Offline);
        assert_eq!(kind(down("VTOP returned 502")), K::VtopDown);
        assert_eq!(kind(CoreError::InvalidCredentials), K::SignInNeeded);
        assert_eq!(kind(CoreError::NotSignedIn), K::SignInNeeded);
        let otp = CoreError::OtpRequired {
            reason: "otp sent".into(),
            issued_at: None,
        };
        assert_eq!(kind(otp), K::OtpNeeded);
        assert_eq!(kind(rejected("Invalid OTP")), K::OtpRejected);
        assert_eq!(kind(CoreError::SemesterNeeded), K::SemesterNeeded);
        assert_eq!(kind(other("parse error")), K::Unexpected);
        // Kotlin's non-CoreException case; ForwardingOff took that branch too.
        let off = CoreError::ForwardingOff {
            kind: crate::ForwardingOffKind::NotSetUp,
            reason: "off".into(),
        };
        let off = Problem::of(&off, "f1", false);
        assert_eq!(off.kind, K::Unexpected);
        assert_eq!(off.title, "Something Went Wrong");
    }

    #[test]
    fn carries_the_flow_and_retryable() {
        let p = Problem::of(&CoreError::Offline, "abc123", false);
        assert_eq!(p.flow, "abc123");
        assert!(p.retryable);
        assert!(!Problem::of(&CoreError::InvalidCredentials, "x", false).retryable);
        assert!(!Problem::of(&CoreError::NotSignedIn, "x", false).retryable);
    }

    #[test]
    fn vtop_down_uses_the_reason_or_a_default() {
        assert_eq!(
            Problem::of(&down("VTOP returned 502."), "f", false).message,
            "VTOP returned 502. This is on VTOP's side, not your connection; try again in a few minutes."
        );
        assert!(
            Problem::of(&down("  "), "f", false)
                .message
                .starts_with("VTOP is not answering.")
        );
    }

    #[test]
    fn vtop_down_trims_reason_and_falls_back() {
        assert_eq!(
            Problem::of(&down("  Gateway timeout...  "), "f", false).message,
            "Gateway timeout. This is on VTOP's side, not your connection; try again in a few minutes."
        );
        assert_eq!(
            Problem::of(&down(" ... "), "f", false).message,
            "VTOP is not answering. This is on VTOP's side, not your connection; try again in a few minutes."
        );
    }

    #[test]
    fn otp_rejected_shows_vtop_text_or_a_default() {
        assert_eq!(
            Problem::of(&rejected(" Invalid OTP "), "f", false).message,
            "Invalid OTP"
        );
        assert!(
            Problem::of(&rejected(""), "f", false)
                .message
                .starts_with("That OTP didn't work")
        );
    }

    #[test]
    fn only_user_initiated_problems_toast_unless_background_errors_are_on() {
        let offline = Problem::of(&CoreError::Offline, "f", false);
        let otp = Problem::of(
            &CoreError::OtpRequired {
                reason: "otp sent".into(),
                issued_at: None,
            },
            "f",
            false,
        );
        let background = |p: &Problem| Problem {
            user_initiated: false,
            ..p.clone()
        };
        assert!(offline.toasts(false));
        assert!(!background(&offline).toasts(false));
        assert!(!otp.toasts(false));
        assert!(background(&offline).toasts(true));
        assert!(!background(&otp).toasts(true));
    }

    #[test]
    fn unexpected_never_shows_internals() {
        let p = Problem::of(&other("serde: missing field `x` at line 3"), "f", false);
        assert!(!p.message.contains("serde"));
    }

    #[test]
    fn server_failures_blame_the_server_and_keep_their_flow() {
        let offline = Problem::of(&CoreError::Offline, "s1", true);
        assert_eq!(offline.flow, "s1");
        assert!(offline.message.contains("the server") && !offline.message.contains("VTOP"));
        assert_eq!(
            Problem::of(&other(" Proof rejected "), "s2", true).message,
            "Proof rejected"
        );
        assert!(
            Problem::of(&other(""), "s3", true)
                .message
                .starts_with("Something went wrong")
        );
    }

    #[test]
    fn server_other_shows_reason() {
        let server = Problem::of(&other("Rate limited"), "s", true);
        assert_eq!(
            (server.title.as_str(), server.message.as_str()),
            ("Server Problem", "Rate limited")
        );
        let vtop = Problem::of(&other("Rate limited"), "s", false);
        assert_eq!(
            (vtop.title.as_str(), vtop.message.as_str()),
            ("VTOP Problem", UNEXPECTED)
        );
    }

    #[test]
    fn not_loaded_is_quiet_and_retryable() {
        let p = Problem::not_loaded("f");
        assert_eq!(p.title, "Not Loaded");
        assert!(p.retryable && !p.user_initiated);
        assert!(!p.toasts(false));
    }
}

//! Stand-ins for VTOP, for this crate's tests.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use vtop_core::SessionState;

use crate::login::{ForwardedLogin, LoginError, OtpCodes};

pub fn session(cookies: &str) -> SessionState {
    SessionState {
        cookies: cookies.to_string(),
        csrf_token: None,
        registration_number: None,
        otp_issued_at: None,
        logged_in_at: None,
    }
}

/// Answers sign-ins from a queue (then with the last answer).
pub struct FakeLogin {
    pub results: Mutex<VecDeque<Result<SessionState, LoginError>>>,
    pub calls: AtomicUsize,
}

impl FakeLogin {
    pub fn answering(result: Result<SessionState, LoginError>) -> Self {
        Self {
            results: Mutex::new(VecDeque::from([result])),
            calls: AtomicUsize::new(0),
        }
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Default for FakeLogin {
    /// Signs in as 22BCE0001.
    fn default() -> Self {
        Self::answering(Ok(SessionState {
            registration_number: Some("22BCE0001".into()),
            ..session("JSESSIONID=fresh")
        }))
    }
}

/// Asks `codes` once (as a real sign-in would when VTOP wants an OTP)
/// unless the queued answer is `InvalidCredentials`, then answers from the
/// queue; with no code it fails as a real sign-in would.
#[async_trait]
impl ForwardedLogin for FakeLogin {
    async fn login(
        &self,
        _username: &str,
        _password: &str,
        codes: &dyn OtpCodes,
        wait: Duration,
    ) -> Result<SessionState, LoginError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let next = self.results.lock().unwrap().front().cloned();
        if !matches!(next, Some(Err(LoginError::InvalidCredentials)))
            && codes
                .next(vtop_core::now_unix(), wait, Vec::new())
                .await
                .is_none()
        {
            return Err(LoginError::Failed(
                "otp_not_found: the OTP mail did not arrive".into(),
            ));
        }
        let mut results = self.results.lock().unwrap();
        if results.len() > 1 {
            results.pop_front().unwrap()
        } else {
            results.front().cloned().expect("FakeLogin has no answer")
        }
    }
}

/// Reads the owner from the cookie header: `REG=<number>` names it,
/// `expired` means VTOP rejects it, `down` means VTOP is unreachable.
#[derive(Default)]
pub struct FakeVerifier;

#[async_trait]
impl crate::verify::SessionVerifier for FakeVerifier {
    async fn registration_number(
        &self,
        session: &SessionState,
    ) -> Result<String, crate::verify::VerifyError> {
        use crate::verify::VerifyError;
        if session.cookies.contains("down") {
            return Err(VerifyError::Unavailable("VTOP unreachable".into()));
        }
        session
            .cookies
            .split(';')
            .find_map(|part| part.trim().strip_prefix("REG="))
            .map(str::to_uppercase)
            .ok_or(VerifyError::Expired)
    }
}

/// Hands out a token for any grant, or refuses them all as revoked.
#[derive(Default)]
pub struct FakeRefresher {
    pub revoked: bool,
    pub calls: AtomicUsize,
}

#[async_trait]
impl crate::gmail::GmailRefresher for FakeRefresher {
    async fn refresh(
        &self,
        _grant: &crate::gmail::GmailGrant,
    ) -> Result<crate::gmail::FreshToken, crate::gmail::RefreshError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.revoked {
            return Err(crate::gmail::RefreshError::InvalidGrant);
        }
        Ok(crate::gmail::FreshToken {
            access_token: "access".into(),
            expires_at: vtop_core::now_unix() + 3600,
        })
    }
}

//! Stand-ins for Google and VTOP, for this crate's tests and for
//! integration tests.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use vtop_core::gmail::GmailAccess;
use vtop_core::SessionState;

use crate::gmail::{FreshToken, GmailGrant, GmailRefresher, RefreshError};
use crate::login::{LoginError, VtopLogin};

pub fn session(cookies: &str) -> SessionState {
    SessionState {
        cookies: cookies.to_string(),
        csrf_token: None,
        registration_number: None,
        otp_issued_at: None,
        logged_in_at: None,
    }
}

/// Answers every refresh with `result`.
pub struct FakeRefresher {
    pub result: Mutex<Result<(), RefreshError>>,
    pub calls: AtomicUsize,
}

impl Default for FakeRefresher {
    fn default() -> Self {
        Self {
            result: Mutex::new(Ok(())),
            calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl GmailRefresher for FakeRefresher {
    async fn refresh(&self, _grant: &GmailGrant) -> Result<FreshToken, RefreshError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.result.lock().unwrap().clone().map(|()| FreshToken {
            access_token: "access".into(),
            expires_at: vtop_core::now_unix() + 3599,
        })
    }
}

/// Answers logins from a queue (then with the last answer), after `delay`.
pub struct FakeLogin {
    pub results: Mutex<VecDeque<Result<SessionState, LoginError>>>,
    pub calls: AtomicUsize,
    pub delay: Duration,
}

impl FakeLogin {
    pub fn answering(result: Result<SessionState, LoginError>) -> Self {
        Self {
            results: Mutex::new(VecDeque::from([result])),
            calls: AtomicUsize::new(0),
            delay: Duration::ZERO,
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

#[async_trait]
impl VtopLogin for FakeLogin {
    async fn login(
        &self,
        _username: &str,
        _password: &str,
        _gmail: GmailAccess,
        _wait: Duration,
    ) -> Result<SessionState, LoginError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(self.delay).await;
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

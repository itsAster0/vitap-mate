//! The OTP prompt. When a sign-in a student watches needs the emailed code, the call waits here
//! for the prompt's outcome instead of failing: the app's sheet shows the [`OtpChallenge`]
//! ([`OtpListener`]), and a code accepted anywhere (the sheet, auto-fill, another route) lets
//! every waiting call go on. A call nobody watches (background sync, the open-app sweep) gets
//! `OtpRequired` at once, as before, and opens no prompt.
//!
//! The VTOP challenge itself is the session's phase ([`crate::SessionPhase`]); the prompt is
//! the [`Board`]'s, opened by the first waiting call and closed only when the session signs in,
//! its password is rejected, the student cancels or the session goes (sign-out, another
//! account). A fresh sign-in under an open prompt (the code's sign-in had ended) keeps it open
//! with the same id.

use std::future::poll_fn;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Poll, Waker};
use std::time::Duration;

use futures::future::{Either, select};
use tracing::{info, warn};
use vtop_core::sync::sleep;

use crate::{Core, CoreError, Slot, traced};

/// How long a call waits on an open prompt before it gives up with `OtpRequired` (the prompt
/// stays open, and the page's retry goes through once the code is in).
const WAIT: Duration = Duration::from_secs(10 * 60);

/// The security-OTP challenge the app's sheet shows.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OtpChallenge {
    /// The prompt's: the same while it stays open, through a resent code or a fresh sign-in.
    pub id: u64,
    /// VTOP's text.
    pub message: String,
    /// When VTOP mailed the current code (unix seconds); None when unknown.
    pub issued_at: Option<u64>,
}

/// Told the prompt as it opens, changes and closes (None), outside any lock. The app sets one
/// ([`crate::Platform`]); it is told the current one when set.
#[uniffi::export(with_foreign)]
pub trait OtpListener: Send + Sync {
    fn on_challenge(&self, challenge: Option<OtpChallenge>);
}

/// The open prompt, if any, and a counter of session changes waiting calls wake on.
#[derive(Default)]
pub(crate) struct Board(Mutex<State>);

#[derive(Default)]
struct State {
    /// Bumped on every phase change of any session; waiting calls look again.
    version: u64,
    shown: Option<Shown>,
    next_id: u64,
    next_session: u64,
    wakers: Vec<Waker>,
    listener: Option<Arc<dyn OtpListener>>,
}

/// The prompt and the session ([`crate::Session::otp_session`]) it is for.
struct Shown {
    session: u64,
    challenge: OtpChallenge,
}

impl Board {
    fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A new session's id, for [`Board::prompt`] and [`Board::close`].
    pub(crate) fn session(&self) -> u64 {
        let mut state = self.state();
        state.next_session += 1;
        state.next_session
    }

    pub(crate) fn set_listener(&self, listener: Arc<dyn OtpListener>) {
        let shown = {
            let mut state = self.state();
            state.listener = Some(listener.clone());
            state.challenge()
        };
        listener.on_challenge(shown);
    }

    /// `change`d, the version bumped and waiting calls woken; the listener told when the
    /// prompt changed.
    fn update(&self, change: impl FnOnce(&mut State)) {
        let (wakers, told) = {
            let mut state = self.state();
            let before = state.challenge();
            change(&mut state);
            state.version += 1;
            let after = state.challenge();
            let told = (before != after)
                .then(|| state.listener.clone().map(|l| (l, after)))
                .flatten();
            (std::mem::take(&mut state.wakers), told)
        };
        wakers.into_iter().for_each(Waker::wake);
        if let Some((listener, challenge)) = told {
            listener.on_challenge(challenge);
        }
    }

    /// A session's phase changed: waiting calls look again.
    pub(crate) fn touch(&self) {
        self.update(|_| {});
    }

    /// Opens the prompt for `session`'s challenge, or keeps it open with this text and time.
    fn prompt(&self, session: u64, message: &str, issued_at: Option<u64>) {
        self.update(|state| {
            let id = match &state.shown {
                Some(shown) if shown.session == session => shown.challenge.id,
                _ => {
                    state.next_id += 1;
                    state.next_id
                }
            };
            state.shown = Some(Shown {
                session,
                challenge: OtpChallenge {
                    id,
                    message: message.to_owned(),
                    issued_at,
                },
            });
        });
    }

    /// `session` began a new challenge: an open prompt for it shows the new one.
    pub(crate) fn challenged(&self, session: u64, message: &str, issued_at: Option<u64>) {
        if self.is_shown(session) {
            self.prompt(session, message, issued_at);
        } else {
            self.touch();
        }
    }

    /// `session`'s prompt closes (signed in, password rejected, cancelled).
    pub(crate) fn close(&self, session: u64) {
        self.update(|state| {
            if state
                .shown
                .as_ref()
                .is_some_and(|shown| shown.session == session)
            {
                state.shown = None;
            }
        });
    }

    /// Any prompt closes: the session went (sign-out, another account).
    pub(crate) fn close_all(&self) {
        self.update(|state| state.shown = None);
    }

    fn is_shown(&self, session: u64) -> bool {
        self.state()
            .shown
            .as_ref()
            .is_some_and(|shown| shown.session == session)
    }

    fn version(&self) -> u64 {
        self.state().version
    }

    /// Waits until the version is past `seen`.
    async fn changed(&self, seen: u64) {
        poll_fn(|context| {
            let mut state = self.state();
            if state.version != seen {
                return Poll::Ready(());
            }
            if !state.wakers.iter().any(|w| w.will_wake(context.waker())) {
                state.wakers.push(context.waker().clone());
            }
            Poll::Pending
        })
        .await;
    }
}

impl State {
    fn challenge(&self) -> Option<OtpChallenge> {
        self.shown.as_ref().map(|shown| shown.challenge.clone())
    }
}

#[uniffi::export]
impl Core {
    /// Waits for the OTP of the sign-in just started (`sign_in` said `NeedsOtp`) or of the
    /// prompt open now: opens the prompt, then Ok once the session is signed in.
    /// `OtpRequired` when nothing waits for a code (the student closed the prompt), it stayed
    /// open over ten minutes or nobody watches `flow`; `InvalidCredentials` or `NotSignedIn` as
    /// for a fetch.
    pub async fn await_otp(&self, flow: String) -> Result<(), CoreError> {
        traced("await_otp", &flow, async {
            let slot = self.current()?;
            let (logins, failed) = {
                let session = slot.read().await;
                let failed = CoreError::OtpRequired {
                    reason: session
                        .otp_message()
                        .unwrap_or("The OTP prompt was closed.")
                        .into(),
                    issued_at: session.client.otp_issued_at(),
                };
                (session.logins, failed)
            };
            self.wait_for_otp(&slot, &flow, logins, failed).await
        })
        .await
    }
}

impl Core {
    /// After signing `slot` in again `failed` with `OtpRequired` (its session at `logins`
    /// sign-ins then): for a call nobody watches, that error. For one the student watches
    /// (`flow` attended), opens the prompt and waits: Ok once the session signed in (the
    /// call runs again), else what ended the wait.
    pub(crate) async fn wait_for_otp(
        &self,
        slot: &Arc<Slot>,
        flow: &str,
        logins: u64,
        failed: CoreError,
    ) -> Result<(), CoreError> {
        self.wait_for_otp_until(slot, flow, logins, failed, sleep(WAIT))
            .await
    }

    /// [`Core::wait_for_otp`], giving up when `deadline` is done.
    pub(crate) async fn wait_for_otp_until(
        &self,
        slot: &Arc<Slot>,
        flow: &str,
        logins: u64,
        failed: CoreError,
        deadline: impl Future<Output = ()>,
    ) -> Result<(), CoreError> {
        if !crate::remote::consent::attended(&self.shared, flow) {
            return Err(failed);
        }
        let board = &self.shared.otp;
        let session = {
            let session = slot.read().await;
            if let Some(outcome) = self.waited(slot, &session, logins) {
                return outcome;
            }
            match session.otp_message() {
                Some(message) => {
                    board.prompt(session.otp_session, message, session.client.otp_issued_at());
                }
                // A fresh sign-in under the prompt failed (offline): it stays for a retry.
                None if board.is_shown(session.otp_session) => {}
                None => return Err(failed), // Closed meanwhile.
            }
            session.otp_session
        };
        info!("waiting for the otp");
        let mut deadline = std::pin::pin!(deadline);
        loop {
            let seen = board.version();
            {
                let current = slot.read().await;
                if let Some(outcome) = self.waited(slot, &current, logins) {
                    return outcome;
                }
            }
            if !board.is_shown(session) {
                info!("otp prompt closed");
                return Err(failed);
            }
            let change = std::pin::pin!(board.changed(seen));
            if let Either::Right(_) = select(change, deadline.as_mut()).await {
                warn!(wait_s = WAIT.as_secs(), "gave up waiting for the otp");
                return Err(failed);
            }
        }
    }

    /// How a wait on `session`'s OTP ended, if it did: signed in since `logins`, the password
    /// rejected, or the session gone.
    fn waited(
        &self,
        slot: &Arc<Slot>,
        session: &crate::Session,
        logins: u64,
    ) -> Option<Result<(), CoreError>> {
        if !self.is_current(slot) {
            Some(Err(CoreError::NotSignedIn))
        } else if session.logins != logins {
            info!("otp answered");
            Some(Ok(()))
        } else if session.is_password_rejected() {
            Some(Err(CoreError::InvalidCredentials))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;

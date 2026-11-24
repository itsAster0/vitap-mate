//! Sign-in, the OTP prompt, signing in again when VTOP ends the session, and sign-out.

use std::sync::Arc;
use std::time::SystemTime;

use tracing::{info, warn};
use vtop_core::VtopError;
use vtop_core::sync::RwLock;
use vtop_core::types::PersistedVtopSession;

use crate::remote::route;
use crate::{
    ACCOUNT, Account, Core, CoreError, OtpVia, SESSION, STUDENT_FILES, SavedAccount, Session,
    SignIn, Slot, demo, obs, traced, traced_sync, unix_now,
};

#[uniffi::export]
impl Core {
    /// The saved account's username; None without a saved password (one restored from
    /// another phone, which the student types again).
    pub fn signed_in_user(&self, flow: String) -> Option<String> {
        traced_sync("signed_in_user", &flow, || Ok(self.read_account()))
            .ok()
            .flatten()
            .filter(|account| !account.password.is_empty())
            .map(|account| account.username)
    }

    /// The saved username and whether VTOP rejected the password, for Settings' "Needs
    /// attention" and the Gmail sign-in's registration number. The password is
    /// [`Core::saved_password`]'s.
    pub fn saved_account(&self, flow: String) -> Option<SavedAccount> {
        traced_sync("saved_account", &flow, || Ok(self.read_account()))
            .ok()
            .flatten()
            .map(|account| SavedAccount {
                username: account.username,
                rejected: account.rejected,
                // An account saved before this field: the saved session has it.
                registration_number: account.registration_number.or_else(|| {
                    self.read::<PersistedVtopSession>(SESSION)
                        .and_then(|saved| saved.registration_number)
                }),
            })
    }

    /// The saved password, for the Account page only (it opens behind the phone's screen
    /// lock, and fills it in); None when absent. Never logged.
    pub fn saved_password(&self, flow: String) -> Option<String> {
        traced_sync("saved_password", &flow, || Ok(self.read_account()))
            .ok()
            .flatten()
            .map(|account| account.password)
            .filter(|password| !password.is_empty())
    }

    pub async fn sign_in(
        &self,
        flow: String,
        username: String,
        password: String,
    ) -> Result<SignIn, CoreError> {
        let account = Account {
            username: username.trim().to_uppercase(),
            password,
            rejected: false,
            registration_number: None,
        };
        traced("sign_in", &flow, async {
            if demo::is(&account) && account.password == demo::PASSWORD {
                return self.sign_in_demo(&flow, account).await;
            }
            let mut session = Session::new(account, &self.shared)?;
            let outcome = match session.client.login().await {
                Ok(()) => SignIn::Done,
                Err(VtopError::OTPRequired(message, _)) => {
                    session.begin_otp(OtpVia::Phone, message.clone());
                    SignIn::NeedsOtp { message }
                }
                Err(error) => return Err(error.into()),
            };
            // VTOP asks for the OTP only after it accepted the password. For the saved
            // account (the Account page fixing a password), keep the new password now, so
            // a cancelled OTP doesn't leave the old, rejected one on disk.
            if outcome != SignIn::Done {
                self.keep_accepted_password(&session.account);
            }
            // Accepted: its cookies become the app's (and the web view's).
            session.cookies.activate(&self.shared.cookies, true);
            let slot = Arc::new(RwLock::new(session));
            self.install(Some(slot.clone()));
            if outcome == SignIn::Done {
                let session = slot.read().await;
                obs::set_regno(session.client.registration_number().ok());
                self.save(&slot, &session);
            }
            Ok(outcome)
        })
        .await
    }

    pub async fn submit_otp(&self, flow: String, code: String) -> Result<(), CoreError> {
        traced("submit_otp", &flow, async {
            let slot = self.current()?;
            if let Some(challenge_id) = slot.read().await.server_otp_challenge() {
                return self
                    .server_otp(&slot, challenge_id, Some(code.trim()), &flow)
                    .await;
            }
            let mut session = slot.write().await;
            if session.client.otp_issued_at().is_none() {
                // Another call (auto-fill, the prompt) already got in with a code.
                if !session.is_awaiting_otp() && session.client.is_authenticated() {
                    info!("otp no longer needed: signed in");
                    return Ok(());
                }
                return self.fresh_challenge(&slot, &mut session).await;
            }
            session
                .client
                .submit_security_otp(code.trim())
                .await
                .map_err(|error| match error {
                    VtopError::AuthenticationFailed(reason) => CoreError::OtpRejected { reason },
                    error => error.into(),
                })?;
            session.mark_signed_in();
            obs::set_regno(session.client.registration_number().ok());
            self.save(&slot, &session);
            Ok(())
        })
        .await
    }

    pub async fn resend_otp(&self, flow: String) -> Result<(), CoreError> {
        traced("resend_otp", &flow, async {
            let slot = self.current()?;
            if let Some(challenge_id) = slot.read().await.server_otp_challenge() {
                return self.server_otp(&slot, challenge_id, None, &flow).await;
            }
            let mut session = slot.write().await;
            if session.client.otp_issued_at().is_none() {
                // Signing in afresh mails the new code.
                return match self.fresh_challenge(&slot, &mut session).await {
                    Err(CoreError::OtpRejected { .. }) => Ok(()),
                    result => result,
                };
            }
            Ok(session.client.resend_security_otp().await?)
        })
        .await
    }

    /// Drops a re-login's pending OTP challenge (the student cancelled the prompt), so the next
    /// fetch that needs a session signs in afresh and VTOP mails a new code, instead of reopening
    /// a challenge VTOP may have dropped meanwhile. Nothing pending: nothing changes.
    pub async fn cancel_otp(&self, flow: String) -> Result<(), CoreError> {
        traced("cancel_otp", &flow, async {
            let slot = self.current()?;
            let mut session = slot.write().await;
            if session.is_awaiting_otp() {
                session.restart(&self.shared)?;
                info!("pending otp dropped");
            }
            Ok(())
        })
        .await
    }

    /// Signs in again when the session signed in over `max_age_secs` ago (or when that is
    /// unknown), so a long fetch starts on a fresh one; a newer session asks VTOP nothing.
    /// `OtpRequired` when VTOP wants the emailed code.
    pub async fn ensure_fresh_session(
        &self,
        flow: String,
        max_age_secs: u64,
    ) -> Result<(), CoreError> {
        traced("ensure_fresh_session", &flow, async {
            let slot = self.current()?;
            let mut session = slot.write().await;
            if demo::is(&session.account) {
                return Ok(());
            }
            let age = session
                .client
                .logged_in_at()
                .map(|at| (unix_now() as u64).saturating_sub(at));
            if age.is_some_and(|age| age <= max_age_secs) {
                return Ok(());
            }
            info!(age, max_age_secs, "session older than asked");
            self.sign_in_again(&slot, &mut session).await
        })
        .await
    }

    /// Forgets the account, session, cache, change inbox, connection choice and mail-forwarding address, and queues the
    /// address to be left on the edge (a sign-in can join it again there). Logs stay.
    pub fn sign_out(&self, flow: String) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("sign_out", &flow, || {
            self.install(None);
            // Left on the edge by `leave_old_forwarding`, which the app calls next.
            self.queue_leave();
            self.shared.cookies.sign_out();
            self.forget(&[&STUDENT_FILES[..], &[ACCOUNT, SESSION, route::FILE]].concat());
            route::forget(&self.shared);
            obs::set_regno(None);
            Ok(())
        });
    }
}

impl Core {
    /// The client has no OTP challenge left to take a code for (the app restarted, or a call
    /// dropped it): signs in afresh. Ok when VTOP lets it in without a code; when VTOP mails a
    /// new one, `OtpRejected` saying so, so the prompt stays open and auto-fill takes the new
    /// code instead of failing on a challenge that is gone.
    pub(crate) async fn fresh_challenge(
        &self,
        slot: &Arc<Slot>,
        session: &mut Session,
    ) -> Result<(), CoreError> {
        warn!("no otp challenge left to answer; signing in again");
        session.restart(&self.shared)?;
        match session.client.login().await {
            Ok(()) => {
                session.mark_signed_in();
                obs::set_regno(session.client.registration_number().ok());
                self.save(slot, session);
                Ok(())
            }
            Err(VtopError::OTPRequired(message, _)) => {
                session.begin_otp(OtpVia::Phone, message);
                Err(CoreError::OtpRejected {
                    reason: "That sign-in had ended. VTOP sent a new code; use the newest email."
                        .into(),
                })
            }
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) async fn relogin(&self, slot: &Arc<Slot>, seen: u64) -> Result<(), CoreError> {
        let mut session = slot.write().await;
        if session.logins != seen {
            return Ok(()); // Another call already signed in again.
        }
        info!("session expired");
        self.sign_in_again(slot, &mut session).await
    }

    /// Signs `session` in again with the saved password; an open OTP challenge or a
    /// rejected password answers without asking VTOP.
    pub(crate) async fn sign_in_again(
        &self,
        slot: &Arc<Slot>,
        session: &mut Session,
    ) -> Result<(), CoreError> {
        if !self.is_current(slot) {
            // Replaced or signed out meanwhile: its cookies are gone; never sign it in.
            return Err(CoreError::NotSignedIn);
        }
        if let Some(message) = session.otp_message() {
            // The challenge still open (no new mail): its issue time, so auto-fill knows how
            // long its code is good.
            return Err(CoreError::OtpRequired {
                reason: message.to_owned(),
                issued_at: session.client.otp_issued_at(),
            });
        }
        if session.is_password_rejected() {
            info!("the saved password was rejected, not signing in again");
            return Err(CoreError::InvalidCredentials);
        }
        info!("signing in again");
        if let Err(error) = session.client.login().await {
            match &error {
                VtopError::OTPRequired(message, _) => {
                    session.begin_otp(OtpVia::Phone, message.clone());
                }
                VtopError::InvalidCredentials => {
                    warn!("VTOP rejected the saved password; no more sign-ins with it");
                    session.mark_password_rejected();
                    if self.is_current(slot) {
                        self.write_account(&session.account);
                    }
                }
                _ => {}
            }
            return Err(error.into());
        }
        session.mark_signed_in();
        self.save(slot, session);
        Ok(())
    }

    pub(crate) fn save(&self, slot: &Arc<Slot>, session: &Session) {
        if !self.is_current(slot) {
            return;
        }
        // Another student signed in over the saved one ("Sign in again"): the last one's
        // semesters, pick and pages are not theirs.
        if self.read_account().is_some_and(|saved| {
            !saved
                .username
                .eq_ignore_ascii_case(&session.account.username)
        }) {
            info!("another account signed in: cached pages and forwarding address dropped");
            self.queue_leave();
            self.forget(&STUDENT_FILES);
            // The JWT names the last student: set aside to sign out of their account, while the
            // next server call proves this one.
            self.set_aside_identity();
        }
        let saved_at_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis() as u64);
        let mut account = session.account.clone();
        if let Ok(regno) = session.client.registration_number() {
            account.registration_number = Some(regno);
        }
        self.write_account(&account);
        #[expect(
            clippy::disallowed_methods,
            reason = "saved to session.json on the phone; nothing leaves it"
        )]
        let mut saved = session.client.export_persisted_session(saved_at_ms);
        if session.cookies.is_shared() {
            // The shared store keeps (and saves) them.
            saved.cookies = None;
            session.cookies.flush();
        }
        self.write(SESSION, &saved);
    }

    /// Restores `saved` into `session` (not installed yet) and makes it the shared store's
    /// session. Without cookies in `saved`, the store's are this session's and stay; a
    /// `session.json` from before the shared store (cookies in it) moves them there once and is
    /// saved again without; no `saved`: the store starts empty.
    pub(crate) fn restore(&self, session: &Session, saved: Option<PersistedVtopSession>) {
        let carry = saved.as_ref().is_none_or(|saved| saved.cookies.is_some());
        if let Some(saved) = &saved {
            obs::set_regno(saved.registration_number.clone());
            match session.client.restore_persisted_session(saved) {
                Ok(()) => info!(
                    cookies_in_file = saved.cookies.is_some(),
                    "session restored"
                ),
                Err(error) => warn!(%error, "saved session not restored"),
            }
        }
        session.cookies.activate(&self.shared.cookies, carry);
        if let Some(mut saved) = saved
            && saved.cookies.is_some()
            && session.cookies.is_shared()
        {
            saved.cookies = None;
            self.write(SESSION, &saved);
            info!("saved cookies moved to the shared store");
        }
    }

    /// Saves `account`'s password over the saved account's when it's the same username
    /// (another username is another account, saved only once signed in).
    pub(crate) fn keep_accepted_password(&self, account: &Account) {
        if let Some(saved) = self.read_account()
            && saved.username == account.username
            && (saved.password != account.password || saved.rejected)
        {
            info!("password accepted before the otp: saved");
            self.write_account(account);
        }
    }
}

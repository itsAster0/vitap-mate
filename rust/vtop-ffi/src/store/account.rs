//! `account.json`: the saved sign-in, its password sealed by [`PasswordCipher`] (an Android
//! Keystore key) so the file holds only ciphertext. A file from before the cipher (the password
//! in plain text) is sealed and written back once, when the core opens ([`Core::upgrade_account`]).
//! The key never leaves the phone, nor survives an uninstall: a copy restored on another phone (or
//! after a reinstall) can't be opened, so its password is gone and the student types it again. A
//! Keystore that fails for a moment is not that: the password stays on disk and is opened again
//! on the next use. Neither the password nor the sealed text is ever logged.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{ACCOUNT, Account, Core, Slot, demo, traced_sync};

/// [`PasswordCipher::seal`]'s answer.
#[derive(uniffi::Enum)]
pub enum Sealing {
    Sealed {
        sealed: String,
    },
    /// The key can't be used right now (the Keystore failed): nothing is written over the file.
    Unavailable,
}

/// [`PasswordCipher::open`]'s answer.
#[derive(uniffi::Enum)]
pub enum Opening {
    Opened {
        plain: String,
    },
    /// Sealed under another key (another phone's, or one an uninstall dropped), or not sealed
    /// text at all: it never opens here, and the student types the password again.
    WrongKey,
    /// The Keystore failed for a moment; the same text may open on the next try.
    Unavailable,
}

/// Seals the saved VTOP password at rest; Kotlin implements it with an AES-GCM key in the
/// Android Keystore. Called off the main thread; neither call may throw.
#[uniffi::export(with_foreign)]
pub trait PasswordCipher: Send + Sync {
    /// `plain` sealed as opaque text.
    fn seal(&self, plain: String) -> Sealing;
    /// What [`PasswordCipher::seal`] gave back, opened.
    fn open(&self, sealed: String) -> Opening;
}

/// [`ACCOUNT`] on disk. With a cipher the password is in `sealed`; `password` is plain text
/// only in a file from before the cipher (or in host tests, which have none).
#[derive(Serialize, Deserialize)]
struct Stored {
    username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    /// VTOP rejected the password on a sign-in from this phone (or its server route).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    rejected: bool,
    /// VTOP rejected the password the server keeps (its `credentials_rejected` push). The phone
    /// still signs in with its own; kept until the student signs in again.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    server_rejected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    registration_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sealed: Option<String>,
}

/// Who is signed in and where their account stands: the app's screens follow it (the login
/// screen or the shell), Kotlin lets go of a student's things when another one's `username`
/// comes, and Settings' "Needs attention", the profile and the sign-in problems read the
/// standing. Rust's alone: Kotlin hears it through [`Core::watch_account`].
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum AccountStatus {
    SignedOut,
    /// `username` (the VTOP login, as saved) is signed in: their account is saved and their
    /// session installed, or their password waits on the Keystore.
    SignedIn {
        username: String,
        standing: Standing,
    },
}

/// Where a signed-in account stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Standing {
    Good,
    /// The demo account: what needs a real VTOP session or the server is hidden.
    Demo,
    /// VTOP rejected the saved password: the student updates it on the Account page.
    PasswordRejected {
        by: RejectedBy,
    },
}

/// Whose sign-in VTOP turned down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RejectedBy {
    /// This phone's (or its server route's): no sign-in is tried with it again.
    Phone,
    /// The server's, with the password it keeps.
    Server,
}

/// Hears every change of [`AccountStatus`]; Kotlin implements it. Called off the main thread.
#[uniffi::export(with_foreign)]
pub trait AccountListener: Send + Sync {
    fn changed(&self, status: AccountStatus);
}

/// The listener and the status it heard last, so it hears each change once.
#[derive(Default)]
pub(crate) struct Watch {
    listener: Option<Arc<dyn AccountListener>>,
    heard: Option<AccountStatus>,
}

/// The saved password as read back. No `Debug`: it holds the password.
#[derive(Clone)]
pub(crate) enum SavedPassword {
    Open(String),
    /// None saved, or sealed under a key this phone doesn't have: the student types it again.
    Gone,
    /// Sealed, but the Keystore failed to open it just now: still saved, opened on a later try.
    Locked,
}

/// The saved account as read back. No `Debug`: it holds the password.
#[derive(Clone)]
pub(crate) struct Saved {
    pub(crate) username: String,
    pub(crate) rejected: bool,
    pub(crate) registration_number: Option<String>,
    pub(crate) password: SavedPassword,
}

impl Saved {
    /// The account a sign-in can use; None without an open password.
    pub(crate) fn usable(self) -> Option<Account> {
        let SavedPassword::Open(password) = self.password else {
            return None;
        };
        Some(Account {
            password,
            username: self.username,
            registration_number: self.registration_number,
        })
    }

    /// The password, when it opened.
    pub(crate) fn password(&self) -> Option<&str> {
        match &self.password {
            SavedPassword::Open(password) => Some(password),
            SavedPassword::Gone | SavedPassword::Locked => None,
        }
    }
}

/// The account lock, held. `account.json` is written only under it, and a sign-out forgets it
/// under it too, so a write that checked the session is still signed in can't land after.
pub(crate) struct AccountFile<'a> {
    _held: MutexGuard<'a, ()>,
}

/// The last password sealed or opened and its sealed text, so writing the same password again
/// needs no Keystore (and can't lose it when the Keystore fails), and reads open it once.
#[derive(Default)]
pub(crate) struct Sealed(Mutex<Option<(String, String)>>);

impl Sealed {
    fn plain_for(&self, sealed: &str) -> Option<String> {
        let last = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        last.as_ref()
            .filter(|(_, text)| text == sealed)
            .map(|(plain, _)| plain.clone())
    }

    fn sealed_for(&self, plain: &str) -> Option<String> {
        let last = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        last.as_ref()
            .filter(|(password, _)| password == plain)
            .map(|(_, text)| text.clone())
    }

    fn keep(&self, plain: &str, sealed: &str) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) =
            Some((plain.to_owned(), sealed.to_owned()));
    }
}

impl Core {
    /// The saved account. It only reads: a plain-text password is sealed once, by
    /// [`Core::upgrade_account`].
    pub(crate) fn read_account(&self) -> Option<Saved> {
        let stored = self.read::<Stored>(ACCOUNT)?;
        let plain = stored.password.filter(|password| !password.is_empty());
        let password = match (stored.sealed, &self.shared.cipher) {
            (Some(sealed), Some(cipher)) => self.open_sealed(cipher.as_ref(), sealed),
            (Some(_), None) => SavedPassword::Gone,
            (None, _) => plain.map_or(SavedPassword::Gone, SavedPassword::Open),
        };
        Some(Saved {
            username: stored.username,
            rejected: stored.rejected,
            registration_number: stored.registration_number,
            password,
        })
    }

    fn open_sealed(&self, cipher: &dyn PasswordCipher, sealed: String) -> SavedPassword {
        if let Some(plain) = self.shared.sealed.plain_for(&sealed) {
            return SavedPassword::Open(plain);
        }
        match cipher.open(sealed.clone()) {
            Opening::Opened { plain } if !plain.is_empty() => {
                self.shared.sealed.keep(&plain, &sealed);
                SavedPassword::Open(plain)
            }
            Opening::Opened { .. } => SavedPassword::Gone,
            Opening::WrongKey => {
                warn!("saved password not opened (another phone's key): asked again");
                SavedPassword::Gone
            }
            Opening::Unavailable => {
                warn!("saved password not opened: the keystore failed, kept for the next try");
                SavedPassword::Locked
            }
        }
    }

    /// Seals a password saved in plain text by an older version and writes it back; run once,
    /// when the core opens. When the Keystore fails the plain file stays as it is, for the next
    /// start.
    pub(crate) fn upgrade_account(&self) {
        let Some(cipher) = &self.shared.cipher else {
            return;
        };
        let _file = self.lock_account();
        let Some(mut stored) = self.read::<Stored>(ACCOUNT) else {
            return;
        };
        let Some(plain) = stored
            .password
            .take()
            .filter(|password| !password.is_empty())
        else {
            return;
        };
        if stored.sealed.is_some() {
            return;
        }
        match cipher.seal(plain.clone()) {
            Sealing::Sealed { sealed } => {
                self.shared.sealed.keep(&plain, &sealed);
                stored.sealed = Some(sealed);
                self.write(ACCOUNT, &stored);
                info!("saved password sealed");
            }
            Sealing::Unavailable => warn!("saved password not sealed yet: the keystore failed"),
        }
    }

    pub(crate) fn lock_account(&self) -> AccountFile<'_> {
        AccountFile {
            _held: self
                .shared
                .locks
                .account
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        }
    }

    /// Writes `account`, `rejected` when VTOP turned its password down. The same student's
    /// registration number (when `account` has none: it is only known once signed in) and the
    /// server's rejection stay.
    pub(crate) fn write_account(&self, account: &Account, rejected: bool) {
        {
            let file = self.lock_account();
            self.write_account_held(&file, account, rejected);
        }
        self.account_changed();
    }

    /// [`Core::write_account`] for `slot`'s sign-in, unless it was signed out or replaced
    /// meanwhile; checked under the lock, so a sign-out can't land between.
    pub(crate) fn write_account_of(&self, slot: &Arc<Slot>, account: &Account, rejected: bool) {
        {
            let file = self.lock_account();
            if !self.is_current(slot) {
                return;
            }
            self.write_account_held(&file, account, rejected);
        }
        self.account_changed();
    }

    /// [`Core::write_account`] with the lock held and nothing announced: the caller says so
    /// ([`Core::account_changed`]) once it let go of the lock.
    pub(crate) fn write_account_held(
        &self,
        _file: &AccountFile<'_>,
        account: &Account,
        rejected: bool,
    ) {
        let same = self
            .read::<Stored>(ACCOUNT)
            .filter(|saved| saved.username == account.username);
        let mut account = account.clone();
        if account.registration_number.is_none() {
            account.registration_number = same
                .as_ref()
                .and_then(|saved| saved.registration_number.clone());
        }
        let server_rejected = same.as_ref().is_some_and(|saved| saved.server_rejected);
        // A plain-text password from an older version, not sealed yet: kept as it is if the
        // Keystore can't seal it now either.
        let plain = same
            .and_then(|saved| saved.password)
            .filter(|plain| *plain == account.password);
        self.write_stored(&account, rejected, server_rejected, plain);
    }

    /// Flags (or clears) the server's rejection of the saved account's password.
    pub(crate) fn set_server_rejected(&self, rejected: bool) {
        {
            let _file = self.lock_account();
            let Some(mut stored) = self.read::<Stored>(ACCOUNT) else {
                return;
            };
            if stored.server_rejected == rejected {
                return;
            }
            stored.server_rejected = rejected;
            self.write(ACCOUNT, &stored);
        }
        self.account_changed();
    }

    /// Writes `account` with its password sealed; without a cipher (host tests), as it is. The
    /// password sealed last is reused, so the Keystore is asked only for a new one. `plain`:
    /// the file holds this password in plain text, which stays when the Keystore fails.
    fn write_stored(
        &self,
        account: &Account,
        rejected: bool,
        server_rejected: bool,
        plain: Option<String>,
    ) {
        let (password, sealed) = match &self.shared.cipher {
            Some(cipher) => {
                let sealed = self.shared.sealed.sealed_for(&account.password);
                match sealed {
                    Some(sealed) => (None, Some(sealed)),
                    None => match cipher.seal(account.password.clone()) {
                        Sealing::Sealed { sealed } => {
                            self.shared.sealed.keep(&account.password, &sealed);
                            (None, Some(sealed))
                        }
                        Sealing::Unavailable if plain.is_some() => {
                            warn!("password not sealed yet: the keystore failed, kept as it was");
                            (plain, None)
                        }
                        Sealing::Unavailable => {
                            // A new password the Keystore can't take: the old sealed one is
                            // not this one, so the student types it again after a restart.
                            warn!("password not sealed: saved without it");
                            (None, None)
                        }
                    },
                }
            }
            None => (Some(account.password.clone()), None),
        };
        let stored = Stored {
            username: account.username.clone(),
            password,
            rejected,
            server_rejected,
            registration_number: account.registration_number.clone(),
            sealed,
        };
        self.write(ACCOUNT, &stored);
    }

    /// Where the account stands now, as the listener would hear it.
    #[cfg(test)]
    pub(crate) fn account_status(&self) -> AccountStatus {
        let watch = self
            .shared
            .account_watch
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.account_status_after(watch.heard.as_ref())
    }

    /// Where the account stands after `heard`: the installed (or locked) student, once their
    /// account is saved; a rejection as saved beside it, this phone's first. A sign-in waiting
    /// for its first OTP has saved nothing yet, so the phone is still whoever `heard` said
    /// (another student, whose account and pages stay until the code is accepted, or nobody).
    fn account_status_after(&self, heard: Option<&AccountStatus>) -> AccountStatus {
        let Some(username) = self.student() else {
            return AccountStatus::SignedOut;
        };
        let Some(stored) = self
            .read::<Stored>(ACCOUNT)
            .filter(|stored| stored.username.eq_ignore_ascii_case(&username))
        else {
            return heard.cloned().unwrap_or(AccountStatus::SignedOut);
        };
        let standing = if username == demo::USERNAME {
            Standing::Demo
        } else if stored.rejected {
            Standing::PasswordRejected {
                by: RejectedBy::Phone,
            }
        } else if stored.server_rejected {
            Standing::PasswordRejected {
                by: RejectedBy::Server,
            }
        } else {
            Standing::Good
        };
        AccountStatus::SignedIn { username, standing }
    }

    /// Tells the listener the status, when it changed since it last heard. The watch's lock is
    /// held throughout, so two changes reach it in order.
    pub(crate) fn account_changed(&self) {
        let mut watch = self
            .shared
            .account_watch
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let Some(listener) = watch.listener.clone() else {
            return;
        };
        let status = self.account_status_after(watch.heard.as_ref());
        if watch.heard.as_ref() != Some(&status) {
            info!(?status, "account status");
            watch.heard = Some(status.clone());
            listener.changed(status);
        }
    }
}

#[uniffi::export]
impl Core {
    /// Sends `listener` the [`AccountStatus`] now and on every change after; one listener, the
    /// last one given.
    pub fn watch_account(&self, flow: String, listener: Arc<dyn AccountListener>) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("watch_account", &flow, || {
            *self
                .shared
                .account_watch
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Watch {
                listener: Some(listener),
                heard: None,
            };
            self.account_changed();
            Ok(())
        });
    }

    /// VTOP rejected the password the server keeps (its `credentials_rejected` push): kept with
    /// the account, so Settings flags it after a restart too, until the student signs in again.
    pub fn password_rejected_on_server(&self, flow: String) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("password_rejected_on_server", &flow, || {
            if self.has_account() {
                self.set_server_rejected(true);
            }
            Ok(())
        });
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use crate::{CoreError, Shared};

    /// Reverses the text behind a tag; `open` fails for another phone's tag, and both fail
    /// while the Keystore is `down`.
    struct Fake {
        tag: &'static str,
        down: AtomicBool,
    }

    impl Fake {
        fn new(tag: &'static str) -> Arc<Self> {
            Arc::new(Self {
                tag,
                down: AtomicBool::new(false),
            })
        }

        fn set_down(&self, down: bool) {
            self.down.store(down, Ordering::SeqCst);
        }
    }

    impl PasswordCipher for Fake {
        fn seal(&self, plain: String) -> Sealing {
            if self.down.load(Ordering::SeqCst) {
                return Sealing::Unavailable;
            }
            Sealing::Sealed {
                sealed: format!("{}:{}", self.tag, plain.chars().rev().collect::<String>()),
            }
        }
        fn open(&self, sealed: String) -> Opening {
            if self.down.load(Ordering::SeqCst) {
                return Opening::Unavailable;
            }
            match sealed
                .strip_prefix(self.tag)
                .and_then(|rest| rest.strip_prefix(':'))
            {
                Some(text) => Opening::Opened {
                    plain: text.chars().rev().collect(),
                },
                None => Opening::WrongKey,
            }
        }
    }

    fn dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-account-{name}-{}", std::process::id()));
        // A leftover from an earlier run, if any.
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn shared(cipher: &Arc<Fake>) -> Shared {
        Shared {
            cipher: Some(cipher.clone() as Arc<dyn PasswordCipher>),
            ..Shared::default()
        }
    }

    fn core(name: &str, cipher: &Arc<Fake>) -> Core {
        Core::for_test_on(dir(name), shared(cipher))
    }

    /// [`Core::open`] on `dir`, as the app starts.
    fn open(dir: &std::path::Path, cipher: &Arc<Fake>) -> Arc<Core> {
        Core::open(
            "f".into(),
            dir.to_string_lossy().into(),
            crate::tests::shared_cache(),
            shared(cipher),
        )
        .unwrap()
    }

    fn file(core: &Core) -> String {
        fs::read_to_string(core.at(ACCOUNT)).unwrap()
    }

    fn account(password: &str) -> Account {
        Account {
            username: "22BCE0001".into(),
            password: password.into(),
            registration_number: Some("22BCE0001".into()),
        }
    }

    /// What the listener heard, in order.
    #[derive(Default)]
    struct Heard(Mutex<Vec<AccountStatus>>);

    impl AccountListener for Heard {
        fn changed(&self, status: AccountStatus) {
            self.0.lock().unwrap().push(status);
        }
    }

    impl Heard {
        fn take(&self) -> Vec<AccountStatus> {
            std::mem::take(&mut *self.0.lock().unwrap())
        }
    }

    /// 22BCE0001 signed in, as `standing`.
    fn signed_in(standing: Standing) -> AccountStatus {
        AccountStatus::SignedIn {
            username: "22BCE0001".into(),
            standing,
        }
    }

    const PHONE: Standing = Standing::PasswordRejected {
        by: RejectedBy::Phone,
    };
    const SERVER: Standing = Standing::PasswordRejected {
        by: RejectedBy::Server,
    };

    #[tokio::test]
    async fn the_status_is_heard_on_each_change_and_survives_a_restart() {
        let cipher = Fake::new("phone-a");
        let dir = dir("status");
        let core = open(&dir, &cipher);
        let heard = Arc::new(Heard::default());
        core.watch_account("f".into(), heard.clone());
        assert_eq!(heard.take(), [AccountStatus::SignedOut]);
        // Signed out, the server's push has nothing to flag.
        core.password_rejected_on_server("f".into());
        assert!(heard.take().is_empty());

        let session = crate::Session::new(account("pw-1"), &core.shared).unwrap();
        core.write_account(&session.account, false);
        core.install(session);
        assert_eq!(heard.take(), [signed_in(Standing::Good)]);
        core.password_rejected_on_server("f".into());
        assert_eq!(heard.take(), [signed_in(SERVER)]);
        // Written again (a sign-in saving the registration number): the server's flag stays.
        core.write_account(&account("pw-1"), false);
        assert!(heard.take().is_empty());
        drop(core);

        // As after the process died: read back from account.json.
        let core = open(&dir, &cipher);
        assert_eq!(core.account_status(), signed_in(SERVER));
        core.write_account(&account("pw-1"), true);
        assert_eq!(
            core.account_status(),
            signed_in(PHONE),
            "this phone's first"
        );
        assert!(
            !core.current().unwrap().read().await.is_password_rejected(),
            "the open session went on as it was"
        );
        let core = open(&dir, &cipher);
        assert!(
            core.current().unwrap().read().await.is_password_rejected(),
            "a restored session doesn't sign in with a rejected password"
        );
        core.set_server_rejected(false);
        core.write_account(&account("pw-2"), false);
        assert_eq!(core.account_status(), signed_in(Standing::Good));
        core.sign_out_here("f");
        assert_eq!(core.account_status(), AccountStatus::SignedOut);
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_sign_in_counts_once_its_account_is_saved() {
        let core = core("status-pending", &Fake::new("phone-a"));
        let heard = Arc::new(Heard::default());
        core.watch_account("f".into(), heard.clone());
        assert_eq!(heard.take(), [AccountStatus::SignedOut]);
        // Waiting for its first OTP: the login screen stays.
        let slot = core.install(crate::Session::new(account("pw-1"), &core.shared).unwrap());
        assert!(heard.take().is_empty());
        core.save(&slot, &*slot.read().await);
        assert_eq!(heard.take(), [signed_in(Standing::Good)]);

        // Another student's sign-in waiting for its OTP: the phone is still the last one's.
        let other = Account {
            username: "22BCE0002".into(),
            ..account("pw-2")
        };
        let slot = core.install(crate::Session::new(other, &core.shared).unwrap());
        assert!(heard.take().is_empty());
        assert_eq!(core.account_status(), signed_in(Standing::Good));
        core.save(&slot, &*slot.read().await);
        assert_eq!(
            heard.take(),
            [AccountStatus::SignedIn {
                username: "22BCE0002".into(),
                standing: Standing::Good,
            }]
        );
        core.sign_out_here("f");
        assert_eq!(heard.take(), [AccountStatus::SignedOut]);
        let _ = fs::remove_dir_all(&core.dir);
    }

    /// Signs `core` out on another thread when asked to seal, as a sign-out landing in the
    /// middle of a save does; seals like [`Fake`].
    struct SignOutWhileSealing {
        core: Mutex<Option<std::sync::Weak<Core>>>,
        signing_out: Mutex<Option<std::thread::JoinHandle<()>>>,
        past_install: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
    }

    impl PasswordCipher for SignOutWhileSealing {
        fn seal(&self, plain: String) -> Sealing {
            if let Some(core) = self
                .core
                .lock()
                .unwrap()
                .take()
                .and_then(|core| core.upgrade())
            {
                *self.signing_out.lock().unwrap() =
                    Some(std::thread::spawn(move || core.sign_out_here("sign_out")));
                // The sign-out has dropped the session (its status heard) before the save goes
                // on to write.
                if let Some(past) = self.past_install.lock().unwrap().take() {
                    past.recv().unwrap();
                }
            }
            Sealing::Sealed { sealed: plain }
        }
        fn open(&self, sealed: String) -> Opening {
            Opening::Opened { plain: sealed }
        }
    }

    /// Tells `to` once it hears the student signed out.
    struct HearsSignOut(Mutex<std::sync::mpsc::Sender<()>>);

    impl AccountListener for HearsSignOut {
        fn changed(&self, status: AccountStatus) {
            if status == AccountStatus::SignedOut {
                // The test may have stopped listening; nothing else waits on it.
                let _ = self.0.lock().unwrap().send(());
            }
        }
    }

    #[tokio::test]
    async fn a_sign_out_during_a_save_still_ends_signed_out_on_disk() {
        let dir = dir("save-race");
        let (told, past) = std::sync::mpsc::channel();
        let cipher = Arc::new(SignOutWhileSealing {
            core: Mutex::new(None),
            signing_out: Mutex::new(None),
            past_install: Mutex::new(Some(past)),
        });
        let core = Arc::new(Core::for_test_on(
            &dir,
            Shared {
                cipher: Some(cipher.clone() as Arc<dyn PasswordCipher>),
                ..Shared::default()
            },
        ));
        let session = crate::Session::new(account("pw-1"), &core.shared).unwrap();
        let slot = core.install(session);
        core.watch_account("f".into(), Arc::new(HearsSignOut(Mutex::new(told))));
        *cipher.core.lock().unwrap() = Some(Arc::downgrade(&core));

        core.save(&slot, &*slot.read().await);
        let signing_out = cipher.signing_out.lock().unwrap().take().unwrap();
        signing_out.join().unwrap();
        assert!(!core.at(ACCOUNT).exists(), "the account came back");
        assert!(!core.at(crate::SESSION).exists(), "the session came back");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn another_student_starts_without_the_last_ones_rejections() {
        let core = core("status-other", &Fake::new("phone-a"));
        core.write_account(&account("pw-1"), true);
        core.set_server_rejected(true);
        let other = Account {
            username: "22BCE0002".into(),
            ..account("pw-2")
        };
        core.write_account(&other, false);
        assert_eq!(
            file(&core),
            r#"{"username":"22BCE0002","registration_number":"22BCE0001","sealed":"phone-a:2-wp"}"#
        );
        let _ = fs::remove_dir_all(&core.dir);
    }

    #[test]
    fn a_file_from_before_the_flags_reads_as_not_rejected() {
        let core = core("status-old", &Fake::new("phone-a"));
        fs::write(
            core.at(ACCOUNT),
            r#"{"username":"22BCE0001","sealed":"phone-a:1-wp"}"#,
        )
        .unwrap();
        let saved = core.read_account().unwrap();
        assert!(!saved.rejected && saved.password() == Some("pw-1"));
        core.set_server_rejected(true);
        assert_eq!(
            file(&core),
            r#"{"username":"22BCE0001","server_rejected":true,"sealed":"phone-a:1-wp"}"#
        );
        let _ = fs::remove_dir_all(&core.dir);
    }

    const PLAIN: &str = r#"{"username":"22BCE0001","password":"pw-secret-1","rejected":true}"#;

    #[test]
    fn a_plain_password_is_sealed_when_the_core_opens_and_reads_only_read() {
        let cipher = Fake::new("phone-a");
        let dir = dir("migrate");
        fs::write(dir.join(ACCOUNT), PLAIN).unwrap();
        // A read never writes: the upgrade is the core's, once, under the account lock.
        let reader = Core::for_test_on(&dir, shared(&cipher));
        let saved = reader.read_account().unwrap();
        assert!(saved.password() == Some("pw-secret-1") && saved.rejected);
        assert_eq!(file(&reader), PLAIN);

        let core = open(&dir, &cipher);
        let saved = file(&core);
        assert!(
            !saved.contains("pw-secret-1") && !saved.contains("\"password\""),
            "{saved}"
        );
        assert!(saved.contains("1-terces-wp"), "{saved}");
        assert_eq!(core.read_account().unwrap().password(), Some("pw-secret-1"));
        assert!(core.current().is_ok());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_seal_keeps_the_plain_password_for_the_next_start() {
        let cipher = Fake::new("phone-a");
        cipher.set_down(true);
        let dir = dir("migrate-down");
        fs::write(dir.join(ACCOUNT), PLAIN).unwrap();
        let core = open(&dir, &cipher);
        assert_eq!(file(&core), PLAIN);
        assert!(core.current().is_ok(), "signed in on the plain password");
        drop(core);

        cipher.set_down(false);
        let core = open(&dir, &cipher);
        assert!(file(&core).contains("\"sealed\":\"phone-a:1-terces-wp\""));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rewriting_a_plain_password_while_the_keystore_fails_keeps_it() {
        let cipher = Fake::new("phone-a");
        cipher.set_down(true);
        let dir = dir("plain-down");
        fs::write(dir.join(ACCOUNT), PLAIN).unwrap();
        let core = open(&dir, &cipher);
        // As a sign-in on the plain password saves the registration number.
        let mut same = account("pw-secret-1");
        same.registration_number = None;
        core.write_account(&same, false);
        assert_eq!(
            file(&core),
            r#"{"username":"22BCE0001","password":"pw-secret-1"}"#,
            "kept as it was, for the next start to seal"
        );
        // Another password the Keystore can't take is still not saved in plain text.
        core.write_account(&account("pw-2"), false);
        assert!(!file(&core).contains("pw-2") && !file(&core).contains("password"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writing_the_same_password_while_the_keystore_fails_keeps_it() {
        let cipher = Fake::new("phone-a");
        let core = core("rewrite-down", &cipher);
        core.write_account(&account("pw-1"), false);
        let sealed = file(&core);
        cipher.set_down(true);
        // As a rejected password is flagged, or the registration number saved.
        core.write_account(&account("pw-1"), true);
        assert_eq!(
            file(&core),
            sealed.replace(
                "\"username\":\"22BCE0001\",",
                "\"username\":\"22BCE0001\",\"rejected\":true,"
            )
        );
        // A new password the Keystore can't take isn't saved in plain text.
        core.write_account(&account("pw-2"), false);
        assert!(!file(&core).contains("pw-2") && !file(&core).contains("sealed"));
        let _ = fs::remove_dir_all(&core.dir);
    }

    #[test]
    fn a_keystore_that_fails_at_start_keeps_the_student_signed_in() {
        let cipher = Fake::new("phone-a");
        let first = core("locked", &cipher);
        first.write_account(&account("pw-1"), false);
        cipher.set_down(true);
        let core = open(&first.dir, &cipher);
        assert_eq!(
            core.signed_in_user("f".into()).as_deref(),
            Some("22BCE0001")
        );
        assert_eq!(core.saved_password("f".into()), None);
        assert!(matches!(core.current(), Err(CoreError::PasswordLocked)));
        assert!(file(&core).contains("sealed"), "the password stays saved");

        cipher.set_down(false);
        assert!(core.current().is_ok(), "opened on a later try");
        assert_eq!(core.saved_password("f".into()).as_deref(), Some("pw-1"));
        let _ = fs::remove_dir_all(&first.dir);
    }

    #[test]
    fn a_locked_password_that_turns_out_gone_is_heard_as_signed_out() {
        let cipher = Fake::new("phone-a");
        let first = core("locked-gone", &cipher);
        first.write_account(&account("pw-1"), false);
        cipher.set_down(true);
        let core = open(&first.dir, &cipher);
        let heard = Arc::new(Heard::default());
        core.watch_account("f".into(), heard.clone());
        assert_eq!(
            heard.take(),
            [signed_in(Standing::Good)],
            "while it waits on the Keystore"
        );
        // Opened again on the next call, it is another phone's after all.
        fs::write(
            core.at(ACCOUNT),
            r#"{"username":"22BCE0001","sealed":"phone-b:1-wp"}"#,
        )
        .unwrap();
        cipher.set_down(false);
        assert!(matches!(core.current(), Err(CoreError::NotSignedIn)));
        assert_eq!(heard.take(), [AccountStatus::SignedOut]);
        let _ = fs::remove_dir_all(&first.dir);
    }

    #[test]
    fn another_phones_copy_has_no_password_and_signs_in_nobody() {
        let first = core("restore-a", &Fake::new("phone-a"));
        first.write_account(&account("pw-secret-1"), false);
        // As a backup restores it on a new phone, whose Keystore has another key.
        let other = Fake::new("phone-b");
        let second = core("restore-b", &other);
        fs::write(second.at(ACCOUNT), file(&first)).unwrap();

        let account = second.read_account().unwrap();
        assert!(matches!(account.password, SavedPassword::Gone));
        assert_eq!(account.registration_number.as_deref(), Some("22BCE0001"));
        let opened = open(&second.dir, &other);
        assert_eq!(opened.signed_in_user("f".into()), None);
        assert!(matches!(opened.current(), Err(CoreError::NotSignedIn)));
        let _ = fs::remove_dir_all(&first.dir);
        let _ = fs::remove_dir_all(&second.dir);
    }

    #[test]
    fn the_stored_form_roundtrips_and_an_empty_password_is_none() {
        let core = core("stored", &Fake::new("phone-a"));
        core.write_account(&account("pw-1"), true);
        assert_eq!(
            file(&core),
            r#"{"username":"22BCE0001","rejected":true,"registration_number":"22BCE0001","sealed":"phone-a:1-wp"}"#
        );
        let saved = core.read_account().unwrap();
        assert!(saved.rejected && saved.password() == Some("pw-1"));
        // A plain file with an empty password (never sealed) has none, and isn't rewritten.
        let plain = r#"{"username":"22BCE0001","password":""}"#;
        fs::write(core.at(ACCOUNT), plain).unwrap();
        assert!(core.read_account().unwrap().usable().is_none());
        core.upgrade_account();
        assert_eq!(file(&core), plain);
        let _ = fs::remove_dir_all(&core.dir);
    }
}

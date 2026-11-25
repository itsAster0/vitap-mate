//! `account.json`: the saved sign-in, its password sealed by [`PasswordCipher`] (an Android
//! Keystore key) so the file holds only ciphertext. A file from before the cipher (the password
//! in plain text) is sealed and written back once, when the core opens ([`Core::upgrade_account`]).
//! The key never leaves the phone, nor survives an uninstall: a copy restored on another phone (or
//! after a reinstall) can't be opened, so its password is gone and the student types it again. A
//! Keystore that fails for a moment is not that: the password stays on disk and is opened again
//! on the next use. Neither the password nor the sealed text is ever logged.

use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{ACCOUNT, Account, Core};

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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    rejected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    registration_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sealed: Option<String>,
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
            rejected: self.rejected,
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
        let _file = self
            .shared
            .account_file
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
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

    /// Writes `account`, keeping the saved registration number when `account` has none (it is
    /// only known once signed in).
    pub(crate) fn write_account(&self, account: &Account) {
        let _file = self
            .shared
            .account_file
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let mut account = account.clone();
        if account.registration_number.is_none() {
            account.registration_number = self
                .read::<Stored>(ACCOUNT)
                .filter(|saved| saved.username == account.username)
                .and_then(|saved| saved.registration_number);
        }
        self.write_stored(&account);
    }

    /// Writes `account` with its password sealed; without a cipher (host tests), as it is. The
    /// password sealed last is reused, so the Keystore is asked only for a new one.
    fn write_stored(&self, account: &Account) {
        let (password, sealed) = match &self.shared.cipher {
            Some(cipher) => {
                let sealed = self
                    .shared
                    .sealed
                    .sealed_for(&account.password)
                    .or_else(|| match cipher.seal(account.password.clone()) {
                        Sealing::Sealed { sealed } => {
                            self.shared.sealed.keep(&account.password, &sealed);
                            Some(sealed)
                        }
                        Sealing::Unavailable => {
                            // A new password the Keystore can't take: the old sealed one is
                            // not this one, so the student types it again after a restart.
                            warn!("password not sealed: saved without it");
                            None
                        }
                    });
                (None, sealed)
            }
            None => (Some(account.password.clone()), None),
        };
        let stored = Stored {
            username: account.username.clone(),
            password,
            rejected: account.rejected,
            registration_number: account.registration_number.clone(),
            sealed,
        };
        self.write(ACCOUNT, &stored);
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

    fn account(password: &str, rejected: bool) -> Account {
        Account {
            username: "22BCE0001".into(),
            password: password.into(),
            rejected,
            registration_number: Some("22BCE0001".into()),
        }
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
    fn writing_the_same_password_while_the_keystore_fails_keeps_it() {
        let cipher = Fake::new("phone-a");
        let core = core("rewrite-down", &cipher);
        core.write_account(&account("pw-1", false));
        let sealed = file(&core);
        cipher.set_down(true);
        // As a rejected password is flagged, or the registration number saved.
        core.write_account(&account("pw-1", true));
        assert_eq!(
            file(&core),
            sealed.replace(
                "\"username\":\"22BCE0001\",",
                "\"username\":\"22BCE0001\",\"rejected\":true,"
            )
        );
        // A new password the Keystore can't take isn't saved in plain text.
        core.write_account(&account("pw-2", false));
        assert!(!file(&core).contains("pw-2") && !file(&core).contains("sealed"));
        let _ = fs::remove_dir_all(&core.dir);
    }

    #[test]
    fn a_keystore_that_fails_at_start_keeps_the_student_signed_in() {
        let cipher = Fake::new("phone-a");
        let first = core("locked", &cipher);
        first.write_account(&account("pw-1", false));
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
    fn another_phones_copy_has_no_password_and_signs_in_nobody() {
        let first = core("restore-a", &Fake::new("phone-a"));
        first.write_account(&account("pw-secret-1", false));
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
        core.write_account(&account("pw-1", true));
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

//! `account.json`: the saved sign-in, its password sealed by [`PasswordCipher`] (an Android
//! Keystore key) so the file holds only ciphertext. A file from before the cipher (the password
//! in plain text) is sealed and written back the first time it is read. The key never leaves the
//! phone, nor survives an uninstall: a copy restored on another phone (or after a
//! reinstall) can't be opened, so its password is absent and the student types it again. Neither
//! the password nor the sealed text is ever logged.

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{ACCOUNT, Account, Core};

/// Seals the saved VTOP password at rest; Kotlin implements it with an AES-GCM key in the
/// Android Keystore. Called off the main thread; neither call may throw.
#[uniffi::export(with_foreign)]
pub trait PasswordCipher: Send + Sync {
    /// `plain` sealed as opaque text; None when the key can't be used (the password then isn't
    /// saved).
    fn seal(&self, plain: String) -> Option<String>;
    /// What [`PasswordCipher::seal`] gave back opened; None when this phone's key can't open it.
    fn open(&self, sealed: String) -> Option<String>;
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

/// The saved account as read back. No `Debug`: it holds the password.
#[derive(Clone)]
pub(crate) struct Saved {
    pub(crate) username: String,
    pub(crate) rejected: bool,
    pub(crate) registration_number: Option<String>,
    /// None when this phone can't open it (another phone's key, or the key is gone): the
    /// student types it again. Never logged.
    pub(crate) password: Option<String>,
}

impl Saved {
    /// The account a sign-in can use; None without a password.
    pub(crate) fn usable(self) -> Option<Account> {
        Some(Account {
            password: self.password?,
            username: self.username,
            rejected: self.rejected,
            registration_number: self.registration_number,
        })
    }
}

impl Core {
    /// The saved account; one saved in plain text is sealed and written back.
    pub(crate) fn read_account(&self) -> Option<Saved> {
        let stored = self.read::<Stored>(ACCOUNT)?;
        let cipher = self.shared.cipher.as_ref();
        let plain = stored.password.filter(|password| !password.is_empty());
        let migrate = stored.sealed.is_none() && cipher.is_some() && plain.is_some();
        let password = match (stored.sealed, cipher) {
            (Some(sealed), Some(cipher)) => {
                let opened = cipher.open(sealed).filter(|password| !password.is_empty());
                if opened.is_none() {
                    warn!("saved password not opened (another phone's key): asked again");
                }
                opened
            }
            (Some(_), None) => None,
            (None, _) => plain,
        };
        let saved = Saved {
            username: stored.username,
            rejected: stored.rejected,
            registration_number: stored.registration_number,
            password,
        };
        if migrate {
            info!("saved password sealed");
            if let Some(account) = saved.clone().usable() {
                self.write_stored(&account);
            }
        }
        Some(saved)
    }

    /// Writes `account`, keeping the saved registration number when `account` has none (it is
    /// only known once signed in).
    pub(crate) fn write_account(&self, account: &Account) {
        let mut account = account.clone();
        if account.registration_number.is_none() {
            account.registration_number = self
                .read::<Stored>(ACCOUNT)
                .filter(|saved| saved.username == account.username)
                .and_then(|saved| saved.registration_number);
        }
        self.write_stored(&account);
    }

    /// Writes `account` with its password sealed; without a cipher (host tests), as it is.
    fn write_stored(&self, account: &Account) {
        let (password, sealed) = match &self.shared.cipher {
            Some(cipher) => {
                let sealed = cipher.seal(account.password.clone());
                if sealed.is_none() {
                    warn!("password not sealed: saved without it");
                }
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

    use super::*;
    use crate::Shared;

    /// Reverses the text behind a tag; `open` fails for another phone's tag.
    struct Fake(&'static str);

    impl PasswordCipher for Fake {
        fn seal(&self, plain: String) -> Option<String> {
            Some(format!(
                "{}:{}",
                self.0,
                plain.chars().rev().collect::<String>()
            ))
        }
        fn open(&self, sealed: String) -> Option<String> {
            let text = sealed.strip_prefix(self.0)?.strip_prefix(':')?;
            Some(text.chars().rev().collect())
        }
    }

    fn core(name: &str, phone: &'static str) -> Core {
        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-account-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let cipher = Some(Arc::new(Fake(phone)) as Arc<dyn PasswordCipher>);
        Core::for_test_on(
            &dir,
            Shared {
                cipher,
                ..Shared::default()
            },
        )
    }

    fn file(core: &Core) -> String {
        fs::read_to_string(core.at(ACCOUNT)).unwrap()
    }

    #[test]
    fn a_plain_password_is_sealed_on_first_read() {
        let core = core("migrate", "phone-a");
        let plain = r#"{"username":"22BCE0001","password":"pw-secret-1","rejected":true}"#;
        fs::write(core.at(ACCOUNT), plain).unwrap();

        let account = core.read_account().unwrap();
        assert!(account.password.as_deref() == Some("pw-secret-1") && account.rejected);
        let saved = file(&core);
        assert!(
            !saved.contains("pw-secret-1") && !saved.contains("\"password\""),
            "{saved}"
        );
        assert!(saved.contains("1-terces-wp"), "{saved}");
        assert_eq!(
            core.read_account().unwrap().password.as_deref(),
            Some("pw-secret-1")
        );
        let _ = fs::remove_dir_all(&core.dir);
    }

    #[test]
    fn another_phones_copy_has_no_password_and_signs_in_nobody() {
        let first = core("restore-a", "phone-a");
        first.write_account(&Account {
            username: "22BCE0001".into(),
            password: "pw-secret-1".into(),
            rejected: false,
            registration_number: Some("22BCE0001".into()),
        });
        // As a backup restores it on a new phone, whose Keystore has another key.
        let second = core("restore-b", "phone-b");
        fs::write(second.at(ACCOUNT), file(&first)).unwrap();

        let account = second.read_account().unwrap();
        assert!(account.password.is_none());
        assert_eq!(account.registration_number.as_deref(), Some("22BCE0001"));
        let opened = Core::open(
            "f".into(),
            second.dir.to_string_lossy().into(),
            crate::tests::shared_cache(),
            Shared {
                cipher: Some(Arc::new(Fake("phone-b"))),
                ..Shared::default()
            },
        )
        .unwrap();
        assert_eq!(opened.signed_in_user("f".into()), None);
        assert!(opened.current().is_err());
        let _ = fs::remove_dir_all(&first.dir);
        let _ = fs::remove_dir_all(&second.dir);
    }

    #[test]
    fn the_stored_form_roundtrips_and_an_empty_password_is_none() {
        let core = core("stored", "phone-a");
        core.write_account(&Account {
            username: "22BCE0001".into(),
            password: "pw-1".into(),
            rejected: true,
            registration_number: Some("22BCE0001".into()),
        });
        assert_eq!(
            file(&core),
            r#"{"username":"22BCE0001","rejected":true,"registration_number":"22BCE0001","sealed":"phone-a:1-wp"}"#
        );
        let saved = core.read_account().unwrap();
        assert!(saved.rejected && saved.password.as_deref() == Some("pw-1"));
        // A plain file with an empty password (never sealed) has none, and isn't rewritten.
        let plain = r#"{"username":"22BCE0001","password":""}"#;
        fs::write(core.at(ACCOUNT), plain).unwrap();
        assert!(core.read_account().unwrap().usable().is_none());
        assert_eq!(file(&core), plain);
        let _ = fs::remove_dir_all(&core.dir);
    }
}

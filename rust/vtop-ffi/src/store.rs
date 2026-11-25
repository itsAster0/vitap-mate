//! The app's files: where each lives, and JSON reads and crash-safe writes.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::PoisonError;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::warn;

use crate::pages::changes;
use crate::remote::{consent, forwarding, regno};
use crate::{Core, PAGES, SELECTED, SEMESTERS};

pub(crate) mod account;
pub(crate) mod cookies;
pub(crate) mod decoded;

impl Core {
    /// Removes `files` and every cached page: the student's data, at a sign-out or another
    /// student. One file alone goes with [`Core::remove_file`].
    pub(crate) fn forget(&self, files: &[&str]) {
        // Not while a pull records mail into forwarding.json (it would write it back).
        let _forwarding = forwarding::FILE_LOCK
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if files.contains(&consent::FILE) {
            consent::forget(&self.shared);
            regno::forget(self);
        }
        for file in files {
            self.remove_file(file);
        }
        match fs::remove_dir_all(self.at(PAGES)) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => {
                warn!(%error, "cached pages not removed");
            }
            _ => {}
        }
        decoded::forget(&self.dir);
    }

    /// Removes `file` and its decoded copy, leaving everything else.
    pub(crate) fn remove_file(&self, file: &str) {
        let path = self.at(file);
        match fs::remove_file(&path) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => {
                warn!(file, %error, "not removed");
            }
            _ => {}
        }
        decoded::forget(&path);
    }

    /// Where `file` lives: the change inbox in the cache folder, everything
    /// else (pages, account, session, keys) in the data folder.
    pub(crate) fn at(&self, file: impl AsRef<Path>) -> PathBuf {
        let file = file.as_ref();
        if file == Path::new(changes::FILE) {
            self.cache.join(file)
        } else {
            self.dir.join(file)
        }
    }

    pub(crate) fn read<T: DeserializeOwned>(&self, file: &str) -> Option<T> {
        decode(file, &fs::read(self.at(file)).ok()?)
    }

    /// [`Core::read`] for the semester list, the pick and pages, which stay decoded in memory
    /// while the file is unchanged ([`decoded`]); other files are read each time.
    pub(crate) fn read_kept<T: DeserializeOwned + Clone + Send + Sync + 'static>(
        &self,
        file: &str,
    ) -> Option<T> {
        if file.starts_with(&format!("{PAGES}/")) || file == SEMESTERS || file == SELECTED {
            decoded::read(&self.at(file), |bytes| decode(file, bytes))
        } else {
            self.read(file)
        }
    }

    /// Writes `value` as JSON through a temporary file, so a crash never
    /// leaves half a file. A page compared for the change inbox records
    /// what changed against the copy it replaces.
    pub(crate) fn write<T: Serialize + 'static>(&self, file: &str, value: &T) {
        match serde_json::to_vec(value) {
            Ok(bytes) => {
                self.record_changes(file, value, &bytes);
                self.write_bytes(file, &bytes);
            }
            Err(error) => warn!(file, %error, "not saved"),
        }
    }

    /// [`Core::write`] without the change inbox (the inbox's own file).
    pub(crate) fn write_plain(&self, file: &str, value: &impl Serialize) {
        match serde_json::to_vec(value) {
            Ok(bytes) => self.write_bytes(file, &bytes),
            Err(error) => warn!(file, %error, "not saved"),
        }
    }

    pub(crate) fn write_bytes(&self, file: &str, bytes: &[u8]) {
        let path = self.at(file);
        let result = write_atomic(&path, bytes);
        decoded::forget(&path);
        if let Err(error) = result {
            warn!(file, %error, "not saved");
        }
    }
}

/// Writes `bytes` to `path` through a temporary file beside it, synced before it replaces
/// `path`, so a crash leaves the old file or the new one. Each write has a temporary file of
/// its own, so two writers to one file never trip over each other's.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut temp_name = path
        .file_name()
        .map_or_else(Default::default, OsStr::to_os_string);
    temp_name.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let temp = path.with_file_name(temp_name);
    let written = File::create(&temp)
        .and_then(|mut file| file.write_all(bytes).and_then(|()| file.sync_all()))
        .and_then(|()| fs::rename(&temp, path));
    if written.is_err()
        && let Err(error) = fs::remove_file(&temp)
        && error.kind() != io::ErrorKind::NotFound
    {
        warn!(%error, "temporary file not removed");
    }
    written
}

fn decode<T: DeserializeOwned>(file: &str, bytes: &[u8]) -> Option<T> {
    serde_json::from_slice(bytes)
        .inspect_err(|error| warn!(file, %error, "unreadable"))
        .ok()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::remote::backend::{LAST_STUDENT, Refresh};
    use crate::{Core, PAGES};

    #[tokio::test]
    async fn dropping_the_last_students_token_keeps_the_pages() {
        let dir = std::env::temp_dir().join(format!("vtop-ffi-last-pages-{}", std::process::id()));
        // A leftover from an earlier run, if any.
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(PAGES).join("AP1")).unwrap();
        let core = Core::for_test(&dir);
        let page = format!("{PAGES}/AP1/marks.json");
        core.write_plain(&page, &serde_json::json!({ "records": [] }));
        // Set aside on another server: dropped without a call.
        core.write_plain(
            LAST_STUDENT,
            &Refresh {
                server: "https://elsewhere.invalid".into(),
                token: "t".into(),
                regno: "22BCE0002".into(),
            },
        );
        core.sign_out_last_student("f").await;
        assert!(!core.at(LAST_STUDENT).exists());
        assert!(core.at(&page).exists());
        assert!(core.read_kept::<serde_json::Value>(&page).is_some());
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("vtop-ffi-{name}-{}", std::process::id()));
        // A leftover from an earlier run, if any.
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writers_to_one_file_each_land_whole() {
        let dir = scratch("atomic-race");
        let path = dir.join("page.json");
        std::thread::scope(|scope| {
            for writer in 0..8 {
                let path = &path;
                scope.spawn(move || {
                    for _ in 0..50 {
                        super::write_atomic(path, format!("{writer}").as_bytes()).unwrap();
                    }
                });
            }
        });
        let last: u32 = fs::read_to_string(&path).unwrap().parse().unwrap();
        assert!(last < 8);
        assert_eq!(
            fs::read_dir(&dir).unwrap().count(),
            1,
            "a temporary file left"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failed_write_leaves_no_temporary_file() {
        let dir = scratch("atomic-fail");
        // A folder can't be replaced by a file.
        let path = dir.join("page.json");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("inside"), "x").unwrap();
        assert!(super::write_atomic(&path, b"{}").is_err());
        assert_eq!(
            fs::read_dir(&dir).unwrap().count(),
            1,
            "a temporary file left"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}

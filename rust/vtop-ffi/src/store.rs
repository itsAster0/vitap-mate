//! The app's files: where each lives, and JSON reads and crash-safe writes.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::PoisonError;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::warn;

use crate::pages::changes;
use crate::remote::{consent, forwarding};
use crate::{Core, PAGES, SELECTED, SEMESTERS};

pub(crate) mod cookies;
pub(crate) mod decoded;

impl Core {
    /// Removes `files` and every cached page.
    pub(crate) fn forget(&self, files: &[&str]) {
        // Not while a pull records mail into forwarding.json (it would write it back).
        let _forwarding = forwarding::FILE_LOCK
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if files.contains(&consent::FILE) {
            consent::forget(&self.shared);
        }
        for file in files {
            match fs::remove_file(self.at(file)) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    warn!(file, %error, "not removed");
                }
                _ => {}
            }
        }
        match fs::remove_dir_all(self.at(PAGES)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                warn!(%error, "cached pages not removed");
            }
            _ => {}
        }
        decoded::forget(&self.dir);
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
        let temp = path.with_extension("tmp");
        let result = fs::write(&temp, bytes).and_then(|()| fs::rename(&temp, &path));
        decoded::forget(&path);
        if let Err(error) = result {
            warn!(file, %error, "not saved");
        }
    }
}

fn decode<T: DeserializeOwned>(file: &str, bytes: &[u8]) -> Option<T> {
    serde_json::from_slice(bytes)
        .inspect_err(|error| warn!(file, %error, "unreadable"))
        .ok()
}

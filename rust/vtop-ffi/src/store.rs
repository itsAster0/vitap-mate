//! The app's files: where each lives, and JSON reads and crash-safe writes.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::{debug, info, warn};

use vtop_core::sync::RwLock;

use crate::device::Device;
use crate::pages::changes;
use crate::remote::{consent, regno, udp};
use crate::{Core, PAGES, SELECTED, SEMESTERS};

pub(crate) mod account;
pub(crate) mod cookies;
pub(crate) mod decoded;

/// Each lock serialises the read-modify-write of one of the core's files.
#[derive(Default)]
pub(crate) struct Locks {
    pub(crate) account: Mutex<()>,
    /// `device.json`, kept decoded once read: every signed call reads it (its clock offset).
    /// Only this core writes it, under this lock.
    pub(crate) device: Mutex<Option<Device>>,
    /// `forwarding.json`.
    pub(crate) forwarding: Mutex<()>,
    /// The forwarding addresses still to leave on the server.
    pub(crate) forwarding_leave: Mutex<()>,
    /// The change inbox.
    pub(crate) changes: Mutex<()>,
    /// Developer's demo recording.
    pub(crate) demo: Mutex<()>,
    /// `regno_sharing.json`.
    pub(crate) regno: Mutex<()>,
    /// One class-history fetch per course key at a time; emptied with the student's files.
    pub(crate) attendance_details: Mutex<HashMap<String, Arc<RwLock<()>>>>,
}

impl Core {
    /// Removes `files` and every cached page: the student's data, at a sign-out or another
    /// student. One file alone goes with [`Core::remove_file`].
    pub(crate) fn forget(&self, files: &[&str]) {
        // Not while a pull records mail into forwarding.json (it would write it back).
        let _forwarding = self
            .shared
            .locks
            .forwarding
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
        self.shared
            .locks
            .attendance_details
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
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

/// The largest process id Linux hands out (2^22). A temporary file of [`write_atomic`] carries
/// its writer's id; Kotlin's (`writeAtomically`) carry a nanosecond clock there, far larger, and
/// are Kotlin's to clean.
const PID_MAX: u32 = 1 << 22;

impl Core {
    /// Removes what a write cut short by a crash left: [`write_atomic`]'s (and vtop-wire's)
    /// temporary files from another process, in the data folder, the cached pages, the UDP
    /// sessions and the cache folder. They can hold
    /// what the file they were replacing holds (the sealed password, tokens), so they don't stay.
    pub(crate) fn sweep_temporary(&self) {
        let own = std::process::id();
        let mut folders = vec![
            (self.dir.clone(), Sweep::Folder),
            (self.cache.clone(), Sweep::Folder),
            (self.at(udp::WIRE_DIR), Sweep::Wire),
            (self.at(PAGES), Sweep::Tree),
        ];
        let mut removed = 0_u32;
        while let Some((folder, sweep)) = folders.pop() {
            let Ok(entries) = fs::read_dir(&folder) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name();
                let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
                if is_dir {
                    if sweep == Sweep::Tree {
                        folders.push((path, Sweep::Tree));
                    }
                } else if orphan(&name, own) || (sweep == Sweep::Wire && old_wire_temp(&name)) {
                    match fs::remove_file(&path) {
                        Ok(()) => removed += 1,
                        Err(error) => warn!(%error, "temporary file not removed"),
                    }
                }
            }
        }
        if removed > 0 {
            info!(removed, "temporary files left by a crash removed");
        }
    }
}

/// How [`Core::sweep_temporary`] goes through a folder.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sweep {
    /// Its own files only.
    Folder,
    /// Its files and every folder under it (the cached pages).
    Tree,
    /// vtop-wire's sessions, which also had temporary files of an older name.
    Wire,
}

/// Whether `name` is [`write_atomic`]'s temporary file (`<file>.<pid>.<n>.tmp`) from another
/// process than `own`.
fn orphan(name: &OsStr, own: u32) -> bool {
    let Some(rest) = name.to_str().and_then(|name| name.strip_suffix(".tmp")) else {
        return false;
    };
    let mut parts = rest.rsplitn(3, '.');
    let (Some(count), Some(pid), Some(file)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !file.is_empty()
        && count.parse::<u64>().is_ok()
        && pid
            .parse::<u32>()
            .is_ok_and(|pid| pid != own && pid <= PID_MAX)
}

/// Whether `name` is the temporary file older vtop-wire versions wrote (`<stem>.tmp<pid>`).
/// None is being written now, whatever its pid: vtop-wire writes [`orphan`]'s name instead.
fn old_wire_temp(name: &OsStr) -> bool {
    name.to_str()
        .and_then(|name| name.rsplit_once(".tmp"))
        .is_some_and(|(stem, pid)| {
            !stem.is_empty() && !pid.is_empty() && pid.bytes().all(|byte| byte.is_ascii_digit())
        })
}

/// Writes `bytes` to `path` through a temporary file beside it, synced before it replaces
/// `path`, so a crash leaves the old file or the new one; the folder is synced after, so the
/// new name survives a power cut too. Each write has a temporary file of its own, so two
/// writers to one file never trip over each other's.
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
    // The file is whole either way; a folder that can't be synced (some file systems refuse)
    // only risks the old copy after a power cut.
    if written.is_ok()
        && let Some(folder) = path.parent()
        && let Err(error) = File::open(folder).and_then(|folder| folder.sync_all())
    {
        debug!(%error, "folder not synced");
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

    use crate::remote::backend::{KEPT_SIGN_OUT, Refresh};
    use crate::remote::udp;
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
            KEPT_SIGN_OUT,
            &Refresh {
                server: "https://elsewhere.invalid".into(),
                token: "t".into(),
                regno: "22BCE0002".into(),
            },
        );
        core.send_kept_sign_out("f").await;
        assert!(!core.at(KEPT_SIGN_OUT).exists());
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
    fn forgetting_the_student_drops_the_class_history_locks() {
        let dir = scratch("detail-locks");
        let core = Core::for_test(&dir);
        let locks = || core.shared.locks.attendance_details.lock().unwrap().len();
        core.shared
            .locks
            .attendance_details
            .lock()
            .unwrap()
            .insert("CSE1001.theory".into(), Default::default());
        assert_eq!(locks(), 1);
        core.forget(&[]);
        assert_eq!(locks(), 0);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn opening_removes_temporary_files_a_crash_left() {
        let dir = scratch("sweep");
        let pages = dir.join(PAGES).join("AP1").join("attendance_detail");
        fs::create_dir_all(&pages).unwrap();
        let wire = dir.join(udp::WIRE_DIR);
        fs::create_dir_all(&wire).unwrap();
        let own = std::process::id();
        let other = if own == 4321 { 4322 } else { 4321 };
        let orphans = [
            dir.join(format!("account.json.{other}.0.tmp")),
            dir.join(format!("account_token.json.{other}.7.tmp")),
            pages.join(format!("CSE1001.json.{other}.3.tmp")),
            // vtop-wire's own name before it took this one (`<stem>.tmp<pid>`), any pid.
            wire.join(format!("session.tmp{other}")),
            wire.join(format!("session.tmp{own}")),
        ];
        let kept = [
            // This process's own write, under way.
            dir.join(format!("account.json.{own}.1.tmp")),
            // Kotlin's writeAtomically: a nanosecond clock in place of the pid.
            dir.join("settings.json.81234567890123.4.tmp"),
            dir.join("notes.tmp"),
            dir.join("account.json"),
            wire.join("session.json"),
            wire.join("session.tmpx"),
            // The old vtop-wire name only counts in its own folder.
            dir.join(format!("account.tmp{other}")),
        ];
        for path in orphans.iter().chain(&kept) {
            fs::write(path, "x").unwrap();
        }
        Core::for_test(&dir).sweep_temporary();
        for path in &orphans {
            assert!(!path.exists(), "{} left", path.display());
        }
        for path in &kept {
            assert!(path.exists(), "{} removed", path.display());
        }
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

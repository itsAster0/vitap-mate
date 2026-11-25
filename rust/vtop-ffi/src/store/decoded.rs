//! Cached files kept decoded: [`crate::Core::read_kept`] keeps what it parsed from the semester list,
//! the pick and each page, keyed by the file's path and type, so the next read of an unchanged
//! file costs a `stat` and a clone instead of a read and a parse. A hit needs the file's length,
//! modified time and inode as they were before it was read, so a file replaced any other way
//! (a test, an older build) is read again. [`crate::Core::write_bytes`] drops the path,
//! [`crate::Core::forget`] (sign-out, another student, cleared pages) everything in the core's
//! folder, and Developer's removals what they remove. Each core keeps its own ([`crate::Shared`]):
//! its budget is its own, and the copies go with it. Two cores on one folder (tests) each catch
//! the other's writes by the stamp.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::SystemTime;

/// What the decoded copies may add up to, in the JSON bytes they came from; the least recently
/// used go first (the pages cache on disk has no cap).
const BUDGET: u64 = 4 << 20;
/// A bigger file is read every time.
const LARGEST: u64 = BUDGET / 4;

/// The file as it was before it was read.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
    inode: u64,
}

impl Stamp {
    fn of(path: &Path) -> Option<Self> {
        let meta = fs::metadata(path).ok()?;
        #[cfg(unix)]
        let inode = std::os::unix::fs::MetadataExt::ino(&meta);
        #[cfg(not(unix))]
        let inode = 0;
        Some(Self {
            len: meta.len(),
            modified: meta.modified().ok(),
            inode,
        })
    }
}

struct Entry {
    stamp: Stamp,
    value: Arc<dyn Any + Send + Sync>,
    used: u64,
}

#[derive(Default)]
struct Cache {
    entries: HashMap<(PathBuf, TypeId), Entry>,
    bytes: u64,
    clock: u64,
    /// Reads parsing outside the lock, by token: one whose path [`forget`] dropped meanwhile
    /// may hold what was there before, so it isn't kept.
    reads: HashMap<u64, Reading>,
    next_read: u64,
}

struct Reading {
    path: PathBuf,
    spoiled: bool,
}

/// A read's token, given back however the read ends.
struct Token<'a>(&'a Decoded, Option<u64>);

impl Drop for Token<'_> {
    fn drop(&mut self) {
        if let Some(token) = self.1 {
            self.0.cache().reads.remove(&token);
        }
    }
}

/// One core's decoded copies.
#[derive(Default)]
pub(crate) struct Decoded(Mutex<Cache>);

impl Decoded {
    fn cache(&self) -> MutexGuard<'_, Cache> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// `path` decoded as `T`: the kept copy while the file is unchanged, else `load(bytes)`, kept.
    pub(crate) fn read<T: Clone + Send + Sync + 'static>(
        &self,
        path: &Path,
        load: impl FnOnce(&[u8]) -> Option<T>,
    ) -> Option<T> {
        let key = (path.to_path_buf(), TypeId::of::<T>());
        let Some(stamp) = Stamp::of(path) else {
            self.forget(path);
            return None;
        };
        let mut token = {
            let mut cache = self.cache();
            cache.clock += 1;
            let clock = cache.clock;
            if let Some(entry) = cache.entries.get_mut(&key)
                && entry.stamp == stamp
                && let Some(value) = entry.value.downcast_ref::<T>()
            {
                entry.used = clock;
                return Some(value.clone());
            }
            let token = cache.next_read;
            cache.next_read += 1;
            let reading = Reading {
                path: path.to_path_buf(),
                spoiled: false,
            };
            cache.reads.insert(token, reading);
            Token(self, Some(token))
        };
        let bytes = fs::read(path).ok()?;
        let value = load(&bytes)?;
        let mut cache = self.cache();
        let fresh = token
            .1
            .take()
            .and_then(|token| cache.reads.remove(&token))
            .is_some_and(|reading| !reading.spoiled);
        if fresh && stamp.len <= LARGEST {
            let used = cache.clock;
            let entry = Entry {
                stamp,
                value: Arc::new(value.clone()),
                used,
            };
            if let Some(old) = cache.entries.insert(key, entry) {
                cache.bytes -= old.stamp.len;
            }
            cache.bytes += stamp.len;
            while cache.bytes > BUDGET {
                let Some(oldest) = cache
                    .entries
                    .iter()
                    .min_by_key(|(_, entry)| entry.used)
                    .map(|(key, _)| key.clone())
                else {
                    break;
                };
                if let Some(gone) = cache.entries.remove(&oldest) {
                    cache.bytes -= gone.stamp.len;
                }
            }
        }
        Some(value)
    }

    /// Drops the copies of `path` and of every file under it (written, removed, or a folder
    /// removed).
    pub(crate) fn forget(&self, path: &Path) {
        let mut cache = self.cache();
        let mut freed = 0;
        cache.entries.retain(|(at, _), entry| {
            let keep = !at.starts_with(path);
            if !keep {
                freed += entry.stamp.len;
            }
            keep
        });
        cache.bytes -= freed;
        for reading in cache.reads.values_mut() {
            if reading.path.starts_with(path) {
                reading.spoiled = true;
            }
        }
    }

    /// How many copies are kept, for tests.
    #[cfg(test)]
    pub(crate) fn kept(&self, path: &Path) -> usize {
        self.cache()
            .entries
            .keys()
            .filter(|(at, _)| at == path)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_forgotten_while_parsing_is_not_kept() {
        let dir = std::env::temp_dir().join(format!("vtop-ffi-decoded-gen-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("page.json");
        fs::write(&path, "1").unwrap();
        let decoded = Decoded::default();
        let read = decoded.read(&path, |bytes| {
            // A write lands while this read parses, outside the lock.
            decoded.forget(&path);
            Some(bytes.to_vec())
        });
        assert_eq!(read.as_deref(), Some(&b"1"[..]));
        assert_eq!(decoded.kept(&path), 0);
        // Undisturbed, the next read is kept.
        decoded.read(&path, |bytes| Some(bytes.to_vec()));
        assert_eq!(decoded.kept(&path), 1);
        fs::remove_dir_all(&dir).unwrap();
    }
}

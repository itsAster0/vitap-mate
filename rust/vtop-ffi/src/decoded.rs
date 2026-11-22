//! Cached files kept decoded: [`crate::Core::read_kept`] keeps what it parsed from the semester list,
//! the pick and each page, keyed by the file's path and type, so the next read of an unchanged
//! file costs a `stat` and a clone instead of a read and a parse. A hit needs the file's length,
//! modified time and inode as they were before it was read, so a file replaced any other way
//! (a test, an older build) is read again. [`crate::Core::write_bytes`] drops the path,
//! [`crate::Core::forget`] (sign-out, another student, cleared pages) everything in the core's
//! folder, and Developer's removals what they remove. Process-wide, keyed by full path, so two
//! cores on one folder share it.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};
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
}

static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(Mutex::default);

fn cache() -> MutexGuard<'static, Cache> {
    CACHE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `path` decoded as `T`: the kept copy while the file is unchanged, else `load(bytes)`, kept.
pub(crate) fn read<T: Clone + Send + Sync + 'static>(
    path: &Path,
    load: impl FnOnce(&[u8]) -> Option<T>,
) -> Option<T> {
    let key = (path.to_path_buf(), TypeId::of::<T>());
    let Some(stamp) = Stamp::of(path) else {
        forget(path);
        return None;
    };
    {
        let mut cache = cache();
        cache.clock += 1;
        let clock = cache.clock;
        if let Some(entry) = cache.entries.get_mut(&key)
            && entry.stamp == stamp
            && let Some(value) = entry.value.downcast_ref::<T>()
        {
            entry.used = clock;
            return Some(value.clone());
        }
    }
    let bytes = fs::read(path).ok()?;
    let value = load(&bytes)?;
    if stamp.len <= LARGEST {
        let mut cache = cache();
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
pub(crate) fn forget(path: &Path) {
    let mut cache = cache();
    let mut freed = 0;
    cache.entries.retain(|(at, _), entry| {
        let keep = !at.starts_with(path);
        if !keep {
            freed += entry.stamp.len;
        }
        keep
    });
    cache.bytes -= freed;
}

/// How many copies are kept, for tests.
#[cfg(test)]
pub(crate) fn kept(path: &Path) -> usize {
    cache().entries.keys().filter(|(at, _)| at == path).count()
}

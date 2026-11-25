//! What the client keeps in `state_dir`: the saved sessions (`session.N`, each locked by
//! one run), the device key (`device.key`) and the atomic writes they all go through.

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{WireError, io};
use crate::frame::RESET_TOKEN;
use crate::{hex, unhex};

const SLOTS: usize = 4;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Saved {
    pub(super) sid: u32,
    pub(super) send: [u8; 32],
    pub(super) recv: [u8; 32],
    pub(super) next_ctr: u32,
    pub(super) port: usize,
    pub(super) created_ms: u64,
    /// The gateway's proof for a reset of this session.
    pub(super) reset: [u8; RESET_TOKEN],
    /// The gateway key the session was made with: a session for another
    /// gateway, or from before its key changed, is not used.
    pub(super) gateway: [u8; 32],
}

impl Saved {
    fn parse(text: &str) -> Option<Self> {
        let f: Vec<&str> = text.split_whitespace().collect();
        let [sid, send, recv, next, port, created, reset, gateway] = f[..] else {
            return None; // also a session saved by an older version: hand-shake again
        };
        Some(Self {
            sid: sid.parse().ok()?,
            send: unhex(send)?.try_into().ok()?,
            recv: unhex(recv)?.try_into().ok()?,
            next_ctr: next.parse().ok()?,
            port: port.parse().ok()?,
            created_ms: created.parse().ok()?,
            reset: unhex(reset)?.try_into().ok()?,
            gateway: unhex(gateway)?.try_into().ok()?,
        })
    }

    fn format(&self) -> String {
        format!(
            "{} {} {} {} {} {} {} {}\n",
            self.sid,
            hex(&self.send),
            hex(&self.recv),
            self.next_ctr,
            self.port,
            self.created_ms,
            hex(&self.reset),
            hex(&self.gateway)
        )
    }
}

/// A session file this run holds the lock on.
pub(super) struct Slot {
    file: File,
}

impl Slot {
    /// The first free slot; None when all are busy (the run then does a
    /// handshake and keeps nothing).
    pub(super) fn claim(dir: &Path) -> Option<Self> {
        (0..SLOTS).find_map(|i| {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(dir.join(format!("session.{i}")))
                .ok()?;
            file.try_lock().ok()?;
            Some(Self { file })
        })
    }

    pub(super) fn read(&mut self) -> Option<Saved> {
        let mut text = String::new();
        self.file.seek(SeekFrom::Start(0)).ok()?;
        self.file.read_to_string(&mut text).ok()?;
        Saved::parse(&text)
    }

    /// Durable before it returns: the counter reservation depends on it.
    pub(super) fn write(&mut self, saved: &Saved) -> bool {
        let text = saved.format();
        self.file.set_len(0).is_ok()
            && self.file.seek(SeekFrom::Start(0)).is_ok()
            && self.file.write_all(text.as_bytes()).is_ok()
            && self.file.sync_all().is_ok()
    }
}

pub(super) fn device_key(dir: &Path) -> Result<[u8; 32], WireError> {
    let path = dir.join("device.key");
    if let Some(key) = fs::read_to_string(&path)
        .ok()
        .and_then(|t| unhex(&t))
        .and_then(|k| k.try_into().ok())
    {
        return Ok(key);
    }
    let (private, _) = crate::try_generate_keypair()
        .ok_or_else(|| WireError::Io("no random source for the device key".into()))?;
    write_atomic(&path, hex(&private).as_bytes()).map_err(io)?;
    Ok(private)
}

/// Writes `bytes` to `path` through a temporary file of this write's own beside it
/// (`<file>.<pid>.<n>.tmp`, the app's pattern, so it sweeps one a crash left), synced before it
/// replaces `path`: a crash leaves the old file or the new one, and two writers never share one.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut name = path
        .file_name()
        .map(OsStr::to_os_string)
        .unwrap_or_default();
    name.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let tmp = path.with_file_name(name);
    let written = File::create(&tmp)
        .and_then(|mut file| file.write_all(bytes).and_then(|()| file.sync_all()))
        .and_then(|()| fs::rename(&tmp, path));
    if written.is_err()
        && let Err(error) = fs::remove_file(&tmp)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        log::warn!("temporary file not removed: {error}");
    }
    written
}

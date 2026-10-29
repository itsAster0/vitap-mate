//! vtop-core's `log` records as JSON lines in `rust.log`, beside the app's
//! `app.log` and in the same format (see `lib/observability/log.dart`),
//! rolled over to `rust.log.1` past [`MAX_BYTES`]. Lines before
//! [`native_logs_open`] are dropped.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_BYTES: u64 = 1024 * 1024;

struct LogFile {
    path: PathBuf,
    file: File,
}

static LOG_FILE: Mutex<Option<LogFile>> = Mutex::new(None);

fn log_file() -> MutexGuard<'static, Option<LogFile>> {
    // A poisoned lock only means another write panicked; keep logging.
    LOG_FILE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Starts writing Rust's log to `dir/rust.log`.
pub fn native_logs_open(dir: String) {
    let path = PathBuf::from(dir).join("rust.log");
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
        *log_file() = Some(LogFile { path, file });
    }
}

#[flutter_rust_bridge::frb(ignore)]
struct NativeLogger;

impl log::Log for NativeLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_millis());
        let line = serde_json::json!({
            "t": millis as u64,
            "l": record.level().as_str().to_lowercase(),
            "src": record.target(),
            "msg": record.args().to_string(),
        });
        let mut guard = log_file();
        let Some(log) = guard.as_mut() else { return };
        // Logging never breaks the app: a full disk only loses lines.
        if log.file.metadata().is_ok_and(|meta| meta.len() > MAX_BYTES) {
            let _ = fs::rename(&log.path, log.path.with_extension("log.1"));
            if let Ok(file) = File::create(&log.path) {
                log.file = file;
            }
        }
        let _ = writeln!(log.file, "{line}");
    }

    fn flush(&self) {}
}

/// Routes vtop-core's logging to the log file. Safe to call more than once.
pub(crate) fn install_native_logger() {
    static LOGGER: NativeLogger = NativeLogger;
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

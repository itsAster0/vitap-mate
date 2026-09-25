use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_NATIVE_LOGS: usize = 500;

fn native_logs_store() -> &'static Mutex<Vec<String>> {
    static STORE: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Vec::new()))
}

fn now_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

/// Forwards `log` records from vtop-core into the in-app log buffer.
#[flutter_rust_bridge::frb(ignore)]
struct NativeLogger;

impl log::Log for NativeLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            append_native_log(
                record.level().as_str(),
                record.target(),
                &record.args().to_string(),
            );
        }
    }

    fn flush(&self) {}
}

/// Routes vtop-core's logging into [`append_native_log`]. Safe to call more
/// than once.
pub(crate) fn install_native_logger() {
    static LOGGER: NativeLogger = NativeLogger;
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

pub fn append_native_log(level: &str, source: &str, message: &str) {
    let line = format!(
        "[{}][{}][{}] {}",
        now_epoch_millis(),
        level,
        source,
        message
    );

    println!("{line}");

    let mut logs = native_logs_store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    logs.insert(0, line);
    if logs.len() > MAX_NATIVE_LOGS {
        logs.truncate(MAX_NATIVE_LOGS);
    }
}

#[flutter_rust_bridge::frb(sync)]
pub fn native_logs_get_entries() -> Vec<String> {
    native_logs_store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

#[flutter_rust_bridge::frb(sync)]
pub fn native_logs_clear() {
    native_logs_store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
}

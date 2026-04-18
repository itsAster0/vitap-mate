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

pub fn append_native_log(level: &str, source: &str, message: &str) {
    let line = format!(
        "[{}][{}][{}] {}",
        now_epoch_millis(),
        level,
        source,
        message
    );

    println!("{line}");

    let mut logs = native_logs_store().lock().unwrap();
    logs.insert(0, line);
    if logs.len() > MAX_NATIVE_LOGS {
        logs.truncate(MAX_NATIVE_LOGS);
    }
}

#[flutter_rust_bridge::frb(sync)]
pub fn native_logs_get_entries() -> Vec<String> {
    native_logs_store().lock().unwrap().clone()
}

#[flutter_rust_bridge::frb(sync)]
pub fn native_logs_clear() {
    native_logs_store().lock().unwrap().clear();
}

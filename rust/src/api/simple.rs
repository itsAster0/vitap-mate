#[flutter_rust_bridge::frb(sync)]
pub fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Not setup_default_user_utils: its Trace-level console logger claims
    // the global logger first, so ours never installs and the HTML parser
    // logs every selector match while pages are parsed.
    flutter_rust_bridge::setup_backtrace();
    crate::api::native_logs::install_native_logger();
}

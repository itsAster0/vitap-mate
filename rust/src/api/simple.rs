#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Not setup_default_user_utils: its Trace-level console logger claims
    // the global logger first, so ours never installs and the HTML parser
    // logs every selector match while pages are parsed.
    flutter_rust_bridge::setup_backtrace();
    crate::api::native_logs::install_native_logger();
}

/// Developer → Inspect Traffic: sends VTOP and Gmail requests through
/// `proxy` (`host:port`), trusting `ca_pems` too. Call once at launch,
/// before the first request.
#[flutter_rust_bridge::frb(sync)]
pub fn set_traffic_inspection(proxy: Option<String>, ca_pems: Vec<String>) {
    vtop_core::client::set_inspection(vtop_core::client::Inspection { proxy, ca_pems });
}

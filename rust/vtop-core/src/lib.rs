//! VTOP client core: login, session handling, fetchers and HTML parsers.
//!
//! This crate knows nothing about Flutter or HTTP servers. `rust_lib_vitapmate`
//! wraps it for the app over flutter_rust_bridge and `vtop-server` exposes it
//! over HTTP.
//!
//! Logging goes through the [`log`] facade with the targets `rust.auth` and
//! `rust.network`. Cookies, passwords, OTPs and CSRF tokens are never logged.

mod captcha;
pub mod client;
pub mod config;
pub mod error;
pub mod gmail;
pub mod inputs;
pub mod parser;
pub mod session;
pub mod types;

pub use client::VtopClient;
pub use config::VtopConfig;
pub use error::{VtopError, VtopResult};
pub use session::SessionState;

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds since the unix epoch; 1 if the clock is before it.
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(1)
}

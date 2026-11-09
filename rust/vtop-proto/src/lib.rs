//! VTOP without I/O: HTML parsers, the typed data they return, validated
//! inputs, errors, and the [`http`] request/response shapes a transport
//! carries. No async runtime, networking or TLS; the app (Kotlin, OkHttp)
//! and the server (reqwest) each bring their own transport.
//!
//! Nothing here logs. Cookies, passwords, OTPs and CSRF tokens never appear
//! in a `Debug` output.

pub mod captcha;
pub mod course_pages;
pub mod error;
pub mod http;
pub mod inputs;
pub mod login;
pub mod machine;
pub mod otp;
pub mod outing;
pub mod pages;
pub mod parser;
pub mod types;

pub use error::{VtopError, VtopResult};

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds since the unix epoch; 1 if the clock is before it.
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(1)
}

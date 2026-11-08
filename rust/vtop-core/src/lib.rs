//! VTOP client core: login, session handling, fetchers and HTML parsers.
//!
//! The parsers, types, inputs and errors live in `vtop-proto` (no I/O) and are
//! re-exported here at their old paths. `vtop-ffi` wraps this crate for the
//! app; `vtop-bridge` and `vtop-mcp` use it on the server.
//!
//! Logging goes through the [`log`] facade with the targets `rust.auth` and
//! `rust.network`. Cookies, passwords, OTPs and CSRF tokens are never logged.

mod captcha;
pub mod client;
pub mod config;
pub mod gmail;
pub mod session;

pub use client::VtopClient;
pub use config::VtopConfig;
pub use session::SessionState;
pub use vtop_proto::{error, http, inputs, now_unix, parser, types};
pub use vtop_proto::{VtopError, VtopResult};
pub use vtop_shared::mail_seal;
pub use vtop_shared::page_dictionary;

//! Proves the registration number of a VTOP session with TLSNotary (proxy mode), without the
//! verifier seeing the session cookie. The app proves ([`session::prove`]); the engine relays
//! the app's TLS connection to VTOP, which it dials itself, and checks what the proof reveals
//! ([`session::verify`]). See `ARCHITECTURE.md`, "Account proof".
//!
//! - [`pattern`]: byte patterns with literals, hidden holes and revealed claims.
//! - [`http`]: one HTTP/1.1 response's framing over a partly hidden transcript.
//! - [`statement`]: request + page patterns; `check` (verifier) and `plan` (prover).
//! - [`content`]: the `/vtop/content` statement and the device binding.
//! - [`session`]: the tlsn prover and verifier sessions, and [`session::Pipe`], a byte stream
//!   over messages (a WebSocket).

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::string_slice,
        clippy::panic
    )
)]

pub mod content;
#[cfg(feature = "fake-vtop")]
pub mod fake;
pub mod http;
pub mod pattern;
pub mod session;
pub mod statement;

pub use tlsn::webpki::RootCertStore;

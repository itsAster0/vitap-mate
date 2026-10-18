//! Formats the app, the edge Worker and the engine must agree on. No I/O, so
//! it builds for native targets and for `wasm32-unknown-unknown`.

pub mod account;
pub mod cookies;
pub mod engine;
pub mod events;
pub mod mail_seal;
pub mod mailbox;
pub mod tokens;

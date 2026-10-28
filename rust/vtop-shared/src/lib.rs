//! Formats the app and the engine share with each other and with the edge
//! Worker. The edge is TypeScript and keeps its own copy of what it needs
//! (`vtop-edge/src/shared/`); a change here must be made there too. No
//! I/O, so it builds for every target.

pub mod account;
pub mod engine;
pub mod mail_seal;
pub mod mailbox;
pub mod page_dictionary;
pub mod tokens;
pub mod wire_page;

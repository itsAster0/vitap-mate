//! A Convex sync client on the app's WebSocket, with no async runtime: `base_client` is
//! convex-rs's state machine, `client` drives it from the socket's callbacks and one timer thread.
//! The server (`backend/`) is reached only through it ([`crate::remote::backend`]).

mod base_client;
mod client;
mod socket;
pub(crate) mod value;

#[cfg(test)]
mod tests;

pub(crate) use client::{CallError, ConvexClient, Served, SubscriptionId};
pub use socket::{ConvexNetwork, ConvexSocket, ConvexSocketListener};

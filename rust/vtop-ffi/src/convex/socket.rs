//! The WebSocket the Convex client runs on: the app opens it (`SocketConvexNetwork` in Kotlin) and Rust is told what
//! happens to it.

use std::sync::Arc;

/// Opens WebSockets (the app's, so they share its network setup).
///
/// The client never calls these while holding its lock, but implementations should still not
/// call the listener synchronously from inside `open`, `send` or `disconnect`: deliver its calls
/// from the socket's own thread (as `SocketConvexNetwork` does).
#[uniffi::export(with_foreign)]
pub trait ConvexNetwork: Send + Sync {
    /// Starts connecting to `url` and returns at once. Must not call `listener` before
    /// returning.
    fn open(&self, url: String, listener: Arc<dyn ConvexSocketListener>) -> Arc<dyn ConvexSocket>;
}

/// One open (or opening) WebSocket.
#[uniffi::export(with_foreign)]
pub trait ConvexSocket: Send + Sync {
    /// Queues a text frame; false when the socket is closing or closed. Must not call the
    /// listener synchronously.
    fn send(&self, text: String) -> bool;
    /// Starts closing. Must not call the listener synchronously. (Not `close`: UniFFI's Kotlin
    /// objects already have one.)
    fn disconnect(&self);
}

/// What happens to a socket, from the app's socket threads.
#[uniffi::export]
pub trait ConvexSocketListener: Send + Sync {
    fn on_open(&self);
    fn on_text(&self, text: String);
    /// The socket closed or failed to open; it gets no more calls.
    fn on_closed(&self, reason: String);
}

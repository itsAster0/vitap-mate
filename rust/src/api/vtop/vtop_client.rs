//! The opaque client handle Dart holds.

pub use super::types::*;
pub use super::vtop_errors::{VtopError, VtopResult};

/// Wraps `vtop_core::VtopClient` so Dart keeps seeing the same opaque
/// `VtopClient` class it always has.
#[flutter_rust_bridge::frb(opaque)]
pub struct VtopClient {
    pub(crate) inner: vtop_core::VtopClient,
}

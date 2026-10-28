//! Identifiers that only need to be unpredictable enough not to collide:
//! session ids, request ids, handshake tags. None of them is a secret (they
//! travel in clear headers); keys come from snow's own random source.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

/// `RandomState` is seeded from the OS once per thread and stepped on each
/// call, so successive values differ even within the same nanosecond.
pub fn u64() -> u64 {
    let mut h = RandomState::new().build_hasher();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    h.write_u128(nanos);
    h.finish()
}

pub fn u32() -> u32 {
    u64() as u32
}

pub fn u16() -> u16 {
    u64() as u16
}

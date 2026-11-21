//! The phone ↔ gateway transport for slow, lossy networks ("UDP mode").
//!
//! One request, one response, over UDP, built for links of 1–8 kbit/s that
//! lose 10–30% of packets, where TLS over TCP spends most of its time on
//! certificates and retransmission timers. See `ARCHITECTURE.md` for the
//! measurements behind each choice.
//!
//! - **Handshake** ([`client`], [`gateway`]): Noise IK. The gateway's static
//!   key is built into the app, and the request rides in message 1, so a new
//!   session costs one round trip and ~250 bytes. Message 1 carries a
//!   timestamp that must be fresh and newer than the device's last one, so a
//!   captured message 1 cannot be replayed.
//! - **Framing** ([`frame`]): the split keys encrypt every later packet with
//!   ChaCha20-Poly1305 and an explicit counter (9-byte header, 16-byte tag),
//!   so a session survives app restarts without another handshake.
//! - **Transfer** ([`fec`]): the response goes out as Reed-Solomon symbols,
//!   all at once, paced at the client's measured rate. Any `k` symbols
//!   rebuild it; a request for more gets fresh repair symbols, never repeats.
//! - **Loss detection** ([`link`]): from expected arrival times, not a
//!   silence timer; rate, round trip and loss are learned per network and
//!   kept between runs.

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

pub mod client;
pub mod fec;
pub mod frame;
pub mod gateway;
pub mod link;
pub mod msg;
mod noise;
mod rand;
#[cfg(test)]
mod tests;

/// The Noise pattern: the client knows the gateway's static key (IK), so its
/// first message is already encrypted and authenticated.
pub const NOISE: &str = "Noise_IK_25519_ChaChaPoly_BLAKE2s";

/// The gateway's UDP ports, in the order a client tries them: STUN's port
/// (open wherever video calls work), WireGuard's, then 443 for networks that
/// only block the others. Some networks drop UDP 443 to force QUIC back to TCP.
pub const PORTS: [u16; 3] = [3478, 51820, 443];

/// Generates a Noise static key pair: (private, public), or `None` if the OS
/// random source fails.
pub fn try_generate_keypair() -> Option<([u8; 32], [u8; 32])> {
    let private = noise::random_key()?;
    Some((private, noise::public_key(&private)))
}

/// [`try_generate_keypair`], for the gateway's key rotation. A key must never
/// come from a weak source, so if the OS random source keeps failing after a
/// few tries the process aborts rather than return one.
pub fn generate_keypair() -> ([u8; 32], [u8; 32]) {
    for _ in 0..8 {
        if let Some(pair) = try_generate_keypair() {
            return pair;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    log::error!("the OS random source fails: no key can be made");
    std::process::abort()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn unhex(text: &str) -> Option<Vec<u8>> {
    let text = text.trim();
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(text.get(2 * i..2 * i + 2)?, 16).ok())
        .collect()
}

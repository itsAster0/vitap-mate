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

pub mod client;
pub mod fec;
pub mod frame;
pub mod gateway;
pub mod link;
pub mod msg;
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

/// Generates a Noise static key pair: (private, public).
pub fn generate_keypair() -> ([u8; 32], [u8; 32]) {
    let kp = snow::Builder::new(NOISE.parse().expect("valid Noise pattern"))
        .generate_keypair()
        .expect("key generation");
    (
        kp.private.try_into().expect("32-byte key"),
        kp.public.try_into().expect("32-byte key"),
    )
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

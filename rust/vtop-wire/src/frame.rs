//! Packet framing after the handshake.
//!
//! `[type u8][session id u32][counter u32]` + ChaCha20-Poly1305(plaintext),
//! with the 9-byte header as associated data. The counter is the nonce, so it
//! must never repeat under one key: the client reserves counters in blocks
//! and saves the block's end before using any of it (see [`crate::client`]).
//! The tag stays at 16 bytes: RFC 8439 §4 forbids truncating it.
//!
//! Handshake packets have their own layout (see [`crate::msg`]).

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};

/// Client → gateway: a request, a request for more symbols, or a pure ack.
pub const T_REQ: u8 = 0x11;
/// Gateway → client: one response symbol.
pub const T_DATA: u8 = 0x12;
/// Client → gateway: the response arrived; drop anything still queued for it.
pub const T_DONE: u8 = 0x13;
/// Gateway → client: the request is in hand and the answer is not ready yet.
pub const T_WORKING: u8 = 0x14;
/// Client → gateway: Noise message 1.
pub const T_HS1: u8 = 0x15;
/// Gateway → client: Noise message 2.
pub const T_HS2: u8 = 0x16;
/// Gateway → client: "no such session" (after a gateway restart), as
/// `[0x17][sid u32][reset token, 16 bytes]`. The gateway no longer has the
/// session's keys, so it cannot encrypt this; it proves itself with the
/// session's [`reset_token`] instead, which it sent inside the encrypted HS2
/// and can recompute from its own key. The client ignores a reset without
/// the right token, so nothing unencrypted or unauthenticated is acted on.
pub const T_RESET: u8 = 0x17;
pub const RESET_TOKEN: usize = 16;
/// Gateway → client: "Noise message 1 does not decrypt", as `[0x18][the
/// first 4 bytes of its ephemeral key]`: the client holds a wrong or old
/// public key for this gateway. Unauthenticated, so the client only stops
/// trying and fetches the key again over HTTPS.
pub const T_WRONG_KEY: u8 = 0x18;

pub const HEADER: usize = 9;
pub const TAG: usize = 16;
/// IPv4 + UDP headers, counted in every size estimate.
pub const IP_UDP: usize = 28;

pub struct Keys {
    send: ChaCha20Poly1305,
    recv: ChaCha20Poly1305,
}

pub struct Opened {
    pub ptype: u8,
    pub sid: u32,
    pub ctr: u32,
    pub plain: Vec<u8>,
}

impl Keys {
    pub fn new(send: &[u8; 32], recv: &[u8; 32]) -> Self {
        Self {
            send: ChaCha20Poly1305::new(send.into()),
            recv: ChaCha20Poly1305::new(recv.into()),
        }
    }

    pub fn seal(&self, ptype: u8, sid: u32, ctr: u32, plain: &[u8]) -> Vec<u8> {
        let mut packet = Vec::with_capacity(HEADER + plain.len() + TAG);
        packet.push(ptype);
        packet.extend_from_slice(&sid.to_be_bytes());
        packet.extend_from_slice(&ctr.to_be_bytes());
        let sealed = self
            .send
            .encrypt(
                &nonce(ctr),
                Payload {
                    msg: plain,
                    aad: &packet,
                },
            )
            .expect("ChaCha20-Poly1305 encryption cannot fail for in-range sizes");
        packet.extend_from_slice(&sealed);
        packet
    }

    pub fn open(&self, packet: &[u8]) -> Option<Opened> {
        let (ptype, sid, ctr) = header(packet)?;
        let plain = self
            .recv
            .decrypt(
                &nonce(ctr),
                Payload {
                    msg: &packet[HEADER..],
                    aad: &packet[..HEADER],
                },
            )
            .ok()?;
        Some(Opened {
            ptype,
            sid,
            ctr,
            plain,
        })
    }
}

/// The reset token for session `sid`: a MAC under a key derived from the
/// gateway's static key, so the gateway can recompute it after a restart and
/// nobody else can produce it (QUIC's stateless reset, RFC 9000 §10.3).
pub fn reset_token(gateway_key: &[u8; 32], sid: u32) -> [u8; RESET_TOKEN] {
    // Derive a separate key first, so the static key itself never keys the MAC.
    let derived = ChaCha20Poly1305::new(gateway_key.into())
        .encrypt(Nonce::from_slice(b"vtop-reset-1"), [0u8; 32].as_slice())
        .expect("ChaCha20-Poly1305 encryption cannot fail for in-range sizes");
    let mut n = [0u8; 12];
    n[..4].copy_from_slice(&sid.to_be_bytes());
    n[4..].copy_from_slice(b"reset-id");
    let tag = ChaCha20Poly1305::new(derived[..32].into())
        .encrypt(&n.into(), [].as_slice())
        .expect("ChaCha20-Poly1305 encryption cannot fail for in-range sizes");
    tag.try_into().expect("a 16-byte tag")
}

/// Compares in constant time.
pub fn same_token(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// (type, session id, counter) of a framed packet, before decryption.
pub fn header(packet: &[u8]) -> Option<(u8, u32, u32)> {
    if packet.len() < HEADER + TAG {
        return None;
    }
    Some((
        packet[0],
        u32::from_be_bytes(packet[1..5].try_into().ok()?),
        u32::from_be_bytes(packet[5..9].try_into().ok()?),
    ))
}

fn nonce(ctr: u32) -> Nonce {
    let mut n = [0u8; 12];
    n[4..8].copy_from_slice(&ctr.to_le_bytes());
    n.into()
}

/// WireGuard-style sliding window over the last 64 counters.
#[derive(Default)]
pub struct ReplayWindow {
    top: u32,
    bits: u64,
    started: bool,
}

impl ReplayWindow {
    /// Records `ctr`; false if it was seen before or is too old to tell.
    pub fn accept(&mut self, ctr: u32) -> bool {
        if !self.started {
            self.started = true;
            self.top = ctr;
            self.bits = 1;
            return true;
        }
        if ctr > self.top {
            let shift = ctr - self.top;
            self.bits = if shift >= 64 { 0 } else { self.bits << shift };
            self.bits |= 1;
            self.top = ctr;
            return true;
        }
        let back = self.top - ctr;
        if back >= 64 || self.bits & (1 << back) != 0 {
            return false;
        }
        self.bits |= 1 << back;
        true
    }
}

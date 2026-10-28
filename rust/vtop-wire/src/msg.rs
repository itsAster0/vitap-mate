//! Plaintext layouts inside the frames, and the two handshake packets.
//!
//! - `T_HS1`: `[0x15]` + Noise message 1, whose payload is a timestamp
//!   (µs, u64) and a [`Req`].
//! - `T_HS2`: `[0x16][first 4 bytes of the client's ephemeral key]` + Noise
//!   message 2, whose payload is the session id (u32) and the session's
//!   reset token (16 bytes, see [`crate::frame::T_RESET`]).

/// A request must fit one datagram with room for the handshake around it.
pub const MAX_REQUEST: usize = 1000;
/// "Loss unknown" in [`Req::loss_pct`].
pub const LOSS_UNKNOWN: u8 = 255;

/// Client → gateway. `round` 0 is the request itself; later rounds ask for
/// `want` fresh symbols. `want` 0 on a known request is a pure ack (it proves
/// the client's address). `body` is sent until the client hears anything back,
/// in case the gateway never got round 0.
#[derive(Clone, Debug, PartialEq)]
pub struct Req {
    pub req_id: u16,
    pub round: u8,
    pub want: u8,
    /// Cautious loss estimate in %, or [`LOSS_UNKNOWN`].
    pub loss_pct: u8,
    /// Measured link rate in bytes/s; 0 when not measured yet.
    pub rate: u16,
    pub rtt_ms: u16,
    /// The newest gateway counter received + 1 (0: nothing yet). Proves the
    /// client receives at its current address.
    pub last_ctr: u32,
    pub body: Vec<u8>,
}

impl Req {
    pub fn encode(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(13 + self.body.len());
        v.extend_from_slice(&self.req_id.to_be_bytes());
        v.extend_from_slice(&[self.round, self.want, self.loss_pct]);
        v.extend_from_slice(&self.rate.to_be_bytes());
        v.extend_from_slice(&self.rtt_ms.to_be_bytes());
        v.extend_from_slice(&self.last_ctr.to_be_bytes());
        v.extend_from_slice(&self.body);
        v
    }

    pub fn decode(b: &[u8]) -> Option<Self> {
        if b.len() < 13 {
            return None;
        }
        Some(Self {
            req_id: u16::from_be_bytes([b[0], b[1]]),
            round: b[2],
            want: b[3],
            loss_pct: b[4],
            rate: u16::from_be_bytes([b[5], b[6]]),
            rtt_ms: u16::from_be_bytes([b[7], b[8]]),
            last_ctr: u32::from_be_bytes(b[9..13].try_into().ok()?),
            body: b[13..].to_vec(),
        })
    }
}

/// Gateway → client: one symbol of a response. `pos` and `len` place it in
/// its round's burst, so the client knows how many more are on the way.
#[derive(Debug, PartialEq)]
pub struct Data<'a> {
    pub req_id: u16,
    pub sym: u8,
    pub k: u8,
    pub total: u16,
    pub round: u8,
    pub pos: u8,
    pub len: u8,
    pub bytes: &'a [u8],
}

pub const DATA_HEADER: usize = 9;

impl<'a> Data<'a> {
    pub fn encode(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(DATA_HEADER + self.bytes.len());
        v.extend_from_slice(&self.req_id.to_be_bytes());
        v.extend_from_slice(&[self.sym, self.k]);
        v.extend_from_slice(&self.total.to_be_bytes());
        v.extend_from_slice(&[self.round, self.pos, self.len]);
        v.extend_from_slice(self.bytes);
        v
    }

    pub fn decode(b: &'a [u8]) -> Option<Self> {
        if b.len() < DATA_HEADER {
            return None;
        }
        Some(Self {
            req_id: u16::from_be_bytes([b[0], b[1]]),
            sym: b[2],
            k: b[3],
            total: u16::from_be_bytes([b[4], b[5]]),
            round: b[6],
            pos: b[7],
            len: b[8],
            bytes: &b[DATA_HEADER..],
        })
    }
}

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    now_us() / 1000
}

/// Microseconds since the Unix epoch: message 1's timestamp, fine enough
/// that two isolates of one install do not pick the same one.
pub fn now_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or_default()
}

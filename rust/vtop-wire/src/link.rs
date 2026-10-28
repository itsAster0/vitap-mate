//! What the client knows about the network it is on: rate, round trip,
//! loss, jitter, and which gateway port answers. Kept per network between
//! runs, so a new handshake on a known network starts informed instead of
//! guessing.

use crate::fec::DATA_WIRE;
use crate::msg::LOSS_UNKNOWN;

/// The rate assumed before anything is measured: the slowest link we design
/// for (1 kbit/s), so no timer fires before a packet could have arrived.
pub const RATE_FLOOR: f64 = 125.0;
/// The loss estimate's prior, worth this many packets at 10% loss.
const PRIOR_PACKETS: f64 = 5.0;
const PRIOR_LOSS: f64 = 0.1;
/// Per-packet decay of the loss counts: a window of about 33 packets.
const DECAY: f64 = 0.97;

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    /// Bytes/s at the bottleneck, from packet spacing.
    pub rate: f64,
    /// Whether `rate` was measured (only then is it sent to the gateway,
    /// which paces by it).
    pub measured: bool,
    pub rtt: f64,
    /// Packet loss in both directions as decayed counts.
    pub lost: f64,
    pub sent: f64,
    /// Smoothed deviation of packet spacing, seconds.
    pub jitter: f64,
    /// Index into the port list of the port that last answered.
    pub port: usize,
}

impl Default for Link {
    fn default() -> Self {
        Self {
            rate: RATE_FLOOR,
            measured: false,
            rtt: 0.4,
            lost: 0.0,
            sent: 0.0,
            jitter: 0.0,
            port: 0,
        }
    }
}

impl Link {
    pub fn parse(text: &str) -> Option<Self> {
        let f: Vec<f64> = text
            .split_whitespace()
            .map(|x| x.parse().ok())
            .collect::<Option<_>>()?;
        let [rate, rtt, lost, sent, jitter, measured, port] = f[..] else {
            return None;
        };
        Some(Self {
            rate: rate.max(50.0),
            measured: measured > 0.5,
            rtt: rtt.clamp(0.02, 5.0),
            lost: lost.max(0.0),
            sent: sent.max(0.0),
            jitter: jitter.max(0.0),
            port: port as usize,
        })
    }

    pub fn format(&self) -> String {
        format!(
            "{:.1} {:.3} {:.3} {:.3} {:.4} {} {}\n",
            self.rate, self.rtt, self.lost, self.sent, self.jitter, self.measured as u8, self.port
        )
    }

    /// Cautious loss: the mean (pulled toward 10% while evidence is thin)
    /// plus one standard deviation, so a lucky streak on a bad link does not
    /// switch the spares off.
    pub fn loss(&self) -> f64 {
        let n = self.sent + PRIOR_PACKETS;
        let mean = (self.lost + PRIOR_LOSS * PRIOR_PACKETS) / n;
        (mean + (mean * (1.0 - mean) / n).sqrt()).clamp(0.0, 0.6)
    }

    pub fn known(&self) -> bool {
        self.sent > 0.0
    }

    pub fn loss_pct(&self) -> u8 {
        if self.known() {
            (self.loss() * 100.0).round() as u8
        } else {
            LOSS_UNKNOWN
        }
    }

    /// Control packets go out twice where packets get lost (or may): the
    /// uplink is idle during a download, so the copy costs no download time.
    pub fn duplicate(&self) -> bool {
        !self.known() || self.loss() >= 0.03
    }

    pub fn observe(&mut self, lost: usize, sent: usize) {
        let d = DECAY.powi(sent as i32);
        self.lost = self.lost * d + lost as f64;
        self.sent = self.sent * d + sent as f64;
    }

    /// Time to receive one full data packet.
    pub fn packet_time(&self) -> f64 {
        DATA_WIRE / self.rate
    }

    /// Allowance for a late packet: a quarter packet time, four jitters
    /// (capped at two packet times), at least 25 ms, plus 50 ms.
    pub fn slack(&self) -> f64 {
        let pt = self.packet_time();
        (pt / 4.0).max((4.0 * self.jitter).min(2.0 * pt)).max(0.025) + 0.05
    }

    /// A rate sample from two packets of one burst `per` seconds apart.
    pub fn sample_rate(&mut self, wire_bytes: usize, per: f64, first: bool) {
        let sample = (wire_bytes as f64 / per).clamp(50.0, 1_000_000.0);
        self.rate = if first && !self.measured {
            sample
        } else {
            0.5 * self.rate + 0.5 * sample
        };
        self.measured = true;
        self.jitter = 0.75 * self.jitter + 0.25 * (per - self.packet_time()).abs();
    }

    /// Round-trip samples only ever come out too high (queueing), so keep
    /// the minimum and let a higher sample in slowly.
    pub fn sample_rtt(&mut self, sample: f64) {
        let sample = sample.clamp(0.05, 5.0);
        self.rtt = if sample < self.rtt {
            sample
        } else {
            0.9 * self.rtt + 0.1 * sample
        };
    }
}

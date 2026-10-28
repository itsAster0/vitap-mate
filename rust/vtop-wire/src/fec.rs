//! Reed-Solomon symbols for a response, and how many to send.
//!
//! A response is cut into `k` data symbols of [`SYMBOL`] bytes (one symbol
//! per packet). Repair symbols come from a pool of [`repair_pool`] made up
//! front; any `k` distinct symbols rebuild the response. Unlike XOR parity,
//! a lost repair symbol never leaves one particular hole open: the next
//! fresh symbol fills any hole. At 30% loss with 5 data symbols and 3 repair,
//! one round succeeds 81% of the time against 56% for XOR groups.

/// Bytes per symbol. A full data packet is 28 + 9 + 9 + 400 + 16 = 462 bytes.
pub const SYMBOL: usize = 400;
/// Symbol indexes are a byte; 100 data symbols leave room for the pool.
pub const MAX_K: usize = 100;
pub const MAX_RESPONSE: usize = SYMBOL * MAX_K;
/// Wire size of a full data packet.
pub const DATA_WIRE: f64 = (crate::frame::IP_UDP
    + crate::frame::HEADER
    + crate::msg::DATA_HEADER
    + SYMBOL
    + crate::frame::TAG) as f64;

/// (data symbols, symbol size) for a body. Shards must be even-sized; a
/// one-symbol body is not padded to [`SYMBOL`].
pub fn shape(total: usize) -> (usize, usize) {
    let k = total.div_ceil(SYMBOL).max(1);
    let size = if k == 1 {
        (total.max(2) + 1) & !1
    } else {
        SYMBOL
    };
    (k, size)
}

pub fn repair_pool(k: usize) -> usize {
    (3 * k).clamp(4, 255 - k)
}

/// The data symbols (the last one unpadded, except a lone symbol) and the
/// repair pool for `body`.
pub fn encode(body: &[u8]) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let (k, size) = shape(body.len());
    let mut data: Vec<Vec<u8>> = if body.is_empty() {
        vec![vec![]]
    } else {
        body.chunks(SYMBOL).map(<[u8]>::to_vec).collect()
    };
    let padded: Vec<Vec<u8>> = data
        .iter()
        .map(|d| {
            let mut d = d.clone();
            d.resize(size, 0);
            d
        })
        .collect();
    let repair = reed_solomon_simd::encode(k, repair_pool(k), &padded).expect("valid shard shape");
    if k == 1 {
        data[0] = padded[0].clone();
    }
    (data, repair)
}

/// Rebuilds a body of `total` bytes from at least `k` distinct symbols
/// (index → bytes; indexes ≥ k are repair symbols).
pub fn decode(total: usize, symbols: &std::collections::BTreeMap<u8, Vec<u8>>) -> Option<Vec<u8>> {
    let (k, size) = shape(total);
    let mut data: Vec<Option<Vec<u8>>> = (0..k)
        .map(|i| {
            symbols.get(&(i as u8)).map(|d| {
                let mut d = d.clone();
                d.resize(size, 0);
                d
            })
        })
        .collect();
    if data.iter().any(Option::is_none) {
        let originals: Vec<(usize, Vec<u8>)> = data
            .iter()
            .enumerate()
            .filter_map(|(i, d)| d.clone().map(|d| (i, d)))
            .collect();
        let recovery: Vec<(usize, &Vec<u8>)> = symbols
            .iter()
            .filter(|(s, b)| **s as usize >= k && b.len() == size)
            .map(|(s, b)| (*s as usize - k, b))
            .collect();
        let restored = reed_solomon_simd::decode(k, repair_pool(k), originals, recovery).ok()?;
        for (i, d) in restored {
            data[i] = Some(d);
        }
    }
    let mut body: Vec<u8> = data.into_iter().collect::<Option<Vec<_>>>()?.concat();
    body.truncate(total);
    Some(body)
}

fn binom(n: usize, j: usize) -> f64 {
    (0..j).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

/// P(at least `k` of `n` packets arrive) at loss `p`.
pub fn p_enough(k: usize, n: usize, p: f64) -> f64 {
    (k..=n)
        .map(|j| binom(n, j) * (1.0 - p).powi(j as i32) * p.powi((n - j) as i32))
        .sum()
}

/// Repair symbols to send with the first burst, by expected time. Each costs
/// one packet time `tp`; coming up `d` short costs another round (noticing
/// the loss takes about a packet time plus slack, then the request upload
/// `up` and a round trip; weighted 1.5x since extra rounds make the slow
/// tail) and d / (1 - p) more packet times. Tiny responses on very lossy
/// links always get one spare: the two choices are close on average, and the
/// retry round is what makes a fetch take a minute.
pub fn choose_repair(k: usize, p: f64, tp: f64, up: f64, rtt: f64) -> usize {
    let round = 1.5 * (tp + 0.3 + up + rtt);
    let best = (0..=k.min(8))
        .map(|r| {
            let n = k + r;
            let short: f64 = (0..k)
                .map(|a| {
                    let pa = binom(n, a) * (1.0 - p).powi(a as i32) * p.powi((n - a) as i32);
                    pa * (round + (k - a) as f64 * tp / (1.0 - p).max(0.05))
                })
                .sum();
            (r, n as f64 * tp + short)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(0, |x| x.0);
    best.max(usize::from(k <= 2 && p >= 0.2))
}

/// Extra symbols on top of a deficit, so the next round likely suffices.
pub fn extra_for(deficit: usize, p: f64) -> usize {
    let mut extra = 0;
    while deficit > 0 && extra < 4 && p_enough(deficit, deficit + extra, p) < 0.85 {
        extra += 1;
    }
    extra
}

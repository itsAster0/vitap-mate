//! Byte patterns for a request or a page, matched two ways: by the prover over
//! the plaintext it knows, and by the verifier over a transcript in which the
//! prover's hidden bytes are `None`.
//!
//! A pattern is anchored at offset 0. Every revealed byte the verifier reads
//! is either a literal of the pattern or the value of a claim with a narrow
//! charset, and every hidden run sits where the pattern puts a hole, with a
//! bounded length. So the prover chooses only the hole contents and the claim
//! values; it cannot move a claim to bytes of its own choosing.

use std::ops::{Range, RangeInclusive};

use anyhow::{Result, anyhow, bail, ensure};

pub type ByteOk = fn(u8) -> bool;
pub type ValueOk = fn(&str) -> bool;

#[derive(Clone, Debug)]
pub enum Seg {
    /// Exact bytes, all revealed.
    Lit(Vec<u8>),
    /// Zero or more revealed ASCII whitespace bytes.
    Ws,
    /// A run of hidden bytes with a length in `len`. Always followed by a `Lit`.
    Hole {
        name: &'static str,
        len: RangeInclusive<usize>,
    },
    /// Revealed bytes, each `byte_ok`, 1 to `max` of them, the whole `ok`.
    /// Followed by a `Lit` whose first byte is not `byte_ok`.
    Claim {
        name: &'static str,
        max: usize,
        byte_ok: ByteOk,
        ok: ValueOk,
    },
    /// Revealed decimal digits: the length of what follows `BodyStart`.
    ContentLength,
    /// Where the request body starts; matches no bytes.
    BodyStart,
}

/// What a match found, as offsets into the matched bytes.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Found {
    pub end: usize,
    pub claims: Vec<(&'static str, String)>,
    pub holes: Vec<(&'static str, Range<usize>)>,
    /// Bytes the verifier must see: literals, whitespace, claims, lengths.
    pub revealed: Vec<Range<usize>>,
    pub content_length: Option<(usize, usize)>,
}

impl Found {
    pub fn claim(&self, name: &str) -> Option<&str> {
        self.claims
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.as_str())
    }

    fn reveal(&mut self, r: Range<usize>) {
        if !r.is_empty() {
            self.revealed.push(r);
        }
    }
}

/// The longest hole a pattern may have; keeps offset arithmetic far from
/// overflow even for a pattern that came from elsewhere.
pub const MAX_HOLE: usize = 1 << 24;

/// Checks the shape rules the matchers rely on.
pub fn validate(segs: &[Seg]) -> Result<()> {
    let mut names = std::collections::BTreeSet::new();
    let mut body_starts = 0;
    for (i, seg) in segs.iter().enumerate() {
        let next_lit = match segs.get(i + 1) {
            Some(Seg::Lit(l)) => l.first().copied(),
            _ => None,
        };
        match seg {
            Seg::Hole { name, len } => {
                ensure!(
                    next_lit.is_some(),
                    "hole {name} must be followed by a literal"
                );
                ensure!(
                    len.start() <= len.end() && *len.end() <= MAX_HOLE,
                    "hole {name} has a bad length range"
                );
                ensure!(names.insert(*name), "{name} appears twice");
            }
            // A literal, not whitespace: `Ws` matches zero bytes, so a
            // hidden byte right after the claim would cut it short.
            Seg::Claim { name, byte_ok, .. } => {
                ensure!(
                    next_lit.is_some_and(|b| !byte_ok(b)),
                    "claim {name} must be followed by a literal it cannot run into"
                );
                ensure!(names.insert(*name), "{name} appears twice");
            }
            Seg::ContentLength => {
                ensure!(
                    next_lit.is_some_and(|b| !b.is_ascii_digit()),
                    "Content-Length must be followed by a non-digit literal"
                );
                ensure!(
                    segs.iter().skip(i).any(|s| matches!(s, Seg::BodyStart)),
                    "Content-Length without a BodyStart after it"
                );
            }
            Seg::BodyStart => body_starts += 1,
            Seg::Lit(l) => ensure!(!l.is_empty(), "empty literal"),
            Seg::Ws => {}
        }
    }
    ensure!(body_starts <= 1, "more than one BodyStart");
    Ok(())
}

fn claim_at(
    at: usize,
    byte: impl Fn(usize) -> Option<u8>,
    name: &str,
    max: usize,
    byte_ok: ByteOk,
    ok: ValueOk,
) -> Result<(String, usize)> {
    let mut value = Vec::new();
    while value.len() < max {
        match byte(at + value.len()) {
            Some(b) if byte_ok(b) => value.push(b),
            _ => break,
        }
    }
    let end = at + value.len();
    let value = String::from_utf8(value).unwrap_or_default();
    ensure!(
        !value.is_empty() && ok(&value),
        "claim {name} is missing or malformed"
    );
    Ok((value, end))
}

fn digits_at(at: usize, byte: impl Fn(usize) -> Option<u8>) -> Result<(usize, usize)> {
    let mut end = at;
    while byte(end).is_some_and(|b| b.is_ascii_digit()) && end - at < 9 {
        end += 1;
    }
    ensure!(end > at, "missing length");
    let n = (at..end).fold(0usize, |n, i| {
        n * 10 + usize::from(byte(i).unwrap_or(b'0') - b'0')
    });
    Ok((n, end))
}

/// The verifier's match over a partly hidden transcript (`None` = hidden).
pub fn match_hidden(bytes: &[Option<u8>], segs: &[Seg]) -> Result<Found> {
    validate(segs)?;
    let byte = |i: usize| bytes.get(i).copied().flatten();
    let mut f = Found::default();
    let mut pos = 0;
    let mut body_start = None;
    for seg in segs {
        let start = pos;
        match seg {
            Seg::Lit(lit) => {
                for (k, want) in lit.iter().enumerate() {
                    match bytes.get(pos + k) {
                        Some(Some(b)) if b == want => {}
                        Some(None) => bail!("byte {} is hidden inside a literal", pos + k),
                        _ => bail!("literal does not match at byte {}", pos + k),
                    }
                }
                pos += lit.len();
                f.reveal(start..pos);
            }
            Seg::Ws => {
                while byte(pos).is_some_and(|b| b.is_ascii_whitespace()) {
                    pos += 1;
                }
                f.reveal(start..pos);
            }
            Seg::Hole { name, len } => {
                while matches!(bytes.get(pos), Some(None)) {
                    pos += 1;
                }
                ensure!(
                    len.contains(&(pos - start)),
                    "hole {name} is {} bytes, outside {len:?}",
                    pos - start
                );
                f.holes.push((name, start..pos));
            }
            Seg::Claim {
                name,
                max,
                byte_ok,
                ok,
            } => {
                let (value, end) = claim_at(pos, byte, name, *max, *byte_ok, *ok)?;
                pos = end;
                f.claims.push((name, value));
                f.reveal(start..pos);
            }
            Seg::ContentLength => {
                let (n, end) = digits_at(pos, byte)?;
                pos = end;
                f.content_length = Some((n, 0));
                f.reveal(start..pos);
            }
            Seg::BodyStart => body_start = Some(pos),
        }
    }
    f.end = pos;
    if let (Some((n, _)), Some(b)) = (f.content_length, body_start) {
        f.content_length = Some((n, b));
    }
    Ok(f)
}

/// The prover's match over plaintext it knows. A hole runs to the first
/// occurrence of the literal after it; page data is HTML-escaped, so markup
/// in that literal cannot appear inside the hole.
pub fn match_plain(bytes: &[u8], segs: &[Seg]) -> Result<Found> {
    validate(segs)?;
    let byte = |i: usize| bytes.get(i).copied();
    let mut f = Found::default();
    let mut pos = 0;
    let mut body_start = None;
    for (i, seg) in segs.iter().enumerate() {
        let start = pos;
        match seg {
            Seg::Lit(lit) => {
                ensure!(
                    bytes.get(pos..pos + lit.len()) == Some(lit.as_slice()),
                    "page or request differs from the pattern at byte {pos}"
                );
                pos += lit.len();
                f.reveal(start..pos);
            }
            Seg::Ws => {
                while byte(pos).is_some_and(|b| b.is_ascii_whitespace()) {
                    pos += 1;
                }
                f.reveal(start..pos);
            }
            Seg::Hole { name, len } => {
                let Some(Seg::Lit(next)) = segs.get(i + 1) else {
                    bail!("hole {name} is not followed by a literal");
                };
                let from = pos + len.start();
                let to = pos
                    .saturating_add(*len.end())
                    .saturating_add(next.len())
                    .min(bytes.len());
                let at = bytes
                    .get(from..to)
                    .and_then(|w| w.windows(next.len()).position(|w| w == next.as_slice()))
                    .map(|k| from + k);
                let Some(at) = at else {
                    bail!("hole {name} has no end within {len:?} bytes");
                };
                pos = at;
                f.holes.push((name, start..pos));
            }
            Seg::Claim {
                name,
                max,
                byte_ok,
                ok,
            } => {
                let (value, end) = claim_at(pos, byte, name, *max, *byte_ok, *ok)?;
                pos = end;
                f.claims.push((name, value));
                f.reveal(start..pos);
            }
            Seg::ContentLength => {
                let (n, end) = digits_at(pos, byte)?;
                pos = end;
                f.content_length = Some((n, 0));
                f.reveal(start..pos);
            }
            Seg::BodyStart => body_start = Some(pos),
        }
    }
    f.end = pos;
    if let (Some((n, _)), Some(b)) = (f.content_length, body_start) {
        f.content_length = Some((n, b));
    }
    Ok(f)
}

/// Builds a page pattern from template text: `{{hole:NAME:MIN:MAX}}` and
/// `{{claim:NAME}}` markers (claims looked up in `claims`, hole names in
/// `holes`), and each run of whitespace as `Ws`.
pub fn template(text: &str, claims: &[Seg], holes: &[&'static str]) -> Result<Vec<Seg>> {
    let mut segs = Vec::new();
    let mut lit = Vec::new();
    let flush = |lit: &mut Vec<u8>, segs: &mut Vec<Seg>| {
        if !lit.is_empty() {
            segs.push(Seg::Lit(std::mem::take(lit)));
        }
    };
    let mut rest = text;
    while let Some(c) = rest.bytes().next() {
        if let Some(after) = rest.strip_prefix("{{") {
            let Some((marker, tail)) = after.split_once("}}") else {
                bail!("unclosed marker");
            };
            flush(&mut lit, &mut segs);
            let parts: Vec<&str> = marker.split(':').collect();
            match parts.as_slice() {
                ["hole", name, min, max] => segs.push(Seg::Hole {
                    name: holes
                        .iter()
                        .copied()
                        .find(|h| h == name)
                        .ok_or_else(|| anyhow!("unknown hole {name}"))?,
                    len: min.parse()?..=max.parse()?,
                }),
                ["claim", name] => segs.push(
                    claims
                        .iter()
                        .find(|c| matches!(c, Seg::Claim { name: n, .. } if n == name))
                        .cloned()
                        .ok_or_else(|| anyhow!("unknown claim {name}"))?,
                ),
                _ => bail!("bad marker {{{{{marker}}}}}"),
            }
            rest = tail;
            continue;
        }
        if c.is_ascii_whitespace() {
            flush(&mut lit, &mut segs);
            if !matches!(segs.last(), Some(Seg::Ws)) {
                segs.push(Seg::Ws);
            }
            rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
        } else {
            // A lone `{` (not a marker) is one byte, so `n` is a char boundary.
            let n = rest
                .find(|c: char| c.is_ascii_whitespace() || c == '{')
                .unwrap_or(rest.len())
                .max(1);
            let (head, tail) = rest
                .split_at_checked(n)
                .ok_or_else(|| anyhow!("template is not split at a character"))?;
            lit.extend_from_slice(head.as_bytes());
            rest = tail;
        }
    }
    flush(&mut lit, &mut segs);
    validate(&segs)?;
    Ok(segs)
}

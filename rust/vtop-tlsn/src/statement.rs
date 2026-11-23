//! A statement: one request, the start of the page it must return, and which
//! claims must agree. `check` is the verifier's whole policy and `plan` the
//! prover's reveal; both are pure, so they are tested without TLS.

use std::{collections::BTreeMap, ops::Range};

use anyhow::{Context, Result, bail, ensure};

use crate::{
    http,
    pattern::{self, Found, Seg},
};

#[derive(Clone, Debug)]
pub struct Statement {
    /// Server name the proof must carry, and the host the verifier dials.
    pub host: String,
    /// The whole request, every byte accounted for.
    pub request: Vec<Seg>,
    /// The page body from its first byte, up to the last claim it proves.
    pub page: Vec<Seg>,
    /// Pairs (request claim, page claim) that must be equal.
    pub same: Vec<(&'static str, &'static str)>,
}

/// What the verifier learned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proven {
    pub request: BTreeMap<String, String>,
    pub page: BTreeMap<String, String>,
    /// Lengths of the page holes (all the verifier learns about them).
    pub hole_lengths: BTreeMap<String, usize>,
}

/// The prover's reveal, as transcript ranges.
#[derive(Debug, Clone)]
pub struct Plan {
    pub sent: Vec<Range<usize>>,
    pub recv: Vec<Range<usize>>,
    pub proven: Proven,
}

fn claims(f: &Found) -> BTreeMap<String, String> {
    f.claims
        .iter()
        .map(|(n, v)| ((*n).to_string(), v.clone()))
        .collect()
}

fn proven(req: &Found, page: &Found) -> Proven {
    Proven {
        request: claims(req),
        page: claims(page),
        hole_lengths: page
            .holes
            .iter()
            .map(|(n, r)| ((*n).to_string(), r.len()))
            .collect(),
    }
}

impl Statement {
    /// The request bytes for these hole and claim values.
    pub fn render(&self, values: &BTreeMap<&str, &str>) -> Result<Vec<u8>> {
        let mut head = Vec::new();
        let mut body = Vec::new();
        let mut in_body = false;
        let mut cl_at = None;
        for seg in &self.request {
            let out = if in_body { &mut body } else { &mut head };
            match seg {
                Seg::Lit(l) => out.extend_from_slice(l),
                Seg::Hole { name, .. } | Seg::Claim { name, .. } => out.extend_from_slice(
                    values
                        .get(name)
                        .with_context(|| format!("no value for {name}"))?
                        .as_bytes(),
                ),
                Seg::ContentLength => {
                    ensure!(!in_body, "Content-Length inside the body");
                    cl_at = Some(out.len());
                }
                Seg::BodyStart => in_body = true,
                Seg::Ws => bail!("whitespace in a request pattern"),
            }
        }
        if let Some(at) = cl_at {
            let n = body.len().to_string();
            head.splice(at..at, n.bytes());
        }
        head.extend_from_slice(&body);
        Ok(head)
    }

    /// The verifier's policy over the proof's transcript (`None` = hidden).
    pub fn check(&self, sent: &[Option<u8>], recv: &[Option<u8>]) -> Result<Proven> {
        let req = pattern::match_hidden(sent, &self.request).context("request")?;
        ensure!(req.end == sent.len(), "request has bytes after the pattern");
        if let Some((n, body_start)) = req.content_length {
            ensure!(
                n == sent.len() - body_start,
                "Content-Length does not match the body"
            );
        }

        let framed = http::frame(recv).context("response")?;
        ensure!(framed.status == 200, "response status is {}", framed.status);
        let body: Vec<Option<u8>> = framed
            .body
            .iter()
            .map(|&i| recv.get(i).copied().flatten())
            .collect();
        let page = pattern::match_hidden(&body, &self.page).context("page")?;

        let proven = proven(&req, &page);
        for (a, b) in &self.same {
            ensure!(
                proven.request.contains_key(*a) && proven.request.get(*a) == proven.page.get(*b),
                "request {a} and page {b} differ"
            );
        }
        Ok(proven)
    }

    /// The prover's reveal: the request minus its holes; the whole response
    /// head, the chunk framing, and the page pattern minus its holes. The
    /// rest of the body stays hidden.
    pub fn plan(&self, sent: &[u8], recv: &[u8]) -> Result<Plan> {
        let req = pattern::match_plain(sent, &self.request).context("request")?;
        ensure!(req.end == sent.len(), "request has bytes after the pattern");

        let all: Vec<Option<u8>> = recv.iter().copied().map(Some).collect();
        let framed = http::frame(&all).context("response")?;
        ensure!(framed.status == 200, "VTOP answered {}", framed.status);
        let body: Vec<u8> = framed
            .body
            .iter()
            .filter_map(|&i| recv.get(i).copied())
            .collect();
        let page = pattern::match_plain(&body, &self.page).context("page")?;

        let mut recv_reveal = framed.framing.clone();
        recv_reveal.push(0..framed.head_end);
        for r in &page.revealed {
            recv_reveal.extend(runs(framed.body.get(r.clone()).unwrap_or_default()));
        }
        let plan = Plan {
            sent: complement(
                sent.len(),
                &sorted(req.holes.iter().map(|(_, r)| r.clone())),
            ),
            recv: merge(recv_reveal),
            proven: proven(&req, &page),
        };
        // What the verifier will see must pass, or the proof is wasted.
        let sent_seen = mask(sent, &plan.sent);
        let recv_seen = mask(recv, &plan.recv);
        let seen = self.check(&sent_seen, &recv_seen).context("own check")?;
        ensure!(seen == plan.proven, "own check found other claims");
        Ok(plan)
    }
}

/// `bytes` with only `reveal` known, as the verifier sees them.
pub fn mask(bytes: &[u8], reveal: &[Range<usize>]) -> Vec<Option<u8>> {
    let mut out = vec![None; bytes.len()];
    for r in reveal {
        if let (Some(to), Some(from)) = (out.get_mut(r.clone()), bytes.get(r.clone())) {
            for (o, b) in to.iter_mut().zip(from) {
                *o = Some(*b);
            }
        }
    }
    out
}

fn sorted(it: impl Iterator<Item = Range<usize>>) -> Vec<Range<usize>> {
    let mut v: Vec<_> = it.collect();
    v.sort_by_key(|r| r.start);
    v
}

/// `0..len` minus `holes` (sorted, disjoint).
fn complement(len: usize, holes: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut pos = 0;
    for h in holes {
        if h.start > pos {
            out.push(pos..h.start);
        }
        pos = pos.max(h.end);
    }
    if pos < len {
        out.push(pos..len);
    }
    out
}

/// Contiguous runs of transcript offsets.
fn runs(offsets: &[usize]) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    for &i in offsets {
        match out.last_mut() {
            Some(r) if r.end == i => r.end += 1,
            _ => out.push(i..i + 1),
        }
    }
    out
}

fn merge(v: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let v = sorted(v.into_iter().filter(|r| !r.is_empty()));
    let mut out: Vec<Range<usize>> = Vec::new();
    for r in v {
        match out.last_mut() {
            Some(l) if r.start <= l.end => l.end = l.end.max(r.end),
            _ => out.push(r),
        }
    }
    out
}

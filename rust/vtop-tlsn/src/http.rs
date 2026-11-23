//! HTTP/1.1 response framing over a partly hidden transcript.
//!
//! The prover calls this over its plaintext (every byte `Some`) to learn what
//! to reveal; the verifier calls it over the proof's transcript. The head and
//! the chunk framing must be revealed in full, so both read the same
//! framing; only body bytes may be hidden.

use std::ops::Range;

use anyhow::{Context, Result, bail, ensure};

/// One parsed response.
#[derive(Debug)]
pub struct Framed {
    pub status: u16,
    /// End of the head (after the blank line).
    pub head_end: usize,
    /// Chunk-size lines and CRLFs: always revealed.
    pub framing: Vec<Range<usize>>,
    /// Transcript offset of each body byte, in order.
    pub body: Vec<usize>,
}

fn find(bytes: &[Option<u8>], needle: &[u8], from: usize) -> Option<usize> {
    let from = bytes.get(from..)?;
    from.windows(needle.len())
        .position(|w| w.iter().zip(needle).all(|(b, n)| *b == Some(*n)))
        .map(|i| bytes.len() - from.len() + i)
}

/// Revealed bytes as text, or `None` if any is hidden or not ASCII.
fn text(bytes: &[Option<u8>], r: Range<usize>) -> Option<String> {
    let v: Vec<u8> = bytes.get(r)?.iter().copied().collect::<Option<_>>()?;
    v.is_ascii().then(|| String::from_utf8(v).ok()).flatten()
}

/// A decimal or hex number of at most `max_digits` digits, nothing else.
fn number(s: &str, radix: u32, max_digits: usize) -> Option<usize> {
    let ok = !s.is_empty() && s.len() <= max_digits && s.chars().all(|c| c.is_digit(radix));
    ok.then(|| usize::from_str_radix(s, radix).ok()).flatten()
}

/// Frames one response that fills the whole transcript. A response that sets
/// a cookie is refused: it may carry a new session, and hiding part of a
/// header would let a hidden run swallow its neighbours.
pub fn frame(bytes: &[Option<u8>]) -> Result<Framed> {
    let blank = find(bytes, b"\r\n\r\n", 0).context("no revealed end of the response head")?;
    let head_end = blank + 4;
    let head = text(bytes, 0..head_end).context("response head is hidden or not ASCII")?;
    let mut lines = head.get(..blank).unwrap_or_default().split("\r\n");
    let status_line = lines.next().unwrap_or("");
    let status = status_line
        .strip_prefix("HTTP/1.1 ")
        .filter(|s| s.len() == 3 || s.as_bytes().get(3) == Some(&b' '))
        .and_then(|s| s.get(..3))
        .and_then(|s| number(s, 10, 3))
        .context("not an HTTP/1.1 status line")?;
    let status = u16::try_from(status)?;

    let mut headers: Vec<(String, String)> = Vec::new();
    for line in lines {
        ensure!(
            !line.contains(['\r', '\n']),
            "bare CR or LF in the response head"
        );
        let (name, value) = line.split_once(':').context("header without a colon")?;
        ensure!(
            !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
            "bad header name"
        );
        let name = name.to_ascii_lowercase();
        ensure!(name != "set-cookie", "the response sets a cookie");
        headers.push((name, value.trim().to_string()));
    }
    let header = |name: &str| -> Vec<&str> {
        headers
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
            .collect()
    };
    match header("content-encoding").as_slice() {
        [] | ["identity"] => {}
        _ => bail!("response body is encoded; it must be identity"),
    }
    let chunked = match header("transfer-encoding").as_slice() {
        [] => false,
        [te] if te.eq_ignore_ascii_case("chunked") => true,
        _ => bail!("unexpected Transfer-Encoding"),
    };
    let len = bytes.len();
    let crlf = |at: usize| bytes.get(at..at.saturating_add(2)) == Some(&[Some(b'\r'), Some(b'\n')]);
    let (mut framing, mut body) = (Vec::new(), Vec::new());
    if chunked {
        ensure!(
            header("content-length").is_empty(),
            "both chunked and Content-Length"
        );
        let mut pos = head_end;
        loop {
            let eol = find(bytes, b"\r\n", pos).context("hidden chunk-size line")?;
            let line = text(bytes, pos..eol).context("hidden chunk size")?;
            // No chunk extensions: VTOP sends none, and they only widen parsing.
            let size = number(&line, 16, 8).context("bad chunk size")?;
            framing.push(pos..eol + 2);
            pos = eol + 2;
            if size == 0 {
                ensure!(crlf(pos), "chunk trailers are not supported");
                framing.push(pos..pos + 2);
                pos += 2;
                break;
            }
            let end = pos.checked_add(size).filter(|e| e.saturating_add(2) <= len);
            let end = end.context("chunk runs past the transcript")?;
            body.extend(pos..end);
            pos = end;
            ensure!(crlf(pos), "chunk not followed by a revealed CRLF");
            framing.push(pos..pos + 2);
            pos += 2;
        }
        ensure!(
            pos == len,
            "bytes after the last chunk: not exactly one response"
        );
    } else {
        let cl = match header("content-length").as_slice() {
            [cl] => number(cl, 10, 9).context("bad Content-Length")?,
            _ => bail!("response needs one Content-Length or chunked encoding"),
        };
        ensure!(
            head_end.checked_add(cl) == Some(len),
            "body is not exactly Content-Length: not one response"
        );
        body.extend(head_end..len);
    }
    Ok(Framed {
        status,
        head_end,
        framing,
        body,
    })
}

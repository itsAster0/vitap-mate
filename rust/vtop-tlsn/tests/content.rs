//! The `/vtop/content` statement against honest and hostile transcripts, without TLS. Each
//! hostile case is a way a prover could try to claim another student.

use std::collections::BTreeMap;

use vtop_tlsn::{
    content::{Content, binding, session_id},
    fake::{ME, Page, SID, VICTIM, chunked, content_page},
    http,
    pattern::{Seg, validate},
    statement::{Statement, mask},
};

const CHALLENGE: &str = "0123456789abcdef0123456789abcdef";
const DEVICE: &str = "0123456789abcdef0123456789abcdef";

fn stmt() -> Statement {
    Content::new(binding(CHALLENGE, DEVICE))
        .statement()
        .unwrap()
}

fn request(sid: &str) -> Vec<u8> {
    let values: BTreeMap<&str, &str> = [("JSESSIONID", sid)].into();
    stmt().render(&values).unwrap()
}

fn page() -> Vec<u8> {
    chunked(&content_page(Page::Honest), 4000)
}

fn err(r: anyhow::Result<impl std::fmt::Debug>) -> String {
    format!("{:#}", r.expect_err("must be rejected"))
}

/// The request with only the session id hidden, as an honest prover shows it.
fn sent_seen(sent: &[u8], sid: &str) -> Vec<Option<u8>> {
    let at = sent
        .windows(sid.len())
        .position(|w| w == sid.as_bytes())
        .unwrap();
    sent.iter()
        .enumerate()
        .map(|(i, b)| (!(at..at + sid.len()).contains(&i)).then_some(*b))
        .collect()
}

/// The verifier's view of an honest plan, for tampering.
fn seen() -> (Vec<u8>, Vec<Option<u8>>, Vec<Option<u8>>) {
    let (sent, recv) = (request(SID), page());
    let plan = stmt().plan(&sent, &recv).unwrap();
    (
        recv.clone(),
        mask(&sent, &plan.sent),
        mask(&recv, &plan.recv),
    )
}

#[test]
fn honest_proof_reveals_only_static_markup_and_the_regno() {
    let sent = request(SID);
    let recv = page();
    let plan = stmt().plan(&sent, &recv).unwrap();
    assert_eq!(plan.proven.page["regno"], ME);
    assert!(plan.proven.request.is_empty() && plan.proven.hole_lengths.is_empty());
    let shown: Vec<u8> = mask(&sent, &plan.sent).into_iter().flatten().collect();
    assert!(!String::from_utf8_lossy(&shown).contains(SID));
    assert!(shown.starts_with(b"GET /vtop/content?n="));
    // Of the body, only the template and the regno: nothing after it.
    let rshown: Vec<u8> = mask(&recv, &plan.recv).into_iter().flatten().collect();
    let rshown = String::from_utf8_lossy(&rshown);
    assert!(rshown.contains(&format!("value=\"{ME}\" />")));
    assert!(!rshown.contains("filler") && !rshown.contains("container-fluid"));
}

#[test]
fn chunk_boundaries_anywhere_still_prove() {
    let sent = request(SID);
    for size in [1, 13, 512, 2290, 2300, 2310, 4096, 100_000] {
        let recv = chunked(&content_page(Page::Honest), size);
        let plan = stmt()
            .plan(&sent, &recv)
            .unwrap_or_else(|e| panic!("chunk {size}: {e:#}"));
        assert_eq!(plan.proven.page["regno"], ME);
    }
}

#[test]
fn a_content_length_page_proves_too() {
    let body = content_page(Page::Honest);
    let mut recv = format!("HTTP/1.1 200 \r\nContent-Length: {}\r\n\r\n", body.len()).into_bytes();
    recv.extend_from_slice(body.as_bytes());
    assert_eq!(
        stmt().plan(&request(SID), &recv).unwrap().proven.page["regno"],
        ME
    );
}

#[test]
fn rejects_the_signed_out_redirect_and_a_compressed_page() {
    let recv = b"HTTP/1.1 302 \r\nLocation: /vtop/login\r\nContent-Length: 0\r\n\r\n";
    assert!(err(stmt().plan(&request(SID), recv)).contains("302"));
    let gz = String::from_utf8(page()).unwrap().replacen(
        "Transfer-Encoding",
        "Content-Encoding: gzip\r\nTransfer-Encoding",
        1,
    );
    assert!(err(stmt().plan(&request(SID), gz.as_bytes())).contains("encoded"));
}

#[test]
fn rejects_a_planted_copy_later_in_the_page() {
    // Hide the real top, reveal the head, the framing and the plant.
    let sent = request(SID);
    let recv = chunked(&content_page(Page::Planted), 4000);
    let plant = format!("value=\"{VICTIM}\" />");
    let p = recv
        .windows(plant.len())
        .position(|w| w == plant.as_bytes())
        .unwrap();
    let head_end = recv.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    let all: Vec<Option<u8>> = recv.iter().copied().map(Some).collect();
    let mut reveal = vec![0..head_end, p - 80..p + plant.len()];
    reveal.extend(http::frame(&all).unwrap().framing);
    let e = err(stmt().check(&sent_seen(&sent, SID), &mask(&recv, &reveal)));
    assert!(e.contains("page"), "{e}");
}

#[test]
fn rejects_a_longer_session_hole() {
    // One spare byte would let the prover smuggle a CRLF and a header.
    let sid = format!("{SID}\r\nX: 1");
    let sent = request(&sid);
    let plan = stmt().plan(&request(SID), &page()).unwrap();
    let e = err(stmt().check(&sent_seen(&sent, &sid), &mask(&page(), &plan.recv)));
    assert!(e.contains("hole JSESSIONID"), "{e}");
}

#[test]
fn rejects_a_shorter_or_revealed_session_id() {
    let (_, _, rseen) = seen();
    let short = &SID[..31];
    let sent = request(short);
    assert!(err(stmt().check(&sent_seen(&sent, short), &rseen)).contains("hole JSESSIONID"));
    // Fully revealed: the hole is empty.
    let all: Vec<Option<u8>> = request(SID).into_iter().map(Some).collect();
    assert!(stmt().check(&all, &rseen).is_err());
}

#[test]
fn rejects_another_request() {
    let (_, _, rseen) = seen();
    for (from, to) in [
        ("GET /vtop/content?", "POST /vtop/content?"),
        ("GET /vtop/content?", "GET /vtop/contentX?"),
        ("Connection: close\r\n", "Connection: close\r\nX-A: b\r\n"),
        ("Host: vtop.vitap.ac.in", "Host: evil.example"),
    ] {
        let sent = String::from_utf8(request(SID))
            .unwrap()
            .replacen(from, to, 1)
            .into_bytes();
        assert!(
            stmt().check(&sent_seen(&sent, SID), &rseen).is_err(),
            "{to}"
        );
    }
    // And bytes after the request (a second, pipelined one).
    let mut sent = request(SID);
    sent.extend_from_slice(b"GET /x HTTP/1.1\r\n\r\n");
    assert!(err(stmt().check(&sent_seen(&sent, SID), &rseen)).contains("after the pattern"));
}

#[test]
fn rejects_another_devices_binding() {
    // What a relay that swapped provers produces: the verifier holds the proof
    // to the attacker's binding, the request carries the victim's.
    let (_, sseen, rseen) = seen();
    let theirs = Content::new(binding(CHALLENGE, "ffffffffffffffffffffffffffffffff"))
        .statement()
        .unwrap();
    let e = err(theirs.check(&sseen, &rseen));
    assert!(e.contains("request"), "{e}");
}

#[test]
fn rejects_a_hidden_byte_in_the_template_or_the_head() {
    let (recv, sseen, mut rseen) = seen();
    let at = recv
        .windows(13)
        .position(|w| w == b"authorizedIDX")
        .unwrap();
    rseen[at] = None;
    assert!(err(stmt().check(&sseen, &rseen)).contains("page"));

    let (recv, sseen, mut rseen) = seen();
    let at = recv.windows(12).position(|w| w == b"Content-Type").unwrap();
    rseen[at + 14] = None;
    assert!(err(stmt().check(&sseen, &rseen)).contains("head is hidden"));
}

#[test]
fn rejects_a_hidden_chunk_size_line() {
    let (recv, sseen, mut rseen) = seen();
    let head_end = recv.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    rseen[head_end] = None;
    assert!(err(stmt().check(&sseen, &rseen)).contains("chunk"));
}

#[test]
fn rejects_bytes_after_the_response() {
    let mut recv = page();
    recv.extend_from_slice(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
    assert!(err(stmt().plan(&request(SID), &recv)).contains("one response"));
}

#[test]
fn refuses_a_response_that_sets_a_cookie() {
    // It may carry a new session, and hiding part of a header would let the
    // hidden run swallow its neighbours, so neither side accepts one.
    for name in ["Set-Cookie", "set-cookie", "SET-COOKIE"] {
        let recv = String::from_utf8(page())
            .unwrap()
            .replacen(
                "\r\nTransfer",
                &format!("\r\n{name}: JSESSIONID={SID}; Path=/\r\nTransfer"),
                1,
            )
            .into_bytes();
        assert!(
            err(stmt().plan(&request(SID), &recv)).contains("sets a cookie"),
            "{name}"
        );
    }
}

#[test]
fn rejects_loose_framing() {
    let good = String::from_utf8(page()).unwrap();
    let cases = [
        // A chunk size with a sign, an extension, or one that would overflow.
        good.replacen("\r\n\r\nfa0\r\n", "\r\n\r\n+fa0\r\n", 1),
        good.replacen("\r\n\r\nfa0\r\n", "\r\n\r\nfa0;x=1\r\n", 1),
        good.replacen("\r\n\r\nfa0\r\n", "\r\n\r\nffffffffffffffff\r\n", 1),
        // A status line that runs on, and a bare LF in a header.
        good.replacen("HTTP/1.1 200 ", "HTTP/1.1 2000", 1),
        good.replacen("Content-Type: text/html", "Content-Type: text/\nhtml", 1),
    ];
    for (i, recv) in cases.iter().enumerate() {
        assert_ne!(recv, &good, "case {i} changed nothing");
        assert!(
            stmt().plan(&request(SID), recv.as_bytes()).is_err(),
            "case {i}"
        );
    }
}

#[test]
fn rejects_a_page_that_is_not_the_content_page() {
    let fake = format!("<!DOCTYPE html><input id=\"authorizedIDX\" value=\"{ME}\" />");
    assert!(err(stmt().plan(&request(SID), &chunked(&fake, 100))).contains("page"));
}

#[test]
fn validate_refuses_shapes_that_could_cut_a_claim() {
    let claim = Seg::Claim {
        name: "c",
        max: 10,
        byte_ok: |b| b.is_ascii_alphanumeric(),
        ok: |_| true,
    };
    let lit = |s: &str| Seg::Lit(s.as_bytes().to_vec());
    // Ws can match nothing, so a hidden byte after the claim would cut it.
    assert!(validate(&[claim.clone(), Seg::Ws, lit("<")]).is_err());
    assert!(validate(std::slice::from_ref(&claim)).is_err());
    assert!(validate(&[claim.clone(), lit("x")]).is_err());
    assert!(validate(&[claim.clone(), lit("<")]).is_ok());
    assert!(
        validate(&[claim.clone(), lit("<"), claim.clone(), lit(">")]).is_err(),
        "duplicate name"
    );
    assert!(
        validate(&[Seg::ContentLength, lit("\r\n")]).is_err(),
        "no BodyStart"
    );
    assert!(
        validate(&[Seg::ContentLength, lit("1"), Seg::BodyStart]).is_err(),
        "digit after"
    );
    let huge = Seg::Hole {
        name: "h",
        len: 0..=usize::MAX,
    };
    assert!(validate(&[huge, lit("x")]).is_err());
}

#[test]
fn binding_is_fixed_length_hex_and_differs_per_device_and_challenge() {
    let a = binding("c1c1c1c1c1c1c1c1", "dev-a");
    assert_eq!(a.len(), 32);
    assert!(
        a.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    assert_ne!(a, binding("c1c1c1c1c1c1c1c1", "dev-b"));
    assert_ne!(a, binding("c2c2c2c2c2c2c2c2", "dev-a"));
    assert!(Content::new("not-hex".into()).statement().is_err());
}

#[test]
fn session_id_takes_only_a_32_hex_digit_jsessionid() {
    assert_eq!(
        session_id(&format!("a=1; JSESSIONID={SID}; b=2")).unwrap(),
        SID
    );
    for bad in [
        "a=1".to_string(),
        format!("JSESSIONID={}", &SID[..31]),
        format!("JSESSIONID={SID}0"),
        format!("JSESSIONID={}\r\nX", &SID[..29]),
    ] {
        assert!(session_id(&bad).is_err(), "{bad:?}");
    }
}

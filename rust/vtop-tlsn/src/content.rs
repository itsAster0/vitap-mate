//! The statement that proves the registration number: `GET /vtop/content`, the page VTOP
//! serves after sign-in, whose `authorizedIDX` holds the session's own regno (the app reads it
//! there too). The session id is the only hidden byte of the request, and the request names no
//! student, so VTOP has nothing to echo; only static markup comes before the regno. Signed out,
//! VTOP answers 302, not this page.
//!
//! Why the request leaves no room for an attacker: every byte is fixed but the session id,
//! whose length is exact. A working request needs the real id in that hole, byte for byte, so
//! there is no spare byte to smuggle a header, a parameter or a second request into. Never
//! widen it.
//!
//! The URL carries `?n=<binding>` ([`binding`]), which VTOP ignores and does not echo. It is a
//! literal of the statement, so a proof made for one device and challenge fails for any other,
//! wherever it is relayed.

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};

use crate::{
    pattern::{self, Seg},
    statement::Statement,
};

pub const VTOP_HOST: &str = "vtop.vitap.ac.in";
pub const CONTENT_PATH: &str = "/vtop/content";
/// Tomcat's session id: 32 hex digits.
pub const SESSION_LEN: usize = 32;
/// The intermediate VTOP sometimes leaves out of its chain (vtop-core's copy).
pub const SECTIGO_DV_R36: &str = include_str!("../../vtop-core/assets/sectigo_dv_r36.pem");
/// `/vtop/content` from its first byte to the `authorizedIDX` value, from a real page: static
/// markup only, no holes.
const CONTENT_TEMPLATE: &str = include_str!("../assets/content.tmpl");

/// The fixed headers after `Host`: vtop-core's, with identity encoding (a compressed body
/// cannot be revealed in part) and `Connection: close` (one request per connection). Changing
/// one changes the statement, so the app and the engine must change together.
pub const HEADERS: &[(&str, &str)] = &[
    (
        "User-Agent",
        "Mozilla/5.0 (X11; Linux x86_64; rv:157.0) Gecko/20100101 Firefox/157.0",
    ),
    (
        "Accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    ),
    ("Accept-Language", "en-US,en;q=0.5"),
    ("Accept-Encoding", "identity"),
    ("Upgrade-Insecure-Requests", "1"),
    ("Sec-Fetch-Dest", "document"),
    ("Sec-Fetch-Mode", "navigate"),
    ("Sec-Fetch-Site", "same-origin"),
    ("Sec-Fetch-User", "?1"),
    ("Connection", "close"),
];

/// Ties a proof to one challenge and one device: 32 lower-case hex digits of SHA-256 over
/// both. The edge hands `challenge` to a signed device and tells the engine both; the prover
/// derives this with its own `device_id`, so a challenge fed to the wrong device yields a
/// binding the verifier does not expect.
pub fn binding(challenge: &str, device_id: &str) -> String {
    let h = Sha256::digest(format!("vitapmate-tlsn-v1:{challenge}:{device_id}"));
    h.iter().take(16).map(|b| format!("{b:02x}")).collect()
}

pub fn is_regno(s: &str) -> bool {
    let b = s.as_bytes();
    (b.len() == 9 || b.len() == 10)
        && b.iter().take(2).all(u8::is_ascii_digit)
        && b.iter().skip(2).take(3).all(u8::is_ascii_uppercase)
        && b.iter().skip(5).all(u8::is_ascii_digit)
}

/// The `JSESSIONID` of a `Cookie` header, when it is one the statement can carry.
pub fn session_id(cookie_header: &str) -> Result<&str> {
    let id = cookie_header
        .split(';')
        .filter_map(|c| c.trim().split_once('='))
        .find(|(n, _)| *n == "JSESSIONID")
        .map(|(_, v)| v)
        .context("no JSESSIONID cookie")?;
    ensure!(
        id.len() == SESSION_LEN && id.bytes().all(|b| b.is_ascii_hexdigit()),
        "JSESSIONID is not {SESSION_LEN} hex digits"
    );
    Ok(id)
}

fn lit(s: impl AsRef<str>) -> Seg {
    Seg::Lit(s.as_ref().as_bytes().to_vec())
}

#[derive(Clone, Debug)]
pub struct Content {
    /// [`VTOP_HOST`], or a fake VTOP's in tests.
    pub host: String,
    /// `binding(challenge, device_id)` for this proof.
    pub binding: String,
}

impl Content {
    pub fn new(binding: String) -> Self {
        Self {
            host: VTOP_HOST.into(),
            binding,
        }
    }

    pub fn statement(&self) -> Result<Statement> {
        let b = &self.binding;
        ensure!(
            b.len() == 32
                && b.bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
            "binding must be 32 lower-case hex digits"
        );
        let mut head = format!(
            "GET {CONTENT_PATH}?n={b} HTTP/1.1\r\nHost: {}\r\n",
            self.host
        );
        for (n, v) in HEADERS {
            head.push_str(&format!("{n}: {v}\r\n"));
        }
        head.push_str("Cookie: JSESSIONID=");
        let request = vec![
            lit(head),
            Seg::Hole {
                name: "JSESSIONID",
                len: SESSION_LEN..=SESSION_LEN,
            },
            lit("\r\n\r\n"),
        ];
        pattern::validate(&request)?;
        let regno = Seg::Claim {
            name: "regno",
            max: 10,
            byte_ok: |b| b.is_ascii_alphanumeric(),
            ok: is_regno,
        };
        let mut page = vec![Seg::Ws];
        page.extend(pattern::template(CONTENT_TEMPLATE, &[regno], &[])?);
        Ok(Statement {
            host: self.host.clone(),
            request,
            page,
            same: Vec::new(),
        })
    }
}

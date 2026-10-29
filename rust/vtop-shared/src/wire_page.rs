//! A page fetch over the UDP transport (`vtop-wire`), between the app and
//! `vtop-gateway`. The edge never sees these, so there is no TypeScript copy.
//!
//! Binary, because every byte costs time at 1 kbit/s: a fetch without the
//! session is about 40 bytes where JSON would be 150. The VTOP session
//! (~700 bytes of cookies) goes once; the gateway keeps it per device and
//! later requests carry only its [`session_hash`]. When the gateway does not
//! hold that session it answers [`NEED_SESSION`] and the app sends it again.

use std::collections::BTreeMap;

/// The gateway does not hold the session with this hash: send it.
pub const NEED_SESSION: u16 = 428;
/// The answer is too large for the UDP transport: use HTTPS.
pub const TOO_LARGE: u16 = 413;
/// The gateway has as many engine calls running as it allows: use HTTPS.
pub const BUSY: u16 = 429;
/// Sign-in requests are named by the edge path the phone uses over HTTPS
/// (`/v1/vtop/login`, its `/otp` and `/resend`, and `/v1/app/login`), with
/// that request's JSON body in `args[SIGN_IN_BODY]`. They carry no session.
pub const SIGN_IN_BODY: &str = "body";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WirePageRequest {
    pub name: String,
    pub semester: Option<String>,
    pub args: BTreeMap<String, String>,
    /// The page dictionary the app holds (`page_dictionary::id`).
    pub dictionary: Option<String>,
    pub session_hash: u64,
    /// The session as JSON (`vtop_core::SessionState`), when the gateway may
    /// not hold it.
    pub session: Option<String>,
    /// With [Self::session], when the phone has the server sign in for it:
    /// `/v1/app/login`'s body, so the gateway can renew an expired session
    /// in the fetch (from the account's store, which connected apps share).
    pub account: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WirePageResponse {
    /// The engine's HTTP status.
    pub status: u16,
    /// A page packed with the page dictionary; otherwise JSON (the page as
    /// `{"data": …}`, or an error as `{"code", "error"}`).
    pub packed: bool,
    pub body: Vec<u8>,
    /// The session the gateway renewed for this answer, as JSON; the phone
    /// uses it from now on.
    pub session: Option<String>,
}

/// FNV-1a over the session's cookies (the whole JSON if it has none):
/// they are what tells VTOP sessions apart, so the phone and the gateway
/// agree on a renewed session however each writes its JSON. The channel is
/// authenticated per device, so nobody else can pick a hash.
pub fn session_hash(session_json: &str) -> u64 {
    let cookies = serde_json::from_str::<serde_json::Value>(session_json)
        .ok()
        .and_then(|v| v.get("cookies")?.as_str().map(str::to_owned));
    cookies
        .as_deref()
        .unwrap_or(session_json)
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
        })
}

impl WirePageRequest {
    pub fn encode(&self) -> Option<Vec<u8>> {
        let mut v = Vec::new();
        short(&mut v, &self.name)?;
        short(&mut v, self.semester.as_deref().unwrap_or(""))?;
        v.push(u8::try_from(self.args.len()).ok()?);
        for (k, val) in &self.args {
            short(&mut v, k)?;
            short(&mut v, val)?;
        }
        short(&mut v, self.dictionary.as_deref().unwrap_or(""))?;
        v.extend_from_slice(&self.session_hash.to_be_bytes());
        long(&mut v, self.session.as_deref().unwrap_or(""))?;
        // Only when set, so requests without it are as before.
        if let Some(account) = &self.account {
            long(&mut v, account)?;
        }
        Some(v)
    }

    pub fn decode(b: &[u8]) -> Option<Self> {
        let mut r = Reader(b);
        let name = r.short()?;
        let semester = Some(r.short()?).filter(|s| !s.is_empty());
        let mut args = BTreeMap::new();
        for _ in 0..r.byte()? {
            let k = r.short()?;
            args.insert(k, r.short()?);
        }
        let dictionary = Some(r.short()?).filter(|s| !s.is_empty());
        let session_hash = u64::from_be_bytes(r.take(8)?.try_into().ok()?);
        let session = Some(r.long()?).filter(|s| !s.is_empty());
        let account = if r.0.is_empty() {
            None
        } else {
            Some(r.long()?)
        };
        r.0.is_empty().then_some(Self {
            name,
            semester,
            args,
            dictionary,
            session_hash,
            session,
            account,
        })
    }
}

impl WirePageResponse {
    /// Status, a flags byte (1: packed, 2: a renewed session follows), the
    /// session if any, then the body.
    pub fn encode(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(3 + self.body.len());
        v.extend_from_slice(&self.status.to_be_bytes());
        v.push(self.packed as u8 | (self.session.is_some() as u8) << 1);
        if let Some(session) = &self.session {
            // A session is far under 64 KB.
            let _ = long(&mut v, session);
        }
        v.extend_from_slice(&self.body);
        v
    }

    pub fn decode(b: &[u8]) -> Option<Self> {
        let mut r = Reader(b);
        let status = u16::from_be_bytes(r.take(2)?.try_into().ok()?);
        let flags = r.byte()?;
        let session = if flags & 2 != 0 {
            Some(r.long()?)
        } else {
            None
        };
        Some(Self {
            status,
            packed: flags & 1 != 0,
            body: r.0.to_vec(),
            session,
        })
    }
}

/// A string of up to 64 KB, its length in two bytes.
fn long(v: &mut Vec<u8>, s: &str) -> Option<()> {
    v.extend_from_slice(&u16::try_from(s.len()).ok()?.to_be_bytes());
    v.extend_from_slice(s.as_bytes());
    Some(())
}

fn short(v: &mut Vec<u8>, s: &str) -> Option<()> {
    v.push(u8::try_from(s.len()).ok()?);
    v.extend_from_slice(s.as_bytes());
    Some(())
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.0.len() < n {
            return None;
        }
        let (head, tail) = self.0.split_at(n);
        self.0 = tail;
        Some(head)
    }
    fn byte(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn long(&mut self) -> Option<String> {
        let n = u16::from_be_bytes(self.take(2)?.try_into().ok()?) as usize;
        String::from_utf8(self.take(n)?.to_vec()).ok()
    }

    fn short(&mut self) -> Option<String> {
        let n = self.byte()? as usize;
        String::from_utf8(self.take(n)?.to_vec()).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_round_trip_and_stay_small() {
        let mut req = WirePageRequest {
            name: "attendance".into(),
            semester: Some("AP2026275".into()),
            args: BTreeMap::new(),
            dictionary: Some("a1b2c3d4".into()),
            session_hash: session_hash("{}"),
            session: None,
            account: None,
        };
        let small = req.encode().unwrap();
        assert!(small.len() <= 48, "{} bytes", small.len());
        assert_eq!(WirePageRequest::decode(&small).unwrap(), req);
        req.args.insert("date".into(), "28/10/2026".into());
        req.session = Some(r#"{"cookies":"JSESSIONID=x"}"#.into());
        assert_eq!(
            WirePageRequest::decode(&req.encode().unwrap()).unwrap(),
            req
        );
        assert!(WirePageRequest::decode(&small[..small.len() - 1]).is_none());
        req.account = Some(r#"{"appSecret":"s"}"#.into());
        assert_eq!(
            WirePageRequest::decode(&req.encode().unwrap()).unwrap(),
            req
        );
    }

    #[test]
    fn a_session_is_known_by_its_cookies() {
        assert_eq!(
            session_hash(r#"{"cookies":"JSESSIONID=x","csrf_token":"t"}"#),
            session_hash(r#"{"registration_number":"R","cookies":"JSESSIONID=x"}"#),
        );
        assert_ne!(
            session_hash(r#"{"cookies":"JSESSIONID=x"}"#),
            session_hash(r#"{"cookies":"JSESSIONID=y"}"#),
        );
    }

    #[test]
    fn responses_round_trip() {
        let mut r = WirePageResponse {
            status: 200,
            packed: true,
            body: vec![1, 2, 3],
            session: None,
        };
        assert_eq!(WirePageResponse::decode(&r.encode()).unwrap(), r);
        r.session = Some(r#"{"cookies":"JSESSIONID=n"}"#.into());
        assert_eq!(WirePageResponse::decode(&r.encode()).unwrap(), r);
    }
}

//! `Noise_IK_25519_ChaChaPoly_BLAKE2s` (Noise revision 34), just the parts the
//! wire uses: an empty prologue, the responder's static key known in advance,
//! two messages, then the raw split keys. BLAKE2s (RFC 7693) is written out
//! here so the app's library carries one copy of each crypto crate.

use chacha20poly1305::ChaCha20Poly1305;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroize;

const PROTOCOL: &[u8] = b"Noise_IK_25519_ChaChaPoly_BLAKE2s";
const DH_LEN: usize = 32;
const TAG_LEN: usize = 16;

// --- BLAKE2s-256 (RFC 7693) ---

const IV: [u32; 8] = [
    0x6A09_E667,
    0xBB67_AE85,
    0x3C6E_F372,
    0xA54F_F53A,
    0x510E_527F,
    0x9B05_688C,
    0x1F83_D9AB,
    0x5BE0_CD19,
];

const SIGMA: [[u8; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

const BLOCK: usize = 64;
pub const HASH_LEN: usize = 32;

#[derive(Clone)]
struct Blake2s {
    h: [u32; 8],
    t: u64,
    buf: [u8; BLOCK],
    len: usize,
}

/// One G mixing step per (a, b, c, d) column, with message words (x, y).
fn mix(rows: &mut [[u32; 4]; 4], words: &mut std::slice::Iter<'_, [u32; 2]>) {
    let [a, b, c, d] = rows;
    let columns = a
        .iter_mut()
        .zip(b.iter_mut())
        .zip(c.iter_mut())
        .zip(d.iter_mut());
    // `zip` asks the columns first, so this takes exactly four word pairs.
    for ((((a, b), c), d), xy) in columns.zip(words) {
        let &[x, y] = xy;
        *a = a.wrapping_add(*b).wrapping_add(x);
        *d = (*d ^ *a).rotate_right(16);
        *c = c.wrapping_add(*d);
        *b = (*b ^ *c).rotate_right(12);
        *a = a.wrapping_add(*b).wrapping_add(y);
        *d = (*d ^ *a).rotate_right(8);
        *c = c.wrapping_add(*d);
        *b = (*b ^ *c).rotate_right(7);
    }
}

fn compress(h: &mut [u32; 8], block: &[u8; BLOCK], t: u64, last: bool) {
    let mut m = [0u32; 16];
    for (w, bytes) in m.iter_mut().zip(block.as_chunks::<4>().0) {
        *w = u32::from_le_bytes(*bytes);
    }
    let [h0, h1, h2, h3, h4, h5, h6, h7] = *h;
    let [i0, i1, i2, i3, i4, i5, i6, i7] = IV;
    let mut rows = [
        [h0, h1, h2, h3],
        [h4, h5, h6, h7],
        [i0, i1, i2, i3],
        [
            i4 ^ t as u32,
            i5 ^ (t >> 32) as u32,
            if last { !i6 } else { i6 },
            i7,
        ],
    ];
    for sigma in &SIGMA {
        let words = sigma.map(|i| m.get(usize::from(i)).copied().unwrap_or_default());
        let mut pairs = words.as_chunks::<2>().0.iter();
        mix(&mut rows, &mut pairs);
        // The diagonals as columns: (v0, v5, v10, v15), (v1, v6, v11, v12), ...
        let [_, r1, r2, r3] = &mut rows;
        r1.rotate_left(1);
        r2.rotate_left(2);
        r3.rotate_left(3);
        mix(&mut rows, &mut pairs);
        let [_, r1, r2, r3] = &mut rows;
        r1.rotate_right(1);
        r2.rotate_right(2);
        r3.rotate_right(3);
    }
    let [r0, r1, r2, r3] = rows;
    let low = r0.iter().chain(&r1);
    let high = r2.iter().chain(&r3);
    for (h, (l, r)) in h.iter_mut().zip(low.zip(high)) {
        *h ^= l ^ r;
    }
}

impl Blake2s {
    fn new() -> Self {
        let mut h = IV;
        // No key, a 32-byte digest.
        h[0] ^= 0x0101_0000 ^ HASH_LEN as u32;
        Self {
            h,
            t: 0,
            buf: [0; BLOCK],
            len: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) -> &mut Self {
        while !data.is_empty() {
            // A full buffer is compressed only once more data follows: the
            // last block must go through `finalize`.
            if self.len == BLOCK {
                self.t = self.t.wrapping_add(BLOCK as u64);
                compress(&mut self.h, &self.buf, self.t, false);
                self.len = 0;
            }
            let take = data.len().min(BLOCK - self.len);
            let Some((now, rest)) = data.split_at_checked(take) else {
                break;
            };
            if let Some(dst) = self.buf.get_mut(self.len..self.len + take) {
                dst.copy_from_slice(now);
            }
            self.len += take;
            data = rest;
        }
        self
    }

    fn finalize(&mut self) -> [u8; HASH_LEN] {
        self.t = self.t.wrapping_add(self.len as u64);
        if let Some(rest) = self.buf.get_mut(self.len..) {
            rest.fill(0);
        }
        compress(&mut self.h, &self.buf, self.t, true);
        let mut out = [0u8; HASH_LEN];
        for (bytes, w) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.h) {
            bytes.copy_from_slice(&w.to_le_bytes());
        }
        self.buf.zeroize();
        self.h.zeroize();
        out
    }
}

pub fn blake2s(parts: &[&[u8]]) -> [u8; HASH_LEN] {
    let mut b = Blake2s::new();
    for part in parts {
        b.update(part);
    }
    b.finalize()
}

/// HMAC-BLAKE2s (RFC 2104) with a 32-byte key, as Noise uses it.
pub fn hmac(key: &[u8; HASH_LEN], parts: &[&[u8]]) -> [u8; HASH_LEN] {
    let mut pad = [0u8; BLOCK];
    for (p, k) in pad.iter_mut().zip(key) {
        *p = *k;
    }
    let mut ipad = pad.map(|b| b ^ 0x36);
    let mut opad = pad.map(|b| b ^ 0x5c);
    let mut inner = Blake2s::new();
    inner.update(&ipad);
    for part in parts {
        inner.update(part);
    }
    let mut inner = inner.finalize();
    let out = blake2s(&[&opad, &inner]);
    pad.zeroize();
    ipad.zeroize();
    opad.zeroize();
    inner.zeroize();
    out
}

/// Noise's HKDF (§4.3), two outputs.
fn hkdf(ck: &[u8; HASH_LEN], ikm: &[u8]) -> ([u8; HASH_LEN], [u8; HASH_LEN]) {
    let mut temp = hmac(ck, &[ikm]);
    let one = hmac(&temp, &[&[1]]);
    let two = hmac(&temp, &[&one, &[2]]);
    temp.zeroize();
    (one, two)
}

// --- Noise state ---

/// A key and its counter nonce (4 zero bytes, then the counter little-endian).
#[derive(Clone)]
struct CipherState {
    k: Option<[u8; 32]>,
    n: u64,
}

impl CipherState {
    fn nonce(&self) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        if let Some(tail) = nonce.get_mut(4..) {
            tail.copy_from_slice(&self.n.to_le_bytes());
        }
        nonce
    }

    fn encrypt(&mut self, ad: &[u8], plain: &[u8]) -> Option<Vec<u8>> {
        let Some(k) = &self.k else {
            return Some(plain.to_vec());
        };
        let n = self.n.checked_add(1)?;
        let out = ChaCha20Poly1305::new(k.into())
            .encrypt(
                &self.nonce().into(),
                Payload {
                    msg: plain,
                    aad: ad,
                },
            )
            .ok()?;
        self.n = n;
        Some(out)
    }

    fn decrypt(&mut self, ad: &[u8], sealed: &[u8]) -> Option<Vec<u8>> {
        let Some(k) = &self.k else {
            return Some(sealed.to_vec());
        };
        let n = self.n.checked_add(1)?;
        let out = ChaCha20Poly1305::new(k.into())
            .decrypt(
                &self.nonce().into(),
                Payload {
                    msg: sealed,
                    aad: ad,
                },
            )
            .ok()?;
        self.n = n;
        Some(out)
    }
}

impl Drop for CipherState {
    fn drop(&mut self) {
        self.k.zeroize();
    }
}

#[derive(Clone)]
struct SymmetricState {
    ck: [u8; HASH_LEN],
    h: [u8; HASH_LEN],
    cs: CipherState,
}

impl SymmetricState {
    /// InitializeSymmetric, the empty prologue, then the pre-message `<- s`.
    fn new(responder_static: &[u8; 32]) -> Self {
        // The name is 33 bytes, longer than HASHLEN, so it is hashed.
        let h = blake2s(&[PROTOCOL]);
        let mut ss = Self {
            ck: h,
            h,
            cs: CipherState { k: None, n: 0 },
        };
        ss.mix_hash(&[]);
        ss.mix_hash(responder_static);
        ss
    }

    fn mix_hash(&mut self, data: &[u8]) {
        self.h = blake2s(&[&self.h, data]);
    }

    fn mix_key(&mut self, mut ikm: [u8; DH_LEN]) {
        let (ck, k) = hkdf(&self.ck, &ikm);
        ikm.zeroize();
        self.ck = ck;
        self.cs = CipherState { k: Some(k), n: 0 };
    }

    fn encrypt_and_hash(&mut self, plain: &[u8], out: &mut Vec<u8>) -> Option<()> {
        let sealed = self.cs.encrypt(&self.h, plain)?;
        self.mix_hash(&sealed);
        out.extend_from_slice(&sealed);
        Some(())
    }

    fn decrypt_and_hash(&mut self, sealed: &[u8]) -> Option<Vec<u8>> {
        let plain = self.cs.decrypt(&self.h, sealed)?;
        self.mix_hash(sealed);
        Some(plain)
    }

    /// (initiator → responder key, responder → initiator key).
    fn split(&self) -> Split {
        hkdf(&self.ck, &[])
    }
}

impl Drop for SymmetricState {
    fn drop(&mut self) {
        self.ck.zeroize();
    }
}

/// (initiator → responder, responder → initiator), as snow's raw split.
pub type Split = ([u8; 32], [u8; 32]);

fn dh(private: &StaticSecret, public: &[u8; 32]) -> [u8; DH_LEN] {
    private.diffie_hellman(&PublicKey::from(*public)).to_bytes()
}

pub fn public_key(private: &[u8; 32]) -> [u8; 32] {
    PublicKey::from(&StaticSecret::from(*private)).to_bytes()
}

/// 32 bytes from the OS random source.
pub fn random_key() -> Option<[u8; 32]> {
    let mut key = [0u8; 32];
    getrandom::fill(&mut key).ok()?;
    Some(key)
}

/// Splits `n` bytes off the front of `msg`.
fn take<'a>(msg: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
    let (head, rest) = msg.split_at_checked(n)?;
    *msg = rest;
    Some(head)
}

fn key32(bytes: &[u8]) -> Option<[u8; 32]> {
    bytes.try_into().ok()
}

/// The initiator after message 1, waiting for message 2.
#[derive(Clone)]
pub struct Initiator {
    ss: SymmetricState,
    s: StaticSecret,
    e: StaticSecret,
}

impl Initiator {
    /// Writes message 1, `-> e, es, s, ss` + payload. `ephemeral` comes from
    /// [`random_key`]; a fixed one only in tests.
    pub fn start(
        local_private: &[u8; 32],
        remote_public: &[u8; 32],
        ephemeral: [u8; 32],
        payload: &[u8],
    ) -> Option<(Self, Vec<u8>)> {
        let mut ss = SymmetricState::new(remote_public);
        let s = StaticSecret::from(*local_private);
        let e = StaticSecret::from(ephemeral);
        let mut msg = Vec::with_capacity(DH_LEN * 2 + TAG_LEN * 2 + payload.len());
        let e_pub = PublicKey::from(&e).to_bytes();
        msg.extend_from_slice(&e_pub);
        ss.mix_hash(&e_pub);
        ss.mix_key(dh(&e, remote_public));
        ss.encrypt_and_hash(PublicKey::from(&s).as_bytes(), &mut msg)?;
        ss.mix_key(dh(&s, remote_public));
        ss.encrypt_and_hash(payload, &mut msg)?;
        Some((Self { ss, s, e }, msg))
    }

    /// Reads message 2, `<- e, ee, se` + payload. Leaves `self` as it was, so a
    /// forged message 2 does not spoil the real one that follows.
    pub fn finish(&self, mut msg: &[u8]) -> Option<(Vec<u8>, Split)> {
        let mut ss = self.ss.clone();
        let re = key32(take(&mut msg, DH_LEN)?)?;
        ss.mix_hash(&re);
        ss.mix_key(dh(&self.e, &re));
        ss.mix_key(dh(&self.s, &re));
        let payload = ss.decrypt_and_hash(msg)?;
        Some((payload, ss.split()))
    }
}

/// The responder after message 1, ready to answer.
pub struct Responder {
    ss: SymmetricState,
    re: [u8; 32],
    rs: [u8; 32],
}

impl Responder {
    /// Reads message 1 under the responder's static key: (state, payload).
    pub fn read(local_private: &[u8; 32], mut msg: &[u8]) -> Option<(Self, Vec<u8>)> {
        let s = StaticSecret::from(*local_private);
        let mut ss = SymmetricState::new(PublicKey::from(&s).as_bytes());
        let re = key32(take(&mut msg, DH_LEN)?)?;
        ss.mix_hash(&re);
        ss.mix_key(dh(&s, &re));
        let rs = key32(&ss.decrypt_and_hash(take(&mut msg, DH_LEN + TAG_LEN)?)?)?;
        ss.mix_key(dh(&s, &rs));
        let payload = ss.decrypt_and_hash(msg)?;
        Some((Self { ss, re, rs }, payload))
    }

    /// The initiator's static public key.
    pub fn remote_static(&self) -> [u8; 32] {
        self.rs
    }

    /// Writes message 2, `<- e, ee, se` + payload.
    pub fn finish(mut self, ephemeral: [u8; 32], payload: &[u8]) -> Option<(Vec<u8>, Split)> {
        let e = StaticSecret::from(ephemeral);
        let mut msg = Vec::with_capacity(DH_LEN + TAG_LEN + payload.len());
        let e_pub = PublicKey::from(&e).to_bytes();
        msg.extend_from_slice(&e_pub);
        self.ss.mix_hash(&e_pub);
        self.ss.mix_key(dh(&e, &self.re));
        self.ss.mix_key(dh(&e, &self.rs));
        self.ss.encrypt_and_hash(payload, &mut msg)?;
        Some((msg, self.ss.split()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex;

    #[test]
    fn blake2s_rfc7693() {
        // RFC 7693 Appendix B.
        assert_eq!(
            hex(&blake2s(&[b"abc"])),
            "508c5e8c327c14e2e1a72ba34eeb452f37458b209ed63a294d999b4c86675982"
        );
        assert_eq!(
            hex(&blake2s(&[])),
            "69217a3079908094e11121d042354a7c1f55b6482ca1a51e1b250dfd1ed0eef9"
        );
        // Split across calls and across block boundaries.
        let data: Vec<u8> = (0..=255u8).collect();
        let whole = blake2s(&[&data]);
        assert_eq!(
            blake2s(&[&data[..63], &data[63..64], &data[64..200], &data[200..]]),
            whole
        );
        assert_eq!(blake2s(&[&data[..64], &data[64..128], &data[128..]]), whole);
    }

    #[test]
    fn blake2s_known_answers() {
        assert_eq!(
            hex(&blake2s(&[b"The quick brown fox jumps over the lazy dog"])),
            "606beeec743ccbeff6cbcdf5d5302aa855c256c29b88c8ed331ea1a6bf3c8812"
        );
        // Many blocks through the counter, fed in odd-sized pieces.
        let a = vec![b'a'; 1000];
        assert_eq!(blake2s(&[&a]), blake2s(&a.chunks(7).collect::<Vec<_>>()));
    }

    #[test]
    fn hmac_matches_definition() {
        let key = [0x0bu8; 32];
        let mut k = [0u8; 64];
        k[..32].copy_from_slice(&key);
        let ipad = k.map(|b| b ^ 0x36);
        let opad = k.map(|b| b ^ 0x5c);
        let inner = blake2s(&[&ipad, b"Hi There"]);
        assert_eq!(hmac(&key, &[b"Hi ", b"There"]), blake2s(&[&opad, &inner]));
    }

    fn snow_builder() -> snow::Builder<'static> {
        snow::Builder::new(crate::NOISE.parse().unwrap())
    }

    /// Ours as initiator against snow as responder, and the other way round,
    /// with the same message 1 bytes as `tests::known_answers`.
    #[test]
    fn interoperates_with_snow() {
        let gateway = [7u8; 32];
        let gateway_pub = public_key(&gateway);
        assert_eq!(
            hex(&gateway_pub),
            "13be4feaeaf204c7fd3358fc9c00721881d174278128227ec674f37f7fe97b6d"
        );
        let (ours, msg1) = Initiator::start(&[1; 32], &gateway_pub, [3; 32], b"vm1").unwrap();
        assert_eq!(
            hex(&msg1),
            "5dfedd3b6bd47f6fa28ee15d969d5bb0ea53774d488bdaf9df1c6e0124b3ef22ceee0eda7cefc5af950f648062adf3c29edc2a5ac7c1ab4297f72530bb40de0b35a883bc2ba2be8abed7c0881a175cdb40195db3067eb9bc601995f9c74a8e16ea3997"
        );

        let mut snow_resp = snow_builder()
            .local_private_key(&gateway)
            .unwrap()
            .fixed_ephemeral_key_for_testing_only(&[9; 32])
            .build_responder()
            .unwrap();
        let mut buf = [0u8; 256];
        let n = snow_resp.read_message(&msg1, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"vm1");
        assert_eq!(snow_resp.get_remote_static().unwrap(), public_key(&[1; 32]));
        let n = snow_resp.write_message(b"sid+reset", &mut buf).unwrap();
        let snow_msg2 = buf[..n].to_vec();
        let snow_split = snow_resp.dangerously_get_raw_split();

        // A forged message 2 first: the real one must still open after it.
        let mut forged = snow_msg2.clone();
        forged[40] ^= 1;
        assert!(ours.finish(&forged).is_none());
        assert!(ours.finish(&snow_msg2[..31]).is_none());
        let (payload, split) = ours.finish(&snow_msg2).unwrap();
        assert_eq!(payload, b"sid+reset");
        assert_eq!(split, snow_split);

        // Our responder, byte for byte the same message 2 as snow's.
        let (resp, payload) = Responder::read(&gateway, &msg1).unwrap();
        assert_eq!(payload, b"vm1");
        assert_eq!(resp.remote_static(), public_key(&[1; 32]));
        let (msg2, split) = resp.finish([9; 32], b"sid+reset").unwrap();
        assert_eq!(msg2, snow_msg2);
        assert_eq!(split, snow_split);

        // snow as initiator against our responder.
        let mut snow_init = snow_builder()
            .local_private_key(&[5; 32])
            .unwrap()
            .remote_public_key(&gateway_pub)
            .unwrap()
            .build_initiator()
            .unwrap();
        let mut big = [0u8; 1024];
        let n = snow_init.write_message(&[0xAB; 300], &mut big).unwrap();
        let (resp, payload) = Responder::read(&gateway, &big[..n]).unwrap();
        assert_eq!(payload, [0xAB; 300]);
        let (msg2, split) = resp.finish(random_key().unwrap(), b"").unwrap();
        let mut out = [0u8; 64];
        assert_eq!(snow_init.read_message(&msg2, &mut out).unwrap(), 0);
        assert_eq!(split, snow_init.dangerously_get_raw_split());
    }

    #[test]
    fn rejects_bad_message_1() {
        let gateway = [7u8; 32];
        let (_, msg1) = Initiator::start(&[1; 32], &public_key(&gateway), [3; 32], b"vm1").unwrap();
        assert!(
            Responder::read(&[8; 32], &msg1).is_none(),
            "another gateway key"
        );
        for cut in [0, 31, 32, 79, msg1.len() - 1] {
            assert!(Responder::read(&gateway, &msg1[..cut]).is_none());
        }
    }
}

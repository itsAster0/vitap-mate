//! Signed requests from a phone to the edge. Each install keeps an Ed25519
//! seed (32 bytes, base64url without padding) and sends, with every request:
//! `x-vm-device` (its id), `x-vm-time` (unix seconds) and `x-vm-sig`, the
//! signature over [`message`]. The edge checks it with WebCrypto in its
//! TypeScript copy (`backend/convex/lib/signed.ts`, `crypto.ts`), against `vectors/device_sig.json`.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

pub const DEVICE: &str = "x-vm-device";
pub const TIME: &str = "x-vm-time";
pub const SIG: &str = "x-vm-sig";

/// A new seed, base64url. It stays on the phone.
// No key is better than a weak one: without the OS random source this
// aborts. Making it fallible changes the callers in vtop-ffi and vtop-bridge.
#[allow(
    clippy::expect_used,
    reason = "no key is better than a weak one: abort without the OS random source"
)]
pub fn generate_seed() -> String {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).expect("the OS random source");
    URL_SAFE_NO_PAD.encode(seed)
}

fn signing_key(seed: &str) -> Option<SigningKey> {
    let bytes = URL_SAFE_NO_PAD.decode(seed.trim()).ok()?;
    Some(SigningKey::from_bytes(&<[u8; 32]>::try_from(bytes).ok()?))
}

/// The seed's public key (base64url), registered with the edge.
pub fn public_key(seed: &str) -> Option<String> {
    Some(URL_SAFE_NO_PAD.encode(signing_key(seed)?.verifying_key().as_bytes()))
}

/// The device id of `public` (a base64url key): `hex(sha256(key bytes))[..32]`, as the server
/// derives it, so it needs no lookup.
pub fn device_id(public: &str) -> Option<String> {
    let key = URL_SAFE_NO_PAD.decode(public.trim()).ok()?;
    let mut id = hex::encode(Sha256::digest(key));
    id.truncate(32);
    Some(id)
}

/// What is signed: `vm1\n{METHOD}\n{path with query}\n{time}\n{hex sha256(body)}`.
pub fn message(method: &str, path: &str, time: i64, body: &[u8]) -> Vec<u8> {
    format!(
        "vm1\n{}\n{path}\n{time}\n{}",
        method.to_ascii_uppercase(),
        hex::encode(Sha256::digest(body))
    )
    .into_bytes()
}

/// `x-vm-sig` for a request; None for a malformed seed.
pub fn sign(seed: &str, method: &str, path: &str, time: i64, body: &[u8]) -> Option<String> {
    let key = signing_key(seed)?;
    Some(URL_SAFE_NO_PAD.encode(key.sign(&message(method, path, time, body)).to_bytes()))
}

/// Whether `sig` is `public`'s signature of the request.
pub fn verify(public: &str, sig: &str, method: &str, path: &str, time: i64, body: &[u8]) -> bool {
    let key = URL_SAFE_NO_PAD
        .decode(public.trim())
        .ok()
        .and_then(|b| <[u8; 32]>::try_from(b).ok())
        .and_then(|b| VerifyingKey::from_bytes(&b).ok());
    let sig = URL_SAFE_NO_PAD
        .decode(sig.trim())
        .ok()
        .and_then(|b| <[u8; 64]>::try_from(b).ok())
        .map(|b| Signature::from_bytes(&b));
    match (key, sig) {
        (Some(key), Some(sig)) => key.verify(&message(method, path, time, body), &sig).is_ok(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A request signed with a fixed seed, shared with the edge's
    /// `auth.test.ts`.
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Vector {
        seed: String,
        public: String,
        method: String,
        path: String,
        time: i64,
        body: String,
        message: String,
        sig: String,
        empty_body_sha256: String,
    }

    fn vector() -> Vector {
        serde_json::from_str(include_str!("../vectors/device_sig.json")).unwrap()
    }

    #[test]
    fn signs_and_verifies() {
        let seed = generate_seed();
        let public = public_key(&seed).unwrap();
        let sig = sign(
            &seed,
            "post",
            "/v1/mail-otp/inbox?x=1",
            1_800_000_000,
            b"{}",
        )
        .unwrap();
        assert!(verify(
            &public,
            &sig,
            "POST",
            "/v1/mail-otp/inbox?x=1",
            1_800_000_000,
            b"{}"
        ));
        assert!(!verify(
            &public,
            &sig,
            "POST",
            "/v1/mail-otp/inbox?x=1",
            1_800_000_000,
            b"{ }"
        ));
        assert!(!verify(
            &public,
            &sig,
            "POST",
            "/v1/mail-otp/inbox",
            1_800_000_000,
            b"{}"
        ));
        assert!(!verify(
            &public,
            &sig,
            "POST",
            "/v1/mail-otp/inbox?x=1",
            1_800_000_001,
            b"{}"
        ));
    }

    #[test]
    fn empty_body_hash() {
        let m = String::from_utf8(message("get", "/v1/events", 1, b"")).unwrap();
        assert!(m.ends_with(&vector().empty_body_sha256));
    }

    #[test]
    fn vector_matches_the_edge() {
        let v = vector();
        let sig = sign(&v.seed, &v.method, &v.path, v.time, v.body.as_bytes()).unwrap();
        assert_eq!(public_key(&v.seed).unwrap(), v.public);
        assert_eq!(sig, v.sig);
    }

    /// Known answers pinned before the dalek 3 / sha2 0.11 bump: RFC 8032 §7.1
    /// test 1, and the edge's vector checked by `verify` and as bytes.
    #[test]
    fn known_answers() {
        let rfc_seed = URL_SAFE_NO_PAD.encode(
            hex::decode("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
                .unwrap(),
        );
        let rfc = signing_key(&rfc_seed).unwrap();
        assert_eq!(
            hex::encode(rfc.verifying_key().as_bytes()),
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        );
        assert_eq!(
            hex::encode(rfc.sign(b"").to_bytes()),
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
        );
        let v = vector();
        assert_eq!(
            message(&v.method.to_lowercase(), &v.path, v.time, v.body.as_bytes()),
            v.message.as_bytes()
        );
        let body = v.body.as_bytes();
        assert!(verify(&v.public, &v.sig, &v.method, &v.path, v.time, body));
        assert!(!verify(
            &v.public,
            &v.sig,
            &v.method,
            &v.path,
            v.time + 1,
            body
        ));
        assert_eq!(URL_SAFE_NO_PAD.decode(generate_seed()).unwrap().len(), 32);
    }

    /// The edge's `deviceIdFor`: sha256 of the raw key, not of its base64url text.
    #[test]
    fn device_id_hashes_the_key_bytes() {
        let key = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let public = URL_SAFE_NO_PAD.encode(hex::decode(key).unwrap());
        let id = device_id(&public).unwrap();
        assert_eq!(
            id,
            &hex::encode(Sha256::digest(hex::decode(key).unwrap()))[..32]
        );
        assert_eq!(device_id("not base64!"), None);
    }
}

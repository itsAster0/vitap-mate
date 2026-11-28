//! Pre-built circuits for MPC, stored brotli-compressed and unpacked on first use.
use mpz_circuits_core::Circuit;
use std::sync::{Arc, LazyLock};

fn unpack(packed: &[u8]) -> Arc<Circuit> {
    let mut raw = Vec::new();
    brotli_decompressor::BrotliDecompress(&mut &packed[..], &mut raw)
        .expect("bundled circuit unpacks");
    Arc::new(bincode::deserialize(&raw).expect("bundled circuit decodes"))
}

/// AES-128 circuit: `fn(key: [u8; 16], msg: [u8; 16]) -> [u8; 16]`.
#[cfg(feature = "aes")]
pub static AES128: LazyLock<Arc<Circuit>> =
    LazyLock::new(|| unpack(include_bytes!("../data/aes_128.bin.br")));

/// AES-128 key schedule circuit: `fn(key: [u8; 16]) -> [u8; 176]`, 11 round keys of 16 bytes.
#[cfg(feature = "aes")]
pub static AES128_KS: LazyLock<Arc<Circuit>> =
    LazyLock::new(|| unpack(include_bytes!("../data/aes_128_ks.bin.br")));

/// AES-128 post key schedule encryption circuit:
/// `fn(key_schedule: [u8; 176], msg: [u8; 16]) -> [u8; 16]`.
#[cfg(feature = "aes")]
pub static AES128_POST_KS: LazyLock<Arc<Circuit>> =
    LazyLock::new(|| unpack(include_bytes!("../data/aes_128_post_ks.bin.br")));

/// SHA-256 compression circuit: `fn(msg: [u8; 64], state: [u32; 8]) -> [u32; 8]`.
#[cfg(feature = "sha2")]
pub static SHA256_COMPRESS: LazyLock<Arc<Circuit>> =
    LazyLock::new(|| unpack(include_bytes!("../data/sha256.bin.br")));

#[cfg(all(test, feature = "aes", feature = "sha2"))]
mod tests {
    use super::*;

    #[test]
    fn every_circuit_unpacks() {
        for circuit in [&AES128, &AES128_KS, &AES128_POST_KS, &SHA256_COMPRESS] {
            assert!(!circuit.gates().is_empty());
        }
    }
}

//! The Brotli dictionary a parsed page is packed with between the engine
//! and the phone in "Fetch on the server" mode. A page is mostly field
//! names and VTOP's fixed words (`Embedded Theory`, `CAT-1`, slot names),
//! which a shared dictionary carries instead of every reply: about 30%
//! smaller than plain Brotli on the parser fixtures.
//!
//! `page_dictionary.json` holds those words and each page's shape (its
//! fields, with no values), generated from vtop-core's parser fixtures by
//! `vtop-bridge`'s `page_pack` tests. The phone names the [`id`] it holds;
//! the engine packs only for the same one, and sends plain JSON otherwise,
//! so an app with an older dictionary still reads every page.

use std::collections::BTreeMap;
use std::io;
use std::sync::OnceLock;

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The content type of a packed page: Brotli with [`for_page`] as the
/// dictionary, not `Content-Encoding`, so nothing on the way unpacks it.
pub const CONTENT_TYPE: &str = "application/vnd.vmate.page+br";

/// The dictionary file, as committed.
pub const SOURCE: &str = include_str!("page_dictionary.json");

#[derive(Deserialize)]
pub struct Source {
    pub vocabulary: Vec<String>,
    /// Page name → its shape: the fixture with strings emptied, numbers
    /// zeroed and at most two items per list.
    pub shapes: BTreeMap<String, serde_json::Value>,
}

struct Parts {
    id: String,
    vocabulary: Vec<u8>,
    shapes: BTreeMap<String, Vec<u8>>,
}

fn parts() -> &'static Parts {
    static PARTS: OnceLock<Parts> = OnceLock::new();
    PARTS.get_or_init(|| {
        let source: Source = serde_json::from_str(SOURCE).expect("page_dictionary.json is valid");
        let compact =
            |value: &serde_json::Value| serde_json::to_vec(value).expect("JSON serialises");
        Parts {
            id: hex::encode(&Sha256::digest(SOURCE.as_bytes())[..6]),
            vocabulary: serde_json::to_vec(&source.vocabulary).expect("JSON serialises"),
            shapes: source
                .shapes
                .iter()
                .map(|(name, shape)| (name.clone(), compact(shape)))
                .collect(),
        }
    })
}

/// Which dictionary this build holds: the start of its SHA-256, in hex.
pub fn id() -> &'static str {
    &parts().id
}

/// The dictionary for page `name`: the vocabulary, the other pages' shapes,
/// then its own last, where Brotli reaches it most cheaply. A page without
/// a shape gets every shape.
pub fn for_page(name: &str) -> Vec<u8> {
    let parts = parts();
    let mut dictionary = parts.vocabulary.clone();
    for (page, shape) in &parts.shapes {
        if page != name {
            dictionary.extend_from_slice(shape);
        }
    }
    if let Some(own) = parts.shapes.get(name) {
        dictionary.extend_from_slice(own);
    }
    dictionary
}

/// A packed page `name` back as its JSON bytes.
pub fn unpack(name: &str, packed: &[u8]) -> io::Result<Vec<u8>> {
    let mut json = Vec::new();
    brotli_decompressor::BrotliDecompressCustomDict(
        &mut io::Cursor::new(packed),
        &mut json,
        &mut [0; 4096],
        &mut [0; 4096],
        for_page(name),
    )?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_shape_comes_last() {
        let source: Source = serde_json::from_str(SOURCE).unwrap();
        let Some((name, shape)) = source.shapes.iter().next() else {
            return;
        };
        let dictionary = for_page(name);
        assert!(dictionary.ends_with(&serde_json::to_vec(shape).unwrap()));
        assert!(dictionary.starts_with(&serde_json::to_vec(&source.vocabulary).unwrap()));
        // The same parts in another order.
        assert_eq!(for_page("no_such_page").len(), dictionary.len());
    }

    #[test]
    fn the_id_is_twelve_hex_digits() {
        assert_eq!(id().len(), 12);
        assert!(id().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn garbage_does_not_unpack() {
        assert!(unpack("timetable", b"not brotli at all").is_err());
    }
}

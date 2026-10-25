//! Pages the engine packs with the page dictionary in "Fetch on the
//! server" mode. See `vtop_core::page_dictionary`.

use vtop_core::page_dictionary;

use crate::api::vtop::vtop_errors::VtopError;

/// Which page dictionary this app holds, sent with each server fetch.
#[flutter_rust_bridge::frb(sync)]
pub fn server_page_dictionary() -> String {
    page_dictionary::id().to_string()
}

/// A page `name` the engine packed, as its JSON. Errors are `ParseError`.
#[flutter_rust_bridge::frb(sync)]
pub fn server_page_unpack(name: String, packed: Vec<u8>) -> Result<String, VtopError> {
    let json = page_dictionary::unpack(&name, &packed)
        .map_err(|error| VtopError::ParseError(format!("packed {name}: {error}")))?;
    String::from_utf8(json).map_err(|_| VtopError::ParseError(format!("packed {name} is not text")))
}

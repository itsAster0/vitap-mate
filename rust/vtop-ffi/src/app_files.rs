//! Files the Kotlin side keeps for its own screens: the Docs index (`files/docs/index.json`) and
//! the download folder's list of what the app saved (`downloads` preferences, key `saved`).
//! Kotlin reads and writes them only through these codecs, so each has one stored shape, still
//! read from what older apps wrote, with roundtrip tests here. No I/O.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

/// The Docs index: the library's sort and every document, newest last.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct StoredDocIndex {
    /// The Kotlin `DocSort` name; empty when unknown, and the app uses its default.
    #[serde(default)]
    pub sort: String,
    /// Read one by one: a document that doesn't read is dropped, not the index.
    #[serde(default, deserialize_with = "readable_docs")]
    pub docs: Vec<StoredDoc>,
}

/// One saved document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct StoredDoc {
    pub id: String,
    pub name: String,
    /// Its file name in the download folder (the kind and type come from it).
    pub file: String,
    /// None only for a document still in app storage from before. Older apps wrote that as "",
    /// and read a missing one as "", so none is left out.
    #[serde(
        default,
        deserialize_with = "blank_is_none",
        skip_serializing_if = "Option::is_none"
    )]
    pub uri: Option<String>,
    /// The Kotlin `DocKind` name; empty or unknown, the app takes it from [`StoredDoc::file`].
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub size: i64,
    #[serde(default = "imported")]
    pub source: String,
    #[serde(default)]
    pub added_at: i64,
    #[serde(default)]
    pub last_opened_at: Option<i64>,
    #[serde(default)]
    pub as_mess_menu: bool,
    #[serde(default)]
    pub page: i32,
    #[serde(default = "whole")]
    pub zoom: f64,
}

fn imported() -> String {
    "import".into()
}

fn whole() -> f64 {
    1.0
}

fn blank_is_none<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(Option::<String>::deserialize(d)?.filter(|uri| !uri.is_empty()))
}

fn readable_docs<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<StoredDoc>, D::Error> {
    let docs = Vec::<Value>::deserialize(d)?;
    Ok(docs
        .into_iter()
        .filter_map(|doc| {
            serde_json::from_value(doc)
                .inspect_err(|error| warn!(%error, "docs index: a document dropped"))
                .ok()
        })
        .collect())
}

/// A file the app saved in the download folder, so a folder change moves it (and only it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct StoredSavedFile {
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub mime: String,
}

/// The Docs index in `raw`; None when it doesn't read at all.
#[uniffi::export]
pub fn decode_doc_index(raw: String) -> Option<StoredDocIndex> {
    serde_json::from_str(&raw)
        .inspect_err(|error| warn!(%error, "docs index unreadable"))
        .ok()
}

#[uniffi::export]
pub fn encode_doc_index(index: StoredDocIndex) -> String {
    serde_json::to_string(&index).unwrap_or_default()
}

/// The saved files in `raw`; empty when it doesn't read (older apps did the same).
#[uniffi::export]
pub fn decode_saved_files(raw: String) -> Vec<StoredSavedFile> {
    serde_json::from_str(&raw)
        .inspect_err(|error| warn!(%error, "saved files unreadable"))
        .unwrap_or_default()
}

#[uniffi::export]
pub fn encode_saved_files(files: Vec<StoredSavedFile>) -> String {
    serde_json::to_string(&files).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> StoredDoc {
        StoredDoc {
            id: "doc-1".into(),
            name: "Mess menu".into(),
            file: "Mess menu.xlsx".into(),
            uri: Some("content://media/1".into()),
            kind: "Spreadsheet".into(),
            size: 2048,
            source: "import".into(),
            added_at: 1_700_000_000_000,
            last_opened_at: None,
            as_mess_menu: true,
            page: 2,
            zoom: 1.5,
        }
    }

    #[test]
    fn doc_index_roundtrips() {
        let index = StoredDocIndex {
            sort: "RecentlyAdded".into(),
            docs: vec![doc()],
        };
        let text = r#"{"sort":"RecentlyAdded","docs":[{"id":"doc-1","name":"Mess menu","file":"Mess menu.xlsx","uri":"content://media/1","kind":"Spreadsheet","size":2048,"source":"import","addedAt":1700000000000,"lastOpenedAt":null,"asMessMenu":true,"page":2,"zoom":1.5}]}"#;
        assert_eq!(encode_doc_index(index.clone()), text);
        assert_eq!(decode_doc_index(text.into()), Some(index));
    }

    #[test]
    fn reads_what_older_apps_wrote() {
        // org.json's order and its whole-number zoom; an early index without uri, kind or source.
        let old = r#"{"docs":[{"lastOpenedAt":1700000000500,"zoom":1,"id":"doc-1","name":"Mess menu","file":"doc-1/Mess menu.xlsx","addedAt":5},{"name":"no id","file":"x.pdf"}],"sort":"Gone"}"#;
        let index = decode_doc_index(old.into()).unwrap();
        assert_eq!(index.sort, "Gone");
        assert_eq!(
            index.docs,
            [StoredDoc {
                id: "doc-1".into(),
                name: "Mess menu".into(),
                file: "doc-1/Mess menu.xlsx".into(),
                uri: None,
                kind: String::new(),
                size: 0,
                source: "import".into(),
                added_at: 5,
                last_opened_at: Some(1_700_000_000_500),
                as_mess_menu: false,
                page: 0,
                zoom: 1.0,
            }]
        );
        assert_eq!(
            decode_doc_index("{}".into()),
            Some(StoredDocIndex {
                sort: String::new(),
                docs: vec![],
            })
        );
        assert_eq!(decode_doc_index("not json".into()), None);
    }

    #[test]
    fn a_doc_still_in_app_storage_has_no_uri() {
        let old = StoredDoc {
            uri: None,
            file: "doc-1/Mess menu.xlsx".into(),
            ..doc()
        };
        let index = StoredDocIndex {
            sort: "RecentlyAdded".into(),
            docs: vec![old],
        };
        // Left out, which older apps read as "".
        let text = encode_doc_index(index.clone());
        assert!(!text.contains("uri"), "{text}");
        assert_eq!(decode_doc_index(text.clone()), Some(index.clone()));
        // What older apps wrote for one: "" or null.
        for uri in [r#""uri":"","#, r#""uri":null,"#] {
            let old = text.replace(r#""kind""#, &format!(r#"{uri}"kind""#));
            assert_eq!(decode_doc_index(old), Some(index.clone()));
        }
    }

    #[test]
    fn saved_files_roundtrip_and_read_old_lists() {
        let files = vec![StoredSavedFile {
            uri: "content://tree/a".into(),
            name: "pass.pdf".into(),
            mime: "application/pdf".into(),
        }];
        let text = r#"[{"uri":"content://tree/a","name":"pass.pdf","mime":"application/pdf"}]"#;
        assert_eq!(encode_saved_files(files.clone()), text);
        assert_eq!(decode_saved_files(text.into()), files);
        // org.json's key order, as older apps wrote it.
        let old = r#"[{"mime":"application/pdf","name":"pass.pdf","uri":"content://tree/a"}]"#;
        assert_eq!(decode_saved_files(old.into()), files);
        assert!(decode_saved_files("[]".into()).is_empty());
        assert!(decode_saved_files("garbage".into()).is_empty());
    }
}

//! Docs library index (`docs/index.json`) and download folder bookkeeping (`docs/downloads.json`).
//!
//! Keeps `index.json` in [`StoredDocIndex`]'s exact JSON shape so existing files read unchanged.
//! File I/O and SAF operations stay in Kotlin (`Docs.kt`, `Downloads.kt`); Kotlin calls Rust
//! after each SAF write or delete. Logs never include file names.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::PoisonError;

use tracing::warn;

use crate::app_files::{StoredDoc, StoredDocIndex, StoredSavedFile};
use crate::store::write_atomic;
use crate::{Core, CoreError, traced_sync};

const INDEX_FILE: &str = "docs/index.json";
const DOWNLOADS_FILE: &str = "docs/downloads.json";

/// What Docs can show a file as, from its extension; anything else is a plain [`DocKind::File`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, uniffi::Enum)]
pub enum DocKind {
    Pdf,
    Image,
    Spreadsheet,
    Text,
    File,
}

impl DocKind {
    fn as_stored(self) -> &'static str {
        match self {
            Self::Pdf => "Pdf",
            Self::Image => "Image",
            Self::Spreadsheet => "Spreadsheet",
            Self::Text => "Text",
            Self::File => "File",
        }
    }

    fn from_stored(name: &str) -> Option<Self> {
        match name {
            "Pdf" => Some(Self::Pdf),
            "Image" => Some(Self::Image),
            "Spreadsheet" => Some(Self::Spreadsheet),
            "Text" => Some(Self::Text),
            "File" => Some(Self::File),
            _ => None,
        }
    }
}

/// Extension-based kind for `file_name` (case-insensitive).
#[uniffi::export]
pub fn doc_kind_of(file_name: String) -> DocKind {
    kind_of(&file_name)
}

fn kind_of(file_name: &str) -> DocKind {
    let Some((_, ext)) = file_name.rsplit_once('.') else {
        return DocKind::File;
    };
    match ext.to_ascii_lowercase().as_str() {
        "pdf" => DocKind::Pdf,
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" => DocKind::Image,
        "xlsx" | "xlsm" | "xls" | "ods" => DocKind::Spreadsheet,
        "xml" | "html" | "htm" | "txt" | "json" | "md" | "csv" => DocKind::Text,
        _ => DocKind::File,
    }
}

/// The Flutter app's Docs sort options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, uniffi::Enum)]
pub enum DocSort {
    #[default]
    RecentlyOpened,
    RecentlyAdded,
    NameAscending,
    NameDescending,
    Largest,
    Type,
}

impl DocSort {
    fn as_stored(self) -> &'static str {
        match self {
            Self::RecentlyOpened => "RecentlyOpened",
            Self::RecentlyAdded => "RecentlyAdded",
            Self::NameAscending => "NameAscending",
            Self::NameDescending => "NameDescending",
            Self::Largest => "Largest",
            Self::Type => "Type",
        }
    }

    fn from_stored(name: &str) -> Self {
        match name {
            "RecentlyOpened" => Self::RecentlyOpened,
            "RecentlyAdded" => Self::RecentlyAdded,
            "NameAscending" => Self::NameAscending,
            "NameDescending" => Self::NameDescending,
            "Largest" => Self::Largest,
            "Type" => Self::Type,
            _ => Self::RecentlyOpened,
        }
    }
}

/// One saved document in the student's download folder.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Doc {
    pub id: String,
    pub name: String,
    pub file: String,
    /// None only for a document still in app storage from before migration.
    pub uri: Option<String>,
    pub kind: DocKind,
    pub size: i64,
    pub source: String,
    pub added_at: i64,
    pub last_opened_at: Option<i64>,
    pub as_mess_menu: bool,
    pub page: i32,
    pub zoom: f64,
}

impl From<StoredDoc> for Doc {
    fn from(stored: StoredDoc) -> Self {
        let kind = DocKind::from_stored(&stored.kind).unwrap_or_else(|| kind_of(&stored.file));
        Self {
            id: stored.id,
            name: stored.name,
            file: stored.file,
            uri: stored.uri,
            kind,
            size: stored.size,
            source: stored.source,
            added_at: stored.added_at,
            last_opened_at: stored.last_opened_at,
            as_mess_menu: stored.as_mess_menu,
            page: stored.page,
            zoom: stored.zoom,
        }
    }
}

impl From<&Doc> for StoredDoc {
    fn from(doc: &Doc) -> Self {
        Self {
            id: doc.id.clone(),
            name: doc.name.clone(),
            file: doc.file.clone(),
            uri: doc.uri.clone(),
            kind: doc.kind.as_stored().to_owned(),
            size: doc.size,
            source: doc.source.clone(),
            added_at: doc.added_at,
            last_opened_at: doc.last_opened_at,
            as_mess_menu: doc.as_mess_menu,
            page: doc.page,
            zoom: doc.zoom,
        }
    }
}

/// One file observed in the download folder by Kotlin's SAF listing, for [`Core::docs_reconcile`].
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FolderFile {
    pub uri: String,
    pub name: String,
    pub size: i64,
}

/// Result of catching up the Docs index with external folder changes.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct DocReconcile {
    pub changed: bool,
    pub docs: Vec<Doc>,
    pub removed: u32,
    pub renamed: u32,
    pub added: u32,
}

/// Filters `docs` by case-insensitive substring `query` and optional `kind`, sorted by `sort`
/// with ties broken by lowercase name then id.
#[uniffi::export]
pub fn query_docs(docs: Vec<Doc>, sort: DocSort, query: String, kind: Option<DocKind>) -> Vec<Doc> {
    query_slice(&docs, sort, &query, kind)
}

/// The newest document marked as the mess menu (first max on ties, matching Kotlin `maxByOrNull`).
#[uniffi::export]
pub fn newest_mess_menu(docs: Vec<Doc>) -> Option<Doc> {
    newest_mess_menu_slice(&docs)
}

fn query_slice(docs: &[Doc], sort: DocSort, query: &str, kind: Option<DocKind>) -> Vec<Doc> {
    let needle = query.trim().to_lowercase();
    let mut filtered: Vec<Doc> = docs
        .iter()
        .filter(|d| {
            d.name.to_lowercase().contains(&needle) && kind.is_none_or(|wanted| d.kind == wanted)
        })
        .cloned()
        .collect();
    filtered.sort_by(|a, b| compare_docs(a, b, sort));
    filtered
}

fn compare_docs(a: &Doc, b: &Doc, sort: DocSort) -> Ordering {
    let primary = match sort {
        // Never-opened documents go last.
        DocSort::RecentlyOpened => match (a.last_opened_at, b.last_opened_at) {
            (Some(ta), Some(tb)) => tb.cmp(&ta),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        },
        DocSort::RecentlyAdded => b.added_at.cmp(&a.added_at),
        DocSort::NameAscending => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        DocSort::NameDescending => b.name.to_lowercase().cmp(&a.name.to_lowercase()),
        DocSort::Largest => b.size.cmp(&a.size),
        DocSort::Type => a.kind.cmp(&b.kind),
    };
    primary
        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        .then_with(|| a.id.cmp(&b.id))
}

fn newest_mess_menu_slice(docs: &[Doc]) -> Option<Doc> {
    let mut best: Option<&Doc> = None;
    for doc in docs.iter().filter(|d| d.as_mess_menu) {
        if best.is_none_or(|b| doc.added_at > b.added_at) {
            best = Some(doc);
        }
    }
    best.cloned()
}

fn stem_of(file_name: &str) -> String {
    match file_name.rsplit_once('.') {
        Some((before, _)) if !before.is_empty() => before.to_owned(),
        _ => file_name.to_owned(),
    }
}

/// Per-core in-memory Docs index and download bookkeeping state, lazily read from disk on first use.
#[derive(Default)]
pub(crate) struct State {
    index_loaded: bool,
    sort: DocSort,
    docs: Vec<Doc>,
    reconcile_counter: u64,
    downloads_loaded: bool,
    downloads: Vec<StoredSavedFile>,
}

impl State {
    fn ensure_index(&mut self, core: &Core) {
        if self.index_loaded {
            return;
        }
        let path = core.at(INDEX_FILE);
        if path.is_file() {
            match fs::read(&path) {
                Ok(bytes) => match serde_json::from_slice::<StoredDocIndex>(&bytes) {
                    Ok(stored) => {
                        self.sort = DocSort::from_stored(&stored.sort);
                        self.docs = stored.docs.into_iter().map(Doc::from).collect();
                    }
                    Err(error) => warn!(file = INDEX_FILE, %error, "docs index unreadable"),
                },
                Err(error) => warn!(file = INDEX_FILE, %error, "docs index unreadable"),
            }
        }
        self.index_loaded = true;
    }

    fn save_index(&self, core: &Core) {
        let stored = StoredDocIndex {
            sort: self.sort.as_stored().to_owned(),
            docs: self.docs.iter().map(StoredDoc::from).collect(),
        };
        let path = core.at(INDEX_FILE);
        if let Some(parent) = path.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            warn!(file = INDEX_FILE, %error, "docs dir not created");
            return;
        }
        match serde_json::to_vec(&stored) {
            Ok(bytes) => {
                if let Err(error) = write_atomic(&path, &bytes) {
                    warn!(file = INDEX_FILE, %error, "docs index not saved");
                }
            }
            Err(error) => warn!(file = INDEX_FILE, %error, "docs index not serialized"),
        }
    }

    fn ensure_downloads(&mut self, core: &Core) {
        if self.downloads_loaded {
            return;
        }
        let path = core.at(DOWNLOADS_FILE);
        if path.is_file() {
            match fs::read(&path) {
                Ok(bytes) => match serde_json::from_slice::<Vec<StoredSavedFile>>(&bytes) {
                    Ok(files) => self.downloads = files,
                    Err(error) => warn!(file = DOWNLOADS_FILE, %error, "saved files unreadable"),
                },
                Err(error) => warn!(file = DOWNLOADS_FILE, %error, "saved files unreadable"),
            }
        }
        self.downloads_loaded = true;
    }

    fn save_downloads(&self, core: &Core) -> Result<(), CoreError> {
        let path = core.at(DOWNLOADS_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(&self.downloads).map_err(|error| CoreError::Other {
            reason: error.to_string(),
        })?;
        write_atomic(&path, &bytes)?;
        Ok(())
    }
}

#[uniffi::export]
impl Core {
    /// Every document in the Docs index, in saved order.
    pub fn docs(&self, flow: String) -> Vec<Doc> {
        traced_sync("docs", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            Ok(state.docs.clone())
        })
        .unwrap_or_default()
    }

    /// The library's saved sort option.
    pub fn docs_sort(&self, flow: String) -> DocSort {
        traced_sync("docs_sort", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            Ok(state.sort)
        })
        .unwrap_or_default()
    }

    /// Saves the library's sort option in `index.json`.
    pub fn docs_sort_set(&self, flow: String, sort: DocSort) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("docs_sort_set", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            state.sort = sort;
            state.save_index(self);
            Ok(())
        });
    }

    /// Filters and sorts the Docs index.
    pub fn docs_query(
        &self,
        flow: String,
        sort: DocSort,
        query: String,
        kind: Option<DocKind>,
    ) -> Vec<Doc> {
        traced_sync("docs_query", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            Ok(query_slice(&state.docs, sort, &query, kind))
        })
        .unwrap_or_default()
    }

    /// Adds or replaces `doc` (matching by id or non-empty uri) and persists `index.json`.
    pub fn doc_add(&self, flow: String, doc: Doc) -> Vec<Doc> {
        traced_sync("doc_add", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            state.docs.retain(|existing| {
                existing.id != doc.id && (doc.uri.is_none() || existing.uri != doc.uri)
            });
            state.docs.push(doc);
            state.save_index(self);
            Ok(state.docs.clone())
        })
        .unwrap_or_default()
    }

    /// Renames `id` to trimmed `name` (blank keeps the old name).
    pub fn doc_rename(&self, flow: String, id: String, name: String) -> Vec<Doc> {
        traced_sync("doc_rename", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            let trimmed = name.trim();
            if !trimmed.is_empty()
                && let Some(doc) = state.docs.iter_mut().find(|d| d.id == id)
            {
                doc.name = trimmed.to_owned();
                state.save_index(self);
            }
            Ok(state.docs.clone())
        })
        .unwrap_or_default()
    }

    /// Marks or unmarks `id` as the mess menu spreadsheet.
    pub fn doc_set_mess_menu(&self, flow: String, id: String, on: bool) -> Vec<Doc> {
        traced_sync("doc_set_mess_menu", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            if let Some(doc) = state.docs.iter_mut().find(|d| d.id == id) {
                doc.as_mess_menu = on;
                state.save_index(self);
            }
            Ok(state.docs.clone())
        })
        .unwrap_or_default()
    }

    /// Records `now_ms` as when `id` was last opened.
    pub fn doc_opened(&self, flow: String, id: String, now_ms: i64) -> Vec<Doc> {
        traced_sync("doc_opened", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            if let Some(doc) = state.docs.iter_mut().find(|d| d.id == id) {
                doc.last_opened_at = Some(now_ms);
                state.save_index(self);
            }
            Ok(state.docs.clone())
        })
        .unwrap_or_default()
    }

    /// Saves viewer `page` and `zoom` for `id`.
    pub fn doc_save_view(&self, flow: String, id: String, page: i32, zoom: f64) -> Vec<Doc> {
        traced_sync("doc_save_view", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            if let Some(doc) = state.docs.iter_mut().find(|d| d.id == id) {
                doc.page = page;
                doc.zoom = zoom;
                state.save_index(self);
            }
            Ok(state.docs.clone())
        })
        .unwrap_or_default()
    }

    /// Removes `id` from the index, persists `index.json`, and returns the removed [`Doc`].
    pub fn doc_remove(&self, flow: String, id: String) -> Option<Doc> {
        traced_sync("doc_remove", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            let Some(pos) = state.docs.iter().position(|d| d.id == id) else {
                return Ok(None);
            };
            let removed = state.docs.remove(pos);
            state.save_index(self);
            Ok(Some(removed))
        })
        .ok()
        .flatten()
    }

    /// The newest document in the index marked as the mess menu.
    pub fn docs_newest_mess_menu(&self, flow: String) -> Option<Doc> {
        traced_sync("docs_newest_mess_menu", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            Ok(newest_mess_menu_slice(&state.docs))
        })
        .ok()
        .flatten()
    }

    /// Remaps document URIs after a download-folder move (`old_uri -> new_uri`).
    pub fn docs_remap(&self, flow: String, moves: HashMap<String, String>) -> Vec<Doc> {
        traced_sync("docs_remap", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            for doc in &mut state.docs {
                if let Some(uri) = doc.uri.as_deref()
                    && let Some(new_uri) = moves.get(uri)
                {
                    doc.uri = Some(new_uri.clone());
                }
            }
            state.save_index(self);
            Ok(state.docs.clone())
        })
        .unwrap_or_default()
    }

    /// Replaces the entire list (used after one-time migration out of app storage).
    pub fn docs_replace_all(&self, flow: String, docs: Vec<Doc>) -> Vec<Doc> {
        traced_sync("docs_replace_all", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);
            state.docs = docs;
            state.save_index(self);
            Ok(state.docs.clone())
        })
        .unwrap_or_default()
    }

    /// Catches up with external edits in the download folder: removes deleted files, matches
    /// renames by size and kind, and adds newly appearing files with source `"folder"`.
    pub fn docs_reconcile(
        &self,
        flow: String,
        files: Vec<FolderFile>,
        now_ms: i64,
    ) -> DocReconcile {
        traced_sync("docs_reconcile", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_index(self);

            let by_uri: HashMap<&str, &FolderFile> =
                files.iter().map(|f| (f.uri.as_str(), f)).collect();
            let known: HashSet<&str> = state.docs.iter().filter_map(|d| d.uri.as_deref()).collect();
            let mut added: Vec<&FolderFile> = files
                .iter()
                .filter(|f| !known.contains(f.uri.as_str()))
                .collect();
            let mut renamed = 0u32;
            let mut removed = 0u32;
            let mut kept: Vec<Doc> = Vec::with_capacity(state.docs.len());

            for d in &state.docs {
                let Some(uri) = d.uri.as_deref() else {
                    kept.push(d.clone());
                    continue;
                };
                if let Some(file) = by_uri.get(uri) {
                    if file.name == d.file {
                        kept.push(d.clone());
                    } else {
                        let mut updated = d.clone();
                        updated.file = file.name.clone();
                        updated.name = stem_of(&file.name);
                        updated.kind = kind_of(&file.name);
                        kept.push(updated);
                    }
                } else if let Some(pos) = added
                    .iter()
                    .position(|f| f.size == d.size && kind_of(&f.name) == d.kind)
                {
                    let moved = added.remove(pos);
                    renamed += 1;
                    let mut updated = d.clone();
                    updated.uri = Some(moved.uri.clone());
                    updated.file = moved.name.clone();
                    updated.name = stem_of(&moved.name);
                    kept.push(updated);
                } else {
                    removed += 1;
                }
            }

            let mut joined: Vec<Doc> = Vec::with_capacity(added.len());
            for (i, f) in added.into_iter().enumerate() {
                let counter = state.reconcile_counter;
                state.reconcile_counter = state.reconcile_counter.saturating_add(1);
                joined.push(Doc {
                    id: format!("doc-{now_ms}-{counter}-{i}"),
                    name: stem_of(&f.name),
                    file: f.name.clone(),
                    uri: Some(f.uri.clone()),
                    kind: kind_of(&f.name),
                    size: f.size,
                    source: "folder".to_owned(),
                    added_at: now_ms,
                    last_opened_at: None,
                    as_mess_menu: false,
                    page: 0,
                    zoom: 1.0,
                });
            }

            let added_count = joined.len() as u32;
            if removed == 0 && renamed == 0 && joined.is_empty() && kept == state.docs {
                return Ok(DocReconcile {
                    changed: false,
                    docs: state.docs.clone(),
                    removed: 0,
                    renamed: 0,
                    added: 0,
                });
            }
            kept.extend(joined);
            state.docs = kept;
            state.save_index(self);
            Ok(DocReconcile {
                changed: true,
                docs: state.docs.clone(),
                removed,
                renamed,
                added: added_count,
            })
        })
        .unwrap_or_else(|_| DocReconcile {
            changed: false,
            docs: Vec::new(),
            removed: 0,
            renamed: 0,
            added: 0,
        })
    }

    /// True while `docs/downloads.json` has not been created yet (one-time import from
    /// `"downloads"` SharedPreferences needed).
    pub fn downloads_needs_import(&self, flow: String) -> bool {
        traced_sync("downloads_needs_import", &flow, || {
            Ok(!self.at(DOWNLOADS_FILE).is_file())
        })
        .unwrap_or(false)
    }

    /// Imports legacy `"downloads"` SharedPreferences `saved` list into `docs/downloads.json`.
    pub fn downloads_import(
        &self,
        flow: String,
        files: Vec<StoredSavedFile>,
    ) -> Result<(), CoreError> {
        traced_sync("downloads_import", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let prev = std::mem::replace(&mut state.downloads, files);
            if let Err(error) = state.save_downloads(self) {
                state.downloads = prev;
                return Err(error);
            }
            state.downloads_loaded = true;
            Ok(())
        })
    }

    /// Records a file this app saved into the download folder.
    pub fn downloads_saved(&self, flow: String, name: String, uri: String, mime: String) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("downloads_saved", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_downloads(self);
            state.downloads.push(StoredSavedFile { uri, name, mime });
            if let Err(error) = state.save_downloads(self) {
                warn!(file = DOWNLOADS_FILE, %error, "saved files not saved");
            }
            Ok(())
        });
    }

    /// Updates a tracked download's URI after a folder move.
    pub fn downloads_moved(&self, flow: String, old_uri: String, new_uri: String) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("downloads_moved", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_downloads(self);
            for file in &mut state.downloads {
                if file.uri == old_uri {
                    file.uri = new_uri.clone();
                }
            }
            if let Err(error) = state.save_downloads(self) {
                warn!(file = DOWNLOADS_FILE, %error, "saved files not saved");
            }
            Ok(())
        });
    }

    /// Forgets a tracked download (after deletion or when its old URI is gone).
    pub fn downloads_forget(&self, flow: String, uri: String) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("downloads_forget", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_downloads(self);
            state.downloads.retain(|f| f.uri != uri);
            if let Err(error) = state.save_downloads(self) {
                warn!(file = DOWNLOADS_FILE, %error, "saved files not saved");
            }
            Ok(())
        });
    }

    /// All tracked saved files, in saved order, for folder moves.
    pub fn downloads_saved_list(&self, flow: String) -> Vec<StoredSavedFile> {
        traced_sync("downloads_saved_list", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_downloads(self);
            Ok(state.downloads.clone())
        })
        .unwrap_or_default()
    }

    /// What the app saved, by file name → URI (the newest when a name repeats).
    pub fn downloads_saved_by_name(&self, flow: String) -> HashMap<String, String> {
        traced_sync("downloads_saved_by_name", &flow, || {
            let mut state = self
                .shared
                .docs
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            state.ensure_downloads(self);
            let mut map = HashMap::with_capacity(state.downloads.len());
            for file in &state.downloads {
                map.insert(file.name.clone(), file.uri.clone());
            }
            Ok(map)
        })
        .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests;

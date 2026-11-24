//! The change inbox: what a fetch brought that the cached copy did not have (attendance posted
//! or corrected, a course crossing 75%, marks, timetable, exams, grades, an outing decided, new
//! course files, the academic calendar), one [`Change`] each, kept newest first in
//! `changes.json` beside the cache (not under `pages/`, so clearing pages keeps it).
//! [`Core::write`] calls [`Core::record_changes`] before a page is overwritten.
//!
//! A change's title is what it is about (the course by name, never its code when the name is
//! known; "General outing"; "Academic calendar") and its body says what happened ("Attendance
//! posted · 2 classes present, now 84%").
//!
//! Nothing is recorded for a page's first copy, nor for the first write of each page after the
//! semester pick changes (that copy may be months old). Opening a page counts its kind as seen
//! ([`Core::mark_page_seen`]). Logs carry counts and kinds only.

use std::collections::BTreeMap;
use std::fs;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{Core, traced_sync};

mod diff;
mod store;

use store::trim;

pub(crate) const FILE: &str = "changes.json";
const EXAMS: &str = crate::pages::exams::EXAMS;
/// A course's lecture plans (`<class>.json`), where its files are listed.
const PLANS: &str = vtop_core::spec::COURSE_PAGE_DETAIL.file;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Enum,
)]
pub enum ChangeKind {
    Attendance,
    Marks,
    Timetable,
    Exams,
    Grades,
    Outing,
    Files,
    Calendar,
}

/// One change, for the inbox and notifications.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct Change {
    pub id: String,
    pub kind: ChangeKind,
    /// When it was found (ms since 1970).
    pub at_ms: i64,
    /// What it is about: "Digital Image Processing (Theory)", "General outing".
    pub title: String,
    /// What happened: "Attendance posted · 2 classes present, now 84%".
    pub body: String,
    /// The attendance course it is about ("CSE4007.theory"), for opening it.
    pub course_key: Option<String>,
    pub semester: String,
    /// Bad news the student should act on (below 75%, an outing rejected).
    #[serde(default)]
    pub urgent: bool,
}

/// A change before it has an id and a time.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Draft {
    pub kind: ChangeKind,
    pub title: String,
    pub body: String,
    pub course_key: Option<String>,
    pub urgent: bool,
}

/// `subject` as the title; `what` and `detail` joined as the body.
fn draft(
    kind: ChangeKind,
    subject: &str,
    what: &str,
    detail: &str,
    course_key: Option<String>,
) -> Draft {
    let body = if detail.is_empty() {
        what.to_string()
    } else {
        format!("{what} · {detail}")
    };
    Draft {
        kind,
        title: subject.trim().to_string(),
        body,
        course_key,
        urgent: false,
    }
}

/// What [`diff::detect`] needs besides the two copies.
#[derive(Debug, Default)]
pub(crate) struct Context {
    /// Course code → name, from the semester's attendance and timetable.
    pub names: BTreeMap<String, String>,
    /// Today in India, days since 1970-01-01.
    pub today: i32,
}

impl Context {
    /// `name` when VTOP gave one, else the course's name from another page, else `code`.
    fn name(&self, name: &str, code: &str) -> String {
        let (name, code) = (name.trim(), code.trim());
        if !name.is_empty() {
            return name.to_string();
        }
        self.names
            .get(code)
            .cloned()
            .unwrap_or_else(|| code.to_string())
    }
}

#[uniffi::export]
impl Core {
    /// The recorded changes, newest first.
    pub fn changes(&self, flow: String) -> Vec<Change> {
        traced_sync("changes", &flow, || Ok(self.store().changes)).unwrap_or_default()
    }

    /// How many changes came after the inbox (or their page) was last seen.
    pub fn unseen_changes(&self, flow: String) -> u32 {
        traced_sync("unseen_changes", &flow, || {
            let store = self.store();
            Ok(store.changes.iter().filter(|c| store.unseen(c)).count() as u32)
        })
        .unwrap_or(0)
    }

    /// The ids of the changes [`Self::unseen_changes`] counts.
    pub fn unseen_change_ids(&self, flow: String) -> Vec<String> {
        traced_sync("unseen_change_ids", &flow, || {
            let store = self.store();
            Ok(store
                .changes
                .iter()
                .filter(|c| store.unseen(c))
                .map(|c| c.id.clone())
                .collect())
        })
        .unwrap_or_default()
    }

    /// Every change so far counts as seen.
    pub fn mark_changes_seen(&self, flow: String) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("mark_changes_seen", &flow, || {
            self.update_store(|store| {
                store.seen_ms = store
                    .changes
                    .first()
                    .map_or(store.seen_ms, |newest| newest.at_ms.max(store.seen_ms));
            });
            Ok(())
        });
    }

    /// The student opened `page` (a feature id: "attendance", "outing", "coursepage"): its
    /// changes so far count as seen. Other pages do nothing.
    pub fn mark_page_seen(&self, flow: String, page: String) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("mark_page_seen", &flow, || {
            let Some(kind) = kind_of_page(&page) else {
                return Ok(());
            };
            self.update_store(|store| {
                if let Some(newest) = store.changes.iter().find(|c| c.kind == kind) {
                    let at = newest.at_ms;
                    let seen = store.seen_kinds.entry(kind).or_default();
                    *seen = (*seen).max(at);
                }
            });
            Ok(())
        });
    }

    /// Empties the inbox (what is compared stays primed).
    pub fn clear_changes(&self, flow: String) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("clear_changes", &flow, || {
            self.update_store(|store| store.changes.clear());
            info!("changes cleared");
            Ok(())
        });
    }

    /// The inbox file's size, for the app's storage cap.
    pub fn changes_bytes(&self, flow: String) -> u64 {
        traced_sync("changes_bytes", &flow, || {
            Ok(fs::metadata(self.at(FILE)).map_or(0, |meta| meta.len()))
        })
        .unwrap_or(0)
    }

    /// Drops the oldest changes until the file is at most `max_bytes`.
    pub fn trim_changes(&self, flow: String, max_bytes: u64) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("trim_changes", &flow, || {
            self.update_store(|store| {
                let before = store.changes.len();
                trim(store, max_bytes);
                info!(
                    dropped = before - store.changes.len(),
                    kept = store.changes.len(),
                    "changes trimmed"
                );
            });
            Ok(())
        });
    }
}

/// The kind a feature's page shows.
fn kind_of_page(page: &str) -> Option<ChangeKind> {
    Some(match page {
        "attendance" => ChangeKind::Attendance,
        "marks" => ChangeKind::Marks,
        "timetable" => ChangeKind::Timetable,
        "exams" => ChangeKind::Exams,
        "grades" => ChangeKind::Grades,
        "outing" => ChangeKind::Outing,
        "coursepage" => ChangeKind::Files,
        "calendar" => ChangeKind::Calendar,
        _ => return None,
    })
}

#[cfg(test)]
mod tests;

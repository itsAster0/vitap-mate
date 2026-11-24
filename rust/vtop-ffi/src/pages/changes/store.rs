//! The inbox file: what is primed, and recording a fetch's changes into it.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, PoisonError};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::info;
use vtop_core::types::{AttendanceData, TimetableData};

use super::diff::{Version, detect, labelled};
use super::{Change, ChangeKind, Context, Draft, EXAMS, FILE, PLANS};
use crate::pages::grades::GRADES;
use crate::pages::marks::MARKS;
use crate::pages::outing::{GENERAL, WEEKEND};
use crate::pages::records::{Attendance, today_ist_day};
use crate::{ATTENDANCE, ATTENDANCE_DETAIL, CALENDAR, Core, PAGES, SELECTED, TIMETABLE};

/// The same change again within this window is one change.
const DEDUPE_MS: i64 = 10 * 60 * 1000;
/// A cap on count besides the app's byte cap ([`Core::trim_changes`]).
const KEEP: usize = 1000;
/// The semester part of the primed key for pages kept outside a semester (outings).
const NO_SEMESTER: &str = "-";

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct Store {
    /// Newest first.
    pub(super) changes: Vec<Change>,
    /// Changes found after this were not seen yet.
    pub(super) seen_ms: i64,
    /// Per kind, when its page was last opened: its changes up to then count as seen.
    #[serde(default)]
    pub(super) seen_kinds: BTreeMap<ChangeKind, i64>,
    /// The pick the primed set is for.
    pub(super) semester: Option<String>,
    /// `<semester>/<file>` written since the pick changed: only these are compared.
    pub(super) primed: BTreeSet<String>,
    /// [`FORMAT`] once the changes read "subject" / "what · detail"; older ones are converted.
    #[serde(default)]
    pub(super) format: u8,
}

/// The title-is-the-subject format of [`Change`].
pub(super) const FORMAT: u8 = 1;

/// A change recorded before [`FORMAT`] (title "Attendance posted", body "Course: …") in the
/// current shape; one it can't read stays as it was.
pub(super) fn migrate(mut change: Change) -> Change {
    let what = change.title.trim().to_string();
    let body = change.body.trim().to_string();
    // "Exam scheduled" bodies are "<course> <exam>: <when>"; "Exam removed" "<course> <exam>".
    let exam = |rest: &str| -> Option<(String, String, String)> {
        let (head, tail) = rest.split_once(": ").unwrap_or((rest, ""));
        let (course, exam) = head.rsplit_once(' ')?;
        Some((course.to_string(), exam.to_string(), tail.to_string()))
    };
    let (subject, what, detail) = match what.as_str() {
        "Attendance corrected" => {
            let Some((code, rest)) = body.split_once(" on ") else {
                return change;
            };
            let lab = change
                .course_key
                .as_deref()
                .is_some_and(|k| k.ends_with(".lab"));
            (labelled(code, lab), what, rest.to_string())
        }
        "Exam scheduled" | "Exam schedule updated" | "Exam removed" => {
            let Some((course, paper, tail)) = exam(&body) else {
                return change;
            };
            let verb = match what.as_str() {
                "Exam scheduled" => "scheduled",
                "Exam removed" => "removed",
                _ => "updated",
            };
            (course, format!("{paper} {verb}"), tail)
        }
        "Attendance posted"
        | "Attendance below 75%"
        | "Attendance back at 75%"
        | "New marks published"
        | "Marks updated"
        | "Timetable changed"
        | "Grade published"
        | "Grade changed" => {
            let Some((course, rest)) = body.split_once(": ") else {
                return change;
            };
            let what = if what == "New marks published" {
                "New marks".to_string()
            } else {
                what
            };
            (course.to_string(), what, rest.to_string())
        }
        _ => return change,
    };
    change.urgent = change.urgent || what == "Attendance below 75%";
    change.title = subject;
    change.body = if detail.is_empty() {
        what
    } else {
        format!("{what} · {detail}")
    };
    change
}

/// `store` in the current format.
pub(super) fn upgraded(mut store: Store) -> Store {
    if store.format < FORMAT {
        store.changes = store.changes.into_iter().map(migrate).collect();
        store.format = FORMAT;
    }
    store
}

impl Store {
    pub(super) fn unseen(&self, change: &Change) -> bool {
        let kind = self.seen_kinds.get(&change.kind).copied().unwrap_or(0);
        change.at_ms > self.seen_ms.max(kind)
    }
}

impl Core {
    /// Before `file` (a path under the cache dir) is replaced by `new`: the changes against the
    /// copy there, recorded. Only the pages compared here are read; any other write costs nothing.
    pub(crate) fn record_changes(&self, file: &str, value: &dyn Any, new: &[u8]) {
        let Some(rest) = file.strip_prefix(&format!("{PAGES}/")) else {
            return;
        };
        let (semester, page) = match rest.split_once('/') {
            Some(split) => split,
            // Outings are kept outside a semester; nothing else there is compared.
            None if rest == GENERAL || rest == WEEKEND => (NO_SEMESTER, rest),
            None => return,
        };
        if !compared(page) {
            return;
        }
        let picked = self.read_kept::<String>(SELECTED);
        // Grades may come from the semester before the pick (no grades yet in the picked one).
        if page != GRADES && semester != NO_SEMESTER && picked.as_deref() != Some(semester) {
            return;
        }
        if !self.at(file).is_file() {
            // A first copy: nothing to compare, but it is primed now.
            self.update_store(|store| {
                prime(store, picked.as_deref(), semester, page);
            });
            return;
        }
        let context = Context {
            names: self.course_names(semester),
            today: today_ist_day(),
        };
        let old = Saved { core: self, file };
        let drafts = detect(page, &old, &Written { value, json: new }, &context);
        self.update_store(|store| {
            if !prime(store, picked.as_deref(), semester, page) {
                return; // First write since the pick changed: the old copy may be stale.
            }
            if drafts.is_empty() {
                return;
            }
            let at_ms = now_ms();
            let added = add(store, drafts, semester, at_ms);
            let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
            for change in store.changes.iter().take(added) {
                *kinds.entry(kind_name(change.kind)).or_default() += 1;
            }
            for (kind, n) in kinds {
                info!(kind, n, "changes recorded");
            }
        });
    }

    /// Course code → name, from `semester`'s saved attendance, then its timetable.
    fn course_names(&self, semester: &str) -> BTreeMap<String, String> {
        let mut names = BTreeMap::new();
        if semester == NO_SEMESTER {
            return names;
        }
        if let Some(data) =
            self.read_kept::<AttendanceData>(&format!("{PAGES}/{semester}/{ATTENDANCE}"))
        {
            for course in Attendance::from(data).courses {
                if !course.name.trim().is_empty() {
                    names.insert(course.code, course.name);
                }
            }
        }
        if let Some(data) =
            self.read_kept::<TimetableData>(&format!("{PAGES}/{semester}/{TIMETABLE}"))
        {
            for slot in data.slots {
                if !slot.name.trim().is_empty() {
                    names
                        .entry(slot.course_code.trim().to_string())
                        .or_insert_with(|| slot.name.trim().to_string());
                }
            }
        }
        names
    }

    pub(super) fn store(&self) -> Store {
        let _one = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        upgraded(self.read::<Store>(FILE).unwrap_or_default())
    }

    pub(super) fn update_store(&self, change: impl FnOnce(&mut Store)) {
        let _one = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut store = upgraded(self.read::<Store>(FILE).unwrap_or_default());
        change(&mut store);
        self.write_plain(FILE, &store);
    }
}

/// Whether `page` (a file in a semester's pages, or an outing file) is compared.
fn compared(page: &str) -> bool {
    matches!(
        page,
        ATTENDANCE | MARKS | TIMETABLE | EXAMS | GRADES | CALENDAR | GENERAL | WEEKEND
    ) || page.starts_with(&format!("{ATTENDANCE_DETAIL}/"))
        || page.starts_with(&format!("{PLANS}/"))
}

/// Marks `semester/page` as written; a new pick starts over. Whether it was primed before (so
/// its old copy is worth comparing).
pub(super) fn prime(store: &mut Store, picked: Option<&str>, semester: &str, page: &str) -> bool {
    if store.semester.as_deref() != picked {
        store.semester = picked.map(str::to_string);
        store.primed.clear();
    }
    !store.primed.insert(format!("{semester}/{page}"))
}

/// Adds `drafts` newest first (skipping one seen within [`DEDUPE_MS`]); how many were added.
pub(super) fn add(store: &mut Store, drafts: Vec<Draft>, semester: &str, at_ms: i64) -> usize {
    let mut fresh = Vec::new();
    for (n, d) in drafts.into_iter().enumerate() {
        let repeat = store
            .changes
            .iter()
            .take_while(|c| at_ms - c.at_ms < DEDUPE_MS)
            .any(|c| {
                c.kind == d.kind
                    && c.title == d.title
                    && c.body == d.body
                    && c.course_key == d.course_key
            });
        if !repeat {
            fresh.push(Change {
                id: format!("{at_ms}-{n}"),
                kind: d.kind,
                at_ms,
                title: d.title,
                body: d.body,
                course_key: d.course_key,
                semester: semester.to_string(),
                urgent: d.urgent,
            });
        }
    }
    let added = fresh.len();
    fresh.append(&mut store.changes);
    fresh.truncate(KEEP);
    store.changes = fresh;
    added
}

/// Drops the oldest changes until the store's JSON fits `max_bytes`.
pub(super) fn trim(store: &mut Store, max_bytes: u64) {
    let size = |store: &Store| serde_json::to_vec(store).map_or(0, |bytes| bytes.len() as u64);
    let mut total = size(store);
    while total > max_bytes && !store.changes.is_empty() {
        // Roughly how many to drop, so a big trim isn't one serialisation per change.
        let per = total / store.changes.len().max(1) as u64;
        let over = ((total - max_bytes) / per.max(1)).max(1) as usize;
        let keep = store.changes.len().saturating_sub(over);
        store.changes.truncate(keep);
        total = size(store);
    }
}

fn kind_name(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Attendance => "attendance",
        ChangeKind::Marks => "marks",
        ChangeKind::Timetable => "timetable",
        ChangeKind::Exams => "exams",
        ChangeKind::Grades => "grades",
        ChangeKind::Outing => "outing",
        ChangeKind::Files => "files",
        ChangeKind::Calendar => "calendar",
    }
}

pub(super) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

/// The copy on disk, through the decoded-page cache (a screen likely read it already).
struct Saved<'a> {
    core: &'a Core,
    file: &'a str,
}

impl Version for Saved<'_> {
    fn decode<T: DeserializeOwned + Clone + Send + Sync + 'static>(&self) -> Option<T> {
        self.core.read_kept(self.file)
    }
}

/// The copy being written: the value itself when it is the page's type, else its JSON.
struct Written<'a> {
    value: &'a dyn Any,
    json: &'a [u8],
}

impl Version for Written<'_> {
    fn decode<T: DeserializeOwned + Clone + Send + Sync + 'static>(&self) -> Option<T> {
        match self.value.downcast_ref::<T>() {
            Some(value) => Some(value.clone()),
            None => self.json.decode(),
        }
    }
}

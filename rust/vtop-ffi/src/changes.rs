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

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::sync::{Mutex, PoisonError};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::info;
use vtop_core::types::{
    AcademicCalendarData, AttendanceData, CoursePageDetail, ExamScheduleData, FullAttendanceData,
    GeneralOutingData, GradeViewData, MarksData, TimetableData, WeekendOutingData,
};

use crate::grades::GRADES;
use crate::marks::MARKS;
use crate::outing::{GENERAL, WEEKEND, format_day};
use crate::records::{Attendance, AttendanceCourse, ClassKind, epoch_day, today_ist_day};
use crate::{
    ATTENDANCE, ATTENDANCE_DETAIL, CALENDAR, Core, PAGES, SELECTED, TIMETABLE, traced_sync,
};

pub(crate) const FILE: &str = "changes.json";
const EXAMS: &str = crate::exams::EXAMS;
/// A course's lecture plans (`<class>.json`), where its files are listed.
const PLANS: &str = vtop_core::spec::COURSE_PAGE_DETAIL.file;
/// The same change again within this window is one change.
const DEDUPE_MS: i64 = 10 * 60 * 1000;
/// A cap on count besides the app's byte cap ([`Core::trim_changes`]).
const KEEP: usize = 1000;
const THRESHOLD: f64 = 75.0;
/// The semester part of the primed key for pages kept outside a semester (outings).
const NO_SEMESTER: &str = "-";

static LOCK: Mutex<()> = Mutex::new(());

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

#[derive(Debug, Default, Serialize, Deserialize)]
struct Store {
    /// Newest first.
    changes: Vec<Change>,
    /// Changes found after this were not seen yet.
    seen_ms: i64,
    /// Per kind, when its page was last opened: its changes up to then count as seen.
    #[serde(default)]
    seen_kinds: BTreeMap<ChangeKind, i64>,
    /// The pick the primed set is for.
    semester: Option<String>,
    /// `<semester>/<file>` written since the pick changed: only these are compared.
    primed: BTreeSet<String>,
    /// [`FORMAT`] once the changes read "subject" / "what · detail"; older ones are converted.
    #[serde(default)]
    format: u8,
}

/// The title-is-the-subject format of [`Change`].
const FORMAT: u8 = 1;

/// A change recorded before [`FORMAT`] (title "Attendance posted", body "Course: …") in the
/// current shape; one it can't read stays as it was.
fn migrate(mut change: Change) -> Change {
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
fn upgraded(mut store: Store) -> Store {
    if store.format < FORMAT {
        store.changes = store.changes.into_iter().map(migrate).collect();
        store.format = FORMAT;
    }
    store
}

impl Store {
    fn unseen(&self, change: &Change) -> bool {
        let kind = self.seen_kinds.get(&change.kind).copied().unwrap_or(0);
        change.at_ms > self.seen_ms.max(kind)
    }
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

/// What [`detect`] needs besides the two copies.
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

    fn store(&self) -> Store {
        let _one = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        upgraded(self.read::<Store>(FILE).unwrap_or_default())
    }

    fn update_store(&self, change: impl FnOnce(&mut Store)) {
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
fn prime(store: &mut Store, picked: Option<&str>, semester: &str, page: &str) -> bool {
    if store.semester.as_deref() != picked {
        store.semester = picked.map(str::to_string);
        store.primed.clear();
    }
    !store.primed.insert(format!("{semester}/{page}"))
}

/// Adds `drafts` newest first (skipping one seen within [`DEDUPE_MS`]); how many were added.
fn add(store: &mut Store, drafts: Vec<Draft>, semester: &str, at_ms: i64) -> usize {
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
fn trim(store: &mut Store, max_bytes: u64) {
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

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

/// One copy of a page to compare, decoded as the page's type.
pub(crate) trait Version {
    fn decode<T: DeserializeOwned + Clone + Send + Sync + 'static>(&self) -> Option<T>;
}

/// Its JSON.
impl<B: AsRef<[u8]> + ?Sized> Version for B {
    fn decode<T: DeserializeOwned + Clone + Send + Sync + 'static>(&self) -> Option<T> {
        serde_json::from_slice(self.as_ref()).ok()
    }
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

/// The changes between two copies of `page`; none when either doesn't parse.
pub(crate) fn detect(
    page: &str,
    old: &(impl Version + ?Sized),
    new: &(impl Version + ?Sized),
    context: &Context,
) -> Vec<Draft> {
    fn both<T: DeserializeOwned + Clone + Send + Sync + 'static>(
        old: &(impl Version + ?Sized),
        new: &(impl Version + ?Sized),
    ) -> Option<(T, T)> {
        Some((old.decode()?, new.decode()?))
    }
    match page {
        ATTENDANCE => both(old, new).map(|(o, n)| attendance(o, n)),
        MARKS => both(old, new).map(|(o, n)| marks(&o, &n, context)),
        TIMETABLE => both(old, new).map(|(o, n)| timetable(&o, &n, context)),
        EXAMS => both(old, new).map(|(o, n)| exams(&o, &n, context)),
        GRADES => both(old, new).map(|(o, n)| grades(&o, &n, context)),
        CALENDAR => both(old, new).map(|(o, n)| calendar(&o, &n, context.today)),
        GENERAL => both(old, new).map(|(o, n)| general_outing(&o, &n)),
        WEEKEND => both(old, new).map(|(o, n)| weekend_outing(&o, &n)),
        _ if page.starts_with(&format!("{PLANS}/")) => {
            both(old, new).map(|(o, n)| files(&o, &n, context))
        }
        _ => page
            .strip_prefix(&format!("{ATTENDANCE_DETAIL}/"))
            .and_then(|name| name.strip_suffix(".json"))
            .and_then(|key| both(old, new).map(|(o, n)| history(key, &o, &n, context))),
    }
    .unwrap_or_default()
}

fn plural(n: i32, one: &str) -> String {
    format!("{n} {one}{}", if n == 1 { "" } else { "s" })
}

/// "Digital Image Processing (Lab)": attendance is per course and type.
fn labelled(name: &str, lab: bool) -> String {
    format!("{name} ({})", if lab { "Lab" } else { "Theory" })
}

fn label(course: &AttendanceCourse) -> String {
    let name = if course.name.trim().is_empty() {
        &course.code
    } else {
        &course.name
    };
    labelled(name.trim(), course.kind == ClassKind::Lab)
}

/// Per course: classes posted (more held), and a crossing of 75% either way, from the summary.
fn attendance(old: AttendanceData, new: AttendanceData) -> Vec<Draft> {
    let before: BTreeMap<String, AttendanceCourse> = Attendance::from(old)
        .courses
        .into_iter()
        .map(|c| (c.key.clone(), c))
        .collect();
    let mut out = Vec::new();
    for course in Attendance::from(new).courses {
        let Some(was) = before.get(&course.key) else {
            continue; // A course new to the list: nothing to compare.
        };
        let lab = course.kind == ClassKind::Lab;
        let held = course.total - was.total;
        let present = (course.attended - was.attended).max(0);
        let crossed_down = was.percent >= THRESHOLD && course.percent < THRESHOLD;
        let crossed_up = was.percent < THRESHOLD && course.percent >= THRESHOLD && was.total > 0;
        if course.debarred != was.debarred {
            let what = if course.debarred {
                "Debarred"
            } else {
                "No longer debarred"
            };
            let detail = course.debar_line.clone().unwrap_or_default();
            let mut d = draft(
                ChangeKind::Attendance,
                &label(&course),
                what,
                &detail,
                Some(course.key.clone()),
            );
            d.urgent = course.debar.as_ref().is_some_and(|debar| !debar.permitted);
            out.push(d);
        }
        if held <= 0 && !crossed_down && !crossed_up {
            continue;
        }
        let unit = |n: i32| {
            if lab {
                plural(n / 2, "session")
            } else {
                plural(n, "class")
            }
        };
        let mut parts = Vec::new();
        if held > 0 {
            let absent = (held - present).max(0);
            if present > 0 {
                parts.push(format!("{} present", unit(present)));
            }
            if absent > 0 {
                parts.push(format!("{} absent", unit(absent)));
            }
            if parts.is_empty() {
                parts.push(format!("{} posted", unit(held)));
            }
        }
        parts.push(format!("now {}%", course.percent.round()));
        let what = if crossed_down {
            "Attendance below 75%"
        } else if crossed_up {
            "Attendance back at 75%"
        } else {
            "Attendance posted"
        };
        let mut d = draft(
            ChangeKind::Attendance,
            &label(&course),
            what,
            &parts.join(", "),
            Some(course.key),
        );
        d.urgent = crossed_down;
        out.push(d);
    }
    out
}

/// A posted class whose status changed (an absence turned on duty, say); new classes are
/// counted by the summary instead.
fn history(
    key: &str,
    old: &FullAttendanceData,
    new: &FullAttendanceData,
    context: &Context,
) -> Vec<Draft> {
    let posted = |status: &str| {
        let s = status.trim();
        !s.is_empty() && s != "-"
    };
    let before: BTreeMap<(String, String), &str> = old
        .records
        .iter()
        .map(|r| ((r.date.clone(), r.slot.clone()), r.status.as_str()))
        .collect();
    let (code, kind) = key.split_once('.').unwrap_or((key, "theory"));
    let subject = labelled(&context.name("", code), kind == "lab");
    new.records
        .iter()
        .filter_map(|r| {
            let was = *before.get(&(r.date.clone(), r.slot.clone()))?;
            (posted(was) && posted(&r.status) && !was.trim().eq_ignore_ascii_case(r.status.trim()))
                .then(|| {
                    let mut d = draft(
                        ChangeKind::Attendance,
                        &subject,
                        "Attendance corrected",
                        &format!(
                            "{}: {} → {}",
                            r.date.trim(),
                            was.trim().to_lowercase(),
                            r.status.trim().to_lowercase()
                        ),
                        Some(key.to_string()),
                    );
                    d.urgent = r.status.trim().eq_ignore_ascii_case("absent");
                    d
                })
        })
        .collect()
}

/// The attendance key a marks or exam row's course would have.
fn key_of(code: &str, course_type: &str) -> String {
    let lab =
        course_type.ends_with("LA") || course_type.ends_with("LO") || course_type.contains("Lab");
    format!("{}.{}", code.trim(), if lab { "lab" } else { "theory" })
}

/// The same score written two ways ("13" and "13.0") is no change.
fn same_score(a: &str, b: &str) -> bool {
    let (a, b) = (a.trim(), b.trim());
    a == b || matches!((a.parse::<f64>(), b.parse::<f64>()), (Ok(x), Ok(y)) if (x - y).abs() < 1e-9)
}

/// A mark published (a new row, or a score where there was none) or changed.
fn marks(old: &MarksData, new: &MarksData, context: &Context) -> Vec<Draft> {
    let entry = |m: &vtop_core::types::MarksRecordEach| {
        if m.serial.trim().is_empty() {
            m.markstitle.clone()
        } else {
            m.serial.clone()
        }
    };
    let before: BTreeMap<String, BTreeMap<String, &vtop_core::types::MarksRecordEach>> = old
        .records
        .iter()
        .map(|c| {
            (
                format!("{}|{}", c.coursecode, c.coursetype),
                c.marks.iter().map(|m| (entry(m), m)).collect(),
            )
        })
        .collect();
    let mut out = Vec::new();
    for course in &new.records {
        let Some(was) = before.get(&format!("{}|{}", course.coursecode, course.coursetype)) else {
            continue;
        };
        let name = context.name(&course.coursetitle, &course.coursecode);
        let key = Some(key_of(&course.coursecode, &course.coursetype));
        for mark in &course.marks {
            let score = mark.scoredmark.trim();
            if score.is_empty() {
                continue;
            }
            let (title, max) = (mark.markstitle.trim(), mark.maxmarks.trim());
            match was.get(&entry(mark)).map(|prev| prev.scoredmark.trim()) {
                None | Some("") => out.push(draft(
                    ChangeKind::Marks,
                    &name,
                    "New marks",
                    &format!("{title} {score}/{max}"),
                    key.clone(),
                )),
                Some(prev) if !same_score(prev, score) => out.push(draft(
                    ChangeKind::Marks,
                    &name,
                    "Marks updated",
                    &format!("{title} {prev} → {score}/{max}"),
                    key.clone(),
                )),
                Some(_) => {
                    // The same score now counting for more or less of the course.
                    let Some(prev) = was.get(&entry(mark)) else {
                        continue;
                    };
                    let (from, to) = (prev.weightage.trim(), mark.weightage.trim());
                    if !from.is_empty() && !to.is_empty() && !same_score(from, to) {
                        out.push(draft(
                            ChangeKind::Marks,
                            &name,
                            "Weightage changed",
                            &format!("{title} {from} → {to}"),
                            key.clone(),
                        ));
                    }
                }
            }
        }
    }
    out
}

/// Spaces collapsed and upper case, so "CB  512" and "cb 512" are one room.
fn tidy(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}

/// Per course: the slots added and removed, and a room change in place as one line.
fn timetable(old: &TimetableData, new: &TimetableData, context: &Context) -> Vec<Draft> {
    type Slot = vtop_core::types::TimetableSlot;
    // Where and when, then the room.
    let place = |s: &Slot| {
        format!(
            "{}|{}|{}|{}|{}",
            tidy(&s.day),
            tidy(&s.start_time),
            tidy(&s.end_time),
            tidy(&s.slot),
            tidy(&s.course_code)
        )
    };
    let room = |s: &Slot| tidy(&format!("{} {}", s.block, s.room_no));
    let before: BTreeMap<String, &Slot> = old.slots.iter().map(|s| (place(s), s)).collect();
    let after: BTreeMap<String, &Slot> = new.slots.iter().map(|s| (place(s), s)).collect();
    let when = |s: &Slot| format!("{} {}", s.day.trim(), s.start_time.trim());
    // Course code → (name, type, lines).
    let mut courses: BTreeMap<String, (String, String, Vec<String>)> = BTreeMap::new();
    let mut note = |s: &Slot, line: String| {
        courses
            .entry(s.course_code.trim().to_string())
            .or_insert_with(|| {
                (
                    context.name(&s.name, &s.course_code),
                    s.course_type.clone(),
                    Vec::new(),
                )
            })
            .2
            .push(line);
    };
    for (at, s) in &after {
        match before.get(at) {
            None => {
                let room = room(s);
                note(
                    s,
                    if room.is_empty() {
                        format!("+ {}", when(s))
                    } else {
                        format!("+ {} · {room}", when(s))
                    },
                );
            }
            Some(was) if room(was) != room(s) => {
                note(s, format!("{} room {} → {}", when(s), room(was), room(s)));
            }
            Some(_) => {}
        }
    }
    for (at, s) in &before {
        if !after.contains_key(at) {
            note(s, format!("− {}", when(s)));
        }
    }
    courses
        .into_iter()
        .map(|(code, (name, kind, lines))| {
            draft(
                ChangeKind::Timetable,
                &name,
                "Timetable changed",
                &lines.join(", "),
                Some(key_of(&code, &kind)),
            )
        })
        .collect()
}

/// A paper scheduled, removed, or with a new date, time, venue or seat.
fn exams(old: &ExamScheduleData, new: &ExamScheduleData, context: &Context) -> Vec<Draft> {
    type Paper<'a> = (&'a str, &'a vtop_core::types::ExamScheduleRecord);
    let papers = |data: &'_ ExamScheduleData| -> BTreeMap<String, (String, vtop_core::types::ExamScheduleRecord)> {
        data.exams
            .iter()
            .flat_map(|group| group.records.iter().map(move |r| (group.exam_type.as_str(), r)))
            .map(|(exam, r): Paper<'_>| (format!("{}|{exam}", r.course_id), (exam.to_string(), r.clone())))
            .collect()
    };
    let (before, after) = (papers(old), papers(new));
    let mut out = Vec::new();
    let name =
        |r: &vtop_core::types::ExamScheduleRecord| context.name(&r.course_name, &r.course_code);
    for (id, (exam, r)) in &after {
        let key = Some(key_of(&r.course_code, &r.course_type));
        let exam = exam.trim();
        match before.get(id) {
            None => {
                // Only what VTOP has posted ("-" or blank means not yet).
                let parts: Vec<&str> = [&r.exam_date, &r.exam_time, &r.venue]
                    .into_iter()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty() && *s != "-")
                    .collect();
                let when = if parts.is_empty() {
                    "date not posted yet".to_string()
                } else {
                    parts.join(" · ")
                };
                out.push(draft(
                    ChangeKind::Exams,
                    &name(r),
                    &format!("{exam} scheduled"),
                    &when,
                    key,
                ));
            }
            Some((_, was)) => {
                let mut what = Vec::new();
                let unposted = |s: &str| matches!(s.trim(), "" | "-");
                if was.exam_date.trim() != r.exam_date.trim() {
                    what.push(if unposted(&was.exam_date) {
                        format!("date {}", r.exam_date.trim())
                    } else {
                        format!("date {} → {}", was.exam_date.trim(), r.exam_date.trim())
                    });
                }
                if was.exam_time.trim() != r.exam_time.trim() {
                    what.push(format!("time {}", r.exam_time.trim()));
                }
                if was.reporting_time.trim() != r.reporting_time.trim() {
                    what.push(format!("reporting {}", r.reporting_time.trim()));
                }
                if was.venue.trim() != r.venue.trim() {
                    what.push(format!("venue {}", r.venue.trim()));
                }
                if was.seat_location.trim() != r.seat_location.trim()
                    || was.seat_no.trim() != r.seat_no.trim()
                {
                    what.push(
                        format!("seat {} {}", r.seat_location.trim(), r.seat_no.trim())
                            .trim()
                            .to_string(),
                    );
                }
                if !what.is_empty() {
                    out.push(draft(
                        ChangeKind::Exams,
                        &name(r),
                        &format!("{exam} updated"),
                        &what.join(", "),
                        key,
                    ));
                }
            }
        }
    }
    for (id, (exam, r)) in &before {
        if !after.contains_key(id) {
            out.push(draft(
                ChangeKind::Exams,
                &name(r),
                &format!("{} removed", exam.trim()),
                "",
                Some(key_of(&r.course_code, &r.course_type)),
            ));
        }
    }
    out
}

/// A grade published (one where there was none) or changed.
fn grades(old: &GradeViewData, new: &GradeViewData, context: &Context) -> Vec<Draft> {
    let before: BTreeMap<&str, &str> = old
        .courses
        .iter()
        .map(|c| (c.course_id.as_str(), c.grade.trim()))
        .collect();
    new.courses
        .iter()
        .filter_map(|c| {
            let grade = c.grade.trim();
            if grade.is_empty() || grade == "-" {
                return None;
            }
            let name = context.name(&c.course_title, &c.course_code);
            let key = Some(key_of(&c.course_code, &c.course_type));
            let mut d = match before.get(c.course_id.as_str()).copied() {
                Some(was) if was == grade => return None,
                Some(was) if !was.is_empty() && was != "-" => draft(
                    ChangeKind::Grades,
                    &name,
                    "Grade changed",
                    &format!("{was} → {grade}"),
                    key,
                ),
                // A course new to the list with a grade, or a grade where there was none.
                _ => draft(ChangeKind::Grades, &name, "Grade published", grade, key),
            };
            d.urgent = grade.eq_ignore_ascii_case("F") || grade.eq_ignore_ascii_case("N");
            Some(d)
        })
        .collect()
}

/// "22 Nov" for a `YYYY-MM-DD`, else the text as VTOP gave it.
fn day_label(date: &str) -> String {
    epoch_day(date).map_or_else(
        || date.trim().to_string(),
        |day| format_day(day, "EEE d MMM"),
    )
}

/// A request VTOP decided or changed (pending to accepted, say). New requests are the student's
/// own and old ones dropping off the list are no news.
fn outing_status(
    subject: &str,
    before: &BTreeMap<String, String>,
    now: &[(String, String, String)],
) -> Vec<Draft> {
    now.iter()
        .filter_map(|(serial, status, about)| {
            let was = before.get(serial)?;
            (!was.trim().eq_ignore_ascii_case(status.trim())).then(|| {
                let status = status.trim();
                let mut d = draft(ChangeKind::Outing, subject, status, about, None);
                let lower = status.to_lowercase();
                d.urgent = lower.contains("reject") || lower.contains("cancel");
                d
            })
        })
        .collect()
}

fn general_outing(old: &GeneralOutingData, new: &GeneralOutingData) -> Vec<Draft> {
    let before = old
        .records
        .iter()
        .map(|r| (r.serial.clone(), r.status.clone()))
        .collect();
    let now: Vec<_> = new
        .records
        .iter()
        .map(|r| {
            let about = format!("{}, {}", r.place.trim(), day_label(&r.from_date));
            (r.serial.clone(), r.status.clone(), about)
        })
        .collect();
    outing_status("General outing", &before, &now)
}

fn weekend_outing(old: &WeekendOutingData, new: &WeekendOutingData) -> Vec<Draft> {
    let before = old
        .records
        .iter()
        .map(|r| (r.serial.clone(), r.status.clone()))
        .collect();
    let now: Vec<_> = new
        .records
        .iter()
        .map(|r| {
            let about = format!("{}, {}", r.place.trim(), day_label(&r.date));
            (r.serial.clone(), r.status.clone(), about)
        })
        .collect();
    outing_status("Weekend outing", &before, &now)
}

/// Files the course plan lists now that it did not before (by download path).
fn files(old: &CoursePageDetail, new: &CoursePageDetail, context: &Context) -> Vec<Draft> {
    let paths = |d: &CoursePageDetail| -> BTreeSet<String> {
        d.lectures
            .iter()
            .flat_map(|l| l.materials.iter().map(|m| m.path.clone()))
            .collect()
    };
    let before = paths(old);
    let mut seen = BTreeSet::new();
    let added: Vec<String> = new
        .lectures
        .iter()
        .flat_map(|l| {
            l.materials.iter().map(move |m| {
                let label = m.label.trim();
                let topic = l.topic.trim();
                let shown = if !topic.is_empty() { topic } else { label };
                (m.path.clone(), shown.to_string())
            })
        })
        .filter(|(path, _)| !before.contains(path) && seen.insert(path.clone()))
        .map(|(_, shown)| shown)
        .collect();
    if added.is_empty() {
        return Vec::new();
    }
    let class = &new.class;
    let what = if added.len() == 1 {
        "New file".to_string()
    } else {
        format!("{} new files", added.len())
    };
    let mut shown: Vec<String> = added.iter().take(3).cloned().collect();
    if added.len() > 3 {
        shown.push("…".into());
    }
    vec![draft(
        ChangeKind::Files,
        &context.name(&class.course_title, &class.course_code),
        &what,
        &shown.join(", "),
        Some(key_of(&class.course_code, &class.course_type)),
    )]
}

/// Days from today on whose entries changed: one change listing them.
fn calendar(old: &AcademicCalendarData, new: &AcademicCalendarData, today: i32) -> Vec<Draft> {
    let days = |data: &AcademicCalendarData| -> BTreeMap<i32, String> {
        let mut out: BTreeMap<i32, Vec<String>> = BTreeMap::new();
        for e in &data.entries {
            let Some(day) = epoch_day(&e.date) else {
                continue;
            };
            if day < today {
                continue;
            }
            let note = e.note.trim();
            let text = if note.is_empty() {
                e.kind.trim().to_string()
            } else {
                format!("{} ({note})", e.kind.trim())
            };
            let list = out.entry(day).or_default();
            if !list.contains(&text) {
                list.push(text);
            }
        }
        out.into_iter()
            .map(|(day, mut list)| {
                list.sort();
                (day, list.join(", "))
            })
            .collect()
    };
    let (before, after) = (days(old), days(new));
    let mut lines = Vec::new();
    for (day, text) in &after {
        let when = format_day(*day, "EEE d MMM");
        match before.get(day) {
            None => lines.push(format!("{when}: {text}")),
            Some(was) if was != text => lines.push(format!("{when}: {was} → {text}")),
            Some(_) => {}
        }
    }
    for (day, was) in &before {
        if !after.contains_key(day) {
            lines.push(format!("{}: {was} removed", format_day(*day, "EEE d MMM")));
        }
    }
    if lines.is_empty() {
        return Vec::new();
    }
    let what = if lines.len() == 1 {
        "Day changed".to_string()
    } else {
        format!("{} days changed", lines.len())
    };
    vec![draft(
        ChangeKind::Calendar,
        "Academic calendar",
        &what,
        &lines.join("; "),
        None,
    )]
}

#[cfg(test)]
mod tests;

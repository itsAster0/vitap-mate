//! The change inbox: what a fetch brought that the cached copy did not have (attendance posted
//! or corrected, a course crossing 75%, marks, timetable, exams, grades), one [`Change`] each,
//! kept newest first in `changes.json` beside the cache (not under `pages/`, so clearing pages
//! keeps it). [`Core::write`] calls [`Core::record_changes`] before a page is overwritten.
//!
//! Nothing is recorded for a page's first copy, nor for the first write of each page after the
//! semester pick changes (that copy may be months old). Logs carry counts and kinds only.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::sync::{Mutex, PoisonError};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::info;
use vtop_core::types::{
    AttendanceData, ExamScheduleData, FullAttendanceData, GradeViewData, MarksData, TimetableData,
};

use crate::records::{Attendance, AttendanceCourse, ClassKind};
use crate::{ATTENDANCE, ATTENDANCE_DETAIL, Core, PAGES, SELECTED, TIMETABLE, traced_sync};

pub(crate) const FILE: &str = "changes.json";
const MARKS: &str = "marks.json";
const EXAMS: &str = crate::exams::EXAMS;
const GRADES: &str = "grades.json";
/// The same change again within this window is one change.
const DEDUPE_MS: i64 = 10 * 60 * 1000;
/// A cap on count besides the app's byte cap ([`Core::trim_changes`]).
const KEEP: usize = 1000;
const THRESHOLD: f64 = 75.0;

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum ChangeKind {
    Attendance,
    Marks,
    Timetable,
    Exams,
    Grades,
}

/// One change, for the inbox and notifications.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct Change {
    pub id: String,
    pub kind: ChangeKind,
    /// When it was found (ms since 1970).
    pub at_ms: i64,
    pub title: String,
    pub body: String,
    /// The attendance course it is about ("CSE4007.theory"), for opening it.
    pub course_key: Option<String>,
    pub semester: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Store {
    /// Newest first.
    changes: Vec<Change>,
    /// Changes found after this were not seen yet.
    seen_ms: i64,
    /// The pick the primed set is for.
    semester: Option<String>,
    /// `<semester>/<file>` written since the pick changed: only these are compared.
    primed: BTreeSet<String>,
}

/// A change before it has an id and a time.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Draft {
    pub kind: ChangeKind,
    pub title: String,
    pub body: String,
    pub course_key: Option<String>,
}

fn draft(kind: ChangeKind, title: &str, body: String, course_key: Option<String>) -> Draft {
    Draft {
        kind,
        title: title.to_string(),
        body,
        course_key,
    }
}

#[uniffi::export]
impl Core {
    /// The recorded changes, newest first.
    pub fn changes(&self, flow: String) -> Vec<Change> {
        traced_sync("changes", &flow, || Ok(self.store().changes)).unwrap_or_default()
    }

    /// How many changes came after the inbox was last seen.
    pub fn unseen_changes(&self, flow: String) -> u32 {
        traced_sync("unseen_changes", &flow, || {
            let store = self.store();
            Ok(store
                .changes
                .iter()
                .filter(|change| change.at_ms > store.seen_ms)
                .count() as u32)
        })
        .unwrap_or(0)
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

impl Core {
    /// Before `file` (a path under the cache dir) is replaced by `new`: the changes against the
    /// copy there, recorded. Only the pages compared here are read; any other write costs nothing.
    pub(crate) fn record_changes(&self, file: &str, new: &[u8]) {
        let Some(rest) = file.strip_prefix(&format!("{PAGES}/")) else {
            return;
        };
        let Some((semester, page)) = rest.split_once('/') else {
            return; // Not per semester (the grade history).
        };
        if !compared(page) {
            return;
        }
        let picked = self.read::<String>(SELECTED);
        // Grades may come from the semester before the pick (no grades yet in the picked one).
        if page != GRADES && picked.as_deref() != Some(semester) {
            return;
        }
        let Ok(old) = fs::read(self.at(file)) else {
            // A first copy: nothing to compare, but it is primed now.
            self.update_store(|store| {
                prime(store, picked.as_deref(), semester, page);
            });
            return;
        };
        let drafts = detect(page, &old, new);
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
            for change in &store.changes[..added] {
                *kinds.entry(kind_name(change.kind)).or_default() += 1;
            }
            for (kind, n) in kinds {
                info!(kind, n, "changes recorded");
            }
        });
    }

    fn store(&self) -> Store {
        let _one = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        self.read::<Store>(FILE).unwrap_or_default()
    }

    fn update_store(&self, change: impl FnOnce(&mut Store)) {
        let _one = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut store = self.read::<Store>(FILE).unwrap_or_default();
        change(&mut store);
        self.write_plain(FILE, &store);
    }
}

/// Whether `page` (a file in a semester's pages) is compared.
fn compared(page: &str) -> bool {
    matches!(page, ATTENDANCE | MARKS | TIMETABLE | EXAMS | GRADES)
        || page.starts_with(&format!("{ATTENDANCE_DETAIL}/"))
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
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

/// The changes between two copies of `page`; none when either doesn't parse.
pub(crate) fn detect(page: &str, old: &[u8], new: &[u8]) -> Vec<Draft> {
    fn both<T: DeserializeOwned>(old: &[u8], new: &[u8]) -> Option<(T, T)> {
        Some((
            serde_json::from_slice(old).ok()?,
            serde_json::from_slice(new).ok()?,
        ))
    }
    match page {
        ATTENDANCE => both(old, new).map(|(o, n)| attendance(o, n)),
        MARKS => both(old, new).map(|(o, n)| marks(&o, &n)),
        TIMETABLE => both(old, new).map(|(o, n)| timetable(&o, &n)),
        EXAMS => both(old, new).map(|(o, n)| exams(&o, &n)),
        GRADES => both(old, new).map(|(o, n)| grades(&o, &n)),
        _ => page
            .strip_prefix(&format!("{ATTENDANCE_DETAIL}/"))
            .and_then(|name| name.strip_suffix(".json"))
            .and_then(|key| both(old, new).map(|(o, n)| history(key, &o, &n))),
    }
    .unwrap_or_default()
}

fn plural(n: i32, one: &str) -> String {
    format!("{n} {one}{}", if n == 1 { "" } else { "s" })
}

fn label(course: &AttendanceCourse) -> String {
    let name = if course.name.is_empty() {
        &course.code
    } else {
        &course.name
    };
    let kind = if course.kind == ClassKind::Lab {
        "Lab"
    } else {
        "Theory"
    };
    format!("{name} ({kind})")
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
        let posted = if held > 0 {
            let absent = (held - present).max(0);
            let mut parts = Vec::new();
            if present > 0 {
                parts.push(format!("{} present", unit(present)));
            }
            if absent > 0 {
                parts.push(format!("{} absent", unit(absent)));
            }
            if parts.is_empty() {
                parts.push(format!("{} posted", unit(held)));
            }
            format!("{}, ", parts.join(", "))
        } else {
            String::new()
        };
        let body = format!(
            "{}: {posted}now {}%",
            label(&course),
            course.percent.round()
        );
        let title = if crossed_down {
            "Attendance below 75%"
        } else if crossed_up {
            "Attendance back at 75%"
        } else {
            "Attendance posted"
        };
        out.push(draft(ChangeKind::Attendance, title, body, Some(course.key)));
    }
    out
}

/// A posted class whose status changed (an absence turned on duty, say); new classes are
/// counted by the summary instead.
fn history(key: &str, old: &FullAttendanceData, new: &FullAttendanceData) -> Vec<Draft> {
    let posted = |status: &str| {
        let s = status.trim();
        !s.is_empty() && s != "-"
    };
    let before: BTreeMap<(String, String), &str> = old
        .records
        .iter()
        .map(|r| ((r.date.clone(), r.slot.clone()), r.status.as_str()))
        .collect();
    new.records
        .iter()
        .filter_map(|r| {
            let was = *before.get(&(r.date.clone(), r.slot.clone()))?;
            (posted(was) && posted(&r.status) && !was.trim().eq_ignore_ascii_case(r.status.trim()))
                .then(|| {
                    let code = key.split('.').next().unwrap_or(key);
                    draft(
                        ChangeKind::Attendance,
                        "Attendance corrected",
                        format!(
                            "{code} on {}: {} → {}",
                            r.date.trim(),
                            was.trim().to_lowercase(),
                            r.status.trim().to_lowercase()
                        ),
                        Some(key.to_string()),
                    )
                })
        })
        .collect()
}

/// The attendance key a marks or exam row's course would have.
fn key_of(code: &str, course_type: &str) -> String {
    let lab = course_type.ends_with("LA") || course_type.ends_with("LO");
    format!("{}.{}", code.trim(), if lab { "lab" } else { "theory" })
}

/// A mark published (a new row, or a score where there was none) or changed.
fn marks(old: &MarksData, new: &MarksData) -> Vec<Draft> {
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
        let name = if course.coursetitle.trim().is_empty() {
            &course.coursecode
        } else {
            &course.coursetitle
        };
        let key = Some(key_of(&course.coursecode, &course.coursetype));
        for mark in &course.marks {
            let score = mark.scoredmark.trim();
            let shown = format!(
                "{} {score}/{}",
                mark.markstitle.trim(),
                mark.maxmarks.trim()
            );
            match was.get(&entry(mark)) {
                None | Some(_) if score.is_empty() => {}
                None => out.push(draft(
                    ChangeKind::Marks,
                    "New marks published",
                    format!("{}: {shown}", name.trim()),
                    key.clone(),
                )),
                Some(prev) if prev.scoredmark.trim().is_empty() => {
                    out.push(draft(
                        ChangeKind::Marks,
                        "New marks published",
                        format!("{}: {shown}", name.trim()),
                        key.clone(),
                    ));
                }
                Some(prev) if prev.scoredmark.trim() != score => out.push(draft(
                    ChangeKind::Marks,
                    "Marks updated",
                    format!(
                        "{}: {} {} → {score}/{}",
                        name.trim(),
                        mark.markstitle.trim(),
                        prev.scoredmark.trim(),
                        mark.maxmarks.trim()
                    ),
                    key.clone(),
                )),
                Some(_) => {}
            }
        }
    }
    out
}

/// Per course: the slots added and removed (a move is one of each).
fn timetable(old: &TimetableData, new: &TimetableData) -> Vec<Draft> {
    let signature = |s: &vtop_core::types::TimetableSlot| {
        format!(
            "{}|{}|{}|{}|{}|{}|{}",
            s.day, s.start_time, s.end_time, s.slot, s.course_code, s.block, s.room_no
        )
    };
    let before: BTreeSet<String> = old.slots.iter().map(signature).collect();
    let after: BTreeSet<String> = new.slots.iter().map(signature).collect();
    // Course code → (name, type, lines).
    let mut courses: BTreeMap<String, (String, String, Vec<String>)> = BTreeMap::new();
    let mut note = |s: &vtop_core::types::TimetableSlot, line: String| {
        courses
            .entry(s.course_code.trim().to_string())
            .or_insert_with(|| (s.name.trim().to_string(), s.course_type.clone(), Vec::new()))
            .2
            .push(line);
    };
    for s in new.slots.iter().filter(|s| !before.contains(&signature(s))) {
        let room = format!("{} {}", s.block.trim(), s.room_no.trim());
        let room = room.trim();
        note(
            s,
            format!(
                "+ {} {}{}",
                s.day.trim(),
                s.start_time.trim(),
                if room.is_empty() {
                    String::new()
                } else {
                    format!(" · {room}")
                }
            ),
        );
    }
    for s in old.slots.iter().filter(|s| !after.contains(&signature(s))) {
        note(s, format!("− {} {}", s.day.trim(), s.start_time.trim()));
    }
    courses
        .into_iter()
        .map(|(code, (name, kind, lines))| {
            let who = if name.is_empty() { code.clone() } else { name };
            draft(
                ChangeKind::Timetable,
                "Timetable changed",
                format!("{who}: {}", lines.join(", ")),
                Some(key_of(&code, &kind)),
            )
        })
        .collect()
}

/// A paper scheduled, removed, or with a new date, time, venue or seat.
fn exams(old: &ExamScheduleData, new: &ExamScheduleData) -> Vec<Draft> {
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
    let name = |r: &vtop_core::types::ExamScheduleRecord| {
        if r.course_name.trim().is_empty() {
            r.course_code.trim().to_string()
        } else {
            r.course_name.trim().to_string()
        }
    };
    for (id, (exam, r)) in &after {
        let key = Some(key_of(&r.course_code, &r.course_type));
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
                    "Exam scheduled",
                    format!("{} {exam}: {when}", name(r)),
                    key,
                ));
            }
            Some((_, was)) => {
                let mut what = Vec::new();
                if was.exam_date.trim() != r.exam_date.trim() {
                    what.push(format!(
                        "date {} → {}",
                        was.exam_date.trim(),
                        r.exam_date.trim()
                    ));
                }
                if was.exam_time.trim() != r.exam_time.trim()
                    || was.reporting_time.trim() != r.reporting_time.trim()
                {
                    what.push(format!("time {}", r.exam_time.trim()));
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
                        "Exam schedule updated",
                        format!("{} {exam}: {}", name(r), what.join(", ")),
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
                "Exam removed",
                format!("{} {exam}", name(r)),
                Some(key_of(&r.course_code, &r.course_type)),
            ));
        }
    }
    out
}

/// A grade published (one where there was none) or changed.
fn grades(old: &GradeViewData, new: &GradeViewData) -> Vec<Draft> {
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
            let name = if c.course_title.trim().is_empty() {
                c.course_code.trim()
            } else {
                c.course_title.trim()
            };
            let key = Some(key_of(&c.course_code, &c.course_type));
            match before.get(c.course_id.as_str()).copied() {
                Some(was) if was == grade => None,
                Some(was) if !was.is_empty() && was != "-" => Some(draft(
                    ChangeKind::Grades,
                    "Grade changed",
                    format!("{name}: {was} → {grade}"),
                    key,
                )),
                // A course new to the list with a grade, or a grade where there was none.
                _ => Some(draft(
                    ChangeKind::Grades,
                    "Grade published",
                    format!("{name}: {grade}"),
                    key,
                )),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtop_core::types::{
        AttendanceRecord, ExamScheduleRecord, FullAttendanceRecord, GradeCourseRecord, MarksRecord,
        MarksRecordEach, PerExamScheduleRecord, TimetableSlot,
    };

    fn json(value: &impl Serialize) -> Vec<u8> {
        serde_json::to_vec(value).unwrap()
    }

    fn att(attended: &str, total: &str, percent: &str) -> AttendanceData {
        AttendanceData {
            records: vec![AttendanceRecord {
                serial: "1".into(),
                category: String::new(),
                course_name: "CSE4007 - Digital Image Processing".into(),
                course_code: "CSE4007".into(),
                course_type: "ETH".into(),
                faculty_detail: String::new(),
                classes_attended: attended.into(),
                total_classes: total.into(),
                attendance_percentage: percent.into(),
                attendence_fat_cat: String::new(),
                debar_status: String::new(),
                course_id: "AM_CSE4007".into(),
            }],
            semester_id: "S".into(),
            update_time: 0,
        }
    }

    #[test]
    fn attendance_posted_and_crossings() {
        let posted = detect(
            ATTENDANCE,
            &json(&att("30", "36", "84%")),
            &json(&att("31", "38", "82%")),
        );
        assert_eq!(posted.len(), 1);
        assert_eq!(posted[0].title, "Attendance posted");
        assert_eq!(posted[0].course_key.as_deref(), Some("CSE4007.theory"));
        assert!(
            posted[0].body.contains("1 class present, 1 class absent"),
            "{}",
            posted[0].body
        );
        let down = detect(
            ATTENDANCE,
            &json(&att("27", "36", "75%")),
            &json(&att("27", "37", "73%")),
        );
        assert_eq!(down[0].title, "Attendance below 75%");
        let up = detect(
            ATTENDANCE,
            &json(&att("27", "37", "73%")),
            &json(&att("28", "37", "76%")),
        );
        assert_eq!(up[0].title, "Attendance back at 75%");
    }

    #[test]
    fn unchanged_data_records_nothing() {
        let a = json(&att("30", "36", "84%"));
        assert!(detect(ATTENDANCE, &a, &a).is_empty());
        let t = json(&tt(vec![slot("MON", "08:00", "321")]));
        assert!(detect(TIMETABLE, &t, &t).is_empty());
        assert!(detect("unknown.json", &a, &a).is_empty());
    }

    #[test]
    fn history_corrections() {
        let rec = |status: &str| FullAttendanceRecord {
            serial: "1".into(),
            date: "03-Nov-2026".into(),
            slot: "B2".into(),
            day_time: String::new(),
            status: status.into(),
            remark: String::new(),
        };
        let data = |status: &str| FullAttendanceData {
            records: vec![rec(status)],
            semester_id: "S".into(),
            update_time: 0,
            course_id: "AM_CSE4007".into(),
            course_type: "ETH".into(),
        };
        let page = format!("{ATTENDANCE_DETAIL}/CSE4007.theory.json");
        let fixed = detect(&page, &json(&data("Absent")), &json(&data("On Duty")));
        assert_eq!(fixed.len(), 1);
        assert_eq!(fixed[0].body, "CSE4007 on 03-Nov-2026: absent → on duty");
        // Posted for the first time: the summary counts it, not the history.
        assert!(detect(&page, &json(&data("")), &json(&data("Present"))).is_empty());
    }

    fn marks_of(score: &str, titles: &[&str]) -> MarksData {
        MarksData {
            records: vec![MarksRecord {
                serial: "1".into(),
                coursecode: "CSE4007".into(),
                coursetitle: "Digital Image Processing".into(),
                coursetype: "ETH".into(),
                faculity: String::new(),
                slot: String::new(),
                marks: titles
                    .iter()
                    .enumerate()
                    .map(|(i, t)| MarksRecordEach {
                        serial: format!("{}", i + 1),
                        markstitle: (*t).into(),
                        maxmarks: "50".into(),
                        weightage: String::new(),
                        status: String::new(),
                        scoredmark: score.into(),
                        weightagemark: String::new(),
                        remark: String::new(),
                    })
                    .collect(),
            }],
            semester_id: "S".into(),
            update_time: 0,
        }
    }

    #[test]
    fn marks_published_and_updated() {
        let new = detect(
            MARKS,
            &json(&marks_of("40", &["CAT-1"])),
            &json(&marks_of("40", &["CAT-1", "CAT-2"])),
        );
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].title, "New marks published");
        assert_eq!(new[0].body, "Digital Image Processing: CAT-2 40/50");
        let filled = detect(
            MARKS,
            &json(&marks_of("", &["CAT-1"])),
            &json(&marks_of("41", &["CAT-1"])),
        );
        assert_eq!(filled[0].title, "New marks published");
        let changed = detect(
            MARKS,
            &json(&marks_of("40", &["CAT-1"])),
            &json(&marks_of("42", &["CAT-1"])),
        );
        assert_eq!(changed[0].title, "Marks updated");
        // A row without a score yet is not news.
        assert!(
            detect(
                MARKS,
                &json(&marks_of("", &["CAT-1"])),
                &json(&marks_of("", &["CAT-1", "CAT-2"]))
            )
            .is_empty()
        );
    }

    fn slot(day: &str, start: &str, room: &str) -> TimetableSlot {
        TimetableSlot {
            serial: String::new(),
            day: day.into(),
            slot: "A1".into(),
            course_code: "CSE4007".into(),
            course_type: "ETH".into(),
            room_no: room.into(),
            block: "CB".into(),
            start_time: start.into(),
            end_time: String::new(),
            name: "Digital Image Processing".into(),
            kind: vtop_core::types::ClassKind::Theory,
            faculty: String::new(),
            credits: String::new(),
        }
    }

    fn tt(slots: Vec<TimetableSlot>) -> TimetableData {
        TimetableData {
            slots,
            courses: vec![],
            semester_id: "S".into(),
            update_time: 0,
        }
    }

    #[test]
    fn timetable_moves_are_one_change_per_course() {
        let moved = detect(
            TIMETABLE,
            &json(&tt(vec![slot("MON", "08:00", "321")])),
            &json(&tt(vec![slot("MON", "08:00", "512")])),
        );
        assert_eq!(moved.len(), 1);
        assert_eq!(
            moved[0].body,
            "Digital Image Processing: + MON 08:00 · CB 512, − MON 08:00"
        );
    }

    fn exam(date: &str, venue: &str) -> ExamScheduleData {
        ExamScheduleData {
            exams: vec![PerExamScheduleRecord {
                records: vec![ExamScheduleRecord {
                    serial: String::new(),
                    slot: String::new(),
                    course_name: "Digital Image Processing".into(),
                    course_code: "CSE4007".into(),
                    course_type: "ETH".into(),
                    course_id: "AM_CSE4007".into(),
                    exam_date: date.into(),
                    exam_session: String::new(),
                    reporting_time: String::new(),
                    exam_time: "10:00".into(),
                    venue: venue.into(),
                    seat_location: String::new(),
                    seat_no: String::new(),
                }],
                exam_type: "FAT".into(),
            }],
            semester_id: "S".into(),
            update_time: 0,
        }
    }

    #[test]
    fn exams_added_changed_removed() {
        let empty = ExamScheduleData {
            exams: vec![],
            semester_id: "S".into(),
            update_time: 0,
        };
        let added = detect(EXAMS, &json(&empty), &json(&exam("16-Nov-2026", "")));
        assert_eq!(added[0].title, "Exam scheduled");
        assert_eq!(
            added[0].body,
            "Digital Image Processing FAT: 16-Nov-2026 · 10:00"
        );
        let unposted = detect(EXAMS, &json(&empty), &json(&exam("-", "-")));
        assert_eq!(unposted[0].body, "Digital Image Processing FAT: 10:00");
        let moved = detect(
            EXAMS,
            &json(&exam("16-Nov-2026", "")),
            &json(&exam("18-Nov-2026", "CB 101")),
        );
        assert_eq!(
            moved[0].body,
            "Digital Image Processing FAT: date 16-Nov-2026 → 18-Nov-2026, venue CB 101"
        );
        let removed = detect(EXAMS, &json(&exam("16-Nov-2026", "")), &json(&empty));
        assert_eq!(removed[0].title, "Exam removed");
    }

    fn grade(g: &str) -> GradeViewData {
        GradeViewData {
            courses: vec![GradeCourseRecord {
                serial: String::new(),
                course_code: "CSE4007".into(),
                course_title: "Digital Image Processing".into(),
                course_type: "ETH".into(),
                grading_type: String::new(),
                grand_total: String::new(),
                grade: g.into(),
                course_id: "AM_CSE4007".into(),
            }],
            semesters: vec![],
            semester_id: "S".into(),
            update_time: 0,
        }
    }

    #[test]
    fn grades_published_and_changed() {
        let out = detect(GRADES, &json(&grade("")), &json(&grade("A")));
        assert_eq!(
            (out[0].title.as_str(), out[0].body.as_str()),
            ("Grade published", "Digital Image Processing: A")
        );
        let out = detect(GRADES, &json(&grade("B")), &json(&grade("A")));
        assert_eq!(out[0].body, "Digital Image Processing: B → A");
        assert!(detect(GRADES, &json(&grade("A")), &json(&grade("A"))).is_empty());
    }

    #[test]
    fn store_primes_dedupes_and_trims() {
        let mut store = Store::default();
        // First write of a page (or first after a new pick) is not compared.
        assert!(!prime(&mut store, Some("S"), "S", ATTENDANCE));
        assert!(prime(&mut store, Some("S"), "S", ATTENDANCE));
        assert!(!prime(&mut store, Some("T"), "T", ATTENDANCE));
        let d = |body: &str| draft(ChangeKind::Marks, "New marks published", body.into(), None);
        assert_eq!(add(&mut store, vec![d("a")], "S", 1_000), 1);
        assert_eq!(add(&mut store, vec![d("a"), d("b")], "S", 2_000), 1);
        assert_eq!(store.changes[0].body, "b");
        for n in 0..200 {
            add(
                &mut store,
                vec![d(&format!("change {n}"))],
                "S",
                10_000_000 + n,
            );
        }
        trim(&mut store, 4_000);
        assert!(serde_json::to_vec(&store).unwrap().len() <= 4_000);
        assert!(!store.changes.is_empty());
        assert_eq!(store.changes[0].body, "change 199", "the newest stays");
    }

    #[test]
    fn core_records_only_after_a_primed_copy() {
        let dir = std::env::temp_dir().join(format!("changes-{}", now_ms()));
        fs::create_dir_all(dir.join(format!("{PAGES}/S"))).unwrap();
        let core = Core {
            dir: dir.clone(),
            cache: dir.clone(),
            slot: Mutex::new(None),
        };
        core.write_plain(SELECTED, &"S".to_string());
        let file = format!("{PAGES}/S/{ATTENDANCE}");
        // First copy, then the same again: nothing.
        core.write(&file, &att("30", "36", "84%"));
        core.write(&file, &att("30", "36", "84%"));
        assert_eq!(core.unseen_changes("t".into()), 0);
        core.write(&file, &att("31", "37", "84%"));
        assert_eq!(core.unseen_changes("t".into()), 1);
        core.mark_changes_seen("t".into());
        assert_eq!(core.unseen_changes("t".into()), 0);
        assert_eq!(core.changes("t".into()).len(), 1);
        core.clear_changes("t".into());
        assert!(core.changes("t".into()).is_empty());
        let _ = fs::remove_dir_all(dir);
    }
}

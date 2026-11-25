//! Comparing two copies of a page: one [`Draft`] per thing that changed.

use std::collections::{BTreeMap, BTreeSet};

use serde::de::DeserializeOwned;
use vtop_core::types::{
    AcademicCalendarData, AttendanceData, CoursePageDetail, ExamScheduleData, FullAttendanceData,
    GeneralOutingData, GradeViewData, MarksData, TimetableData, WeekendOutingData,
};

use super::{ChangeKind, Context, Draft, EXAMS, PLANS, draft};
use crate::pages::grades::GRADES;
use crate::pages::marks::MARKS;
use crate::pages::outing::time::format_day;
use crate::pages::outing::{GENERAL, WEEKEND};
use crate::pages::records::{Attendance, AttendanceCourse, ClassKind, epoch_day};
use crate::{ATTENDANCE, ATTENDANCE_DETAIL, CALENDAR, TIMETABLE};

const THRESHOLD: f64 = 75.0;

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
pub(super) fn labelled(name: &str, lab: bool) -> String {
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

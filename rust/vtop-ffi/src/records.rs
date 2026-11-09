//! Ready-to-draw records built from vtop-core's raw tables.

use std::collections::{HashMap, HashSet};

use vtop_core::types::{self as raw, AttendanceData, FullAttendanceData, TimetableData};

use crate::calendar::SemesterCalendar;

/// Minimum attendance VIT requires, in percent.
const THRESHOLD: f64 = 75.0;

/// VTOP's day names, Monday first.
const DAYS: [&str; 7] = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum ClassKind {
    Theory,
    Lab,
}

impl From<raw::ClassKind> for ClassKind {
    fn from(kind: raw::ClassKind) -> Self {
        match kind {
            raw::ClassKind::Theory => Self::Theory,
            raw::ClassKind::Lab => Self::Lab,
        }
    }
}

/// One semester as VTOP names it, with the name split for display:
/// "Winter Semester 2024-25 Freshers" is season "Winter", year "2024-25",
/// note "Freshers". A name that does not follow that shape is all `season`.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Semester {
    pub id: String,
    pub name: String,
    pub season: String,
    pub year: String,
    pub note: String,
}

impl Semester {
    pub(crate) fn new(id: String, name: String) -> Self {
        let words: Vec<&str> = name.split_whitespace().collect();
        let shaped = words.len() >= 3
            && words[1].eq_ignore_ascii_case("semester")
            && words[2].len() >= 7
            && words[2].chars().take(4).all(|c| c.is_ascii_digit())
            && words[2].as_bytes()[4] == b'-';
        let (season, year, note) = if shaped {
            let mut season = words[0].to_lowercase();
            if let Some(first) = season.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            (season, words[2].to_string(), words[3..].join(" "))
        } else {
            (name.trim().to_string(), String::new(), String::new())
        };
        Self {
            id,
            name,
            season,
            year,
            note,
        }
    }
}

/// VTOP's semesters, newest first, and the one pages are shown for: the
/// student's pick, none until they pick one (pages fail with
/// [`crate::CoreError::SemesterNeeded`] meanwhile).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Semesters {
    pub items: Vec<Semester>,
    pub selected: Option<String>,
    /// Unix seconds.
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Attendance {
    pub semester_id: String,
    /// Unix seconds.
    pub updated_at: i64,
    pub courses: Vec<AttendanceCourse>,
}

/// One course. `attended`, `total` and `percent` are VTOP's; the standing
/// (`safe`, `can_skip`, `must_attend`, `advice`) follows VTOP's percentage
/// and counts labs in two-period sessions, like the Flutter card.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct AttendanceCourse {
    /// `<code>.theory` or `<code>.lab`: the course's key in the app and in
    /// [`AttendanceDetail`] calls.
    pub key: String,
    /// VTOP's ids for the per-class history page.
    pub course_id: String,
    pub course_type: String,
    pub code: String,
    pub name: String,
    pub kind: ClassKind,
    pub category: String,
    pub faculty: String,
    pub attended: i32,
    pub total: i32,
    /// The classes VTOP's percentage is over: `total` less the classes held
    /// but not posted yet, which VTOP's total counts and its percentage does
    /// not.
    pub counted: i32,
    /// VTOP's rounded figure when it sends one, else computed.
    pub percent: f64,
    pub safe: bool,
    pub can_skip: i32,
    pub must_attend: i32,
    /// "Can skip 3", "Attend 2", "Don't skip" or "No classes yet".
    pub advice: String,
    pub debarred: bool,
    /// What VTOP's debar status says, when it debars.
    pub debar: Option<Debar>,
}

/// A debarment, which holds for one exam: attendance is checked again
/// before the next.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Debar {
    /// "CAT-I", "CAT-II", "FAT" (the calendar's names), or VTOP's text;
    /// empty when VTOP names none.
    pub exam: String,
    /// VTOP's "Debarred & Permitted": let in after all.
    pub permitted: bool,
    /// The exam attendance is checked for next ("FAT" after "CAT-II").
    pub next_exam: Option<String>,
}

impl Debar {
    /// "CAT - II : Debarred & Permitted" → CAT-II, permitted, next FAT.
    fn parse(raw: &str) -> Option<Self> {
        let lower = raw.to_lowercase();
        if !lower.contains("debar") {
            return None;
        }
        let exam = raw
            .split_once(':')
            .map_or(String::new(), |(exam, _)| crate::calendar::exam_name(exam));
        let next_exam = match exam.as_str() {
            "CAT-I" => Some("CAT-II".to_string()),
            "CAT-II" => Some("FAT".to_string()),
            _ => None,
        };
        Some(Self {
            exam,
            permitted: lower.contains("permit"),
            next_exam,
        })
    }
}

/// One course's class-by-class attendance.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct AttendanceDetail {
    /// The course as the saved attendance has it.
    pub course: AttendanceCourse,
    /// Unix seconds.
    pub updated_at: i64,
    /// Newest first; a lab session is one class.
    pub classes: Vec<AttendedClass>,
    /// Until the next exam (when one comes before the last class day), then
    /// until the semester's end; none without a saved academic calendar.
    pub plans: Vec<AttendancePlan>,
}

/// One course's [`AttendancePlan`] for the list, by [`AttendanceCourse::key`].
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CourseOutlook {
    pub key: String,
    pub plan: AttendancePlan,
}

/// What the rest of the semester, or the time until an exam, holds for a
/// course; labs in sessions. From the saved timetable and academic calendar.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct AttendancePlan {
    /// The exam's name ("CAT-II", "FAT"), or empty for the semester's end.
    pub until: String,
    /// The day counting stops before (the exam's first day, or the day after
    /// the last class day).
    pub until_day: i32,
    /// Attended and held as VTOP's percentage counts them.
    pub attended: i32,
    pub held: i32,
    /// Classes still to come from now, by the timetable and calendar.
    pub upcoming: i32,
    /// Held but not posted yet (VTOP's total less what its percentage counts).
    pub unposted: i32,
    /// Of `upcoming + unposted`, how many can be missed to finish at 75%.
    pub can_miss: i32,
    /// How many must be attended to finish at 75%; none when even all of
    /// them are not enough.
    pub must_attend: Option<i32>,
    /// The percentage when every one left is attended.
    pub best_percent: f64,
    /// The days of `upcoming`, oldest first, with how many classes each holds.
    pub days: Vec<PlanDay>,
}

/// A day still to come with classes of the course (sessions for labs).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct PlanDay {
    /// Days since 1970-01-01.
    pub day: i32,
    pub classes: i32,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct AttendedClass {
    /// Days since 1970-01-01.
    pub day: i32,
    /// 0 = Monday.
    pub weekday: i32,
    /// "A1", or "L10+L11" for a lab.
    pub slot: String,
    /// Minutes since midnight, when VTOP gives the time.
    pub start: Option<i32>,
    pub end: Option<i32>,
    /// None until VTOP posts it.
    pub status: Option<ClassStatus>,
    /// VTOP's remark ("Medical", "No punch found ...").
    pub remark: String,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Timetable {
    pub semester_id: String,
    /// Unix seconds.
    pub updated_at: i64,
    /// By day, then start; back-to-back periods of a lab are one slot.
    pub slots: Vec<ClassSlot>,
    pub courses: Vec<Course>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ClassSlot {
    /// 0 = Monday.
    pub day: i32,
    /// Minutes since midnight.
    pub start: i32,
    pub end: i32,
    pub code: String,
    pub name: String,
    pub kind: ClassKind,
    pub room: String,
    pub block: String,
    /// "A1", or "L1+L2" for a merged lab.
    pub slot: String,
    pub faculty: String,
    pub credits: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Course {
    pub code: String,
    pub name: String,
    pub kind: ClassKind,
    pub credits: Option<f64>,
}

/// One day of the agenda.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct TimetableDay {
    /// Days since 1970-01-01, local.
    pub day: i32,
    pub classes: Vec<DayClass>,
    pub mark: Option<CalendarMark>,
    /// The calendar's name for the day ("Deepavali", "CAT - I").
    pub title: Option<String>,
    pub holds_theory: bool,
    pub last_class_day: bool,
    pub papers: Vec<ExamPaper>,
    pub class_days_left: i32,
    pub upcoming_exam: Option<ExamStart>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct DayClass {
    pub slot: ClassSlot,
    pub state: ClassDay,
    /// The course's last class of the semester.
    pub last_class: bool,
    /// What the attendance history says, once posted.
    pub posted: Option<ClassStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ClassDay {
    Held,
    LabFat,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CalendarMark {
    Exam,
    LabFat,
    Holiday,
    NoClasses,
    Special,
    Classes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ClassStatus {
    Present,
    OnDuty,
    Absent,
    Other,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ExamPaper {
    pub name: String,
    pub course: String,
    pub day: i32,
    /// Minutes since midnight.
    pub start: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ExamStart {
    pub name: String,
    pub day: i32,
}

/// `count` days of the weekly timetable from epoch day `from_day`, shaped by
/// the academic calendar when there is one (holidays and exams have no
/// classes, labs on Lab FAT days are the lab exam, working days follow the
/// weekday they name), like the Flutter agenda.
pub(crate) fn timetable_days(
    slots: &[ClassSlot],
    from_day: i32,
    count: i32,
    calendar: Option<&SemesterCalendar>,
) -> Vec<TimetableDay> {
    let calendar = calendar.filter(|calendar| !calendar.is_empty());
    // The weekdays each course has labs on, for its Lab FAT day.
    let lab_days = |code: &str| -> Vec<i32> {
        slots
            .iter()
            .filter(|slot| slot.kind == ClassKind::Lab && slot.code == code)
            .map(|slot| slot.day)
            .collect()
    };
    let state = |slot: &ClassSlot, day: i32| match calendar {
        Some(calendar) => {
            let lab = slot.kind == ClassKind::Lab;
            calendar.class_day(
                day,
                lab,
                &if lab {
                    lab_days(&slot.code)
                } else {
                    Vec::new()
                },
            )
        }
        None => ClassDay::Held,
    };
    let schedule = |day: i32| -> Vec<&ClassSlot> {
        let weekday = calendar.map_or_else(|| weekday(day), |calendar| calendar.weekday_of(day));
        slots.iter().filter(|slot| slot.day == weekday).collect()
    };
    let last_day = calendar.and_then(SemesterCalendar::last_class_day);
    // Days from `from_day` to the last class day on which a class meets.
    let class_days: Vec<i32> = last_day.map_or_else(Vec::new, |last| {
        (from_day..=last)
            .filter(|&day| {
                schedule(day)
                    .iter()
                    .any(|slot| state(slot, day) != ClassDay::Off)
            })
            .collect()
    });
    // The day each course (code, kind) meets for the last time, looking back
    // from the last class day.
    let mut last_class: HashMap<(&str, ClassKind), i32> = HashMap::new();
    if let (Some(last), Some(first)) = (
        last_day,
        calendar.and_then(SemesterCalendar::first_class_day),
    ) {
        let courses = slots
            .iter()
            .map(|slot| (slot.code.as_str(), slot.kind))
            .collect::<HashSet<_>>()
            .len();
        let mut day = last;
        while day >= first && last_class.len() < courses {
            for slot in schedule(day) {
                if state(slot, day) == ClassDay::Held {
                    last_class
                        .entry((slot.code.as_str(), slot.kind))
                        .or_insert(day);
                }
            }
            day -= 1;
        }
    }
    (from_day..from_day + count.max(0))
        .map(|day| {
            let classes: Vec<DayClass> = schedule(day)
                .into_iter()
                .map(|slot| {
                    let state = state(slot, day);
                    DayClass {
                        slot: slot.clone(),
                        state,
                        last_class: state == ClassDay::Held
                            && last_class.get(&(slot.code.as_str(), slot.kind)) == Some(&day),
                        posted: None,
                    }
                })
                .collect();
            let (mark, title) = calendar
                .and_then(|calendar| calendar.mark_on(day))
                .map_or((None, None), |(mark, title)| (Some(mark), Some(title)));
            TimetableDay {
                day,
                holds_theory: match calendar {
                    Some(calendar) if calendar.covers(day) => calendar.holds_classes(day, false),
                    _ => !classes.is_empty(),
                },
                classes,
                mark,
                title,
                last_class_day: last_day == Some(day),
                papers: Vec::new(),
                class_days_left: (class_days.len() - class_days.partition_point(|&d| d <= day))
                    as i32,
                upcoming_exam: calendar
                    .and_then(|calendar| calendar.upcoming_exam(day))
                    .map(|(day, name)| ExamStart { name, day }),
            }
        })
        .collect()
}

/// When a course (code and kind) meets next.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct NextClass {
    pub code: String,
    pub kind: ClassKind,
    /// 0 = today, 7 = the same weekday next week.
    pub days_ahead: i32,
    /// Minutes since midnight.
    pub start: i32,
}

/// The next class of each course from `now_unix` (local time at
/// `utc_offset_minutes`), soonest first; a class today counts once it has
/// not started yet. Without a calendar, the weekly timetable a week ahead.
/// With one, four weeks ahead by its rules, as in [`timetable_days`]:
/// holidays, exams and no-class days skipped, working days following
/// another day's order, labs stopping at the Lab FAT.
pub(crate) fn next_classes(
    slots: &[ClassSlot],
    now_unix: i64,
    utc_offset_minutes: i32,
    calendar: Option<&SemesterCalendar>,
) -> Vec<NextClass> {
    let calendar = calendar.filter(|calendar| !calendar.is_empty());
    let local = now_unix + i64::from(utc_offset_minutes) * 60;
    let today = local.div_euclid(86_400) as i32;
    let minute = (local.rem_euclid(86_400) / 60) as i32;
    let lab_days = |code: &str| -> Vec<i32> {
        slots
            .iter()
            .filter(|slot| slot.kind == ClassKind::Lab && slot.code == code)
            .map(|slot| slot.day)
            .collect()
    };
    let mut next: Vec<NextClass> = Vec::new();
    for days_ahead in 0..=if calendar.is_some() { 28 } else { 7 } {
        let day = today + days_ahead;
        let order = calendar.map_or_else(|| weekday(day), |calendar| calendar.weekday_of(day));
        let mut meeting: Vec<&ClassSlot> = slots
            .iter()
            .filter(|slot| slot.day == order && (days_ahead > 0 || slot.start > minute))
            .collect();
        meeting.sort_by_key(|slot| slot.start);
        for slot in meeting {
            if next
                .iter()
                .any(|known| known.code == slot.code && known.kind == slot.kind)
            {
                continue;
            }
            let lab = slot.kind == ClassKind::Lab;
            let held = calendar.is_none_or(|calendar| {
                calendar.class_day(
                    day,
                    lab,
                    &if lab {
                        lab_days(&slot.code)
                    } else {
                        Vec::new()
                    },
                ) == ClassDay::Held
            });
            if held {
                next.push(NextClass {
                    code: slot.code.clone(),
                    kind: slot.kind,
                    days_ahead,
                    start: slot.start,
                });
            }
        }
    }
    next.sort_by_key(|class| (class.days_ahead, class.start));
    next
}

/// Puts each held class's status from `histories` (by course code and
/// kind, as [`AttendanceDetail::classes`]) on `days`: the history row of
/// that date with the same slot, else the same start, else the next unused
/// one. Not posted stays none.
pub(crate) fn fill_posted(
    days: &mut [TimetableDay],
    histories: &HashMap<(String, ClassKind), Vec<AttendedClass>>,
) {
    for day in days {
        let mut used: HashMap<(String, ClassKind), Vec<usize>> = HashMap::new();
        for class in &mut day.classes {
            if class.state != ClassDay::Held {
                continue;
            }
            let key = (class.slot.code.clone(), class.slot.kind);
            let Some(rows) = histories.get(&key) else {
                continue;
            };
            let used = used.entry(key).or_default();
            let free: Vec<(usize, &AttendedClass)> = rows
                .iter()
                .enumerate()
                .filter(|(i, row)| row.day == day.day && !used.contains(i))
                .collect();
            let pick = free
                .iter()
                .find(|(_, row)| row.slot == class.slot.slot)
                .or_else(|| {
                    free.iter()
                        .find(|(_, row)| row.start == Some(class.slot.start))
                })
                .or_else(|| free.first());
            if let Some((i, row)) = pick {
                used.push(*i);
                class.posted = row.status;
            }
        }
    }
}

/// Whether a saved history matches `course`'s summary: as many classes
/// attended (present or on duty) and listed (posted or not) as VTOP counts,
/// labs twice. A history behind the summary is fetched again.
pub(crate) fn history_in_sync(course: &AttendanceCourse, classes: &[AttendedClass]) -> bool {
    let factor = if course.kind == ClassKind::Lab { 2 } else { 1 };
    let attended = classes
        .iter()
        .filter(|class| {
            matches!(
                class.status,
                Some(ClassStatus::Present | ClassStatus::OnDuty)
            )
        })
        .count() as i32;
    attended * factor == course.attended && classes.len() as i32 * factor == course.total
}

/// The last day VTOP's summary covers: the latest class in a history that
/// matches it (Flutter's `countedThrough`).
pub(crate) fn counted_through(course: &AttendanceCourse, classes: &[AttendedClass]) -> Option<i32> {
    history_in_sync(course, classes)
        .then(|| classes.iter().map(|class| class.day).max())
        .flatten()
}

/// Where a course stands against 75%, with the strict arithmetic of the
/// attendance calculator.
#[derive(Debug, PartialEq)]
struct Standing {
    safe: bool,
    can_skip: i32,
    must_attend: i32,
    advice: String,
}

/// The standing for `attended` of `total` classes. VTOP's `reported`
/// percentage decides safe or not (it is what debars); the counts only say
/// by how much, and never contradict it.
fn standing(attended: i32, total: i32, reported: Option<f64>) -> Standing {
    let (a, t) = (i64::from(attended), i64::from(total));
    let by_counts = t == 0 || a as f64 / t as f64 * 100.0 >= THRESHOLD;
    let safe = match reported {
        Some(percent) if t > 0 => percent >= THRESHOLD,
        _ => by_counts,
    };
    let (can_skip, must_attend) = if safe {
        let skip = if t == 0 {
            0
        } else {
            (4 * a - 3 * t).div_euclid(3)
        };
        (skip.max(0) as i32, 0)
    } else {
        (0, (3 * t - 4 * a).max(1) as i32)
    };
    let advice = match () {
        _ if t == 0 => "No classes yet".to_string(),
        _ if !safe => format!("Attend {must_attend}"),
        _ if can_skip == 0 => "Don't skip".to_string(),
        _ => format!("Can skip {can_skip}"),
    };
    Standing {
        safe,
        can_skip,
        must_attend,
        advice,
    }
}

/// The total VTOP's whole-number `reported` percentage of `attended` is
/// over: the largest from `attended` to `total` that gives it, rounded up as
/// VTOP rounds (31 of 34 is 92%). VTOP's total counts classes not posted
/// yet, its percentage does not. `total` when nothing matches.
fn counted_total(attended: i32, total: i32, reported: Option<f64>) -> i32 {
    let Some(percent) = reported.filter(|percent| percent.fract() == 0.0) else {
        return total;
    };
    let a = i64::from(attended) * 100;
    (attended.max(1)..=total)
        .rev()
        .find(|&t| {
            let t = i64::from(t);
            ((a + t - 1) / t) as f64 == percent
        })
        .unwrap_or(total)
}

/// The course's plans from `today` at `minute` (local, India): classes of
/// its kind in `slots` on each day `calendar` holds them (a lab session is
/// one slot), today's only when not started; until the next exam and until
/// the last class day. Not posted: what VTOP lists as such, plus the classes
/// held after `counted_through` (the last day VTOP's summary covers) up to
/// now. Strict 75% arithmetic, like the calculator.
pub(crate) fn attendance_plans(
    course: &AttendanceCourse,
    slots: &[ClassSlot],
    calendar: &SemesterCalendar,
    today: i32,
    minute: i32,
    counted_through: Option<i32>,
) -> Vec<AttendancePlan> {
    let Some(last) = calendar.last_class_day() else {
        return Vec::new();
    };
    let lab = course.kind == ClassKind::Lab;
    let (attended, held, unposted) = if lab {
        (
            course.attended / 2,
            (course.counted + 1) / 2,
            (course.total - course.counted + 1) / 2,
        )
    } else {
        (
            course.attended,
            course.counted,
            course.total - course.counted,
        )
    };
    let mine: Vec<&ClassSlot> = slots
        .iter()
        .filter(|slot| slot.code == course.code && slot.kind == course.kind)
        .collect();
    let meetings = |day: i32| -> Vec<&&ClassSlot> {
        let weekday = calendar.weekday_of(day);
        mine.iter().filter(|slot| slot.day == weekday).collect()
    };
    // Held since the summary's last day, not on VTOP yet.
    let unposted = unposted
        + counted_through
            .filter(|&through| through < today)
            .map_or(0, |through| {
                (through + 1..=today.min(last))
                    .filter(|&day| calendar.holds_classes(day, lab))
                    .map(|day| {
                        meetings(day)
                            .into_iter()
                            .filter(|slot| day < today || slot.start <= minute)
                            .count() as i32
                    })
                    .sum()
            });
    let upcoming = |until_day: i32| -> Vec<PlanDay> {
        (today..until_day.min(last + 1))
            .filter(|&day| calendar.holds_classes(day, lab))
            .map(|day| {
                let weekday = calendar.weekday_of(day);
                let classes = mine
                    .iter()
                    .filter(|slot| slot.day == weekday && (day > today || slot.start > minute))
                    .count() as i32;
                PlanDay { day, classes }
            })
            .filter(|day| day.classes > 0)
            .collect()
    };
    let plan = |until: String, until_day: i32| {
        let days = upcoming(until_day);
        let upcoming = days.iter().map(|day| day.classes).sum();
        let left = i64::from(upcoming + unposted);
        let (a, t) = (i64::from(attended), i64::from(held));
        let total = t + left;
        let must = (3 * total - 4 * a + 3).div_euclid(4).max(0);
        AttendancePlan {
            until,
            until_day,
            attended,
            held,
            upcoming,
            unposted,
            can_miss: (4 * (a + left) - 3 * total).div_euclid(4).clamp(0, left) as i32,
            must_attend: (must <= left).then_some(must as i32),
            best_percent: if total == 0 {
                0.0
            } else {
                (a + left) as f64 / total as f64 * 100.0
            },
            days,
        }
    };
    let mut plans = Vec::new();
    if let Some((day, name)) = calendar
        .upcoming_exam(today)
        .filter(|(day, _)| *day <= last)
    {
        plans.push(plan(name, day));
    }
    plans.push(plan(String::new(), last + 1));
    plans
}

/// Now in India (VIT-AP's time zone): days since 1970-01-01 and minutes
/// since midnight.
pub(crate) fn ist_now() -> (i32, i32) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
        + 19_800;
    (
        now.div_euclid(86_400) as i32,
        (now.rem_euclid(86_400) / 60) as i32,
    )
}

/// "CSE2001 - Data Structures - Embedded Theory" → ("CSE2001", "Data
/// Structures"). Without separators the name is kept and `code` used.
fn split_course_name(full: &str, code: &str) -> (String, String) {
    let parts: Vec<&str> = full.split(" - ").map(str::trim).collect();
    match parts.as_slice() {
        [code, name] => (code.to_string(), name.to_string()),
        [code, name @ .., _kind] => (code.to_string(), name.join(" - ")),
        _ => (code.trim().to_string(), full.trim().to_string()),
    }
}

/// "PRASHANTH KUMAR - SCOPE" → "Prashanth Kumar". Mixed-case names are
/// kept so initials like "Md" survive.
fn person_name(raw: &str) -> String {
    let name = raw
        .rfind('-')
        .filter(|&i| i > 0)
        .map_or(raw, |i| &raw[..i])
        .trim();
    if name != name.to_uppercase() {
        return name.to_string();
    }
    let word = |w: &str| match w.chars().next() {
        Some(first) if w.chars().count() > 1 && !w.contains('.') => {
            format!("{first}{}", w[first.len_utf8()..].to_lowercase())
        }
        _ => w.to_string(),
    };
    name.split_whitespace()
        .map(word)
        .collect::<Vec<_>>()
        .join(" ")
}

/// "08:00", "2:05 PM" → minutes since midnight.
pub(crate) fn minutes(raw: &str) -> Option<i32> {
    let upper = raw.trim().to_ascii_uppercase();
    let (clock, pm) = match (upper.strip_suffix("PM"), upper.strip_suffix("AM")) {
        (Some(clock), _) => (clock, Some(true)),
        (_, Some(clock)) => (clock, Some(false)),
        _ => (upper.as_str(), None),
    };
    let (hour, minute) = clock.trim().split_once(':')?;
    let (mut hour, minute): (i32, i32) = (hour.trim().parse().ok()?, minute.trim().parse().ok()?);
    match pm {
        Some(true) if hour != 12 => hour += 12,
        Some(false) if hour == 12 => hour = 0,
        _ => {}
    }
    ((0..24).contains(&hour) && (0..60).contains(&minute)).then_some(hour * 60 + minute)
}

fn credits(raw: &str) -> Option<f64> {
    raw.trim().parse().ok()
}

impl From<AttendanceData> for Attendance {
    fn from(data: AttendanceData) -> Self {
        let course = |record: raw::AttendanceRecord| {
            let attended = record.classes_attended.trim().parse().unwrap_or(0);
            let total: i32 = record.total_classes.trim().parse().unwrap_or(0);
            let lab = record.course_type.ends_with("LA") || record.course_type.ends_with("LO");
            let reported: Option<f64> = record
                .attendance_percentage
                .replace('%', "")
                .trim()
                .parse()
                .ok();
            let counted = counted_total(attended, total, reported);
            // Labs meet in two-period sessions VTOP counts twice; an odd total
            // rounds up and an odd attended count down.
            let plan = if lab {
                standing(attended / 2, (counted + 1) / 2, reported)
            } else {
                standing(attended, counted, reported)
            };
            let computed = if counted == 0 {
                0.0
            } else {
                f64::from(attended) / f64::from(counted) * 100.0
            };
            let (code, name) = split_course_name(&record.course_name, &record.course_code);
            let debar = Debar::parse(&record.debar_status);
            AttendanceCourse {
                key: format!("{code}.{}", if lab { "lab" } else { "theory" }),
                code,
                name,
                kind: if lab {
                    ClassKind::Lab
                } else {
                    ClassKind::Theory
                },
                category: record.category,
                faculty: person_name(&record.faculty_detail),
                attended,
                total,
                counted,
                percent: reported.unwrap_or(computed),
                safe: plan.safe,
                can_skip: plan.can_skip,
                must_attend: plan.must_attend,
                advice: plan.advice,
                debarred: debar.is_some(),
                debar,
                course_id: record.course_id,
                course_type: record.course_type,
            }
        };
        Self {
            semester_id: data.semester_id,
            updated_at: data.update_time as i64,
            courses: data.records.into_iter().map(course).collect(),
        }
    }
}

impl AttendanceDetail {
    pub(crate) fn new(
        course: AttendanceCourse,
        data: FullAttendanceData,
        plans: Vec<AttendancePlan>,
    ) -> Self {
        let mut classes: Vec<AttendedClass> = data
            .records
            .into_iter()
            .filter_map(|record| {
                let day = history_day(&record.date)?;
                // "WED / 11:00-12:40"
                let time = record.day_time.rsplit('/').next().unwrap_or_default();
                let (start, end) = time.split_once('-').unzip();
                Some(AttendedClass {
                    day,
                    weekday: (day + 3).rem_euclid(7),
                    slot: record.slot,
                    start: start.and_then(minutes),
                    end: end.and_then(minutes),
                    status: match record.status.to_lowercase().replace(' ', "").as_str() {
                        "present" => Some(ClassStatus::Present),
                        "onduty" => Some(ClassStatus::OnDuty),
                        "absent" => Some(ClassStatus::Absent),
                        "notposted" | "" => None,
                        _ => Some(ClassStatus::Other),
                    },
                    remark: record.remark,
                })
            })
            .collect();
        classes.sort_by_key(|class| (std::cmp::Reverse(class.day), std::cmp::Reverse(class.start)));
        Self {
            course,
            updated_at: data.update_time as i64,
            classes,
            plans,
        }
    }
}

/// A history date, "19-11-2025" or "01-Jul-2026", as days since 1970-01-01.
pub(crate) fn history_day(raw: &str) -> Option<i32> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let mut parts = raw.trim().split('-').map(str::trim);
    let day: u32 = parts.next()?.parse().ok()?;
    let month = parts.next()?.to_lowercase();
    let month = match month.parse::<u32>() {
        Ok(month) => month,
        Err(_) => MONTHS.iter().position(|name| month.starts_with(name))? as u32 + 1,
    };
    let year: u32 = parts.next()?.parse().ok()?;
    epoch_day(&format!("{year:04}-{month:02}-{day:02}"))
}

impl From<TimetableData> for Timetable {
    fn from(data: TimetableData) -> Self {
        let mut slots: Vec<ClassSlot> = data
            .slots
            .into_iter()
            .filter_map(|slot| {
                Some(ClassSlot {
                    day: DAYS
                        .iter()
                        .position(|day| slot.day.trim().eq_ignore_ascii_case(day))?
                        as i32,
                    start: minutes(&slot.start_time)?,
                    end: minutes(&slot.end_time)?,
                    code: slot.course_code,
                    name: slot.name,
                    kind: slot.kind.into(),
                    room: slot.room_no,
                    block: slot.block,
                    slot: slot.slot,
                    faculty: person_name(&slot.faculty),
                    credits: credits(&slot.credits),
                })
            })
            .collect();
        slots.sort_by_key(|slot| (slot.day, slot.start));
        // Join back-to-back periods of the same lab into one session.
        let mut merged: Vec<ClassSlot> = Vec::with_capacity(slots.len());
        for slot in slots {
            match merged.last_mut() {
                Some(prev)
                    if prev.day == slot.day
                        && prev.kind == ClassKind::Lab
                        && slot.kind == ClassKind::Lab
                        && prev.code == slot.code =>
                {
                    prev.end = slot.end;
                    prev.slot = format!("{}+{}", prev.slot, slot.slot);
                }
                _ => merged.push(slot),
            }
        }
        let courses = data
            .courses
            .into_iter()
            .map(|course| Course {
                kind: if course.course_type.to_lowercase().contains("lab") {
                    ClassKind::Lab
                } else {
                    ClassKind::Theory
                },
                credits: credits(&course.credits),
                code: course.course_code,
                name: course.name,
            })
            .collect();
        Self {
            semester_id: data.semester_id,
            updated_at: data.update_time as i64,
            slots: merged,
            courses,
        }
    }
}

/// Today in India, days since 1970-01-01.
pub(crate) fn today_ist_day() -> i32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64);
    (now + 19_800).div_euclid(86_400) as i32
}

/// 0 = Monday, for days since 1970-01-01 (a Thursday).
pub(crate) fn weekday(day: i32) -> i32 {
    (day + 3).rem_euclid(7)
}

/// Days since 1970-01-01 for a `YYYY-MM-DD` (Howard Hinnant's days_from_civil).
pub(crate) fn epoch_day(date: &str) -> Option<i32> {
    let mut parts = date.trim().splitn(3, '-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: String = parts
        .next()?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let day: i64 = day.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146_097 + doe - 719_468) as i32)
}

/// `YYYY-MM-DD` for days since 1970-01-01 (Howard Hinnant's civil_from_days).
pub(crate) fn civil_date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn semester_names_split() {
        let s = super::Semester::new("AP1".into(), "Winter Semester 2024-25 Freshers".into());
        assert_eq!(
            (s.season.as_str(), s.year.as_str(), s.note.as_str()),
            ("Winter", "2024-25", "Freshers")
        );
        let s = super::Semester::new("AP2".into(), "FALL SEMESTER 2026-27".into());
        assert_eq!(
            (s.season.as_str(), s.year.as_str(), s.note.as_str()),
            ("Fall", "2026-27", "")
        );
        let s = super::Semester::new("AP3".into(), "Special Course Weekend".into());
        assert_eq!(
            (s.season.as_str(), s.year.as_str()),
            ("Special Course Weekend", "")
        );
    }

    use super::*;

    // Cases from test/attendance_standing_test.dart.
    #[test]
    fn safe_course_reports_skips() {
        let s = standing(22, 24, None);
        assert!(s.safe);
        assert_eq!(s.can_skip, 5);
        assert_eq!(s.advice, "Can skip 5");
    }

    #[test]
    fn short_course_reports_classes_needed() {
        let s = standing(6, 10, None);
        assert!(!s.safe);
        assert_eq!(s.must_attend, 6);
        assert_eq!(s.advice, "Attend 6");
    }

    #[test]
    fn exactly_on_the_line_cannot_skip() {
        let s = standing(3, 4, None);
        assert!(s.safe);
        assert_eq!(s.can_skip, 0);
        assert_eq!(s.advice, "Don't skip");
    }

    #[test]
    fn no_classes_yet() {
        let s = standing(0, 0, None);
        assert!(s.safe);
        assert_eq!(s.advice, "No classes yet");
    }

    #[test]
    fn course_names_split() {
        let split = |full| split_course_name(full, "X");
        assert_eq!(
            split("CSE2001 - Data Structures - Embedded Theory"),
            ("CSE2001".into(), "Data Structures".into())
        );
        assert_eq!(
            split("MAT1001 - Calculus-I - Theory Only"),
            ("MAT1001".into(), "Calculus-I".into())
        );
        assert_eq!(
            split("ENG1001 - English"),
            ("ENG1001".into(), "English".into())
        );
        assert_eq!(
            split("Data Structures"),
            ("X".into(), "Data Structures".into())
        );
    }

    #[test]
    fn names_and_times() {
        assert_eq!(person_name("PRASHANTH KUMAR - SCOPE"), "Prashanth Kumar");
        assert_eq!(person_name("Dr. Md Rafi - SCOPE"), "Dr. Md Rafi");
        assert_eq!(minutes("08:00"), Some(480));
        assert_eq!(minutes("2:05 PM"), Some(845));
        assert_eq!(minutes("12:10 am"), Some(10));
        assert_eq!(minutes("-"), None);
    }

    fn record(
        course_type: &str,
        attended: &str,
        total: &str,
        percent: &str,
    ) -> raw::AttendanceRecord {
        raw::AttendanceRecord {
            serial: "1".into(),
            category: "Program Core".into(),
            course_name: "SWE1002 - Principles of Software Engineering - Embedded Lab".into(),
            course_code: "AP2025262000671 - L10+L11 - 409".into(),
            course_type: course_type.into(),
            faculty_detail: "SHARMILA SHAIK - SCOPE".into(),
            classes_attended: attended.into(),
            total_classes: total.into(),
            attendance_percentage: percent.into(),
            attendence_fat_cat: "-".into(),
            debar_status: "-".into(),
            course_id: "AM_SWE1002_00200".into(),
        }
    }

    fn courses(records: Vec<raw::AttendanceRecord>) -> Vec<AttendanceCourse> {
        Attendance::from(AttendanceData {
            records,
            semester_id: "AP2025262".into(),
            update_time: 1,
        })
        .courses
    }

    #[test]
    fn lab_standing_counts_sessions() {
        let courses = courses(vec![
            record("ELA", "11", "14", "79%"),
            record("ETH", "11", "14", "79%"),
        ]);
        assert_eq!(
            (courses[0].kind, courses[0].percent, courses[0].key.as_str()),
            (ClassKind::Lab, 79.0, "SWE1002.lab")
        );
        // Lab: 5 of 7 sessions by the counts, but VTOP says 79%: safe.
        assert_eq!((courses[0].safe, courses[0].must_attend), (true, 0));
        assert_eq!(courses[0].advice, "Don't skip");
        assert_eq!(courses[1].key, "SWE1002.theory");
        assert_eq!(courses[1].advice, "Don't skip");
    }

    // Fall 2025-26, SWE1002 lab: 11 sessions attended, 16 held, 2 not posted;
    // VTOP counts 22 of 32 periods but reports 11 of 14 = 79%.
    #[test]
    fn standing_follows_vtops_percentage() {
        let lab = &courses(vec![record("ELA", "22", "32", "79%")])[0];
        assert_eq!((lab.percent, lab.counted), (79.0, 28));
        assert!(lab.safe);
        assert_eq!((lab.must_attend, lab.advice.as_str()), (0, "Don't skip"));
        // Below VTOP's line: always something to attend, even when the
        // counts alone would say safe.
        let short = &courses(vec![record("ETH", "30", "40", "74%")])[0];
        assert!(!short.safe);
        assert!(short.must_attend >= 1);
        assert!(short.advice.starts_with("Attend"));
        // VTOP's 74.5 rounds to 75: safe.
        let s = standing(149, 200, Some(75.0));
        assert!(s.safe && s.must_attend == 0);
        // No percentage from VTOP: the counts decide.
        let s = standing(6, 10, None);
        assert_eq!((s.safe, s.must_attend), (false, 6));
    }

    #[test]
    fn counted_total_drops_unposted_classes() {
        assert_eq!(counted_total(44, 57, Some(79.0)), 56);
        assert_eq!(counted_total(33, 44, Some(77.0)), 43);
        assert_eq!(counted_total(16, 26, Some(67.0)), 24);
        assert_eq!(counted_total(0, 10, Some(0.0)), 10);
        assert_eq!(counted_total(5, 10, Some(99.0)), 10);
        assert_eq!(counted_total(5, 10, None), 10);
        // VTOP rounds up: 31 of 34 (91.2%) shows as 92%, 35 of 41 as 86%.
        assert_eq!(counted_total(31, 34, Some(92.0)), 34);
        assert_eq!(counted_total(35, 41, Some(86.0)), 41);
    }

    #[test]
    fn debar_status_names_the_exam() {
        let debar = Debar::parse("CAT - II : Debarred & Permitted").unwrap();
        assert_eq!(
            (
                debar.exam.as_str(),
                debar.permitted,
                debar.next_exam.as_deref()
            ),
            ("CAT-II", true, Some("FAT"))
        );
        let debar = Debar::parse("FAT : Debarred").unwrap();
        assert_eq!(
            (debar.exam.as_str(), debar.permitted, debar.next_exam),
            ("FAT", false, None)
        );
        assert_eq!(Debar::parse("Debarred").unwrap().exam, "");
        assert_eq!(Debar::parse("-"), None);
    }

    #[test]
    fn plans_count_classes_left_by_the_calendar() {
        use crate::calendar::{SemesterCalendar, tests::fall_2026};
        let calendar = SemesterCalendar::of(&fall_2026());
        let slot = |day, start, kind| ClassSlot {
            day,
            start,
            end: start + 50,
            code: "SWE1002".into(),
            name: String::new(),
            kind,
            room: String::new(),
            block: String::new(),
            slot: String::new(),
            faculty: String::new(),
            credits: None,
        };
        let slots = [
            slot(0, 480, ClassKind::Theory),
            slot(3, 600, ClassKind::Theory),
            slot(1, 660, ClassKind::Lab),
        ];
        // Thursday 2026-11-05, 09:00: Thursday's 10:00 today and next
        // Thursday's, until the last class day (14 Nov); Monday 9 Nov is a
        // holiday. The FAT starts after the last class day: one plan.
        let theory = &courses(vec![record("ETH", "30", "40", "75%")])[0];
        let plans = attendance_plans(theory, &slots, &calendar, 20_762, 540, None);
        assert_eq!(plans.len(), 1);
        let plan = &plans[0];
        assert_eq!((plan.until.as_str(), plan.until_day), ("", 20_772));
        assert_eq!((plan.upcoming, plan.unposted), (2, 0));
        // Today (Thursday 5 Nov) and next Thursday (12 Nov), one class each.
        assert_eq!(
            plan.days,
            vec![
                PlanDay {
                    day: 20_762,
                    classes: 1
                },
                PlanDay {
                    day: 20_769,
                    classes: 1
                },
            ]
        );
        assert_eq!((plan.can_miss, plan.must_attend), (0, Some(2)));
        assert!((plan.best_percent - 32.0 / 42.0 * 100.0).abs() < 1e-9);
        // Labs stop at the Lab FAT; the 2 sessions not posted yet still count.
        let lab = &courses(vec![record("ELA", "22", "32", "79%")])[0];
        let plan = &attendance_plans(lab, &slots, &calendar, 20_762, 540, None)[0];
        assert_eq!(
            (plan.attended, plan.held, plan.upcoming, plan.unposted),
            (11, 14, 0, 2)
        );
        assert_eq!((plan.can_miss, plan.must_attend), (1, Some(1)));
        // Too far below: even attending every class left is not enough.
        let short = &courses(vec![record("ETH", "10", "40", "25%")])[0];
        let plan = &attendance_plans(short, &slots, &calendar, 20_762, 540, None)[0];
        assert_eq!(plan.must_attend, None);
    }

    fn slot_of(code: &str, day: i32, start: i32, kind: ClassKind, name: &str) -> ClassSlot {
        ClassSlot {
            day,
            start,
            end: start + 50,
            code: code.into(),
            name: String::new(),
            kind,
            room: String::new(),
            block: String::new(),
            slot: name.into(),
            faculty: String::new(),
            credits: None,
        }
    }

    #[test]
    fn next_classes_follow_the_calendar() {
        use crate::calendar::{SemesterCalendar, tests::fall_2026};
        let calendar = SemesterCalendar::of(&fall_2026());
        let slots = [
            slot_of("LAB1", 1, 660, ClassKind::Lab, "L1+L2"),
            slot_of("THU1", 3, 600, ClassKind::Theory, "A1"),
            slot_of("MON1", 0, 480, ClassKind::Theory, "B1"),
        ];
        // Thursday 2026-11-05, 09:00 IST.
        let now = 20_762 * 86_400 + 9 * 3_600 - 330 * 60;
        let next = next_classes(&slots, now, 330, Some(&calendar));
        let found = |code: &str| {
            next.iter()
                .find(|class| class.code == code)
                .map(|class| (class.days_ahead, class.start))
        };
        assert_eq!(found("THU1"), Some((0, 600)));
        // Labs stopped at the Lab FAT; Monday 9 Nov is Deepavali and 16 Nov
        // the FAT: no Monday class before the semester ends.
        assert_eq!((found("LAB1"), found("MON1")), (None, None));
        // Without the calendar, the plain week.
        let plain = next_classes(&slots, now, 330, None);
        assert_eq!(
            plain
                .iter()
                .find(|class| class.code == "LAB1")
                .map(|class| class.days_ahead),
            Some(5)
        );
    }

    #[test]
    fn held_classes_take_their_posted_status() {
        let row = |day, slot: &str, start, status| AttendedClass {
            day,
            weekday: 0,
            slot: slot.into(),
            start: Some(start),
            end: None,
            status,
            remark: String::new(),
        };
        let mut days = timetable_days(
            &[
                slot_of("CSE1", 0, 480, ClassKind::Theory, "A1"),
                slot_of("CSE1", 0, 600, ClassKind::Theory, "TA1"),
            ],
            20_759,
            2,
            None,
        );
        let histories = HashMap::from([(
            ("CSE1".to_string(), ClassKind::Theory),
            vec![
                row(20_759, "TA1", 600, Some(ClassStatus::Absent)),
                row(20_759, "A1", 480, Some(ClassStatus::Present)),
            ],
        )]);
        fill_posted(&mut days, &histories);
        let posted: Vec<_> = days[0].classes.iter().map(|class| class.posted).collect();
        assert_eq!(
            posted,
            [Some(ClassStatus::Present), Some(ClassStatus::Absent)]
        );
        assert!(days[1].classes.is_empty());
    }

    // VTOP's summary runs to Tuesday 3 Nov; Wednesday's class was held but is
    // not on VTOP yet: "1+1 left" on Thursday morning.
    #[test]
    fn classes_held_after_the_summary_count_as_not_posted() {
        use crate::calendar::{SemesterCalendar, tests::fall_2026};
        let calendar = SemesterCalendar::of(&fall_2026());
        let slots = [
            slot_of("SWE1002", 2, 480, ClassKind::Theory, "A1"),
            slot_of("SWE1002", 3, 600, ClassKind::Theory, "B1"),
        ];
        let theory = &courses(vec![record("ETH", "30", "40", "75%")])[0];
        let plan = &attendance_plans(theory, &slots, &calendar, 20_762, 540, Some(20_760))[0];
        // Thu 5 Nov 10:00 and Wed 11 / Thu 12 Nov to come.
        assert_eq!((plan.unposted, plan.upcoming), (1, 3));
        // Up to date: nothing held after it.
        let plan = &attendance_plans(theory, &slots, &calendar, 20_762, 540, Some(20_762))[0];
        assert_eq!(plan.unposted, 0);
        // After 10:00 today's class is held too.
        let plan = &attendance_plans(theory, &slots, &calendar, 20_762, 660, Some(20_760))[0];
        assert_eq!((plan.unposted, plan.upcoming), (2, 2));
    }

    #[test]
    fn histories_match_their_summary() {
        let lab = courses(vec![record("ELA", "22", "32", "79%")]).remove(0);
        let class = |status| AttendedClass {
            day: 0,
            weekday: 0,
            slot: String::new(),
            start: None,
            end: None,
            status,
            remark: String::new(),
        };
        let mut classes: Vec<_> = (0..11).map(|_| class(Some(ClassStatus::Present))).collect();
        classes.extend((0..3).map(|_| class(Some(ClassStatus::Absent))));
        classes.extend((0..2).map(|_| class(None)));
        assert!(history_in_sync(&lab, &classes));
        classes.pop();
        assert!(!history_in_sync(&lab, &classes));
        assert_eq!(counted_through(&lab, &classes), None);
    }

    #[test]
    fn history_is_newest_first_with_times() {
        let course = courses(vec![record("ELA", "22", "32", "79%")]).remove(0);
        let row = |date: &str, status: &str| raw::FullAttendanceRecord {
            serial: "1".into(),
            date: date.into(),
            slot: "L10+L11".into(),
            day_time: "WED / 11:00-12:40".into(),
            status: status.into(),
            remark: String::new(),
        };
        let detail = AttendanceDetail::new(
            course,
            FullAttendanceData {
                records: vec![
                    row("12-11-2025", "Present"),
                    row("19-11-2025", "Not Posted"),
                    row("01-Jul-2026", "On Duty"),
                    row("bad", "Absent"),
                ],
                semester_id: "AP2025262".into(),
                update_time: 9,
                course_id: "AM_SWE1002_00200".into(),
                course_type: "ELA".into(),
            },
            Vec::new(),
        );
        let days: Vec<_> = detail.classes.iter().map(|class| class.day).collect();
        // 2026-07-01, 2025-11-19, 2025-11-12.
        assert_eq!(days, [20_635, 20_411, 20_404]);
        assert_eq!(civil_date(20_411.into()), "2025-11-19");
        let last = &detail.classes[2];
        assert_eq!(
            (last.weekday, last.start, last.end),
            (2, Some(660), Some(760))
        );
        assert_eq!(
            detail
                .classes
                .iter()
                .map(|class| class.status)
                .collect::<Vec<_>>(),
            [Some(ClassStatus::OnDuty), None, Some(ClassStatus::Present)]
        );
    }

    #[test]
    fn next_class_per_course() {
        let slot = |day, start, code: &str, kind| ClassSlot {
            day,
            start,
            end: start + 50,
            code: code.into(),
            name: String::new(),
            kind,
            room: String::new(),
            block: String::new(),
            slot: String::new(),
            faculty: String::new(),
            credits: None,
        };
        let slots = [
            slot(0, 480, "CSE2001", ClassKind::Theory),
            slot(2, 600, "CSE2001", ClassKind::Theory),
            slot(0, 840, "CSE2001", ClassKind::Lab),
            slot(1, 540, "MAT1001", ClassKind::Theory),
        ];
        // Monday 2026-11-02, 10:00 IST (04:30 UTC).
        let monday_10am = 1_793_593_800;
        let next = next_classes(&slots, monday_10am, 330, None);
        let found = |code: &str, kind| {
            let class = next
                .iter()
                .find(|c| c.code == code && c.kind == kind)
                .unwrap();
            (class.days_ahead, class.start)
        };
        assert_eq!(found("CSE2001", ClassKind::Lab), (0, 840));
        assert_eq!(found("MAT1001", ClassKind::Theory), (1, 540));
        // Monday's 08:00 has passed, so Wednesday's comes next.
        assert_eq!(found("CSE2001", ClassKind::Theory), (2, 600));
        assert_eq!(next[0].kind, ClassKind::Lab);
        // Only class already over today: same weekday next week.
        let late = next_classes(&slots[..1], monday_10am, 330, None);
        assert_eq!((late[0].days_ahead, late[0].start), (7, 480));
    }

    #[test]
    fn timetable_days_follow_weekdays() {
        let slot = |day, start| ClassSlot {
            day,
            start,
            end: start + 50,
            code: "CSE2001".into(),
            name: String::new(),
            kind: ClassKind::Theory,
            room: String::new(),
            block: String::new(),
            slot: String::new(),
            faculty: String::new(),
            credits: None,
        };
        let slots = [slot(0, 480), slot(0, 600), slot(2, 540)];
        // Epoch day 20759 is Monday 2026-11-02.
        let days = timetable_days(&slots, 20_759, 3, None);
        assert_eq!(
            days.iter().map(|day| day.day).collect::<Vec<_>>(),
            [20_759, 20_760, 20_761]
        );
        assert_eq!(days[0].classes.len(), 2);
        assert!(days[0].holds_theory && !days[1].holds_theory);
        assert_eq!(days[2].classes[0].slot.start, 540);
        assert_eq!(days[2].classes[0].state, ClassDay::Held);
    }

    fn class(day: i32, start: i32, code: &str, kind: ClassKind) -> ClassSlot {
        ClassSlot {
            day,
            start,
            end: start + 50,
            code: code.into(),
            name: String::new(),
            kind,
            room: String::new(),
            block: String::new(),
            slot: String::new(),
            faculty: String::new(),
            credits: None,
        }
    }

    #[test]
    fn the_calendar_shapes_timetable_days() {
        use crate::calendar::{SemesterCalendar, tests::fall_2026};
        let slots = [
            class(1, 600, "CSE1001", ClassKind::Lab),
            class(3, 480, "CSE1001", ClassKind::Theory),
            class(3, 840, "CSE1001", ClassKind::Lab),
            class(4, 480, "MAT1001", ClassKind::Theory),
            class(5, 600, "MAT1001", ClassKind::Theory),
        ];
        let calendar = SemesterCalendar::of(&fall_2026());
        let monday = epoch_day("2026-11-02").unwrap();
        let days = timetable_days(&slots, monday, 15, Some(&calendar));
        let on = |date: &str| &days[(epoch_day(date).unwrap() - monday) as usize];
        let states = |date: &str| on(date).classes.iter().map(|c| c.state).collect::<Vec<_>>();

        let mon = on("2026-11-02");
        assert_eq!(
            (mon.mark, mon.title.as_deref(), mon.holds_theory),
            (
                Some(CalendarMark::NoClasses),
                Some("No instructional day"),
                false
            )
        );
        // Lab FAT week: the course's first lab of it is the exam, its later
        // one that week does not meet; theory carries on.
        assert_eq!(states("2026-11-03"), [ClassDay::LabFat]);
        assert_eq!(states("2026-11-05"), [ClassDay::Held, ClassDay::Off]);
        assert_eq!(on("2026-11-05").mark, Some(CalendarMark::LabFat));
        assert!(on("2026-11-05").holds_theory);
        // Deepavali: no classes.
        let sat = on("2026-11-07");
        assert_eq!(states("2026-11-07"), [ClassDay::Off]);
        assert_eq!(
            (sat.mark, sat.title.as_deref(), sat.holds_theory),
            (Some(CalendarMark::Holiday), Some("Deepavali"), false)
        );
        // After the Lab FAT, labs stop.
        assert_eq!(states("2026-11-12"), [ClassDay::Held, ClassDay::Off]);
        // Fri 6, Thu 12, Fri 13 and Sat 14 still have classes.
        assert_eq!(on("2026-11-05").class_days_left, 4);
        assert_eq!(on("2026-11-14").class_days_left, 0);
        assert!(on("2026-11-14").last_class_day && !on("2026-11-13").last_class_day);
        assert!(on("2026-11-12").classes[0].last_class);
        assert!(!on("2026-11-06").classes[0].last_class && on("2026-11-14").classes[0].last_class);
        assert_eq!(
            on("2026-11-05").upcoming_exam,
            Some(ExamStart {
                name: "FAT".into(),
                day: epoch_day("2026-11-16").unwrap()
            })
        );
        let exam = on("2026-11-16");
        assert_eq!(
            (exam.mark, exam.title.as_deref()),
            (Some(CalendarMark::Exam), Some("FAT"))
        );
        assert!(exam.classes.is_empty() && !exam.holds_theory);
    }

    #[test]
    fn a_working_day_follows_the_weekday_it_names() {
        use crate::calendar::{SemesterCalendar, tests::entry};
        let data = raw::AcademicCalendarData {
            entries: vec![
                entry("2026-11-13", "Instructional Day", "WorkingDay"),
                entry("2026-11-14", "Instructional Day", "Tuesday Day Order"),
                entry("2026-11-15", "Holiday", "Holiday"),
            ],
            semester_id: "S".into(),
            update_time: 1,
        };
        let calendar = SemesterCalendar::of(&data);
        let slots = [
            class(1, 540, "PHY1001", ClassKind::Theory),
            class(5, 600, "MAT1001", ClassKind::Theory),
        ];
        let saturday = epoch_day("2026-11-14").unwrap();
        let days = timetable_days(&slots, saturday, 1, Some(&calendar));
        assert_eq!(days[0].classes.len(), 1);
        assert_eq!(days[0].classes[0].slot.code, "PHY1001");
        assert_eq!(days[0].classes[0].state, ClassDay::Held);
        assert_eq!(days[0].title.as_deref(), Some("Tuesday Day Order"));
        // Outside the listed days the plain timetable applies.
        let later = timetable_days(&slots, saturday + 7, 1, Some(&calendar));
        assert_eq!(later[0].classes[0].slot.code, "MAT1001");
        assert_eq!(later[0].mark, None);
    }

    #[test]
    fn epoch_days_round_trip() {
        for day in [-1, 0, 20_759, 20_762, 11_016] {
            assert_eq!(epoch_day(&civil_date(i64::from(day))), Some(day));
        }
        assert_eq!(epoch_day("2026-11-02"), Some(20_759));
        assert_eq!(epoch_day("2026-13-02"), None);
        assert_eq!(weekday(20_759), 0);
    }

    #[test]
    fn labs_merge_and_slots_sort() {
        let slot = |day: &str, slot: &str, start: &str, end: &str, kind| raw::TimetableSlot {
            serial: "1".into(),
            day: day.into(),
            slot: slot.into(),
            course_code: "CSE2001".into(),
            course_type: "ELA".into(),
            room_no: "107".into(),
            block: "AB1".into(),
            start_time: start.into(),
            end_time: end.into(),
            name: "Data Structures".into(),
            kind,
            faculty: "Dr. Test - SCOPE".into(),
            credits: "1.0".into(),
        };
        let data = TimetableData {
            slots: vec![
                slot("TUE", "A1", "08:00", "08:50", raw::ClassKind::Theory),
                slot("MON", "L2", "14:50", "15:40", raw::ClassKind::Lab),
                slot("MON", "L1", "14:00", "14:50", raw::ClassKind::Lab),
            ],
            courses: vec![],
            semester_id: "AP2026".into(),
            update_time: 1,
        };
        let slots = Timetable::from(data).slots;
        assert_eq!(slots.len(), 2);
        assert_eq!((slots[0].day, slots[0].start, slots[0].end), (0, 840, 940));
        assert_eq!(slots[0].slot, "L1+L2");
        assert_eq!((slots[1].day, slots[1].credits), (1, Some(1.0)));
    }
}

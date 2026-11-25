//! Ready-to-draw records built from vtop-core's raw tables.

use vtop_core::dates;
use vtop_core::types as raw;

mod attendance;
mod timetable;

pub use attendance::{attendance_goal_split, project_attendance};
pub(crate) use attendance::{
    attendance_plans, counted_through, fill_posted, history_day, history_in_sync, longest_run,
};
pub(crate) use timetable::{next_classes, timetable_days};

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
        let (season, year, note) = match words.as_slice() {
            [season, kind, year, note @ ..]
                if kind.eq_ignore_ascii_case("semester")
                    && year.len() >= 7
                    && year.chars().take(4).all(|c| c.is_ascii_digit())
                    && year.as_bytes().get(4) == Some(&b'-') =>
            {
                let mut season = season.to_lowercase();
                if let Some(first) = season.get_mut(0..1) {
                    first.make_ascii_uppercase();
                }
                (season, year.to_string(), note.join(" "))
            }
            _ => (name.trim().to_string(), String::new(), String::new()),
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
    /// Attended, held and not-posted counts in sessions (labs halved, matching
    /// VTOP's percentage and [`AttendancePlan`]).
    pub sessions_attended: i32,
    pub sessions_held: i32,
    pub sessions_unposted: i32,
    /// VTOP's rounded figure when it sends one, else computed.
    pub percent: f64,
    pub safe: bool,
    pub can_skip: i32,
    pub must_attend: i32,
    /// "Can skip 3", "Attend 2 labs", "Don't skip" or "No classes yet".
    pub advice: String,
    pub debarred: bool,
    /// What VTOP's debar status says, when it debars.
    pub debar: Option<Debar>,
    /// The debarment in one line: "Debarred for CAT-II · Permitted · Clear
    /// for FAT", or "… · Clear for FAT at 3 more" while below 75%.
    pub debar_line: Option<String>,
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

/// One course's class-by-class attendance.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct AttendanceDetail {
    /// The course as the saved attendance has it.
    pub course: AttendanceCourse,
    /// Unix seconds.
    pub updated_at: i64,
    /// Newest first; a lab session is one class.
    pub classes: Vec<AttendedClass>,
    /// Posted classes by status.
    pub present: i32,
    pub on_duty: i32,
    pub absent: i32,
    /// Present or on duty in a row, back from the newest posted class;
    /// only an absence breaks it, and a cancelled class is skipped.
    pub streak: i32,
    /// The day of the newest absence.
    pub last_missed: Option<i32>,
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
    /// The exam's name ("CAT-II", "FAT"), or none for the semester's end.
    pub until: Option<String>,
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

/// Projected attendance after hypothetical attended, skipped and on-duty classes.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct AttendanceProjection {
    /// Exact percentage before any hypothetical changes (0 when no classes).
    pub base_percent: f64,
    /// Exact percentage after the hypothetical changes (0 when no classes).
    pub percent: f64,
    /// Whether the projected standing meets the 75% threshold.
    pub safe: bool,
    /// Projected attended count (in sessions for labs).
    pub attended: i32,
    /// Projected held count (in sessions for labs).
    pub total: i32,
}

/// How remaining classes split into must-attend and can-miss for a target percentage.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GoalSplit {
    /// Classes that must be attended to reach the goal; none when attending all still falls short.
    pub must_attend: Option<i32>,
    /// Classes that can be missed while still reaching the goal.
    pub can_miss: i32,
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
        .and_then(|i| raw.get(..i))
        .unwrap_or(raw)
        .trim();
    if name != name.to_uppercase() {
        return name.to_string();
    }
    let word = |w: &str| match w.chars().next() {
        Some(first) if w.chars().count() > 1 && !w.contains('.') => {
            format!(
                "{first}{}",
                w.get(first.len_utf8()..).unwrap_or_default().to_lowercase()
            )
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
/// Today in India, days since 1970-01-01.
pub(crate) fn today_ist_day() -> i32 {
    dates::today_in_india() as i32
}

/// 0 = Monday, for days since 1970-01-01 (a Thursday).
pub(crate) fn weekday(day: i32) -> i32 {
    (day + 3).rem_euclid(7)
}

/// Days since 1970-01-01 for a `YYYY-MM-DD`.
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
    Some(dates::day_number(day, month, year) as i32)
}

/// `YYYY-MM-DD` for days since 1970-01-01.
pub(crate) fn civil_date(days: i64) -> String {
    let (year, month, day) = dates::civil_date(days);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests;

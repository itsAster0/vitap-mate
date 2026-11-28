//! Semester Wrapped: every fact of the end-of-semester story, from the selected semester's
//! cached pages. A fact without data is none (or empty), and the story skips its slide. Logs
//! say which facts exist, never their values.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;

use tracing::info;
use vtop_core::types::{
    AcademicCalendarData, AttendanceData, GeneralOutingData, SemesterData, TimetableData,
    WeekendOutingData,
};

use crate::pages::biometric::{BiometricDay, Punch, PunchKind};
use crate::pages::calendar::{SemesterCalendar, exam_name};
use crate::pages::exams::Exams;
use crate::pages::grades::{
    GradeCount, GradeHistory, GradeSession, Grades, counts, grade_points, semester_summary,
};
use crate::pages::marks::{Assessment, Marks};
use crate::pages::outing::{self, OutingStanding};
use crate::pages::records::{
    self, Attendance, AttendanceCourse, AttendedClass, ClassKind, ClassSlot, ClassStatus, Timetable,
};
use crate::{
    ATTENDANCE, CALENDAR, Core, CoreError, PAGES, SEMESTERS, TIMETABLE, traced, traced_sync,
};

mod campus;
mod classes;
mod days;
mod ffcs;
mod mixtape;
mod results;
mod titles;

use campus::*;
use classes::*;
use days::*;
use ffcs::*;
use mixtape::*;
use results::*;
use titles::*;

/// Every fact of the selected semester's story; days are days since 1970-01-01 in India.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedReport {
    pub semester_id: String,
    pub semester_name: String,
    pub first_day: i32,
    /// The last "Instructional Day" in the calendar.
    pub last_day: i32,
    /// The calendar's last entry: the outing window's end.
    pub window_end: i32,
    pub courses: i32,
    pub class_day_count: i32,
    pub holidays: i32,
    /// Days from `first_day` to `window_end`, both counted.
    pub span_days: i32,
    /// The semester's course codes, sorted, for the opening.
    pub course_codes: Vec<String>,
    pub attendance: Option<WrappedAttendance>,
    pub mosaic: Option<WrappedMosaic>,
    pub arc: Option<WrappedArc>,
    pub on_duty: Option<WrappedOnDuty>,
    pub streak: Option<WrappedStreak>,
    /// Course names, badged on the Courses slide.
    pub never_missed: Vec<String>,
    pub comeback: Option<WrappedComeback>,
    pub close_call: Option<WrappedCloseCall>,
    pub traps: Option<WrappedTraps>,
    pub habits: Option<WrappedHabits>,
    pub day_half: Option<WrappedDayHalf>,
    pub time: Option<WrappedTime>,
    pub places: Option<WrappedPlaces>,
    /// None in a first semester (no FFCS yet) or without a timetable.
    pub ffcs: Option<WrappedFfcs>,
    pub marks: Option<WrappedMarks>,
    pub full_marks: Option<WrappedFullMarks>,
    pub assessments: Option<WrappedAssessments>,
    pub exams: Option<WrappedExams>,
    pub crunch: Option<WrappedCrunch>,
    pub grades: Option<WrappedGrades>,
    pub cgpa: Option<WrappedCgpa>,
    pub degree: Option<WrappedDegree>,
    pub roi: Option<WrappedRoi>,
    pub outings: Option<WrappedOutings>,
    pub campus: Option<WrappedCampus>,
    pub hostel: Option<WrappedHostel>,
    pub moves: Option<WrappedMoves>,
    pub previous: Option<WrappedPrevious>,
    /// Every title the facts earn, the strongest first, ending with The Steady Cruiser; the
    /// title is the first one the share's privacy choices leave.
    pub traits: Vec<WrappedTrait>,
    pub title: WrappedTitle,
    /// The title when Hide grades is on.
    pub title_without_grades: WrappedTitle,
    /// Picked from the facts above; none skips the slide.
    pub mixtape: Option<WrappedMixtape>,
    /// The mixtape when Hide grades is on.
    pub mixtape_without_grades: Option<WrappedMixtape>,
    /// The mixtape when Hide hostel and outings is on, and when both are.
    pub mixtape_without_hostel: Option<WrappedMixtape>,
    pub mixtape_without_either: Option<WrappedMixtape>,
    /// Every instructional day, for the campus fetch.
    pub class_days: Vec<i32>,
}

/// Classes in VTOP's units: a lab session is two.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedAttendance {
    /// Attended over counted, summed across courses, rounded up as VTOP does.
    pub percent: i32,
    pub attended: i32,
    pub counted: i32,
    pub on_duty: i32,
    pub best: String,
    pub best_percent: i32,
    pub least: String,
    pub least_percent: i32,
    /// The best-attended courses, up to 3, best first; never the least-attended one.
    pub top: Vec<WrappedCourse>,
    /// Four ranges for the guess before the reveal, lowest first, one holding `percent`.
    pub guesses: Vec<WrappedGuess>,
    /// Each course part (theory, lab) with a counted class, in VTOP's order.
    pub lines: Vec<WrappedCourseLine>,
}

/// A guess's range of attendance percent.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedGuess {
    pub from: i32,
    /// Inclusive; none for the top range ("90%+"), which runs to 100.
    pub to: Option<i32>,
    /// Points from the range to the real percent: 0 inside it, more than 0 when the real one is
    /// higher (undersold), less when lower.
    pub off: i32,
}

/// A course part's attendance, in VTOP's units (a lab session is two).
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedCourseLine {
    pub name: String,
    pub lab: bool,
    pub attended: i32,
    /// Rounded up as VTOP does.
    pub percent: i32,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedCourse {
    pub name: String,
    pub percent: i32,
}

/// The semester a day at a time, Monday to Sunday rows.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedMosaic {
    /// The Monday on or before the first class day: `days`' first.
    pub first_day: i32,
    /// To the Sunday on or after the window's end; a multiple of 7 long.
    pub days: Vec<WrappedDay>,
    /// Calendar months with a day in the window, in order.
    pub months: Vec<WrappedMonth>,
    /// Indices into `months`, among months with [`MONTH_MIN`] counted classes; none
    /// with fewer than two such months, or when they tie.
    pub best_month: Option<i32>,
    pub worst_month: Option<i32>,
    /// Days with a counted class.
    pub class_days: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Enum)]
pub enum WrappedDay {
    /// Before the first class day or after the window.
    #[default]
    Outside,
    /// A named day off (a festival), without a class.
    Holiday,
    /// An exam day, without a class.
    Exam,
    /// Another day without a counted class.
    NoClass,
    AllPresent,
    /// Every class attended, one or more on duty.
    OnDuty,
    /// Some classes missed.
    Mixed,
    AllAbsent,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedMonth {
    /// 1 = January.
    pub month: i32,
    /// Its first and last day inside the window.
    pub first_day: i32,
    pub last_day: i32,
    /// Classes in VTOP's units (a lab session is two).
    pub counted: i32,
    /// Rounded up as VTOP does; none without a counted class.
    pub percent: Option<i32>,
}

/// Pooled attendance over the semester, as it ran.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedArc {
    /// At the end of each class day from the [`ARC_FROM`]th counted class on, thinned to about
    /// [`ARC_POINTS`], keeping the peak and the low; in day order.
    pub points: Vec<WrappedPoint>,
    /// The calendar's exams inside the window, by first day.
    pub exams: Vec<WrappedSpan>,
    /// Between the CATs; empty without one in the calendar.
    pub phases: Vec<WrappedPhase>,
    pub shape: WrappedArcShape,
    /// The highest and lowest points of the whole line, the earliest of equals.
    pub peak: WrappedPoint,
    pub low: WrappedPoint,
    /// The largest fall from a point to a later one, of [`ARC_DROP`] points or more.
    pub drop: Option<WrappedDrop>,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedPoint {
    pub day: i32,
    /// Exact, not rounded.
    pub percent: f64,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedSpan {
    /// "CAT-I", "CAT-II", "FAT".
    pub name: String,
    pub first_day: i32,
    pub last_day: i32,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedPhase {
    /// The exam it follows ("CAT-I"); none for the first.
    pub after: Option<String>,
    /// The exam it leads to; none for the last.
    pub before: Option<String>,
    /// Classes inside it only, rounded up as VTOP does.
    pub percent: i32,
    /// Lower than every other phase.
    pub lowest: bool,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedDrop {
    pub from: WrappedPoint,
    pub to: WrappedPoint,
}

/// The run of the line, by [`COMEBACK_POINTS`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Enum)]
pub enum WrappedArcShape {
    /// Fell, then came back up.
    Comeback,
    /// Ended lower than it started.
    Slide,
    /// Ended higher than it started.
    Climb,
    #[default]
    Steady,
}

/// Classes in VTOP's units (a lab session is two).
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedOnDuty {
    pub classes: i32,
    /// Days with a class on duty.
    pub days: i32,
    pub present: i32,
    pub absent: i32,
    /// The course with the most classes on duty, and one that ties it; `percent` is the count.
    pub top: Vec<WrappedCourse>,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedStreak {
    pub length: i32,
    pub first_day: i32,
    pub last_day: i32,
    pub perfect_weeks: i32,
    /// Posted classes before the run, in time order: where it starts among [`Self::total`].
    pub offset: i32,
    /// Posted classes (present, on duty or absent), the classes the run is counted over.
    pub total: i32,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedComeback {
    pub low_percent: i32,
    pub low_day: i32,
    pub final_percent: i32,
}

/// The moment a course came closest to 75%, in VTOP's units (a lab session is two).
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedCloseCall {
    pub course: String,
    /// The class that set the moment.
    pub day: i32,
    /// Further absences that keep the course at 75% or more; none when under 75.
    pub spare: Option<i32>,
    pub percent: i32,
    /// After the course's last counted class.
    pub final_percent: i32,
}

/// Absences VTOP put down to a missing punch ("No punch found during class hours between faculty
/// punch."); classes counted once, a lab session too.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedTraps {
    pub trapped: i32,
    /// Every absence.
    pub absences: i32,
    /// Of `trapped`, on days with a cached biometric copy, and of those, with a class punch from
    /// [`PUNCH_EARLY`] minutes before the class to its end.
    pub checked: i32,
    pub punched: i32,
    /// The courses with the most, up to 3, most first; `percent` is the count.
    pub by_course: Vec<WrappedCourse>,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedHabits {
    /// 0 = Monday; none without an absence.
    pub worst_weekday: Option<i32>,
    pub worst_weekday_misses: i32,
    pub misses: i32,
    /// Over classes starting before 9:00.
    pub early_percent: Option<i32>,
    pub early_classes: i32,
    /// Of those, attended.
    pub early_attended: i32,
    pub first_slot_days: i32,
    pub last_slot_days: i32,
    /// Absences by weekday, Monday first; 7 long.
    pub misses_by_weekday: Vec<i32>,
    /// Attended classes (a lab session once) on an average class day of each weekday, in
    /// tenths, Monday first; 7 long, 0 for a weekday without [`QUIET_MIN_DAYS`] class days.
    pub classes_per_day_tenths: Vec<i32>,
    /// 0 = Monday: the fewest of those; none with fewer than two such weekdays.
    pub quietest_weekday: Option<i32>,
}

/// Attended classes with a start time, a lab session once: before noon or from 2 PM.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedDayHalf {
    /// Before noon is the side with [`DAY_HALF_PERCENT`]; false: from 2 PM.
    pub morning: bool,
    /// That side's share, rounded down.
    pub percent: i32,
    pub classes: i32,
    pub total: i32,
    /// Classes starting in each hour from 8 to 18; 11 long.
    pub by_hour: Vec<i32>,
}

/// Attended classes; a lab session is one class of 100 minutes.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedTime {
    pub classes: i32,
    pub minutes: i32,
    pub lab_minutes: i32,
    /// 0 = Monday.
    pub busiest_weekday: i32,
    /// Attended classes by weekday in VTOP's units (a lab session is two), Monday first; 7 long.
    pub classes_by_weekday: Vec<i32>,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedPlaces {
    /// "AB1-205".
    pub room: Option<String>,
    pub room_minutes: i32,
    pub faculty: Option<String>,
    /// Attended class time with that faculty; a lab session is 100 minutes.
    pub faculty_minutes: i32,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedMarks {
    pub best_cat: Option<WrappedCat>,
    pub best_course: Option<String>,
    pub best_course_percent: i32,
    /// The average CAT-I to CAT-II change in points.
    pub cat_change: Option<i32>,
    /// The course that rose the most from CAT-I to CAT-II, whatever the average did.
    pub most_improved: Option<WrappedRise>,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedRise {
    pub course: String,
    /// Percentage points, more than 0.
    pub points: i32,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedCat {
    pub course: String,
    /// As named: "CAT" (a semester's only one), "CAT-I", "CAT-II".
    pub exam: String,
    pub percent: i32,
}

/// Posted assessments scored at their max marks, re-evaluations left out.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedFullMarks {
    pub count: i32,
    /// Posted assessments.
    pub of: i32,
    /// Most first.
    pub by_kind: Vec<WrappedKindCount>,
    /// The course with the most; none when that is one.
    pub top_course: Option<String>,
    pub top_count: i32,
}

/// Posted assessments, re-evaluations left out.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedAssessments {
    pub total: i32,
    /// Most first, ties in [`KINDS`] order.
    pub by_kind: Vec<WrappedKindCount>,
}

/// Assessments of a kind of [`KINDS`] ("quizzes").
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedKindCount {
    pub kind: String,
    pub count: i32,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedExams {
    pub cats: i32,
    pub fats: i32,
    pub top_venue: Option<String>,
    /// Time in exam halls: each paper's start to end, else [`CAT_MINUTES`] or [`FAT_MINUTES`].
    pub minutes: i32,
}

/// The 7 days from a paper's day with the most papers sat.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedCrunch {
    pub papers: i32,
    pub first_day: i32,
    /// `first_day` + 6.
    pub last_day: i32,
    /// Papers on each day of the window, `first_day` first; 7 long.
    pub by_day: Vec<i32>,
    /// The most papers sat on one day, and the earliest such day.
    pub most_in_a_day: i32,
    pub most_day: i32,
    /// Days with a paper sat.
    pub exam_days: i32,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedGrades {
    pub gpa: f64,
    pub mix: Vec<GradeCount>,
    pub s_count: i32,
    /// More S grades than any earlier semester in the grade history.
    pub s_record: bool,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedCgpa {
    pub before: Option<f64>,
    pub after: f64,
    pub credits_semester: f64,
    pub credits_total: f64,
}

/// The grade history up to this semester.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedDegree {
    /// Graded semesters, oldest first; this one is `current`.
    pub semesters: Vec<WrappedSemesterGpa>,
    /// This semester's GPA among them: 1 is the best (ties share it).
    pub rank: i32,
    /// Credits earned up to this semester.
    pub credits: f64,
    /// Those credits by course distribution, most first, none empty.
    pub by_distribution: Vec<WrappedCredits>,
    /// The CGPA after this semester.
    pub cgpa: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedSemesterGpa {
    /// "Nov '25", as the grade history's trend.
    pub short: String,
    pub gpa: f64,
    pub current: bool,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedCredits {
    /// "Core" (programme core), "Univ" (university core), "Elect" (electives), else VTOP's code.
    pub label: String,
    pub credits: f64,
}

/// Attendance against score per course (theory and lab pooled by code).
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedRoi {
    pub points: Vec<WrappedRoiPoint>,
    pub source: WrappedScoreSource,
    pub verdict: WrappedRoiVerdict,
    /// Indices into `points`: the most score over attendance, and the most attendance over
    /// score (another course, at least [`GRIND_GAP`] points apart).
    pub best: i32,
    pub grind: Option<i32>,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedRoiPoint {
    pub course: String,
    /// Percent, rounded up as VTOP does.
    pub attendance: i32,
    /// Out of 100.
    pub score: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Enum)]
pub enum WrappedScoreSource {
    /// The grand total on the grades page.
    Total,
    /// Weighted marks so far.
    #[default]
    Marks,
}

/// From the correlation of attendance and score.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Enum)]
pub enum WrappedRoiVerdict {
    ShowedUpPaidOff,
    BunkedAndAced,
    #[default]
    NoPattern,
}

/// The semester as a five-track tape, picked by [`mixtape::RULES`].
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct WrappedMixtape {
    /// "The Comeback Cut".
    pub album_title: String,
    /// "Lab-Coat Ambient".
    pub genre: String,
    /// "FALL '26", on the cover.
    pub stamp: String,
    /// The tracks' lengths summed, "m:ss".
    pub total_duration: String,
    pub total_secs: i32,
    /// From the student and semester, for the cover's shapes: the same tape every time.
    pub cover_seed: u64,
    pub cover_tone: WrappedMixtapeTone,
    /// A run of [`mixtape::NO_SKIPS`] classes or more: "No skips" beside the runtime.
    pub no_skips: bool,
    /// Five, in play order.
    pub tracks: Vec<WrappedTrack>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct WrappedTrack {
    /// 1 to 5.
    pub number: i32,
    pub slot: WrappedTrackSlot,
    pub title: String,
    /// The fact behind it: "8 AM classes: 64%".
    pub stat_line: String,
    /// "6:04": `duration_secs` as "m:ss".
    pub duration: String,
    pub duration_secs: i32,
}

/// Where a track's rule belongs on the tape.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Enum)]
pub enum WrappedTrackSlot {
    #[default]
    Opener,
    Attendance,
    Grind,
    Campus,
    Exams,
    Wildcard,
    Results,
    Closer,
}

/// The cover's lead colour.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Enum)]
pub enum WrappedMixtapeTone {
    #[default]
    Warning,
    Accent,
    Success,
    Lab,
    Danger,
}

/// Approved requests starting inside the window.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedOutings {
    pub leaves: i32,
    pub weekends: i32,
    pub top_place: Option<String>,
    pub longest_leave_days: Option<i32>,
    pub days_away: i32,
    pub longest_stay_days: i32,
}

/// From the class days with a cached biometric copy.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedCampus {
    pub days_cached: i32,
    pub days_total: i32,
    /// "7:12 AM".
    pub earliest_entry: Option<String>,
    pub latest_return: Option<String>,
    pub days_on_campus: i32,
}

/// A hosteller's class days, from hostel gate punches (`MH2-FACE-OUT-3`): out is each OUT to the
/// next IN that day.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedHostel {
    /// Class days with an OUT and an IN after it.
    pub days: i32,
    /// Hours out on an average one of those days, in tenths (94 is 9.4 hours).
    pub out_tenths: i32,
    /// Half hours from 6:00 to 24:00, each out on at least half of `days`; 36 long.
    pub out_by_half_hour: Vec<bool>,
    /// Trips back to the hostel between two class punches.
    pub naps: i32,
    /// 0 = Monday, with the most naps; none without a nap.
    pub nap_weekday: Option<i32>,
}

/// From class punches (`CB-214-FACE`), in time order each class day.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedMoves {
    /// Each day's first floor and every rise after it; a room in another block counts from
    /// that block's ground floor.
    pub floors: i32,
    /// Distinct block and room.
    pub rooms: i32,
    /// The most block changes in a day, and the earliest such day.
    pub most_hops: i32,
    pub most_hops_day: i32,
    /// Up to 3, most rooms first, then by name.
    pub blocks: Vec<WrappedBlock>,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedBlock {
    pub block: String,
    pub rooms: i32,
}

/// The week's timetable graded: four parts of 25 points, each from one fact.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedFfcs {
    /// 0 to 100, the parts' sum.
    pub score: i32,
    pub grade: WrappedGrade,
    /// The part with the most points and the one with the fewest (the first of equals).
    pub best: WrappedFfcsPart,
    pub worst: WrappedFfcsPart,
    /// Monday to Saturday without a class, 0 = Monday.
    pub free_days: Vec<i32>,
    /// Classes starting before 9:00 in a week.
    pub early: i32,
    /// Minutes a week waiting between classes, the lunch break left out.
    pub gap_minutes: i32,
    /// Class days with nothing from 12:30 to 14:00, of `class_days`.
    pub lunch_free_days: i32,
    pub class_days: i32,
    /// Back-to-back classes in different blocks a week, and the pair seen most.
    pub sprints: i32,
    pub sprint_from: Option<String>,
    pub sprint_to: Option<String>,
    /// The week drawn: each class once (a lab session once), by day then start.
    pub week: Vec<WrappedWeekSlot>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Enum)]
pub enum WrappedGrade {
    S,
    A,
    B,
    #[default]
    C,
    D,
    E,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Enum)]
pub enum WrappedFfcsPart {
    /// Fewer 8 AMs.
    #[default]
    Early,
    /// Less waiting between classes.
    Gaps,
    /// Lunch kept free.
    Lunch,
    /// Days without a class.
    FreeDays,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedWeekSlot {
    /// 0 = Monday.
    pub day: i32,
    pub start: i32,
    pub end: i32,
    pub lab: bool,
}

/// A title the facts earn and how strongly, 1 to 100.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedTrait {
    pub title: WrappedTitle,
    pub score: i32,
    /// Left out with Hide grades, and with Hide hostel and outings.
    pub grades: bool,
    pub hostel: bool,
}

/// Against the next older semester; only the halves that improved.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedPrevious {
    pub semester_name: String,
    pub attendance_change: Option<i32>,
    pub gpa_change: Option<f64>,
}

/// In [`titles::RULES`] order, which breaks ties.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Enum)]
pub enum WrappedTitle {
    Topper,
    GhostGenius,
    ExamClutch,
    Perfectionist,
    Comeback,
    EdgeWalker,
    IronStreak,
    EarlyRegular,
    Regular,
    ClubCaptain,
    LabRat,
    Explorer,
    Stairmaster,
    NightOwl,
    /// Nothing else matched.
    #[default]
    SteadyCruiser,
}

#[uniffi::export]
impl Core {
    /// The selected semester's story; none until its classes are over (today after the last
    /// instructional day) or without a calendar. No network.
    pub fn cached_wrapped(&self, flow: String) -> Option<WrappedReport> {
        traced_sync("cached_wrapped", &flow, || {
            Ok(self.wrapped(&flow, records::today_ist_day()))
        })
        .ok()
        .flatten()
    }

    /// The selected semester's class days up to today without a complete cached biometric
    /// copy. No network.
    pub fn wrapped_missing_days(&self, flow: String) -> Vec<i32> {
        traced_sync("wrapped_missing_days", &flow, || {
            let today = records::today_ist_day();
            let missing: Vec<i32> = self
                .class_days()
                .into_iter()
                .filter(|&day| day <= today)
                .filter(|&day| {
                    self.cached_day(day)
                        .is_none_or(|data| !BiometricDay::new(day, &data).complete)
                })
                .collect();
            info!(missing = missing.len(), "wrapped missing days");
            Ok(missing)
        })
        .unwrap_or_default()
    }

    /// Fetches the next older semester's attendance into its own pages folder when it isn't
    /// cached: one VTOP request, none without an older semester.
    pub async fn fetch_wrapped_previous(&self, flow: String) -> Result<(), CoreError> {
        traced("fetch_wrapped_previous", &flow, async {
            let Some((previous, _)) = self
                .read_kept::<SemesterData>(SEMESTERS)
                .and_then(|list| self.previous_semester(&list))
            else {
                info!("no older semester");
                return Ok(());
            };
            let path = format!("{PAGES}/{previous}/{ATTENDANCE}");
            if self.read_kept::<AttendanceData>(&path).is_some() {
                info!("older semester's attendance cached");
                return Ok(());
            }
            let data = self
                .page(&vtop_core::spec::ATTENDANCE, Some(&previous), &[])
                .await?;
            if let Some(parent) = self.at(&path).parent() {
                #[expect(
                    clippy::let_underscore_must_use,
                    reason = "a failed mkdir shows up in the write that follows"
                )]
                let _ = fs::create_dir_all(parent);
            }
            self.write(&path, &data);
            info!(courses = data.records.len(), "older semester's attendance");
            Ok(())
        })
        .await
    }
}

/// A class of a course, from the course's cached history.
type Taken<'a> = (&'a AttendanceCourse, AttendedClass);

impl Core {
    fn wrapped(&self, flow: &str, today: i32) -> Option<WrappedReport> {
        let list = self.read_kept::<SemesterData>(SEMESTERS)?;
        let semester_id = self.pick(&list)?;
        let calendar = SemesterCalendar::of(&self.read_page::<AcademicCalendarData>(CALENDAR)?);
        let (Some(first_day), Some(last_day)) =
            (calendar.first_class_day(), calendar.last_class_day())
        else {
            info!("no class days in the calendar");
            return None;
        };
        // From the second day after the last class: its attendance is usually posted by then.
        if today <= last_day + 1 {
            info!(days_left = last_day + 1 - today, "classes not over");
            return None;
        }
        let window_end = calendar.last_listed_day().unwrap_or(last_day);
        let class_days = calendar.class_days();
        let courses: Vec<AttendanceCourse> = self
            .read_page::<AttendanceData>(ATTENDANCE)
            .map_or_default(|data| Attendance::from(data).courses);
        let classes: Vec<Taken> = courses
            .iter()
            .flat_map(|course| {
                self.cached_history(course)
                    .unwrap_or_default()
                    .into_iter()
                    .map(move |class| (course, class))
            })
            .collect();
        let timetable = self
            .read_page::<TimetableData>(TIMETABLE)
            .map(Timetable::from);
        let slots = timetable.as_ref().map_or(&[][..], |t| t.slots.as_slice());
        let mut codes: HashSet<&str> = courses.iter().map(|c| c.code.as_str()).collect();
        if codes.is_empty() {
            codes = slots.iter().map(|slot| slot.code.as_str()).collect();
        }
        let history = self.cached_grade_history(flow.to_string());
        let own_grades = self.cached_grades(flow.to_string(), semester_id.clone());
        // This semester's exam session in the grade history, and the one before it. Without its
        // own grades page (an old semester), its courses come from attendance or the timetable.
        let (this, before) = history.as_ref().map_or_default(|history| {
            let own = own_grades
                .as_ref()
                .map(grade_codes)
                .filter(|own| !own.is_empty())
                .unwrap_or_else(|| codes.iter().map(|c| c.trim().to_uppercase()).collect());
            sessions(history, &own, first_day, last_day)
        });
        let grades = grades(own_grades.as_ref(), history.as_ref(), this);
        let cgpa = history
            .as_ref()
            .zip(this)
            .and_then(|(history, this)| cgpa(history, this));
        let previous = self.previous_semester(&list).and_then(|(id, name)| {
            let before_courses = self
                .read_kept::<AttendanceData>(&format!("{PAGES}/{id}/{ATTENDANCE}"))
                .map(|data| Attendance::from(data).courses);
            let gpa_before = history
                .as_ref()
                .zip(before)
                .and_then(|(history, at)| history.sessions.get(at))
                .and_then(|session| session.gpa);
            previous(
                name,
                pooled(&courses)
                    .map(|p| p.0)
                    .zip(before_courses.as_deref().and_then(pooled).map(|p| p.0)),
                grades.as_ref().map(|g| g.gpa).zip(gpa_before),
            )
        });
        let biometric: Vec<BiometricDay> = class_days
            .iter()
            .filter_map(|&day| {
                self.cached_day(day)
                    .map(|data| BiometricDay::new(day, &data))
                    .filter(|day| day.complete)
            })
            .collect();
        let schedule = self.cached_exams(flow.to_string());
        let marks_page = self.cached_marks(flow.to_string());
        // FFCS starts in the second semester: an exam session dated before this one's classes.
        let started = day_month(first_day);
        let first_semester = history.as_ref().is_none_or(|history| {
            !history
                .sessions
                .iter()
                .any(|s| session_month(s).zip(started).is_some_and(|(m, at)| m < at))
        });
        let mut course_codes: Vec<String> = codes.iter().map(|c| c.trim().to_string()).collect();
        course_codes.sort();
        course_codes.dedup();
        let mut report = WrappedReport {
            semester_name: list
                .semesters
                .iter()
                .find(|semester| semester.id == semester_id)
                .map_or_default(|semester| semester.name.clone()),
            semester_id,
            first_day,
            last_day,
            window_end,
            courses: codes.len() as i32,
            class_day_count: class_days.len() as i32,
            holidays: calendar.holidays().len() as i32,
            span_days: window_end - first_day + 1,
            course_codes,
            attendance: attendance(&courses, &classes),
            mosaic: mosaic(&classes, &calendar, first_day, window_end),
            arc: arc(&classes, &calendar, first_day, window_end),
            on_duty: on_duty(&classes),
            streak: streak(&classes),
            never_missed: never_missed(&courses),
            comeback: comeback(&classes),
            close_call: close_call(&classes),
            traps: traps(&classes, &biometric),
            habits: habits(&classes, slots),
            day_half: day_half(&classes),
            time: time(&classes),
            places: places(&classes, slots),
            ffcs: (!first_semester).then(|| ffcs(slots)).flatten(),
            marks: marks_page.as_ref().and_then(marks),
            full_marks: marks_page.as_ref().and_then(full_marks),
            assessments: marks_page.as_ref().and_then(assessments),
            exams: schedule.as_ref().and_then(|e| exams(e, today)),
            crunch: schedule.as_ref().and_then(|e| crunch(e, today)),
            degree: history
                .as_ref()
                .zip(this)
                .and_then(|(history, this)| degree(history, this)),
            roi: roi(&courses, own_grades.as_ref(), marks_page.as_ref()),
            grades,
            cgpa,
            outings: outings(
                self.read_kept::<GeneralOutingData>(&format!("{PAGES}/{}", outing::GENERAL))
                    .as_ref(),
                self.read_kept::<WeekendOutingData>(&format!("{PAGES}/{}", outing::WEEKEND))
                    .as_ref(),
                first_day,
                window_end.min(today - 1),
                today,
                |day| {
                    self.cached_day(day)
                        .map(|data| BiometricDay::new(day, &data))
                        .is_some_and(|day| day.complete && !day.punches.is_empty())
                },
            ),
            campus: campus(&biometric, class_days.len() as i32),
            hostel: hostel(&biometric),
            moves: moves(&biometric),
            previous,
            traits: Vec::new(),
            title: WrappedTitle::SteadyCruiser,
            title_without_grades: WrappedTitle::SteadyCruiser,
            mixtape: None,
            mixtape_without_grades: None,
            mixtape_without_hostel: None,
            mixtape_without_either: None,
            class_days,
        };
        report.traits = traits(&report);
        report.title = title(&report.traits, true, true);
        report.title_without_grades = title(&report.traits, false, true);
        // The tape's seed: the student and semester, so it plays the same every time.
        let regno = self
            .saved_regno()
            .or_else(|| history.as_ref().map(|h| h.reg_no.clone()))
            .unwrap_or_default();
        report.mixtape = mixtape(&report, &regno, true, true);
        report.mixtape_without_grades = mixtape(&report, &regno, false, true);
        report.mixtape_without_hostel = mixtape(&report, &regno, true, false);
        report.mixtape_without_either = mixtape(&report, &regno, false, false);
        info!(
            courses = report.courses,
            classes = classes.len(),
            biometric_days = biometric.len(),
            attendance = report.attendance.is_some(),
            mosaic = report.mosaic.is_some(),
            arc = report.arc.is_some(),
            on_duty = report.on_duty.is_some(),
            streak = report.streak.is_some(),
            comeback = report.comeback.is_some(),
            close_call = report.close_call.is_some(),
            traps = report.traps.is_some(),
            ffcs = report.ffcs.is_some(),
            first_semester,
            habits = report.habits.is_some(),
            day_half = report.day_half.is_some(),
            time = report.time.is_some(),
            places = report.places.is_some(),
            marks = report.marks.is_some(),
            full_marks = report.full_marks.is_some(),
            assessments = report.assessments.is_some(),
            exams = report.exams.is_some(),
            crunch = report.crunch.is_some(),
            grades = report.grades.is_some(),
            cgpa = report.cgpa.is_some(),
            degree = report.degree.is_some(),
            roi = report.roi.is_some(),
            mixtape = report.mixtape.is_some(),
            outings = report.outings.is_some(),
            hostel = report.hostel.is_some(),
            moves = report.moves.is_some(),
            previous = report.previous.is_some(),
            traits = report.traits.len(),
            "wrapped"
        );
        Some(report)
    }
}

/// Classes a course of this kind counts per class (VTOP counts a lab session twice).
fn periods(course: &AttendanceCourse) -> i32 {
    if course.kind == ClassKind::Lab { 2 } else { 1 }
}

fn is_attended(class: &AttendedClass) -> bool {
    matches!(
        class.status,
        Some(ClassStatus::Present | ClassStatus::OnDuty)
    )
}

/// Present, on duty or absent: the classes a percentage counts.
fn is_counted(class: &AttendedClass) -> bool {
    is_attended(class) || class.status == Some(ClassStatus::Absent)
}

/// `attended` of `counted` in percent, rounded up as VTOP does; none without classes.
fn percent(attended: i32, counted: i32) -> Option<i32> {
    let (a, c) = (i64::from(attended), i64::from(counted));
    (c > 0).then(|| ((a * 100 + c - 1) / c) as i32)
}

/// The pooled percent, attended and counted across `courses`.
fn pooled(courses: &[AttendanceCourse]) -> Option<(i32, i32, i32)> {
    let attended = courses.iter().map(|c| c.attended).sum();
    let counted = courses.iter().map(|c| c.counted).sum();
    Some((percent(attended, counted)?, attended, counted))
}

/// Each key's sum, keys compared ignoring case, in the order first seen.
fn sums(items: impl IntoIterator<Item = (String, i32)>) -> Vec<(String, i32)> {
    let mut sums: Vec<(String, i32)> = Vec::new();
    for (key, n) in items {
        match sums.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(&key)) {
            Some((_, sum)) => *sum += n,
            None => sums.push((key, n)),
        }
    }
    sums
}

/// The key with the largest sum, as [`sums`]; the first seen wins a tie.
fn most(items: impl IntoIterator<Item = (String, i32)>) -> Option<(String, i32)> {
    sums(items)
        .into_iter()
        .fold(None, |best, (key, n)| match best {
            Some((_, top)) if top >= n => best,
            _ => Some((key, n)),
        })
}

/// Only the halves that improved: (now, before) pooled attendance percent and GPA.
fn previous(
    semester_name: String,
    attendance: Option<(i32, i32)>,
    gpa: Option<(f64, f64)>,
) -> Option<WrappedPrevious> {
    let attendance_change = attendance
        .map(|(now, before)| now - before)
        .filter(|&change| change > 0);
    // At hundredths, as drawn: never "up 0.00".
    let gpa_change = gpa
        .map(|(now, before)| (hundredths(now) - hundredths(before)) / 100.0)
        .filter(|&change| change >= 0.01);
    (attendance_change.is_some() || gpa_change.is_some()).then_some(WrappedPrevious {
        semester_name,
        attendance_change,
        gpa_change,
    })
}

impl Core {
    /// The selected semester's class days from its cached calendar.
    fn class_days(&self) -> Vec<i32> {
        self.read_page::<AcademicCalendarData>(CALENDAR)
            .map_or_default(|data| SemesterCalendar::of(&data).class_days())
    }

    /// The semester after the pick in VTOP's list (newest first): its id and name.
    fn previous_semester(&self, list: &SemesterData) -> Option<(String, String)> {
        let picked = self.pick(list)?;
        let at = list.semesters.iter().position(|s| s.id == picked)?;
        list.semesters
            .get(at + 1)
            .map(|s| (s.id.clone(), s.name.clone()))
    }
}

#[cfg(test)]
mod tests;

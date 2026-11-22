//! Semester Wrapped: every fact of the end-of-semester story, from the selected semester's
//! cached pages. A fact without data is none (or empty), and the story skips its slide. Logs
//! say which facts exist, never their values.

use std::collections::{HashMap, HashSet};
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
mod results;

use campus::*;
use classes::*;
use results::*;

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
    pub attendance: Option<WrappedAttendance>,
    pub on_duty: Option<WrappedOnDuty>,
    pub streak: Option<WrappedStreak>,
    /// Course names; empty skips the slide.
    pub never_missed: Vec<String>,
    pub comeback: Option<WrappedComeback>,
    pub close_call: Option<WrappedCloseCall>,
    pub habits: Option<WrappedHabits>,
    pub day_half: Option<WrappedDayHalf>,
    pub time: Option<WrappedTime>,
    pub places: Option<WrappedPlaces>,
    pub marks: Option<WrappedMarks>,
    pub full_marks: Option<WrappedFullMarks>,
    pub assessments: Option<WrappedAssessments>,
    pub exams: Option<WrappedExams>,
    pub crunch: Option<WrappedCrunch>,
    pub grades: Option<WrappedGrades>,
    pub cgpa: Option<WrappedCgpa>,
    pub outings: Option<WrappedOutings>,
    pub campus: Option<WrappedCampus>,
    pub hostel: Option<WrappedHostel>,
    pub moves: Option<WrappedMoves>,
    pub previous: Option<WrappedPrevious>,
    pub title: WrappedTitle,
    /// The title when Hide grades is on.
    pub title_without_grades: WrappedTitle,
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
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedCourse {
    pub name: String,
    pub percent: i32,
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

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedHabits {
    /// 0 = Monday; none without an absence.
    pub worst_weekday: Option<i32>,
    pub worst_weekday_misses: i32,
    pub misses: i32,
    /// Over classes starting before 9:00.
    pub early_percent: Option<i32>,
    pub early_classes: i32,
    pub first_slot_days: i32,
    pub last_slot_days: i32,
    /// Absences by weekday, Monday first; 7 long.
    pub misses_by_weekday: Vec<i32>,
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
    pub faculty_classes: i32,
}

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedMarks {
    pub best_cat: Option<WrappedCat>,
    pub best_course: Option<String>,
    pub best_course_percent: i32,
    /// The average CAT-I to CAT-II change in points.
    pub cat_change: Option<i32>,
    pub most_improved: Option<String>,
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
    /// Each day's first floor and every rise after it.
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

/// Against the next older semester; only the halves that improved.
#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedPrevious {
    pub semester_name: String,
    pub attendance_change: Option<i32>,
    pub gpa_change: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Enum)]
pub enum WrappedTitle {
    Comeback,
    IronStreak,
    EarlyRegular,
    Topper,
    Explorer,
    LabRat,
    Regular,
    #[default]
    FreeSpirit,
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

/// The Iron Streak: a run of this share (percent) of all posted classes, and this many.
const IRON_STREAK_PERCENT: i32 = 15;
const IRON_STREAK_MIN: i32 = 15;

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
            .map(|data| Attendance::from(data).courses)
            .unwrap_or_default();
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
        let (this, before) = history
            .as_ref()
            .map(|history| {
                let own = own_grades
                    .as_ref()
                    .map(grade_codes)
                    .filter(|own| !own.is_empty())
                    .unwrap_or_else(|| codes.iter().map(|c| c.trim().to_uppercase()).collect());
                sessions(history, &own, first_day, last_day)
            })
            .unwrap_or_default();
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
        let mut report = WrappedReport {
            semester_name: list
                .semesters
                .iter()
                .find(|semester| semester.id == semester_id)
                .map(|semester| semester.name.clone())
                .unwrap_or_default(),
            semester_id,
            first_day,
            last_day,
            window_end,
            courses: codes.len() as i32,
            class_day_count: class_days.len() as i32,
            holidays: calendar.holidays().len() as i32,
            attendance: attendance(&courses, &classes),
            on_duty: on_duty(&classes),
            streak: streak(&classes),
            never_missed: never_missed(&courses),
            comeback: comeback(&classes),
            close_call: close_call(&classes),
            habits: habits(&classes, slots),
            day_half: day_half(&classes),
            time: time(&classes),
            places: places(&classes, slots),
            marks: marks_page.as_ref().and_then(marks),
            full_marks: marks_page.as_ref().and_then(full_marks),
            assessments: marks_page.as_ref().and_then(assessments),
            exams: schedule.as_ref().and_then(|e| exams(e, today)),
            crunch: schedule.as_ref().and_then(|e| crunch(e, today)),
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
            title: WrappedTitle::FreeSpirit,
            title_without_grades: WrappedTitle::FreeSpirit,
            class_days,
        };
        report.title = title(&report, true);
        report.title_without_grades = title(&report, false);
        info!(
            courses = report.courses,
            classes = classes.len(),
            biometric_days = biometric.len(),
            attendance = report.attendance.is_some(),
            on_duty = report.on_duty.is_some(),
            streak = report.streak.is_some(),
            comeback = report.comeback.is_some(),
            close_call = report.close_call.is_some(),
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
            outings = report.outings.is_some(),
            hostel = report.hostel.is_some(),
            moves = report.moves.is_some(),
            previous = report.previous.is_some(),
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

/// The first title the facts match; `grades` false skips The Topper (Hide grades).
fn title(report: &WrappedReport, grades: bool) -> WrappedTitle {
    let rules = [
        (
            WrappedTitle::Comeback,
            report
                .comeback
                .as_ref()
                .is_some_and(|c| c.final_percent - c.low_percent >= 10),
        ),
        (
            WrappedTitle::IronStreak,
            report.streak.as_ref().is_some_and(|s| {
                s.length >= IRON_STREAK_MIN && s.length * 100 >= s.total * IRON_STREAK_PERCENT
            }),
        ),
        (
            WrappedTitle::EarlyRegular,
            report
                .habits
                .as_ref()
                .is_some_and(|h| h.early_classes >= 15 && h.early_percent.is_some_and(|p| p >= 90)),
        ),
        (
            WrappedTitle::Topper,
            grades && report.grades.as_ref().is_some_and(|g| g.gpa >= 9.0),
        ),
        (
            WrappedTitle::Explorer,
            report
                .outings
                .as_ref()
                .is_some_and(|o| o.leaves + o.weekends >= 8),
        ),
        (
            WrappedTitle::LabRat,
            report
                .time
                .as_ref()
                .is_some_and(|t| t.minutes > 0 && t.lab_minutes * 100 >= t.minutes * 40),
        ),
        (
            WrappedTitle::Regular,
            report.attendance.as_ref().is_some_and(|a| a.percent >= 90),
        ),
    ];
    rules
        .into_iter()
        .find(|(_, matched)| *matched)
        .map_or(WrappedTitle::FreeSpirit, |(title, _)| title)
}

impl Core {
    /// The selected semester's class days from its cached calendar.
    fn class_days(&self) -> Vec<i32> {
        self.read_page::<AcademicCalendarData>(CALENDAR)
            .map(|data| SemesterCalendar::of(&data).class_days())
            .unwrap_or_default()
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

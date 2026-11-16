//! Semester Wrapped: every fact of the end-of-semester story, from the selected semester's
//! cached pages. A fact without data is none (or empty), and the story skips its slide. Logs
//! say which facts exist, never their values.

use std::collections::{HashMap, HashSet};
use std::fs;

use tracing::info;
use vtop_core::VtopError;
use vtop_core::inputs::SemesterId;
use vtop_core::types::{
    AcademicCalendarData, AttendanceData, GeneralOutingData, SemesterData, TimetableData,
    WeekendOutingData,
};

use crate::biometric::{BiometricDay, PunchKind};
use crate::calendar::{SemesterCalendar, exam_name};
use crate::exams::Exams;
use crate::grades::{
    GradeCount, GradeHistory, GradeSession, Grades, counts, grade_points, semester_summary,
};
use crate::marks::Marks;
use crate::outing::{self, OutingStanding};
use crate::records::{
    self, Attendance, AttendanceCourse, AttendedClass, ClassKind, ClassSlot, ClassStatus, Timetable,
};
use crate::route::page;
use crate::{
    ATTENDANCE, CALENDAR, Core, CoreError, PAGES, SEMESTERS, TIMETABLE, traced, traced_sync,
};

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
    pub streak: Option<WrappedStreak>,
    /// Course names; empty skips the slide.
    pub never_missed: Vec<String>,
    pub comeback: Option<WrappedComeback>,
    pub habits: Option<WrappedHabits>,
    pub time: Option<WrappedTime>,
    pub places: Option<WrappedPlaces>,
    pub marks: Option<WrappedMarks>,
    pub exams: Option<WrappedExams>,
    pub grades: Option<WrappedGrades>,
    pub cgpa: Option<WrappedCgpa>,
    pub outings: Option<WrappedOutings>,
    pub campus: Option<WrappedCampus>,
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

#[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
pub struct WrappedExams {
    pub cats: i32,
    pub fats: i32,
    pub top_venue: Option<String>,
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
                .read::<SemesterData>(SEMESTERS)
                .and_then(|list| self.previous_semester(&list))
            else {
                info!("no older semester");
                return Ok(());
            };
            let path = format!("{PAGES}/{previous}/{ATTENDANCE}");
            if self.read::<AttendanceData>(&path).is_some() {
                info!("older semester's attendance cached");
                return Ok(());
            }
            let id = SemesterId::parse(&previous)?;
            let data: AttendanceData = page!(self, "attendance", Some(&previous), [], |client| {
                client.attendance(&id)
            })?;
            if let Some(parent) = self.at(&path).parent() {
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

/// One period of class, in minutes; a lab session is two.
const PERIOD_MINUTES: i32 = 50;
/// The comeback counts from this posted class on, and needs this many points.
const COMEBACK_FROM: usize = 20;
const COMEBACK_POINTS: i32 = 5;
/// "8 AM" classes start before this minute.
const EARLY_BEFORE: i32 = 9 * 60;
/// A return counts as late after this minute.
const EVENING: i32 = 17 * 60;
/// The Iron Streak: a run of this share (percent) of all posted classes, and this many.
const IRON_STREAK_PERCENT: i32 = 15;
const IRON_STREAK_MIN: i32 = 15;
/// Courses on the Courses slide's podium.
const TOP_COURSES: usize = 3;

impl Core {
    fn wrapped(&self, flow: &str, today: i32) -> Option<WrappedReport> {
        let list = self.read::<SemesterData>(SEMESTERS)?;
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
                .read::<AttendanceData>(&format!("{PAGES}/{id}/{ATTENDANCE}"))
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
            streak: streak(&classes),
            never_missed: never_missed(&courses),
            comeback: comeback(&classes),
            habits: habits(&classes, slots),
            time: time(&classes),
            places: places(&classes, slots),
            marks: self.cached_marks(flow.to_string()).and_then(|m| marks(&m)),
            exams: self
                .cached_exams(flow.to_string())
                .and_then(|e| exams(&e, today)),
            grades,
            cgpa,
            outings: outings(
                self.read::<GeneralOutingData>(&format!("{PAGES}/{}", outing::GENERAL))
                    .as_ref(),
                self.read::<WeekendOutingData>(&format!("{PAGES}/{}", outing::WEEKEND))
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
            streak = report.streak.is_some(),
            comeback = report.comeback.is_some(),
            habits = report.habits.is_some(),
            time = report.time.is_some(),
            places = report.places.is_some(),
            marks = report.marks.is_some(),
            exams = report.exams.is_some(),
            grades = report.grades.is_some(),
            cgpa = report.cgpa.is_some(),
            outings = report.outings.is_some(),
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

/// The key with the largest sum, keys compared ignoring case; the first seen wins a tie.
fn most(items: impl IntoIterator<Item = (String, i32)>) -> Option<(String, i32)> {
    let mut sums: Vec<(String, i32)> = Vec::new();
    for (key, n) in items {
        match sums.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(&key)) {
            Some((_, sum)) => *sum += n,
            None => sums.push((key, n)),
        }
    }
    sums.into_iter().fold(None, |best, (key, n)| match best {
        Some((_, top)) if top >= n => best,
        _ => Some((key, n)),
    })
}

fn attendance(courses: &[AttendanceCourse], classes: &[Taken]) -> Option<WrappedAttendance> {
    let (percent, attended, counted) = pooled(courses)?;
    let held = || courses.iter().filter(|course| course.counted > 0);
    let least = held().min_by(|a, b| a.percent.total_cmp(&b.percent))?;
    // Best first, more attended classes first among equals; a course's other part (theory,
    // lab) and the least-attended course left out.
    let mut ranked: Vec<&AttendanceCourse> = held().collect();
    ranked.sort_by(|a, b| {
        b.percent
            .total_cmp(&a.percent)
            .then(b.attended.cmp(&a.attended))
    });
    let mut top: Vec<WrappedCourse> = Vec::new();
    for course in ranked {
        if top.len() < TOP_COURSES
            && course.name != least.name
            && !top.iter().any(|c| c.name == course.name)
        {
            top.push(WrappedCourse {
                name: course.name.clone(),
                percent: course.percent.ceil() as i32,
            });
        }
    }
    let (best, best_percent) = top.first().map_or_else(
        || (least.name.clone(), least.percent.ceil() as i32),
        |c| (c.name.clone(), c.percent),
    );
    Some(WrappedAttendance {
        percent,
        attended,
        counted,
        on_duty: classes
            .iter()
            .filter(|(_, class)| class.status == Some(ClassStatus::OnDuty))
            .map(|(course, _)| periods(course))
            .sum(),
        best,
        best_percent,
        least: least.name.clone(),
        least_percent: least.percent.ceil() as i32,
        top,
    })
}

fn streak(classes: &[Taken]) -> Option<WrappedStreak> {
    let runs: Vec<_> = classes
        .iter()
        .map(|(_, class)| (class.day, class.start, class.status))
        .collect();
    let (length, first_day, last_day) = records::longest_run(&runs)?;
    // Where the run sits among the posted classes, in time order: the first window of `length`
    // attended classes from its first day to its last, as `longest_run` keeps the first.
    let mut posted: Vec<&AttendedClass> = classes
        .iter()
        .map(|(_, class)| class)
        .filter(|class| is_counted(class))
        .collect();
    posted.sort_by_key(|class| (class.day, class.start));
    let offset = (0..posted.len())
        .find(|&at| {
            posted.get(at..at + length as usize).is_some_and(|run| {
                run.iter().all(|class| is_attended(class))
                    && run.first().is_some_and(|class| class.day == first_day)
                    && run.last().is_some_and(|class| class.day == last_day)
            })
        })
        .unwrap_or(0);
    // Each week (by its Monday) with a posted class: whether it had no absence.
    let mut weeks: HashMap<i32, bool> = HashMap::new();
    for (_, class) in classes {
        let Some(status) = class.status else {
            continue;
        };
        let perfect = weeks
            .entry(class.day - records::weekday(class.day))
            .or_insert(true);
        *perfect &= status != ClassStatus::Absent;
    }
    Some(WrappedStreak {
        length,
        first_day,
        last_day,
        perfect_weeks: weeks.values().filter(|&&perfect| perfect).count() as i32,
        offset: offset as i32,
        total: posted.len() as i32,
    })
}

/// Courses with a counted class and no absence in any part (theory, lab) of the same code.
fn never_missed(courses: &[AttendanceCourse]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for course in courses {
        let mut parts = courses.iter().filter(|part| part.code == course.code);
        if parts.clone().any(|part| part.counted > 0)
            && parts.all(|part| part.attended >= part.counted)
            && !names.contains(&course.name)
        {
            names.push(course.name.clone());
        }
    }
    names
}

fn comeback(classes: &[Taken]) -> Option<WrappedComeback> {
    let mut posted: Vec<&Taken> = classes
        .iter()
        .filter(|(_, class)| is_counted(class))
        .collect();
    posted.sort_by_key(|(_, class)| (class.day, class.start));
    let (mut attended, mut counted) = (0, 0);
    let (mut low, mut last): (Option<(i32, i32)>, Option<i32>) = (None, None);
    for (n, (course, class)) in posted.into_iter().enumerate() {
        counted += periods(course);
        if is_attended(class) {
            attended += periods(course);
        }
        if n + 1 < COMEBACK_FROM {
            continue;
        }
        let now = percent(attended, counted)?;
        if low.is_none_or(|(lowest, _)| now < lowest) {
            low = Some((now, class.day));
        }
        last = Some(now);
    }
    let ((low_percent, low_day), final_percent) = (low?, last?);
    (final_percent - low_percent >= COMEBACK_POINTS).then_some(WrappedComeback {
        low_percent,
        low_day,
        final_percent,
    })
}

fn habits(classes: &[Taken], slots: &[ClassSlot]) -> Option<WrappedHabits> {
    let posted: Vec<&AttendedClass> = classes
        .iter()
        .map(|(_, class)| class)
        .filter(|class| is_counted(class))
        .collect();
    if posted.is_empty() {
        return None;
    }
    let mut misses = [0; 7];
    for class in posted
        .iter()
        .filter(|class| class.status == Some(ClassStatus::Absent))
    {
        if let Some(count) = misses.get_mut(class.weekday as usize) {
            *count += 1;
        }
    }
    let (worst_weekday, worst_misses) = misses
        .iter()
        .enumerate()
        .max_by_key(|&(day, &n)| (n, std::cmp::Reverse(day)))
        .filter(|&(_, &n)| n > 0)
        .map(|(day, &n)| (day as i32, n))
        .unzip();
    let early: Vec<&&AttendedClass> = posted
        .iter()
        .filter(|class| class.start.is_some_and(|start| start < EARLY_BEFORE))
        .collect();
    let early_attended = early.iter().filter(|class| is_attended(class)).count() as i32;
    // Days with an attended class at the timetable's first start (or last end).
    let days_at = |edge: Option<i32>, at: fn(&AttendedClass) -> Option<i32>| {
        edge.map_or(0, |edge| {
            posted
                .iter()
                .filter(|class| is_attended(class) && at(class) == Some(edge))
                .map(|class| class.day)
                .collect::<HashSet<_>>()
                .len() as i32
        })
    };
    Some(WrappedHabits {
        worst_weekday,
        worst_weekday_misses: worst_misses.unwrap_or(0),
        misses: misses.iter().sum(),
        early_percent: percent(early_attended, early.len() as i32),
        early_classes: early.len() as i32,
        first_slot_days: days_at(slots.iter().map(|s| s.start).min(), |c| c.start),
        last_slot_days: days_at(slots.iter().map(|s| s.end).max(), |c| c.end),
        misses_by_weekday: misses.to_vec(),
    })
}

fn time(classes: &[Taken]) -> Option<WrappedTime> {
    let (mut count, mut minutes, mut lab_minutes) = (0, 0, 0);
    let (mut by_weekday, mut classes_by_weekday) = ([0; 7], [0; 7]);
    for (course, class) in classes.iter().filter(|(_, class)| is_attended(class)) {
        let length = periods(course) * PERIOD_MINUTES;
        count += 1;
        minutes += length;
        if course.kind == ClassKind::Lab {
            lab_minutes += length;
        }
        if let Some(day) = by_weekday.get_mut(class.weekday as usize) {
            *day += length;
        }
        if let Some(day) = classes_by_weekday.get_mut(class.weekday as usize) {
            *day += periods(course);
        }
    }
    (count > 0).then(|| WrappedTime {
        classes: count,
        minutes,
        lab_minutes,
        busiest_weekday: by_weekday
            .iter()
            .enumerate()
            .max_by_key(|&(day, &m)| (m, std::cmp::Reverse(day)))
            .map_or(0, |(day, _)| day as i32),
        classes_by_weekday: classes_by_weekday.to_vec(),
    })
}

/// "AB1-205" from the block and room.
fn room_label(slot: &ClassSlot) -> String {
    match (slot.block.trim(), slot.room.trim()) {
        ("", room) => room.to_string(),
        (block, "") => block.to_string(),
        (block, room) if room.starts_with(block) => room.to_string(),
        (block, room) => format!("{block}-{room}"),
    }
}

fn places(classes: &[Taken], slots: &[ClassSlot]) -> Option<WrappedPlaces> {
    let (mut rooms, mut faculty) = (Vec::new(), Vec::new());
    for (course, class) in classes.iter().filter(|(_, class)| is_attended(class)) {
        let mine = || {
            slots
                .iter()
                .filter(|slot| slot.code == course.code && slot.kind == course.kind)
        };
        let Some(slot) = mine().find(|slot| slot.slot == class.slot).or_else(|| {
            mine().find(|slot| slot.day == class.weekday && Some(slot.start) == class.start)
        }) else {
            continue;
        };
        let room = room_label(slot);
        if !room.is_empty() {
            rooms.push((room, slot.end - slot.start));
        }
        let name = if slot.faculty.is_empty() {
            &course.faculty
        } else {
            &slot.faculty
        };
        if !name.is_empty() {
            faculty.push((name.clone(), 1));
        }
    }
    let (room, room_minutes) = most(rooms).unzip();
    let (faculty, faculty_classes) = most(faculty).unzip();
    (room.is_some() || faculty.is_some()).then(|| WrappedPlaces {
        room,
        room_minutes: room_minutes.unwrap_or(0),
        faculty,
        faculty_classes: faculty_classes.unwrap_or(0),
    })
}

/// The CAT a marks title names, as it is named: "CAT" for a semester's only one (freshers'
/// first semester), else "CAT-I", "CAT-II", ... ("Continuous Assessment Test - II", "CAT1",
/// "CAT 2 (Theory)").
fn cat_of(title: &str) -> Option<String> {
    let lower = title
        .trim()
        .to_lowercase()
        .replace(['\u{2013}', '\u{2014}'], "-");
    let rest = lower
        .strip_prefix("continuous assessment test")
        .or_else(|| lower.strip_prefix("cat"))?;
    // The number, before any note in brackets.
    let number = rest
        .split('(')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_start_matches(['-', '_', ' '])
        .trim();
    if number.is_empty() {
        return Some("CAT".to_string());
    }
    Some(exam_name(&format!("cat {number}"))).filter(|name| name.starts_with("CAT-"))
}

fn marks(marks: &Marks) -> Option<WrappedMarks> {
    let mut best_cat: Option<(f64, WrappedCat)> = None;
    let mut rises: Vec<(&str, f64)> = Vec::new();
    for course in &marks.courses {
        // The course's CATs as named, each its first scored attempt.
        let cats: Vec<(String, f64)> = course
            .assessments
            .iter()
            .filter(|a| !a.reevaluation)
            .filter_map(|a| Some((cat_of(&a.title)?, a.percent?)))
            .fold(Vec::new(), |mut cats, (exam, percent)| {
                if !cats.iter().any(|(seen, _)| *seen == exam) {
                    cats.push((exam, percent));
                }
                cats
            });
        let cat = |exam: &str| cats.iter().find(|(name, _)| name == exam).map(|c| c.1);
        for (exam, percent) in &cats {
            if best_cat.as_ref().is_none_or(|(best, _)| percent > best) {
                best_cat = Some((
                    *percent,
                    WrappedCat {
                        course: course.name.clone(),
                        exam: exam.clone(),
                        percent: percent.round() as i32,
                    },
                ));
            }
        }
        if let (Some(one), Some(two)) = (cat("CAT-I"), cat("CAT-II")) {
            rises.push((&course.name, two - one));
        }
    }
    let best_course = marks
        .courses
        .iter()
        .filter(|course| course.max_weighted >= 50.0)
        .filter_map(|course| Some((course, course.percent?)))
        .max_by(|a, b| a.1.total_cmp(&b.1));
    let report = WrappedMarks {
        best_cat: best_cat.map(|(_, cat)| cat),
        best_course: best_course.map(|(course, _)| course.name.clone()),
        best_course_percent: best_course.map_or(0, |(_, percent)| percent.round() as i32),
        cat_change: (!rises.is_empty())
            .then(|| (rises.iter().map(|r| r.1).sum::<f64>() / rises.len() as f64).round() as i32),
        most_improved: rises
            .iter()
            .filter(|r| r.1 > 0.0)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|r| r.0.to_string()),
    };
    (report.best_cat.is_some() || report.best_course.is_some() || report.cat_change.is_some())
        .then_some(report)
}

/// Papers dated before `today`: the ones already sat.
fn exams(exams: &Exams, today: i32) -> Option<WrappedExams> {
    let sat = |group: &'_ crate::exams::ExamGroup| {
        group
            .papers
            .iter()
            .filter(|paper| paper.day.is_some_and(|day| day < today))
            .count() as i32
    };
    let papers = |matches: fn(&str) -> bool| -> i32 {
        exams
            .groups
            .iter()
            .filter(|group| matches(&group.name))
            .map(sat)
            .sum()
    };
    let (cats, fats) = (
        papers(|n| n == "CAT" || n.starts_with("CAT-")),
        papers(|n| n == "FAT"),
    );
    let venues = exams
        .groups
        .iter()
        .flat_map(|group| &group.papers)
        .filter(|paper| paper.day.is_some_and(|day| day < today) && !paper.venue.is_empty())
        .map(|paper| (paper.venue.clone(), 1));
    (cats + fats > 0).then(|| WrappedExams {
        cats,
        fats,
        top_venue: most(venues).map(|(venue, _)| venue),
    })
}

/// The semester's course codes, uppercase.
fn grade_codes(grades: &Grades) -> HashSet<String> {
    grades
        .courses
        .iter()
        .map(|course| course.code.trim().to_uppercase())
        .collect()
}

/// A session's exam month ("Nov-2025") as months since year 0; none for "Other".
fn session_month(session: &GradeSession) -> Option<i32> {
    let month = session.month.trim();
    if month.is_empty() {
        return None;
    }
    day_month(records::history_day(&format!("01-{month}"))?)
}

/// `day`'s month, as [`session_month`].
fn day_month(day: i32) -> Option<i32> {
    let date = records::civil_date(i64::from(day));
    let mut parts = date.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: i32 = parts.next()?.parse().ok()?;
    Some(year * 12 + month - 1)
}

/// The indices in `history.sessions` of this semester's exam session and of the one before it.
/// This semester's is the newest dated session after the month classes started (`first_day`),
/// or in it when classes also ended that month (`last_day`, a short summer semester), that
/// holds one of its course `codes`, so an earlier attempt of a re-registered course stays in its
/// own session. The one before is the newest dated session older than that (or than every
/// session, while this semester isn't in the history yet).
fn sessions(
    history: &GradeHistory,
    codes: &HashSet<String>,
    first_day: i32,
    last_day: i32,
) -> (Option<usize>, Option<usize>) {
    let dated = || {
        history
            .sessions
            .iter()
            .enumerate()
            .filter_map(|(at, session)| Some((at, session_month(session)?, session)))
    };
    let (started, ended) = (day_month(first_day), day_month(last_day));
    let this = dated()
        .filter(|&(_, month, session)| {
            started.is_some_and(|started| {
                month > started || (month == started && ended == Some(started))
            }) && session
                .courses
                .iter()
                .any(|c| codes.contains(&c.code.trim().to_uppercase()))
        })
        .max_by_key(|&(at, month, _)| (month, at))
        .map(|(at, month, _)| (at, month));
    let before = dated()
        .filter(|&(at, month, _)| this.is_none_or(|(this, newest)| at != this && month < newest))
        .max_by_key(|&(at, month, _)| (month, at))
        .map(|(at, _, _)| at);
    (this.map(|(at, _)| at), before)
}

/// None until VTOP posts a grade with points. From the semester's own grades page, else from
/// `this`, its session in the grade history (an old semester's page isn't cached); the S record
/// needs `this` too.
fn grades(
    own: Option<&Grades>,
    history: Option<&GradeHistory>,
    this: Option<usize>,
) -> Option<WrappedGrades> {
    let session = history
        .zip(this)
        .and_then(|(history, this)| history.sessions.get(this));
    let graded = own.filter(|own| own.courses.iter().any(|c| grade_points(&c.grade).is_some()));
    let (gpa, mix) = match graded {
        Some(own) => {
            let summary = semester_summary(own.courses.clone(), history.cloned());
            (summary.gpa?, summary.counts)
        }
        None => {
            let session = session?;
            let mix = counts(session.courses.iter().map(|c| c.grade.as_str()));
            (session.gpa?, mix)
        }
    };
    let s_count = mix
        .iter()
        .find(|count| count.grade == "S")
        .map_or(0, |count| count.count);
    // The S grades of each earlier semester (dated exam session) in the history.
    let this_month = session.and_then(session_month);
    let earlier: Vec<i32> = history
        .iter()
        .flat_map(|history| &history.sessions)
        .filter(|session| {
            !session.courses.is_empty()
                && this_month
                    .zip(session_month(session))
                    .is_some_and(|(this, m)| m < this)
        })
        .map(|session| session.courses.iter().filter(|c| c.grade == "S").count() as i32)
        .collect();
    Some(WrappedGrades {
        gpa,
        mix,
        s_count,
        s_record: s_count > 0 && !earlier.is_empty() && earlier.iter().all(|&n| s_count > n),
    })
}

/// `this` is this semester's session in the grade history. The CGPA as it was then: before over
/// the dated sessions older than this one (none for a first semester), after over those and
/// this one, so later semesters are left out of an old semester's story. After is VTOP's own
/// while no later session exists.
fn cgpa(history: &GradeHistory, this: usize) -> Option<WrappedCgpa> {
    let session = history.sessions.get(this)?;
    let month = session_month(session)?;
    let newest = !history
        .sessions
        .iter()
        .any(|s| session_month(s).is_some_and(|m| m > month));
    let earned = session
        .courses
        .iter()
        .filter(|course| passed(&course.grade))
        .filter_map(|course| course.credits.filter(|credit| *credit > 0.0))
        .sum();
    let vtop = |raw: &str| raw.trim().parse::<f64>().ok().filter(|n| n.is_finite());
    let (after, total) = if newest {
        (
            vtop(&history.cgpa)?,
            vtop(&history.credits_earned).unwrap_or(0.0),
        )
    } else {
        let (cgpa, total) = cumulative(history, |m| m <= month);
        (cgpa?, total)
    };
    Some(WrappedCgpa {
        before: cumulative(history, |m| m < month).0,
        after,
        credits_semester: earned,
        credits_total: total,
    })
}

/// A grade that earns the course's credits.
fn passed(grade: &str) -> bool {
    !matches!(grade, "F" | "N" | "" | "-")
}

/// The CGPA (none without a graded credit) and credits earned over the dated sessions whose
/// month `counts`. A course taken more than once counts its latest attempt only, as VTOP's CGPA
/// does; N and grades without points are left out of the CGPA, as in a session's GPA.
fn cumulative(history: &GradeHistory, counts: impl Fn(i32) -> bool) -> (Option<f64>, f64) {
    let mut latest: HashMap<String, (i32, &crate::grades::HistoryCourse)> = HashMap::new();
    for session in &history.sessions {
        let Some(month) = session_month(session).filter(|&m| counts(m)) else {
            continue;
        };
        for course in &session.courses {
            let code = course.code.trim().to_uppercase();
            if latest.get(&code).is_none_or(|&(seen, _)| month >= seen) {
                latest.insert(code, (month, course));
            }
        }
    }
    let (mut credits, mut points, mut earned) = (0.0, 0.0, 0.0);
    for (_, course) in latest.values() {
        let Some(credit) = course.credits.filter(|credit| *credit > 0.0) else {
            continue;
        };
        if passed(&course.grade) {
            earned += credit;
        }
        if let Some(value) = grade_points(&course.grade).filter(|_| course.grade != "N") {
            credits += credit;
            points += credit * value;
        }
    }
    ((credits > 0.0).then(|| points / credits), earned)
}

/// `value` in hundredths, as the story draws a GPA.
fn hundredths(value: f64) -> f64 {
    (value * 100.0).round()
}

/// Approved requests starting from `first` to `end`, the outing window up to yesterday. A
/// leave's length counts only days before `today`. A leave is one trip: its longest run of days
/// without a punch (`on_campus`: a complete cached biometric day with one), since students
/// apply for more days than they take; without biometric it is the whole leave.
fn outings(
    general: Option<&GeneralOutingData>,
    weekend: Option<&WeekendOutingData>,
    first: i32,
    end: i32,
    today: i32,
    on_campus: impl Fn(i32) -> bool,
) -> Option<WrappedOutings> {
    let window = first..=end;
    let approved = |status: &str| outing::standing(status) == OutingStanding::Approved;
    let leaves: Vec<(Option<(i32, i32)>, &str)> = general
        .iter()
        .flat_map(|data| &data.records)
        .filter(|record| approved(&record.status))
        .filter_map(|record| {
            let from = records::epoch_day(&record.from_date).filter(|day| window.contains(day))?;
            let to = records::epoch_day(&record.to_date)
                .unwrap_or(from)
                .min(today - 1)
                .max(from);
            Some((trip(from, to, &on_campus), record.place.trim()))
        })
        .collect();
    let weekends: Vec<&str> = weekend
        .iter()
        .flat_map(|data| &data.records)
        .filter(|record| {
            approved(&record.status)
                && records::epoch_day(&record.date).is_some_and(|day| window.contains(&day))
        })
        .map(|record| record.place.trim())
        .collect();
    if leaves.is_empty() && weekends.is_empty() {
        return None;
    }
    let trips = || leaves.iter().filter_map(|leave| leave.0);
    let away: HashSet<i32> = trips().flat_map(|(from, to)| from..=to.min(end)).collect();
    let (mut stay, mut longest_stay) = (0, 0);
    for day in window {
        stay = if away.contains(&day) { 0 } else { stay + 1 };
        longest_stay = longest_stay.max(stay);
    }
    let places = leaves
        .iter()
        .map(|leave| leave.1)
        .chain(weekends.iter().copied())
        .filter(|place| !place.is_empty())
        .map(|place| (place.to_string(), 1));
    Some(WrappedOutings {
        leaves: leaves.len() as i32,
        weekends: weekends.len() as i32,
        top_place: most(places).map(|(place, _)| place),
        longest_leave_days: trips().map(|(from, to)| to - from + 1).max(),
        days_away: away.len() as i32,
        longest_stay_days: longest_stay,
    })
}

/// The longest run of days from `from` to `to` not `on_campus` (the earliest of equals); none
/// when every day is.
fn trip(from: i32, to: i32, on_campus: impl Fn(i32) -> bool) -> Option<(i32, i32)> {
    let (mut best, mut run): (Option<(i32, i32)>, Option<i32>) = (None, None);
    for day in from..=to {
        if on_campus(day) {
            run = None;
            continue;
        }
        let start = *run.get_or_insert(day);
        if best.is_none_or(|(a, b)| day - start > b - a) {
            best = Some((start, day));
        }
    }
    best
}

/// From the cached days of the semester's `total` class days; none without class days.
fn campus(days: &[BiometricDay], total: i32) -> Option<WrappedCampus> {
    if total == 0 {
        return None;
    }
    let entries = |day: &BiometricDay| -> Vec<i32> {
        day.punches
            .iter()
            .filter(|punch| punch.kind == PunchKind::Entry)
            .filter_map(|punch| punch.minute)
            .collect()
    };
    let earliest = days
        .iter()
        .filter_map(|day| entries(day).into_iter().min())
        .min();
    let latest = days
        .iter()
        .filter_map(|day| entries(day).into_iter().max())
        .filter(|&minute| minute > EVENING)
        .max();
    Some(WrappedCampus {
        days_cached: days.len() as i32,
        days_total: total,
        earliest_entry: earliest.map(clock),
        latest_return: latest.map(clock),
        days_on_campus: days.iter().filter(|day| !day.punches.is_empty()).count() as i32,
    })
}

/// "7:12 AM" for minutes since midnight.
fn clock(minute: i32) -> String {
    let (hour, minute) = (minute / 60, minute % 60);
    let half = if hour < 12 { "AM" } else { "PM" };
    let hour = match hour % 12 {
        0 => 12,
        hour => hour,
    };
    format!("{hour}:{minute:02} {half}")
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

//! Attendance: the summary, each course's class-by-class history and plans, and which
//! histories to fetch again.

use std::sync::{Arc, PoisonError};

use tracing::info;
use vtop_core::spec;
use vtop_core::sync::RwLock;
use vtop_core::types::{AcademicCalendarData, AttendanceData, FullAttendanceData, TimetableData};

use crate::pages::{calendar, records};
use crate::{
    ATTENDANCE, ATTENDANCE_DETAIL, Attendance, AttendanceCourse, AttendanceDetail, AttendancePlan,
    AttendedClass, CALENDAR, ClassDay, ClassKind, ClassSlot, Core, CoreError, CourseOutlook,
    TIMETABLE, Timetable, traced, traced_sync, unix_now,
};

#[uniffi::export]
impl Core {
    pub fn cached_attendance(&self, flow: String) -> Option<Attendance> {
        traced_sync("cached_attendance", &flow, || {
            Ok(self
                .read_page::<AttendanceData>(ATTENDANCE)
                .map(Attendance::from))
        })
        .ok()
        .flatten()
    }

    pub async fn fetch_attendance(&self, flow: String) -> Result<Attendance, CoreError> {
        traced("fetch_attendance", &flow, async {
            let data = self.fetch(&spec::ATTENDANCE, None, &[]).await?;
            info!(courses = data.records.len(), "attendance");
            Ok(data.into())
        })
        .await
    }

    /// The saved class-by-class history of `course_key` (an
    /// [`AttendanceCourse::key`] in the saved attendance).
    pub fn cached_attendance_detail(
        &self,
        flow: String,
        course_key: String,
    ) -> Option<AttendanceDetail> {
        traced_sync("cached_attendance_detail", &flow, || {
            let Some(course) = self.cached_course(&course_key) else {
                return Ok(None);
            };
            Ok(self
                .read_page::<FullAttendanceData>(&detail_file(&course))
                .map(|data| {
                    let plans = self.plans(&course);
                    AttendanceDetail::new(course, data, plans)
                }))
        })
        .ok()
        .flatten()
    }

    /// Fetches `course_key`'s class-by-class history and saves it. The course
    /// comes from the saved attendance, fetched first when it lacks the course.
    pub async fn fetch_attendance_detail(
        &self,
        flow: String,
        course_key: String,
    ) -> Result<AttendanceDetail, CoreError> {
        traced("fetch_attendance_detail", &flow, async {
            // One fetch per course at a time (the attendance list and the
            // timetable both sync histories); a caller that waited, or one
            // right after, gets the history just fetched.
            let lock = self.detail_lock(&course_key);
            let _one = lock.write().await;
            if let Some(course) = self.cached_course(&course_key)
                && let Some(data) = self.read_page::<FullAttendanceData>(&detail_file(&course))
                && unix_now() - data.update_time as i64 <= DETAIL_FRESH_S
            {
                info!(course = %course.key, "attendance detail just fetched");
                let plans = self.plans(&course);
                return Ok(AttendanceDetail::new(course, data, plans));
            }
            let course = match self.cached_course(&course_key) {
                Some(course) => course,
                None => Attendance::from(self.fetch(&spec::ATTENDANCE, None, &[]).await?)
                    .courses
                    .into_iter()
                    .find(|course| course.key == course_key)
                    .ok_or_else(|| CoreError::Other {
                        reason: format!("no course {course_key} in this semester's attendance"),
                    })?,
            };
            let data = self
                .fetch(
                    &spec::FULL_ATTENDANCE,
                    Some(&detail_file(&course)),
                    &[
                        ("course_id", &course.course_id),
                        ("course_type", &course.course_type),
                    ],
                )
                .await?;
            info!(course = %course.key, classes = data.records.len(), "attendance detail");
            let plans = self.plans(&course);
            Ok(AttendanceDetail::new(course, data, plans))
        })
        .await
    }

    /// Keys of the courses whose saved history is missing or no longer
    /// matches the summary (VTOP says 10/11, the history 9/10), for the list
    /// to fetch after each refresh (like Flutter's `pullMismatchedHistory`,
    /// which skips missing ones; here they are fetched too). Courses without
    /// classes yet are left out. No network.
    pub fn mismatched_attendance_details(&self, flow: String) -> Vec<String> {
        traced_sync("mismatched_attendance_details", &flow, || {
            let keys: Vec<String> = self
                .read_page::<AttendanceData>(ATTENDANCE)
                .map(|data| Attendance::from(data).courses)
                .unwrap_or_default()
                .into_iter()
                .filter(|course| match self.cached_history(course) {
                    Some(classes) => !records::history_in_sync(course, &classes),
                    // Not fetched yet: worth fetching once VTOP lists a class.
                    None => course.total > 0,
                })
                .map(|course| course.key)
                .collect();
            info!(mismatched = keys.len(), "mismatched attendance details");
            Ok(keys)
        })
        .unwrap_or_default()
    }

    /// Each saved course's plan, until the next exam when one comes before
    /// the last class day, else until the semester's end; from the saved
    /// attendance, timetable and academic calendar (empty without a
    /// calendar). No network.
    pub fn attendance_outlook(&self, flow: String) -> Vec<CourseOutlook> {
        traced_sync("attendance_outlook", &flow, || {
            let (Some(attendance), Some((calendar, slots))) = (
                self.read_page::<AttendanceData>(ATTENDANCE),
                self.plan_inputs(),
            ) else {
                return Ok(Vec::new());
            };
            let (today, minute) = records::ist_now();
            let outlook: Vec<CourseOutlook> = Attendance::from(attendance)
                .courses
                .iter()
                .filter_map(|course| {
                    let through = self
                        .cached_history(course)
                        .and_then(|classes| records::counted_through(course, &classes));
                    let plan = records::attendance_plans(
                        course, &slots, &calendar, today, minute, through,
                    )
                    .into_iter()
                    .next()?;
                    Some(CourseOutlook {
                        key: course.key.clone(),
                        plan,
                    })
                })
                .collect();
            info!(courses = outlook.len(), "attendance outlook");
            Ok(outlook)
        })
        .unwrap_or_default()
    }

    /// Keys of the saved courses with a class held in `count` days from
    /// `from_day` (up to today) whose saved history is missing or behind
    /// the summary, for the timetable to fetch in the background. No network.
    pub fn stale_attendance_details(&self, flow: String, from_day: i32, count: i32) -> Vec<String> {
        traced_sync("stale_attendance_details", &flow, || {
            let Some(attendance) = self.read_page::<AttendanceData>(ATTENDANCE) else {
                return Ok(Vec::new());
            };
            let slots = self
                .read_page::<TimetableData>(TIMETABLE)
                .map_or_else(Vec::new, |data| Timetable::from(data).slots);
            let calendar = self
                .read_page::<AcademicCalendarData>(CALENDAR)
                .map(|data| calendar::SemesterCalendar::of(&data));
            let (today, _) = records::ist_now();
            let held: Vec<(String, ClassKind)> =
                records::timetable_days(&slots, from_day, count, calendar.as_ref())
                    .iter()
                    .filter(|day| day.day <= today)
                    .flat_map(|day| &day.classes)
                    .filter(|class| class.state == ClassDay::Held)
                    .map(|class| (class.slot.code.clone(), class.slot.kind))
                    .collect();
            let stale: Vec<String> = Attendance::from(attendance)
                .courses
                .into_iter()
                .filter(|course| held.contains(&(course.code.clone(), course.kind)))
                .filter(|course| {
                    self.cached_history(course)
                        .is_none_or(|classes| !records::history_in_sync(course, &classes))
                })
                .map(|course| course.key)
                .collect();
            info!(stale = stale.len(), "stale attendance details");
            Ok(stale)
        })
        .unwrap_or_default()
    }
}

impl Core {
    /// `course`'s plans from now, by the saved timetable and academic
    /// calendar; none without a calendar (its last class day is unknown).
    pub(crate) fn plans(&self, course: &AttendanceCourse) -> Vec<AttendancePlan> {
        self.plan_inputs()
            .map(|(calendar, slots)| {
                let (today, minute) = records::ist_now();
                let through = self
                    .cached_history(course)
                    .and_then(|classes| records::counted_through(course, &classes));
                records::attendance_plans(course, &slots, &calendar, today, minute, through)
            })
            .unwrap_or_default()
    }

    /// `course`'s saved class history, newest first.
    pub(crate) fn cached_history(&self, course: &AttendanceCourse) -> Option<Vec<AttendedClass>> {
        let data = self.read_page::<FullAttendanceData>(&detail_file(course))?;
        Some(AttendanceDetail::new(course.clone(), data, Vec::new()).classes)
    }

    /// The saved academic calendar and timetable slots plans count from.
    pub(crate) fn plan_inputs(&self) -> Option<(calendar::SemesterCalendar, Vec<ClassSlot>)> {
        let Some(calendar) = self
            .read_page::<AcademicCalendarData>(CALENDAR)
            .map(|data| calendar::SemesterCalendar::of(&data))
        else {
            info!("no calendar: no plan");
            return None;
        };
        let slots = self
            .read_page::<TimetableData>(TIMETABLE)
            .map_or_else(Vec::new, |data| Timetable::from(data).slots);
        Some((calendar, slots))
    }

    /// `key`'s course in the saved attendance of the selected semester.
    pub(crate) fn cached_course(&self, key: &str) -> Option<AttendanceCourse> {
        Attendance::from(self.read_page::<AttendanceData>(ATTENDANCE)?)
            .courses
            .into_iter()
            .find(|course| course.key == key)
    }
}

/// A history this young (seconds) is served again instead of fetched.
const DETAIL_FRESH_S: i64 = 20;

impl Core {
    /// The lock that keeps one history fetch per course key at a time.
    fn detail_lock(&self, key: &str) -> Arc<RwLock<()>> {
        self.shared
            .locks
            .attendance_details
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(key.to_string())
            .or_default()
            .clone()
    }
}

/// Named from the course found, never from the caller's key, so a key can't
/// reach outside the cache.
fn detail_file(course: &AttendanceCourse) -> String {
    format!(
        "{ATTENDANCE_DETAIL}/{}.json",
        course.key.replace(['/', '\\'], "_")
    )
}

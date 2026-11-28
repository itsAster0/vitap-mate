//! The timetable: the cached slots, each course's next class and the agenda days.

use std::collections::HashMap;

use tracing::info;
use vtop_core::spec;
use vtop_core::types::{AcademicCalendarData, AttendanceData, TimetableData};

use crate::pages::{calendar, exams, records};
use crate::{
    ATTENDANCE, Attendance, AttendedClass, CALENDAR, ClassKind, Core, CoreError, NextClass,
    TIMETABLE, Timetable, TimetableDay, traced, traced_sync,
};

#[uniffi::export]
impl Core {
    pub fn cached_timetable(&self, flow: String) -> Option<Timetable> {
        traced_sync("cached_timetable", &flow, || {
            Ok(self
                .read_page::<TimetableData>(TIMETABLE)
                .map(Timetable::from))
        })
        .ok()
        .flatten()
    }

    /// The next class of each course in the cached timetable, within a week
    /// of `now_unix` at the device's `utc_offset_minutes`.
    pub fn next_classes(
        &self,
        flow: String,
        now_unix: i64,
        utc_offset_minutes: i32,
    ) -> Vec<NextClass> {
        traced_sync("next_classes", &flow, || {
            let slots = self
                .read_page::<TimetableData>(TIMETABLE)
                .map_or_else(Vec::new, |data| Timetable::from(data).slots);
            let calendar = self
                .read_page::<AcademicCalendarData>(CALENDAR)
                .map(|data| calendar::SemesterCalendar::of(&data));
            Ok(records::next_classes(
                &slots,
                now_unix,
                utc_offset_minutes,
                calendar.as_ref(),
            ))
        })
        .unwrap_or_default()
    }

    /// `count` agenda days from epoch day `from_day` (local), from the
    /// cached timetable and academic calendar (without a calendar every
    /// class is held).
    pub fn timetable_days(&self, flow: String, from_day: i32, count: i32) -> Vec<TimetableDay> {
        traced_sync("timetable_days", &flow, || {
            let slots = self
                .read_page::<TimetableData>(TIMETABLE)
                .map_or_else(Vec::new, |data| Timetable::from(data).slots);
            let calendar = self
                .read_page::<AcademicCalendarData>(CALENDAR)
                .map(|data| calendar::SemesterCalendar::of(&data));
            let mut days = records::timetable_days(&slots, from_day, count, calendar.as_ref());
            let histories: HashMap<(String, ClassKind), Vec<AttendedClass>> = self
                .read_page::<AttendanceData>(ATTENDANCE)
                .map_or_default(|data| Attendance::from(data).courses)
                .into_iter()
                .filter_map(|course| {
                    let classes = self.cached_history(&course)?;
                    Some(((course.code, course.kind), classes))
                })
                .collect();
            records::fill_posted(&mut days, &histories);
            exams::fill_papers(&mut days, self.read_page(exams::EXAMS));
            info!(
                histories = histories.len(),
                from_day,
                days = days.len(),
                slots = slots.len(),
                calendar = calendar.is_some(),
                marked = days.iter().filter(|day| day.mark.is_some()).count(),
                "timetable days"
            );
            Ok(days)
        })
        .unwrap_or_default()
    }

    pub async fn fetch_timetable(&self, flow: String) -> Result<Timetable, CoreError> {
        traced("fetch_timetable", &flow, async {
            let data = self.fetch(&spec::TIMETABLE, None, &[]).await?;
            info!(slots = data.slots.len(), "timetable");
            Ok(data.into())
        })
        .await
    }
}

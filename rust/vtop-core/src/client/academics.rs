//! Semesters, timetable, attendance and the academic calendar.

use super::VtopClient;
use crate::error::VtopResult;
use crate::inputs::{CourseId, CourseType, SemesterId};
use crate::pages;
use crate::parser::{attendance, timetable};
use crate::types::{
    AcademicCalendarData, AttendanceData, FullAttendanceData, SemesterData, TimetableData,
};

impl VtopClient {
    pub async fn semesters_page(&self) -> VtopResult<String> {
        self.page("get_semesters", pages::semesters).await
    }

    pub async fn semesters(&self) -> VtopResult<SemesterData> {
        let page = self.semesters_page().await?;
        Ok(timetable::parse_semid_timetable(&page))
    }

    pub async fn timetable_page(&self, semester: &SemesterId) -> VtopResult<String> {
        self.page("get_timetable", |s| pages::timetable(s, semester))
            .await
    }

    pub async fn timetable(&self, semester: &SemesterId) -> VtopResult<TimetableData> {
        let page = self.timetable_page(semester).await?;
        Ok(timetable::parse_timetable(&page, semester.as_str()))
    }

    pub async fn attendance_page(&self, semester: &SemesterId) -> VtopResult<String> {
        self.page("get_attendance", |s| pages::attendance(s, semester))
            .await
    }

    pub async fn attendance(&self, semester: &SemesterId) -> VtopResult<AttendanceData> {
        let page = self.attendance_page(semester).await?;
        Ok(attendance::parse_attendance(&page, semester.as_str()))
    }

    pub async fn full_attendance_page(
        &self,
        semester: &SemesterId,
        course: &CourseId,
        course_type: &CourseType,
    ) -> VtopResult<String> {
        self.page("get_full_attendance", |s| {
            pages::full_attendance(s, semester, course, course_type)
        })
        .await
    }

    pub async fn full_attendance(
        &self,
        semester: &SemesterId,
        course: &CourseId,
        course_type: &CourseType,
    ) -> VtopResult<FullAttendanceData> {
        let page = self
            .full_attendance_page(semester, course, course_type)
            .await?;
        Ok(attendance::parse_full_attendance(
            &page,
            semester.as_str(),
            course.as_str(),
            course_type.as_str(),
        ))
    }

    /// The semester's calendar page: the class groups and one button per
    /// month.
    pub async fn calendar_months_page(&self, semester: &SemesterId) -> VtopResult<String> {
        self.page("get_calendar_months", |s| {
            pages::calendar_months(s, semester)
        })
        .await
    }

    /// One month of the calendar; [`cal_date`] is a `01-OCT-2026` from
    /// [`calendar::parse_calendar_months`].
    pub async fn calendar_month_page(
        &self,
        semester: &SemesterId,
        cal_date: &str,
    ) -> VtopResult<String> {
        self.page("get_calendar_month", |s| {
            pages::calendar_month(s, semester, cal_date)
        })
        .await
    }

    /// Every month of the semester's calendar: one request for the month
    /// list, then one per month.
    pub async fn academic_calendar(
        &self,
        semester: &SemesterId,
    ) -> VtopResult<AcademicCalendarData> {
        let signed = self.signed("get_academic_calendar").await?;
        let machine = pages::AcademicCalendar::new(signed, semester.clone());
        self.run("get_academic_calendar", machine).await
    }
}

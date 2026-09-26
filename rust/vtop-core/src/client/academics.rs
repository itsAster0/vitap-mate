//! Semesters, timetable, attendance and the academic calendar.

use super::VtopClient;
use crate::error::VtopResult;
use crate::inputs::{CourseId, CourseType, SemesterId};
use crate::now_unix;
use crate::parser::{attendance, calendar, timetable};
use crate::types::{
    AcademicCalendarData, AttendanceData, FullAttendanceData, SemesterData, TimetableData,
};

/// The calendar's "All Class Group (Combined)" option, which VTOP selects
/// by default.
const CALENDAR_CLASS_GROUP: &str = "COMB";

impl VtopClient {
    pub async fn semesters_page(&self) -> VtopResult<String> {
        let auth = self.authorize("get_semesters").await?;
        let url = self.config.url("/vtop/academics/common/StudentTimeTable");
        let body = format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            auth.registration_number, auth.csrf,
        );
        Self::log_request("get_semesters.send", "POST", &url);
        self.send_authenticated("get_semesters", || self.http.post(&url).body(body.clone()))
            .await
    }

    pub async fn semesters(&self) -> VtopResult<SemesterData> {
        let page = self.semesters_page().await?;
        Ok(timetable::parse_semid_timetable(&page))
    }

    pub async fn timetable_page(&self, semester: &SemesterId) -> VtopResult<String> {
        let auth = self.authorize("get_timetable").await?;
        let url = self.config.url("/vtop/processViewTimeTable");
        let body = format!(
            "_csrf={}&semesterSubId={}&authorizedID={}",
            auth.csrf,
            semester.as_str(),
            auth.registration_number
        );
        Self::log_request("get_timetable.send", "POST", &url);
        self.send_authenticated("get_timetable", || self.http.post(&url).body(body.clone()))
            .await
    }

    pub async fn timetable(&self, semester: &SemesterId) -> VtopResult<TimetableData> {
        let page = self.timetable_page(semester).await?;
        Ok(timetable::parse_timetable(&page, semester.as_str()))
    }

    pub async fn attendance_page(&self, semester: &SemesterId) -> VtopResult<String> {
        let auth = self.authorize("get_attendance").await?;
        let url = self.config.url("/vtop/processViewStudentAttendance");
        let body = format!(
            "_csrf={}&semesterSubId={}&authorizedID={}",
            auth.csrf,
            semester.as_str(),
            auth.registration_number
        );
        Self::log_request("get_attendance.send", "POST", &url);
        self.send_authenticated("get_attendance", || self.http.post(&url).body(body.clone()))
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
        let auth = self.authorize("get_full_attendance").await?;
        let url = self.config.url("/vtop/processViewAttendanceDetail");
        let body = format!(
            "_csrf={}&semesterSubId={}&registerNumber={}&courseId={}&courseType={}&authorizedID={}",
            auth.csrf,
            semester.as_str(),
            auth.registration_number,
            course.as_str(),
            course_type.as_str(),
            auth.registration_number
        );
        Self::log_request("get_full_attendance.send", "POST", &url);
        self.send_authenticated("get_full_attendance", || {
            self.http.post(&url).body(body.clone())
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
        let auth = self.authorize("get_calendar_months").await?;
        let url = self.config.url("/vtop/getDateForSemesterPreview");
        let body = format!(
            "_csrf={}&paramReturnId=getDateForSemesterPreview&semSubId={}&authorizedID={}",
            auth.csrf,
            semester.as_str(),
            auth.registration_number
        );
        Self::log_request("get_calendar_months.send", "POST", &url);
        self.send_authenticated("get_calendar_months", || {
            self.http.post(&url).body(body.clone())
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
        let auth = self.authorize("get_calendar_month").await?;
        let url = self.config.url("/vtop/processViewCalendar");
        let body = format!(
            "_csrf={}&calDate={}&semSubId={}&classGroupId={}&authorizedID={}",
            auth.csrf,
            urlencoding::encode(cal_date),
            semester.as_str(),
            CALENDAR_CLASS_GROUP,
            auth.registration_number
        );
        Self::log_request("get_calendar_month.send", "POST", &url);
        self.send_authenticated("get_calendar_month", || {
            self.http.post(&url).body(body.clone())
        })
        .await
    }

    /// Every month of the semester's calendar: one request for the month
    /// list, then one per month.
    pub async fn academic_calendar(
        &self,
        semester: &SemesterId,
    ) -> VtopResult<AcademicCalendarData> {
        let months = calendar::parse_calendar_months(&self.calendar_months_page(semester).await?);
        let mut entries = Vec::new();
        for cal_date in months {
            let page = self.calendar_month_page(semester, &cal_date).await?;
            entries.extend(calendar::parse_calendar_month(&page, &cal_date));
        }
        entries.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(AcademicCalendarData {
            entries,
            semester_id: semester.as_str().to_string(),
            update_time: now_unix(),
        })
    }
}

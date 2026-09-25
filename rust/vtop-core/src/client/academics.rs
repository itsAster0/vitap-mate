//! Semesters, timetable and attendance.

use super::VtopClient;
use crate::error::VtopResult;
use crate::inputs::{CourseId, CourseType, SemesterId};
use crate::parser::{attendance, timetable};
use crate::types::{AttendanceData, FullAttendanceData, SemesterData, TimetableData};

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
}

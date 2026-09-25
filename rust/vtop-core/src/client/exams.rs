//! Marks, exam schedule, grades and grade history.

use reqwest::multipart::Form;

use super::VtopClient;
use crate::error::VtopResult;
use crate::inputs::{CourseId, SemesterId};
use crate::parser::{exam_schedule, grade_history, grades, marks};
use crate::types::{
    ExamScheduleData, GradeDetailsData, GradeHistoryData, GradeViewData, MarksData,
};

impl VtopClient {
    /// The `authorizedID` / `semesterSubId` / `_csrf` multipart form several
    /// examination pages take.
    async fn semester_form_page(
        &self,
        context: &str,
        path: &str,
        semester: &SemesterId,
    ) -> VtopResult<String> {
        let auth = self.authorize(context).await?;
        let url = self.config.url(path);
        let build = || {
            let form = Form::new()
                .text("authorizedID", auth.registration_number.as_str().to_owned())
                .text("semesterSubId", semester.as_str().to_owned())
                .text("_csrf", auth.csrf.clone());
            self.http.post(&url).multipart(form)
        };
        Self::log_request(&format!("{context}.send"), "POST", &url);
        self.send_authenticated(context, build).await
    }

    pub async fn marks_page(&self, semester: &SemesterId) -> VtopResult<String> {
        self.semester_form_page(
            "get_marks",
            "/vtop/examinations/doStudentMarkView",
            semester,
        )
        .await
    }

    pub async fn marks(&self, semester: &SemesterId) -> VtopResult<MarksData> {
        let page = self.marks_page(semester).await?;
        Ok(marks::parse_marks(&page, semester.as_str()))
    }

    pub async fn exam_schedule_page(&self, semester: &SemesterId) -> VtopResult<String> {
        self.semester_form_page(
            "get_exam_schedule",
            "/vtop/examinations/doSearchExamScheduleForStudent",
            semester,
        )
        .await
    }

    pub async fn exam_schedule(&self, semester: &SemesterId) -> VtopResult<ExamScheduleData> {
        let page = self.exam_schedule_page(semester).await?;
        Ok(exam_schedule::parse_schedule(&page, semester.as_str()))
    }

    pub async fn grade_view_page(&self, semester: &SemesterId) -> VtopResult<String> {
        self.semester_form_page(
            "get_grade_view",
            "/vtop/examinations/examGradeView/doStudentGradeView",
            semester,
        )
        .await
    }

    pub async fn grade_view(&self, semester: &SemesterId) -> VtopResult<GradeViewData> {
        let page = self.grade_view_page(semester).await?;
        Ok(grades::parse_grade_view(&page, semester.as_str()))
    }

    pub async fn grade_view_details_page(
        &self,
        semester: &SemesterId,
        course: &CourseId,
    ) -> VtopResult<String> {
        let auth = self.authorize("get_grade_view_details").await?;
        let url = self
            .config
            .url("/vtop/examinations/examGradeView/getGradeViewDetails");
        let params = [
            ("authorizedID", auth.registration_number.as_str()),
            ("x", "codex"),
            ("semesterSubId", semester.as_str()),
            ("courseId", course.as_str()),
            ("_csrf", auth.csrf.as_str()),
        ];
        Self::log_request("get_grade_view_details.send", "POST", &url);
        self.send_authenticated("get_grade_view_details", || {
            self.http.post(&url).form(&params)
        })
        .await
    }

    pub async fn grade_view_details(
        &self,
        semester: &SemesterId,
        course: &CourseId,
    ) -> VtopResult<GradeDetailsData> {
        let page = self.grade_view_details_page(semester, course).await?;
        Ok(grades::parse_grade_view_details(
            &page,
            semester.as_str(),
            course.as_str(),
        ))
    }

    pub async fn grade_history_page(&self) -> VtopResult<String> {
        let auth = self.authorize("get_grade_history").await?;
        let url = self
            .config
            .url("/vtop/examinations/examGradeView/StudentGradeHistory");
        let body = format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            auth.registration_number, auth.csrf,
        );
        Self::log_request("get_grade_history.send", "POST", &url);
        self.send_authenticated("get_grade_history", || {
            self.http.post(&url).body(body.clone())
        })
        .await
    }

    pub async fn grade_history(&self) -> VtopResult<GradeHistoryData> {
        let page = self.grade_history_page().await?;
        Ok(grade_history::parse_grade_history(&page))
    }
}

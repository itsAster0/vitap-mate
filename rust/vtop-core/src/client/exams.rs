//! Marks, exam schedule, grades and grade history.

use super::VtopClient;
use crate::error::VtopResult;
use crate::inputs::{CourseId, SemesterId};
use crate::pages;
use crate::parser::{exam_schedule, grade_history, grades, marks};
use crate::types::{
    ExamScheduleData, GradeDetailsData, GradeHistoryData, GradeViewData, MarksData,
};

impl VtopClient {
    /// The `authorizedID` / `semesterSubId` / `_csrf` multipart form several
    /// examination pages take.
    pub async fn marks_page(&self, semester: &SemesterId) -> VtopResult<String> {
        self.page("get_marks", |s| pages::marks(s, semester)).await
    }

    pub async fn marks(&self, semester: &SemesterId) -> VtopResult<MarksData> {
        let page = self.marks_page(semester).await?;
        Ok(marks::parse_marks(&page, semester.as_str()))
    }

    pub async fn exam_schedule_page(&self, semester: &SemesterId) -> VtopResult<String> {
        self.page("get_exam_schedule", |s| pages::exam_schedule(s, semester))
            .await
    }

    pub async fn exam_schedule(&self, semester: &SemesterId) -> VtopResult<ExamScheduleData> {
        let page = self.exam_schedule_page(semester).await?;
        Ok(exam_schedule::parse_schedule(&page, semester.as_str()))
    }

    pub async fn grade_view_page(&self, semester: &SemesterId) -> VtopResult<String> {
        self.page("get_grade_view", |s| pages::grade_view(s, semester))
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
        self.page("get_grade_view_details", |s| {
            pages::grade_view_details(s, semester, course)
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
        self.page("get_grade_history", pages::grade_history).await
    }

    pub async fn grade_history(&self) -> VtopResult<GradeHistoryData> {
        let page = self.grade_history_page().await?;
        Ok(grade_history::parse_grade_history(&page))
    }
}

//! The MCP tools: read-only VTOP pages for the caller's account.

use std::future::Future;
use std::sync::Arc;

use axum::http::request::Parts;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, ServerCapabilities, ServerConfig};
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler};
use serde::{Deserialize, Serialize};
use vtop_core::inputs::{BiometricDate, CourseId, CourseType, SemesterId};
use vtop_core::{VtopClient, VtopError, VtopResult};

use crate::bridge::SessionSource;

/// The username the bearer key maps to, put on the request by the auth
/// layer.
#[derive(Clone)]
pub struct Caller(pub String);

/// Runs `call` with a client for `username`'s session. When VTOP says the
/// session expired, drops it at the bridge and tries once more.
pub async fn with_retry<T, F, Fut>(
    source: &dyn SessionSource,
    username: &str,
    call: F,
) -> Result<T, String>
where
    F: Fn(VtopClient) -> Fut,
    Fut: Future<Output = VtopResult<T>>,
{
    let mut retried = false;
    loop {
        let session = source.session(username).await?;
        let client = VtopClient::builder()
            .with_session(&session)
            .map_err(|error| error.to_string())?;
        match call(client).await {
            Err(VtopError::SessionExpired) if !retried => {
                retried = true;
                source.expire(username).await?;
            }
            result => return result.map_err(|error| error.to_string()),
        }
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SemesterArgs {
    #[schemars(description = "VTOP semester id, e.g. AP2026271 (from get_semesters)")]
    pub semester_id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct CourseArgs {
    #[schemars(description = "VTOP semester id (from get_semesters)")]
    pub semester_id: String,
    #[schemars(description = "Course id from the attendance or grades list")]
    pub course_id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct FullAttendanceArgs {
    #[schemars(description = "VTOP semester id (from get_semesters)")]
    pub semester_id: String,
    #[schemars(description = "Course id from get_attendance")]
    pub course_id: String,
    #[schemars(description = "Course type from get_attendance, e.g. ETH or ELA")]
    pub course_type: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct DateArgs {
    #[schemars(description = "Date as DD/MM/YYYY")]
    pub date: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct NoArgs {}

fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

fn input<T>(parsed: VtopResult<T>) -> Result<T, CallToolResult> {
    parsed.map_err(|error| tool_error(format!("invalid input: {error}")))
}

#[derive(Clone)]
pub struct VtopTools {
    source: Arc<dyn SessionSource>,
    tool_router: ToolRouter<Self>,
}

impl VtopTools {
    pub fn new(source: Arc<dyn SessionSource>) -> Self {
        Self {
            source,
            tool_router: Self::tool_router(),
        }
    }

    /// Fetches with the caller's session and renders the result as JSON.
    async fn fetch<T, F, Fut>(&self, parts: &Parts, call: F) -> CallToolResult
    where
        T: Serialize,
        F: Fn(VtopClient) -> Fut,
        Fut: Future<Output = VtopResult<T>>,
    {
        let Some(Caller(username)) = parts.extensions.get::<Caller>().cloned() else {
            return tool_error("no caller on the request");
        };
        match with_retry(self.source.as_ref(), &username, call).await {
            Ok(value) => match serde_json::to_string(&value) {
                Ok(json) => CallToolResult::success(vec![ContentBlock::text(json)]),
                Err(error) => tool_error(format!("could not encode the result: {error}")),
            },
            Err(message) => tool_error(message),
        }
    }
}

#[tool_router]
impl VtopTools {
    #[tool(
        description = "List the student's VTOP semesters (ids and names). Call this first to get a semester_id."
    )]
    async fn get_semesters(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        self.fetch(&parts, |client| async move { client.semesters().await })
            .await
    }

    #[tool(description = "Attendance summary for every course in a semester.")]
    async fn get_attendance(
        &self,
        Parameters(args): Parameters<SemesterArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let semester = match input(SemesterId::parse(&args.semester_id)) {
            Ok(value) => value,
            Err(error) => return error,
        };
        self.fetch(&parts, |client| {
            let semester = semester.clone();
            async move { client.attendance(&semester).await }
        })
        .await
    }

    #[tool(description = "Class-by-class attendance for one course.")]
    async fn get_full_attendance(
        &self,
        Parameters(args): Parameters<FullAttendanceArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let parsed = (|| {
            Ok((
                SemesterId::parse(&args.semester_id)?,
                CourseId::parse(&args.course_id)?,
                CourseType::parse(&args.course_type)?,
            ))
        })();
        let (semester, course, kind) = match input(parsed) {
            Ok(value) => value,
            Err(error) => return error,
        };
        self.fetch(&parts, |client| {
            let (semester, course, kind) = (semester.clone(), course.clone(), kind.clone());
            async move { client.full_attendance(&semester, &course, &kind).await }
        })
        .await
    }

    #[tool(description = "Weekly timetable for a semester.")]
    async fn get_timetable(
        &self,
        Parameters(args): Parameters<SemesterArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let semester = match input(SemesterId::parse(&args.semester_id)) {
            Ok(value) => value,
            Err(error) => return error,
        };
        self.fetch(&parts, |client| {
            let semester = semester.clone();
            async move { client.timetable(&semester).await }
        })
        .await
    }

    #[tool(description = "Internal marks (CAT, quizzes, assignments) for a semester.")]
    async fn get_marks(
        &self,
        Parameters(args): Parameters<SemesterArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let semester = match input(SemesterId::parse(&args.semester_id)) {
            Ok(value) => value,
            Err(error) => return error,
        };
        self.fetch(&parts, |client| {
            let semester = semester.clone();
            async move { client.marks(&semester).await }
        })
        .await
    }

    #[tool(description = "Exam schedule (dates, slots, venues, seats) for a semester.")]
    async fn get_exam_schedule(
        &self,
        Parameters(args): Parameters<SemesterArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let semester = match input(SemesterId::parse(&args.semester_id)) {
            Ok(value) => value,
            Err(error) => return error,
        };
        self.fetch(&parts, |client| {
            let semester = semester.clone();
            async move { client.exam_schedule(&semester).await }
        })
        .await
    }

    #[tool(description = "Final grades for a semester.")]
    async fn get_grades(
        &self,
        Parameters(args): Parameters<SemesterArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let semester = match input(SemesterId::parse(&args.semester_id)) {
            Ok(value) => value,
            Err(error) => return error,
        };
        self.fetch(&parts, |client| {
            let semester = semester.clone();
            async move { client.grade_view(&semester).await }
        })
        .await
    }

    #[tool(description = "Mark breakdown behind one course's grade.")]
    async fn get_grade_details(
        &self,
        Parameters(args): Parameters<CourseArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let parsed = (|| {
            Ok((
                SemesterId::parse(&args.semester_id)?,
                CourseId::parse(&args.course_id)?,
            ))
        })();
        let (semester, course) = match input(parsed) {
            Ok(value) => value,
            Err(error) => return error,
        };
        self.fetch(&parts, |client| {
            let (semester, course) = (semester.clone(), course.clone());
            async move { client.grade_view_details(&semester, &course).await }
        })
        .await
    }

    #[tool(description = "Full grade history and CGPA across all semesters.")]
    async fn get_grade_history(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        self.fetch(&parts, |client| async move { client.grade_history().await })
            .await
    }

    #[tool(description = "Biometric (hostel/campus entry) log for one day.")]
    async fn get_biometric(
        &self,
        Parameters(args): Parameters<DateArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let date = match input(BiometricDate::parse(&args.date)) {
            Ok(value) => value,
            Err(error) => return error,
        };
        self.fetch(&parts, |client| {
            let date = date.clone();
            async move { client.biometric_history(&date).await }
        })
        .await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for VtopTools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Read-only access to the student's VTOP (VIT-AP) data. Start with get_semesters to find a semester_id.",
        )
    }
}

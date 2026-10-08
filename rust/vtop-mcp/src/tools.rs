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

/// The caller's access key, put on the request by the auth layer.
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
pub struct CourseClassesArgs {
    #[schemars(description = "VTOP semester id (from get_semesters)")]
    pub semester_id: String,
    #[schemars(description = "Course id from get_courses")]
    pub course_id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct CourseDetailArgs {
    #[schemars(description = "VTOP semester id (from get_semesters)")]
    pub semester_id: String,
    #[schemars(description = "Faculty ERP id of the class, from get_course_classes")]
    pub erp_id: String,
    #[schemars(description = "Class id from get_course_classes")]
    pub class_id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct NoArgs {}

/// Course-page ids are plain alphanumerics, as vtop-core requires; checked
/// here so a bad id fails before a session is fetched.
fn plain_id(value: &str, label: &str) -> VtopResult<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 40 || !value.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(VtopError::ConfigurationError(format!(
            "{label} is not valid"
        )));
    }
    Ok(value.to_string())
}

/// The semester list with the current one marked: the semester picked in
/// the student's app, else the newest (VTOP lists newest first).
pub fn mark_current(
    data: vtop_core::types::SemesterData,
    picked: Option<&str>,
) -> serde_json::Value {
    let current = picked
        .filter(|id| data.semesters.iter().any(|semester| semester.id == *id))
        .map(str::to_string)
        .or_else(|| data.semesters.first().map(|semester| semester.id.clone()));
    let semesters: Vec<_> = data
        .semesters
        .iter()
        .map(|semester| {
            serde_json::json!({
                "id": semester.id,
                "name": semester.name,
                "current": current.as_deref() == Some(semester.id.as_str()),
            })
        })
        .collect();
    serde_json::json!({ "current_semester_id": current, "semesters": semesters })
}

const VTOP_HOST: &str = "vtop.vitap.ac.in";

/// A session as browser cookies (Playwright and Puppeteer take this shape)
/// and as a Cookie header.
pub fn browser_session(cookie_header: &str) -> serde_json::Value {
    let pairs: Vec<(&str, &str)> = cookie_header
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .map(|(name, value)| (name.trim(), value.trim()))
        .filter(|(name, _)| !name.is_empty())
        .collect();
    let cookies: Vec<_> = pairs
        .iter()
        .map(|(name, value)| {
            serde_json::json!({
                "name": name,
                "value": value,
                "domain": VTOP_HOST,
                "path": "/",
                "secure": true,
                "httpOnly": true,
            })
        })
        .collect();
    let header = pairs
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; ");
    serde_json::json!({
        "url": format!("https://{VTOP_HOST}/vtop/content"),
        "cookie_header": header,
        "cookies": cookies,
    })
}

fn json_result(value: &impl Serialize) -> CallToolResult {
    match serde_json::to_string(value) {
        Ok(json) => CallToolResult::success(vec![ContentBlock::text(json)]),
        Err(error) => tool_error(format!("could not encode the result: {error}")),
    }
}

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
            tool_router: Self::tool_router() + Self::more_tools_router(),
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
        description = "List the student's VTOP semesters (ids and names). The one marked current (also current_semester_id) is the semester picked in the student's app; use it unless they name another. Call this first to get a semester_id."
    )]
    async fn get_semesters(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let Some(Caller(key)) = parts.extensions.get::<Caller>().cloned() else {
            return tool_error("no caller on the request");
        };
        let data = match with_retry(self.source.as_ref(), &key, |client| async move {
            client.semesters().await
        })
        .await
        {
            Ok(data) => data,
            Err(message) => return tool_error(message),
        };
        // No VTOP request: the bridge has the app's pick.
        let picked = match self.source.whoami(&key).await {
            Ok(Some(identity)) => identity.semester_id,
            _ => None,
        };
        json_result(&mark_current(data, picked.as_deref()))
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

#[tool_router(router = more_tools_router)]
impl VtopTools {
    #[tool(
        description = "Academic calendar of a semester: every entry (holidays, exams, instructional days), sorted by date."
    )]
    async fn get_academic_calendar(
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
            async move { client.academic_calendar(&semester).await }
        })
        .await
    }

    #[tool(
        description = "Courses on the course page for a semester: id, code, title and type (ETH, ELA…). Use an id with get_course_classes."
    )]
    async fn get_courses(
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
            async move { client.course_page_courses(&semester).await }
        })
        .await
    }

    #[tool(
        description = "Classes of one course: class_id, erp_id, slot, faculty and school. Use them with get_course_detail."
    )]
    async fn get_course_classes(
        &self,
        Parameters(args): Parameters<CourseClassesArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let parsed = (|| {
            Ok((
                SemesterId::parse(&args.semester_id)?,
                plain_id(&args.course_id, "course id")?,
            ))
        })();
        let (semester, course) = match input(parsed) {
            Ok(value) => value,
            Err(error) => return error,
        };
        self.fetch(&parts, |client| {
            let (semester, course) = (semester.clone(), course.clone());
            async move { client.course_page_classes(&semester, &course).await }
        })
        .await
    }

    #[tool(
        description = "Course page of one class: the lecture plan (dates, topics and materials per lecture), syllabus and course plan availability."
    )]
    async fn get_course_detail(
        &self,
        Parameters(args): Parameters<CourseDetailArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let parsed = (|| {
            Ok((
                SemesterId::parse(&args.semester_id)?,
                plain_id(&args.erp_id, "faculty id")?,
                plain_id(&args.class_id, "class id")?,
            ))
        })();
        let (semester, erp, class) = match input(parsed) {
            Ok(value) => value,
            Err(error) => return error,
        };
        self.fetch(&parts, |client| {
            let (semester, erp, class) = (semester.clone(), erp.clone(), class.clone());
            async move { client.course_page_detail(&semester, &erp, &class).await }
        })
        .await
    }

    #[tool(
        description = "General (day) outing: the form state, student info, and current and past outing records."
    )]
    async fn get_general_outing(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        self.fetch(
            &parts,
            |client| async move { client.general_outing().await },
        )
        .await
    }

    #[tool(
        description = "Weekend outing: the form state, student info, and current and past outing records."
    )]
    async fn get_weekend_outing(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        self.fetch(
            &parts,
            |client| async move { client.weekend_outing().await },
        )
        .await
    }

    #[tool(
        description = "A live, signed-in VTOP session for a browser, so you can open VTOP as the student (for pages no other tool covers). Returns `url`, `cookies` (Playwright `context.add_cookies(cookies)` / Puppeteer `page.setCookie(...cookies)` shape) and `cookie_header` (for curl or requests). To use it: add the cookies to the browser context, then open `url`; it lands on the signed-in VTOP home. If VTOP shows its login page, the session expired: call get_session again. This is the student's full account: anything done in the browser acts as them, so do only what they asked, never print or share the cookies, and confirm before submitting any form."
    )]
    async fn get_session(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let Some(Caller(key)) = parts.extensions.get::<Caller>().cloned() else {
            return tool_error("no caller on the request");
        };
        // The bridge checks the session with VTOP before handing it out.
        match self.source.session(&key).await {
            Ok(session) => json_result(&browser_session(&session.cookies)),
            Err(message) => tool_error(message),
        }
    }

    #[tool(
        description = "Who this access key belongs to: registration number and key label. Does not contact VTOP."
    )]
    async fn whoami(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let Some(Caller(key)) = parts.extensions.get::<Caller>().cloned() else {
            return tool_error("no caller on the request");
        };
        match self.source.whoami(&key).await {
            Ok(Some(identity)) => match serde_json::to_string(&identity) {
                Ok(json) => CallToolResult::success(vec![ContentBlock::text(json)]),
                Err(error) => tool_error(format!("could not encode the result: {error}")),
            },
            Ok(None) => tool_error("key_unknown: Unknown or revoked access key."),
            Err(message) => tool_error(message),
        }
    }
}

/// What an agent reads on connecting: how the tools fit together.
const INSTRUCTIONS: &str = "\
Access to one VIT-AP student's VTOP data. Every tool is read-only except get_session, \
which hands over a live login for a browser: use it only for pages the other tools do \
not cover, act only as the student asked, confirm before submitting anything, and never \
show the cookies.

- Call get_semesters first and use the semester marked current (current_semester_id) \
as semester_id unless the student names another. It is the semester picked in the \
student's app, or else the newest, as VTOP lists them newest first.
- Ids chain from one tool to the next: get_attendance gives course_id and course_type \
for get_full_attendance; get_courses gives course_id for get_course_classes, which \
gives erp_id and class_id for get_course_detail. Dates are DD/MM/YYYY.
- Attendance below 75% risks being barred from exams. CAT-1 and CAT-2 are the \
mid-term exams; FAT is the final exam.
- The first call can take up to a minute while the student's phone or the server \
signs in to VTOP; wait for it rather than retrying.
- Errors: key_unknown means the access key was revoked; phone_unreachable or \
phone_timeout means the student should open the VITAP Mate app; rate_limited means \
wait a minute.";

#[tool_handler(router = self.tool_router)]
impl ServerHandler for VtopTools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(INSTRUCTIONS)
    }
}

//! The MCP tools: read-only VTOP pages for the caller's account.

use std::future::Future;
use std::sync::Arc;

use axum::http::request::Parts;
use rmcp::handler::server::router::prompt::PromptRouter;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, ResourceContents, ServerCapabilities, ServerConfig,
};
use rmcp::{prompt_handler, schemars, tool, tool_handler, tool_router, ServerHandler};
use serde::{Deserialize, Serialize};
use vtop_core::client::CourseFile;
use vtop_core::inputs::{BiometricDate, CourseFilePath, CourseId, CourseType, SemesterId};
use vtop_core::types::StudentProfile;
use vtop_core::{VtopClient, VtopError, VtopResult};

use crate::bridge::SessionSource;

/// The caller's access key, put on the request by the auth layer.
#[derive(Clone)]
pub struct Caller(pub String);

/// Runs `call` with a client for `username`'s session. When VTOP says the
/// session expired, or answers with an HTTP error (it gives 404 for a
/// session it dropped, e.g. after use from another address), drops the
/// session (here and at the edge) and tries once more with a fresh one.
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
            Err(VtopError::SessionExpired | VtopError::VtopServerError(_)) if !retried => {
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
pub struct DateRangeArgs {
    #[schemars(description = "First day, DD/MM/YYYY")]
    pub from: String,
    #[schemars(description = "Last day, DD/MM/YYYY; at most 14 days after from")]
    pub to: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct CourseFileArgs {
    #[schemars(
        description = "A download path from get_course_detail: a lecture material's path, syllabus_path, all_materials_path or general_materials_path"
    )]
    pub path: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct CoursePlanArgs {
    #[schemars(description = "VTOP semester id (from get_semesters)")]
    pub semester_id: String,
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

pub(crate) fn json_result(value: &impl Serialize) -> CallToolResult {
    match serde_json::to_string(value) {
        Ok(json) => CallToolResult::success(vec![ContentBlock::text(json)]),
        Err(error) => tool_error(format!("could not encode the result: {error}")),
    }
}

pub(crate) fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

pub(crate) fn input<T>(parsed: VtopResult<T>) -> Result<T, CallToolResult> {
    parsed.map_err(|error| tool_error(format!("invalid input: {error}")))
}

/// The most days get_biometric_range fetches, one VTOP request each.
const MAX_BIOMETRIC_DAYS: i64 = 14;

/// Files larger than this are not sent to the agent: base64 makes them a
/// third bigger, and few agents read more.
const MAX_FILE_BYTES: usize = 10 * 1024 * 1024;

/// Days since 1970-01-01 of a valid `DD/MM/YYYY` (Howard Hinnant's
/// days_from_civil).
pub(crate) fn day_number(day: i64, month: i64, year: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// (year, month, day) of a day number: civil_from_days.
pub(crate) fn civil_date(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn biometric_day(date: &BiometricDate) -> i64 {
    let mut parts = date
        .as_str()
        .split('/')
        .map(|part| part.parse().unwrap_or(1));
    let (day, month, year) = (
        parts.next().unwrap_or(1),
        parts.next().unwrap_or(1),
        parts.next().unwrap_or(1970),
    );
    day_number(day, month, year)
}

/// Every day from `from` to `to`, both included; at most
/// [`MAX_BIOMETRIC_DAYS`].
pub fn biometric_days(from: &str, to: &str) -> VtopResult<Vec<BiometricDate>> {
    let (first, last) = (
        biometric_day(&BiometricDate::parse(from)?),
        biometric_day(&BiometricDate::parse(to)?),
    );
    if last < first {
        return Err(VtopError::ConfigurationError(
            "to must not be before from".to_string(),
        ));
    }
    if last - first >= MAX_BIOMETRIC_DAYS {
        return Err(VtopError::ConfigurationError(format!(
            "at most {MAX_BIOMETRIC_DAYS} days at a time"
        )));
    }
    (first..=last)
        .map(|days| {
            let (year, month, day) = civil_date(days);
            BiometricDate::parse(&format!("{day:02}/{month:02}/{year:04}"))
        })
        .collect()
}

/// A file as the tool result: what it is, as text, then the file itself
/// as an embedded resource.
pub(crate) fn file_result(
    file_name: &str,
    content_type: &str,
    bytes: &[u8],
    uri: &str,
) -> CallToolResult {
    use base64::Engine;
    if bytes.len() > MAX_FILE_BYTES {
        return tool_error(format!(
            "{file_name} is {} MB, too large to hand over (limit {} MB); open it with get_session instead",
            bytes.len() / (1024 * 1024),
            MAX_FILE_BYTES / (1024 * 1024),
        ));
    }
    let about = serde_json::json!({
        "file_name": file_name,
        "content_type": content_type,
        "size_bytes": bytes.len(),
    });
    let blob = base64::engine::general_purpose::STANDARD.encode(bytes);
    CallToolResult::success(vec![
        ContentBlock::text(about.to_string()),
        ContentBlock::resource(ResourceContents::blob(blob, uri).with_mime_type(content_type)),
    ])
}

#[derive(Clone)]
pub struct VtopTools {
    pub(crate) source: Arc<dyn SessionSource>,
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
}

impl VtopTools {
    pub fn new(source: Arc<dyn SessionSource>) -> Self {
        Self {
            source,
            tool_router: Self::tool_router()
                + Self::more_tools_router()
                + Self::outing_tools_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    /// Fetches with the caller's session and renders the result as JSON.
    pub(crate) async fn fetch<T, F, Fut>(&self, parts: &Parts, call: F) -> CallToolResult
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

    /// [`Self::fetch`] for a download, handed over as an embedded resource.
    async fn fetch_file<F, Fut>(&self, parts: &Parts, uri: &str, call: F) -> CallToolResult
    where
        F: Fn(VtopClient) -> Fut,
        Fut: Future<Output = VtopResult<CourseFile>>,
    {
        let Some(Caller(username)) = parts.extensions.get::<Caller>().cloned() else {
            return tool_error("no caller on the request");
        };
        match with_retry(self.source.as_ref(), &username, call).await {
            Ok(file) => file_result(&file.file_name, &file.content_type, &file.bytes, uri),
            Err(message) => tool_error(message),
        }
    }
}

#[tool_router]
impl VtopTools {
    #[tool(
        description = "List the student's VTOP semesters (ids and names). The one marked current (also current_semester_id) is the semester picked in the student's app; use it unless they name another. Call this first to get a semester_id.",
        annotations(read_only_hint = true)
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

    #[tool(
        description = "Attendance summary for every course in a semester: classes attended, total classes, percentage and debar status (Permitted means allowed into the next exam), with the course_id and course_type that get_full_attendance takes.",
        annotations(read_only_hint = true)
    )]
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

    #[tool(
        description = "Class-by-class attendance for one course: date, slot, day and time, status and remark of each class.",
        annotations(read_only_hint = true)
    )]
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

    #[tool(
        description = "Weekly timetable for a semester: each slot's day, start and end time, course, theory or lab, room, block, faculty and credits, plus the list of registered courses. It is the usual week only; holidays, exam days and other changes are in get_academic_calendar.",
        annotations(read_only_hint = true)
    )]
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

    #[tool(
        description = "Marks for every course in a semester: each assessment (CATs, quizzes, assignments, FAT once published) with max marks, weightage, scored mark and weighted mark.",
        annotations(read_only_hint = true)
    )]
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

    #[tool(
        description = "Exam schedule of a semester, grouped by exam type (CAT-1, CAT-2, FAT): date, session, reporting and exam time, venue and seat for each course. Lab FATs are not listed; get_academic_calendar marks those days.",
        annotations(read_only_hint = true)
    )]
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

    #[tool(
        description = "Final grades for a semester: each course's grand total, grade and grading type (RG relative, AG absolute), with the course_id that get_grade_details takes.",
        annotations(read_only_hint = true)
    )]
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

    #[tool(
        description = "Mark breakdown behind one course's grade, with the grade ranges (mark cutoffs for each grade) of that class.",
        annotations(read_only_hint = true)
    )]
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

    #[tool(
        description = "Full grade history across all semesters: every course with credits, grade, exam month and earlier attempts, plus CGPA, credits registered and earned, and the count of each grade.",
        annotations(read_only_hint = true)
    )]
    async fn get_grade_history(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        self.fetch(&parts, |client| async move { client.grade_history().await })
            .await
    }

    #[tool(
        description = "Biometric punches for one day: time and venue of each entry, at classes and the hostel.",
        annotations(read_only_hint = true)
    )]
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

    #[tool(
        description = "The student's VTOP profile: name, register number, VIT email, program and school, then VTOP's sections in its own wording (PERSONAL INFORMATION with current and permanent address, EDUCATIONAL INFORMATION, FAMILY INFORMATION with parent details, PROCTOR INFORMATION, HOSTEL INFORMATION with block, room, bed type and mess). Each section is groups of label/value fields; empty, '-' or 'NA' means not filled in. The photo is left out and the Aadhaar number shows only its last four digits.",
        annotations(read_only_hint = true)
    )]
    async fn get_profile(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        self.fetch(&parts, |client| async move {
            client.profile().await.map(for_agent)
        })
        .await
    }

    #[tool(
        description = "The student's mentor (proctor, also called faculty adviser): faculty id, name, designation, school, department, cabin, email, intercom and mobile as label/value fields; and the messages the mentor sent in the last week (VTOP removes older ones), each as the message table's column/value pairs. An empty messages list means none.",
        annotations(read_only_hint = true)
    )]
    async fn get_mentor(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        self.fetch(&parts, |client| async move { client.mentor().await })
            .await
    }
}

/// The profile without what an agent does not need: the photo (a large
/// base64 image) and all but the last four digits of the Aadhaar number.
fn for_agent(mut profile: StudentProfile) -> StudentProfile {
    profile.photo.clear();
    for field in profile
        .sections
        .iter_mut()
        .flat_map(|section| &mut section.groups)
        .flat_map(|group| &mut group.fields)
    {
        let label = field.label.to_ascii_uppercase();
        if label.contains("AADHAR") || label.contains("AADHAAR") {
            let digits = field.value.chars().filter(char::is_ascii_digit).count();
            if digits > 4 {
                let tail: String = field
                    .value
                    .chars()
                    .skip(field.value.chars().count() - 4)
                    .collect();
                field.value = format!("{}{tail}", "X".repeat(digits - 4));
            }
        }
    }
    profile
}

#[tool_router(router = more_tools_router)]
impl VtopTools {
    #[tool(
        description = "Academic calendar of a semester, sorted by date: each entry has date (YYYY-MM-DD), kind (Instructional Day, Holiday, No Instructional Day, CAT - I…), group and note (WorkingDay, LAB FAT, a holiday's name…). A day can have several entries.",
        annotations(read_only_hint = true)
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
        description = "Courses on the course page for a semester: id, code, title and type (ETH, ELA…). Use an id with get_course_classes.",
        annotations(read_only_hint = true)
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
        description = "Classes of one course: class_id, erp_id, slot, faculty and school. Use them with get_course_detail.",
        annotations(read_only_hint = true)
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
        description = "Course page of one class: the lecture plan (dates, topics and materials per lecture), syllabus and course plan availability.",
        annotations(read_only_hint = true)
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
        description = "Downloads a file from a class's course page: a lecture's material, the syllabus, or a bundle of all materials. Takes a path from get_course_detail. Returns its name, type and size, then the file itself (usually a PDF, slides or a zip) as an embedded resource. Files over 10 MB are refused.",
        annotations(read_only_hint = true)
    )]
    async fn get_course_file(
        &self,
        Parameters(args): Parameters<CourseFileArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let path = match input(CourseFilePath::parse(&args.path)) {
            Ok(value) => value,
            Err(error) => return error,
        };
        let uri = format!("vtop://course-page/{}", path.as_str());
        self.fetch_file(&parts, &uri, |client| {
            let path = path.clone();
            async move { client.course_page_file(&path).await }
        })
        .await
    }

    #[tool(
        description = "Downloads a class's course plan, an Excel sheet of what each lecture covers, when get_course_detail says has_course_plan. Returned as an embedded resource.",
        annotations(read_only_hint = true)
    )]
    async fn get_course_plan(
        &self,
        Parameters(args): Parameters<CoursePlanArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let parsed = (|| {
            Ok((
                SemesterId::parse(&args.semester_id)?,
                plain_id(&args.class_id, "class id")?,
            ))
        })();
        let (semester, class) = match input(parsed) {
            Ok(value) => value,
            Err(error) => return error,
        };
        let uri = format!("vtop://course-plan/{}/{class}", semester.as_str());
        self.fetch_file(&parts, &uri, |client| {
            let (semester, class) = (semester.clone(), class.clone());
            async move { client.course_plan(&semester, &class).await }
        })
        .await
    }

    #[tool(
        description = "Biometric punches for each day from one date to another (at most 14 days), oldest first: each day's date and its entries, as get_biometric gives them.",
        annotations(read_only_hint = true)
    )]
    async fn get_biometric_range(
        &self,
        Parameters(args): Parameters<DateRangeArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let days = match input(biometric_days(&args.from, &args.to)) {
            Ok(value) => value,
            Err(error) => return error,
        };
        self.fetch(&parts, |client| {
            let days = days.clone();
            async move {
                let mut found = Vec::with_capacity(days.len());
                for date in &days {
                    found.push(client.biometric_history(date).await?);
                }
                Ok(found)
            }
        })
        .await
    }

    #[tool(
        description = "General (day) outing: the form state, student info, and current and past outing records.",
        annotations(read_only_hint = true)
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
        description = "Weekend outing: the form state, student info, and current and past outing records.",
        annotations(read_only_hint = true)
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
        description = "A live, signed-in VTOP session for a browser, so you can open VTOP as the student (for pages no other tool covers). Returns `url`, `cookies` (Playwright `context.add_cookies(cookies)` / Puppeteer `page.setCookie(...cookies)` shape) and `cookie_header` (for curl or requests). To use it: add the cookies to the browser context, then open `url`; it lands on the signed-in VTOP home. If VTOP shows its login page, the session expired: call get_session again. This is the student's full account: anything done in the browser acts as them, so do only what they asked, never print or share the cookies, and confirm before submitting any form.",
        annotations(read_only_hint = false, destructive_hint = false)
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
        description = "Who this access key belongs to: registration number, key label, and outings (whether the student allowed this key to apply for and cancel outings). Does not contact VTOP.",
        annotations(read_only_hint = true)
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
show the cookies; and apply_general_outing, apply_weekend_outing and cancel_outing, \
which change the student's outing requests. Those take two calls: the first only checks \
the request and returns it with a confirmation code. Show the student that request and \
send the code back only after they say yes; never confirm on their behalf. They also need \
the student's permission on this access key (whoami shows outings); without it they \
answer outings_off, and the student can turn it on in the app.

- Call get_semesters first and use the semester marked current (current_semester_id) \
as semester_id unless the student names another. It is the semester picked in the \
student's app, or else the newest, as VTOP lists them newest first.
- Ids chain from one tool to the next: get_attendance gives course_id and course_type \
for get_full_attendance; get_courses gives course_id for get_course_classes, which \
gives erp_id and class_id for get_course_detail.
- Dates: get_biometric and get_biometric_range take DD/MM/YYYY. VTOP returns them in mixed forms: \
01-Jul-2026 in attendance and exams, 18/07/2026 in biometrics, 2026-10-01 in the \
academic calendar. Take \"today\" in India time (IST).
- Attendance is counted per course and type, theory and lab separately; a lab \
session counts once even when it spans two slots. Approved OD counts as present. \
It is checked before each CAT and before the FAT: below 75% in a course then means \
the student is debarred from that next exam; debarred from the FAT, they must \
re-register the course. The percentage is rounded, so 74.5% \
counts as 75%. When asked, work out from the attended and total classes how many \
more the student must attend, or can miss, to stay at 75%. OD is applied by a \
faculty or club coordinator and can show up days later. Attendance is usually posted \
the same day, sometimes right after class and sometimes at night.
- CAT-1 and CAT-2 are the mid-term exams, each covering about two modules of the \
syllabus; no classes run on CAT days. FAT is the final exam over the full syllabus.
- Saturday is usually a class day and Sunday a holiday. Before saying whether a day \
has classes, check get_academic_calendar for that date: it marks holidays, exam days \
and other changes to the usual timetable.
- Lab FATs are held in the lab slots of that day's timetable, so the student sits the \
lab exam in place of that lab class; theory classes that day happen as usual. Lab FAT \
days are marked only in get_academic_calendar, not in get_exam_schedule.
- Timetable slots: letter slots such as A1, B2 or TA1 are theory, 50 minutes each; \
L slots are lab, and a lab session is two back-to-back L slots (100 minutes).
- Slot times differ by day and slot: read them from get_timetable, never assume them.
- Course types: ETH, ELA and EPJ are the theory, lab and project parts of one \
embedded course; TH, LO and PJT are standalone theory, lab and project courses; SS \
is soft skills. Credits are typically theory 3, lab 1 and embedded 4; read the real \
value from get_timetable or get_grade_history.
- Before each semester, students register on VTOP (FFCS), picking their courses, \
slots and faculty.
- A year has a Fall and a Winter semester, plus a short summer semester.
- Theory marks are typically CAT-1 15, CAT-2 15, continuous assessment (quizzes, \
assignments) 30 and FAT 40, out of 100, but a course can change this, so read the \
weights from get_marks. Lab courses are typically continuous assessment 60 and lab \
FAT 40. Passing also needs at least 40% in the FAT itself.
- Grading is usually relative (S, A, B, C, D, E, F), with absolute grading for small \
classes. CGPA is credit-weighted, with grade points S=10, A=9, B=8, C=7, D=6, E=5 \
and F=0. A student who fails a course, or misses its FAT without an approved reason, \
must re-register it in a later semester; a passed course can also be re-registered to \
improve the grade. An approved medical or OD absence can get a re-CAT or re-FAT.
- Faculty names, emails and cabins from VTOP may be shown to the student.
- The mentor (proctor or faculty adviser) is the student's first contact for \
attendance, OD, leave and academic problems; get_mentor gives their cabin, email \
and mobile, and the messages they sent this week. get_profile has the student's \
own details, addresses, parents, school before VIT and hostel room; share only what \
the student asks for, and do not repeat personal details (date of birth, family, \
religion, caste, income) unless asked.
- Outings: a general outing is a leave; a weekend outing means going out in the \
morning and returning by night. There is no fixed limit on outings. Outing dates are \
DD-Mon-YYYY (11-Oct-2026) and general outing times 24-hour HH:MM. A weekend outing's \
place and time_slot are option values from get_weekend_outing. Once a request is \
accepted, its pass_id gives the PDF pass through get_outing_pass.
- Files: get_course_file takes a path from get_course_detail (a lecture's materials, \
syllabus_path, all_materials_path) and returns the file as an embedded resource; \
get_course_plan gives the course plan sheet.
- The biometric log records entries at both classes and the hostel. Hostellers must \
be back by 8:30 pm and may leave from 5 am (boys) or 6 am (girls); these times can \
change.
- Common questions:
  - Classes today or on a date: get_timetable for that weekday, then \
get_academic_calendar for that date.
  - Can I skip a class: the day's classes from get_timetable, each course's \
attended and total from get_attendance, then the 75% calculation.
  - Next exam: get_exam_schedule for CATs and theory FATs, get_academic_calendar \
for lab FATs.
  - Marks needed in the FAT: the weighted marks so far from get_marks, the FAT's \
weightage, the 40% FAT minimum, and the grade ranges from get_grade_details once \
grades exist.
- The first call can take up to a minute while the student's phone or the server \
signs in to VTOP; wait for it rather than retrying.
- Errors: key_unknown means the access key was revoked; phone_unreachable or \
phone_timeout means the student should open the VITAP Mate app; rate_limited means \
wait a minute.";

#[tool_handler(router = self.tool_router)]
#[prompt_handler(router = self.prompt_router)]
impl ServerHandler for VtopTools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
        )
        .with_instructions(INSTRUCTIONS)
    }
}

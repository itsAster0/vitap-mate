//! Request handlers. Every data endpoint takes a JSON body with the caller's
//! `session` (at least `{"cookies": "..."}`) plus its own parameters, and
//! answers with the matching vtop-core type as JSON. The refreshed session
//! comes back base64url-encoded in the `X-Vtop-Session` header so the caller
//! can skip validation next time.

use std::future::Future;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request, State};
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use vtop_core::gmail::GmailAccess;
use vtop_core::inputs::{BiometricDate, CourseId, CourseType, Password, SemesterId, Username};
use vtop_core::{SessionState, VtopClient, VtopError, VtopResult};

use crate::error::ApiError;
use crate::AppState;

pub const SESSION_HEADER: &str = "x-vtop-session";

/// `Json` with our error body on a bad request. The message stays generic
/// because serde errors can quote the offending value.
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(request, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(JsonRejection::MissingJsonContentType(_)) => Err(ApiError::BadRequest(
                "expected Content-Type: application/json".into(),
            )),
            Err(_) => Err(ApiError::BadRequest(
                "request body is not the expected JSON shape".into(),
            )),
        }
    }
}

#[derive(Deserialize)]
pub struct SessionOnly {
    session: SessionState,
}

#[derive(Deserialize)]
pub struct SemesterRequest {
    session: SessionState,
    semester_id: String,
}

#[derive(Deserialize)]
pub struct GradeDetailsRequest {
    session: SessionState,
    semester_id: String,
    course_id: String,
}

#[derive(Deserialize)]
pub struct FullAttendanceRequest {
    session: SessionState,
    semester_id: String,
    course_id: String,
    course_type: String,
}

#[derive(Deserialize)]
pub struct BiometricRequest {
    session: SessionState,
    /// `DD/MM/YYYY`
    date: String,
}

/// No `Debug`: this holds a password.
#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
    /// Lets the server answer VTOP's emailed OTP itself. Used for this
    /// request only; never stored or logged.
    #[serde(default)]
    gmail: Option<GmailAccess>,
}

/// No `Debug`: this holds an OTP.
#[derive(Deserialize)]
pub struct OtpRequest {
    session: SessionState,
    otp: String,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AuthResponse {
    Authenticated {
        session: SessionState,
    },
    OtpRequired {
        session: SessionState,
        message: String,
        issued_at: u64,
        /// Why Gmail did not supply the OTP when a token was sent:
        /// `gmail_unauthorized` (refresh the token), `gmail_forbidden`,
        /// `gmail_otp_not_found` or `gmail_unavailable`.
        #[serde(skip_serializing_if = "Option::is_none")]
        gmail: Option<&'static str>,
    },
    OtpSent {
        session: SessionState,
    },
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

fn session_header(state: &SessionState) -> Option<HeaderValue> {
    let json = serde_json::to_vec(state).ok()?;
    HeaderValue::from_str(&URL_SAFE_NO_PAD.encode(json)).ok()
}

/// Runs `fetch` against a client for `session`, reusing a cached one when
/// the cookie was seen recently.
async fn with_session<T, F, Fut>(
    state: &AppState,
    session: SessionState,
    fetch: F,
) -> Result<Response, ApiError>
where
    T: Serialize,
    F: FnOnce(Arc<VtopClient>) -> Fut,
    Fut: Future<Output = VtopResult<T>>,
{
    with_session_within(state, session, state.config.request_timeout, fetch).await
}

async fn with_session_within<T, F, Fut>(
    state: &AppState,
    session: SessionState,
    limit: std::time::Duration,
    fetch: F,
) -> Result<Response, ApiError>
where
    T: Serialize,
    F: FnOnce(Arc<VtopClient>) -> Fut,
    Fut: Future<Output = VtopResult<T>>,
{
    if session.cookies.trim().is_empty() {
        return Err(ApiError::BadRequest("session.cookies is required".into()));
    }
    let cookies = session.cookies.clone();
    let client = state.cache.client_for(&cookies, || {
        VtopClient::builder()
            .config(state.config.vtop.clone())
            .with_session(&session)
    })?;

    let result = tokio::time::timeout(limit, fetch(client.clone()))
        .await
        .map_err(|_| ApiError::Timeout)?;
    match result {
        Ok(data) => {
            state.cache.put(&cookies, client.clone());
            let mut response = Json(data).into_response();
            if let Some(value) = session_header(&client.session_state()) {
                response.headers_mut().insert(SESSION_HEADER, value);
            }
            Ok(response)
        }
        Err(error) => {
            // Drop a client that never signed in or whose session died, so
            // the next request starts fresh.
            if error == VtopError::SessionExpired || !client.is_authenticated() {
                state.cache.remove(&cookies);
            }
            Err(error.into())
        }
    }
}

type AppResult = Result<Response, ApiError>;

pub async fn semesters(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<SessionOnly>,
) -> AppResult {
    with_session(&state, body.session, |client| async move {
        client.semesters().await
    })
    .await
}

pub async fn attendance(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<SemesterRequest>,
) -> AppResult {
    let semester = SemesterId::parse(&body.semester_id)?;
    with_session(&state, body.session, |client| async move {
        client.attendance(&semester).await
    })
    .await
}

pub async fn full_attendance(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<FullAttendanceRequest>,
) -> AppResult {
    let semester = SemesterId::parse(&body.semester_id)?;
    let course = CourseId::parse(&body.course_id)?;
    let course_type = CourseType::parse(&body.course_type)?;
    with_session(&state, body.session, |client| async move {
        client
            .full_attendance(&semester, &course, &course_type)
            .await
    })
    .await
}

pub async fn timetable(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<SemesterRequest>,
) -> AppResult {
    let semester = SemesterId::parse(&body.semester_id)?;
    with_session(&state, body.session, |client| async move {
        client.timetable(&semester).await
    })
    .await
}

pub async fn marks(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<SemesterRequest>,
) -> AppResult {
    let semester = SemesterId::parse(&body.semester_id)?;
    with_session(&state, body.session, |client| async move {
        client.marks(&semester).await
    })
    .await
}

pub async fn exam_schedule(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<SemesterRequest>,
) -> AppResult {
    let semester = SemesterId::parse(&body.semester_id)?;
    with_session(&state, body.session, |client| async move {
        client.exam_schedule(&semester).await
    })
    .await
}

pub async fn grades(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<SemesterRequest>,
) -> AppResult {
    let semester = SemesterId::parse(&body.semester_id)?;
    with_session(&state, body.session, |client| async move {
        client.grade_view(&semester).await
    })
    .await
}

pub async fn grade_details(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<GradeDetailsRequest>,
) -> AppResult {
    let semester = SemesterId::parse(&body.semester_id)?;
    let course = CourseId::parse(&body.course_id)?;
    with_session(&state, body.session, |client| async move {
        client.grade_view_details(&semester, &course).await
    })
    .await
}

pub async fn grade_history(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<SessionOnly>,
) -> AppResult {
    with_session(&state, body.session, |client| async move {
        client.grade_history().await
    })
    .await
}

pub async fn biometric(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<BiometricRequest>,
) -> AppResult {
    let date = BiometricDate::parse(&body.date)?;
    with_session(&state, body.session, |client| async move {
        client.biometric_history(&date).await
    })
    .await
}

// ---- batch refresh --------------------------------------------------------

/// A page `/v1/refresh` can include.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshPart {
    Semesters,
    Attendance,
    Timetable,
    Marks,
    ExamSchedule,
    Grades,
    /// Per-class attendance detail for every course on the attendance page.
    FullAttendance,
}

impl RefreshPart {
    /// Order of work. Full attendance comes last so it can reuse the
    /// attendance page fetched in the same call.
    const ALL: [Self; 7] = [
        Self::Semesters,
        Self::Attendance,
        Self::Timetable,
        Self::Marks,
        Self::ExamSchedule,
        Self::Grades,
        Self::FullAttendance,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Semesters => "semesters",
            Self::Attendance => "attendance",
            Self::Timetable => "timetable",
            Self::Marks => "marks",
            Self::ExamSchedule => "exam_schedule",
            Self::Grades => "grades",
            Self::FullAttendance => "full_attendance",
        }
    }
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    session: SessionState,
    semester_id: String,
    /// Defaults to every part except `full_attendance`, which costs one VTOP
    /// request per course.
    #[serde(default)]
    include: Option<Vec<RefreshPart>>,
}

fn to_json(value: impl Serialize) -> VtopResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|_| VtopError::InvalidResponse)
}

/// Fetches several pages in one call and answers
/// `{"attendance": {"data": …}, "marks": {"error": {…}}, …}`.
///
/// Pages are fetched at the same time over the shared warm connection pool
/// (measured much faster than one after another once connections are
/// warm), then per-course attendance details `detail_concurrency` at a
/// time. An expired session fails the whole call with 401 so the caller
/// can sign in again; any other failure only marks that part.
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<RefreshRequest>,
) -> AppResult {
    let semester = SemesterId::parse(&body.semester_id)?;
    let requested = body.include.unwrap_or_else(|| {
        RefreshPart::ALL
            .into_iter()
            .filter(|part| *part != RefreshPart::FullAttendance)
            .collect()
    });
    let parts: Vec<RefreshPart> = RefreshPart::ALL
        .into_iter()
        .filter(|part| requested.contains(part))
        .collect();
    let limit = state.config.login_timeout;
    let concurrency = state.config.detail_concurrency;
    with_session_within(&state, body.session, limit, |client| async move {
        use futures::stream::{self, StreamExt};

        type PageFetch<'a> =
            std::pin::Pin<Box<dyn Future<Output = VtopResult<serde_json::Value>> + Send + 'a>>;

        // All pages at once over the shared warm pool; per-course details
        // afterwards, reusing the attendance page.
        let wants_attendance = parts.contains(&RefreshPart::Attendance)
            || parts.contains(&RefreshPart::FullAttendance);
        let attendance_fetch = async {
            if wants_attendance {
                Some(client.attendance(&semester).await)
            } else {
                None
            }
        };
        let pages: Vec<(RefreshPart, PageFetch<'_>)> = parts
            .iter()
            .filter_map(|part| {
                let fetch: PageFetch<'_> = match part {
                    RefreshPart::Semesters => {
                        Box::pin(async { client.semesters().await.and_then(to_json) })
                    }
                    RefreshPart::Timetable => {
                        Box::pin(async { client.timetable(&semester).await.and_then(to_json) })
                    }
                    RefreshPart::Marks => {
                        Box::pin(async { client.marks(&semester).await.and_then(to_json) })
                    }
                    RefreshPart::ExamSchedule => {
                        Box::pin(async { client.exam_schedule(&semester).await.and_then(to_json) })
                    }
                    RefreshPart::Grades => {
                        Box::pin(async { client.grade_view(&semester).await.and_then(to_json) })
                    }
                    RefreshPart::Attendance | RefreshPart::FullAttendance => return None,
                };
                Some((*part, fetch))
            })
            .collect();
        let (page_parts, fetches): (Vec<_>, Vec<_>) = pages.into_iter().unzip();
        // Bounded so one refresh never opens more than `concurrency`
        // connections to VTOP at once.
        let (attendance, page_results) = tokio::join!(
            attendance_fetch,
            stream::iter(fetches)
                .buffered(concurrency.saturating_sub(1).max(1))
                .collect::<Vec<_>>()
        );

        let mut outcomes: Vec<(RefreshPart, VtopResult<serde_json::Value>)> =
            page_parts.into_iter().zip(page_results).collect();
        let attendance = match attendance {
            Some(Ok(data)) => Some(data),
            Some(Err(error)) => {
                outcomes.push((RefreshPart::Attendance, Err(error)));
                None
            }
            None => None,
        };
        if parts.contains(&RefreshPart::Attendance) {
            if let Some(data) = &attendance {
                outcomes.push((RefreshPart::Attendance, to_json(data)));
            }
        }
        if parts.contains(&RefreshPart::FullAttendance) {
            let details = match attendance {
                Some(data) => {
                    full_attendance_for_all(&client, &semester, Some(data), concurrency).await
                }
                None => Err(VtopError::VtopServerError(
                    "attendance page unavailable".to_string(),
                )),
            };
            outcomes.push((RefreshPart::FullAttendance, details));
        }

        let mut results = serde_json::Map::new();
        for (part, outcome) in outcomes {
            let entry = match outcome {
                Ok(data) => serde_json::json!({ "data": data }),
                Err(VtopError::SessionExpired) => return Err(VtopError::SessionExpired),
                Err(error) => {
                    let (code, message) = ApiError::Vtop(error).code_and_message();
                    serde_json::json!({ "error": { "code": code, "message": message } })
                }
            };
            results.insert(part.key().to_string(), entry);
        }
        Ok(serde_json::Value::Object(results))
    })
    .await
}

/// Fetches the attendance detail of every course listed on the attendance
/// page (reusing `attendance` when this call already fetched it),
/// `concurrency` at a time over the shared connection pool. Courses whose
/// detail fails are left out, so the caller fetches just those one by one;
/// an expired session still fails the whole call.
async fn full_attendance_for_all(
    client: &VtopClient,
    semester: &SemesterId,
    attendance: Option<vtop_core::types::AttendanceData>,
    concurrency: usize,
) -> VtopResult<serde_json::Value> {
    use futures::stream::{self, StreamExt};

    let attendance = match attendance {
        Some(attendance) => attendance,
        None => client.attendance(semester).await?,
    };
    let courses: Vec<(CourseId, CourseType)> = attendance
        .records
        .iter()
        .filter_map(|record| {
            Some((
                CourseId::parse(&record.course_id).ok()?,
                CourseType::parse(&record.course_type).ok()?,
            ))
        })
        .collect();
    // Boxed so the handler future stays Send without a higher-ranked
    // closure (axum requires Send handlers).
    type Detail<'a> = std::pin::Pin<
        Box<dyn Future<Output = VtopResult<vtop_core::types::FullAttendanceData>> + Send + 'a>,
    >;
    let fetches: Vec<Detail<'_>> = courses
        .iter()
        .map(|(course, course_type)| {
            Box::pin(client.full_attendance(semester, course, course_type)) as Detail<'_>
        })
        .collect();
    let results: Vec<_> = stream::iter(fetches)
        .buffered(concurrency.max(1))
        .collect()
        .await;
    let mut details = Vec::with_capacity(results.len());
    for result in results {
        match result {
            Ok(detail) => details.push(detail),
            Err(VtopError::SessionExpired) => return Err(VtopError::SessionExpired),
            Err(_) => {}
        }
    }
    to_json(details)
}

// ---- stateless login ------------------------------------------------------
//
// The server logs in on the caller's behalf and hands the whole session back.
// It keeps nothing: the next call must carry the returned `session`.

async fn with_login_timeout<T>(
    state: &AppState,
    work: impl Future<Output = T>,
) -> Result<T, ApiError> {
    tokio::time::timeout(state.config.login_timeout, work)
        .await
        .map_err(|_| ApiError::Timeout)
}

fn login_client(state: &AppState, session: &SessionState) -> Result<VtopClient, ApiError> {
    Ok(VtopClient::builder()
        .config(state.config.vtop.clone())
        .with_session(session)?)
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    if !state.config.allow_login {
        return Err(ApiError::LoginDisabled);
    }
    let mut client = VtopClient::builder()
        .config(state.config.vtop.clone())
        .with_credentials(
            Username::parse(&body.username)?,
            Password::parse(&body.password)?,
        )?;
    let gmail = body.gmail;
    let wait = state.config.gmail_wait;

    let outcome =
        with_login_timeout(&state, client.login_with_gmail_otp(gmail.as_ref(), wait)).await?;
    drop(gmail);
    match outcome.result {
        Ok(()) => Ok(Json(AuthResponse::Authenticated {
            session: client.session_state(),
        })),
        Err(VtopError::OTPRequired(message, issued_at)) => Ok(Json(AuthResponse::OtpRequired {
            session: client.session_state(),
            message,
            issued_at,
            gmail: outcome.gmail_error.map(|error| error.code()),
        })),
        Err(error) => Err(error.into()),
    }
}

pub async fn submit_otp(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<OtpRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    if !state.config.allow_login {
        return Err(ApiError::LoginDisabled);
    }
    let mut client = login_client(&state, &body.session)?;
    with_login_timeout(&state, client.submit_security_otp(&body.otp)).await??;
    Ok(Json(AuthResponse::Authenticated {
        session: client.session_state(),
    }))
}

pub async fn resend_otp(
    State(state): State<Arc<AppState>>,
    ApiJson(body): ApiJson<SessionOnly>,
) -> Result<Json<AuthResponse>, ApiError> {
    if !state.config.allow_login {
        return Err(ApiError::LoginDisabled);
    }
    let mut client = login_client(&state, &body.session)?;
    with_login_timeout(&state, client.resend_security_otp()).await??;
    Ok(Json(AuthResponse::OtpSent {
        session: client.session_state(),
    }))
}

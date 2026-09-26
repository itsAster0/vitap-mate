//! The Dart-facing VTOP API. Each function validates its input and hands off
//! to vtop-core.
//!
//! Fetches borrow the client immutably, so flutter_rust_bridge takes a read
//! lock and several fetches can run at once. Login and OTP take `&mut`.

use vtop_core::inputs::{BiometricDate, CourseId, CourseType, Password, SemesterId, Username};

use crate::api::vtop::{
    types::{
        AcademicCalendarData, AttendanceData, BiometricData, ExamScheduleData, FullAttendanceData,
        GradeDetailsData, GradeHistoryData, GradeViewData, MarksData, PersistedVtopSession,
        SemesterData, SessionState, TimetableData,
    },
    vtop_client::{VtopClient, VtopError},
};

#[flutter_rust_bridge::frb]
pub async fn get_vtop_client(
    username: String,
    password: String,
    persisted_session: Option<PersistedVtopSession>,
) -> Result<VtopClient, VtopError> {
    let inner = vtop_core::VtopClient::builder()
        .with_credentials(Username::parse(&username)?, Password::parse(&password)?)?;
    if let Some(session) = persisted_session {
        inner.restore_persisted_session(&session)?;
    }
    Ok(VtopClient { inner })
}

#[flutter_rust_bridge::frb()]
pub async fn vtop_client_login(client: &mut VtopClient) -> Result<(), VtopError> {
    client.inner.login().await
}

#[flutter_rust_bridge::frb()]
pub async fn vtop_client_submit_security_otp(
    client: &mut VtopClient,
    otp_code: String,
) -> Result<(), VtopError> {
    client.inner.submit_security_otp(&otp_code).await
}

#[flutter_rust_bridge::frb()]
pub async fn vtop_client_resend_security_otp(client: &mut VtopClient) -> Result<(), VtopError> {
    client.inner.resend_security_otp().await
}

#[flutter_rust_bridge::frb(sync)]
pub fn vtop_client_registration_number(client: &VtopClient) -> Result<String, VtopError> {
    client.inner.registration_number()
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_semesters(client: &VtopClient) -> Result<SemesterData, VtopError> {
    client.inner.semesters().await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_attendance(
    client: &VtopClient,
    semester_id: String,
) -> Result<AttendanceData, VtopError> {
    client
        .inner
        .attendance(&SemesterId::parse(&semester_id)?)
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_biometric_history(
    client: &VtopClient,
    date: String,
) -> Result<BiometricData, VtopError> {
    client
        .inner
        .biometric_history(&BiometricDate::parse(&date)?)
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_full_attendance(
    client: &VtopClient,
    semester_id: String,
    course_id: String,
    course_type: String,
) -> Result<FullAttendanceData, VtopError> {
    client
        .inner
        .full_attendance(
            &SemesterId::parse(&semester_id)?,
            &CourseId::parse(&course_id)?,
            &CourseType::parse(&course_type)?,
        )
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_timetable(
    client: &VtopClient,
    semester_id: String,
) -> Result<TimetableData, VtopError> {
    client
        .inner
        .timetable(&SemesterId::parse(&semester_id)?)
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_marks(client: &VtopClient, semester_id: String) -> Result<MarksData, VtopError> {
    client.inner.marks(&SemesterId::parse(&semester_id)?).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_exam_schedule(
    client: &VtopClient,
    semester_id: String,
) -> Result<ExamScheduleData, VtopError> {
    client
        .inner
        .exam_schedule(&SemesterId::parse(&semester_id)?)
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_academic_calendar(
    client: &VtopClient,
    semester_id: String,
) -> Result<AcademicCalendarData, VtopError> {
    client
        .inner
        .academic_calendar(&SemesterId::parse(&semester_id)?)
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_grade_view(
    client: &VtopClient,
    semester_id: String,
) -> Result<GradeViewData, VtopError> {
    client
        .inner
        .grade_view(&SemesterId::parse(&semester_id)?)
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_grade_view_details(
    client: &VtopClient,
    semester_id: String,
    course_id: String,
) -> Result<GradeDetailsData, VtopError> {
    client
        .inner
        .grade_view_details(
            &SemesterId::parse(&semester_id)?,
            &CourseId::parse(&course_id)?,
        )
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_grade_history(client: &VtopClient) -> Result<GradeHistoryData, VtopError> {
    client.inner.grade_history().await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_cookies(client: &VtopClient) -> Result<Vec<u8>, VtopError> {
    client.inner.cookie_bytes(true)
}

#[flutter_rust_bridge::frb(sync)]
pub fn export_session_snapshot(
    client: &VtopClient,
    saved_at_epoch_ms: u64,
) -> PersistedVtopSession {
    client.inner.export_persisted_session(saved_at_epoch_ms)
}

/// The current session with its CSRF token and registration number, for
/// sending to a vtop-server.
#[flutter_rust_bridge::frb(sync)]
pub fn export_session_state(client: &VtopClient) -> SessionState {
    client.inner.session_state()
}

/// Replaces the client's session with one logged in elsewhere (a
/// vtop-server login), including a pending OTP challenge.
#[flutter_rust_bridge::frb(sync)]
pub fn vtop_client_resume_session(
    client: &VtopClient,
    session: SessionState,
) -> Result<(), VtopError> {
    client.inner.resume_session(&session)
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_is_auth(client: &VtopClient) -> bool {
    client.inner.is_authenticated()
}

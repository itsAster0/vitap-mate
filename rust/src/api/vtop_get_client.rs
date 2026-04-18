use crate::api::vtop::{
    types::{
        AttendanceData, ExamScheduleData, FullAttendanceData, GradeDetailsData, GradeHistoryData,
        GradeViewData, MarksData, PersistedVtopSession, SemesterData, TimetableData,
    },
    vtop_client::{VtopClient, VtopError},
    vtop_config::VtopClientBuilder,
};

#[flutter_rust_bridge::frb(sync)]
pub fn get_vtop_client(
    username: String,
    password: String,
    persisted_session: Option<PersistedVtopSession>,
) -> VtopClient {
    let mut client = VtopClientBuilder::new().build(username, password);
    if let Some(session) = persisted_session {
        client.restore_session_snapshot(session);
    }
    return client;
}

#[flutter_rust_bridge::frb()]
pub async fn vtop_client_login(client: &mut VtopClient) -> Result<(), VtopError> {
    client.login().await
}

#[flutter_rust_bridge::frb()]
pub async fn vtop_client_submit_security_otp(
    client: &mut VtopClient,
    otp_code: String,
) -> Result<(), VtopError> {
    client.submit_security_otp(&otp_code).await
}

#[flutter_rust_bridge::frb()]
pub async fn vtop_client_resend_security_otp(client: &mut VtopClient) -> Result<(), VtopError> {
    client.resend_security_otp().await
}
#[flutter_rust_bridge::frb()]
pub async fn fetch_semesters(client: &mut VtopClient) -> Result<SemesterData, VtopError> {
    client.get_semesters(true).await
}
#[flutter_rust_bridge::frb()]
pub async fn fetch_attendance(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<AttendanceData, VtopError> {
    client.get_attendance(&semester_id).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_full_attendance(
    client: &mut VtopClient,
    semester_id: String,
    course_id: String,
    course_type: String,
) -> Result<FullAttendanceData, VtopError> {
    client
        .get_full_attendance(&semester_id, &course_id, &course_type)
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_timetable(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<TimetableData, VtopError> {
    client.get_timetable(&semester_id).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_marks(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<MarksData, VtopError> {
    client.get_marks(&semester_id).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_exam_shedule(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<ExamScheduleData, VtopError> {
    client.get_exam_schedule(&semester_id).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_grade_view(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<GradeViewData, VtopError> {
    client.get_grade_view(&semester_id).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_grade_view_details(
    client: &mut VtopClient,
    semester_id: String,
    course_id: String,
) -> Result<GradeDetailsData, VtopError> {
    client
        .get_grade_view_details(&semester_id, &course_id)
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_grade_history(client: &mut VtopClient) -> Result<GradeHistoryData, VtopError> {
    client.get_grade_history().await
}

#[flutter_rust_bridge::frb()]
#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_cookies(client: &mut VtopClient) -> Result<Vec<u8>, VtopError> {
    client.get_cookie(true).await.clone()
}

#[flutter_rust_bridge::frb(sync)]
pub fn export_session_snapshot(
    client: &VtopClient,
    saved_at_epoch_ms: u64,
    expires_at_epoch_ms: u64,
) -> PersistedVtopSession {
    client.export_session_snapshot(saved_at_epoch_ms, expires_at_epoch_ms)
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_is_auth(client: &mut VtopClient) -> bool {
    client.is_authenticated().clone()
}

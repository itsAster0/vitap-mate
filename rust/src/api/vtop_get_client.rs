use crate::api::logging::{log_debug, log_info, log_warn};
use crate::api::vtop::{
    types::{
        AttendanceData, ExamScheduleData, FullAttendanceData, GradeDetailsData, GradeHistoryData,
        GradeViewData, MarksData, SemesterData, TimetableData,
    },
    vtop_client::{VtopClient, VtopError},
    vtop_config::VtopClientBuilder,
};

#[flutter_rust_bridge::frb(sync)]
pub fn get_vtop_client(username: String, password: String, cookie: Option<String>) -> VtopClient {
    log_info("bridge", format!("Creating VTOP client for {username}"));
    let mut client = VtopClientBuilder::new().build(username, password);
    if let Some(cookie) = cookie {
        if (!cookie.is_empty()) {
            client.set_cookie(cookie);
        }
    }
    return client;
}

#[flutter_rust_bridge::frb()]
pub async fn vtop_client_login(client: &mut VtopClient) -> Result<(), VtopError> {
    log_info("bridge", "Attempting VTOP login");
    client.login().await
}
#[flutter_rust_bridge::frb()]
pub async fn fetch_semesters(client: &mut VtopClient) -> Result<SemesterData, VtopError> {
    log_debug("bridge", "Fetching semesters");
    client.get_semesters(true).await
}
#[flutter_rust_bridge::frb()]
pub async fn fetch_attendance(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<AttendanceData, VtopError> {
    log_debug("bridge", format!("Fetching attendance for {semester_id}"));
    client.get_attendance(&semester_id).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_full_attendance(
    client: &mut VtopClient,
    semester_id: String,
    course_id: String,
    course_type: String,
) -> Result<FullAttendanceData, VtopError> {
    log_debug(
        "bridge",
        format!("Fetching full attendance for {semester_id}/{course_id}/{course_type}"),
    );
    client
        .get_full_attendance(&semester_id, &course_id, &course_type)
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_timetable(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<TimetableData, VtopError> {
    log_debug("bridge", format!("Fetching timetable for {semester_id}"));
    client.get_timetable(&semester_id).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_marks(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<MarksData, VtopError> {
    log_debug("bridge", format!("Fetching marks for {semester_id}"));
    client.get_marks(&semester_id).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_exam_shedule(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<ExamScheduleData, VtopError> {
    log_debug("bridge", format!("Fetching exam schedule for {semester_id}"));
    client.get_exam_schedule(&semester_id).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_grade_view(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<GradeViewData, VtopError> {
    log_debug("bridge", format!("Fetching grades for {semester_id}"));
    client.get_grade_view(&semester_id).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_grade_view_details(
    client: &mut VtopClient,
    semester_id: String,
    course_id: String,
) -> Result<GradeDetailsData, VtopError> {
    log_debug(
        "bridge",
        format!("Fetching grade details for {semester_id}/{course_id}"),
    );
    client
        .get_grade_view_details(&semester_id, &course_id)
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_grade_history(client: &mut VtopClient) -> Result<GradeHistoryData, VtopError> {
    log_debug("bridge", "Fetching grade history");
    client.get_grade_history().await
}

#[flutter_rust_bridge::frb()]
#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_cookies(client: &mut VtopClient) -> Result<Vec<u8>, VtopError> {
    log_debug("bridge", "Fetching VTOP cookies");
    client.get_cookie(true).await.clone()
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_is_auth(client: &mut VtopClient) -> bool {
    let is_authenticated = client.is_authenticated().clone();
    if !is_authenticated {
        log_warn("bridge", "VTOP client is not authenticated");
    }
    is_authenticated
}

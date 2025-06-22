use crate::api::vtop::{
    types::{
        AttendanceData, ExamScheduleData, FullAttendanceData, MarksData, SemesterData,
        TimetableData,
    },
    vtop_client::{VtopClient, VtopError},
    vtop_config::VtopClientBuilder,
    wifi::*,
};

#[flutter_rust_bridge::frb(sync)]
pub fn get_vtop_client(username: String, password: String) -> VtopClient {
    VtopClientBuilder::new().build(username, password)
}

#[flutter_rust_bridge::frb()]
pub async fn vtop_client_login(client: &mut VtopClient) -> Result<(), VtopError> {
    client.login().await
}
#[flutter_rust_bridge::frb()]
pub async fn fetch_semesters(client: &mut VtopClient) -> Result<SemesterData, VtopError> {
    client.get_semesters().await
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
#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_cookies(client: &mut VtopClient) -> Result<Vec<u8>, VtopError> {
    client.get_cookie().await.clone()
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_is_auth(client: &mut VtopClient) -> bool {
    client.is_authenticated().clone()
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_wifi(username: String, password: String, i: i32) -> (bool, String) {
    wifi_login_logout(i, username, password).await
}

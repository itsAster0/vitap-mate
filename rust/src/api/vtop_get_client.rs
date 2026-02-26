use crate::api::vtop::{
    paraser::wifi_portal::find_captivative_portal,
    types::{
        AttendanceData, ExamScheduleData, FullAttendanceData, GradeDetailsData, GradeViewData,
        GradeHistoryData, MarksData, SemesterData, TimetableData,
    },
    vtop_client::{VtopClient, VtopError},
    vtop_config::VtopClientBuilder,
    wifi::*,
};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::task;

#[flutter_rust_bridge::frb(sync)]
pub fn get_vtop_client(username: String, password: String, cookie: Option<String>) -> VtopClient {
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
    client.login().await
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

#[flutter_rust_bridge::frb()]
pub async fn fetch_is_auth(client: &mut VtopClient) -> bool {
    client.is_authenticated().clone()
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_wifi(username: String, password: String, i: i32) -> (bool, String) {
    if (i != 0) {
        // let k = wifi_login_logout(i, username.clone(), password.clone()).await;
        // if (k.0) {
        //     return k;
        // } else {
        //     return wifi_login_logout_hostel(i, username, password).await;
        // }
        let mut tasks = FuturesUnordered::new();

        let username_clone = username.clone();
        let password_clone = password.clone();
        tasks.push(task::spawn(async move {
            (
                "non_hostel",
                wifi_login_logout(i, username_clone, password_clone).await,
            )
        }));

        let username_clone_2 = username.clone();
        let password_clone_2 = password.clone();
        tasks.push(task::spawn(async move {
            (
                "hostel",
                wifi_login_logout_hostel(i, username_clone_2, password_clone_2).await,
            )
        }));

        while let Some(result) = tasks.next().await {
            match result {
                Ok((_task_name, (is_success, message))) => {
                    if is_success {
                        return (is_success, message);
                    }

                    if tasks.is_empty() {
                        return (is_success, message);
                    }
                }
                Err(_) => {}
            }
        }
        return (false, "Unknow Error".to_string());
    }

    let captive = find_captivative_portal().await;

    if (captive.contains("hfw.vitap.ac.in")) {
        return wifi_login_logout_hostel(i, username, password).await;
    } else if (captive == "IE") {
        return (
            false,
            "You are already connected to the internet.".to_string(),
        );
    } else {
        wifi_login_logout(i, username, password).await
    }
}

//! The Dart-facing VTOP API. Each function validates its input and hands off
//! to vtop-core.
//!
//! Fetches borrow the client immutably, so flutter_rust_bridge takes a read
//! lock and several fetches can run at once. Login and OTP take `&mut`.

use vtop_core::client::{GeneralOutingApplication, OutingKind, WeekendOutingApplication};
use vtop_core::inputs::{
    BiometricDate, ContactNumber, CourseFilePath, CourseId, CourseType, OutingDate, OutingPassId,
    OutingTime, Password, SemesterId, Username,
};

use crate::api::vtop::{
    types::{
        AcademicCalendarData, AttendanceData, BiometricData, CourseFileInfo, CoursePageClasses,
        CoursePageCourses, CoursePageDetail, ExamScheduleData, FullAttendanceData,
        GeneralOutingData, GradeDetailsData, GradeHistoryData, GradeViewData, MarksData,
        OutingApplyResult, OutingCancelResult, PersistedVtopSession, SemesterData, SessionState,
        TimetableData, WeekendOutingData,
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
pub async fn fetch_general_outing(client: &VtopClient) -> Result<GeneralOutingData, VtopError> {
    client.inner.general_outing().await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_weekend_outing(client: &VtopClient) -> Result<WeekendOutingData, VtopError> {
    client.inner.weekend_outing().await
}

/// Dates are `DD-Mon-YYYY` (`11-Oct-2026`); times are 24-hour.
#[allow(clippy::too_many_arguments)]
#[flutter_rust_bridge::frb()]
pub async fn apply_general_outing(
    client: &VtopClient,
    place: String,
    purpose: String,
    out_date: String,
    out_hour: u8,
    out_minute: u8,
    in_date: String,
    in_hour: u8,
    in_minute: u8,
) -> Result<OutingApplyResult, VtopError> {
    let application = GeneralOutingApplication {
        place,
        purpose,
        out_date: OutingDate::parse(&out_date)?,
        out_time: OutingTime::new(out_hour, out_minute)?,
        in_date: OutingDate::parse(&in_date)?,
        in_time: OutingTime::new(in_hour, in_minute)?,
    };
    client.inner.apply_general_outing(&application).await
}

/// `place` and `time_slot` are option values from [`WeekendOutingData`].
#[flutter_rust_bridge::frb()]
pub async fn apply_weekend_outing(
    client: &VtopClient,
    place: String,
    purpose: String,
    date: String,
    time_slot: String,
    contact_number: String,
) -> Result<OutingApplyResult, VtopError> {
    let application = WeekendOutingApplication {
        place,
        purpose,
        date: OutingDate::parse(&date)?,
        time_slot,
        contact_number: ContactNumber::parse(&contact_number)?,
    };
    client.inner.apply_weekend_outing(&application).await
}

/// Cancels a general outing request while VTOP still allows it.
#[flutter_rust_bridge::frb()]
pub async fn cancel_general_outing(
    client: &VtopClient,
    leave_id: String,
) -> Result<OutingCancelResult, VtopError> {
    client
        .inner
        .cancel_general_outing(&OutingPassId::parse(&leave_id)?)
        .await
}

/// Cancels a weekend outing request while VTOP still allows it.
#[flutter_rust_bridge::frb()]
pub async fn cancel_weekend_outing(
    client: &VtopClient,
    booking_id: String,
) -> Result<OutingCancelResult, VtopError> {
    client
        .inner
        .cancel_weekend_outing(&OutingPassId::parse(&booking_id)?)
        .await
}

/// The PDF pass of an accepted outing.
#[flutter_rust_bridge::frb()]
pub async fn fetch_outing_pass(
    client: &VtopClient,
    weekend: bool,
    pass_id: String,
) -> Result<Vec<u8>, VtopError> {
    let kind = if weekend {
        OutingKind::Weekend
    } else {
        OutingKind::General
    };
    client
        .inner
        .outing_pass(kind, &OutingPassId::parse(&pass_id)?)
        .await
}

/// The courses the student is registered for in [semester_id].
#[flutter_rust_bridge::frb()]
pub async fn fetch_course_page_courses(
    client: &VtopClient,
    semester_id: String,
) -> Result<CoursePageCourses, VtopError> {
    client
        .inner
        .course_page_courses(&SemesterId::parse(&semester_id)?)
        .await
}

/// Every class (section) of a course, with its slot and faculty.
#[flutter_rust_bridge::frb()]
pub async fn fetch_course_page_classes(
    client: &VtopClient,
    semester_id: String,
    course_id: String,
) -> Result<CoursePageClasses, VtopError> {
    client
        .inner
        .course_page_classes(&SemesterId::parse(&semester_id)?, &course_id)
        .await
}

/// A class's lecture plan and downloads.
#[flutter_rust_bridge::frb()]
pub async fn fetch_course_page_detail(
    client: &VtopClient,
    semester_id: String,
    erp_id: String,
    class_id: String,
) -> Result<CoursePageDetail, VtopError> {
    client
        .inner
        .course_page_detail(&SemesterId::parse(&semester_id)?, &erp_id, &class_id)
        .await
}

/// The name and type of a course page download (lecture material,
/// syllabus, material bundle), so the system downloader can save it.
#[flutter_rust_bridge::frb()]
pub async fn fetch_course_file_info(
    client: &VtopClient,
    path: String,
) -> Result<CourseFileInfo, VtopError> {
    client
        .inner
        .course_page_file_info(&CourseFilePath::parse(&path)?)
        .await
}

/// The name and type of a class's course plan (Excel).
#[flutter_rust_bridge::frb()]
pub async fn fetch_course_plan_info(
    client: &VtopClient,
    semester_id: String,
    class_id: String,
) -> Result<CourseFileInfo, VtopError> {
    client
        .inner
        .course_plan_info(&SemesterId::parse(&semester_id)?, &class_id)
        .await
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

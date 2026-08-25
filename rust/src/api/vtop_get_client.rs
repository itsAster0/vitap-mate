use crate::api::vtop::{
    types::{
        AttendanceData, BiometricData, ExamScheduleData, FullAttendanceData, GradeDetailsData,
        GradeHistoryData, GradeViewData, MarksData, PersistedVtopSession, SemesterData,
        TimetableData,
    },
    vtop_client::{VtopClient, VtopError},
    vtop_config::VtopClientBuilder,
};

macro_rules! required_text_type {
    ($name:ident, $label:literal) => {
        struct $name(String);

        impl $name {
            fn parse(value: String) -> Result<Self, VtopError> {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(VtopError::ConfigurationError(format!(
                        "{} must not be empty",
                        $label
                    )));
                }
                if trimmed.len() > 256 {
                    return Err(VtopError::ConfigurationError(format!(
                        "{} is too long",
                        $label
                    )));
                }
                Ok(Self(trimmed.to_owned()))
            }
        }
    };
}

required_text_type!(Username, "username");
required_text_type!(SemesterId, "semester_id");
required_text_type!(CourseId, "course_id");
required_text_type!(CourseType, "course_type");

struct Password(String);

impl Password {
    fn parse(value: String) -> Result<Self, VtopError> {
        if value.trim().is_empty() {
            return Err(VtopError::ConfigurationError(
                "password must not be empty".to_string(),
            ));
        }
        if value.len() > 256 {
            return Err(VtopError::ConfigurationError(
                "password is too long".to_string(),
            ));
        }
        Ok(Self(value))
    }
}

macro_rules! impl_as_str {
    ($name:ident) => {
        impl $name {
            fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

impl_as_str!(SemesterId);
impl_as_str!(CourseId);
impl_as_str!(CourseType);

struct Credentials {
    username: Username,
    password: Password,
}

impl Credentials {
    fn parse(username: String, password: String) -> Result<Self, VtopError> {
        Ok(Self {
            username: Username::parse(username)?,
            password: Password::parse(password)?,
        })
    }
}

#[flutter_rust_bridge::frb]
pub async fn get_vtop_client(
    username: String,
    password: String,
    persisted_session: Option<PersistedVtopSession>,
    in_app_captcha_solver_enabled: bool,
) -> Result<VtopClient, VtopError> {
    let credentials = Credentials::parse(username, password)?;
    let mut client = VtopClientBuilder::new().build(credentials.username.0, credentials.password.0);
    client.set_in_app_captcha_solver_enabled(in_app_captcha_solver_enabled);
    if let Some(session) = persisted_session {
        client.restore_session_snapshot(session);
    }
    Ok(client)
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

#[flutter_rust_bridge::frb(sync)]
pub fn vtop_client_registration_number(client: &VtopClient) -> Result<String, VtopError> {
    client.registration_number_value()
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
    let semester_id = SemesterId::parse(semester_id)?;
    client.get_attendance(semester_id.as_str()).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_biometric_history(
    client: &mut VtopClient,
    date: String,
) -> Result<BiometricData, VtopError> {
    client.get_biometric_history(&date).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_full_attendance(
    client: &mut VtopClient,
    semester_id: String,
    course_id: String,
    course_type: String,
) -> Result<FullAttendanceData, VtopError> {
    let semester_id = SemesterId::parse(semester_id)?;
    let course_id = CourseId::parse(course_id)?;
    let course_type = CourseType::parse(course_type)?;
    client
        .get_full_attendance(
            semester_id.as_str(),
            course_id.as_str(),
            course_type.as_str(),
        )
        .await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_timetable(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<TimetableData, VtopError> {
    let semester_id = SemesterId::parse(semester_id)?;
    client.get_timetable(semester_id.as_str()).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_marks(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<MarksData, VtopError> {
    let semester_id = SemesterId::parse(semester_id)?;
    client.get_marks(semester_id.as_str()).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_exam_shedule(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<ExamScheduleData, VtopError> {
    let semester_id = SemesterId::parse(semester_id)?;
    client.get_exam_schedule(semester_id.as_str()).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_grade_view(
    client: &mut VtopClient,
    semester_id: String,
) -> Result<GradeViewData, VtopError> {
    let semester_id = SemesterId::parse(semester_id)?;
    client.get_grade_view(semester_id.as_str()).await
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_grade_view_details(
    client: &mut VtopClient,
    semester_id: String,
    course_id: String,
) -> Result<GradeDetailsData, VtopError> {
    let semester_id = SemesterId::parse(semester_id)?;
    let course_id = CourseId::parse(course_id)?;
    client
        .get_grade_view_details(semester_id.as_str(), course_id.as_str())
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
) -> PersistedVtopSession {
    client.export_session_snapshot(saved_at_epoch_ms)
}

#[flutter_rust_bridge::frb()]
pub async fn fetch_is_auth(client: &mut VtopClient) -> bool {
    client.is_authenticated().clone()
}

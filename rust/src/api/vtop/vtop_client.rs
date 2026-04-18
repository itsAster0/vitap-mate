pub use super::types::*;
pub use super::{
    paraser::*,
    session_manager::SessionManager,
    types::{AttendanceData, ExamScheduleData, FullAttendanceData},
    vtop_config::VtopConfig,
    vtop_errors::{VtopError, VtopResult},
};
use crate::api::native_logs::append_native_log;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};

#[cfg(not(target_arch = "wasm32"))]
pub use reqwest::cookie::{CookieStore, Jar};
use reqwest::{
    header::{HeaderMap, HeaderValue, USER_AGENT},
    multipart, Certificate, Client, Response, Url,
};

use scraper::{Html, Selector};
use serde::Serialize;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
const VITAP_CUSTOM_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIGTDCCBDSgAwIBAgIQOXpmzCdWNi4NqofKbqvjsTANBgkqhkiG9w0BAQwFADBf
MQswCQYDVQQGEwJHQjEYMBYGA1UEChMPU2VjdGlnbyBMaW1pdGVkMTYwNAYDVQQD
Ey1TZWN0aWdvIFB1YmxpYyBTZXJ2ZXIgQXV0aGVudGljYXRpb24gUm9vdCBSNDYw
HhcNMjEwMzIyMDAwMDAwWhcNMzYwMzIxMjM1OTU5WjBgMQswCQYDVQQGEwJHQjEY
MBYGA1UEChMPU2VjdGlnbyBMaW1pdGVkMTcwNQYDVQQDEy5TZWN0aWdvIFB1Ymxp
YyBTZXJ2ZXIgQXV0aGVudGljYXRpb24gQ0EgRFYgUjM2MIIBojANBgkqhkiG9w0B
AQEFAAOCAY8AMIIBigKCAYEAljZf2HIz7+SPUPQCQObZYcrxLTHYdf1ZtMRe7Yeq
RPSwygz16qJ9cAWtWNTcuICc++p8Dct7zNGxCpqmEtqifO7NvuB5dEVexXn9RFFH
12Hm+NtPRQgXIFjx6MSJcNWuVO3XGE57L1mHlcQYj+g4hny90aFh2SCZCDEVkAja
EMMfYPKuCjHuuF+bzHFb/9gV8P9+ekcHENF2nR1efGWSKwnfG5RawlkaQDpRtZTm
M64TIsv/r7cyFO4nSjs1jLdXYdz5q3a4L0NoabZfbdxVb+CUEHfB0bpulZQtH1Rv
38e/lIdP7OTTIlZh6OYL6NhxP8So0/sht/4J9mqIGxRFc0/pC8suja+wcIUna0HB
pXKfXTKpzgis+zmXDL06ASJf5E4A2/m+Hp6b84sfPAwQ766rI65mh50S0Di9E3Pn
2WcaJc+PILsBmYpgtmgWTR9eV9otfKRUBfzHUHcVgarub/XluEpRlTtZudU5xbFN
xx/DgMrXLUAPaI60fZ6wA+PTAgMBAAGjggGBMIIBfTAfBgNVHSMEGDAWgBRWc1hk
lfmSGrASKgRieaFAFYghSTAdBgNVHQ4EFgQUaMASFhgOr872h6YyV6NGUV3LBycw
DgYDVR0PAQH/BAQDAgGGMBIGA1UdEwEB/wQIMAYBAf8CAQAwHQYDVR0lBBYwFAYI
KwYBBQUHAwEGCCsGAQUFBwMCMBsGA1UdIAQUMBIwBgYEVR0gADAIBgZngQwBAgEw
VAYDVR0fBE0wSzBJoEegRYZDaHR0cDovL2NybC5zZWN0aWdvLmNvbS9TZWN0aWdv
UHVibGljU2VydmVyQXV0aGVudGljYXRpb25Sb290UjQ2LmNybDCBhAYIKwYBBQUH
AQEEeDB2ME8GCCsGAQUFBzAChkNodHRwOi8vY3J0LnNlY3RpZ28uY29tL1NlY3Rp
Z29QdWJsaWNTZXJ2ZXJBdXRoZW50aWNhdGlvblJvb3RSNDYucDdjMCMGCCsGAQUF
BzABhhdodHRwOi8vb2NzcC5zZWN0aWdvLmNvbTANBgkqhkiG9w0BAQwFAAOCAgEA
YtOC9Fy+TqECFw40IospI92kLGgoSZGPOSQXMBqmsGWZUQ7rux7cj1du6d9rD6C8
ze1B2eQjkrGkIL/OF1s7vSmgYVafsRoZd/IHUrkoQvX8FZwUsmPu7amgBfaY3g+d
q1x0jNGKb6I6Bzdl6LgMD9qxp+3i7GQOnd9J8LFSietY6Z4jUBzVoOoz8iAU84OF
h2HhAuiPw1ai0VnY38RTI+8kepGWVfGxfBWzwH9uIjeooIeaosVFvE8cmYUB4TSH
5dUyD0jHct2+8ceKEtIoFU/FfHq/mDaVnvcDCZXtIgitdMFQdMZaVehmObyhRdDD
4NQCs0gaI9AAgFj4L9QtkARzhQLNyRf87Kln+YU0lgCGr9HLg3rGO8q+Y4ppLsOd
unQZ6ZxPNGIfOApbPVf5hCe58EZwiWdHIMn9lPP6+F404y8NNugbQixBber+x536
WrZhFZLjEkhp7fFXf9r32rNPfb74X/U90Bdy4lzp3+X1ukh1BuMxA/EEhDoTOS3l
7ABvc7BYSQubQ2490OcdkIzUh3ZwDrakMVrbaTxUM2p24N6dB+ns2zptWCva6jzW
r8IWKIMxzxLPv5Kt3ePKcUdvkBU/smqujSczTzzSjIoR5QqQA6lN1ZRSnuHIWCvh
JEltkYnTAH41QJ6SAWO66GrrUESwN/cgZzL4JLEqz1Y=
-----END CERTIFICATE-----"#;

#[cfg(not(target_arch = "wasm32"))]
fn reqwest_network_error(context: &str, error: reqwest::Error) -> VtopError {
    append_native_log("ERROR", "rust.network", &format!("{context}: {error}"));
    VtopError::NetworkError
}

fn vtop_server_error(context: &str, detail: impl AsRef<str>) -> VtopError {
    let message = format!("{context}: {}", detail.as_ref());
    append_native_log(
        "ERROR",
        "rust.network",
        &format!("returning VtopServerError: {message}"),
    );
    VtopError::VtopServerError(message)
}

fn log_network_request(context: &str, method: &str, url: &str) {
    append_native_log(
        "INFO",
        "rust.network",
        &format!("request {context} {method} {url}"),
    );
}

fn log_auth_event(level: &str, message: &str) {
    append_native_log(level, "rust.auth", message);
}

fn missing_csrf_error(context: &str) -> VtopError {
    log_auth_event(
        "WARN",
        &format!("{context}: CSRF token is missing, session needs validation"),
    );
    VtopError::SessionExpired
}

pub struct VtopClient {
    client: Client,
    config: VtopConfig,
    session: SessionManager,
    current_page: Option<String>,
    username: String,
    password: String,
    captcha_data: Option<String>,
}

impl VtopClient {
    fn mark_session_expired(&mut self, context: &str, reason: &str) {
        self.session.set_authenticated(false);
        log_auth_event(
            "WARN",
            &format!("{context}: session marked expired because {reason}"),
        );
    }

    async fn read_authenticated_response_text(
        &mut self,
        context: &str,
        response: Response,
    ) -> VtopResult<String> {
        let final_url = response.url().to_string();
        let status = response.status();

        if Self::is_login_url(&final_url) {
            self.mark_session_expired(context, &format!("VTOP redirected to {final_url}"));
            return Err(VtopError::SessionExpired);
        }

        let text = response
            .text()
            .await
            .map_err(|error| reqwest_network_error(&format!("{context}.text"), error))?;

        if Self::looks_like_login_page(&text) {
            self.mark_session_expired(context, "VTOP returned the login page");
            return Err(VtopError::SessionExpired);
        }

        if !status.is_success() {
            return Err(vtop_server_error(
                context,
                format!("VTOP returned HTTP {status} at {final_url}; keeping session for reuse"),
            ));
        }

        Ok(text)
    }

    pub fn restore_session_snapshot(&mut self, session: PersistedVtopSession) {
        log_auth_event(
            "INFO",
            &format!(
                "restoring persisted session for {} with {} cookie(s)",
                session.username,
                session.cookies.len()
            ),
        );
        self.session
            .import_persisted_session(self.config.base_url.clone(), session);
    }

    pub fn export_session_snapshot(
        &self,
        saved_at_epoch_ms: u64,
        expires_at_epoch_ms: u64,
    ) -> PersistedVtopSession {
        self.session.export_persisted_session(
            self.config.base_url.clone(),
            self.username.clone(),
            saved_at_epoch_ms,
            expires_at_epoch_ms,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn get_cookie(&self, check: bool) -> VtopResult<Vec<u8>> {
        if !self.session.is_authenticated() && check {
            return Err(VtopError::SessionExpired);
        }

        let mut data = vec![];
        let url = format!("{}/vtop", self.config.base_url);
        let k = self
            .session
            .get_cookie_store()
            .cookies(&Url::parse(&url).unwrap());
        if let Some(cookie) = k {
            data = cookie.as_bytes().to_vec();
        }
        Ok(data)
    }

    pub fn set_cookie(&mut self, cookie: String) {
        let url = format!("{}/vtop", self.config.base_url);

        self.session.set_cookie_from_external(url, cookie);
    }
    pub async fn get_semesters(&mut self, check: bool) -> VtopResult<SemesterData> {
        if check && !self.ensure_authenticated_session().await? {
            return Err(VtopError::SessionExpired);
        }
        let url = format!(
            "{}/vtop/academics/common/StudentTimeTable",
            self.config.base_url
        );

        let body = format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            self.username,
            self.session
                .get_csrf_token()
                .ok_or_else(|| missing_csrf_error("get_semesters"))?,
        );
        log_network_request("get_semesters.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .body(body)
            .send()
            .await
            .map_err(|error| reqwest_network_error("get_semesters.send", error))?;
        let text = self
            .read_authenticated_response_text("get_semesters", res)
            .await?;
        Ok(parsett::parse_semid_timetable(text))
    }

    pub async fn get_timetable(&mut self, semester_id: &str) -> VtopResult<TimetableData> {
        if !self.ensure_authenticated_session().await? {
            return Err(VtopError::SessionExpired);
        }
        let url = format!("{}/vtop/processViewTimeTable", self.config.base_url);
        let body = format!(
            "_csrf={}&semesterSubId={}&authorizedID={}",
            self.session
                .get_csrf_token()
                .ok_or_else(|| missing_csrf_error("get_timetable"))?,
            semester_id,
            self.username
        );
        log_network_request("get_timetable.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .body(body)
            .send()
            .await
            .map_err(|error| reqwest_network_error("get_timetable.send", error))?;
        let text = self
            .read_authenticated_response_text("get_timetable", res)
            .await?;
        Ok(parsett::parse_timetable(text, semester_id))
    }

    pub async fn get_attendance(&mut self, semester_id: &str) -> VtopResult<AttendanceData> {
        if !self.ensure_authenticated_session().await? {
            return Err(VtopError::SessionExpired);
        }
        let url = format!("{}/vtop/processViewStudentAttendance", self.config.base_url);
        let body = format!(
            "_csrf={}&semesterSubId={}&authorizedID={}",
            self.session
                .get_csrf_token()
                .ok_or_else(|| missing_csrf_error("get_attendance"))?,
            semester_id,
            self.username
        );
        log_network_request("get_attendance.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .body(body)
            .send()
            .await
            .map_err(|error| reqwest_network_error("get_attendance.send", error))?;
        let text = self
            .read_authenticated_response_text("get_attendance", res)
            .await?;
        Ok(parseattn::parse_attendance(text, semester_id.to_string()))
    }

    pub async fn get_full_attendance(
        &mut self,
        semester_id: &str,
        course_id: &str,
        course_type: &str,
    ) -> VtopResult<FullAttendanceData> {
        if !self.ensure_authenticated_session().await? {
            return Err(VtopError::SessionExpired);
        }
        let url = format!("{}/vtop/processViewAttendanceDetail", self.config.base_url);
        let body = format!(
            "_csrf={}&semesterSubId={}&registerNumber={}&courseId={}&courseType={}&authorizedID={}",
            self.session
                .get_csrf_token()
                .ok_or_else(|| missing_csrf_error("get_full_attendance"))?,
            semester_id,
            self.username,
            course_id,
            course_type,
            self.username
        );
        log_network_request("get_full_attendance.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .body(body)
            .send()
            .await
            .map_err(|error| reqwest_network_error("get_full_attendance.send", error))?;
        let text = self
            .read_authenticated_response_text("get_full_attendance", res)
            .await?;
        Ok(parseattn::parse_full_attendance(
            text,
            semester_id.to_string(),
            course_id.into(),
            course_type.into(),
        ))
    }

    pub async fn get_marks(&mut self, semester_id: &str) -> VtopResult<MarksData> {
        if !self.ensure_authenticated_session().await? {
            return Err(VtopError::SessionExpired);
        }
        let url = format!(
            "{}/vtop/examinations/doStudentMarkView",
            self.config.base_url
        );
        let form = multipart::Form::new()
            .text("authorizedID", self.username.clone())
            .text("semesterSubId", semester_id.to_string())
            .text(
                "_csrf",
                self.session
                    .get_csrf_token()
                    .ok_or_else(|| missing_csrf_error("get_marks"))?,
            );

        log_network_request("get_marks.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|error| reqwest_network_error("get_marks.send", error))?;
        let text = self
            .read_authenticated_response_text("get_marks", res)
            .await?;

        Ok(parsemarks::parse_marks(text, semester_id.to_string()))
    }

    pub async fn get_grade_view(&mut self, semester_id: &str) -> VtopResult<GradeViewData> {
        if !self.ensure_authenticated_session().await? {
            return Err(VtopError::SessionExpired);
        }
        let url = format!(
            "{}/vtop/examinations/examGradeView/doStudentGradeView",
            self.config.base_url
        );
        let form = multipart::Form::new()
            .text("authorizedID", self.username.clone())
            .text("semesterSubId", semester_id.to_string())
            .text(
                "_csrf",
                self.session
                    .get_csrf_token()
                    .ok_or_else(|| missing_csrf_error("get_grade_view"))?,
            );

        log_network_request("get_grade_view.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|error| reqwest_network_error("get_grade_view.send", error))?;
        let text = self
            .read_authenticated_response_text("get_grade_view", res)
            .await?;
        Ok(parsegrades::parse_grade_view(text, semester_id.to_string()))
    }

    pub async fn get_grade_view_details(
        &mut self,
        semester_id: &str,
        course_id: &str,
    ) -> VtopResult<GradeDetailsData> {
        if !self.ensure_authenticated_session().await? {
            return Err(VtopError::SessionExpired);
        }
        let url = format!(
            "{}/vtop/examinations/examGradeView/getGradeViewDetails",
            self.config.base_url
        );
        let params = [
            ("authorizedID", self.username.clone()),
            ("x", "codex".to_string()),
            ("semesterSubId", semester_id.to_string()),
            ("courseId", course_id.to_string()),
            (
                "_csrf",
                self.session
                    .get_csrf_token()
                    .ok_or_else(|| missing_csrf_error("get_grade_view_details"))?,
            ),
        ];

        log_network_request("get_grade_view_details.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|error| reqwest_network_error("get_grade_view_details.send", error))?;
        let text = self
            .read_authenticated_response_text("get_grade_view_details", res)
            .await?;
        Ok(parsegrades::parse_grade_view_details(
            text,
            semester_id.to_string(),
            course_id.to_string(),
        ))
    }

    pub async fn get_grade_history(&mut self) -> VtopResult<GradeHistoryData> {
        if !self.ensure_authenticated_session().await? {
            return Err(VtopError::SessionExpired);
        }
        let url = format!(
            "{}/vtop/examinations/examGradeView/StudentGradeHistory",
            self.config.base_url
        );
        let body = format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            self.username,
            self.session
                .get_csrf_token()
                .ok_or_else(|| missing_csrf_error("get_grade_history"))?,
        );
        log_network_request("get_grade_history.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .body(body)
            .send()
            .await
            .map_err(|error| reqwest_network_error("get_grade_history.send", error))?;
        let text = self
            .read_authenticated_response_text("get_grade_history", res)
            .await?;
        Ok(parsegradehistory::parse_grade_history(text))
    }

    pub async fn get_exam_schedule(&mut self, semester_id: &str) -> VtopResult<ExamScheduleData> {
        if !self.ensure_authenticated_session().await? {
            return Err(VtopError::SessionExpired);
        }
        let url = format!(
            "{}/vtop/examinations/doSearchExamScheduleForStudent",
            self.config.base_url
        );
        let form = multipart::Form::new()
            .text("authorizedID", self.username.clone())
            .text("semesterSubId", semester_id.to_string())
            .text(
                "_csrf",
                self.session
                    .get_csrf_token()
                    .ok_or_else(|| missing_csrf_error("get_exam_schedule"))?,
            );
        log_network_request("get_exam_schedule.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|error| reqwest_network_error("get_exam_schedule.send", error))?;
        let text = self
            .read_authenticated_response_text("get_exam_schedule", res)
            .await?;
        Ok(parsesched::parse_schedule(text, semester_id.to_string()))
    }
    pub fn is_authenticated(&mut self) -> bool {
        self.session.is_authenticated()
    }
}

// for login
impl VtopClient {
    pub async fn login(&mut self) -> VtopResult<()> {
        if self.session.is_cookie_external() {
            log_auth_event("INFO", "validating restored session before fresh login");
            match self.try_restore_existing_session().await {
                Ok(true) => return Ok(()),
                Ok(false) => {
                    log_auth_event(
                        "WARN",
                        "restored session could not be verified, falling back to full login",
                    );
                }
                Err(error) => {
                    log_auth_event(
                        "WARN",
                        &format!(
                            "restored session validation failed, falling back to fresh login: {}",
                            error
                        ),
                    );
                }
            }
        }

        #[allow(non_snake_case)]
        let MAX_CAP_TRY = 40;
        for i in 0..MAX_CAP_TRY {
            log_auth_event(
                "INFO",
                &format!("starting login attempt {} of {}", i + 1, MAX_CAP_TRY),
            );
            if i == 0 {
                self.load_login_page(true).await?;
            } else {
                self.load_login_page(false).await?;
            }

            let captcha_answer = if let Some(captcha_data) = &self.captcha_data {
                self.solve_captcha(captcha_data).await?
            } else {
                return Err(VtopError::CaptchaRequired);
            };
            match self.perform_login(&captcha_answer).await {
                Ok(_) => {
                    self.session.set_authenticated(true);
                    self.session.set_cookie_external(false);
                    log_auth_event("INFO", "login completed successfully");
                    return Ok(());
                }
                Err(VtopError::AuthenticationFailed(msg)) if msg.contains("Invalid Captcha") => {
                    log_auth_event("WARN", "captcha answer was rejected, retrying login");
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        Err(VtopError::AuthenticationFailed(
            "We could not complete sign-in after several captcha attempts. Please try again."
                .to_string(),
        ))
    }

    async fn perform_login(&mut self, captcha_answer: &str) -> VtopResult<()> {
        let csrf = self
            .session
            .get_csrf_token()
            .ok_or_else(|| missing_csrf_error("perform_login"))?;

        let login_data = format!(
            "_csrf={}&username={}&password={}&captchaStr={}",
            csrf,
            urlencoding::encode(&self.username),
            urlencoding::encode(&self.password),
            captcha_answer
        );
        let url = format!("{}/vtop/login", self.config.base_url);

        log_network_request("perform_login.send", "POST", &url);
        let response = self
            .client
            .post(&url)
            .body(login_data)
            .send()
            .await
            .map_err(|error| reqwest_network_error("perform_login.send", error))?;
        let response_url = response.url().to_string();
        let response_text = response
            .text()
            .await
            .map_err(|error| reqwest_network_error("perform_login.text", error))?;

        if response_url.contains("error") {
            if response_text.contains("Invalid Captcha") {
                return Err(VtopError::AuthenticationFailed(
                    "Invalid Captcha".to_string(),
                ));
            } else if Self::is_invalid_credentials_response(&response_text) {
                log_auth_event("WARN", "login rejected due to invalid credentials");
                return Err(VtopError::InvalidCredentials);
            } else if Self::is_security_otp_required_response(&response_text) {
                self.current_page = Some(response_text);
                let _ = self.extract_csrf_token();
                log_auth_event("INFO", "security OTP verification is required");
                return Err(VtopError::OTPRequired(
                    "Additional verification is required.".to_string(),
                ));
            } else {
                return Err(VtopError::AuthenticationFailed(Self::get_login_page_error(
                    &response_text,
                )));
            }
        } else {
            self.current_page = Some(response_text);
            self.extract_csrf_token()?;
            self.get_regno()?;

            self.current_page = None;
            self.captcha_data = None;
            Ok(())
        }
    }

    pub async fn submit_security_otp(&mut self, otp_code: &str) -> VtopResult<()> {
        let csrf = self
            .session
            .get_csrf_token()
            .ok_or_else(|| missing_csrf_error("submit_security_otp"))?;
        let url = format!("{}/vtop/validateSecurityOtp", self.config.base_url);
        let referer = format!("{}/vtop/login/error", self.config.base_url);

        let form = multipart::Form::new()
            .text("otpCode", otp_code.trim().to_string())
            .text("_csrf", csrf);

        log_network_request("submit_security_otp.send", "POST", &url);
        let response = self
            .client
            .post(&url)
            .header("Accept", "*/*")
            .header("Origin", &self.config.base_url)
            .header("Referer", referer)
            .header("Sec-Fetch-Dest", "empty")
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Site", "same-origin")
            .multipart(form)
            .send()
            .await
            .map_err(|error| reqwest_network_error("submit_security_otp.send", error))?;
        let final_url = response.url().to_string();
        let status_code = response.status();

        let response_text = response
            .text()
            .await
            .map_err(|error| reqwest_network_error("submit_security_otp.text", error))?;

        let otp_response_json = serde_json::from_str::<serde_json::Value>(&response_text).ok();
        let otp_status = otp_response_json
            .as_ref()
            .and_then(|json| json.get("status"))
            .and_then(|status| status.as_str())
            .map(|status| status.to_uppercase());
        let otp_message = otp_response_json
            .as_ref()
            .and_then(|json| json.get("message"))
            .and_then(|message| message.as_str())
            .map(|message| message.to_string());
        let redirect_url = otp_response_json
            .as_ref()
            .and_then(|json| json.get("redirectUrl"))
            .and_then(|redirect_url| redirect_url.as_str())
            .filter(|redirect_url| !redirect_url.trim().is_empty())
            .map(|redirect_url| redirect_url.to_string());
        let response_text_lower = response_text.to_lowercase();

        if matches!(otp_status.as_deref(), Some("INVALID"))
            || response_text_lower.contains("invalid otp")
        {
            let message = otp_message.unwrap_or("Invalid OTP. Please try again.".to_string());
            log_auth_event(
                "WARN",
                "OTP verification failed because the code was invalid",
            );
            return Err(VtopError::AuthenticationFailed(message));
        }

        if matches!(otp_status.as_deref(), Some("EXPIRED")) {
            let message = otp_message.unwrap_or("OTP has expired. Please resend.".to_string());
            log_auth_event("WARN", "OTP verification failed because the code expired");
            return Err(VtopError::AuthenticationFailed(message));
        }

        if matches!(otp_status.as_deref(), Some("SUCCESS")) {
            if let Some(redirect_url) = redirect_url {
                if self.follow_security_otp_redirect(&redirect_url).await? {
                    return Ok(());
                }
            } else {
                log_auth_event(
                    "WARN",
                    "OTP verification returned SUCCESS without redirectUrl",
                );
            }
        }

        if let Some(status) = otp_status.as_deref() {
            if status != "VALID" && status != "SUCCESS" && status != "OK" {
                log_auth_event("WARN", "OTP verification returned a non-success status");
                return Err(VtopError::AuthenticationFailed(
                    otp_message.unwrap_or("Verification failed. Please try again.".to_string()),
                ));
            }
        }

        let response_text_trimmed = response_text.trim().to_uppercase();
        let response_declares_success =
            matches!(
                otp_status.as_deref(),
                Some("VALID") | Some("SUCCESS") | Some("OK")
            ) || matches!(response_text_trimmed.as_str(), "VALID" | "SUCCESS" | "OK");

        let landed_on_content =
            status_code.is_success() && final_url.trim_end_matches('/').ends_with("/vtop/content");
        if landed_on_content {
            self.current_page = Some(response_text);
            let _ = self.extract_csrf_token();
            if self.get_regno().is_err() {
                self.username = self.username.to_uppercase();
            }
            self.session.set_authenticated(true);
            self.session.set_cookie_external(false);
            self.current_page = None;
            self.captcha_data = None;
            log_auth_event("INFO", "OTP verified and session confirmed");
            return Ok(());
        }

        if matches!(self.validate_authenticated_session().await, Ok(true)) {
            if self.get_regno().is_err() {
                self.username = self.username.to_uppercase();
            }
            self.session.set_authenticated(true);
            self.session.set_cookie_external(false);
            self.current_page = None;
            self.captcha_data = None;
            log_auth_event("INFO", "OTP verified and session confirmed");
            return Ok(());
        }

        if response_declares_success {
            log_auth_event(
                "WARN",
                &format!(
                    "OTP endpoint reported success, but session validation failed after landing at {final_url}"
                ),
            );
            return Err(VtopError::AuthenticationFailed(
                "VTOP accepted the OTP, but did not open a signed-in session. Please try again."
                    .to_string(),
            ));
        }

        log_auth_event(
            "WARN",
            &format!(
                "OTP verification did not return a success response: HTTP {status_code} at {final_url}"
            ),
        );
        Err(VtopError::AuthenticationFailed(
            "We could not verify that OTP. Please try again.".to_string(),
        ))
    }

    async fn follow_security_otp_redirect(&mut self, redirect_url: &str) -> VtopResult<bool> {
        let url = self.resolve_vtop_url(redirect_url)?;
        log_network_request("follow_security_otp_redirect.send", "GET", &url);
        let response = self
            .client
            .get(&url)
            .header("Accept", "*/*")
            .send()
            .await
            .map_err(|error| reqwest_network_error("follow_security_otp_redirect.send", error))?;
        let final_url = response.url().to_string();
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| reqwest_network_error("follow_security_otp_redirect.text", error))?;

        if Self::is_login_url(&final_url) || Self::looks_like_login_page(&text) {
            self.mark_session_expired(
                "follow_security_otp_redirect",
                &format!("VTOP returned login page after OTP redirect at {final_url}"),
            );
            return Ok(false);
        }

        if !status.is_success() {
            return Err(vtop_server_error(
                "follow_security_otp_redirect",
                format!("VTOP returned HTTP {status} at {final_url}"),
            ));
        }

        self.current_page = Some(text);
        let _ = self.extract_csrf_token();
        if self.get_regno().is_err() {
            self.username = self.username.to_uppercase();
        }

        if matches!(self.validate_authenticated_session().await, Ok(true)) {
            self.session.set_authenticated(true);
            self.session.set_cookie_external(false);
            self.current_page = None;
            self.captcha_data = None;
            log_auth_event("INFO", "OTP redirect followed and session confirmed");
            return Ok(true);
        }

        Ok(false)
    }

    pub async fn resend_security_otp(&mut self) -> VtopResult<()> {
        let csrf = self
            .session
            .get_csrf_token()
            .ok_or_else(|| missing_csrf_error("resend_security_otp"))?;
        let resend_paths = ["/vtop/resendSecurityOtp", "/vtop/resendSecurityOTP"];
        let referer = format!("{}/vtop/login/error", self.config.base_url);

        for path in resend_paths {
            let url = format!("{}{}", self.config.base_url, path);
            let form = multipart::Form::new().text("_csrf", csrf.clone());

            log_network_request("resend_security_otp.send", "POST", &url);
            let response = self
                .client
                .post(&url)
                .header("Accept", "*/*")
                .header("Origin", &self.config.base_url)
                .header("Referer", &referer)
                .header("Sec-Fetch-Dest", "empty")
                .header("Sec-Fetch-Mode", "cors")
                .header("Sec-Fetch-Site", "same-origin")
                .multipart(form)
                .send()
                .await
                .map_err(|error| reqwest_network_error("resend_security_otp.send", error))?;

            let status_code = response.status();
            let response_text = response
                .text()
                .await
                .map_err(|error| reqwest_network_error("resend_security_otp.text", error))?;

            if status_code.as_u16() == 404 {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
                let status = json
                    .get("status")
                    .and_then(|status| status.as_str())
                    .map(|status| status.to_uppercase());
                let message = json
                    .get("message")
                    .and_then(|message| message.as_str())
                    .unwrap_or("Failed to resend OTP. Please try again.")
                    .to_string();

                if matches!(
                    status.as_deref(),
                    Some("SUCCESS") | Some("SENT") | Some("VALID") | Some("OK")
                ) {
                    log_auth_event("INFO", "OTP resend request completed successfully");
                    return Ok(());
                }
                return Err(VtopError::AuthenticationFailed(message));
            }

            if status_code.is_success()
                && response_text.to_lowercase().contains("otp")
                && response_text.to_lowercase().contains("sent")
            {
                return Ok(());
            }

            return Err(VtopError::AuthenticationFailed(
                "Failed to resend OTP. Please try again.".to_string(),
            ));
        }

        Err(VtopError::AuthenticationFailed(
            "Failed to resend OTP. Please try again.".to_string(),
        ))
    }

    async fn validate_authenticated_session(&mut self) -> VtopResult<bool> {
        let url = format!("{}/vtop/content", self.config.base_url);
        log_network_request("validate_authenticated_session.send", "GET", &url);
        let response =
            self.client.get(&url).send().await.map_err(|error| {
                reqwest_network_error("validate_authenticated_session.send", error)
            })?;
        let final_url = response.url().to_string();
        let status = response.status();

        if Self::is_login_url(&final_url) {
            self.mark_session_expired(
                "validate_authenticated_session",
                &format!("VTOP redirected to {final_url}"),
            );
            return Ok(false);
        }

        let text = response
            .text()
            .await
            .map_err(|error| reqwest_network_error("validate_authenticated_session.text", error))?;

        if Self::looks_like_login_page(&text) {
            self.mark_session_expired(
                "validate_authenticated_session",
                "VTOP returned the login page",
            );
            return Ok(false);
        }

        if !status.is_success() {
            return Err(vtop_server_error(
                "validate_authenticated_session",
                format!("VTOP returned HTTP {status} at {final_url}; keeping session for reuse"),
            ));
        }

        if !Self::is_content_url(&final_url) {
            log_auth_event(
                "WARN",
                &format!(
                    "validate_authenticated_session: expected /vtop/content but landed at {final_url}"
                ),
            );
            return Ok(false);
        }

        self.current_page = Some(text);
        if let Err(error) = self.extract_csrf_token() {
            log_auth_event(
                "WARN",
                &format!(
                    "validate_authenticated_session: session reached content but CSRF refresh failed: {error}"
                ),
            );
        }
        self.session.set_cookie_external(false);
        Ok(true)
    }

    async fn ensure_authenticated_session(&mut self) -> VtopResult<bool> {
        if self.session.is_authenticated() {
            if self.session.get_csrf_token().is_some() {
                return Ok(true);
            }
            log_auth_event(
                "INFO",
                "authenticated session is missing CSRF token, refreshing from VTOP content",
            );
            return self.validate_authenticated_session().await;
        }

        if !self.session.is_cookie_external() {
            return Ok(false);
        }

        self.try_restore_existing_session().await
    }

    async fn load_login_page(&mut self, k: bool) -> VtopResult<()> {
        if k {
            self.load_initial_page().await?;
            self.extract_csrf_token()?;
        }
        #[allow(non_snake_case)]
        let Max_RELOAD_ATTEMPTS = 20;
        let csrf = self
            .session
            .get_csrf_token()
            .ok_or_else(|| missing_csrf_error("load_login_page"))?;
        let url = format!("{}/vtop/prelogin/setup", self.config.base_url);
        let body = format!("_csrf={}&flag=VTOP", csrf);
        self.captcha_data = None;
        for attempt in 0..Max_RELOAD_ATTEMPTS {
            log_network_request("load_login_page.send", "POST", &url);
            let response = self
                .client
                .post(&url)
                .body(body.clone())
                .send()
                .await
                .map_err(|error| reqwest_network_error("load_login_page.send", error))?;
            if !response.status().is_success() {
                return Err(vtop_server_error(
                    "load_login_page",
                    format!(
                        "VTOP returned HTTP {} while preparing captcha",
                        response.status()
                    ),
                ));
            }
            let text = response
                .text()
                .await
                .map_err(|error| reqwest_network_error("load_login_page.text", error))?;
            if text.contains("base64,") {
                self.current_page = Some(text);
                self.extract_captcha_data()?;
                break;
            }
            log_auth_event(
                "WARN",
                &format!(
                    "captcha payload was missing on attempt {} of {}, retrying login page load",
                    attempt + 1,
                    Max_RELOAD_ATTEMPTS
                ),
            );
        }
        if self.captcha_data.is_none() {
            log_auth_event(
                "ERROR",
                "captcha payload was missing after all login page reload attempts",
            );
            return Err(VtopError::CaptchaRequired);
        }
        Ok(())
    }

    async fn try_restore_existing_session(&mut self) -> VtopResult<bool> {
        let cookie = self.get_cookie(false).await?;
        if cookie.is_empty() {
            self.session.clear();
            return Ok(false);
        }

        if matches!(self.validate_authenticated_session().await, Ok(true)) {
            self.session.set_authenticated(true);
            self.session.set_cookie_external(false);
            log_auth_event("INFO", "restored session validation succeeded");
            return Ok(true);
        }

        log_auth_event(
            "WARN",
            "restored session was not accepted by VTOP; keeping cookies available for the next login attempt",
        );
        Ok(false)
    }

    fn is_login_url(url: &str) -> bool {
        Url::parse(url)
            .ok()
            .map(|parsed| parsed.path().starts_with("/vtop/login"))
            .unwrap_or_else(|| url.to_lowercase().contains("/vtop/login"))
    }

    fn is_content_url(url: &str) -> bool {
        Url::parse(url)
            .ok()
            .map(|parsed| parsed.path().trim_end_matches('/') == "/vtop/content")
            .unwrap_or_else(|| {
                url.split('?')
                    .next()
                    .unwrap_or(url)
                    .trim_end_matches('/')
                    .ends_with("/vtop/content")
            })
    }

    fn looks_like_login_page(text: &str) -> bool {
        let lower = text.to_lowercase();
        (lower.contains("/vtop/login")
            || lower.contains("action=\"/vtop/login\"")
            || lower.contains("action='/vtop/login'")
            || lower.contains("id=\"vtoploginform\"")
            || lower.contains("id='vtoploginform'"))
            && (lower.contains("name=\"username\"") || lower.contains("name='username'"))
            && (lower.contains("name=\"password\"") || lower.contains("name='password'"))
    }

    fn extract_captcha_data(&mut self) -> VtopResult<()> {
        let document = Html::parse_document(&self.current_page.as_ref().ok_or(
            VtopError::ParseError("Current page not found at captcha extration".into()),
        )?);
        let selector = Selector::parse("img.form-control.img-fluid.bg-light.border-0").unwrap();
        let captcha_src = document
            .select(&selector)
            .next()
            .and_then(|element| element.value().attr("src"))
            .ok_or(VtopError::CaptchaRequired)?;

        if captcha_src.contains("base64,") {
            self.captcha_data = Some(captcha_src.to_string());
        } else {
            return Err(VtopError::CaptchaRequired);
        }

        Ok(())
    }

    fn get_regno(&mut self) -> VtopResult<()> {
        let current_page = self.current_page.as_ref().ok_or(VtopError::ParseError(
            "Current page not found at captcha extration".into(),
        ))?;
        let document = Html::parse_document(current_page);
        let selector = Selector::parse("input[type=hidden][name=authorizedIDX]").unwrap();
        let k = document
            .select(&selector)
            .next()
            .and_then(|element| element.value().attr("value").map(|value| value.to_string()))
            .or_else(|| Self::extract_javascript_var(current_page, "id"))
            .ok_or(VtopError::RegistrationParsingError)?;

        self.username = k;
        Ok(())
    }
    async fn solve_captcha(&self, captcha_data: &str) -> VtopResult<String> {
        let url_safe_encoded = URL_SAFE.encode(captcha_data.as_bytes());
        let captcha_url = format!("https://cap.va.kryxen.dev/captcha");

        #[derive(Serialize)]
        struct PostData {
            imgstring: String,
        }

        let client_for_post = reqwest::Client::new();
        let post_data = PostData {
            imgstring: url_safe_encoded,
        };
        log_network_request("solve_captcha.send", "POST", &captcha_url);
        let response = client_for_post
            .post(&captcha_url)
            .json(&post_data)
            .send()
            .await
            .map_err(|error| reqwest_network_error("solve_captcha.send", error))?;

        if !response.status().is_success() {
            return Err(VtopError::NetworkError);
        }
        response
            .text()
            .await
            .map_err(|error| reqwest_network_error("solve_captcha.text", error))
    }
    fn extract_csrf_token(&mut self) -> VtopResult<()> {
        let current_page = self.current_page.as_ref().ok_or(VtopError::ParseError(
            "Current page not found at csrf extration".into(),
        ))?;
        let document = Html::parse_document(current_page);
        let selector = Selector::parse("input[name='_csrf']").unwrap();
        let csrf_token = document
            .select(&selector)
            .next()
            .and_then(|element| element.value().attr("value"))
            .map(|value| value.to_string())
            .or_else(|| Self::extract_javascript_var(current_page, "csrfValue"))
            .ok_or(VtopError::ParseError("CSRF token not found".to_string()))?;
        self.session.set_csrf_token(csrf_token.to_string());
        Ok(())
    }

    fn extract_javascript_var(data: &str, name: &str) -> Option<String> {
        let marker = format!("var {name}");
        let after_marker = data.split(&marker).nth(1)?;
        let after_equals = after_marker.split_once('=')?.1.trim_start();
        let quote = after_equals.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        after_equals[quote.len_utf8()..]
            .split_once(quote)
            .map(|(value, _)| value.to_string())
    }

    fn resolve_vtop_url(&self, path_or_url: &str) -> VtopResult<String> {
        if Url::parse(path_or_url).is_ok() {
            return Ok(path_or_url.to_string());
        }

        let base_url = Url::parse(&self.config.base_url)
            .map_err(|error| VtopError::ConfigurationError(error.to_string()))?;
        base_url
            .join(path_or_url)
            .map(|url| url.to_string())
            .map_err(|error| VtopError::ConfigurationError(error.to_string()))
    }

    async fn load_initial_page(&mut self) -> VtopResult<()> {
        let url = format!("{}/vtop/open/page", self.config.base_url);
        log_network_request("load_initial_page.send", "GET", &url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|error| reqwest_network_error("load_initial_page.send", error))?;

        if !response.status().is_success() {
            return Err(vtop_server_error(
                "load_initial_page",
                format!("VTOP returned HTTP {} at {url}", response.status()),
            ));
        }
        self.current_page = Some(
            response
                .text()
                .await
                .map_err(|error| reqwest_network_error("load_initial_page.text", error))?,
        );

        Ok(())
    }
    fn get_login_page_error(data: &str) -> String {
        if let Some(message) = Self::login_alert_message(data) {
            if message.is_empty() {
                "We could not sign you in. Please try again.".into()
            } else {
                message
            }
        } else {
            "We could not sign you in because VTOP returned an unexpected response.".into()
        }
    }

    fn login_alert_message(data: &str) -> Option<String> {
        let ptext = r#"span.text-danger.text-center[role="alert"]"#;
        let document = Html::parse_document(data);
        let selector = Selector::parse(&ptext).unwrap();
        document.select(&selector).next().map(|element| {
            element
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
    }

    fn normalize_login_alert_message(message: &str) -> String {
        message
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    }

    fn is_invalid_credentials_response(data: &str) -> bool {
        let Some(message) = Self::login_alert_message(data) else {
            return false;
        };
        matches!(
            Self::normalize_login_alert_message(&message).as_str(),
            "invalid username/password" | "invalid loginid/password"
        )
    }

    fn is_security_otp_required_response(data: &str) -> bool {
        data.to_lowercase()
            .contains(&"OTP sent to your registered email.".to_lowercase())
    }
}
// for building
impl VtopClient {
    pub fn with_config(
        config: VtopConfig,
        session: SessionManager,
        username: String,
        password: String,
    ) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client =
                Self::make_client(session.get_cookie_store(), session.get_persisted_headers());
            Self {
                client: client,
                config: config,
                session: session,
                current_page: None,
                username: username,
                password: password,
                captcha_data: None,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut headers = HeaderMap::new();
            headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/x-www-form-urlencoded"),
            );
            let client = reqwest::Client::builder()
                .default_headers(headers)
                .build()
                .unwrap();
            Self {
                client: client,
                config: config,
                session: session,
                current_page: None,
                username: username,
                password: password,
                captcha_data: None,
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn make_client(cookie_store: Arc<Jar>, persisted_headers: Vec<PersistedHeader>) -> Client {
        use std::time::Duration;

        let mut headers = HeaderMap::new();

        for header in persisted_headers {
            let Ok(name) = header.name.parse::<reqwest::header::HeaderName>() else {
                continue;
            };
            let Ok(value) = HeaderValue::from_str(&header.value) else {
                continue;
            };
            headers.insert(name, value);
        }

        if !headers.contains_key(USER_AGENT) {
            headers.insert(
                USER_AGENT,
                HeaderValue::from_static(
                    "Mozilla/5.0 (Linux; U; Linux x86_64; en-US) Gecko/20100101 Firefox/130.5",
                ),
            );
        }

        let mut client_builder = reqwest::Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .cookie_provider(cookie_store)
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(60))
            .tcp_keepalive(Duration::from_secs(60));

        if let Ok(cert) = Certificate::from_pem(VITAP_CUSTOM_CERT_PEM.as_bytes()) {
            client_builder = client_builder.add_root_certificate(cert);
        }

        let client: Client = client_builder.build().unwrap();
        return client;
    }
}

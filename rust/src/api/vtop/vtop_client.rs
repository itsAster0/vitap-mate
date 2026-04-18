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
    multipart, Certificate, Client, Url,
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
        if !self.session.is_authenticated() && check {
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
                .ok_or(VtopError::SessionExpired)?,
        );
        log_network_request("get_semesters.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .body(body)
            .send()
            .await
            .map_err(|_| VtopError::NetworkError)?;
        if !res.status().is_success() || res.url().to_string().contains("login") {
            self.session.set_authenticated(false);
            return Err(VtopError::SessionExpired);
        }

        let text = res.text().await.map_err(|_| VtopError::VtopServerError)?;
        Ok(parsett::parse_semid_timetable(text))
    }

    pub async fn get_timetable(&mut self, semester_id: &str) -> VtopResult<TimetableData> {
        if !self.session.is_authenticated() {
            return Err(VtopError::SessionExpired);
        }
        let url = format!("{}/vtop/processViewTimeTable", self.config.base_url);
        let body = format!(
            "_csrf={}&semesterSubId={}&authorizedID={}",
            self.session
                .get_csrf_token()
                .ok_or(VtopError::SessionExpired)?,
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
            .map_err(|_| VtopError::NetworkError)?;
        if !res.status().is_success() || res.url().to_string().contains("login") {
            self.session.set_authenticated(false);
            return Err(VtopError::SessionExpired);
        }
        let text = res.text().await.map_err(|_| VtopError::VtopServerError)?;
        Ok(parsett::parse_timetable(text, semester_id))
    }

    pub async fn get_attendance(&mut self, semester_id: &str) -> VtopResult<AttendanceData> {
        if !self.session.is_authenticated() {
            return Err(VtopError::SessionExpired);
        }
        let url = format!("{}/vtop/processViewStudentAttendance", self.config.base_url);
        let body = format!(
            "_csrf={}&semesterSubId={}&authorizedID={}",
            self.session
                .get_csrf_token()
                .ok_or(VtopError::SessionExpired)?,
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
            .map_err(|_| VtopError::NetworkError)?;
        if !res.status().is_success() || res.url().to_string().contains("login") {
            self.session.set_authenticated(false);
            return Err(VtopError::SessionExpired);
        };
        let text = res.text().await.map_err(|_| VtopError::VtopServerError)?;
        Ok(parseattn::parse_attendance(text, semester_id.to_string()))
    }

    pub async fn get_full_attendance(
        &mut self,
        semester_id: &str,
        course_id: &str,
        course_type: &str,
    ) -> VtopResult<FullAttendanceData> {
        if !self.session.is_authenticated() {
            return Err(VtopError::SessionExpired);
        }
        let url = format!("{}/vtop/processViewAttendanceDetail", self.config.base_url);
        let body = format!(
            "_csrf={}&semesterSubId={}&registerNumber={}&courseId={}&courseType={}&authorizedID={}",
            self.session
                .get_csrf_token()
                .ok_or(VtopError::SessionExpired)?,
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
            .map_err(|_| VtopError::NetworkError)?;
        if !res.status().is_success() || res.url().to_string().contains("login") {
            self.session.set_authenticated(false);
            return Err(VtopError::SessionExpired);
        }
        let text = res.text().await.map_err(|_| VtopError::VtopServerError)?;
        Ok(parseattn::parse_full_attendance(
            text,
            semester_id.to_string(),
            course_id.into(),
            course_type.into(),
        ))
    }

    pub async fn get_marks(&mut self, semester_id: &str) -> VtopResult<MarksData> {
        if !self.session.is_authenticated() {
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
                    .ok_or(VtopError::SessionExpired)?,
            );

        log_network_request("get_marks.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|_| VtopError::NetworkError)?;
        if !res.status().is_success() || res.url().to_string().contains("login") {
            self.session.set_authenticated(false);
            return Err(VtopError::SessionExpired);
        }

        let text = res.text().await.map_err(|_| VtopError::VtopServerError)?;

        Ok(parsemarks::parse_marks(text, semester_id.to_string()))
    }

    pub async fn get_grade_view(&mut self, semester_id: &str) -> VtopResult<GradeViewData> {
        if !self.session.is_authenticated() {
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
                    .ok_or(VtopError::SessionExpired)?,
            );

        log_network_request("get_grade_view.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|_| VtopError::NetworkError)?;
        if !res.status().is_success() || res.url().to_string().contains("login") {
            self.session.set_authenticated(false);
            return Err(VtopError::SessionExpired);
        }

        let text = res.text().await.map_err(|_| VtopError::VtopServerError)?;
        Ok(parsegrades::parse_grade_view(text, semester_id.to_string()))
    }

    pub async fn get_grade_view_details(
        &mut self,
        semester_id: &str,
        course_id: &str,
    ) -> VtopResult<GradeDetailsData> {
        if !self.session.is_authenticated() {
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
                    .ok_or(VtopError::SessionExpired)?,
            ),
        ];

        log_network_request("get_grade_view_details.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|_| VtopError::NetworkError)?;
        if !res.status().is_success() || res.url().to_string().contains("login") {
            self.session.set_authenticated(false);
            return Err(VtopError::SessionExpired);
        }

        let text = res.text().await.map_err(|_| VtopError::VtopServerError)?;
        Ok(parsegrades::parse_grade_view_details(
            text,
            semester_id.to_string(),
            course_id.to_string(),
        ))
    }

    pub async fn get_grade_history(&mut self) -> VtopResult<GradeHistoryData> {
        if !self.session.is_authenticated() {
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
                .ok_or(VtopError::SessionExpired)?,
        );
        log_network_request("get_grade_history.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .body(body)
            .send()
            .await
            .map_err(|_| VtopError::NetworkError)?;
        if !res.status().is_success() || res.url().to_string().contains("login") {
            self.session.set_authenticated(false);
            return Err(VtopError::SessionExpired);
        }
        let text = res.text().await.map_err(|_| VtopError::VtopServerError)?;
        Ok(parsegradehistory::parse_grade_history(text))
    }

    pub async fn get_exam_schedule(&mut self, semester_id: &str) -> VtopResult<ExamScheduleData> {
        if !self.session.is_authenticated() {
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
                    .ok_or(VtopError::SessionExpired)?,
            );
        log_network_request("get_exam_schedule.send", "POST", &url);
        let res = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|_| VtopError::NetworkError)?;
        if !res.status().is_success() || res.url().to_string().contains("login") {
            self.session.set_authenticated(false);
            return Err(VtopError::SessionExpired);
        }
        let text = res.text().await.map_err(|_| VtopError::VtopServerError)?;
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
            .ok_or(VtopError::SessionExpired)?;

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
            } else if response_text
                .to_lowercase()
                .contains(&"Invalid LoginId/Password".to_lowercase())
                || response_text
                    .to_lowercase()
                    .contains(&"Invalid  Username/Password".to_lowercase())
            {
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
            .ok_or(VtopError::SessionExpired)?;
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

        let otp_status = serde_json::from_str::<serde_json::Value>(&response_text)
            .ok()
            .and_then(|json| {
                json.get("status")
                    .and_then(|status| status.as_str())
                    .map(|status| status.to_uppercase())
            });

        if matches!(otp_status.as_deref(), Some("INVALID")) {
            let message = serde_json::from_str::<serde_json::Value>(&response_text)
                .ok()
                .and_then(|json| {
                    json.get("message")
                        .and_then(|value| value.as_str())
                        .map(|value| value.to_string())
                })
                .unwrap_or("Invalid OTP. Please try again.".to_string());
            log_auth_event(
                "WARN",
                "OTP verification failed because the code was invalid",
            );
            return Err(VtopError::AuthenticationFailed(message));
        }

        if let Some(status) = otp_status {
            if status != "VALID" && status != "SUCCESS" && status != "OK" {
                log_auth_event("WARN", "OTP verification returned a non-success status");
                return Err(VtopError::AuthenticationFailed(
                    "We could not verify that OTP. Please try again.".to_string(),
                ));
            }
        }

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

        Err(VtopError::AuthenticationFailed(
            "We verified the OTP, but your session could not be confirmed. Please sign in again."
                .to_string(),
        ))
    }

    pub async fn resend_security_otp(&mut self) -> VtopResult<()> {
        let csrf = self
            .session
            .get_csrf_token()
            .ok_or(VtopError::SessionExpired)?;
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
        let is_same_url = final_url.trim_end_matches('/') == url.trim_end_matches('/');
        let redirected = !is_same_url || status.is_redirection();
        if !status.is_success() || redirected {
            self.session.set_authenticated(false);
            return Ok(false);
        }
        self.current_page = Some(
            response
                .text()
                .await
                .map_err(|error| reqwest_network_error("get_csrf_for_cookie_set.text", error))?,
        );
        let _ = self.extract_csrf_token();
        self.session.set_cookie_external(false);
        Ok(true)
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
            .ok_or(VtopError::SessionExpired)?;
        let url = format!("{}/vtop/prelogin/setup", self.config.base_url);
        let body = format!("_csrf={}&flag=VTOP", csrf);
        for _ in 0..Max_RELOAD_ATTEMPTS {
            log_network_request("load_login_page.send", "POST", &url);
            let response = self
                .client
                .post(&url)
                .body(body.clone())
                .send()
                .await
                .map_err(|error| reqwest_network_error("load_login_page.send", error))?;
            if !response.status().is_success() {
                return Err(VtopError::VtopServerError);
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
                "captcha payload was missing, retrying login page load",
            );
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

        self.session.clear();
        Ok(false)
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
        let document = Html::parse_document(&self.current_page.as_ref().ok_or(
            VtopError::ParseError("Current page not found at captcha extration".into()),
        )?);
        let selector = Selector::parse("input[type=hidden][name=authorizedIDX]").unwrap();
        let k = document
            .select(&selector)
            .next()
            .and_then(|element| element.value().attr("value").map(|value| value.to_string()))
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
        let document = Html::parse_document(&self.current_page.as_ref().ok_or(
            VtopError::ParseError("Current page not found at csrf extration".into()),
        )?);
        let selector = Selector::parse("input[name='_csrf']").unwrap();
        let csrf_token = document
            .select(&selector)
            .next()
            .and_then(|element| element.value().attr("value"))
            .ok_or(VtopError::ParseError("CSRF token not found".to_string()))?;
        self.session.set_csrf_token(csrf_token.to_string());
        Ok(())
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
            return Err(VtopError::VtopServerError);
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
        let ptext = r#"span.text-danger.text-center[role="alert"]"#;
        let document = Html::parse_document(data);
        let selector = Selector::parse(&ptext).unwrap();
        if let Some(element) = document.select(&selector).next() {
            let error_message = element.text().collect::<Vec<_>>().join(" ");
            let trimmed = error_message.trim();
            if trimmed.is_empty() {
                "We could not sign you in. Please try again.".into()
            } else {
                trimmed.into()
            }
        } else {
            "We could not sign you in because VTOP returned an unexpected response.".into()
        }
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

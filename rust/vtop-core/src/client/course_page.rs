//! The course page: every class of a course this semester, and each
//! class's lecture plan with its reference material.

use super::{network_error, server_error, SendWith, VtopClient};
use crate::error::{VtopError, VtopResult};
use crate::inputs::{CourseFilePath, SemesterId};
use crate::parser::course_page;

impl VtopClient {
    /// The menu page, which lists the semesters.
    pub async fn course_page_menu(&self) -> VtopResult<String> {
        let context = "get_course_page";
        let auth = self.authorize(context).await?;
        let url = self.config.url("/vtop/academics/common/StudentCoursePage");
        let body = format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            auth.registration_number, auth.csrf,
        );
        Self::log_request(&format!("{context}.send"), "POST", &url);
        self.send_authenticated(context, || self.http.post(&url).body(body.clone()))
            .await
    }

    /// The courses offered in [semester], as a `<select>` fragment.
    pub async fn course_page_courses_page(&self, semester: &SemesterId) -> VtopResult<String> {
        let context = "get_course_page_courses";
        let auth = self.authorize(context).await?;
        let url = self.config.url("/vtop/getCourseForCoursePage");
        let form = [
            ("_csrf", auth.csrf.as_str()),
            ("paramReturnId", "getCourseForCoursePage"),
            ("semSubId", semester.as_str()),
            ("authorizedID", auth.registration_number.as_str()),
        ];
        Self::log_request(&format!("{context}.send"), "POST", &url);
        self.send_authenticated(context, || self.http.post(&url).form(&form))
            .await
    }

    /// Every class of the course [course_id] (the value of a course
    /// option): slot, faculty and the ids to open its lecture plan.
    pub async fn course_page_classes_page(
        &self,
        semester: &SemesterId,
        course_id: &str,
    ) -> VtopResult<String> {
        let context = "get_course_page_classes";
        let auth = self.authorize(context).await?;
        let url = self.config.url("/vtop/getSlotIdForCoursePage");
        let form = [
            ("_csrf", auth.csrf.as_str()),
            ("classId", course_id),
            ("praType", "source"),
            ("paramReturnId", "getSlotIdForCoursePage"),
            ("semSubId", semester.as_str()),
            ("authorizedID", auth.registration_number.as_str()),
        ];
        Self::log_request(&format!("{context}.send"), "POST", &url);
        self.send_authenticated(context, || self.http.post(&url).form(&form))
            .await
    }

    /// One class's lecture plan and materials.
    pub async fn course_page_detail_page(
        &self,
        semester: &SemesterId,
        erp_id: &str,
        class_id: &str,
    ) -> VtopResult<String> {
        let context = "get_course_page_detail";
        let auth = self.authorize(context).await?;
        let url = self.config.url("/vtop/processViewStudentCourseDetail");
        let form = [
            ("_csrf", auth.csrf.as_str()),
            ("semSubId", semester.as_str()),
            ("erpId", erp_id),
            ("classId", class_id),
            ("authorizedID", auth.registration_number.as_str()),
        ];
        Self::log_request(&format!("{context}.send"), "POST", &url);
        self.send_authenticated(context, || self.http.post(&url).form(&form))
            .await
    }
}

/// A file from the course page: its name as VTOP sends it, and the bytes.
#[derive(Debug, Clone)]
pub struct CourseFile {
    pub file_name: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

impl VtopClient {
    /// Downloads what one of the course page's `vtopDownload('…')` links
    /// points at: a lecture's material, the syllabus or a material bundle.
    pub async fn course_page_file(&self, path: &CourseFilePath) -> VtopResult<CourseFile> {
        let auth = self.authorize("get_course_page_file").await?;
        let url = self.config.url(&format!("/vtop/{}", path.as_str()));
        self.download_course_file(
            &url,
            &[
                ("authorizedID", auth.registration_number.to_string()),
                ("_csrf", auth.csrf.clone()),
                ("x", httpdate::fmt_http_date(std::time::SystemTime::now())),
            ],
        )
        .await
    }

    /// The class's course plan, an Excel sheet.
    pub async fn course_plan(
        &self,
        semester: &SemesterId,
        class_id: &str,
    ) -> VtopResult<CourseFile> {
        let class_id = plain_id(class_id, "class id")?;
        let auth = self.authorize("get_course_plan").await?;
        let url = self
            .config
            .url("/vtop/academics/common/CoursePlanExcelDownload");
        self.download_course_file(
            &url,
            &[
                ("semesterSubId", semester.as_str().to_string()),
                ("classId", class_id.to_string()),
                ("authorizedID", auth.registration_number.to_string()),
                ("x", httpdate::fmt_http_date(std::time::SystemTime::now())),
            ],
        )
        .await
    }

    async fn download_course_file(
        &self,
        url: &str,
        query: &[(&str, String)],
    ) -> VtopResult<CourseFile> {
        let context = "get_course_page_file";
        Self::log_request(context, "GET", url);
        let response = self
            .http
            .get(url)
            .query(query)
            .send_with(self)
            .await
            .map_err(|error| network_error(&format!("{context}.send"), error))?;
        let final_url = response.url().to_string();
        if Self::is_login_url(&final_url) {
            self.mark_session_expired(context, &format!("VTOP redirected to {final_url}"));
            return Err(VtopError::SessionExpired);
        }
        let status = response.status();
        let header = |name: reqwest::header::HeaderName| {
            response
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string()
        };
        let content_type = header(reqwest::header::CONTENT_TYPE);
        let file_name = file_name_from_disposition(&header(reqwest::header::CONTENT_DISPOSITION));
        let bytes = response
            .bytes()
            .await
            .map_err(|error| network_error(&format!("{context}.bytes"), error))?;
        if !status.is_success() {
            return Err(server_error(
                context,
                format!("VTOP returned HTTP {status}"),
            ));
        }
        // An HTML answer is VTOP's error page, not the file.
        if content_type.starts_with("text/html") {
            return Err(VtopError::VtopServerError(
                "VTOP did not send the file".to_string(),
            ));
        }
        Ok(CourseFile {
            file_name,
            content_type,
            bytes: bytes.to_vec(),
        })
    }
}

/// VTOP's response headers for a course page download: what a browser
/// knows when it hands a download to the system download manager.
#[derive(Debug, Clone)]
pub struct CourseFileInfo {
    /// Raw `Content-Disposition`, which carries the file name and extension
    /// the link itself does not show.
    pub content_disposition: String,
    pub content_type: String,
}

impl VtopClient {
    /// The name and type of what a course page link points at, without
    /// downloading it: the body is dropped once the headers arrive. Lets
    /// the system download manager save it under a proper name.
    pub async fn course_page_file_info(&self, path: &CourseFilePath) -> VtopResult<CourseFileInfo> {
        let auth = self.authorize("get_course_page_file_info").await?;
        let url = self.config.url(&format!("/vtop/{}", path.as_str()));
        self.course_file_info(
            &url,
            &[
                ("authorizedID", auth.registration_number.to_string()),
                ("_csrf", auth.csrf.clone()),
                ("x", httpdate::fmt_http_date(std::time::SystemTime::now())),
            ],
        )
        .await
    }

    /// [`Self::course_page_file_info`] for the class's course plan.
    pub async fn course_plan_info(
        &self,
        semester: &SemesterId,
        class_id: &str,
    ) -> VtopResult<CourseFileInfo> {
        let class_id = plain_id(class_id, "class id")?;
        let auth = self.authorize("get_course_plan_info").await?;
        let url = self
            .config
            .url("/vtop/academics/common/CoursePlanExcelDownload");
        self.course_file_info(
            &url,
            &[
                ("semesterSubId", semester.as_str().to_string()),
                ("classId", class_id.to_string()),
                ("authorizedID", auth.registration_number.to_string()),
                ("x", httpdate::fmt_http_date(std::time::SystemTime::now())),
            ],
        )
        .await
    }

    async fn course_file_info(
        &self,
        url: &str,
        query: &[(&str, String)],
    ) -> VtopResult<CourseFileInfo> {
        let context = "get_course_page_file_info";
        Self::log_request(context, "GET", url);
        let response = self
            .http
            .get(url)
            .query(query)
            .send_with(self)
            .await
            .map_err(|error| network_error(&format!("{context}.send"), error))?;
        let final_url = response.url().to_string();
        if Self::is_login_url(&final_url) {
            self.mark_session_expired(context, &format!("VTOP redirected to {final_url}"));
            return Err(VtopError::SessionExpired);
        }
        let status = response.status();
        if !status.is_success() {
            return Err(server_error(
                context,
                format!("VTOP returned HTTP {status}"),
            ));
        }
        let header = |name: reqwest::header::HeaderName| {
            response
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string()
        };
        let content_type = header(reqwest::header::CONTENT_TYPE);
        if content_type.starts_with("text/html") {
            return Err(VtopError::VtopServerError(
                "VTOP did not send the file".to_string(),
            ));
        }
        Ok(CourseFileInfo {
            content_disposition: header(reqwest::header::CONTENT_DISPOSITION),
            content_type,
        })
        // `response` drops here with its body unread, closing the stream.
    }
}

/// `attachment; filename="Unit 1.pdf"` → `Unit 1.pdf`; empty when absent.
fn file_name_from_disposition(header: &str) -> String {
    header
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("filename="))
        .map(|name| name.trim_matches('"').trim().to_string())
        .map(|name| {
            name.rsplit(['/', '\\'])
                .next()
                .unwrap_or_default()
                .to_string()
        })
        .unwrap_or_default()
}

impl VtopClient {
    pub async fn course_page_courses(
        &self,
        semester: &SemesterId,
    ) -> VtopResult<crate::types::CoursePageCourses> {
        let page = self.course_page_courses_page(semester).await?;
        Ok(course_page::parse_courses(&page, semester.as_str()))
    }

    pub async fn course_page_classes(
        &self,
        semester: &SemesterId,
        course_id: &str,
    ) -> VtopResult<crate::types::CoursePageClasses> {
        let course_id = plain_id(course_id, "course id")?;
        let page = self.course_page_classes_page(semester, course_id).await?;
        Ok(course_page::parse_classes(
            &page,
            semester.as_str(),
            course_id,
        ))
    }

    pub async fn course_page_detail(
        &self,
        semester: &SemesterId,
        erp_id: &str,
        class_id: &str,
    ) -> VtopResult<crate::types::CoursePageDetail> {
        let erp_id = plain_id(erp_id, "faculty id")?;
        let class_id = plain_id(class_id, "class id")?;
        let page = self
            .course_page_detail_page(semester, erp_id, class_id)
            .await?;
        Ok(course_page::parse_detail(&page, semester.as_str(), erp_id))
    }
}

/// Ids from the course page go back into form posts: letters and digits only.
fn plain_id<'a>(value: &'a str, label: &str) -> VtopResult<&'a str> {
    let value = value.trim();
    if value.is_empty() || value.len() > 40 || !value.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(VtopError::ConfigurationError(format!(
            "{label} is not valid"
        )));
    }
    Ok(value)
}

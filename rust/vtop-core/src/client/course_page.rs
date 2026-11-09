//! The course page: every class of a course this semester, and each
//! class's lecture plan with its reference material.

use log::warn;
use vtop_proto::course_pages::{self, file_head, plain_id};
use vtop_proto::http::Request;

use super::{network_error, send_error, HopResponse, VtopClient, NETWORK};
use crate::error::{VtopError, VtopResult};
use crate::inputs::{CourseFilePath, SemesterId};
use crate::parser::course_page;

pub use vtop_proto::course_pages::{CourseFile, CourseFileInfo};

impl VtopClient {
    /// The menu page, which lists the semesters.
    pub async fn course_page_menu(&self) -> VtopResult<String> {
        self.page("get_course_page", course_pages::menu).await
    }

    /// The courses offered in [semester], as a `<select>` fragment.
    pub async fn course_page_courses_page(&self, semester: &SemesterId) -> VtopResult<String> {
        self.page("get_course_page_courses", |s| {
            course_pages::courses(s, semester)
        })
        .await
    }

    /// Every class of the course [course_id] (the value of a course
    /// option): slot, faculty and the ids to open its lecture plan.
    pub async fn course_page_classes_page(
        &self,
        semester: &SemesterId,
        course_id: &str,
    ) -> VtopResult<String> {
        self.page("get_course_page_classes", |s| {
            course_pages::classes(s, semester, course_id)
        })
        .await
    }

    /// One class's lecture plan and materials.
    pub async fn course_page_detail_page(
        &self,
        semester: &SemesterId,
        erp_id: &str,
        class_id: &str,
    ) -> VtopResult<String> {
        self.page("get_course_page_detail", |s| {
            course_pages::detail(s, semester, erp_id, class_id)
        })
        .await
    }

    /// Downloads what one of the course page's `vtopDownload('…')` links
    /// points at: a lecture's material, the syllabus or a material bundle.
    pub async fn course_page_file(&self, path: &CourseFilePath) -> VtopResult<CourseFile> {
        self.open_course_page_file(path).await?.collect().await
    }

    /// The class's course plan, an Excel sheet.
    pub async fn course_plan(
        &self,
        semester: &SemesterId,
        class_id: &str,
    ) -> VtopResult<CourseFile> {
        self.open_course_plan(semester, class_id)
            .await?
            .collect()
            .await
    }

    async fn open_course_page_file(&self, path: &CourseFilePath) -> VtopResult<CourseFileDownload> {
        let signed = self.signed("get_course_page_file").await?;
        let request = course_pages::file(&signed, path, vtop_proto::now_unix());
        self.open_course_file("get_course_page_file", request).await
    }

    async fn open_course_plan(
        &self,
        semester: &SemesterId,
        class_id: &str,
    ) -> VtopResult<CourseFileDownload> {
        let class_id = plain_id(class_id, "class id")?;
        let signed = self.signed("get_course_plan").await?;
        let request =
            course_pages::course_plan(&signed, semester, class_id, vtop_proto::now_unix());
        self.open_course_file("get_course_page_file", request).await
    }

    /// [`Self::course_page_file`] for a downloader outside this client
    /// (Android's DownloadManager, straight to VTOP): signs in if needed,
    /// then asks VTOP for the file's name and type (the body is dropped),
    /// so the downloader can save it under that name.
    pub async fn course_page_file_link(&self, path: &CourseFilePath) -> VtopResult<FileLink> {
        let signed = self.signed("get_course_page_file").await?;
        self.file_link(course_pages::file(&signed, path, vtop_proto::now_unix()))
            .await
    }

    /// [`Self::course_plan`] as [`Self::course_page_file_link`].
    pub async fn course_plan_link(
        &self,
        semester: &SemesterId,
        class_id: &str,
    ) -> VtopResult<FileLink> {
        let class_id = plain_id(class_id, "class id")?;
        let signed = self.signed("get_course_plan").await?;
        self.file_link(course_pages::course_plan(
            &signed,
            semester,
            class_id,
            vtop_proto::now_unix(),
        ))
        .await
    }

    async fn file_link(&self, request: Request) -> VtopResult<FileLink> {
        let url = request.url.clone();
        let (_, info) = self
            .send_course_file("get_course_page_file_info", request)
            .await?;
        // The response drops with its body unread, closing the stream.
        let cookie = self.cookie_header().ok_or(VtopError::SessionExpired)?;
        Ok(FileLink {
            url,
            cookie,
            file_name: info.file_name(),
            content_type: info.content_type,
        })
    }

    /// The name and type of what a course page link points at, without
    /// downloading it: the body is dropped once the headers arrive. Lets
    /// the system download manager save it under a proper name.
    pub async fn course_page_file_info(&self, path: &CourseFilePath) -> VtopResult<CourseFileInfo> {
        let signed = self.signed("get_course_page_file_info").await?;
        let request = course_pages::file(&signed, path, vtop_proto::now_unix());
        let (_, info) = self
            .send_course_file("get_course_page_file_info", request)
            .await?;
        Ok(info)
        // The response drops with its body unread, closing the stream.
    }

    /// [`Self::course_page_file_info`] for the class's course plan.
    pub async fn course_plan_info(
        &self,
        semester: &SemesterId,
        class_id: &str,
    ) -> VtopResult<CourseFileInfo> {
        let class_id = plain_id(class_id, "class id")?;
        let signed = self.signed("get_course_plan_info").await?;
        let request =
            course_pages::course_plan(&signed, semester, class_id, vtop_proto::now_unix());
        let (_, info) = self
            .send_course_file("get_course_page_file_info", request)
            .await?;
        Ok(info)
    }

    /// Sends the request and checks the answer is the file; the body is
    /// left to read.
    async fn open_course_file(
        &self,
        context: &str,
        request: Request,
    ) -> VtopResult<CourseFileDownload> {
        let (response, info) = self.send_course_file(context, request).await?;
        Ok(CourseFileDownload {
            file_name: info.file_name(),
            content_type: info.content_type,
            response,
        })
    }

    /// Sends a download `GET` (redirects followed, no retry) and checks its
    /// status and headers with [`file_head`]; the body is not read. The
    /// query carries the CSRF token, so it is never logged.
    async fn send_course_file(
        &self,
        context: &str,
        request: Request,
    ) -> VtopResult<(HopResponse, CourseFileInfo)> {
        let path = request.url.split('?').next().unwrap_or_default();
        Self::log_request(context, "GET", path);
        let response = self
            .send_request(request)
            .await
            .map_err(|error| send_error(&format!("{context}.send"), error))?;
        let info = self.check_file_head(context, &response.head())?;
        Ok((response, info))
    }

    /// [`file_head`]; a bounce to the login page ends the session.
    fn check_file_head(
        &self,
        context: &str,
        head: &vtop_proto::http::Response,
    ) -> VtopResult<CourseFileInfo> {
        match file_head(head) {
            Ok(info) => Ok(info),
            Err(VtopError::SessionExpired) => {
                self.mark_session_expired(context, &format!("VTOP redirected to {}", head.url));
                Err(VtopError::SessionExpired)
            }
            Err(error) => {
                warn!(target: NETWORK, "{context}: {error}");
                Err(error)
            }
        }
    }
}

/// A VTOP file request for another downloader: the URL (its CSRF token in
/// the query) and the `Cookie` header it needs, both secrets never to log,
/// and the file's name and type from VTOP.
#[derive(Clone, PartialEq, Eq)]
pub struct FileLink {
    pub url: String,
    pub cookie: String,
    pub file_name: String,
    pub content_type: String,
}

/// A course page file on its way: VTOP's name and type, then the body.
#[derive(Debug)]
struct CourseFileDownload {
    file_name: String,
    content_type: String,
    response: HopResponse,
}

impl CourseFileDownload {
    /// The whole body in memory.
    async fn collect(self) -> VtopResult<CourseFile> {
        let bytes = self
            .response
            .bytes()
            .await
            .map_err(|error| network_error("get_course_page_file.bytes", error))?;
        Ok(CourseFile {
            file_name: self.file_name,
            content_type: self.content_type,
            bytes,
        })
    }
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

#[cfg(all(test, feature = "reqwest"))]
mod tests {
    use vtop_proto::machine::Signed;

    use super::*;

    /// The download URLs are what reqwest's `.query()` built before.
    #[test]
    fn download_urls_match_reqwest_query() {
        let signed = Signed {
            base_url: "https://vtop.example".into(),
            csrf: "1f0c-9a5b+/=".into(),
            registration_number: "22BCE0001".into(),
        };
        let now = 1_794_218_405;
        let x =
            httpdate::fmt_http_date(std::time::UNIX_EPOCH + std::time::Duration::from_secs(now));
        let old = |url: &str, query: &[(&str, &str)]| {
            reqwest::Client::new()
                .get(url)
                .query(query)
                .build()
                .unwrap()
                .url()
                .to_string()
        };
        let path =
            CourseFilePath::parse("downloadPdf/AP2026272/AP2026272000046/19/17-07-2026").unwrap();
        assert_eq!(
            course_pages::file(&signed, &path, now).url,
            old(
                "https://vtop.example/vtop/downloadPdf/AP2026272/AP2026272000046/19/17-07-2026",
                &[
                    ("authorizedID", "22BCE0001"),
                    ("_csrf", "1f0c-9a5b+/="),
                    ("x", &x)
                ]
            )
        );
        let semester = SemesterId::parse("AP2026272").unwrap();
        assert_eq!(
            course_pages::course_plan(&signed, &semester, "AP2026272000046", now).url,
            old(
                "https://vtop.example/vtop/academics/common/CoursePlanExcelDownload",
                &[
                    ("semesterSubId", "AP2026272"),
                    ("classId", "AP2026272000046"),
                    ("authorizedID", "22BCE0001"),
                    ("x", &x)
                ]
            )
        );
    }

    /// The form posts are what reqwest's `.form()` sent before.
    #[test]
    fn form_bodies_match_reqwest_form() {
        let signed = Signed {
            base_url: "https://vtop.example".into(),
            csrf: "1f0c-9a5b+/=".into(),
            registration_number: "22BCE0001".into(),
        };
        let semester = SemesterId::parse("AP2026272").unwrap();
        let old = |form: &[(&str, &str)]| {
            reqwest::Client::new()
                .post("https://vtop.example/")
                .form(form)
                .build()
                .unwrap()
                .body()
                .and_then(|body| body.as_bytes())
                .unwrap()
                .to_vec()
        };
        assert_eq!(
            course_pages::detail(&signed, &semester, "70435", "AP1").body,
            old(&[
                ("_csrf", "1f0c-9a5b+/="),
                ("semSubId", "AP2026272"),
                ("erpId", "70435"),
                ("classId", "AP1"),
                ("authorizedID", "22BCE0001"),
            ])
        );
        assert_eq!(
            course_pages::classes(&signed, &semester, "AP1").body,
            old(&[
                ("_csrf", "1f0c-9a5b+/="),
                ("classId", "AP1"),
                ("praType", "source"),
                ("paramReturnId", "getSlotIdForCoursePage"),
                ("semSubId", "AP2026272"),
                ("authorizedID", "22BCE0001"),
            ])
        );
        assert_eq!(
            course_pages::courses(&signed, &semester).body,
            old(&[
                ("_csrf", "1f0c-9a5b+/="),
                ("paramReturnId", "getCourseForCoursePage"),
                ("semSubId", "AP2026272"),
                ("authorizedID", "22BCE0001"),
            ])
        );
    }
}

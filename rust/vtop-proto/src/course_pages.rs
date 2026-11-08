//! Course pages: the course list, each course's classes and lecture plan,
//! and the files they link to. Requests are byte for byte what the old
//! client sent. The pages run with [`OnePost`] and parse with
//! [`crate::parser::course_page`]; a file runs with `OnePost::reading` and
//! [`course_file`], or a streaming transport checks its headers with [`file_head`].
//!
//! [`OnePost`]: crate::machine::OnePost

use crate::error::{VtopError, VtopResult};
use crate::http::{Request, Response, form_encode, http_date};
use crate::inputs::{CourseFilePath, SemesterId};
use crate::machine::{Signed, is_login_url};

/// The menu page, which lists the semesters.
pub fn menu(s: &Signed) -> Request {
    Request::post(
        s.url("/vtop/academics/common/StudentCoursePage"),
        format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            s.registration_number, s.csrf,
        ),
    )
}

/// The courses offered in `semester`, as a `<select>` fragment.
pub fn courses(s: &Signed, semester: &SemesterId) -> Request {
    Request::form(
        s.url("/vtop/getCourseForCoursePage"),
        &[
            ("_csrf", &s.csrf),
            ("paramReturnId", "getCourseForCoursePage"),
            ("semSubId", semester.as_str()),
            ("authorizedID", &s.registration_number),
        ],
    )
}

/// Every class of the course `course_id` (the value of a course option).
pub fn classes(s: &Signed, semester: &SemesterId, course_id: &str) -> Request {
    Request::form(
        s.url("/vtop/getSlotIdForCoursePage"),
        &[
            ("_csrf", &s.csrf),
            ("classId", course_id),
            ("praType", "source"),
            ("paramReturnId", "getSlotIdForCoursePage"),
            ("semSubId", semester.as_str()),
            ("authorizedID", &s.registration_number),
        ],
    )
}

/// One class's lecture plan and materials.
pub fn detail(s: &Signed, semester: &SemesterId, erp_id: &str, class_id: &str) -> Request {
    Request::form(
        s.url("/vtop/processViewStudentCourseDetail"),
        &[
            ("_csrf", &s.csrf),
            ("semSubId", semester.as_str()),
            ("erpId", erp_id),
            ("classId", class_id),
            ("authorizedID", &s.registration_number),
        ],
    )
}

/// What one of the course page's `vtopDownload('…')` links points at: a
/// lecture's material, the syllabus or a material bundle. `now` is unix
/// seconds, sent as the `x` cache buster.
pub fn file(s: &Signed, path: &CourseFilePath, now: u64) -> Request {
    get_with_query(
        s.url(&format!("/vtop/{}", path.as_str())),
        &[
            ("authorizedID", &s.registration_number),
            ("_csrf", &s.csrf),
            ("x", &http_date(now)),
        ],
    )
}

/// The class's course plan, an Excel sheet. `class_id` has been through
/// [`plain_id`].
pub fn course_plan(s: &Signed, semester: &SemesterId, class_id: &str, now: u64) -> Request {
    get_with_query(
        s.url("/vtop/academics/common/CoursePlanExcelDownload"),
        &[
            ("semesterSubId", semester.as_str()),
            ("classId", class_id),
            ("authorizedID", &s.registration_number),
            ("x", &http_date(now)),
        ],
    )
}

/// Ids from the course page go back into form posts: letters and digits
/// only, at most 40.
pub fn plain_id<'a>(value: &'a str, label: &str) -> VtopResult<&'a str> {
    let value = value.trim();
    if value.is_empty() || value.len() > 40 || !value.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(VtopError::ConfigurationError(format!(
            "{label} is not valid"
        )));
    }
    Ok(value)
}

/// A file from the course page: its name as VTOP sends it, and the bytes.
#[derive(Debug, Clone)]
pub struct CourseFile {
    pub file_name: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

/// VTOP's response headers for a course page download: what a browser
/// knows when it hands a download to the system download manager.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CourseFileInfo {
    /// Raw `Content-Disposition`, which carries the file name and extension
    /// the link itself does not show.
    pub content_disposition: String,
    pub content_type: String,
}

impl CourseFileInfo {
    /// The file name in [`Self::content_disposition`].
    pub fn file_name(&self) -> String {
        file_name_from_disposition(&self.content_disposition)
    }
}

/// Checks a download's answer is the file, from its status, URL and headers
/// only (the body may still be streaming). A bounce to the login page is
/// [`VtopError::SessionExpired`]; VTOP down or slow (502, 503, 504) is
/// [`VtopError::VtopUnavailable`]; another failure status, or an HTML page
/// (VTOP's error page), is [`VtopError::VtopServerError`].
pub fn file_head(response: &Response) -> VtopResult<CourseFileInfo> {
    if is_login_url(&response.url) || response.redirect().is_some_and(is_login_url) {
        return Err(VtopError::SessionExpired);
    }
    match response.status {
        200..=299 => {}
        504 => {
            return Err(VtopError::VtopUnavailable(
                "VTOP took too long to answer".into(),
            ));
        }
        502 | 503 => return Err(VtopError::VtopUnavailable("VTOP is down right now".into())),
        status => {
            return Err(VtopError::VtopServerError(format!(
                "VTOP returned HTTP {status}"
            )));
        }
    }
    let content_type = response.header("content-type").unwrap_or_default();
    if content_type.starts_with("text/html") {
        return Err(VtopError::VtopServerError(
            "VTOP did not send the file".into(),
        ));
    }
    Ok(CourseFileInfo {
        content_disposition: response
            .header("content-disposition")
            .unwrap_or_default()
            .to_string(),
        content_type: content_type.to_string(),
    })
}

/// `attachment; filename="Unit 1.pdf"` → `Unit 1.pdf`; empty when absent.
pub fn file_name_from_disposition(header: &str) -> String {
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

/// [`file_head`], then the whole body: read a download held in memory with
/// `OnePost::reading(request, course_file)`.
pub fn course_file(response: &Response) -> VtopResult<CourseFile> {
    let info = file_head(response)?;
    Ok(CourseFile {
        file_name: info.file_name(),
        content_type: info.content_type,
        bytes: response.body.clone(),
    })
}

/// A `GET` of `url` with `pairs` as its query, encoded as reqwest's
/// `.query()` did.
fn get_with_query(url: String, pairs: &[(&str, &str)]) -> Request {
    Request::get(format!("{url}?{}", form_encode(pairs)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::{Machine, OnePost, Step};
    use crate::parser::course_page;

    fn signed() -> Signed {
        Signed {
            base_url: "https://vtop.example".into(),
            csrf: "1f0c-9a5b".into(),
            registration_number: "22BCE0001".into(),
        }
    }

    fn semester() -> SemesterId {
        SemesterId::parse("AP2026272").unwrap()
    }

    fn body(request: &Request) -> &str {
        std::str::from_utf8(&request.body).unwrap()
    }

    fn answer(status: u16, url: &str, headers: &[(&str, &str)], body: &[u8]) -> Response {
        Response {
            status,
            headers: headers
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
            url: url.into(),
            body: body.to_vec(),
        }
    }

    // 2026-11-09 10:00:05 UTC.
    const NOW: u64 = 1_794_218_405;

    #[test]
    fn page_requests_match_the_old_client() {
        let s = signed();
        let sem = semester();
        let request = menu(&s);
        assert_eq!(
            request.url,
            "https://vtop.example/vtop/academics/common/StudentCoursePage"
        );
        assert_eq!(
            body(&request),
            "verifyMenu=true&authorizedID=22BCE0001&_csrf=1f0c-9a5b&nocache=@(new Date().getTime())"
        );
        let request = courses(&s, &sem);
        assert_eq!(
            request.url,
            "https://vtop.example/vtop/getCourseForCoursePage"
        );
        assert_eq!(
            body(&request),
            "_csrf=1f0c-9a5b&paramReturnId=getCourseForCoursePage&semSubId=AP2026272&authorizedID=22BCE0001"
        );
        let request = classes(&s, &sem, "AP2026272000055");
        assert_eq!(
            request.url,
            "https://vtop.example/vtop/getSlotIdForCoursePage"
        );
        assert_eq!(
            body(&request),
            "_csrf=1f0c-9a5b&classId=AP2026272000055&praType=source&paramReturnId=getSlotIdForCoursePage&semSubId=AP2026272&authorizedID=22BCE0001"
        );
        let request = detail(&s, &sem, "70435", "AP2026272000046");
        assert_eq!(
            request.url,
            "https://vtop.example/vtop/processViewStudentCourseDetail"
        );
        assert_eq!(
            body(&request),
            "_csrf=1f0c-9a5b&semSubId=AP2026272&erpId=70435&classId=AP2026272000046&authorizedID=22BCE0001"
        );
        assert_eq!(
            request.headers,
            [(
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded".to_string()
            )]
        );
    }

    #[test]
    fn download_requests_match_the_old_client() {
        let s = signed();
        let path =
            CourseFilePath::parse("downloadPdf/AP2026272/AP2026272000046/19/17-07-2026").unwrap();
        let request = file(&s, &path, NOW);
        assert_eq!(request.method, crate::http::Method::Get);
        assert!(request.body.is_empty() && request.headers.is_empty());
        assert_eq!(
            request.url,
            "https://vtop.example/vtop/downloadPdf/AP2026272/AP2026272000046/19/17-07-2026\
             ?authorizedID=22BCE0001&_csrf=1f0c-9a5b&x=Mon%2C+09+Nov+2026+10%3A00%3A05+GMT"
        );
        assert_eq!(
            course_plan(&s, &semester(), "AP2026272000046", NOW).url,
            "https://vtop.example/vtop/academics/common/CoursePlanExcelDownload\
             ?semesterSubId=AP2026272&classId=AP2026272000046&authorizedID=22BCE0001\
             &x=Mon%2C+09+Nov+2026+10%3A00%3A05+GMT"
        );
    }

    #[test]
    fn query_matches_reqwest_with_an_http_date() {
        let time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(NOW);
        assert_eq!(http_date(NOW), httpdate::fmt_http_date(time));
        assert_eq!(
            get_with_query("https://v/x".into(), &[("a", "b c"), ("d", "~*")]).url,
            "https://v/x?a=b+c&d=%7E*"
        );
    }

    #[test]
    fn plain_ids() {
        assert_eq!(
            plain_id(" AP2026272000046 ", "class id"),
            Ok("AP2026272000046")
        );
        for bad in ["", "a b", "a&b=1", &"1".repeat(41)] {
            assert!(plain_id(bad, "class id").is_err(), "{bad}");
        }
    }

    #[test]
    fn courses_page_runs_and_parses() {
        let mut machine = OnePost::new(courses(&signed(), &semester()));
        let Step::Send(request) = machine.start() else {
            panic!("expected a request")
        };
        let html = r#"<select id="courseCode"><option value="">--Choose Course --</option>
            <option value="AP2026272000055">CSE4007 - Digital Image Processing - ETH</option></select>"#;
        let Step::Done(Ok(page)) = machine.feed(answer(200, &request.url, &[], html.as_bytes()))
        else {
            panic!("expected the page")
        };
        let data = course_page::parse_courses(&page, "AP2026272");
        assert_eq!(data.courses[0].id, "AP2026272000055");
    }

    #[test]
    fn file_fetch_sends_once_and_names_the_file() {
        let s = signed();
        let path = CourseFilePath::parse("courseSyllabusDownload/AM_CSE4007_00100/ETH").unwrap();
        let mut machine = OnePost::reading(file(&s, &path, NOW), course_file);
        let Step::Send(request) = machine.start() else {
            panic!("expected a request")
        };
        let done = machine.feed(answer(
            200,
            &request.url,
            &[
                ("Content-Type", "application/pdf"),
                (
                    "Content-Disposition",
                    "attachment; filename=\"x/Unit 1.pdf\"",
                ),
            ],
            b"%PDF",
        ));
        let Step::Done(Ok(file)) = done else {
            panic!("expected the file")
        };
        assert_eq!(file.file_name, "Unit 1.pdf");
        assert_eq!(file.content_type, "application/pdf");
        assert_eq!(file.bytes, b"%PDF");
        assert!(matches!(machine.start(), Step::Done(Err(_))));
    }

    #[test]
    fn file_fetch_stops_on_an_expired_session() {
        let url = "https://vtop.example/vtop/downloadPdf/a";
        for expired in [
            answer(200, "https://vtop.example/vtop/login", &[], b""),
            answer(302, url, &[("Location", "/vtop/login")], b""),
        ] {
            let mut machine = OnePost::reading(Request::get(url), course_file);
            machine.start();
            assert!(matches!(
                machine.feed(expired),
                Step::Done(Err(VtopError::SessionExpired))
            ));
        }
    }

    #[test]
    fn file_head_sorts_answers() {
        let url = "https://vtop.example/vtop/downloadPdf/a";
        let html = answer(
            200,
            url,
            &[("content-type", "text/html;charset=UTF-8")],
            b"",
        );
        assert_eq!(
            file_head(&html).unwrap_err(),
            VtopError::VtopServerError("VTOP did not send the file".into())
        );
        assert!(matches!(
            file_head(&answer(503, url, &[], b"")),
            Err(VtopError::VtopUnavailable(_))
        ));
        assert!(matches!(
            file_head(&answer(504, url, &[], b"")),
            Err(VtopError::VtopUnavailable(_))
        ));
        assert_eq!(
            file_head(&answer(404, url, &[], b"")).unwrap_err(),
            VtopError::VtopServerError("VTOP returned HTTP 404".into())
        );
        let info = file_head(&answer(
            200,
            url,
            &[("Content-Type", "application/zip")],
            b"",
        ))
        .unwrap();
        assert_eq!(info.content_type, "application/zip");
        assert_eq!(info.content_disposition, "");
        assert_eq!(info.file_name(), "");
    }

    #[test]
    fn debug_hides_the_csrf_token() {
        let path = CourseFilePath::parse("courseSyllabusDownload/AM_CSE4007_00100/ETH").unwrap();
        let shown = format!("{:?} {:?}", signed(), file(&signed(), &path, NOW));
        assert!(!shown.contains("1f0c-9a5b"), "{shown}");
        assert!(shown.contains("/vtop/courseSyllabusDownload/"), "{shown}");
    }
}

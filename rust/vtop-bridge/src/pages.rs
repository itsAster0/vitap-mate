//! Every VTOP page the engine fetches for a phone, by name: the arguments
//! it takes and the [`VtopClient`] call behind it. The names and arguments
//! are the app's (`lib/core/vtop_backend/vtop_queries.dart`), so a new page
//! is one entry there and one here. Only reads: what changes VTOP (outing
//! applies and cancels) is always sent by the phone itself.

use std::collections::BTreeMap;

use base64::Engine as _;
use futures::future::BoxFuture;
use serde::Serialize;
use serde_json::Value;
use vtop_core::client::OutingKind;
use vtop_core::inputs::{BiometricDate, CourseFilePath, CourseId, CourseType, OutingPassId, SemesterId};
use vtop_core::{VtopClient, VtopError, VtopResult};

/// A page's arguments, by name.
pub type Args = BTreeMap<String, String>;

/// Fetches one page: the client, the request's semester (empty for pages
/// that are not per semester), the arguments (all of [`PageDef::args`]).
pub type PageCall =
    for<'a> fn(&'a VtopClient, &'a str, &'a Args) -> BoxFuture<'a, VtopResult<Value>>;

pub struct PageDef {
    pub name: &'static str,
    pub args: &'static [&'static str],
    pub call: PageCall,
}

pub fn find(name: &str) -> Option<&'static PageDef> {
    PAGES.iter().find(|def| def.name == name)
}

fn json(value: impl Serialize) -> VtopResult<Value> {
    serde_json::to_value(value).map_err(|_| VtopError::InvalidResponse)
}

fn arg<'a>(args: &'a Args, name: &str) -> &'a str {
    args.get(name).map(String::as_str).unwrap_or_default()
}

/// A [`PageCall`] from a body that may use `c` (client), `s` (semester)
/// and `a` (arguments) and `.await?`.
macro_rules! call {
    ($name:ident, |$c:ident, $s:ident, $a:ident| $body:expr) => {
        #[allow(unused_variables)]
        fn $name<'a>(
            $c: &'a VtopClient,
            $s: &'a str,
            $a: &'a Args,
        ) -> BoxFuture<'a, VtopResult<Value>> {
            Box::pin(async move { json($body) })
        }
    };
}

call!(semesters, |c, s, a| c.semesters().await?);
call!(attendance, |c, s, a| c
    .attendance(&SemesterId::parse(s)?)
    .await?);
call!(full_attendance, |c, s, a| c
    .full_attendance(
        &SemesterId::parse(s)?,
        &CourseId::parse(arg(a, "course_id"))?,
        &CourseType::parse(arg(a, "course_type"))?,
    )
    .await?);
call!(timetable, |c, s, a| c
    .timetable(&SemesterId::parse(s)?)
    .await?);
call!(marks, |c, s, a| c.marks(&SemesterId::parse(s)?).await?);
call!(exam_schedule, |c, s, a| c
    .exam_schedule(&SemesterId::parse(s)?)
    .await?);
call!(grades, |c, s, a| c
    .grade_view(&SemesterId::parse(s)?)
    .await?);
call!(academic_calendar, |c, s, a| c
    .academic_calendar(&SemesterId::parse(s)?)
    .await?);
call!(grade_details, |c, s, a| c
    .grade_view_details(
        &SemesterId::parse(s)?,
        &CourseId::parse(arg(a, "course_id"))?
    )
    .await?);
call!(grade_history, |c, s, a| c.grade_history().await?);
call!(biometric, |c, s, a| c
    .biometric_history(&BiometricDate::parse(arg(a, "date"))?)
    .await?);
call!(general_outing, |c, s, a| c.general_outing().await?);
call!(weekend_outing, |c, s, a| c.weekend_outing().await?);
call!(outing_pass, |c, s, a| {
    let kind = match arg(a, "kind") {
        "general" => OutingKind::General,
        "weekend" => OutingKind::Weekend,
        other => {
            return Err(VtopError::ConfigurationError(format!(
                "No outing kind {other}."
            )))
        }
    };
    pdf(c
        .outing_pass(kind, &OutingPassId::parse(arg(a, "pass_id"))?)
        .await?)
});
call!(course_page_courses, |c, s, a| c
    .course_page_courses(&SemesterId::parse(s)?)
    .await?);
call!(course_page_classes, |c, s, a| c
    .course_page_classes(&SemesterId::parse(s)?, arg(a, "course_id"))
    .await?);
call!(course_page_detail, |c, s, a| c
    .course_page_detail(&SemesterId::parse(s)?, arg(a, "erp_id"), arg(a, "class_id"))
    .await?);
call!(course_file_info, |c, s, a| c
    .course_page_file_info(&CourseFilePath::parse(arg(a, "path"))?)
    .await?);
call!(course_plan_info, |c, s, a| c
    .course_plan_info(&SemesterId::parse(s)?, arg(a, "class_id"))
    .await?);

/// A PDF as JSON: `{"pdf": "<standard base64>"}`.
fn pdf(bytes: Vec<u8>) -> Value {
    serde_json::json!({ "pdf": base64::engine::general_purpose::STANDARD.encode(bytes) })
}

const fn page(name: &'static str, args: &'static [&'static str], call: PageCall) -> PageDef {
    PageDef { name, args, call }
}

pub static PAGES: &[PageDef] = &[
    page("semesters", &[], semesters),
    page("attendance", &[], attendance),
    page("full_attendance", &["course_id", "course_type"], full_attendance),
    page("timetable", &[], timetable),
    page("marks", &[], marks),
    page("exam_schedule", &[], exam_schedule),
    page("grades", &[], grades),
    page("academic_calendar", &[], academic_calendar),
    page("grade_details", &["course_id"], grade_details),
    page("grade_history", &[], grade_history),
    page("biometric", &["date"], biometric),
    page("general_outing", &[], general_outing),
    page("weekend_outing", &[], weekend_outing),
    page("outing_pass", &["kind", "pass_id"], outing_pass),
    page("course_page_courses", &[], course_page_courses),
    page("course_page_classes", &["course_id"], course_page_classes),
    page("course_page_detail", &["erp_id", "class_id"], course_page_detail),
    page("course_file_info", &["path"], course_file_info),
    page("course_plan_info", &["class_id"], course_plan_info),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outing_pass_is_base64() {
        assert_eq!(pdf(vec![1, 2, 3]), serde_json::json!({"pdf": "AQID"}));
    }

    #[tokio::test]
    async fn a_bad_biometric_date_is_a_configuration_error() {
        let client = VtopClient::builder()
            .with_session(&vtop_core::SessionState {
                cookies: "A=1".into(),
                csrf_token: None,
                registration_number: None,
                otp_issued_at: None,
                logged_in_at: None,
            })
            .unwrap();
        let args = Args::from([("date".to_string(), "2026-10-20".to_string())]);
        let def = find("biometric").unwrap();
        let error = (def.call)(&client, "AP1", &args).await.unwrap_err();
        assert!(matches!(error, VtopError::ConfigurationError(_)));
    }

    #[test]
    fn every_page_named_once() {
        let mut names: Vec<_> = PAGES.iter().map(|def| def.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), PAGES.len());
    }
}

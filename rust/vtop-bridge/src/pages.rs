//! Every VTOP page the engine fetches for a phone, by name: the read pages
//! of `vtop_proto::spec` (the names and arguments the app sends), then the
//! files a phone asks about. Only reads: what changes VTOP (outing applies
//! and cancels) is always sent by the phone itself.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use base64::Engine as _;
use futures::future::BoxFuture;
use serde::Serialize;
use serde_json::Value;
use vtop_core::client::OutingKind;
use vtop_core::inputs::{CourseFilePath, OutingPassId, SemesterId};
use vtop_core::spec::{self, PageSpec, Query};
use vtop_core::{VtopClient, VtopError, VtopResult};

/// A page's arguments, by name.
pub type Args = BTreeMap<String, String>;

/// Fetches one page: the client, the request's semester (empty for pages
/// that are not per semester), the arguments (all of [`PageDef::args`]).
pub type PageCall = Box<
    dyn for<'a> Fn(&'a VtopClient, &'a str, &'a Args) -> BoxFuture<'a, VtopResult<Value>>
        + Send
        + Sync,
>;

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

/// Forces `call`'s signature, which a closure boxed straight away can't infer.
fn page(
    name: &'static str,
    args: &'static [&'static str],
    call: impl for<'a> Fn(&'a VtopClient, &'a str, &'a Args) -> BoxFuture<'a, VtopResult<Value>>
        + Send
        + Sync
        + 'static,
) -> PageDef {
    PageDef {
        name,
        args,
        call: Box::new(call),
    }
}

/// Collects [`spec::each`] as [`PageDef`]s.
struct Defs(Vec<PageDef>);

impl spec::Visit for Defs {
    fn page<T: Serialize + Send + 'static>(&mut self, spec: &'static PageSpec<T>) {
        self.0.push(page(spec.name, spec.args, move |c, s, a| {
            Box::pin(async move {
                let args: Vec<_> = a.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                let query = Query {
                    semester: Some(s),
                    args: &args,
                };
                json(c.fetch(spec, &query).await?)
            })
        }));
    }
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

pub static PAGES: LazyLock<Vec<PageDef>> = LazyLock::new(|| {
    let mut pages = Defs(Vec::new());
    spec::each(&mut pages);
    pages.0.extend([
        page("outing_pass", &["kind", "pass_id"], outing_pass),
        page("course_file_info", &["path"], course_file_info),
        page("course_plan_info", &["class_id"], course_plan_info),
    ]);
    pages.0
});

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

//! Several read pages in one round trip on the Server and UDP routes (`vtop:fetchBatch`,
//! or a [`BATCH`] request to the gateway), for the open-app sweep. The batch is asked about
//! once and signs in again once, as a page does ([`Core::on_server`]); its pages wait in
//! [`super::State::prefetched`] for each page's own fetch (`fetch_attendance`, ...), which
//! caches it as usual. On Direct, when the route is off or refused, or on a server too old
//! for batches, those fetches run as they would alone, a few at a time.

use std::collections::BTreeMap;
use std::task::Poll;

use serde_json::Value;
use tracing::info;
use vtop_shared::engine::{BatchPage, PageBatch};
use vtop_shared::page_dictionary;
use vtop_shared::wire_page::{BATCH, WirePageRequest, batch_args};

use super::plan::ServerError;
use super::{Route, page_data};
use crate::remote::udp::UdpFetchError;
use crate::{Core, CoreError, SEMESTERS, SemesterData, traced};

/// The pages a batch takes, by their engine names: those whose fetch caches the page as it
/// came (with no arguments).
const BATCHED: [&str; 6] = [
    "semesters",
    "attendance",
    "timetable",
    "marks",
    "exam_schedule",
    "academic_calendar",
];

/// The engine names of the pages [`Core::fetch_pages`] takes ([`BATCHED`]), for the app's
/// open-app sweep.
#[uniffi::export]
pub fn batched_pages() -> Vec<String> {
    BATCHED.map(str::to_string).to_vec()
}

/// Page fetches run at once on the phone.
const AT_ONCE: usize = 4;

/// A page of [`Core::fetch_pages`], and why it was not fetched.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PageFetched {
    pub name: String,
    pub error: Option<CoreError>,
}

#[uniffi::export]
impl Core {
    /// Fetches and caches `names` (of [`BATCHED`]) as their own fetches do, in one round trip
    /// on the Server and UDP routes; one page failing leaves the others fetched.
    pub async fn fetch_pages(
        &self,
        flow: String,
        names: Vec<String>,
    ) -> Result<Vec<PageFetched>, CoreError> {
        traced("fetch_pages", &flow, async {
            self.current()?;
            let semester = self
                .read::<SemesterData>(SEMESTERS)
                .and_then(|list| self.pick(&list));
            let known: Vec<&str> = names
                .iter()
                .map(String::as_str)
                .filter(|name| BATCHED.contains(name))
                .collect();
            let answers = match semester {
                Some(semester) if !known.is_empty() => {
                    self.server_batch(&flow, &known, &semester).await
                }
                _ => None,
            };
            let mut refused = BTreeMap::new();
            for (name, answer) in known.iter().zip(answers.into_iter().flatten()) {
                match page_answer(answer) {
                    Ok(data) => {
                        let key = (flow.clone(), name.to_string());
                        self.shared.routes().prefetched.insert(key, data);
                    }
                    Err(error) => {
                        refused.insert(*name, error);
                    }
                }
            }
            let mut results = Vec::new();
            for chunk in names.chunks(AT_ONCE) {
                let runs = chunk.iter().map(|name| async {
                    match refused.get(name.as_str()) {
                        Some(error) => Err(error.clone()),
                        None => self.page_alone(&flow, name).await,
                    }
                });
                results.extend(join_all(runs.collect()).await);
            }
            self.shared
                .routes()
                .prefetched
                .retain(|(of, _), _| *of != flow);
            Ok(names
                .into_iter()
                .zip(results)
                .map(|(name, result)| PageFetched {
                    name,
                    error: result.err(),
                })
                .collect())
        })
        .await
    }
}

impl Core {
    /// Page `name` fetched and cached by its own call.
    async fn page_alone(&self, flow: &str, name: &str) -> Result<(), CoreError> {
        let flow = flow.to_string();
        match name {
            "semesters" => self.fetch_semesters(flow).await.map(drop),
            "attendance" => self.fetch_attendance(flow).await.map(drop),
            "timetable" => self.fetch_timetable(flow).await.map(drop),
            "marks" => self.fetch_marks(flow).await.map(drop),
            "exam_schedule" => self.fetch_exams(flow).await.map(drop),
            "academic_calendar" => self.fetch_calendar(flow).await.map(drop),
            _ => Err(CoreError::Other {
                reason: format!("{name} is not fetched in a batch"),
            }),
        }
    }

    /// Each page's answer (`{"data"}` or `{"code", "error"}`) from the server, in order; None
    /// when the phone fetches them (as [`Core::on_server`], or a server without batches).
    async fn server_batch(&self, flow: &str, names: &[&str], semester: &str) -> Option<Vec<Value>> {
        if self.is_demo() {
            return None; // Its pages come from the demo file, one by one.
        }
        let action = format!("Fetch {} through the server", names.join(", "));
        let send = |session| {
            let request = PageBatch {
                session,
                semester: Some(semester.to_string()),
                pages: names
                    .iter()
                    .map(|name| BatchPage {
                        name: name.to_string(),
                        args: BTreeMap::new(),
                    })
                    .collect(),
                dictionary: Some(page_dictionary::id().to_string()),
            };
            async move { self.batch_on_server(flow, &request).await }
        };
        self.on_server(flow, &action, send)
            .await
            .and_then(Result::ok)
            .flatten()
    }

    /// The batch's answers, over UDP while that route is on, else over the socket; None from a
    /// server that has no batches.
    async fn batch_on_server(
        &self,
        flow: &str,
        request: &PageBatch,
    ) -> Result<Option<Vec<Value>>, ServerError> {
        let count = request.pages.len();
        if self.shared.active(Route::Udp) {
            let page = WirePageRequest {
                request_id: Some(flow.to_string()),
                name: BATCH.to_string(),
                semester: request.semester.clone(),
                args: batch_args(&request.pages),
                dictionary: request.dictionary.clone(),
                session_hash: 0,
                session: None,
                account: None,
            };
            match self
                .udp_fetch(flow, page, request.session.to_string())
                .await
            {
                Ok(data) => {
                    self.shared.used(Route::Udp);
                    return answers(data, count).map(Some);
                }
                Err(UdpFetchError::SessionExpired) => {
                    self.shared.used(Route::Udp);
                    return Err(ServerError::Expired);
                }
                // An older gateway or engine, or a batch too large for UDP: the link is fine.
                Err(
                    error @ UdpFetchError::Gateway {
                        status: 404 | 413, ..
                    },
                ) => {
                    info!(reason = %error, "batch not over UDP");
                }
                Err(error) => self.shared.failed(Route::Udp, &error.to_string()),
            }
        }
        let body = serde_json::to_value(request).unwrap_or_default();
        let answer = match self.engine_call(flow, "vtop:fetchBatch", body).await {
            Err(ServerError::Failed(reason)) if reason.starts_with("not_found") => {
                info!("the server has no batches; page by page");
                return Ok(None);
            }
            answer => answer?,
        };
        self.shared.used(Route::Server);
        // Packed with the batch's name, as the gateway's are.
        let data = page_data(BATCH, answer)
            .ok_or_else(|| ServerError::Failed("unreadable packed batch".into()))?;
        answers(data, count).map(Some)
    }
}

/// A batch's `data`: one answer per page.
fn answers(data: Value, count: usize) -> Result<Vec<Value>, ServerError> {
    match data {
        Value::Array(answers) if answers.len() == count => Ok(answers),
        _ => Err(ServerError::Failed("unreadable batch".into())),
    }
}

/// One page's answer in a batch: its `data`, or the engine's error for it.
fn page_answer(mut answer: Value) -> Result<Value, CoreError> {
    if let Some(data) = answer.get_mut("data") {
        return Ok(data.take());
    }
    let text = |key| answer.get(key).and_then(Value::as_str).unwrap_or_default();
    let reason = text("error").to_string();
    Err(match text("code") {
        "vtop_unavailable" | "vtop_timeout" => CoreError::VtopUnavailable { reason },
        code => CoreError::Other {
            reason: format!("{code}: {reason}"),
        },
    })
}

/// `futures` run together (the app has no executor of its own); their outputs in order.
async fn join_all<F: Future>(futures: Vec<F>) -> Vec<F::Output> {
    let mut runs: Vec<_> = futures
        .into_iter()
        .map(|f| (Box::pin(f), None::<F::Output>))
        .collect();
    std::future::poll_fn(|cx| {
        let mut pending = false;
        for (run, out) in runs.iter_mut().filter(|(_, out)| out.is_none()) {
            match run.as_mut().poll(cx) {
                Poll::Ready(value) => *out = Some(value),
                Poll::Pending => pending = true,
            }
        }
        if pending {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await;
    runs.into_iter().filter_map(|(_, out)| out).collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use std::sync::{Arc, Mutex};

    use super::super::tests::core_on;
    use super::*;
    use crate::remote::consent;
    use crate::test_edge::Edge;
    use crate::{PAGES, SELECTED};

    const FETCH_BATCH: &str = "vtop:fetchBatch";
    const ATTENDANCE: &str =
        include_str!("../../../../vtop-proto/tests/fixtures/attendance.expected.json");

    /// A core on `route` with semester AP1 picked.
    async fn picked(name: &str, route: Route) -> (Core, Arc<Mutex<Edge>>) {
        let (core, edge) = core_on(name, route).await;
        let list = r#"{"semesters":[{"id":"AP1","name":"Fall"}],"update_time":7}"#;
        fs::write(core.at(SEMESTERS), list).unwrap();
        fs::write(core.at(SELECTED), r#""AP1""#).unwrap();
        (core, edge)
    }

    fn paths(edge: &Mutex<Edge>) -> Vec<String> {
        let edge = edge.lock().unwrap();
        let vtop = edge.requests.iter().filter(|r| r.path.starts_with("vtop:"));
        vtop.map(|r| r.path.clone()).collect()
    }

    fn can(edge: &Mutex<Edge>, path: &str, status: u16, body: Value) {
        let mut edge = edge.lock().unwrap();
        edge.canned
            .entry(path.into())
            .or_default()
            .push_back((status, body));
    }

    fn attendance() -> Value {
        serde_json::from_str(ATTENDANCE).unwrap()
    }

    #[tokio::test]
    async fn one_round_trip_caches_the_good_pages_and_reports_the_bad() {
        let (core, edge) = picked("batch-mixed", Route::Server).await;
        let gate = consent::tests::yes(&core);
        let answer = json!({ "data": [{ "data": attendance() }, { "code": "vtop_unavailable", "error": "slow" }] });
        can(&edge, FETCH_BATCH, 200, answer);
        let names = vec!["attendance".to_string(), "marks".to_string()];
        let fetched = core.fetch_pages("f".into(), names).await.unwrap();
        assert_eq!(fetched[0].error, None);
        assert_eq!(
            fetched[1].error,
            Some(CoreError::VtopUnavailable {
                reason: "slow".into()
            })
        );
        assert_eq!(paths(&edge), [FETCH_BATCH]);
        assert_eq!(gate.asks(), 1, "asked once for the batch");
        let body = &edge.lock().unwrap().requests.last().unwrap().body.clone();
        assert_eq!(body["semester"], "AP1");
        assert_eq!(
            body["pages"],
            json!([{ "name": "attendance" }, { "name": "marks" }])
        );
        let saved: Value = serde_json::from_slice(
            &fs::read(core.at(format!("{PAGES}/AP1/attendance.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(saved, attendance());
        assert!(core.shared.routes().prefetched.is_empty());
    }

    #[tokio::test]
    async fn an_old_server_gets_page_by_page() {
        // A server without `vtop:fetchBatch`.
        let (core, edge) = picked("batch-old", Route::Server).await;
        let _gate = consent::tests::yes(&core);
        can(&edge, "vtop:fetch", 200, json!({ "data": attendance() }));
        let fetched = core
            .fetch_pages("f".into(), vec!["attendance".into()])
            .await
            .unwrap();
        assert_eq!(fetched[0].error, None);
        assert_eq!(paths(&edge), [FETCH_BATCH, "vtop:fetch"]);
        assert!(core.route_status("f".into()).skipped.is_empty(), "no skip");
        assert!(core.at(format!("{PAGES}/AP1/attendance.json")).exists());
    }

    #[tokio::test]
    async fn a_page_without_a_batch_is_refused_on_its_own() {
        let (core, edge) = picked("batch-unknown", Route::Server).await;
        let _gate = consent::tests::yes(&core);
        can(
            &edge,
            FETCH_BATCH,
            200,
            json!({ "data": [{ "data": attendance() }] }),
        );
        let names = vec!["attendance".to_string(), "biometric".to_string()];
        let fetched = core.fetch_pages("f".into(), names).await.unwrap();
        assert_eq!(fetched[0].error, None);
        assert!(matches!(fetched[1].error, Some(CoreError::Other { .. })));
    }

    #[test]
    fn batched_pages_are_engine_pages() {
        struct Names(Vec<&'static str>);
        impl vtop_core::spec::Visit for Names {
            fn page<T: serde::Serialize + Send + 'static>(
                &mut self,
                spec: &'static vtop_core::spec::PageSpec<T>,
            ) {
                self.0.push(spec.name);
            }
        }
        let mut names = Names(Vec::new());
        vtop_core::spec::each(&mut names);
        assert!(BATCHED.iter().all(|name| names.0.contains(name)));
    }
}

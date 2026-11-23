//! How the app reaches VTOP (Settings → Connection):
//! - Direct: the phone signs in and fetches from VTOP itself.
//! - Server: the engine fetches and parses read pages on the phone's session (`vtop:fetch`)
//!   and signs in with the saved password when it expired (`vtop:login`); writes, downloads and
//!   the web view stay on the phone.
//! - UDP: the server route over vm1 ([`Core::udp_page`]), then over the server's socket.
//!
//! A route that fails is skipped for [`plan::SKIP_S`] and the request goes on to the next, finally
//! Direct; the student never has to change the setting to get their data. Server and UDP send
//! the VTOP session, so each asks first ([`consent::allowed`]); a no goes Direct and is not
//! skipped, so the next fetch the student watches asks again. A background run uses the chosen
//! route only when the student ticked "don't ask again", else Direct.
//!
//! The policy itself is [`plan`], with no I/O; this module keeps the core's instance (in
//! [`Shared`]) and runs it.

mod batch;
mod plan;

use std::collections::BTreeMap;
use std::sync::{Arc, MutexGuard, PoisonError};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{info, warn};
use vtop_core::client::{Hop, HopFuture, Transport};
use vtop_core::config::DEFAULT_BASE_URL;
use vtop_core::{SessionState, VtopError};
use vtop_shared::engine::PageFetch;
use vtop_shared::page_dictionary;

use crate::device::EdgeError;
use crate::remote::backend::Kind;
use crate::remote::consent::{self, CookieKind};
use crate::{Core, CoreError, Shared, Slot, traced_sync, transport, unix_now};

use plan::{Next, Outcome, RoutePlan, ServerError, ServerFetch};

pub(crate) const FILE: &str = "route.json";

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    #[default]
    Direct,
    Server,
    Udp,
}

/// A route skipped after it failed.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SkippedRoute {
    pub route: Route,
    /// Unix seconds.
    pub until: i64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RouteStatus {
    pub chosen: Route,
    /// The route VTOP's last answer came by, and when (unix seconds).
    pub last_used: Option<Route>,
    pub last_used_at: Option<i64>,
    pub skipped: Vec<SkippedRoute>,
}

#[derive(Default, Serialize, Deserialize)]
struct Saved {
    route: Route,
}

/// The core's route state ([`Shared::routes`]).
pub(crate) struct State {
    plan: RoutePlan,
    /// The server, for the consent dialog.
    server: String,
    /// The span and route of the last "route used" line: one line per call, not per hop.
    logged: Option<(Option<u64>, Route)>,
    /// Pages a batch brought, by flow and name, for that flow's page fetches to take
    /// ([`batch`]).
    prefetched: BTreeMap<(String, String), Value>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            plan: RoutePlan::new(),
            server: String::new(),
            logged: None,
            prefetched: BTreeMap::new(),
        }
    }
}

impl Shared {
    fn routes(&self) -> MutexGuard<'_, State> {
        self.route.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether the student lets `action` send the session over a route: to the server's host.
    async fn consented(&self, flow: &str, action: &str) -> bool {
        let server = consent::host(&self.routes().server);
        consent::allowed(self, flow, CookieKind::Route, action, &server, true).await
    }

    /// [`RoutePlan::active`] now.
    fn active(&self, route: Route) -> bool {
        self.routes().plan.active(route, unix_now())
    }

    fn failed(&self, route: Route, reason: &str) {
        warn!(?route, reason, "route failed; skipped for 5 min");
        let outcome = Outcome::Failed(reason.to_string());
        self.routes().plan.on_result(route, outcome, unix_now());
    }

    fn used(&self, route: Route) {
        let mut state = self.routes();
        state.plan.on_result(route, Outcome::Used, unix_now());
        let span = tracing::Span::current().id().map(|id| id.into_u64());
        if state.logged != Some((span, route)) {
            state.logged = Some((span, route));
            info!(?route, "route used");
        }
    }
}

/// Loads the saved choice; called when the core opens.
pub(crate) fn load(core: &Core) {
    let chosen = core.read::<Saved>(FILE).unwrap_or_default().route;
    let server = core.forwarding_server();
    info!(?chosen, "route");
    let mut state = core.shared.routes();
    (state.plan.chosen, state.server) = (chosen, server);
}

/// Back to Direct with nothing skipped (signed out: the next student picks their own).
pub(crate) fn forget(shared: &Shared) {
    shared.routes().plan.forget();
}

#[uniffi::export]
impl Core {
    pub fn route_status(&self, flow: String) -> RouteStatus {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("route_status", &flow, || Ok(()));
        self.shared.routes().plan.status(unix_now())
    }

    /// Saves the choice; earlier failures are forgotten so it is tried straight away.
    pub fn set_route(&self, flow: String, route: Route) -> Result<(), CoreError> {
        traced_sync("set_route", &flow, || {
            self.write(FILE, &Saved { route });
            self.shared.routes().plan.choose(route);
            info!(?route, "route chosen");
            Ok(())
        })
    }
}

/// VTOP's hops, straight to VTOP: what Direct used is recorded for the Connection screen.
/// Holds its core's [`Shared`], for the plan.
pub(crate) struct Routed(pub(crate) Arc<dyn Transport>, pub(crate) Arc<Shared>);

impl std::fmt::Debug for Routed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_tuple("Routed").field(&self.0).finish()
    }
}

impl Transport for Routed {
    fn send(&self, hop: Hop) -> HopFuture<'_> {
        Box::pin(async move {
            let vtop = hop.url.starts_with(&format!("{DEFAULT_BASE_URL}/"));
            let response = self.0.send(hop).await;
            if vtop && response.is_ok() {
                self.1.used(Route::Direct);
            }
            response
        })
    }

    fn pause(
        &self,
        duration: std::time::Duration,
    ) -> std::pin::Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        self.0.pause(duration)
    }
}

impl From<EdgeError> for ServerError {
    fn from(error: EdgeError) -> Self {
        match error {
            EdgeError::Status { code, .. } if code == "session_expired" => Self::Expired,
            EdgeError::Offline => Self::Failed("server unreachable".into()),
            EdgeError::Status { code, message } => Self::Failed(format!("{code}: {message}")),
        }
    }
}

impl Core {
    /// Page `name` from the server (UDP first when chosen), signing in there again once when the
    /// session expired; None when the route is off, the student said no, or the server failed
    /// (the phone fetches).
    pub(crate) async fn server_page<T: DeserializeOwned>(
        &self,
        name: &str,
        semester: Option<&str>,
        args: &[(&str, &str)],
    ) -> Option<Result<T, CoreError>> {
        if let Some(page) = self.demo_page(name, semester, args).await {
            return Some(page);
        }
        let flow = transport::flow();
        let batched = self
            .shared
            .routes()
            .prefetched
            .remove(&(flow.clone(), name.to_string()));
        if let Some(data) = batched.filter(|_| args.is_empty()) {
            return Some(
                serde_json::from_value(data).map_err(|error| CoreError::Other {
                    reason: format!("unreadable {name}: {error}"),
                }),
            );
        }
        let action = format!("Fetch {name} through the server");
        let send = |session| {
            let request = PageFetch {
                session,
                name: name.to_string(),
                semester: semester.map(str::to_string),
                args: args
                    .iter()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect(),
                // Built in: the engine packs the page with it (plain JSON if its copy differs).
                dictionary: Some(page_dictionary::id().to_string()),
            };
            let flow = &flow;
            async move {
                let data = self.page_on_server(flow, &request).await?;
                serde_json::from_value(data)
                    .map_err(|error| ServerError::Failed(format!("unreadable {name}: {error}")))
            }
        };
        self.on_server(&flow, &action, send).await
    }

    /// `send`'s answer on the server with the session (as JSON), signing in there again once
    /// when it expired; None when the route is off, the student said no, or the server failed
    /// (the phone fetches).
    async fn on_server<R, F: Future<Output = Result<R, ServerError>>>(
        &self,
        flow: &str,
        action: &str,
        send: impl Fn(Value) -> F,
    ) -> Option<Result<R, CoreError>> {
        self.shared.routes().plan.next(unix_now())?;
        let slot = self.current().ok()?;
        // Asked once and before the first fetch, never across the lock: a code that goes in
        // meanwhile can't let the session (or, on expiry, the password) go out unasked.
        let mut fetch = ServerFetch::default();
        let mut next = ServerFetch::start(slot.read().await.otp.is_some());
        let mut seen = 0;
        loop {
            next = match next {
                Next::Ask => fetch.consented(self.shared.consented(flow, action).await),
                Next::Fetch => {
                    #[expect(
                        clippy::disallowed_methods,
                        reason = "Fetch only follows an Ask that consent::allowed said yes to"
                    )]
                    let session = {
                        let session = slot.read().await;
                        if session.otp.is_some() {
                            return None; // Waiting for a code: the phone's path tells the student.
                        }
                        seen = session.logins;
                        session.client.session_state()
                    };
                    match send(serde_json::to_value(&session).unwrap_or_default()).await {
                        Ok(answer) if self.is_current(&slot) => return Some(Ok(answer)),
                        Ok(_) => return Some(Err(CoreError::NotSignedIn)),
                        Err(error) => fetch.fetched(error),
                    }
                }
                Next::Relogin => fetch.relogged(self.server_relogin(&slot, seen, flow).await),
                Next::Fail(reason) => {
                    self.shared.failed(Route::Server, &reason);
                    return None;
                }
                Next::GiveUp => return None,
            };
        }
    }

    /// The page's `data`, over UDP while that route is on, else over HTTPS.
    async fn page_on_server(&self, flow: &str, request: &PageFetch) -> Result<Value, ServerError> {
        if self.shared.active(Route::Udp) {
            match self.udp_page(flow, request).await {
                Ok(data) => {
                    self.shared.used(Route::Udp);
                    return Ok(data);
                }
                // The link worked; the session ran out (the caller signs in again).
                Err(reason) if reason.ends_with(" session_expired") => {
                    self.shared.used(Route::Udp);
                    return Err(ServerError::Expired);
                }
                Err(reason) => self.shared.failed(Route::Udp, &reason),
            }
        }
        let body = serde_json::to_value(request).unwrap_or_default();
        let answer = self.engine_call(flow, "vtop:fetch", body).await?;
        self.shared.used(Route::Server);
        page_data(&request.name, answer)
            .ok_or_else(|| ServerError::Failed(format!("unreadable packed {}", request.name)))
    }

    /// An engine action as the student; it proves who they are first only with them watching.
    async fn engine_call(&self, flow: &str, path: &str, body: Value) -> Result<Value, ServerError> {
        let ask = consent::attended(&self.shared, flow);
        // Type-erased: with a proof inside, the page fetch's future is too deep for the
        // compiler's `Send` check otherwise.
        let answer: futures::future::BoxFuture<'_, _> =
            Box::pin(self.account_call(flow, Kind::Action, path, path, ask, body));
        match answer.await {
            Ok(answer) => Ok(answer?),
            Err(error) => Err(ServerError::Failed(format!("no identity: {error}"))),
        }
    }

    /// Signs in on the server with the saved password, as [`Core::relogin`] does on the phone,
    /// and resumes its session here (cookies in the shared store). An OTP is asked for on the
    /// phone and sent back through the server ([`Core::server_otp`]).
    async fn server_relogin(
        &self,
        slot: &Arc<Slot>,
        seen: u64,
        flow: &str,
    ) -> Result<(), ServerError> {
        let mut session = slot.write().await;
        if session.logins != seen {
            return Ok(());
        }
        if !self.is_current(slot) || session.otp.is_some() || session.account.rejected {
            return Err(ServerError::Expired); // The phone's path says why.
        }
        info!("session expired, signing in on the server");
        let body = json!({
            "username": session.account.username,
            "password": session.account.password,
        });
        let login = self.signed_call(flow, Kind::Action, "vtop:login", &body);
        let answer = match login.await {
            Err(EdgeError::Status { code, .. }) if code == "invalid_credentials" => {
                // As on the phone: no more sign-ins with it (the phone's path then says so).
                warn!("VTOP rejected the saved password on the server; no more sign-ins with it");
                session.account.rejected = true;
                self.write_account(&session.account);
                return Err(ServerError::Expired);
            }
            answer => answer?,
        };
        let state: SessionState =
            serde_json::from_value(answer.get("session").cloned().unwrap_or_default())
                .map_err(|_| ServerError::Failed("unexpected sign-in answer".into()))?;
        session
            .client
            .resume_session(&state)
            .map_err(|error| ServerError::Failed(error.to_string()))?;
        if answer
            .get("status")
            .is_some_and(|status| status == "otp_required")
        {
            let message = answer
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Enter the OTP VTOP emailed you.");
            session.otp = Some(message.to_string());
            session.server_otp = true;
            info!("server sign-in waits for the otp");
            return Err(ServerError::Expired);
        }
        session.logins += 1;
        self.save(slot, &session);
        info!("signed in on the server");
        Ok(())
    }

    /// Sends the OTP (or asks for a new one) for a sign-in the server started.
    pub(crate) async fn server_otp(
        &self,
        slot: &Arc<Slot>,
        code: Option<&str>,
        flow: &str,
    ) -> Result<(), CoreError> {
        if !self
            .shared
            .consented(flow, "Send the OTP through the server")
            .await
        {
            return Err(CoreError::ConsentDenied);
        }
        let mut session = slot.write().await;
        #[expect(
            clippy::disallowed_methods,
            reason = "consent::allowed (consented) said yes above"
        )]
        let mut body = json!({ "session": session.client.session_state() });
        let path = match code {
            Some(code) => {
                crate::json_set(&mut body, "otp", json!(code));
                "vtop:loginOtp"
            }
            None => "vtop:loginResend",
        };
        let answer = self
            .signed_call(flow, Kind::Action, path, &body)
            .await
            .map_err(|error| match error {
                EdgeError::Status { code, message } if code == "authentication_failed" => {
                    CoreError::OtpRejected { reason: message }
                }
                error => error.into(),
            })?;
        let state: SessionState =
            serde_json::from_value(answer.get("session").cloned().unwrap_or_default())
                .map_err(|_| CoreError::from(VtopError::InvalidResponse))?;
        session.client.resume_session(&state)?;
        if code.is_some() && answer.get("status").is_some_and(|status| status == "ok") {
            session.otp = None;
            session.server_otp = false;
            session.logins += 1;
            self.save(slot, &session);
        }
        Ok(())
    }
}

/// A page the engine packed with the page dictionary, as its `data`; both server routes
/// (the socket and UDP) get pages this way.
pub(crate) fn unpacked(name: &str, packed: &[u8]) -> Option<Value> {
    serde_json::from_slice(&page_dictionary::unpack(name, packed).ok()?).ok()
}

/// A `vtop:fetch*` answer's `data`: a packed page comes as bytes (an `ArrayBuffer`), anything
/// else as `{ data }`. None for packed bytes that don't unpack.
pub(crate) fn page_data(name: &str, mut answer: Value) -> Option<Value> {
    match crate::convex::value::bytes(&answer) {
        Some(packed) => unpacked(name, &packed),
        None => Some(answer.get_mut("data").map(Value::take).unwrap_or_default()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Mutex;

    use serde_json::Value;

    use super::*;
    use crate::test_edge::{Edge, serve};

    /// A signed-in core on `route`, its server the test edge.
    pub(super) async fn core_on(name: &str, route: Route) -> (Core, Arc<Mutex<Edge>>) {
        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-route-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let edge = Arc::new(Mutex::new(Edge::default()));
        fs::write(
            dir.join(crate::remote::edge::SERVER_FILE),
            serve(edge.clone()).await,
        )
        .unwrap();
        let core = Core::for_test(dir);
        consent::tests::signed_in(&core);
        crate::test_edge::keep_token(&core, &core.forwarding_server());
        core.set_route("f".into(), route).unwrap();
        load(&core);
        (core, edge)
    }

    /// Page `attendance` on `route` with the student saying no.
    async fn denied(name: &str, route: Route) -> (Core, Arc<Mutex<Edge>>) {
        let (core, edge) = core_on(name, route).await;
        let gate = consent::tests::no(&core);
        let page = transport::with_flow(
            "f".into(),
            core.server_page::<Value>("attendance", Some("AP1"), &[]),
        )
        .await;
        assert!(page.is_none(), "the phone fetches");
        assert_eq!(gate.asks(), 1);
        (core, edge)
    }

    #[tokio::test]
    async fn server_route_falls_back_to_direct_when_denied() {
        let (_core, edge) = denied("server", Route::Server).await;
        assert!(edge.lock().unwrap().requests.is_empty());
    }

    #[tokio::test]
    async fn denied_route_is_not_skipped() {
        let (core, _edge) = denied("skip", Route::Server).await;
        let status = core.route_status("f".into());
        assert_eq!(status.chosen, Route::Server);
        assert!(status.skipped.is_empty());
        assert!(core.shared.active(Route::Server));
    }

    #[tokio::test]
    async fn udp_never_sends_the_session_when_denied() {
        // No `/v1/udp` for the gateway either.
        let (_core, edge) = denied("udp", Route::Udp).await;
        assert!(edge.lock().unwrap().requests.is_empty());
    }

    #[tokio::test]
    async fn relogin_password_never_leaves_when_denied() {
        let (_core, edge) = denied("relogin", Route::Server).await;
        let edge = edge.lock().unwrap();
        assert!(!edge.requests.iter().any(|r| r.path == "/v1/vtop/login"));
    }

    #[tokio::test]
    async fn otp_pending_sends_nothing_and_cleared_otp_still_asks() {
        let (core, edge) = core_on("otp-race", Route::Server).await;
        let gate = consent::tests::no(&core);
        let slot = core.current().unwrap();
        slot.write().await.otp = Some("code".into());
        let fetch = || {
            transport::with_flow(
                "f".into(),
                core.server_page::<Value>("attendance", Some("AP1"), &[]),
            )
        };
        assert!(fetch().await.is_none());
        assert_eq!(gate.asks(), 0, "nothing to send while the code is pending");
        slot.write().await.otp = None; // Another flow's code went in.
        assert!(fetch().await.is_none());
        assert_eq!(gate.asks(), 1, "the send after it is asked about");
        assert!(edge.lock().unwrap().requests.is_empty());
    }

    #[tokio::test]
    async fn server_otp_denied_is_consent_denied() {
        let (core, edge) = core_on("otp", Route::Server).await;
        let gate = consent::tests::no(&core);
        let slot = core.current().unwrap();
        let sent = core.server_otp(&slot, Some("123456"), "f").await;
        assert_eq!(sent, Err(CoreError::ConsentDenied));
        assert_eq!(gate.asks(), 1);
        assert!(edge.lock().unwrap().requests.is_empty());
    }
}

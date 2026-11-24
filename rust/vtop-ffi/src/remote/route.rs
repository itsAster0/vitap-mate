//! How the app reaches VTOP (Settings → Connection):
//! - Direct: the phone signs in and fetches from VTOP itself.
//! - Proxy: the same requests through the proxy Worker (`proxy/`, compressed), whose URL the
//!   server hands out (`proxy:info`); [`Routed`] swaps the origin per hop, so cookies, redirects
//!   and the parsers never see the proxy.
//! - Server: the engine fetches and parses read pages on the phone's session (`vtop:fetch`)
//!   and signs in with the saved password when it expired (`vtop:login`); writes, downloads and
//!   the web view stay on the phone.
//! - UDP: the server route over vm1 ([`Core::udp_page`]), then over the server's socket.
//!
//! A route that fails is skipped for [`plan::SKIP_S`] and the request goes on to the next, finally
//! Direct; the student never has to change the setting to get their data. Proxy, Server and UDP
//! send the VTOP session, so each asks first ([`consent::allowed`]); a no goes Direct and is not
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
/// The proxy's URL as the server last gave it ([`ProxyUrl`]).
const PROXY_FILE: &str = "proxy.json";
/// How long the proxy's URL is used before the server is asked again.
const PROXY_KEEP_SECS: i64 = 6 * 3600;

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
    Proxy,
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

/// The proxy Worker's origin from `proxy:info`, for the server that gave it.
#[derive(Clone, Serialize, Deserialize)]
struct ProxyUrl {
    server: String,
    url: String,
    /// Unix seconds.
    fetched: i64,
}

/// The core's route state ([`Shared::routes`]).
pub(crate) struct State {
    plan: RoutePlan,
    /// The server, for the consent dialog.
    server: String,
    proxy: Option<ProxyUrl>,
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
            proxy: None,
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
        let server = self.routes().server.clone();
        self.consented_to(flow, action, &server).await
    }

    /// [`Self::consented`] for `url`'s host (the proxy's).
    async fn consented_to(&self, flow: &str, action: &str, url: &str) -> bool {
        consent::allowed(
            self,
            flow,
            CookieKind::Route,
            action,
            &consent::host(url),
            true,
        )
        .await
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
    let proxy = core
        .read::<ProxyUrl>(PROXY_FILE)
        .filter(|proxy| proxy.server == server);
    let mut state = core.shared.routes();
    (state.plan.chosen, state.server, state.proxy) = (chosen, server, proxy);
}

/// Back to Direct with nothing skipped (signed out: the next student picks their own).
pub(crate) fn forget(shared: &Shared) {
    shared.routes().plan.forget();
}

impl Core {
    /// With the Proxy route chosen, the proxy's URL from the server when none is kept or it is
    /// [`PROXY_KEEP_SECS`] old; a server without one leaves the route to fail over to Direct.
    async fn ensure_proxy(&self, flow: &str) {
        let now = unix_now();
        {
            let state = self.shared.routes();
            if state.plan.chosen != Route::Proxy
                || state
                    .proxy
                    .as_ref()
                    .is_some_and(|proxy| now.saturating_sub(proxy.fetched) < PROXY_KEEP_SECS)
            {
                return;
            }
        }
        let server = self.forwarding_server();
        match self
            .public_call(flow, Kind::Query, "proxy:info", json!({}))
            .await
        {
            Ok(info) => {
                let Some(url) = info.get("url").and_then(Value::as_str) else {
                    warn!("proxy:info: unexpected answer");
                    return;
                };
                let proxy = ProxyUrl {
                    server,
                    url: url.trim_end_matches('/').to_string(),
                    fetched: now,
                };
                info!(host = consent::host(&proxy.url), "proxy url");
                self.write_plain(PROXY_FILE, &proxy);
                self.shared.routes().proxy = Some(proxy);
            }
            Err(error) if error.code() == "proxy_unavailable" => {
                self.shared.failed(Route::Proxy, "the server has no proxy");
            }
            // A URL kept from before stays in use until the server can be asked.
            Err(error) => info!(code = error.code(), "proxy url not fetched"),
        }
    }
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

/// Sends VTOP's hops through the proxy while that route is on, else (or when the proxy fails)
/// straight to VTOP; downloads always go straight to VTOP. Holds its core's [`Shared`], for the
/// plan, the proxy's URL and consent.
pub(crate) struct Routed(pub(crate) Arc<dyn Transport>, pub(crate) Arc<Shared>);

impl std::fmt::Debug for Routed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_tuple("Routed").field(&self.0).finish()
    }
}

/// `hop` again; [`Hop`] isn't `Clone`.
fn copy(hop: &Hop) -> Hop {
    Hop {
        method: hop.method.clone(),
        url: hop.url.clone(),
        headers: hop.headers.clone(),
        body: hop.body.clone(),
        timeout: hop.timeout,
        connect_timeout: hop.connect_timeout,
        download_to: hop.download_to.clone(),
    }
}

/// `value` with origin `from` swapped for `to`.
fn reorigin(value: &str, from: &str, to: &str) -> String {
    match value.strip_prefix(from) {
        Some(rest) => format!("{to}{rest}"),
        None => value.to_string(),
    }
}

impl Routed {
    /// `hop` through the proxy at `proxy`; None when the proxy itself failed (then Direct).
    async fn proxied(
        &self,
        hop: &Hop,
        proxy: &str,
    ) -> Option<Result<vtop_core::client::HopResponse, vtop_core::client::TransportError>> {
        let mut proxied = copy(hop);
        proxied.url = reorigin(&hop.url, DEFAULT_BASE_URL, proxy);
        for (name, value) in &mut proxied.headers {
            if name.eq_ignore_ascii_case("origin") || name.eq_ignore_ascii_case("referer") {
                *value = reorigin(value, DEFAULT_BASE_URL, proxy);
            }
        }
        match self.0.send(proxied).await {
            // The proxy's own errors are JSON; VTOP's are HTML.
            Ok(response)
                if matches!(response.status, 502..=504)
                    && response
                        .header("content-type")
                        .is_some_and(|kind| kind.contains("json")) =>
            {
                self.1
                    .failed(Route::Proxy, &format!("proxy answered {}", response.status));
                None
            }
            Ok(mut response) => {
                response.url = reorigin(&response.url, proxy, DEFAULT_BASE_URL);
                for (name, value) in &mut response.headers {
                    if name.eq_ignore_ascii_case("location") {
                        *value = reorigin(value, proxy, DEFAULT_BASE_URL);
                    }
                }
                self.1.used(Route::Proxy);
                Some(Ok(response))
            }
            Err(error) => {
                self.1.failed(Route::Proxy, &error.to_string());
                None
            }
        }
    }
}

impl Transport for Routed {
    fn send(&self, hop: Hop) -> HopFuture<'_> {
        Box::pin(async move {
            if !hop.url.starts_with(&format!("{DEFAULT_BASE_URL}/")) {
                return self.0.send(hop).await;
            }
            // Downloads go straight to VTOP, as Android's DownloadManager does.
            let download = hop.method == "HEAD" || hop.download_to.is_some();
            if !download && self.1.active(Route::Proxy) {
                let proxy = self
                    .1
                    .routes()
                    .proxy
                    .as_ref()
                    .map(|proxy| proxy.url.clone());
                match proxy {
                    // The dialog may be waited on inside the caller's session lock (hops run
                    // under it), for at most `CookieAsk`; Kotlin's `ask` calls no Rust, so it
                    // can't deadlock.
                    Some(proxy)
                        if (self.1)
                            .consented_to(
                                &transport::flow(),
                                "Reach VTOP through the proxy",
                                &proxy,
                            )
                            .await =>
                    {
                        if let Some(response) = self.proxied(&hop, &proxy).await {
                            return response;
                        }
                    }
                    // Not asked yet (the next page fetch asks), or the server has none
                    // (`ensure_proxy` skips the route then): straight to VTOP meanwhile.
                    Some(_) | None => {}
                }
            }
            let response = self.0.send(hop).await;
            if response.is_ok() {
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
        self.ensure_proxy(&flow).await;
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
        let mut next = ServerFetch::start(slot.read().await.is_awaiting_otp());
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
                        if session.is_awaiting_otp() {
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
        if !self.is_current(slot) || session.is_awaiting_otp() || session.is_password_rejected() {
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
                // As on the phone: clear cookies and stop signing in with it.
                warn!("VTOP rejected the saved password on the server; no more sign-ins with it");
                session.mark_password_rejected();
                if self.is_current(slot) {
                    self.write_account(&session.account);
                }
                return Err(ServerError::Expired);
            }
            answer => answer?,
        };
        if !self.is_current(slot) {
            return Err(ServerError::Expired);
        }
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
            session.begin_otp(crate::OtpVia::Server, message.to_string());
            info!("server sign-in waits for the otp");
            return Err(ServerError::Expired);
        }
        session.mark_signed_in();
        self.save(slot, &session);
        info!("signed in on the server");
        Ok(())
    }

    /// Sends the OTP (or asks for a new one) for a sign-in the server started.
    pub(crate) async fn server_otp(
        &self,
        slot: &Arc<Slot>,
        challenge_id: u64,
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
        if !self.is_current(slot) {
            return Err(CoreError::NotSignedIn);
        }
        if session.server_otp_challenge() != Some(challenge_id) {
            if code.is_some() && !session.is_awaiting_otp() && session.client.is_authenticated() {
                info!("server otp no longer needed: signed in");
                return Ok(());
            }
            return Err(CoreError::OtpRejected {
                reason: "That sign-in had ended. VTOP sent a new code; use the newest email."
                    .into(),
            });
        }
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
        if !self.is_current(slot) {
            return Err(CoreError::NotSignedIn);
        }
        let state: SessionState =
            serde_json::from_value(answer.get("session").cloned().unwrap_or_default())
                .map_err(|_| CoreError::from(VtopError::InvalidResponse))?;
        session.client.resume_session(&state)?;
        if code.is_some() && answer.get("status").is_some_and(|status| status == "ok") {
            session.mark_signed_in();
            crate::obs::set_regno(session.client.registration_number().ok());
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
    use vtop_core::client::HopResponse;

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
        // A server re-login is the `vtop:login` action; denied, nothing goes at all.
        let edge = edge.lock().unwrap();
        assert!(!edge.requests.iter().any(|r| r.path == "vtop:login"));
        assert!(edge.requests.is_empty());
    }

    #[tokio::test]
    async fn otp_pending_sends_nothing_and_cleared_otp_still_asks() {
        let (core, edge) = core_on("otp-race", Route::Server).await;
        let gate = consent::tests::no(&core);
        let slot = core.current().unwrap();
        slot.write()
            .await
            .begin_otp(crate::OtpVia::Phone, "code".into());
        let fetch = || {
            transport::with_flow(
                "f".into(),
                core.server_page::<Value>("attendance", Some("AP1"), &[]),
            )
        };
        assert!(fetch().await.is_none());
        assert_eq!(gate.asks(), 0, "nothing to send while the code is pending");
        slot.write().await.mark_signed_in(); // Another flow's code went in.
        assert!(fetch().await.is_none());
        assert_eq!(gate.asks(), 1, "the send after it is asked about");
        assert!(edge.lock().unwrap().requests.is_empty());
    }

    #[tokio::test]
    async fn server_otp_denied_is_consent_denied() {
        let (core, edge) = core_on("otp", Route::Server).await;
        let gate = consent::tests::no(&core);
        let slot = core.current().unwrap();
        let challenge_id = slot
            .write()
            .await
            .begin_otp(crate::OtpVia::Server, "code".into());
        let sent = core
            .server_otp(&slot, challenge_id, Some("123456"), "f")
            .await;
        assert_eq!(sent, Err(CoreError::ConsentDenied));
        assert_eq!(gate.asks(), 1);
        assert!(edge.lock().unwrap().requests.is_empty());
    }

    #[tokio::test]
    async fn server_otp_rejects_stale_challenge_and_udp_adopt_clears_otp() {
        let (core, edge) = core_on("otp-stale", Route::Server).await;
        let _gate = consent::tests::yes(&core);
        let slot = core.current().unwrap();
        let first = slot
            .write()
            .await
            .begin_otp(crate::OtpVia::Server, "first".into());
        // A newer challenge replaces the first before the stale submit lands.
        let _second = slot
            .write()
            .await
            .begin_otp(crate::OtpVia::Server, "second".into());
        let sent = core.server_otp(&slot, first, Some("123456"), "f").await;
        assert!(
            matches!(sent, Err(CoreError::OtpRejected { .. })),
            "{sent:?}"
        );
        assert!(edge.lock().unwrap().requests.is_empty());

        // Adopting a UDP-renewed session clears the pending OTP phase.
        let renewed = serde_json::to_string(&SessionState {
            cookies: "JSESSIONID=fresh".into(),
            csrf_token: Some("csrf-fresh".into()),
            registration_number: Some("22BCE0001".into()),
            otp_issued_at: None,
            logged_in_at: Some(unix_now() as u64),
        })
        .unwrap();
        core.adopt_renewed(&renewed, Some(&slot)).await;
        let session = slot.read().await;
        assert!(!session.is_awaiting_otp());
        assert_eq!(session.logins, 1);
    }

    /// Answers every hop with 200, or the proxy's JSON 502 when it is down, and keeps its URL.
    #[derive(Debug, Default)]
    struct Hops {
        urls: Mutex<Vec<String>>,
        /// The proxy's own failure for hops to it.
        proxy_down: bool,
    }

    impl Transport for Hops {
        fn send(&self, hop: Hop) -> HopFuture<'_> {
            self.urls.lock().unwrap().push(hop.url.clone());
            let down = self.proxy_down && hop.url.starts_with(PROXY);
            Box::pin(async move {
                Ok(if down {
                    let json = vec![("content-type".into(), "application/json".into())];
                    HopResponse::whole(502, json, hop.url, b"{}".to_vec())
                } else {
                    HopResponse::whole(200, Vec::new(), hop.url, Vec::new())
                })
            })
        }
    }

    const PROXY: &str = "https://proxy.test";

    /// Three VTOP hops in one flow on the Proxy route; the URLs they went to. With `yes`, the
    /// student says yes; `proxy` is the URL the server gave; the core is returned for its state.
    async fn three_hops(yes: bool, proxy: Option<&str>, proxy_down: bool) -> (Vec<String>, Core) {
        let core = Core::for_test(std::env::temp_dir());
        let gate = consent::tests::install(&core, true, !yes);
        {
            let mut state = core.shared.routes();
            state.plan.choose(Route::Proxy);
            state.server = "https://edge.test".into();
            state.proxy = proxy.map(|url| ProxyUrl {
                server: "https://edge.test".into(),
                url: url.into(),
                fetched: unix_now(),
            });
        }
        let hops = Arc::new(Hops {
            proxy_down,
            ..Hops::default()
        });
        let routed = Routed(hops.clone(), core.shared.clone());
        transport::with_flow("f".into(), async {
            for _ in 0..3 {
                let hop = Hop {
                    method: "GET".into(),
                    url: format!("{DEFAULT_BASE_URL}/vtop/content"),
                    headers: Vec::new(),
                    body: Vec::new(),
                    timeout: std::time::Duration::from_secs(1),
                    connect_timeout: std::time::Duration::from_secs(1),
                    download_to: None,
                };
                let response = routed.send(hop).await.unwrap();
                assert!(
                    response.url.starts_with(DEFAULT_BASE_URL),
                    "{}",
                    response.url
                );
            }
        })
        .await;
        if proxy.is_some() {
            assert_eq!(gate.asks(), 1);
        }
        let urls = hops.urls.lock().unwrap().clone();
        (urls, core)
    }

    fn skipped(core: &Core) -> Vec<Route> {
        let status = core.shared.routes().plan.status(unix_now());
        status.skipped.iter().map(|skip| skip.route).collect()
    }

    #[tokio::test]
    async fn proxy_asks_once_per_flow() {
        let (urls, _core) = three_hops(true, Some(PROXY), false).await;
        assert!(
            urls.iter()
                .all(|url| url.starts_with(&format!("{PROXY}/vtop/")))
        );
    }

    #[tokio::test]
    async fn proxy_denied_goes_straight_to_vtop() {
        let (urls, core) = three_hops(false, Some(PROXY), false).await;
        assert!(urls.iter().all(|url| url.starts_with(DEFAULT_BASE_URL)));
        assert!(skipped(&core).is_empty());
    }

    #[tokio::test]
    async fn a_failed_proxy_is_skipped_for_direct() {
        let (urls, core) = three_hops(true, Some(PROXY), true).await;
        // One try at the proxy, then every hop straight to VTOP.
        assert_eq!(urls.iter().filter(|url| url.starts_with(PROXY)).count(), 1);
        assert_eq!(urls.len(), 4);
        assert_eq!(skipped(&core), [Route::Proxy]);
    }

    #[tokio::test]
    async fn no_proxy_from_the_server_goes_direct() {
        let (urls, core) = three_hops(true, None, false).await;
        assert!(urls.iter().all(|url| url.starts_with(DEFAULT_BASE_URL)));
        // Not skipped: the proxy may just not have been asked for yet.
        assert!(skipped(&core).is_empty());
    }

    #[test]
    fn origins_swap_only_at_the_start() {
        assert_eq!(
            reorigin(
                "https://vtop.vitap.ac.in/vtop/login",
                DEFAULT_BASE_URL,
                PROXY
            ),
            "https://proxy.test/vtop/login"
        );
        assert_eq!(
            reorigin("/vtop/content", PROXY, DEFAULT_BASE_URL),
            "/vtop/content"
        );
    }
}

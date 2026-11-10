//! How the app reaches VTOP (Settings → Connection):
//! - Direct: the phone signs in and fetches from VTOP itself.
//! - Proxy: the same requests through the edge's `/vtop/` (compressed); [`Routed`] swaps the
//!   origin per hop, so cookies, redirects and the parsers never see the edge.
//! - Server: the engine fetches and parses read pages on the phone's session
//!   (`/v1/vtop/fetch`) and signs in with the saved password when it expired
//!   (`/v1/vtop/login`); writes, downloads and the web view stay on the phone.
//! - UDP: the server route over vm1 ([`Core::udp_page`]), then over HTTPS.
//!
//! A route that fails is skipped for [`SKIP_S`] and the request goes on to the next, finally
//! Direct; the student never has to change the setting to get their data.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

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
use crate::{Core, CoreError, Slot, traced_sync, transport, unix_now};

const FILE: &str = "route.json";
/// How long a failed route is skipped before it is tried again.
const SKIP_S: i64 = 5 * 60;

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

/// Global, as the transport is.
struct State {
    chosen: Route,
    /// The edge, for the proxy's origin.
    server: String,
    skipped: BTreeMap<Route, (i64, String)>,
    last: Option<(Route, i64)>,
    /// The span and route of the last "route used" line: one line per call, not per hop.
    logged: Option<(Option<u64>, Route)>,
}

static STATE: Mutex<State> = Mutex::new(State {
    chosen: Route::Direct,
    server: String::new(),
    skipped: BTreeMap::new(),
    last: None,
    logged: None,
});

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Whether `route` should be tried now: the chosen one covers it (UDP falls back to the HTTPS
/// server route) and it isn't skipped.
fn active(route: Route) -> bool {
    let state = state();
    let chosen = state.chosen == route || (state.chosen == Route::Udp && route == Route::Server);
    chosen
        && state
            .skipped
            .get(&route)
            .is_none_or(|(until, _)| *until <= unix_now())
}

fn failed(route: Route, reason: &str) {
    warn!(?route, reason, "route failed; skipped for 5 min");
    state()
        .skipped
        .insert(route, (unix_now() + SKIP_S, reason.to_string()));
}

fn used(route: Route) {
    let mut state = state();
    state.skipped.remove(&route);
    state.last = Some((route, unix_now()));
    let span = tracing::Span::current().id().map(|id| id.into_u64());
    if state.logged != Some((span, route)) {
        state.logged = Some((span, route));
        info!(?route, "route used");
    }
}

/// Loads the saved choice; called when the core opens.
pub(crate) fn load(core: &Core) {
    let chosen = core.read::<Saved>(FILE).unwrap_or_default().route;
    let server = core.forwarding_server();
    info!(?chosen, "route");
    let mut state = state();
    (state.chosen, state.server) = (chosen, server);
}

#[uniffi::export]
impl Core {
    pub fn route_status(&self, flow: String) -> RouteStatus {
        let _ = traced_sync("route_status", &flow, || Ok(()));
        let state = state();
        let now = unix_now();
        let mut skipped: Vec<SkippedRoute> = state
            .skipped
            .iter()
            .filter(|(_, (until, _))| *until > now)
            .map(|(route, (until, reason))| SkippedRoute {
                route: *route,
                until: *until,
                reason: reason.clone(),
            })
            .collect();
        skipped.sort_by_key(|skip| skip.until);
        RouteStatus {
            chosen: state.chosen,
            last_used: state.last.map(|(route, _)| route),
            last_used_at: state.last.map(|(_, at)| at),
            skipped,
        }
    }

    /// Saves the choice; earlier failures are forgotten so it is tried straight away.
    pub fn set_route(&self, flow: String, route: Route) -> Result<(), CoreError> {
        traced_sync("set_route", &flow, || {
            self.write(FILE, &Saved { route });
            let mut state = state();
            state.chosen = route;
            state.skipped.clear();
            info!(?route, "route chosen");
            Ok(())
        })
    }
}

/// Sends VTOP's hops through the edge's `/vtop/` while the proxy route is on, else (or when the
/// edge fails) straight to VTOP.
#[derive(Debug)]
pub(crate) struct Routed(pub(crate) Arc<dyn Transport>);

impl Routed {
    fn is_vtop(hop: &Hop) -> bool {
        hop.url.starts_with(&format!("{DEFAULT_BASE_URL}/"))
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

impl Transport for Routed {
    fn send(&self, hop: Hop) -> HopFuture<'_> {
        Box::pin(async move {
            if !Self::is_vtop(&hop) {
                return self.0.send(hop).await;
            }
            if active(Route::Proxy) {
                let edge = state().server.trim_end_matches('/').to_string();
                let mut proxied = copy(&hop);
                proxied.url = reorigin(&hop.url, DEFAULT_BASE_URL, &edge);
                for (name, value) in &mut proxied.headers {
                    if name.eq_ignore_ascii_case("origin") || name.eq_ignore_ascii_case("referer") {
                        *value = reorigin(value, DEFAULT_BASE_URL, &edge);
                    }
                }
                match self.0.send(proxied).await {
                    // The edge's own errors are JSON; VTOP's are HTML.
                    Ok(response)
                        if matches!(response.status, 502..=504)
                            && response
                                .header("content-type")
                                .is_some_and(|kind| kind.contains("json")) =>
                    {
                        failed(Route::Proxy, &format!("edge answered {}", response.status));
                    }
                    Ok(mut response) => {
                        response.url = reorigin(&response.url, &edge, DEFAULT_BASE_URL);
                        for (name, value) in &mut response.headers {
                            if name.eq_ignore_ascii_case("location") {
                                *value = reorigin(value, &edge, DEFAULT_BASE_URL);
                            }
                        }
                        used(Route::Proxy);
                        return Ok(response);
                    }
                    Err(error) => failed(Route::Proxy, &error.to_string()),
                }
            }
            let response = self.0.send(hop).await;
            if response.is_ok() {
                used(Route::Direct);
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

/// Runs `$call` on the phone (`signed_in!`), or fetches page `$name` on the server when the
/// route says so and the server answers.
macro_rules! page {
    ($core:expr, $name:expr, $semester:expr, [$($key:expr => $value:expr),*], |$client:ident| $call:expr) => {{
        let core = $core;
        match core.server_page($name, $semester, &[$(($key, $value)),*]).await {
            Some(result) => result,
            None => signed_in!(core, |$client| $call),
        }
    }};
}
pub(crate) use page;

/// The edge's answer as [`ServerError`]: the session ran out, or the server part failed.
enum ServerError {
    Expired,
    Failed(String),
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
    /// session expired; None when the route is off or the server failed (the phone fetches).
    pub(crate) async fn server_page<T: DeserializeOwned>(
        &self,
        name: &str,
        semester: Option<&str>,
        args: &[(&str, &str)],
    ) -> Option<Result<T, CoreError>> {
        if !active(Route::Server) && !active(Route::Udp) {
            return None;
        }
        let slot = self.current().ok()?;
        let flow = transport::flow();
        for attempt in 0..2 {
            let (session, seen) = {
                let session = slot.read().await;
                if session.otp.is_some() {
                    return None; // Waiting for a code: the phone's path tells the student.
                }
                (session.client.session_state(), session.logins)
            };
            let request = PageFetch {
                session: serde_json::to_value(&session).unwrap_or_default(),
                name: name.to_string(),
                semester: semester.map(str::to_string),
                args: args
                    .iter()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect(),
                // Built in: the engine packs the page with it (plain JSON if its copy differs).
                dictionary: Some(page_dictionary::id().to_string()),
            };
            match self.page_on_server(&flow, &request).await {
                Ok(data) => {
                    return match serde_json::from_value(data) {
                        Ok(page) if self.is_current(&slot) => Some(Ok(page)),
                        Ok(_) => Some(Err(CoreError::NotSignedIn)),
                        Err(error) => {
                            failed(Route::Server, &format!("unreadable {name}: {error}"));
                            None
                        }
                    };
                }
                Err(ServerError::Expired) if attempt == 0 => {
                    match self.server_relogin(&slot, seen, &flow).await {
                        Ok(()) => {}
                        Err(ServerError::Failed(reason)) => {
                            failed(Route::Server, &reason);
                            return None;
                        }
                        Err(ServerError::Expired) => return None,
                    }
                }
                Err(ServerError::Expired) => return None,
                Err(ServerError::Failed(reason)) => {
                    failed(Route::Server, &reason);
                    return None;
                }
            }
        }
        None
    }

    /// The page's `data`, over UDP while that route is on, else over HTTPS.
    async fn page_on_server(&self, flow: &str, request: &PageFetch) -> Result<Value, ServerError> {
        if active(Route::Udp) {
            match self.udp_page(flow, request).await {
                Ok(data) => {
                    used(Route::Udp);
                    return Ok(data);
                }
                // The link worked; the session ran out (the caller signs in again).
                Err(reason) if reason.ends_with(" session_expired") => {
                    used(Route::Udp);
                    return Err(ServerError::Expired);
                }
                Err(reason) => failed(Route::Udp, &reason),
            }
        }
        let body = serde_json::to_value(request).unwrap_or_default();
        let server = self.forwarding_server();
        let mut answer = self
            .edge_answer(&server, "/v1/vtop/fetch", flow, &body)
            .await?;
        used(Route::Server);
        match answer.packed {
            Some(packed) => unpacked(&request.name, &packed)
                .ok_or_else(|| ServerError::Failed(format!("unreadable packed {}", request.name))),
            None => Ok(answer.body["data"].take()),
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
        let server = self.forwarding_server();
        let answer = match self.edge_post(&server, "/v1/vtop/login", flow, &body).await {
            Err(EdgeError::Status { code, .. }) if code == "invalid_credentials" => {
                // As on the phone: no more sign-ins with it (the phone's path then says so).
                warn!("VTOP rejected the saved password on the server; no more sign-ins with it");
                session.account.rejected = true;
                self.write_account(&session.account);
                return Err(ServerError::Expired);
            }
            answer => answer?,
        };
        let state: SessionState = serde_json::from_value(answer["session"].clone())
            .map_err(|_| ServerError::Failed("unexpected sign-in answer".into()))?;
        session
            .client
            .resume_session(&state)
            .map_err(|error| ServerError::Failed(error.to_string()))?;
        if answer["status"] == "otp_required" {
            let message = answer["message"]
                .as_str()
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
        let mut session = slot.write().await;
        let mut body = json!({ "session": session.client.session_state() });
        let path = match code {
            Some(code) => {
                body["otp"] = json!(code);
                "/v1/vtop/login/otp"
            }
            None => "/v1/vtop/login/resend",
        };
        let server = self.forwarding_server();
        let answer =
            self.edge_post(&server, path, flow, &body)
                .await
                .map_err(|error| match error {
                    EdgeError::Status { code, message } if code == "authentication_failed" => {
                        CoreError::OtpRejected { reason: message }
                    }
                    error => error.into(),
                })?;
        let state: SessionState = serde_json::from_value(answer["session"].clone())
            .map_err(|_| CoreError::from(VtopError::InvalidResponse))?;
        session.client.resume_session(&state)?;
        if code.is_some() && answer["status"] == "ok" {
            session.otp = None;
            session.server_otp = false;
            session.logins += 1;
            self.save(slot, &session);
        }
        Ok(())
    }
}

/// A page the engine packed with the page dictionary, as its `data`; both server routes
/// (HTTPS and UDP) get pages this way.
pub(crate) fn unpacked(name: &str, packed: &[u8]) -> Option<Value> {
    serde_json::from_slice(&page_dictionary::unpack(name, packed).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_swap_only_at_the_start() {
        let edge = "https://vmate.aster0.dev";
        assert_eq!(
            reorigin(
                "https://vtop.vitap.ac.in/vtop/login",
                DEFAULT_BASE_URL,
                edge
            ),
            "https://vmate.aster0.dev/vtop/login"
        );
        assert_eq!(
            reorigin("/vtop/content", edge, DEFAULT_BASE_URL),
            "/vtop/content"
        );
    }
}

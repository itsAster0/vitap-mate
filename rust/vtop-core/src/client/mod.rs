//! The VTOP client. Login lives in [`auth`], the security OTP in [`otp`],
//! CAPTCHA solving in `vtop_proto::captcha`, and one module per group of endpoints
//! holds the fetchers.
//!
//! Fetchers take `&self`, so one signed-in client can serve several requests
//! at once. Login and OTP take `&mut self` because they drive a multi-step
//! flow that must not interleave with itself.

mod academics;
mod auth;
mod biometric;
mod course_page;
mod email_otp;
mod exams;
mod hostel;
mod http;
mod otp;
mod profile;
mod transport;

use std::sync::{Arc, Mutex, MutexGuard};

use log::{error, info, warn};

use crate::config::VtopConfig;
use crate::error::{VtopError, VtopResult};
use crate::inputs::{Password, RegistrationNumber, Username};
use crate::session::{
    parse_cookie_pairs, CookieStore, ResettableJar, SessionManager, SessionState,
};
use crate::types::PersistedVtopSession;
use vtop_proto::http::{join_url, url_parts, Method, Request};
use vtop_proto::machine::{Machine, OnePost, Signed, Step};

pub use course_page::{CourseFile, CourseFileInfo, FileLink};
#[cfg(feature = "reqwest")]
pub use email_otp::EmailOtpLogin;
pub use hostel::{
    check_general_outing, check_weekend_outing, GeneralOutingApplication, OutingKind,
    WeekendOutingApplication,
};
pub use http::VITAP_INTERMEDIATE_CA_PEM;
pub use transport::{Hop, HopFuture, HopResponse, Transport, TransportError};

const AUTH: &str = "rust.auth";
const NETWORK: &str = "rust.network";

/// Redirects followed per request, like a browser's limit.
const MAX_REDIRECTS: usize = 10;

/// Pause before the single retry of a failed VTOP request.
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(400);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthStage {
    Idle,
    LoggingIn,
    AwaitingOtp { issued_at_unix_seconds: u64 },
}

#[derive(Debug, Clone)]
struct Credentials {
    username: Username,
    password: Password,
}

/// Mutable session data. Held behind a mutex that is never kept across an
/// `.await`, so fetchers can share one client.
#[derive(Debug)]
struct ClientState {
    session: SessionManager,
    auth_stage: AuthStage,
    /// Unix seconds of the last real password (+ OTP) login. Restoring or
    /// validating a session does not change it.
    logged_in_at: Option<u64>,
}

/// The CSRF token and registration number every authenticated POST needs.
struct AuthorizedRequest {
    csrf: String,
    registration_number: RegistrationNumber,
}

pub struct VtopClient {
    transport: Arc<dyn Transport>,
    config: Arc<VtopConfig>,
    jar: Arc<dyn CookieStore>,
    credentials: Option<Credentials>,
    state: Mutex<ClientState>,
    /// Serialises session validation so concurrent fetches on a restored
    /// session hit `/vtop/content` once, not once each.
    validation: crate::sync::RwLock<()>,
    auth_flow_id: u64,
}

impl std::fmt::Debug for VtopClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VtopClient")
            .field("base_url", &self.config.base_url)
            .field("authenticated", &self.is_authenticated())
            .finish_non_exhaustive()
    }
}

/// Builds a [`VtopClient`] for credentials, a saved session, or both.
#[derive(Debug, Default)]
pub struct VtopClientBuilder {
    config: VtopConfig,
    transport: Option<Arc<dyn Transport>>,
    cookies: Option<Arc<dyn CookieStore>>,
}

impl VtopClientBuilder {
    pub fn config(mut self, config: VtopConfig) -> Self {
        self.config = config;
        self
    }

    /// Sends every hop through `transport` instead of the default one.
    pub fn transport(mut self, transport: Arc<dyn Transport>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Keeps the session's cookies in `store` instead of a jar of its own.
    pub fn cookie_store(mut self, store: Arc<dyn CookieStore>) -> Self {
        self.cookies = Some(store);
        self
    }

    fn build(self, credentials: Option<Credentials>) -> VtopResult<VtopClient> {
        let jar = self
            .cookies
            .unwrap_or_else(|| Arc::new(ResettableJar::default()));
        let transport = match self.transport {
            Some(transport) => transport,
            None => default_transport(&self.config)?,
        };
        Ok(VtopClient {
            transport,
            config: Arc::new(self.config),
            jar,
            credentials,
            state: Mutex::new(ClientState {
                session: SessionManager::new(),
                auth_stage: AuthStage::Idle,
                logged_in_at: None,
            }),
            validation: crate::sync::RwLock::default(),
            auth_flow_id: 0,
        })
    }

    /// A client that can log in. The username is upper-cased, as VTOP expects.
    pub fn with_credentials(
        self,
        username: Username,
        password: Password,
    ) -> VtopResult<VtopClient> {
        self.build(Some(Credentials {
            username: username.to_uppercase(),
            password,
        }))
    }

    /// A client that resumes `state` and cannot log in by itself.
    ///
    /// A state carrying a CSRF token and registration number is trusted as
    /// signed in; if VTOP disagrees, the first fetch fails with
    /// [`VtopError::SessionExpired`]. A bare cookie is checked against VTOP
    /// on first use.
    pub fn with_session(self, state: &SessionState) -> VtopResult<VtopClient> {
        let client = self.build(None)?;
        client.resume_session(state)?;
        Ok(client)
    }
}

impl VtopClient {
    pub fn builder() -> VtopClientBuilder {
        VtopClientBuilder::default()
    }

    fn state(&self) -> MutexGuard<'_, ClientState> {
        // A panic while holding this lock cannot leave the plain data inside
        // half-updated in a way that matters, so recover instead of failing.
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn vtop_url(&self) -> VtopResult<String> {
        let url = self.config.url("/vtop");
        url_parts(&url)
            .map(|_| url.clone())
            .ok_or_else(|| VtopError::ConfigurationError(format!("invalid base URL: {url}")))
    }

    fn auth_log(&self, level: log::Level, stage: &str, message: impl AsRef<str>) {
        log::log!(
            target: AUTH,
            level,
            "auth.flow#{} {} {}",
            self.auth_flow_id,
            stage,
            message.as_ref()
        );
    }

    /// Logs a request's method and URL without its query: download URLs
    /// carry the CSRF token there.
    fn log_request(context: &str, method: &str, url: &str) {
        let url = url.split_once('?').map_or(url, |(path, _)| path);
        info!(target: NETWORK, "request {context} {method} {url}");
    }

    /// Logs which cookies are present, never their values.
    fn log_cookie_names(&self, context: &str) {
        let names = self
            .cookie_header()
            .map(|header| {
                parse_cookie_pairs(&header)
                    .into_iter()
                    .map(|(name, _)| name)
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();
        let external = self.state().session.external_cookie_header().is_some();
        info!(
            target: AUTH,
            "auth.flow#{} {context}: cookies=[{names}] external_header={external}",
            self.auth_flow_id
        );
    }

    fn reset_session_state(&self) {
        self.reset_auth_state();
        self.jar.clear();
    }

    /// [`Self::reset_session_state`] without touching the cookies.
    fn reset_auth_state(&self) {
        let mut state = self.state();
        state.session.clear();
        state.auth_stage = AuthStage::Idle;
        state.logged_in_at = None;
    }

    fn mark_session_expired(&self, context: &str, reason: &str) {
        self.reset_session_state();
        self.auth_log(
            log::Level::Warn,
            context,
            format!("session marked expired because {reason}"),
        );
    }

    /// Makes sure the session is signed in and returns what an
    /// authenticated form post needs.
    async fn authorize(&self, context: &str) -> VtopResult<AuthorizedRequest> {
        if !self.ensure_authenticated_session().await? {
            return Err(VtopError::SessionExpired);
        }
        let state = self.state();
        let csrf = state
            .session
            .csrf_token()
            .ok_or_else(|| missing_csrf_error(context))?;
        let registration_number = state
            .session
            .registration_number()
            .cloned()
            .ok_or(VtopError::RegistrationParsingError)?;
        Ok(AuthorizedRequest {
            csrf,
            registration_number,
        })
    }

    /// Sends `request` and returns the response as it came, retrying once
    /// after a short pause when the connection fails or VTOP answers with a
    /// 5xx. A timeout, or a 504 from the proxy giving up on VTOP, is not
    /// retried: VTOP is slow, and a second request only adds to its load.
    async fn send_retrying(&self, context: &str, request: &Request) -> VtopResult<HopResponse> {
        let retry = || async {
            self.transport.pause(RETRY_DELAY).await;
            self.send_request(request.clone())
                .await
                .map_err(|error| send_error(&format!("{context}.retry"), error))
        };
        match self.send_request(request.clone()).await {
            Ok(response) if response.status < 500 || response.status == 504 => Ok(response),
            Ok(response) => {
                warn!(
                    target: NETWORK,
                    "{context}: VTOP returned HTTP {}, retrying once", response.status
                );
                retry().await
            }
            Err(error) if error.is_timeout() => {
                warn!(target: NETWORK, "{context}: {error}");
                Err(slow_vtop_error(context))
            }
            Err(error) if error.is_connect() => {
                warn!(target: NETWORK, "{context}: {error}, retrying once");
                retry().await
            }
            Err(error) => Err(send_error(&format!("{context}.send"), error)),
        }
    }

    /// What a signed-in request needs, signing in first when the session
    /// is not.
    async fn signed(&self, context: &str) -> VtopResult<Signed> {
        let auth = self.authorize(context).await?;
        Ok(Signed {
            base_url: self.config.base_url.trim_end_matches('/').to_string(),
            csrf: auth.csrf,
            registration_number: auth.registration_number.to_string(),
        })
    }

    /// Signs in if needed, then runs the one page request `request` builds.
    pub(super) async fn page(
        &self,
        context: &str,
        request: impl FnOnce(&Signed) -> Request,
    ) -> VtopResult<String> {
        let signed = self.signed(context).await?;
        self.run(context, OnePost::new(request(&signed))).await
    }

    /// Runs a `vtop-proto` machine over the transport. Each request goes through
    /// [`Self::send_retrying`] (redirects followed, cookies kept), or once
    /// when the request says so; a dead session resets the session.
    async fn run<M: Machine>(&self, context: &str, mut machine: M) -> VtopResult<M::Output> {
        let mut step = machine.start();
        // The last answer's status: a VTOP error after a 5xx resets the
        // session, as it always has; one the machine found in a page does
        // not.
        let mut last_status = 0;
        loop {
            let request = match step {
                Step::Send(request) => request,
                Step::Done(Ok(output)) => return Ok(output),
                Step::Done(Err(error)) => {
                    let expired = matches!(error, VtopError::SessionExpired)
                        || (matches!(error, VtopError::VtopServerError(_)) && last_status >= 500);
                    if expired {
                        self.mark_session_expired(context, &format!("VTOP answered: {error}"));
                    } else {
                        warn!(target: NETWORK, "{context}: {error}");
                    }
                    return Err(error);
                }
            };
            let method = match request.method {
                Method::Get => "GET",
                Method::Post => "POST",
            };
            Self::log_request(&format!("{context}.send"), method, &request.url);
            let response = if request.retry {
                self.send_retrying(context, &request).await?
            } else {
                self.send_request(request)
                    .await
                    .map_err(|error| send_error(&format!("{context}.send"), error))?
            };
            last_status = response.status;
            let response = response
                .read()
                .await
                .map_err(|error| network_error(&format!("{context}.body"), error))?;
            step = machine.feed(response);
        }
    }

    /// Sends a request with this session's cookies and follows redirects,
    /// storing every `Set-Cookie` along the way. Each hop goes through the
    /// [`Transport`] (shared between sessions, one warm connection pool), so
    /// cookies and the default headers are added here. An explicit `Cookie`
    /// header on the request wins over the jar for the first hop. Logs the
    /// answer: host, path, status, size when known and time, redirects
    /// included; or the failure.
    pub(super) async fn send_request(
        &self,
        request: Request,
    ) -> Result<HopResponse, TransportError> {
        self.send_to(request, None).await
    }

    /// [`Self::send_request`], each hop's body going to the file
    /// `download_to` when the transport can write it there.
    async fn send_to(
        &self,
        request: Request,
        download_to: Option<&std::path::Path>,
    ) -> Result<HopResponse, TransportError> {
        let started = std::time::Instant::now();
        let place = |url: &str| {
            url_parts(url).map_or_else(String::new, |(_, host, path)| format!("{host}{path}"))
        };
        let mut method = match request.method {
            Method::Get => "GET",
            Method::Post => "POST",
        };
        let (mut url, mut headers, mut body) = (request.url, request.headers, request.body);
        let mut hops = 0;
        loop {
            if !headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("cookie"))
            {
                if let Some(cookies) = self.jar.header_for(&url) {
                    headers.push(("cookie".to_string(), cookies));
                }
            }
            let mut hop = self.hop(method, url.clone(), headers, body);
            hop.download_to = download_to.map(std::path::Path::to_path_buf);
            let response = match self.transport.send(hop).await {
                Ok(response) => response,
                Err(error) => {
                    warn!(
                        target: NETWORK,
                        "failed {} {}ms: {error}",
                        place(&url),
                        started.elapsed().as_millis()
                    );
                    return Err(error);
                }
            };
            let set_cookies: Vec<&str> = response
                .headers
                .iter()
                .filter(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
                .map(|(_, value)| value.as_str())
                .collect();
            if !set_cookies.is_empty() {
                self.jar.set_cookies(&url, &set_cookies);
            }

            // 307/308 would need the original body again; VTOP only uses
            // 301/302/303, which continue as a GET.
            let next = (matches!(response.status, 301..=303) && hops < MAX_REDIRECTS)
                .then(|| response.header("location"))
                .flatten()
                .and_then(|location| join_url(&url, location));
            let Some(next) = next else {
                let size = response
                    .content_length()
                    .map_or(String::new(), |bytes| format!(" {bytes}B"));
                info!(
                    target: NETWORK,
                    "response {} {}{size} {}ms",
                    place(&response.url),
                    response.status,
                    started.elapsed().as_millis()
                );
                return Ok(response);
            };
            hops += 1;
            (method, url, headers, body) = ("GET", next, Vec::new(), Vec::new());
        }
    }

    /// One [`Hop`]: `headers` (names in lower case), then the default
    /// headers they lack.
    fn hop(&self, method: &str, url: String, headers: Vec<(String, String)>, body: Vec<u8>) -> Hop {
        let mut headers: Vec<(String, String)> = headers
            .into_iter()
            .map(|(name, value)| (name.to_ascii_lowercase(), value))
            .collect();
        for (name, value) in http::DEFAULT_HEADERS {
            if !headers.iter().any(|(present, _)| present == name) {
                headers.push((name.to_string(), value.to_string()));
            }
        }
        Hop {
            method: method.to_string(),
            url,
            headers,
            body,
            timeout: self.config.request_timeout,
            connect_timeout: self.config.connect_timeout,
            download_to: None,
        }
    }

    /// Sends `request` (redirects followed, no retry) with its body going
    /// to the file `to`: the transport writes it there when it can (the
    /// app's), else it streams here. Returns the head, the body's first
    /// bytes and its size; the caller checks them and removes `to` when the
    /// answer is not the file. Nothing stays at `to` when sending fails.
    pub(super) async fn download(
        &self,
        context: &str,
        request: Request,
        to: &std::path::Path,
    ) -> VtopResult<(vtop_proto::http::Response, u64)> {
        Self::log_request(context, "GET", &request.url);
        let result = async {
            let mut response = self
                .send_to(request, Some(to))
                .await
                .map_err(|error| send_error(&format!("{context}.send"), error))?;
            let mut head = response.head();
            let size = match response.saved_size() {
                Some(size) => size,
                None => {
                    use std::io::Write;
                    let io =
                        |error: std::io::Error| network_error(&format!("{context}.file"), error);
                    let mut file = std::fs::File::create(to).map_err(io)?;
                    let mut size = 0u64;
                    while let Some(chunk) = response
                        .chunk()
                        .await
                        .map_err(|error| network_error(&format!("{context}.chunk"), error))?
                    {
                        file.write_all(&chunk).map_err(io)?;
                        size += chunk.len() as u64;
                    }
                    file.sync_all().map_err(io)?;
                    size
                }
            };
            head.body = first_bytes(to, 8);
            Ok((head, size))
        }
        .await;
        if result.is_err() {
            let _ = std::fs::remove_file(to);
        }
        result
    }

    // ---- session import / export ----------------------------------------

    /// Adds cookies from outside the client (saved session, WebView bridge,
    /// server request) and marks the session as needing validation.
    pub fn import_cookie_header(&self, cookie: &str) -> VtopResult<()> {
        if cookie.trim().is_empty() {
            return Ok(());
        }
        let url = self.vtop_url()?;
        let pairs = parse_cookie_pairs(cookie);
        if pairs.is_empty() {
            self.jar.set_cookies(&url, &[cookie]);
        } else {
            let lines: Vec<String> = pairs
                .into_iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            self.jar
                .set_cookies(&url, &lines.iter().map(String::as_str).collect::<Vec<_>>());
        }
        self.state()
            .session
            .set_external_cookie_header(cookie.to_string());
        Ok(())
    }

    /// The `Cookie` header this client would send to VTOP.
    pub fn cookie_header(&self) -> Option<String> {
        self.vtop_url()
            .ok()
            .and_then(|url| self.jar.header_for(&url))
    }

    /// Restores a session saved by [`Self::export_persisted_session`].
    ///
    /// When the snapshot carries the CSRF token and registration number the
    /// session is trusted as signed in straight away (no `/vtop/content`
    /// round trip); if VTOP has dropped it, the first fetch fails with
    /// [`VtopError::SessionExpired`]. Older snapshots with only cookies are
    /// validated on first use. A snapshot without cookies keeps the ones in
    /// the cookie store (one that outlives the client, as the app's).
    pub fn restore_persisted_session(&self, session: &PersistedVtopSession) -> VtopResult<()> {
        self.auth_log(
            log::Level::Info,
            "session.restore",
            "restoring persisted session",
        );
        let snapshot = SessionState {
            cookies: session.cookies.clone().unwrap_or_default(),
            csrf_token: session.csrf_token.clone(),
            registration_number: session.registration_number.clone(),
            otp_issued_at: None,
            logged_in_at: session.logged_in_at,
        };
        self.resume(&snapshot, session.cookies.is_none())?;
        self.log_cookie_names("restore_session_snapshot");
        Ok(())
    }

    pub fn export_persisted_session(&self, saved_at_epoch_ms: u64) -> PersistedVtopSession {
        let state = self.session_state();
        PersistedVtopSession {
            username: self
                .credentials
                .as_ref()
                .map(|credentials| credentials.username.as_str().to_string())
                .unwrap_or_default(),
            saved_at_epoch_ms,
            cookies: (!state.cookies.is_empty()).then_some(state.cookies),
            csrf_token: state.csrf_token,
            registration_number: state.registration_number,
            logged_in_at: state.logged_in_at,
        }
    }

    /// A portable snapshot of this session for a stateless caller to keep.
    pub fn session_state(&self) -> SessionState {
        let cookies = self.cookie_header().unwrap_or_default();
        let state = self.state();
        SessionState {
            cookies,
            csrf_token: state.session.csrf_token(),
            registration_number: state
                .session
                .registration_number()
                .map(|number| number.as_str().to_string()),
            otp_issued_at: match state.auth_stage {
                AuthStage::AwaitingOtp {
                    issued_at_unix_seconds,
                } => Some(issued_at_unix_seconds),
                _ => None,
            },
            logged_in_at: state.logged_in_at,
        }
    }

    /// Replaces this client's session with `snapshot` (for example one a
    /// vtop-server logged in for). With a CSRF token and registration number
    /// the session is trusted as signed in; with a pending OTP it waits for
    /// [`Self::submit_security_otp`]; with only cookies it is validated on
    /// first use.
    pub fn resume_session(&self, snapshot: &SessionState) -> VtopResult<()> {
        self.resume(snapshot, false)
    }

    /// [`Self::resume_session`]; with `keep_cookies`, the store's cookies
    /// stay and `snapshot.cookies` is not read.
    fn resume(&self, snapshot: &SessionState, keep_cookies: bool) -> VtopResult<()> {
        let registration_number = snapshot
            .registration_number
            .as_deref()
            .map(RegistrationNumber::parse)
            .transpose()?;
        if keep_cookies {
            self.reset_auth_state();
        } else {
            self.reset_session_state();
            self.import_cookie_header(&snapshot.cookies)?;
        }
        let mut state = self.state();
        state.logged_in_at = snapshot.logged_in_at;
        if let Some(csrf) = snapshot.csrf_token.clone() {
            state.session.set_csrf_token(csrf);
        }
        if let Some(issued_at_unix_seconds) = snapshot.otp_issued_at {
            state.session.set_cookie_external(false);
            state.auth_stage = AuthStage::AwaitingOtp {
                issued_at_unix_seconds,
            };
        } else if let Some(registration_number) = registration_number {
            if snapshot.csrf_token.is_some() {
                state.session.set_registration_number(registration_number);
                state.session.set_authenticated(true);
            }
        }
        Ok(())
    }

    /// Unix seconds of the last real login on this session, if known.
    pub fn logged_in_at(&self) -> Option<u64> {
        self.state().logged_in_at
    }

    // ---- simple accessors ------------------------------------------------

    pub fn is_authenticated(&self) -> bool {
        self.state().session.is_authenticated()
    }

    pub fn registration_number(&self) -> VtopResult<String> {
        self.state()
            .session
            .registration_number()
            .map(|number| number.as_str().to_string())
            .ok_or(VtopError::RegistrationParsingError)
    }

    /// The raw `Cookie` header bytes. With `check`, fails unless signed in.
    pub fn cookie_bytes(&self, check: bool) -> VtopResult<Vec<u8>> {
        if check && !self.is_authenticated() {
            return Err(VtopError::SessionExpired);
        }
        Ok(self
            .cookie_header()
            .map(String::into_bytes)
            .unwrap_or_default())
    }

    #[cfg(all(test, feature = "reqwest"))]
    fn set_registration_number_for_test(&self, value: &str) {
        if let Ok(number) = RegistrationNumber::parse(value) {
            self.state().session.set_registration_number(number);
        }
    }
}

fn network_error(context: &str, error: impl std::fmt::Display) -> VtopError {
    // These errors carry the URL but never headers or bodies.
    error!(target: NETWORK, "{context}: {error}");
    VtopError::NetworkError
}

fn slow_vtop_error(context: &str) -> VtopError {
    unavailable(context, "VTOP took too long to answer")
}

fn unavailable(context: &str, reason: &str) -> VtopError {
    error!(target: NETWORK, "{context}: {reason}");
    VtopError::VtopUnavailable(reason.to_string())
}

/// A request that got no answer. A timeout is VTOP being slow; anything
/// else may be the phone's own connection.
fn send_error(context: &str, error: TransportError) -> VtopError {
    // The error never carries the URL (its query can hold the CSRF token);
    // the context says which request it was.
    if error.is_timeout() {
        warn!(target: NETWORK, "{context}: {error}");
        return slow_vtop_error(context);
    }
    network_error(context, error)
}

/// Up to `count` bytes from the start of the file at `path`.
fn first_bytes(path: &std::path::Path, count: u64) -> Vec<u8> {
    use std::io::Read;
    let mut bytes = Vec::new();
    if let Ok(file) = std::fs::File::open(path) {
        let _ = file.take(count).read_to_end(&mut bytes);
    }
    bytes
}

fn missing_csrf_error(context: &str) -> VtopError {
    warn!(target: AUTH, "{context}: CSRF token is missing, session needs validation");
    VtopError::SessionExpired
}

/// Against a mock VTOP over reqwest, so with the `reqwest` feature (on in
/// `cargo test --workspace`).
#[cfg(all(test, feature = "reqwest"))]
mod tests;

/// The transport a client gets when none is given: reqwest on a shared
/// pool, with the `reqwest` feature.
pub fn default_transport(config: &VtopConfig) -> VtopResult<Arc<dyn Transport>> {
    #[cfg(feature = "reqwest")]
    return Ok(Arc::new(transport::Reqwest(http::shared_client(config)?)));
    #[cfg(not(feature = "reqwest"))]
    {
        let _ = config;
        Err(VtopError::ConfigurationError(
            "no HTTP transport: set one, or build vtop-core with `reqwest`".into(),
        ))
    }
}

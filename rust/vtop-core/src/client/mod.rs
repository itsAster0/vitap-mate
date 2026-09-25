//! The VTOP client. Login lives in [`auth`], the security OTP in [`otp`],
//! CAPTCHA solving in [`captcha`], and one module per group of endpoints
//! holds the fetchers.
//!
//! Fetchers take `&self`, so one signed-in client can serve several requests
//! at once. Login and OTP take `&mut self` because they drive a multi-step
//! flow that must not interleave with itself.

mod academics;
mod auth;
mod biometric;
mod captcha;
mod email_otp;
mod exams;
mod http;
mod otp;

use std::sync::{Arc, Mutex, MutexGuard};

use log::{error, info, warn};
use reqwest::{Client, Response, Url};

use crate::config::VtopConfig;
use crate::error::{VtopError, VtopResult};
use crate::inputs::{Password, RegistrationNumber, Username};
use crate::session::{parse_cookie_pairs, ResettableJar, SessionManager, SessionState};
use crate::types::PersistedVtopSession;

pub use email_otp::EmailOtpLogin;
use reqwest::cookie::CookieStore;

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
    http: Client,
    config: Arc<VtopConfig>,
    jar: Arc<ResettableJar>,
    credentials: Option<Credentials>,
    state: Mutex<ClientState>,
    /// Serialises session validation so concurrent fetches on a restored
    /// session hit `/vtop/content` once, not once each.
    validation: tokio::sync::Mutex<()>,
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
}

impl VtopClientBuilder {
    pub fn config(mut self, config: VtopConfig) -> Self {
        self.config = config;
        self
    }

    fn build(self, credentials: Option<Credentials>) -> VtopResult<VtopClient> {
        let jar = Arc::new(ResettableJar::default());
        let http = http::shared_client(&self.config)?;
        Ok(VtopClient {
            http,
            config: Arc::new(self.config),
            jar,
            credentials,
            state: Mutex::new(ClientState {
                session: SessionManager::new(),
                auth_stage: AuthStage::Idle,
                logged_in_at: None,
            }),
            validation: tokio::sync::Mutex::new(()),
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

    fn vtop_url(&self) -> VtopResult<Url> {
        Url::parse(&self.config.url("/vtop"))
            .map_err(|error| VtopError::ConfigurationError(format!("invalid base URL: {error}")))
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

    fn log_request(context: &str, method: &str, url: &str) {
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
        let mut state = self.state();
        state.session.clear();
        state.auth_stage = AuthStage::Idle;
        state.logged_in_at = None;
        drop(state);
        self.jar.clear();
    }

    fn mark_session_expired(&self, context: &str, reason: &str) {
        self.reset_session_state();
        self.auth_log(
            log::Level::Warn,
            context,
            format!("session marked expired because {reason}"),
        );
    }

    fn is_login_url(url: &str) -> bool {
        Url::parse(url)
            .map(|parsed| parsed.path().starts_with("/vtop/login"))
            .unwrap_or_else(|_| url.to_lowercase().contains("/vtop/login"))
    }

    fn resolve_vtop_url(&self, path_or_url: &str) -> VtopResult<String> {
        if Url::parse(path_or_url).is_ok() {
            return Ok(path_or_url.to_string());
        }
        Url::parse(&self.config.base_url)
            .and_then(|base| base.join(path_or_url))
            .map(|url| url.to_string())
            .map_err(|error| VtopError::ConfigurationError(error.to_string()))
    }

    /// Reads a response to a signed-in request, treating a bounce to the
    /// login page or an HTTP error as an expired session.
    async fn read_authenticated_response_text(
        &self,
        context: &str,
        response: Response,
    ) -> VtopResult<String> {
        let final_url = response.url().to_string();
        let status = response.status();

        if Self::is_login_url(&final_url) {
            self.mark_session_expired(context, &format!("VTOP redirected to {final_url}"));
            return Err(VtopError::SessionExpired);
        }

        let text = response
            .text()
            .await
            .map_err(|error| network_error(&format!("{context}.text"), error))?;

        if !status.is_success() {
            self.mark_session_expired(
                context,
                &format!("VTOP returned HTTP {status} at {final_url}"),
            );
            return Err(server_error(
                context,
                format!("VTOP returned HTTP {status}"),
            ));
        }

        Ok(text)
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

    /// Sends a signed-in request built by `build`, retrying once after a
    /// short pause when VTOP times out, refuses the connection or answers
    /// with a 5xx. `build` is called again for the retry because multipart
    /// bodies cannot be cloned.
    async fn send_authenticated(
        &self,
        context: &str,
        build: impl Fn() -> reqwest::RequestBuilder,
    ) -> VtopResult<String> {
        let response = match build().send_with(self).await {
            Ok(response) if !response.status().is_server_error() => response,
            Ok(response) => {
                warn!(
                    target: NETWORK,
                    "{context}: VTOP returned HTTP {}, retrying once",
                    response.status()
                );
                tokio::time::sleep(RETRY_DELAY).await;
                build()
                    .send_with(self)
                    .await
                    .map_err(|error| network_error(&format!("{context}.retry"), error))?
            }
            Err(error) if error.is_timeout() || error.is_connect() => {
                warn!(target: NETWORK, "{context}: {error}, retrying once");
                tokio::time::sleep(RETRY_DELAY).await;
                build()
                    .send_with(self)
                    .await
                    .map_err(|error| network_error(&format!("{context}.retry"), error))?
            }
            Err(error) => return Err(network_error(&format!("{context}.send"), error)),
        };
        self.read_authenticated_response_text(context, response)
            .await
    }

    /// Sends a request with this session's cookies and follows redirects,
    /// storing every `Set-Cookie` along the way. The HTTP client is shared
    /// between sessions (one warm connection pool), so cookies are handled
    /// here rather than by reqwest. An explicit `Cookie` header on the
    /// request wins over the jar for the first hop.
    pub(super) async fn send_request(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> reqwest::Result<Response> {
        use reqwest::header::{COOKIE, LOCATION, SET_COOKIE};

        let mut request = builder.build()?;
        let mut hops = 0;
        loop {
            let url = request.url().clone();
            if !request.headers().contains_key(COOKIE) {
                if let Some(cookies) = self.jar.cookies(&url) {
                    request.headers_mut().insert(COOKIE, cookies);
                }
            }
            let response = self.http.execute(request).await?;
            self.jar
                .set_cookies(&mut response.headers().get_all(SET_COOKIE).iter(), &url);

            let status = response.status();
            // 307/308 would need the original body again; VTOP only uses
            // 301/302/303, which continue as a GET.
            if !matches!(status.as_u16(), 301..=303) || hops >= MAX_REDIRECTS {
                return Ok(response);
            }
            let Some(next) = response
                .headers()
                .get(LOCATION)
                .and_then(|location| location.to_str().ok())
                .and_then(|location| url.join(location).ok())
            else {
                return Ok(response);
            };
            hops += 1;
            request = self.http.get(next).build()?;
        }
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
            self.jar.add_cookie_str(cookie, &url);
        } else {
            for (name, value) in pairs {
                self.jar.add_cookie_str(&format!("{name}={value}"), &url);
            }
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
    /// validated on first use.
    pub fn restore_persisted_session(&self, session: &PersistedVtopSession) -> VtopResult<()> {
        self.auth_log(
            log::Level::Info,
            "session.restore",
            "restoring persisted session",
        );
        if let Some(cookies) = &session.cookies {
            self.resume_session(&SessionState {
                cookies: cookies.clone(),
                csrf_token: session.csrf_token.clone(),
                registration_number: session.registration_number.clone(),
                otp_issued_at: None,
                logged_in_at: session.logged_in_at,
            })?;
        }
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
        let registration_number = snapshot
            .registration_number
            .as_deref()
            .map(RegistrationNumber::parse)
            .transpose()?;
        self.reset_session_state();
        self.import_cookie_header(&snapshot.cookies)?;
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

    #[cfg(test)]
    fn set_registration_number_for_test(&self, value: &str) {
        if let Ok(number) = RegistrationNumber::parse(value) {
            self.state().session.set_registration_number(number);
        }
    }
}

fn network_error(context: &str, error: reqwest::Error) -> VtopError {
    // reqwest errors carry the URL but never headers or bodies.
    error!(target: NETWORK, "{context}: {error}");
    VtopError::NetworkError
}

fn server_error(context: &str, detail: impl AsRef<str>) -> VtopError {
    let message = format!("{context}: {}", detail.as_ref());
    error!(target: NETWORK, "returning VtopServerError: {message}");
    VtopError::VtopServerError(message)
}

fn missing_csrf_error(context: &str) -> VtopError {
    warn!(target: AUTH, "{context}: CSRF token is missing, session needs validation");
    VtopError::SessionExpired
}

#[cfg(test)]
mod tests;

/// `.send_with(client)` in place of `.send()`: routes a request through the
/// session's cookie jar and redirect handling on the shared HTTP client.
pub(super) trait SendWith {
    fn send_with<'a>(
        self,
        client: &'a VtopClient,
    ) -> impl std::future::Future<Output = reqwest::Result<Response>> + Send + 'a;
}

impl SendWith for reqwest::RequestBuilder {
    fn send_with<'a>(
        self,
        client: &'a VtopClient,
    ) -> impl std::future::Future<Output = reqwest::Result<Response>> + Send + 'a {
        client.send_request(self)
    }
}

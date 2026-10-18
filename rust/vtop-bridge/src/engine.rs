//! The engine's side of the edge: VTOP work and nothing else kept. The
//! edge holds accounts, keys and requests; it asks here whose a session is,
//! to seal saved credentials (only the engine has `VAULT_KEY`) and to sign
//! in with them. A sign-in answers at once and reports to the edge when
//! done; the OTP comes from the student's forwarded mail, sealed by the
//! edge to a key made for that one sign-in.
//!
//! Routes (`Authorization: Bearer $MAIL_RELAY_SECRET`):
//! `POST /engine/session`, `/engine/seal`, `/engine/login`, and
//! `/internal/mail-otp/owner` (whose a session is, as an owner hash).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use vtop_core::SessionState;
use vtop_shared::account::account_id;
use vtop_shared::engine::{
    LoginDone, LoginOutcome, LoginStart, OtpAwait, SealVault, SealedVault, SessionCheck,
    SessionChecked, SessionStatus, LOGIN_DONE, OTP_AWAIT,
};
use vtop_shared::mail_seal;
use vtop_shared::mailbox::owner_hash;
use vtop_shared::tokens;

use crate::error::ApiError;
use crate::login::{LoginError, OtpCodes};
use crate::verify::VerifyError;
use crate::AppState;

/// What the engine asks of the edge.
#[async_trait]
pub trait EdgeLink: Send + Sync {
    /// The next forwarded mail of `request.owner_hash`, sealed to
    /// `request.public_key`; `None` when nothing came in time.
    async fn await_otp(&self, request: &OtpAwait) -> Result<Option<String>, EdgeError>;
    async fn login_done(&self, done: &LoginDone) -> Result<(), EdgeError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeError {
    /// The student has no forwarding address.
    NoMailbox,
    Unavailable(String),
}

/// The edge over HTTPS.
pub struct HttpEdge {
    pub http: reqwest::Client,
    pub base_url: String,
    pub secret: String,
}

impl HttpEdge {
    async fn post(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<reqwest::Response, EdgeError> {
        self.http
            .post(format!("{}{path}", self.base_url))
            .bearer_auth(&self.secret)
            .timeout(Duration::from_secs(40))
            .json(body)
            .send()
            .await
            .map_err(|error| EdgeError::Unavailable(error.without_url().to_string()))
    }
}

#[async_trait]
impl EdgeLink for HttpEdge {
    async fn await_otp(&self, request: &OtpAwait) -> Result<Option<String>, EdgeError> {
        let response = self.post(OTP_AWAIT, request).await?;
        match response.status().as_u16() {
            200 => {
                let body: vtop_shared::engine::OtpAwaited = response
                    .json()
                    .await
                    .map_err(|error| EdgeError::Unavailable(error.to_string()))?;
                Ok(body.sealed)
            }
            404 => Err(EdgeError::NoMailbox),
            status => Err(EdgeError::Unavailable(format!("edge answered {status}"))),
        }
    }

    async fn login_done(&self, done: &LoginDone) -> Result<(), EdgeError> {
        let response = self.post(LOGIN_DONE, done).await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(EdgeError::Unavailable(format!(
                "edge answered {}",
                response.status()
            )))
        }
    }
}

/// OTP codes from forwarded mail, opened with a key made for this sign-in.
pub struct ForwardedCodes {
    edge: Arc<dyn EdgeLink>,
    owner_hash: String,
    secret: String,
    public_key: String,
    /// Set when the edge says the student has no forwarding address.
    pub no_mailbox: AtomicBool,
}

/// Mail dates come from VTOP's clock; allow it to run a little behind.
const CLOCK_SLACK_SECS: u64 = 120;
/// How long one wait at the edge lasts; the edge's own limit is shorter
/// than a request may take.
const AWAIT_STEP_SECS: u64 = 25;

impl ForwardedCodes {
    pub fn new(edge: Arc<dyn EdgeLink>, owner_hash: String) -> Self {
        let (secret, public_key) = mail_seal::generate_keypair();
        Self {
            edge,
            owner_hash,
            secret,
            public_key,
            no_mailbox: AtomicBool::new(false),
        }
    }

    /// Tells the edge to keep this sign-in's codes from now on, before VTOP
    /// is asked for one, so a mail that comes quickly is not missed.
    pub async fn register(&self) -> Result<(), EdgeError> {
        let request = OtpAwait {
            owner_hash: self.owner_hash.clone(),
            public_key: self.public_key.clone(),
            since: vtop_core::now_unix().saturating_sub(CLOCK_SLACK_SECS),
            wait_secs: 0,
        };
        self.edge.await_otp(&request).await.map(|_| ())
    }

    /// The code in a sealed mail payload, if it is a VTOP OTP.
    fn open(&self, sealed: &str) -> Option<String> {
        #[derive(Deserialize)]
        struct Payload {
            kind: String,
            #[serde(default)]
            code: Option<String>,
        }
        let plaintext = mail_seal::open(&self.secret, &self.owner_hash, sealed).ok()?;
        let payload: Payload = serde_json::from_slice(&plaintext).ok()?;
        (payload.kind == "otp").then_some(payload.code?)
    }
}

#[async_trait]
impl OtpCodes for ForwardedCodes {
    async fn next(
        &self,
        issued_at: u64,
        remaining: Duration,
        tried: Vec<String>,
    ) -> Option<String> {
        let deadline = tokio::time::Instant::now() + remaining;
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left < Duration::from_secs(1) {
                return None;
            }
            let request = OtpAwait {
                owner_hash: self.owner_hash.clone(),
                public_key: self.public_key.clone(),
                since: issued_at.saturating_sub(CLOCK_SLACK_SECS),
                wait_secs: left.as_secs().min(AWAIT_STEP_SECS),
            };
            match self.edge.await_otp(&request).await {
                Ok(Some(sealed)) => match self.open(&sealed) {
                    Some(code) if !tried.contains(&code) => return Some(code),
                    _ => {}
                },
                Ok(None) => {}
                Err(EdgeError::NoMailbox) => {
                    self.no_mailbox.store(true, Ordering::SeqCst);
                    return None;
                }
                Err(EdgeError::Unavailable(reason)) => {
                    tracing::info!("engine: waiting for a forwarded OTP: {reason}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}

/// Saved credentials, sealed to `accounts/<id>`. No `Debug`. Vaults from
/// the old bridge also carry a Gmail grant, which is ignored.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vault {
    /// The VTOP login name; it can change, the registration number cannot.
    pub username: String,
    pub password: String,
}

fn bad_request(message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "bad_request", message)
}

/// What sealed account fields are bound to, as the bridge always did.
fn accounts_aad(account: &str) -> String {
    format!("accounts/{account}")
}

/// Every route needs the secret shared with the edge.
fn require_secret(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let presented = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default()
        .trim();
    if !tokens::hashes_equal(
        &tokens::hash(&state.config.mail_relay_secret),
        &tokens::hash(presented),
    ) {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "The engine secret is required.",
        ));
    }
    Ok(())
}

fn cookies_only(cookies: &str) -> SessionState {
    SessionState {
        cookies: cookies.trim().to_string(),
        csrf_token: None,
        registration_number: None,
        otp_issued_at: None,
        logged_in_at: None,
    }
}

fn checked(status: SessionStatus) -> SessionChecked {
    SessionChecked {
        status,
        registration_number: None,
        cookies: None,
        sealed_session: None,
        owner_hash: None,
    }
}

/// Whose session fresh cookies are, or whether an account's cached one
/// still works. A cached session VTOP cannot be asked about is handed back
/// as it is, as the bridge always did.
pub async fn session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<SessionCheck>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(check) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let (session, expected) = match (&check.cookies, &check.sealed_session, &check.account) {
        (Some(cookies), _, _) if !cookies.trim().is_empty() => (cookies_only(cookies), None),
        (_, Some(sealed), Some(account)) => {
            let account = account_id(account);
            match state
                .accounts_sealer()
                .open_json::<SessionState>(&accounts_aad(&account), sealed)
            {
                Ok(session) => (cookies_only(&session.cookies), Some(account)),
                Err(_) => return Ok(Json(checked(SessionStatus::Expired)).into_response()),
            }
        }
        _ => {
            return Err(bad_request(
                "cookies, or sealedSession and account, are required.",
            ))
        }
    };
    let outcome = match state.verifier.registration_number(&session).await {
        Ok(owner) if expected.as_ref().is_none_or(|expected| *expected == owner) => {
            let owner = account_id(&owner);
            SessionChecked {
                status: SessionStatus::Valid,
                sealed_session: Some(
                    state
                        .accounts_sealer()
                        .seal_json(&accounts_aad(&owner), &session),
                ),
                owner_hash: Some(owner_hash(&state.config.vault_key, &owner)),
                cookies: Some(session.cookies),
                registration_number: Some(owner),
            }
        }
        Ok(_) | Err(VerifyError::Expired) => checked(SessionStatus::Expired),
        Err(VerifyError::Unavailable(reason)) => {
            tracing::info!("engine: cannot check a session: {reason}");
            SessionChecked {
                // Only a cached session is handed out unchecked.
                cookies: expected.map(|_| session.cookies),
                ..checked(SessionStatus::Unknown)
            }
        }
    };
    Ok(Json(outcome).into_response())
}

/// Seals saved credentials for an account; the edge stores the result.
pub async fn seal(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<SealVault>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let account = account_id(&body.account);
    let username = body.username.trim().to_string();
    if account.is_empty() || username.is_empty() || body.password.is_empty() {
        return Err(bad_request("account, username and password are required."));
    }
    let vault = Vault {
        username,
        password: body.password,
    };
    let sealed_vault = state
        .accounts_sealer()
        .seal_json(&accounts_aad(&account), &vault);
    Ok(Json(SealedVault { sealed_vault }).into_response())
}

/// Starts a sign-in with saved credentials; the outcome goes to the edge.
pub async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<LoginStart>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(start) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let edge = state.edge.clone();
    let account = account_id(&start.account);
    tokio::spawn(async move {
        let outcome = sign_in(&state, edge.clone(), &account, &start.sealed_vault).await;
        let done = LoginDone {
            account: account.clone(),
            request_id: start.request_id,
            outcome,
        };
        // The edge's request times out on its own if this never lands.
        for attempt in 0..3 {
            match edge.login_done(&done).await {
                Ok(()) => return,
                Err(error) => {
                    tracing::warn!(
                        "engine: report sign-in of {account} (try {attempt}): {error:?}"
                    );
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    });
    Ok(StatusCode::ACCEPTED.into_response())
}

async fn sign_in(
    state: &AppState,
    edge: Arc<dyn EdgeLink>,
    account: &str,
    sealed_vault: &str,
) -> LoginOutcome {
    let Ok(vault) = state
        .accounts_sealer()
        .open_json::<Vault>(&accounts_aad(account), sealed_vault)
    else {
        return LoginOutcome::VaultUnusable;
    };
    let codes = ForwardedCodes::new(edge, owner_hash(&state.config.vault_key, account));
    match codes.register().await {
        Ok(()) => {}
        // No forwarding address: VTOP is not asked (it would mail an OTP
        // nobody can read here).
        Err(EdgeError::NoMailbox) => return LoginOutcome::ForwardingRequired,
        Err(EdgeError::Unavailable(reason)) => {
            tracing::info!("engine: could not register for {account}'s OTP: {reason}");
        }
    }
    let result = state
        .forwarded_login
        .login(
            &vault.username,
            &vault.password,
            &codes,
            state.config.otp_wait,
        )
        .await;
    match result {
        Ok(session) => {
            let owner = session.registration_number.as_deref().map(account_id);
            if owner.as_deref() != Some(account) {
                tracing::warn!("engine: saved credentials of {account} sign in someone else");
                return LoginOutcome::VaultUnusable;
            }
            tracing::info!("engine: signed {account} in with saved credentials");
            LoginOutcome::Ok {
                sealed_session: state
                    .accounts_sealer()
                    .seal_json(&accounts_aad(account), &session),
                cookies: session.cookies,
            }
        }
        Err(LoginError::InvalidCredentials) => LoginOutcome::InvalidCredentials,
        Err(LoginError::Failed(_)) if codes.no_mailbox.load(Ordering::SeqCst) => {
            LoginOutcome::ForwardingRequired
        }
        Err(LoginError::Failed(reason)) => {
            tracing::info!("engine: sign-in of {account} failed for now: {reason}");
            LoginOutcome::Failed { reason }
        }
    }
}

#[derive(Deserialize)]
pub struct SessionBody {
    #[serde(default)]
    cookies: String,
}

/// Whose a live session is, as the owner hash the edge files forwarding
/// addresses under. The session is used for this check only.
pub async fn owner_check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<SessionBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_secret(&state, &headers)?;
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    if body.cookies.trim().is_empty() {
        return Err(bad_request("session.cookies is required."));
    }
    match state
        .verifier
        .registration_number(&cookies_only(&body.cookies))
        .await
    {
        Ok(account) => Ok(Json(
            json!({ "ownerHash": owner_hash(&state.config.vault_key, &account_id(&account)) }),
        )
        .into_response()),
        Err(VerifyError::Expired) => Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "session_invalid",
            "VTOP does not accept this session; sign in again.",
        )),
        Err(VerifyError::Unavailable(reason)) => {
            tracing::info!("engine: owner check: {reason}");
            Err(ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "vtop_unreachable",
                "VTOP is unreachable; try again.",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::fakes::{session as cookies_session, FakeLogin, FakeVerifier};
    use crate::Parts;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use serde_json::Value;
    use std::sync::Mutex;
    use tower::ServiceExt;

    const RELAY: &str = "relay-secret-relay-secret-relay-secret";
    const REG: &str = "22BCE0001";

    /// Hands out one forwarded OTP (or says there is no mailbox) and keeps
    /// what sign-ins report.
    #[derive(Default)]
    struct FakeEdge {
        code: Option<&'static str>,
        no_mailbox: bool,
        done: Mutex<Vec<LoginDone>>,
        awaited: Mutex<Vec<OtpAwait>>,
    }

    #[async_trait]
    impl EdgeLink for FakeEdge {
        async fn await_otp(&self, request: &OtpAwait) -> Result<Option<String>, EdgeError> {
            self.awaited.lock().unwrap().push(request.clone());
            if self.no_mailbox {
                return Err(EdgeError::NoMailbox);
            }
            Ok(self.code.map(|code| {
                let payload = json!({ "kind": "otp", "code": code, "sentAt": request.since + 130 });
                mail_seal::seal(
                    &request.public_key,
                    &request.owner_hash,
                    payload.to_string().as_bytes(),
                )
                .unwrap()
            }))
        }

        async fn login_done(&self, done: &LoginDone) -> Result<(), EdgeError> {
            self.done.lock().unwrap().push(done.clone());
            Ok(())
        }
    }

    fn engine_app(edge: Arc<FakeEdge>, login: FakeLogin) -> (axum::Router, Arc<AppState>) {
        let config = Config::from_lookup(|name| match name {
            "VAULT_KEY" => Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into()),
            "EDGE_URL" => Some("https://edge.example".into()),
            "MAIL_RELAY_SECRET" => Some(RELAY.into()),
            "OTP_WAIT_SECS" => Some("2".into()),
            _ => None,
        })
        .unwrap();
        let state = Arc::new(AppState::new(
            config,
            Parts {
                forwarded_login: Arc::new(login),
                verifier: Arc::new(FakeVerifier),
                edge,
            },
        ));
        (crate::router(state.clone()), state)
    }

    async fn post(app: &axum::Router, uri: &str, auth: &str, body: Value) -> (StatusCode, Value) {
        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {auth}"))
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn wait_for_done(edge: &FakeEdge) -> LoginDone {
        for _ in 0..200 {
            if let Some(done) = edge.done.lock().unwrap().first().cloned() {
                return done;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("the sign-in never reported");
    }

    #[tokio::test]
    async fn every_route_needs_the_shared_secret() {
        let (app, _) = engine_app(Arc::default(), FakeLogin::default());
        for uri in [
            vtop_shared::engine::SESSION,
            vtop_shared::engine::SEAL,
            vtop_shared::engine::LOGIN,
            "/internal/mail-otp/owner",
        ] {
            let (status, _) = post(&app, uri, "wrong", json!({})).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
        }
    }

    #[tokio::test]
    async fn fresh_cookies_name_their_owner_and_come_back_sealed() {
        let (app, state) = engine_app(Arc::default(), FakeLogin::default());
        let (status, body) = post(
            &app,
            "/engine/session",
            RELAY,
            json!({"cookies": "REG=24mic7076"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "valid");
        assert_eq!(body["registrationNumber"], "24MIC7076");
        assert_eq!(body["cookies"], "REG=24mic7076");
        assert_eq!(
            body["ownerHash"],
            owner_hash(&state.config.vault_key, "24MIC7076")
        );

        // The sealed copy checks out for that account only.
        let sealed = body["sealedSession"].as_str().unwrap();
        let (_, body) = post(
            &app,
            "/engine/session",
            RELAY,
            json!({"sealedSession": sealed, "account": "24MIC7076"}),
        )
        .await;
        assert_eq!(body["status"], "valid");
        assert_eq!(body["cookies"], "REG=24mic7076");
        let (_, body) = post(
            &app,
            "/engine/session",
            RELAY,
            json!({"sealedSession": sealed, "account": "24MIC0000"}),
        )
        .await;
        assert_eq!(body["status"], "expired");
        assert!(body.get("cookies").is_none());
    }

    #[tokio::test]
    async fn dead_and_unreachable_sessions() {
        let (app, state) = engine_app(Arc::default(), FakeLogin::default());
        let (_, body) = post(
            &app,
            "/engine/session",
            RELAY,
            json!({"cookies": "JSESSIONID=x"}),
        )
        .await;
        assert_eq!(body["status"], "expired");
        // Fresh cookies VTOP cannot be asked about are not vouched for.
        let (_, body) = post(
            &app,
            "/engine/session",
            RELAY,
            json!({"cookies": "REG=A; down"}),
        )
        .await;
        assert_eq!(body["status"], "unknown");
        assert!(body.get("cookies").is_none());
        // A cached one is handed back as it is.
        let sealed = state
            .accounts_sealer()
            .seal_json("accounts/A", &cookies_session("REG=A; down"));
        let (_, body) = post(
            &app,
            "/engine/session",
            RELAY,
            json!({"sealedSession": sealed, "account": "a"}),
        )
        .await;
        assert_eq!(body["status"], "unknown");
        assert_eq!(body["cookies"], "REG=A; down");
        let (status, _) = post(&app, "/engine/session", RELAY, json!({})).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    async fn sealed_vault(app: &axum::Router, account: &str) -> String {
        let (status, body) = post(
            app,
            "/engine/seal",
            RELAY,
            json!({"account": account, "username": "yaswanth314", "password": "pw"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        body["sealedVault"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn signs_in_with_the_forwarded_otp_and_reports_to_the_edge() {
        let edge = Arc::new(FakeEdge {
            code: Some("123456"),
            ..FakeEdge::default()
        });
        let (app, state) = engine_app(edge.clone(), FakeLogin::default());
        let vault = sealed_vault(&app, REG).await;
        let (status, _) = post(
            &app,
            "/engine/login",
            RELAY,
            json!({"account": REG, "sealedVault": vault, "requestId": "r1"}),
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        let done = wait_for_done(&edge).await;
        assert_eq!(
            (done.account.as_str(), done.request_id.as_str()),
            (REG, "r1")
        );
        let LoginOutcome::Ok {
            cookies,
            sealed_session,
        } = done.outcome
        else {
            panic!("not signed in");
        };
        assert_eq!(cookies, "JSESSIONID=fresh");
        let opened: SessionState = state
            .accounts_sealer()
            .open_json(&format!("accounts/{REG}"), &sealed_session)
            .unwrap();
        assert_eq!(opened.cookies, "JSESSIONID=fresh");
        let awaited = edge.awaited.lock().unwrap();
        assert_eq!(
            awaited[0].owner_hash,
            owner_hash(&state.config.vault_key, REG)
        );
    }

    #[tokio::test]
    async fn no_forwarding_address_means_no_server_sign_in() {
        let edge = Arc::new(FakeEdge {
            no_mailbox: true,
            ..FakeEdge::default()
        });
        let (app, _) = engine_app(edge.clone(), FakeLogin::default());
        let vault = sealed_vault(&app, REG).await;
        post(
            &app,
            "/engine/login",
            RELAY,
            json!({"account": REG, "sealedVault": vault, "requestId": "r"}),
        )
        .await;
        assert!(wait_for_done(&edge).await.outcome == LoginOutcome::ForwardingRequired);
    }

    #[tokio::test]
    async fn rejected_and_foreign_credentials() {
        let edge = Arc::new(FakeEdge {
            code: Some("1"),
            ..FakeEdge::default()
        });
        let (app, _) = engine_app(
            edge.clone(),
            FakeLogin::answering(Err(LoginError::InvalidCredentials)),
        );
        let vault = sealed_vault(&app, REG).await;
        post(
            &app,
            "/engine/login",
            RELAY,
            json!({"account": REG, "sealedVault": vault, "requestId": "r"}),
        )
        .await;
        assert!(wait_for_done(&edge).await.outcome == LoginOutcome::InvalidCredentials);

        // Credentials sealed for one account do not open for another, and
        // ones that sign in someone else are not used.
        let edge = Arc::new(FakeEdge {
            code: Some("1"),
            ..FakeEdge::default()
        });
        let (app, _) = engine_app(edge.clone(), FakeLogin::default());
        let other = sealed_vault(&app, "24MIC7076").await;
        post(
            &app,
            "/engine/login",
            RELAY,
            json!({"account": REG, "sealedVault": other, "requestId": "r"}),
        )
        .await;
        assert!(wait_for_done(&edge).await.outcome == LoginOutcome::VaultUnusable);
        let edge = Arc::new(FakeEdge {
            code: Some("1"),
            ..FakeEdge::default()
        });
        let (app, _) = engine_app(edge.clone(), FakeLogin::default());
        let theirs = sealed_vault(&app, "24MIC7076").await;
        post(
            &app,
            "/engine/login",
            RELAY,
            json!({"account": "24MIC7076", "sealedVault": theirs, "requestId": "r"}),
        )
        .await;
        assert!(wait_for_done(&edge).await.outcome == LoginOutcome::VaultUnusable);
    }

    #[tokio::test]
    async fn owner_check_names_the_owner_by_hash() {
        let (app, state) = engine_app(Arc::default(), FakeLogin::default());
        let (status, body) = post(
            &app,
            "/internal/mail-otp/owner",
            RELAY,
            json!({"cookies": "REG=24mic7076"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body["ownerHash"],
            owner_hash(&state.config.vault_key, "24MIC7076")
        );
        let (status, _) = post(
            &app,
            "/internal/mail-otp/owner",
            RELAY,
            json!({"cookies": "JSESSIONID=x"}),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn old_vaults_with_a_gmail_grant_still_open() {
        let (_, state) = engine_app(Arc::default(), FakeLogin::default());
        let old = json!({"username": "u", "password": "p",
            "gmail": {"refresh_token": "r", "client_id": "c"}});
        let sealed = state.accounts_sealer().seal_json("accounts/A", &old);
        let vault: Vault = state
            .accounts_sealer()
            .open_json("accounts/A", &sealed)
            .unwrap();
        assert_eq!((vault.username.as_str(), vault.password.as_str()), ("u", "p"));
    }

    #[tokio::test]
    async fn codes_skip_what_vtop_rejected() {
        let edge = Arc::new(FakeEdge {
            code: Some("111111"),
            ..FakeEdge::default()
        });
        let codes = ForwardedCodes::new(edge, "owner".into());
        assert_eq!(
            codes
                .next(1_000, Duration::from_secs(3), Vec::new())
                .await
                .as_deref(),
            Some("111111")
        );
        assert_eq!(
            codes
                .next(1_000, Duration::from_millis(1500), vec!["111111".into()])
                .await,
            None
        );
    }
}

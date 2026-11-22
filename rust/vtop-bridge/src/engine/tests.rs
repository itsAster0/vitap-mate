use super::*;
use crate::Parts;
use crate::config::Config;
use crate::fakes::{FakeLogin, FakeRefresher, FakeVerifier, session as cookies_session};
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::Mutex;
use tower::ServiceExt;

const RELAY: &str = "relay-secret-relay-secret-relay-secret";
const GATEWAY: &str = "gateway-secret-gateway-secret-gateway";
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
    engine_app_on(edge, login, vtop_core::VtopConfig::default())
}

fn engine_app_on(
    edge: Arc<FakeEdge>,
    login: FakeLogin,
    vtop: vtop_core::VtopConfig,
) -> (axum::Router, Arc<AppState>) {
    engine_app_with(edge, Arc::new(login), Arc::default(), vtop)
}

fn engine_app_with(
    edge: Arc<FakeEdge>,
    login: Arc<FakeLogin>,
    gmail: Arc<FakeRefresher>,
    vtop: vtop_core::VtopConfig,
) -> (axum::Router, Arc<AppState>) {
    let config = Config::from_lookup(|name| match name {
        "VAULT_KEY" => Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into()),
        "EDGE_URL" => Some("https://edge.example".into()),
        "INTERNAL_TOKEN" => Some(RELAY.into()),
        "GATEWAY_TOKEN" => Some(GATEWAY.into()),
        "OTP_WAIT_SECS" => Some("2".into()),
        _ => None,
    })
    .unwrap();
    let state = Arc::new(
        AppState::new(
            config,
            Parts {
                forwarded_login: login,
                verifier: Arc::new(FakeVerifier),
                edge,
                gmail,
            },
        )
        .with_vtop(vtop),
    );
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
        vtop_shared::engine::MAIL_OWNER,
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
        vtop_shared::engine::SESSION,
        RELAY,
        json!({"cookies": "REG=23abc4567"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "valid");
    assert_eq!(body["registrationNumber"], "23ABC4567");
    assert_eq!(body["cookies"], "REG=23abc4567");
    assert_eq!(
        body["ownerHash"],
        owner_hash(&state.config.vault_key, "23ABC4567")
    );

    // The sealed copy checks out for that account only.
    let sealed = body["sealedSession"].as_str().unwrap();
    let (_, body) = post(
        &app,
        vtop_shared::engine::SESSION,
        RELAY,
        json!({"sealedSession": sealed, "account": "23ABC4567"}),
    )
    .await;
    assert_eq!(body["status"], "valid");
    assert_eq!(body["cookies"], "REG=23abc4567");
    let (_, body) = post(
        &app,
        vtop_shared::engine::SESSION,
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
        vtop_shared::engine::SESSION,
        RELAY,
        json!({"cookies": "JSESSIONID=x"}),
    )
    .await;
    assert_eq!(body["status"], "expired");
    // Fresh cookies VTOP cannot be asked about are not vouched for.
    let (_, body) = post(
        &app,
        vtop_shared::engine::SESSION,
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
        vtop_shared::engine::SESSION,
        RELAY,
        json!({"sealedSession": sealed, "account": "a"}),
    )
    .await;
    assert_eq!(body["status"], "unknown");
    assert_eq!(body["cookies"], "REG=A; down");
    let (status, _) = post(&app, vtop_shared::engine::SESSION, RELAY, json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

async fn sealed_vault(app: &axum::Router, account: &str) -> String {
    let (status, body) = post(
        app,
        vtop_shared::engine::SEAL,
        RELAY,
        json!({"account": account, "username": "student123", "password": "pw"}),
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
        vtop_shared::engine::LOGIN,
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
        ..
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
        vtop_shared::engine::LOGIN,
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
        vtop_shared::engine::LOGIN,
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
    let other = sealed_vault(&app, "23ABC4567").await;
    post(
        &app,
        vtop_shared::engine::LOGIN,
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
    let theirs = sealed_vault(&app, "23ABC4567").await;
    post(
        &app,
        vtop_shared::engine::LOGIN,
        RELAY,
        json!({"account": "23ABC4567", "sealedVault": theirs, "requestId": "r"}),
    )
    .await;
    assert!(wait_for_done(&edge).await.outcome == LoginOutcome::VaultUnusable);
}

#[tokio::test]
async fn owner_check_names_the_owner_by_hash() {
    let (app, state) = engine_app(Arc::default(), FakeLogin::default());
    let (status, body) = post(
        &app,
        vtop_shared::engine::MAIL_OWNER,
        RELAY,
        json!({"cookies": "REG=23abc4567"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["ownerHash"],
        owner_hash(&state.config.vault_key, "23ABC4567")
    );
    assert_eq!(body["registrationNumber"], "23ABC4567");
    let (status, _) = post(
        &app,
        vtop_shared::engine::MAIL_OWNER,
        RELAY,
        json!({"cookies": "JSESSIONID=x"}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn old_vaults_keep_their_gmail_grant() {
    let (_, state) = engine_app(Arc::default(), FakeLogin::default());
    let old = json!({"username": "u", "password": "p",
        "gmail": {"refresh_token": "r", "client_id": "c"}});
    let sealed = state.accounts_sealer().seal_json("accounts/A", &old);
    let vault: Vault = state
        .accounts_sealer()
        .open_json("accounts/A", &sealed)
        .unwrap();
    assert_eq!(
        (vault.username.as_str(), vault.password.as_str()),
        ("u", "p")
    );
    let grant = vault.gmail.unwrap();
    assert_eq!(
        (grant.refresh_token.as_str(), grant.client_id.as_str()),
        ("r", "c")
    );
}

async fn gmail_vault(app: &axum::Router, gmail: Value) -> String {
    let (status, body) = post(
        app,
        vtop_shared::engine::SEAL,
        RELAY,
        json!({"account": REG, "username": "student123", "password": "pw", "gmail": gmail}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    body["sealedVault"].as_str().unwrap().to_string()
}

fn opened(state: &AppState, sealed: &str) -> Vault {
    state
        .accounts_sealer()
        .open_json(&format!("accounts/{REG}"), sealed)
        .unwrap()
}

#[tokio::test]
async fn gmail_access_is_sealed_with_the_credentials() {
    let (app, state) = engine_app(Arc::default(), FakeLogin::default());
    let sealed = gmail_vault(
        &app,
        json!({"refreshToken": " r ", "clientId": "c", "clientSecret": "s", "deleteAfterReading": true}),
    )
    .await;
    let grant = opened(&state, &sealed).gmail.unwrap();
    assert_eq!(grant.refresh_token, "r");
    assert_eq!(grant.client_secret.as_deref(), Some("s"));
    assert!(grant.delete_after_reading);
    // A grant without a token is not kept.
    let sealed = gmail_vault(&app, json!({"refreshToken": "", "clientId": "c"})).await;
    assert!(opened(&state, &sealed).gmail.is_none());
}

async fn sign_in_once(
    edge: Arc<FakeEdge>,
    login: Arc<FakeLogin>,
    gmail: Arc<FakeRefresher>,
    grant: Option<Value>,
) -> (LoginOutcome, Arc<AppState>) {
    let (app, state) =
        engine_app_with(edge.clone(), login, gmail, vtop_core::VtopConfig::default());
    let vault = match grant {
        Some(grant) => gmail_vault(&app, grant).await,
        None => sealed_vault(&app, REG).await,
    };
    post(
        &app,
        vtop_shared::engine::LOGIN,
        RELAY,
        json!({"account": REG, "sealedVault": vault, "requestId": "r"}),
    )
    .await;
    (wait_for_done(&edge).await.outcome, state)
}

fn no_mailbox() -> Arc<FakeEdge> {
    Arc::new(FakeEdge {
        no_mailbox: true,
        ..FakeEdge::default()
    })
}

#[tokio::test]
async fn without_forwarding_gmail_reads_the_otp() {
    let gmail = Arc::new(FakeRefresher::default());
    let login = Arc::new(FakeLogin::answering(Err(LoginError::InvalidCredentials)));
    let (outcome, _) = sign_in_once(
        no_mailbox(),
        login.clone(),
        gmail.clone(),
        Some(json!({"refreshToken": "r", "clientId": "c"})),
    )
    .await;
    assert!(outcome == LoginOutcome::InvalidCredentials);
    assert_eq!(gmail.calls.load(Ordering::SeqCst), 1);
    assert_eq!(login.calls(), 1);
}

#[tokio::test]
async fn forwarding_comes_before_gmail() {
    let edge = Arc::new(FakeEdge {
        code: Some("123456"),
        ..FakeEdge::default()
    });
    let gmail = Arc::new(FakeRefresher::default());
    let (outcome, _) = sign_in_once(
        edge,
        Arc::new(FakeLogin::default()),
        gmail.clone(),
        Some(json!({"refreshToken": "r", "clientId": "c"})),
    )
    .await;
    assert!(matches!(outcome, LoginOutcome::Ok { .. }));
    assert_eq!(gmail.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn gmail_picked_on_the_phone_skips_forwarding() {
    let edge = Arc::new(FakeEdge {
        code: Some("123456"),
        ..FakeEdge::default()
    });
    let gmail = Arc::new(FakeRefresher::default());
    let login = Arc::new(FakeLogin::answering(Err(LoginError::InvalidCredentials)));
    let (app, state) = engine_app_with(
        edge.clone(),
        login.clone(),
        gmail.clone(),
        vtop_core::VtopConfig::default(),
    );
    let (_, body) = post(
        &app,
        vtop_shared::engine::SEAL,
        RELAY,
        json!({"account": REG, "username": "student123", "password": "pw", "otp": "gmail",
            "gmail": {"refreshToken": "r", "clientId": "c"}}),
    )
    .await;
    let vault = body["sealedVault"].as_str().unwrap();
    assert_eq!(opened(&state, vault).otp, Some(OtpMethod::Gmail));
    post(
        &app,
        vtop_shared::engine::LOGIN,
        RELAY,
        json!({"account": REG, "sealedVault": vault, "requestId": "r"}),
    )
    .await;
    assert!(wait_for_done(&edge).await.outcome == LoginOutcome::InvalidCredentials);
    assert_eq!(gmail.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn revoked_gmail_access_is_dropped_and_the_password_kept() {
    let login = Arc::new(FakeLogin::default());
    let (outcome, state) = sign_in_once(
        no_mailbox(),
        login.clone(),
        Arc::new(FakeRefresher {
            revoked: true,
            ..FakeRefresher::default()
        }),
        Some(json!({"refreshToken": "r", "clientId": "c"})),
    )
    .await;
    let LoginOutcome::GmailRevoked { sealed_vault } = outcome else {
        panic!("not revoked");
    };
    let vault = opened(&state, &sealed_vault);
    assert_eq!(
        (vault.username.as_str(), vault.password.as_str()),
        ("student123", "pw")
    );
    assert!(vault.gmail.is_none());
    // VTOP was not asked: it would have mailed an OTP nobody reads.
    assert_eq!(login.calls(), 0);
}

#[tokio::test]
async fn neither_forwarding_nor_gmail_means_no_server_sign_in() {
    let login = Arc::new(FakeLogin::default());
    let (outcome, _) = sign_in_once(no_mailbox(), login.clone(), Arc::default(), None).await;
    assert!(outcome == LoginOutcome::ForwardingRequired);
    assert_eq!(login.calls(), 0);
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

// ---- phone sign-in and fetches (`vtop_login`, `fetch`) ----------------

fn phone_app() -> axum::Router {
    engine_app(
        Arc::new(FakeEdge::default()),
        FakeLogin::answering(Err(LoginError::InvalidCredentials)),
    )
    .0
}

fn signed_in() -> Value {
    json!({ "cookies": "JSESSIONID=1", "csrf_token": "c", "registration_number": REG })
}

#[tokio::test]
async fn phone_routes_need_the_engine_secret() {
    let app = phone_app();
    for uri in [
        vtop_shared::engine::FETCH,
        vtop_shared::engine::VTOP_LOGIN,
        vtop_shared::engine::VTOP_LOGIN_OTP,
    ] {
        let (status, body) = post(&app, uri, "wrong", json!({})).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
        assert_eq!(body["code"], "unauthorized");
    }
}

#[tokio::test]
async fn the_gateway_token_opens_only_the_gateway_routes() {
    let app = phone_app();
    let (status, _) = post(&app, vtop_shared::engine::FETCH, GATEWAY, json!({})).await;
    assert_ne!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = post(&app, vtop_shared::engine::VTOP_LOGIN, GATEWAY, json!({})).await;
    assert_ne!(status, StatusCode::UNAUTHORIZED);
    for uri in [
        "/internal/mail-otp/owner",
        "/engine/session",
        "/engine/seal",
        "/engine/login",
    ] {
        let (status, body) = post(&app, uri, GATEWAY, json!({ "cookies": "JSESSIONID=1" })).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
        assert_eq!(body["code"], "unauthorized");
    }
}

#[tokio::test]
async fn a_fetch_names_a_known_page_with_its_arguments() {
    let app = phone_app();
    let (status, body) = post(
        &app,
        vtop_shared::engine::FETCH,
        RELAY,
        json!({ "session": signed_in(), "name": "apply_general_outing" }),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("unknown_page"))
    );

    let (status, body) = post(
        &app,
        vtop_shared::engine::FETCH,
        RELAY,
        json!({ "session": signed_in(), "name": "grade_details", "semester": "AP2025264" }),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("bad_arguments"))
    );
}

#[tokio::test]
async fn a_fetch_without_cookies_asks_for_a_sign_in() {
    let (status, body) = post(
        &phone_app(),
        vtop_shared::engine::FETCH,
        RELAY,
        json!({ "session": { "cookies": " " }, "name": "semesters" }),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("session_expired"))
    );
}

#[tokio::test]
async fn an_otp_needs_a_session_waiting_for_one() {
    let (status, body) = post(
        &phone_app(),
        vtop_shared::engine::VTOP_LOGIN_OTP,
        RELAY,
        json!({ "session": signed_in(), "otp": "123456" }),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("no_pending_login"))
    );
}

#[tokio::test]
async fn answers_are_brotli_for_callers_that_take_it() {
    let request = Request::builder()
        .method("POST")
        .uri(vtop_shared::engine::FETCH)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {RELAY}"))
        .header("accept-encoding", "br, gzip")
        .body(Body::from(
            json!({ "session": signed_in(), "name": "nope" }).to_string(),
        ))
        .unwrap();
    let response = phone_app().oneshot(request).await.unwrap();
    assert_eq!(response.headers()["content-encoding"], "br");
}

/// A VTOP that answers every request with the semesters fixture.
async fn vtop_with_semesters() -> (wiremock::MockServer, axum::Router) {
    let vtop = wiremock::MockServer::start().await;
    let html = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../vtop-proto/tests/fixtures/semesters.html"),
    )
    .unwrap();
    wiremock::Mock::given(wiremock::matchers::any())
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(html))
        .mount(&vtop)
        .await;
    let config = vtop_core::VtopConfig {
        base_url: vtop.uri(),
        ..Default::default()
    };
    let app = engine_app_on(
        Arc::new(FakeEdge::default()),
        FakeLogin::answering(Err(LoginError::InvalidCredentials)),
        config,
    )
    .0;
    (vtop, app)
}

async fn fetch_semesters(app: &axum::Router, dictionary: Option<&str>) -> Response {
    let request = Request::builder()
        .method("POST")
        .uri(vtop_shared::engine::FETCH)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {RELAY}"))
        .header("accept-encoding", "br, gzip")
        .body(Body::from(
            json!({ "session": signed_in(), "name": "semesters", "dictionary": dictionary })
                .to_string(),
        ))
        .unwrap();
    app.clone().oneshot(request).await.unwrap()
}

#[tokio::test]
async fn a_phone_with_the_same_dictionary_gets_the_page_packed() {
    use vtop_shared::page_dictionary;
    let (_vtop, app) = vtop_with_semesters().await;
    let response = fetch_semesters(&app, Some(page_dictionary::id())).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        page_dictionary::CONTENT_TYPE
    );
    // Packed already: not compressed again on the way.
    assert!(response.headers().get("content-encoding").is_none());
    let packed = response.into_body().collect().await.unwrap().to_bytes();
    let page: Value =
        serde_json::from_slice(&page_dictionary::unpack("semesters", &packed).unwrap()).unwrap();
    assert!(!page["semesters"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn a_batch_answers_each_page_on_its_own() {
    let (_vtop, app) = vtop_with_semesters().await;
    let pages = json!([{ "name": "semesters" }, { "name": "nope" }, { "name": "grade_details" }]);
    let (status, body) = post(
        &app,
        vtop_shared::engine::FETCH_BATCH,
        RELAY,
        json!({ "session": signed_in(), "semester": "AP1", "pages": pages }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let answers = body["data"].as_array().unwrap();
    assert!(
        !answers[0]["data"]["semesters"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(answers[1]["code"], "unknown_page");
    assert_eq!(answers[2]["code"], "bad_arguments");
}

#[tokio::test]
async fn a_batch_without_cookies_asks_for_a_sign_in() {
    let (status, body) = post(
        &phone_app(),
        vtop_shared::engine::FETCH_BATCH,
        RELAY,
        json!({ "session": { "cookies": "" }, "pages": [{ "name": "semesters" }] }),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("session_expired"))
    );
}

#[tokio::test]
async fn a_batch_comes_packed_for_the_same_dictionary() {
    use vtop_shared::page_dictionary;
    let (_vtop, app) = vtop_with_semesters().await;
    let body = json!({
        "session": signed_in(),
        "pages": [{ "name": "semesters" }, { "name": "semesters" }],
        "dictionary": page_dictionary::id(),
    });
    let request = Request::builder()
        .method("POST")
        .uri(vtop_shared::engine::FETCH_BATCH)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {RELAY}"))
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.headers()["content-type"],
        page_dictionary::CONTENT_TYPE
    );
    let packed = response.into_body().collect().await.unwrap().to_bytes();
    let json = page_dictionary::unpack(vtop_shared::wire_page::BATCH, &packed).unwrap();
    let answers: Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(answers.as_array().unwrap().len(), 2);
    assert!(answers[1]["data"]["semesters"].is_array());
}

#[tokio::test]
async fn another_dictionary_gets_plain_json() {
    let (_vtop, app) = vtop_with_semesters().await;
    for dictionary in [None, Some("000000000000")] {
        let response = fetch_semesters(&app, dictionary).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "application/json");
        assert_eq!(response.headers()["content-encoding"], "br");
    }
}

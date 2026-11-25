use super::*;
use crate::Parts;
use crate::config::Config;
use crate::fakes::{FakeLogin, FakeRefresher, FakeVerifier, STALE_OTP, session as cookies_session};
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::atomic::Ordering;
use tower::ServiceExt;

const RELAY: &str = "relay-secret-relay-secret-relay-secret";
const GATEWAY: &str = "gateway-secret-gateway-secret-gateway";
const REG: &str = "22BCE0001";

fn engine_app(login: FakeLogin) -> (axum::Router, Arc<AppState>) {
    engine_app_on(login, vtop_core::VtopConfig::default())
}

fn engine_app_on(login: FakeLogin, vtop: vtop_core::VtopConfig) -> (axum::Router, Arc<AppState>) {
    engine_app_with(Arc::new(login), Arc::default(), vtop)
}

fn engine_app_with(
    login: Arc<FakeLogin>,
    gmail: Arc<FakeRefresher>,
    vtop: vtop_core::VtopConfig,
) -> (axum::Router, Arc<AppState>) {
    let config = Config::from_lookup(|name| match name {
        "VAULT_KEY" => Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into()),
        "CONVEX_URL" => Some("https://edge.example".into()),
        "INTERNAL_TOKEN" => Some(RELAY.into()),
        "GATEWAY_TOKEN" => Some(GATEWAY.into()),
        "OTP_WAIT_SECS" => Some("2".into()),
        "PORT" => Some("8080".into()),
        _ => None,
    })
    .unwrap();
    let state = Arc::new(
        AppState::new(
            config,
            Parts {
                forwarded_login: login,
                verifier: Arc::new(FakeVerifier),
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

#[tokio::test]
async fn every_route_needs_the_shared_secret() {
    let (app, _) = engine_app(FakeLogin::default());
    for uri in [
        vtop_shared::engine::SESSION,
        vtop_shared::engine::SEAL,
        vtop_shared::engine::LOGIN,
        vtop_shared::engine::MCP_EVICT,
        vtop_shared::engine::MAIL_OWNER,
    ] {
        let (status, _) = post(&app, uri, "wrong", json!({})).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
    }
}

#[tokio::test]
async fn fresh_cookies_name_their_owner_and_come_back_sealed() {
    let (app, state) = engine_app(FakeLogin::default());
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
async fn session_check_omits_sealed_session_when_seal_is_false() {
    let (app, state) = engine_app(FakeLogin::default());
    let (status, body) = post(
        &app,
        vtop_shared::engine::SESSION,
        RELAY,
        json!({"cookies": "REG=23abc4567", "seal": false}),
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
    assert!(body.get("sealedSession").is_none());

    // Explicit `seal: true` still returns `sealedSession`.
    let (status, body) = post(
        &app,
        vtop_shared::engine::SESSION,
        RELAY,
        json!({"cookies": "REG=23abc4567", "seal": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["sealedSession"].is_string());
}

#[tokio::test]
async fn mcp_evict_route_requires_internal_token_and_drops_cached_sessions() {
    use async_trait::async_trait;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;
    use vtop_mcp::bridge::{CachedSessions, Identity, SessionSource};

    #[derive(Default)]
    struct StubSource {
        sessions: AtomicUsize,
    }

    #[async_trait]
    impl SessionSource for StubSource {
        async fn whoami(&self, _key: &str) -> Result<Option<Identity>, String> {
            Ok(Some(Identity {
                registration_number: REG.into(),
                key_label: None,
                semester_id: None,
                outings: false,
            }))
        }
        async fn session(&self, key: &str, _ip: Option<&str>) -> Result<SessionState, String> {
            let n = self.sessions.fetch_add(1, Ordering::SeqCst);
            Ok(cookies_session(&format!("{key}-{n}")))
        }
        async fn expire(&self, _key: &str) -> Result<(), String> {
            Ok(())
        }
    }

    let inner = Arc::new(StubSource::default());
    let sessions = Arc::new(CachedSessions::new(inner.clone(), Duration::from_secs(300)));
    let (_mcp, evictor) =
        vtop_mcp::mcp_router_with_evictor(sessions.clone(), Duration::from_secs(300));
    let config = Config::from_lookup(|name| match name {
        "VAULT_KEY" => Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into()),
        "CONVEX_URL" => Some("https://edge.example".into()),
        "INTERNAL_TOKEN" => Some(RELAY.into()),
        "GATEWAY_TOKEN" => Some(GATEWAY.into()),
        "OTP_WAIT_SECS" => Some("2".into()),
        "PORT" => Some("8080".into()),
        _ => None,
    })
    .unwrap();
    let state = Arc::new(
        AppState::new(
            config,
            Parts {
                forwarded_login: Arc::new(FakeLogin::default()),
                verifier: Arc::new(FakeVerifier),
                gmail: Arc::new(FakeRefresher::default()),
            },
        )
        .with_mcp_evictor(evictor),
    );
    let app = crate::router(state);

    // Populate the RAM cache for two keys.
    let s1 = sessions.session("vtm_key1", None).await.unwrap();
    let s2 = sessions.session("vtm_key2", None).await.unwrap();
    assert_eq!(sessions.session("vtm_key1", None).await.unwrap(), s1);
    assert_eq!(inner.sessions.load(Ordering::SeqCst), 2);

    // Gateway token is rejected.
    let (status, _) = post(
        &app,
        vtop_shared::engine::MCP_EVICT,
        GATEWAY,
        json!({"keyHashes": [tokens::hash("vtm_key1")]}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Internal token evicts only the named key hash.
    let (status, body) = post(
        &app,
        vtop_shared::engine::MCP_EVICT,
        RELAY,
        json!({"keyHashes": [tokens::hash("vtm_key1")]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["evicted"], 1);

    assert_eq!(sessions.session("vtm_key2", None).await.unwrap(), s2);
    assert_ne!(sessions.session("vtm_key1", None).await.unwrap(), s1);
    assert_eq!(inner.sessions.load(Ordering::SeqCst), 3);
}

/// Convex asks for a proved regno's owner hash: the same hash the session check gives, so the
/// accounts' stored hashes keep matching.
#[tokio::test]
async fn a_proved_regno_gets_the_owner_hash_of_its_sessions() {
    let (app, state) = engine_app(FakeLogin::default());
    let ask = |regno: &str| json!({ "registrationNumber": regno });
    let (status, body) = post(&app, OWNER_HASH, RELAY, ask(" 23abc4567 ")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["ownerHash"],
        owner_hash(&state.config.vault_key, "23ABC4567")
    );
    // Pinned for the all-zero test key, as Convex stores it.
    assert_eq!(
        body["ownerHash"],
        "943f2cf09a99e88ac7f3a1da32010f9709b1cf3b523708eabe6dc7eeaab5cd62"
    );
    assert_eq!(
        post(&app, OWNER_HASH, RELAY, ask(" ")).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        post(&app, OWNER_HASH, GATEWAY, ask("23ABC4567")).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn dead_and_unreachable_sessions() {
    let (app, state) = engine_app(FakeLogin::default());
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
    // A cached one is handed back as it is, with its CSRF token.
    let mut cached = cookies_session("REG=A; down");
    cached.csrf_token = Some("csrf-1".into());
    let sealed = state.accounts_sealer().seal_json("accounts/A", &cached);
    let (_, body) = post(
        &app,
        vtop_shared::engine::SESSION,
        RELAY,
        json!({"sealedSession": sealed, "account": "a"}),
    )
    .await;
    assert_eq!(body["status"], "unknown");
    assert_eq!(body["cookies"], "REG=A; down");
    assert_eq!(body["csrfToken"], "csrf-1");
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

/// One sign-in step; the outcome when it is done.
async fn step(app: &axum::Router, body: Value) -> Value {
    let (status, body) = post(app, vtop_shared::engine::LOGIN, RELAY, body).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

fn done(body: Value) -> LoginOutcome {
    serde_json::from_value(body).expect("a finished sign-in")
}

#[tokio::test]
async fn a_forwarded_sign_in_stops_at_the_otp_and_resumes_with_the_mail() {
    let login = Arc::new(FakeLogin::default());
    let (app, state) = engine_app_with(
        login.clone(),
        Arc::default(),
        vtop_core::VtopConfig::default(),
    );
    let vault = sealed_vault(&app, REG).await;
    let waiting = step(
        &app,
        json!({"account": REG, "sealedVault": vault, "forwarding": true}),
    )
    .await;
    assert_eq!(waiting["status"], "otp_needed");
    // VTOP mailed the code at 1000 (the fake's clock); its clock may run behind.
    assert_eq!(waiting["since"], 1_000 - CLOCK_SLACK_SECS);
    let resume = |waiting: &Value, code: &str| json!({"account": REG, "sealedState": waiting["sealedState"], "otp": code});
    // An older mail's code leaves it waiting, and is not sent twice.
    let again = step(&app, resume(&waiting, STALE_OTP)).await;
    assert_eq!(again["status"], "otp_needed");
    let calls = login.calls();
    let again = step(&app, resume(&again, STALE_OTP)).await;
    assert_eq!(
        (again["status"].as_str(), login.calls()),
        (Some("otp_needed"), calls)
    );
    let LoginOutcome::Ok {
        cookies,
        sealed_session,
        ..
    } = done(step(&app, resume(&again, "123456")).await)
    else {
        panic!("not signed in");
    };
    assert_eq!(cookies, "JSESSIONID=fresh");
    let opened: SessionState = state
        .accounts_sealer()
        .open_json(&format!("accounts/{REG}"), &sealed_session)
        .unwrap();
    assert_eq!(opened.cookies, "JSESSIONID=fresh");
    // The waiting sign-in opens for its own account only, and emits a structured error code.
    let theirs =
        json!({"account": "23ABC4567", "sealedState": waiting["sealedState"], "otp": "123456"});
    assert!(matches!(
        done(step(&app, theirs).await),
        LoginOutcome::Failed {
            code: Some(code),
            reason
        } if code == "login_expired" && !reason.contains("login_expired:")
    ));
    for bad in [
        json!({"account": REG}),
        json!({"account": "", "sealedVault": vault}),
        json!({"account": REG, "sealedVault": vault, "sealedState": waiting["sealedState"], "otp": "123456"}),
    ] {
        let (status, _) = post(&app, vtop_shared::engine::LOGIN, RELAY, bad).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}

#[test]
fn login_stepped_wire_roundtrips_every_variant() {
    let cases = [
        (
            LoginStepped::Ok {
                cookies: "JSESSIONID=a".into(),
                sealed_session: "s".into(),
                csrf_token: Some("c".into()),
            },
            json!({"status": "ok", "cookies": "JSESSIONID=a", "sealedSession": "s", "csrfToken": "c"}),
        ),
        (
            LoginStepped::Ok {
                cookies: "JSESSIONID=a".into(),
                sealed_session: "s".into(),
                csrf_token: None,
            },
            json!({"status": "ok", "cookies": "JSESSIONID=a", "sealedSession": "s"}),
        ),
        (
            LoginStepped::InvalidCredentials,
            json!({"status": "invalid_credentials"}),
        ),
        (
            LoginStepped::VaultUnusable,
            json!({"status": "vault_unusable"}),
        ),
        (
            LoginStepped::ForwardingRequired,
            json!({"status": "forwarding_required"}),
        ),
        (
            LoginStepped::GmailRevoked {
                sealed_vault: "v".into(),
            },
            json!({"status": "gmail_revoked", "sealedVault": "v"}),
        ),
        (
            LoginStepped::Failed {
                code: Some("otp_not_found".into()),
                reason: "the OTP mail did not arrive".into(),
            },
            json!({"status": "failed", "code": "otp_not_found", "reason": "the OTP mail did not arrive"}),
        ),
        (
            LoginStepped::Failed {
                code: None,
                reason: "legacy".into(),
            },
            json!({"status": "failed", "reason": "legacy"}),
        ),
        (
            LoginStepped::OtpNeeded {
                sealed_state: "st".into(),
                since: 1234,
            },
            json!({"status": "otp_needed", "sealedState": "st", "since": 1234}),
        ),
    ];
    for (variant, expected) in cases {
        let wire = serde_json::to_value(&variant).unwrap();
        assert_eq!(wire, expected);
        let decoded: LoginStepped = serde_json::from_value(wire).unwrap();
        assert!(decoded == variant);
    }
}

#[tokio::test]
async fn rejected_and_foreign_credentials() {
    let (app, _) = engine_app(FakeLogin::answering(Err(LoginError::InvalidCredentials)));
    let vault = sealed_vault(&app, REG).await;
    let body = json!({"account": REG, "sealedVault": vault, "forwarding": true});
    assert!(done(step(&app, body).await) == LoginOutcome::InvalidCredentials);

    // Credentials sealed for one account do not open for another, and
    // ones that sign in someone else are not used.
    let (app, _) = engine_app(FakeLogin::default());
    let other = sealed_vault(&app, "23ABC4567").await;
    let body = json!({"account": REG, "sealedVault": other, "forwarding": true});
    assert!(done(step(&app, body).await) == LoginOutcome::VaultUnusable);
    let waiting = step(
        &app,
        json!({"account": "23ABC4567", "sealedVault": other, "forwarding": true}),
    )
    .await;
    let body =
        json!({"account": "23ABC4567", "sealedState": waiting["sealedState"], "otp": "123456"});
    assert!(done(step(&app, body).await) == LoginOutcome::VaultUnusable);
}

#[tokio::test]
async fn owner_check_names_the_owner_by_hash() {
    let (app, state) = engine_app(FakeLogin::default());
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
    let (_, state) = engine_app(FakeLogin::default());
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
    let (app, state) = engine_app(FakeLogin::default());
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
    login: Arc<FakeLogin>,
    gmail: Arc<FakeRefresher>,
    grant: Option<Value>,
    forwarding: bool,
) -> (Value, Arc<AppState>) {
    let (app, state) = engine_app_with(login, gmail, vtop_core::VtopConfig::default());
    let vault = match grant {
        Some(grant) => gmail_vault(&app, grant).await,
        None => sealed_vault(&app, REG).await,
    };
    let body = json!({"account": REG, "sealedVault": vault, "forwarding": forwarding});
    (step(&app, body).await, state)
}

#[tokio::test]
async fn without_forwarding_gmail_reads_the_otp() {
    let gmail = Arc::new(FakeRefresher::default());
    let login = Arc::new(FakeLogin::answering(Err(LoginError::InvalidCredentials)));
    let (outcome, _) = sign_in_once(
        login.clone(),
        gmail.clone(),
        Some(json!({"refreshToken": "r", "clientId": "c"})),
        false,
    )
    .await;
    assert!(done(outcome) == LoginOutcome::InvalidCredentials);
    assert_eq!(gmail.calls.load(Ordering::SeqCst), 1);
    assert_eq!(login.calls(), 1);
}

#[tokio::test]
async fn forwarding_comes_before_gmail() {
    let gmail = Arc::new(FakeRefresher::default());
    let (outcome, _) = sign_in_once(
        Arc::new(FakeLogin::default()),
        gmail.clone(),
        Some(json!({"refreshToken": "r", "clientId": "c"})),
        true,
    )
    .await;
    assert_eq!(outcome["status"], "otp_needed");
    assert_eq!(gmail.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn gmail_picked_on_the_phone_skips_forwarding() {
    let gmail = Arc::new(FakeRefresher::default());
    let login = Arc::new(FakeLogin::answering(Err(LoginError::InvalidCredentials)));
    let (app, state) = engine_app_with(
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
    let body = json!({"account": REG, "sealedVault": vault, "forwarding": true});
    assert!(done(step(&app, body).await) == LoginOutcome::InvalidCredentials);
    assert_eq!(gmail.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn revoked_gmail_access_is_dropped_and_the_password_kept() {
    let login = Arc::new(FakeLogin::default());
    let (outcome, state) = sign_in_once(
        login.clone(),
        Arc::new(FakeRefresher {
            revoked: true,
            ..FakeRefresher::default()
        }),
        Some(json!({"refreshToken": "r", "clientId": "c"})),
        false,
    )
    .await;
    let LoginOutcome::GmailRevoked { sealed_vault } = done(outcome) else {
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
    let (outcome, _) = sign_in_once(login.clone(), Arc::default(), None, false).await;
    assert!(done(outcome) == LoginOutcome::ForwardingRequired);
    assert_eq!(login.calls(), 0);
}

// ---- phone sign-in and fetches (`vtop_login`, `fetch`) ----------------

fn phone_app() -> axum::Router {
    engine_app(FakeLogin::answering(Err(LoginError::InvalidCredentials))).0
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

thread_local! {
    /// What this thread's log lines are written into, while a test reads them.
    static CAPTURED: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
}

/// Writes into this thread's `CAPTURED`, if it is capturing.
struct ThreadCapture;

impl std::io::Write for ThreadCapture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        CAPTURED.with_borrow_mut(|captured| {
            if let Some(buffer) = captured {
                buffer.extend_from_slice(bytes);
            }
        });
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The access line of one `/healthz` call with these headers, as logged.
///
/// One global subscriber rather than a scoped one per test: a scoped one
/// races the other tests' threads, which cache the access callsite as off
/// while no subscriber is set. The test runtime runs on this thread, so the
/// thread-local buffer gets exactly this call's lines.
async fn access_line(headers: &[(&str, &str)]) -> String {
    static GLOBAL: std::sync::Once = std::sync::Once::new();
    GLOBAL.call_once(|| {
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(|| ThreadCapture)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("no other test sets a global subscriber");
    });
    CAPTURED.with_borrow_mut(|captured| *captured = Some(Vec::new()));
    let (app, _) = engine_app(FakeLogin::default());
    let mut request = Request::builder().uri("/healthz");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = CAPTURED.with_borrow_mut(Option::take).unwrap_or_default();
    String::from_utf8(bytes).unwrap()
}

#[tokio::test]
async fn access_line_carries_the_callers_flow_and_regno() {
    let line = access_line(&[("x-vtop-flow", "sync-3f2a:1"), ("x-vtop-regno", REG)]).await;
    assert!(
        line.contains(&format!("request{{flow=sync-3f2a:1 regno={REG}}}")),
        "{line}"
    );
    assert!(line.contains("GET /healthz 200"), "{line}");
}

#[tokio::test]
async fn trace_headers_that_are_not_short_and_plain_are_left_out() {
    let long = "f".repeat(65);
    let line = access_line(&[("x-vtop-flow", &long), ("x-vtop-regno", "22BCE 0001\"")]).await;
    assert!(!line.contains("flow="), "{line}");
    assert!(!line.contains("regno="), "{line}");
    assert!(line.contains("GET /healthz 200"), "{line}");
}

#[test]
fn trace_value_takes_only_short_plain_values() {
    let value = |text: &str| axum::http::HeaderValue::from_str(text).unwrap();
    assert_eq!(
        crate::trace_value(Some(&value("a-b_c.d:1")), 64).as_deref(),
        Some("a-b_c.d:1")
    );
    assert_eq!(crate::trace_value(Some(&value("abc")), 2), None);
    for bad in ["", "a b", "a/b", "ünï", "x=1"] {
        let header = axum::http::HeaderValue::from_bytes(bad.as_bytes()).unwrap();
        assert_eq!(crate::trace_value(Some(&header), 64), None, "{bad}");
    }
    assert_eq!(crate::trace_value(None, 64), None);
}

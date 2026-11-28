use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use serde_json::json;
use tracing::field::{Field, Visit};
use tracing::span::Attributes;
use tracing::{Event, Id, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use super::*;
use crate::events::{SecretBox, SecretError, UrlOpener};
use crate::runtime::{Clock, ManualClock};
use crate::transport::{Foreign, HttpRequest, HttpResponse, HttpTransport, TransportError};

#[derive(Default)]
struct MemorySecrets(Mutex<HashMap<String, Vec<u8>>>);

impl SecretBox for MemorySecrets {
    fn seal(&self, name: String, bytes: Vec<u8>) -> Result<(), SecretError> {
        self.0.lock().unwrap().insert(name, bytes);
        Ok(())
    }

    fn open(&self, name: String) -> Result<Option<Vec<u8>>, SecretError> {
        Ok(self.0.lock().unwrap().get(&name).cloned())
    }
}

#[derive(Default)]
struct FakeHttp {
    seen: Mutex<Vec<HttpRequest>>,
    responses: Mutex<VecDeque<Result<HttpResponse, TransportError>>>,
}

impl FakeHttp {
    fn push_json(&self, status: u16, body: Value) {
        self.responses.lock().unwrap().push_back(Ok(HttpResponse {
            status,
            headers: Vec::new(),
            url: String::new(),
            body: body.to_string().into_bytes(),
            saved_size: None,
        }));
    }

    fn push_err(&self, err: TransportError) {
        self.responses.lock().unwrap().push_back(Err(err));
    }
}

#[async_trait::async_trait]
impl HttpTransport for FakeHttp {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
        self.seen.lock().unwrap().push(request);
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Err(TransportError::Other {
                    reason: "no canned response".into(),
                })
            })
    }
}

type OpenHook = Box<dyn Fn(&str) + Send + Sync>;

#[derive(Default)]
struct FakeUrls {
    opened: Mutex<Vec<String>>,
    on_open: Mutex<Option<OpenHook>>,
}

impl UrlOpener for FakeUrls {
    fn open(&self, url: String) {
        self.opened.lock().unwrap().push(url.clone());
        if let Some(cb) = self.on_open.lock().unwrap().as_ref() {
            cb(&url);
        }
    }
}

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<String>>);

impl Visit for Captured {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let _ = writeln!(self.0.lock().unwrap(), "{}={value:?}", field.name());
    }
}

impl<S: Subscriber> Layer<S> for Captured {
    fn on_new_span(&self, attrs: &Attributes<'_>, _id: &Id, _ctx: Context<'_, S>) {
        attrs.record(&mut self.clone());
    }
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        event.record(&mut self.clone());
    }
}

fn setup(
    name: &str,
) -> (
    Core,
    ManualClock,
    Arc<MemorySecrets>,
    Arc<FakeHttp>,
    Arc<FakeUrls>,
) {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-gmail-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let clock = ManualClock::default();
    clock.advance(1_763_700_000_000);
    let secrets = Arc::new(MemorySecrets::default());
    let http = Arc::new(FakeHttp::default());
    let urls = Arc::new(FakeUrls::default());
    let shared = Shared {
        http: Some(Arc::new(Foreign(http.clone()))),
        secrets: Some(secrets.clone()),
        urls: Some(urls.clone()),
        ..Shared::default()
    };
    let core = Core::for_test_with_clock(dir, shared, Arc::new(clock.clone()));
    (core, clock, secrets, http, urls)
}

fn sample_sealed_json(delete_after: bool, expires_at_ms: i64) -> Vec<u8> {
    json!({
        "email": "student.22bce0001@vitapstudent.ac.in",
        "client_id": "1234567890-abcd.apps.googleusercontent.com",
        "client_secret": "GOCSPX-secret-value",
        "project": "vitap-mate",
        "refresh_token": "1//refresh-secret-tok",
        "access_token": "ya29.access-secret-tok",
        "expires_at_ms": expires_at_ms,
        "delete_after_reading": delete_after,
        "grant_ends_at_ms": 0,
        "ended": false,
    })
    .to_string()
    .into_bytes()
}

fn gmail_message_json(id: &str, internal_date_ms: i64, code: &str) -> Value {
    let body = URL_SAFE_NO_PAD.encode(format!("Your OTP is {code}"));
    json!({
        "id": id,
        "internalDate": internal_date_ms.to_string(),
        "snippet": "",
        "payload": {
            "headers": [{ "name": "From", "value": "VTOP <noreply.sdc@vitap.ac.in>" }],
            "body": { "size": 18, "data": body }
        }
    })
}

fn fake_id_token(email: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256"}"#);
    let payload = URL_SAFE_NO_PAD.encode(json!({ "email": email }).to_string());
    format!("{header}.{payload}.sig")
}

#[test]
fn existing_sealed_token_reads_unchanged() {
    let (core, _clock, secrets, _http, _urls) = setup("read-existing");
    let raw = sample_sealed_json(false, 1_763_800_000_000);
    secrets.seal(SECRET_NAME.into(), raw).unwrap();

    let account = core.gmail_account("f".into()).expect("account loaded");
    assert_eq!(account.email, "student.22bce0001@vitapstudent.ac.in");
    assert_eq!(
        account.client.id,
        "1234567890-abcd.apps.googleusercontent.com"
    );
    assert_eq!(
        account.client.secret.as_deref(),
        Some("GOCSPX-secret-value")
    );
    assert_eq!(account.client.project, "vitap-mate");
    assert_eq!(account.refresh_token, "1//refresh-secret-tok");
    assert!(!account.delete_after_reading);
    assert_eq!(account.grant_ends_at_ms, 0);
    assert!(!account.ended);
}

#[tokio::test]
async fn refreshes_when_expired_and_resaves() {
    let (core, clock, secrets, http, _urls) = setup("refresh");
    // Access token already expired relative to clock (1_763_700_000_000).
    let expired_at = clock.now_ms() - 10_000;
    secrets
        .seal(SECRET_NAME.into(), sample_sealed_json(true, expired_at))
        .unwrap();

    // 1) Token refresh, 2) message list (empty).
    http.push_json(
        200,
        json!({ "access_token": "ya29.fresh-token", "expires_in": 3600 }),
    );
    http.push_json(200, json!({ "messages": [] }));

    let otp = core
        .gmail_find_otp("f".into(), clock.now_ms(), vec![])
        .await
        .unwrap();
    assert!(otp.is_none());

    let seen = http.seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].url, TOKEN);
    assert!(seen[1].url.starts_with(&format!("{API}/messages?")));
    assert!(
        seen[1]
            .headers
            .iter()
            .any(|h| h.name == "Authorization" && h.value == "Bearer ya29.fresh-token")
    );

    // Re-saved into SecretBox with new access_token and updated expires_at_ms.
    let saved_bytes = secrets.open(SECRET_NAME.into()).unwrap().unwrap();
    let saved: StoredGmailAccount = serde_json::from_slice(&saved_bytes).unwrap();
    assert_eq!(saved.access_token, "ya29.fresh-token");
    assert_eq!(saved.expires_at_ms, clock.now_ms() + 3_600_000);
}

#[tokio::test]
async fn refused_refresh_reports_gmail_refused_code() {
    let (core, clock, secrets, http, _urls) = setup("refused-refresh");
    secrets
        .seal(SECRET_NAME.into(), sample_sealed_json(true, 0))
        .unwrap();
    http.push_json(400, json!({ "error": "invalid_grant" }));

    let err = core
        .gmail_find_otp("f".into(), clock.now_ms(), vec![])
        .await
        .unwrap_err();
    assert!(matches!(err, GmailError::Ended { .. }), "{err:?}");
    assert_eq!(err.id(), "ended");
    assert!(core.gmail_account("f".into()).unwrap().ended);

    // Also verify invalid_client reports "invalid_client".
    let (core2, clock2, secrets2, http2, _urls2) = setup("refused-client");
    secrets2
        .seal(SECRET_NAME.into(), sample_sealed_json(true, 0))
        .unwrap();
    http2.push_json(401, json!({ "error": "invalid_client" }));
    let err2 = core2
        .gmail_find_otp("f".into(), clock2.now_ms(), vec![])
        .await
        .unwrap_err();
    assert_eq!(err2.id(), "unauthorized");

    let (core3, clock3, secrets3, http3, _urls3) = setup("refused-client-400");
    secrets3
        .seal(SECRET_NAME.into(), sample_sealed_json(true, 0))
        .unwrap();
    http3.push_json(400, json!({ "error": "invalid_client" }));
    let err3 = core3
        .gmail_find_otp("f".into(), clock3.now_ms(), vec![])
        .await
        .unwrap_err();
    assert_eq!(err3.id(), "invalid_client");
}

#[tokio::test]
async fn find_otp_skips_tried_and_older_than_since() {
    let (core, clock, secrets, http, _urls) = setup("find-otp");
    let now_ms = clock.now_ms();
    secrets
        .seal(
            SECRET_NAME.into(),
            sample_sealed_json(true, now_ms + 3_600_000),
        )
        .unwrap();

    // List returns: m1 (already tried), m2 (older than since - 30s), m3 (fresh valid OTP).
    http.push_json(
        200,
        json!({ "messages": [{ "id": "m1" }, { "id": "m2" }, { "id": "m3" }] }),
    );
    http.push_json(200, gmail_message_json("m2", now_ms - 120_000, "111111"));
    http.push_json(200, gmail_message_json("m3", now_ms + 5_000, "654321"));

    let found = core
        .gmail_find_otp("f".into(), now_ms, vec!["m1".into()])
        .await
        .unwrap()
        .expect("should find m3");
    assert_eq!(found.message_id, "m3");
    assert_eq!(found.code, "654321");

    // Verify m1 was never fetched.
    let seen = http.seen.lock().unwrap();
    assert_eq!(seen.len(), 3);
    assert!(!seen.iter().any(|r| r.url.contains("/messages/m1")));
}

#[tokio::test]
async fn redirect_with_wrong_state_is_rejected() {
    let (core, _clock, _secrets, _http, urls) = setup("wrong-state");
    *urls.on_open.lock().unwrap() = Some(Box::new(|url: &str| {
        // Parse redirect_uri=http://127.0.0.1:<port> from the auth URL and send wrong state.
        let query = url.split_once('?').unwrap().1;
        let redirect = query
            .split('&')
            .find_map(|p| p.strip_prefix("redirect_uri="))
            .unwrap();
        let decoded = percent_decode(redirect);
        let addr = decoded.strip_prefix("http://").unwrap().to_owned();
        std::thread::spawn(move || {
            let mut stream = TcpStream::connect(&addr).unwrap();
            stream
                .write_all(
                    b"GET /?code=abc123&state=wrong-state HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
                )
                .unwrap();
            let mut buf = String::new();
            let _ = stream.read_to_string(&mut buf);
        });
    }));

    let client = GmailClient {
        id: "1234567890-abcd.apps.googleusercontent.com".into(),
        secret: Some("sec".into()),
        project: "proj".into(),
    };
    let err = core
        .gmail_connect("f".into(), client, "22BCE0001".into())
        .await
        .unwrap_err();
    assert!(matches!(err, GmailError::BadState { .. }), "{err:?}");
}

#[tokio::test]
async fn tidy_up_deletes_only_when_on() {
    let (core, clock, secrets, http, _urls) = setup("tidy-on");
    let now_ms = clock.now_ms();
    secrets
        .seal(
            SECRET_NAME.into(),
            sample_sealed_json(true, now_ms + 3_600_000),
        )
        .unwrap();
    http.push_json(200, json!({}));
    core.gmail_tidy_up(
        "f".into(),
        GmailOtp {
            code: "123456".into(),
            message_id: "msg-1".into(),
        },
    )
    .await;
    {
        let seen = http.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].url.ends_with("/messages/msg-1/trash"));
        assert!(seen[0].body.is_empty());
    }

    // Turn delete_after_reading off -> modifies UNREAD label instead of trashing.
    let updated = core
        .gmail_set_delete_after_reading("f".into(), false)
        .await
        .unwrap();
    assert!(!updated.delete_after_reading);
    http.push_json(200, json!({}));
    core.gmail_tidy_up(
        "f".into(),
        GmailOtp {
            code: "123456".into(),
            message_id: "msg-2".into(),
        },
    )
    .await;
    let seen = http.seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert!(seen[1].url.ends_with("/messages/msg-2/modify"));
    assert_eq!(
        String::from_utf8_lossy(&seen[1].body),
        r#"{"removeLabelIds":["UNREAD"]}"#
    );
}

#[tokio::test]
async fn never_logs_token_or_code() {
    let captured = Captured::default();
    let subscriber = tracing_subscriber::registry().with(captured.clone());
    let _guard = tracing::subscriber::set_default(subscriber);

    let (core, clock, secrets, http, urls) = setup("no-secrets-in-logs");
    let secret_code = "987654";
    let secret_refresh = "1//super-secret-refresh-token";
    let secret_access = "ya29.super-secret-access-token";
    let secret_client = "GOCSPX-super-secret-client";
    let secret_email = "alice.22bce0001@vitapstudent.ac.in";
    let secret_auth_code = "4/0A-super-secret-auth-code";

    // Full connect flow + find_otp + tidy_up + disconnect.
    *urls.on_open.lock().unwrap() = Some(Box::new(move |url: &str| {
        let query = url.split_once('?').unwrap().1;
        let redirect = query
            .split('&')
            .find_map(|p| p.strip_prefix("redirect_uri="))
            .unwrap();
        let state = query
            .split('&')
            .find_map(|p| p.strip_prefix("state="))
            .unwrap();
        let addr = percent_decode(redirect)
            .strip_prefix("http://")
            .unwrap()
            .to_owned();
        let state = percent_decode(state);
        std::thread::spawn(move || {
            let mut stream = TcpStream::connect(&addr).unwrap();
            let req = format!(
                "GET /?code={secret_auth_code}&state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
            );
            stream.write_all(req.as_bytes()).unwrap();
            let mut buf = String::new();
            let _ = stream.read_to_string(&mut buf);
        });
    }));

    // 1) token exchange, 2) verify profile, 3) list messages, 4) get message, 5) trash, 6) revoke.
    http.push_json(
        200,
        json!({
            "access_token": secret_access,
            "refresh_token": secret_refresh,
            "expires_in": 3600,
            "scope": format!("openid email {SCOPE}"),
            "id_token": fake_id_token(secret_email),
        }),
    );
    http.push_json(200, json!({ "emailAddress": secret_email }));
    http.push_json(200, json!({ "messages": [{ "id": "m99" }] }));
    http.push_json(
        200,
        gmail_message_json("m99", clock.now_ms() + 1_000, secret_code),
    );
    http.push_json(200, json!({}));
    http.push_json(200, json!({}));

    let client = GmailClient {
        id: "1234567890-abcd.apps.googleusercontent.com".into(),
        secret: Some(secret_client.into()),
        project: "vitap-mate".into(),
    };
    let connected = core
        .gmail_connect("f-conn".into(), client, "22BCE0001".into())
        .await
        .unwrap();
    assert_eq!(connected.email, secret_email);
    let otp = core
        .gmail_find_otp("f-otp".into(), clock.now_ms(), vec![])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(otp.code, secret_code);
    core.gmail_tidy_up("f-tidy".into(), otp).await;
    core.gmail_disconnect("f-disc".into()).await;
    assert!(core.gmail_account("f-check".into()).is_none());
    assert!(
        secrets
            .open(SECRET_NAME.into())
            .unwrap()
            .unwrap_or_default()
            .is_empty()
    );

    let logs = captured.0.lock().unwrap();
    for secret in [
        secret_code,
        secret_refresh,
        secret_access,
        secret_client,
        secret_email,
        secret_auth_code,
    ] {
        assert!(
            !logs.contains(secret),
            "secret leaked in logs ({secret}): {logs}"
        );
    }
}

#[test]
fn parse_client_validates_google_cloud_json() {
    assert!(matches!(
        gmail_parse_client(vec![]),
        Err(GmailError::BadFile { .. })
    ));
    assert!(matches!(
        gmail_parse_client(b"not json".to_vec()),
        Err(GmailError::BadJson { .. })
    ));
    assert!(matches!(
        gmail_parse_client(br#"{"web":{"client_id":"123.apps.googleusercontent.com"}}"#.to_vec()),
        Err(GmailError::WrongClientType { .. })
    ));
    assert!(matches!(
        gmail_parse_client(br#"{"installed":{"client_id":"bad"}}"#.to_vec()),
        Err(GmailError::BadClientId { .. })
    ));
    let ok = gmail_parse_client(
        br#"{"installed":{"client_id":"1234567890-abcd.apps.googleusercontent.com","client_secret":"sec","project_id":"my-proj"}}"#
            .to_vec(),
    )
    .unwrap();
    assert_eq!(ok.id, "1234567890-abcd.apps.googleusercontent.com");
    assert_eq!(ok.secret.as_deref(), Some("sec"));
    assert_eq!(ok.project, "my-proj");
}

#[test]
fn unreachable_exchange_retries_until_reachable() {
    let (core, clock, _secrets, http, _urls) = setup("retry-unreachable");
    core.shared.gmail().reachable_window_ms_override = Some(60_000);
    http.push_err(TransportError::Connect {
        reason: "dns".into(),
    });
    http.push_json(200, json!({ "ok": true }));

    let mut fut = Box::pin(core.gmail_until_reachable("f", || {
        core.gmail_post(TOKEN, "f", &[("grant_type", Some("authorization_code"))])
    }));
    assert!(smol::future::block_on(smol::future::poll_once(&mut fut)).is_none());
    clock.advance(RETRY_BASE_MS);
    let res = smol::future::block_on(fut).unwrap();
    assert_eq!(res["ok"], true);
}

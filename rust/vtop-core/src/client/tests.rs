//! Session reuse tests against a tiny in-process stand-in for VTOP.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;
use crate::inputs::{OutingPassId, SemesterId};

const CONTENT_PAGE: &str = r#"<html><input name="_csrf" value="csrf-1">
<input type="hidden" name="authorizedIDX" value="22BCE0001"></html>"#;
const ATTENDANCE_PAGE: &str = r#"<table><tr><th>h</th></tr>
<tr><td>1</td><td>PC</td><td>Maths</td><td>MAT1001</td><td>Dr X</td><td>10</td><td>12</td><td>83%</td><td>0</td><td>-</td>
<td><a onclick="javascript:processViewAttendanceDetail('AP2026','22BCE0001','AP2026001','ETH');">view</a></td></tr></table>"#;

/// A general outing list with one request VTOP still lets be cancelled.
const OUTING_PAGE: &str = r#"<table id="BookingRequests"><thead><tr>
<th>S.No</th><th>Registration Number</th><th>Place Of Visit</th><th>Purpose Of Visit</th>
<th>From Date</th><th>From Time</th><th>To Date</th><th>To Time</th><th>Action</th>
<th>Status</th><th>Download OutPass</th></tr></thead><tbody>
<tr><td>1</td><td>22BCE0001</td><td>Home</td><td>Holidays</td><td>2026-10-17 00:00:00.0</td>
<td>11:30 AM</td><td>2026-10-21 00:00:00.0</td><td>08:00 PM</td>
<td><button onclick="deleteStudentBookingInfo(this.getAttribute('data-bookingId'));"
data-bookingId="L1">x</button></td><td>Waiting</td><td></td></tr></tbody></table>"#;

/// Counts requests per path and answers with canned pages.
#[derive(Clone, Default)]
struct MockVtop {
    hits: Arc<Mutex<HashMap<String, usize>>>,
    /// Paths that bounce to the login page, as VTOP does for a dead session.
    expired: Arc<Mutex<bool>>,
    /// How many upcoming requests get an HTTP 503 before normal answers.
    failures_left: Arc<Mutex<usize>>,
    /// Signed-in pages answer 404, as VTOP does for some dead sessions.
    not_found: Arc<Mutex<bool>>,
    /// A path that always answers HTTP 503.
    down: Arc<Mutex<Option<&'static str>>>,
    /// Every request answers HTTP 504, as the edge does when VTOP is slow.
    gateway_timeout: Arc<Mutex<bool>>,
}

impl MockVtop {
    fn hits(&self, path: &str) -> usize {
        self.hits.lock().unwrap().get(path).copied().unwrap_or(0)
    }

    async fn start(&self) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mock = self.clone();
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let mock = mock.clone();
                tokio::spawn(async move {
                    let mut buffer = vec![0u8; 16 * 1024];
                    loop {
                        let Ok(read) = socket.read(&mut buffer).await else {
                            return;
                        };
                        if read == 0 {
                            return;
                        }
                        let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                        let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
                        *mock.hits.lock().unwrap().entry(path.clone()).or_default() += 1;
                        let cookie = request
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("cookie")
                                    .then(|| value.trim().to_string())
                            })
                            .unwrap_or_default();
                        let response = if path == "/echo-cookie" {
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{cookie}",
                                cookie.len()
                            )
                        } else if path == "/set-then-redirect" {
                            "HTTP/1.1 302 Found\r\nSet-Cookie: SERVERID=s9; Path=/\r\nLocation: /echo-cookie\r\nContent-Length: 0\r\n\r\n".to_string()
                        } else {
                            mock.respond(&path)
                        };
                        if socket.write_all(response.as_bytes()).await.is_err() {
                            return;
                        }
                    }
                });
            }
        });
        format!("http://{address}")
    }

    fn respond(&self, path: &str) -> String {
        {
            let mut failures = self.failures_left.lock().unwrap();
            if *failures > 0 {
                *failures -= 1;
                return "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n".to_string();
            }
        }
        if *self.gateway_timeout.lock().unwrap() {
            return "HTTP/1.1 504 Gateway Timeout\r\nContent-Length: 0\r\n\r\n".to_string();
        }
        if *self.down.lock().unwrap() == Some(path) {
            return "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n".to_string();
        }
        if *self.not_found.lock().unwrap() && path == "/vtop/processViewStudentAttendance" {
            return "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string();
        }
        if *self.expired.lock().unwrap() && path != "/vtop/login" {
            return "HTTP/1.1 302 Found\r\nLocation: /vtop/login\r\nContent-Length: 0\r\n\r\n"
                .to_string();
        }
        let body = match path {
            "/vtop/content" => CONTENT_PAGE,
            "/vtop/processViewStudentAttendance" => ATTENDANCE_PAGE,
            "/vtop/hostel/StudentGeneralOuting" => OUTING_PAGE,
            _ => "<html>login</html>",
        };
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
    }
}

fn config(base_url: String) -> VtopConfig {
    VtopConfig {
        base_url,
        ..VtopConfig::default()
    }
}

fn semester() -> SemesterId {
    SemesterId::parse("AP2026").unwrap()
}

#[tokio::test]
async fn full_session_state_skips_validation() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let state = SessionState {
        cookies: "JSESSIONID=abc".into(),
        csrf_token: Some("csrf-1".into()),
        registration_number: Some("22BCE0001".into()),
        otp_issued_at: None,
        logged_in_at: None,
    };
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&state)
        .unwrap();

    let data = client.attendance(&semester()).await.unwrap();

    assert_eq!(data.records.len(), 1);
    assert_eq!(data.records[0].course_id, "AP2026001");
    assert_eq!(data.records[0].course_type, "ETH");
    assert_eq!(mock.hits("/vtop/content"), 0);
    assert_eq!(mock.hits("/vtop/processViewStudentAttendance"), 1);
}

#[tokio::test]
async fn cookie_only_session_validates_once_then_reuses() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let state = SessionState {
        cookies: "JSESSIONID=abc".into(),
        csrf_token: None,
        registration_number: None,
        otp_issued_at: None,
        logged_in_at: None,
    };
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&state)
        .unwrap();

    client.attendance(&semester()).await.unwrap();
    client.attendance(&semester()).await.unwrap();

    assert_eq!(mock.hits("/vtop/content"), 1);
    assert_eq!(mock.hits("/vtop/processViewStudentAttendance"), 2);
    let exported = client.session_state();
    assert_eq!(exported.csrf_token.as_deref(), Some("csrf-1"));
    assert_eq!(exported.registration_number.as_deref(), Some("22BCE0001"));
}

#[tokio::test]
async fn concurrent_fetches_share_one_validation() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&SessionState {
            cookies: "JSESSIONID=abc".into(),
            csrf_token: None,
            registration_number: None,
            otp_issued_at: None,
            logged_in_at: None,
        })
        .unwrap();

    let semester = semester();
    let (a, b, c) = tokio::join!(
        client.attendance(&semester),
        client.attendance(&semester),
        client.attendance(&semester)
    );
    assert!(a.is_ok() && b.is_ok() && c.is_ok());
    assert_eq!(mock.hits("/vtop/content"), 1);
}

#[tokio::test]
async fn login_redirect_expires_the_session() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&SessionState {
            cookies: "JSESSIONID=abc".into(),
            csrf_token: Some("csrf-1".into()),
            registration_number: Some("22BCE0001".into()),
            otp_issued_at: None,
            logged_in_at: None,
        })
        .unwrap();
    *mock.expired.lock().unwrap() = true;

    let result = client.attendance(&semester()).await;

    assert_eq!(result.unwrap_err(), VtopError::SessionExpired);
    assert!(!client.is_authenticated());
    assert_eq!(client.cookie_header(), None);
}

#[tokio::test]
async fn persisted_session_round_trips_cookies() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = VtopClient::builder()
        .config(config(base.clone()))
        .with_credentials(
            Username::parse("login-name").unwrap(),
            Password::parse("password").unwrap(),
        )
        .unwrap();
    client
        .restore_persisted_session(&PersistedVtopSession {
            username: "LOGIN-NAME".into(),
            saved_at_epoch_ms: 0,
            cookies: Some("JSESSIONID=abc; SERVERID=s1".into()),
            csrf_token: None,
            registration_number: None,
            logged_in_at: None,
        })
        .unwrap();

    let exported = client.export_persisted_session(42);

    assert_eq!(exported.username, "LOGIN-NAME");
    assert_eq!(exported.saved_at_epoch_ms, 42);
    // The jar does not keep cookie order.
    let mut cookies: Vec<_> = exported
        .cookies
        .as_deref()
        .unwrap_or_default()
        .split("; ")
        .collect();
    cookies.sort_unstable();
    assert_eq!(cookies, ["JSESSIONID=abc", "SERVERID=s1"]);
    client.attendance(&semester()).await.unwrap();
    assert_eq!(mock.hits("/vtop/content"), 1);
}

#[test]
fn registration_number_does_not_replace_login_username() {
    let client = VtopClient::builder()
        .with_credentials(
            Username::parse("login-name").unwrap(),
            Password::parse("password").unwrap(),
        )
        .unwrap();
    assert!(client.registration_number().is_err());

    client.set_registration_number_for_test("22BCE0001");

    assert_eq!(client.export_persisted_session(0).username, "LOGIN-NAME");
    assert_eq!(client.registration_number().unwrap(), "22BCE0001");
}

#[test]
fn clients_without_credentials_cannot_log_in() {
    let mut client = VtopClient::builder()
        .with_session(&SessionState {
            cookies: String::new(),
            csrf_token: None,
            registration_number: None,
            otp_issued_at: None,
            logged_in_at: None,
        })
        .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = runtime.block_on(client.login());
    assert!(matches!(result, Err(VtopError::ConfigurationError(_))));
}

fn full_state(logged_in_at: Option<u64>) -> SessionState {
    SessionState {
        cookies: "JSESSIONID=abc".into(),
        csrf_token: Some("csrf-1".into()),
        registration_number: Some("22BCE0001".into()),
        otp_issued_at: None,
        logged_in_at,
    }
}

#[tokio::test]
async fn a_404_on_a_signed_in_page_expires_the_session() {
    // A session VTOP dropped can get 404 instead of the login page; callers
    // then sign in again (the app) or try the next session (the server).
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&full_state(None))
        .unwrap();
    *mock.not_found.lock().unwrap() = true;

    let result = client.attendance(&semester()).await;

    assert_eq!(result.unwrap_err(), VtopError::SessionExpired);
}

#[tokio::test]
async fn a_single_vtop_5xx_is_retried() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&full_state(None))
        .unwrap();
    *mock.failures_left.lock().unwrap() = 1;

    client.attendance(&semester()).await.unwrap();

    assert_eq!(mock.hits("/vtop/processViewStudentAttendance"), 2);
}

#[tokio::test]
async fn a_gateway_timeout_is_not_retried_and_keeps_the_session() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&full_state(None))
        .unwrap();
    *mock.gateway_timeout.lock().unwrap() = true;

    let error = client.attendance(&semester()).await.unwrap_err();

    assert!(matches!(error, VtopError::VtopUnavailable(ref m) if m.contains("too long")));
    assert_eq!(mock.hits("/vtop/processViewStudentAttendance"), 1);
    assert!(client.is_authenticated());
}

#[tokio::test]
async fn persisted_trio_restores_without_validation_and_keeps_login_time() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = VtopClient::builder()
        .config(config(base))
        .with_credentials(
            Username::parse("login-name").unwrap(),
            Password::parse("password").unwrap(),
        )
        .unwrap();
    client
        .restore_persisted_session(&PersistedVtopSession {
            username: "LOGIN-NAME".into(),
            saved_at_epoch_ms: 0,
            cookies: Some("JSESSIONID=abc".into()),
            csrf_token: Some("csrf-1".into()),
            registration_number: Some("22BCE0001".into()),
            logged_in_at: Some(1_700_000_000),
        })
        .unwrap();

    assert!(client.is_authenticated());
    client.attendance(&semester()).await.unwrap();
    assert_eq!(mock.hits("/vtop/content"), 0);

    let saved = client.export_persisted_session(5);
    assert_eq!(saved.csrf_token.as_deref(), Some("csrf-1"));
    assert_eq!(saved.registration_number.as_deref(), Some("22BCE0001"));
    assert_eq!(saved.logged_in_at, Some(1_700_000_000));
}

#[tokio::test]
async fn validating_a_cookie_does_not_invent_a_login_time() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&SessionState {
            cookies: "JSESSIONID=abc".into(),
            csrf_token: None,
            registration_number: None,
            otp_issued_at: None,
            logged_in_at: None,
        })
        .unwrap();

    client.attendance(&semester()).await.unwrap();

    assert!(client.is_authenticated());
    assert_eq!(client.logged_in_at(), None);
}

#[tokio::test]
async fn resume_replaces_the_previous_session() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&full_state(Some(1)))
        .unwrap();

    client
        .resume_session(&SessionState {
            cookies: "JSESSIONID=new".into(),
            csrf_token: Some("csrf-2".into()),
            registration_number: Some("22BCE0001".into()),
            otp_issued_at: None,
            logged_in_at: Some(2),
        })
        .unwrap();

    let state = client.session_state();
    assert_eq!(state.cookies, "JSESSIONID=new");
    assert_eq!(state.csrf_token.as_deref(), Some("csrf-2"));
    assert_eq!(state.logged_in_at, Some(2));
}

fn cookie_client(base: &str, cookies: &str) -> VtopClient {
    VtopClient::builder()
        .config(config(base.to_string()))
        .with_session(&SessionState {
            cookies: cookies.into(),
            csrf_token: None,
            registration_number: None,
            otp_issued_at: None,
            logged_in_at: None,
        })
        .unwrap()
}

#[tokio::test]
async fn sessions_on_the_shared_pool_keep_their_own_cookies() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let alice = cookie_client(&base, "JSESSIONID=alice");
    let bob = cookie_client(&base, "JSESSIONID=bob");
    let url = format!("{base}/echo-cookie");

    for _ in 0..3 {
        let (a, b) = tokio::join!(
            alice.send_request(alice.http.get(&url)),
            bob.send_request(bob.http.get(&url)),
        );
        assert_eq!(a.unwrap().text().await.unwrap(), "JSESSIONID=alice");
        assert_eq!(b.unwrap().text().await.unwrap(), "JSESSIONID=bob");
    }
}

#[tokio::test]
async fn cookies_set_during_a_redirect_are_kept() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = cookie_client(&base, "JSESSIONID=abc");

    let response = client
        .send_request(client.http.get(format!("{base}/set-then-redirect")))
        .await
        .unwrap();

    assert!(response.url().path().ends_with("/echo-cookie"));
    let sent = response.text().await.unwrap();
    assert!(
        sent.contains("JSESSIONID=abc") && sent.contains("SERVERID=s9"),
        "{sent}"
    );
    assert!(client.cookie_header().unwrap().contains("SERVERID=s9"));
}

fn cookie_session(cookies: &str) -> SessionState {
    SessionState {
        cookies: cookies.into(),
        csrf_token: None,
        registration_number: None,
        otp_issued_at: None,
        logged_in_at: None,
    }
}

#[tokio::test]
async fn verify_session_returns_registration_number() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&cookie_session("JSESSIONID=abc"))
        .unwrap();

    assert_eq!(client.verify_session().await.unwrap(), "22BCE0001");
    assert_eq!(mock.hits("/vtop/content"), 1);
}

#[tokio::test]
async fn verify_session_ignores_a_claimed_registration_number() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&SessionState {
            registration_number: Some("21BCE9999".into()),
            csrf_token: Some("forged".into()),
            ..cookie_session("JSESSIONID=abc")
        })
        .unwrap();

    assert_eq!(client.verify_session().await.unwrap(), "22BCE0001");
    assert_eq!(mock.hits("/vtop/content"), 1);
}

#[tokio::test]
async fn verify_session_on_server_error_is_not_expired() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    *mock.failures_left.lock().unwrap() = 100;
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&cookie_session("JSESSIONID=abc"))
        .unwrap();

    // A 503 is VTOP being down, not the session.
    assert_eq!(
        client.verify_session().await.unwrap_err(),
        VtopError::VtopUnavailable("VTOP is down right now".into())
    );
}

#[tokio::test]
async fn verify_session_on_login_redirect_is_expired() {
    let mock = MockVtop::default();
    let base = mock.start().await;
    *mock.expired.lock().unwrap() = true;
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&cookie_session("JSESSIONID=abc"))
        .unwrap();

    assert_eq!(
        client.verify_session().await.unwrap_err(),
        VtopError::SessionExpired
    );
}

#[tokio::test]
async fn outing_actions_are_sent_once() {
    // A read that gets a 5xx is asked again; a post that changes VTOP is
    // not, since VTOP may have acted on it before failing.
    let mock = MockVtop::default();
    let base = mock.start().await;
    let client = VtopClient::builder()
        .config(config(base))
        .with_session(&full_state(None))
        .unwrap();
    *mock.down.lock().unwrap() = Some("/vtop/hostel/deleteGeneralOutingInfo");

    let result = client
        .cancel_general_outing(&OutingPassId::parse("L1").unwrap())
        .await;

    assert!(result.is_err());
    assert_eq!(mock.hits("/vtop/hostel/StudentGeneralOuting"), 1);
    assert_eq!(mock.hits("/vtop/hostel/deleteGeneralOutingInfo"), 1);
}

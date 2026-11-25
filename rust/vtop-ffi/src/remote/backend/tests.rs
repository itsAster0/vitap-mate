use std::fs;
use std::sync::{Arc, Mutex};

use super::auth::GrantKind;
use super::*;
use crate::test_edge::{Edge, REFRESH, keep_token, paths, serve, signed_in};

async fn setup(name: &str) -> (Core, Arc<Mutex<Edge>>, String) {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-backend-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    (signed_in(&dir, server.clone()), edge, server)
}

fn can(edge: &Arc<Mutex<Edge>>, path: &str, status: u16, body: Value) {
    let mut edge = edge.lock().unwrap();
    edge.canned
        .entry(path.into())
        .or_default()
        .push_back((status, body));
}

fn body(edge: &Arc<Mutex<Edge>>, index: usize) -> Value {
    edge.lock().unwrap().requests[index].body.clone()
}

async fn view(core: &Core, flow: &str, ask: bool) -> Result<Result<Value, EdgeError>, CoreError> {
    let args = json!({});
    core.account_call(flow, Kind::Query, "account:view", "Load", ask, args)
        .await
}

#[tokio::test]
async fn a_kept_refresh_token_gets_a_jwt_once_and_is_replaced() {
    let (core, edge, server) = setup("refresh").await;
    keep_token(&core, &server);
    can(&edge, "account:view", 200, json!({ "thisPhone": true }));
    can(&edge, "account:view", 200, json!({ "thisPhone": true }));
    view(&core, "f1", false).await.unwrap().unwrap();
    view(&core, "f2", false).await.unwrap().unwrap();
    assert_eq!(paths(&edge), ["auth:token", "account:view", "account:view"]);
    assert_eq!(body(&edge, 0), json!({ "refreshToken": REFRESH }));
    let edge = edge.lock().unwrap();
    assert!(
        edge.requests.iter().all(|r| r.signed),
        "signed, then with the JWT"
    );
    // A query carries no flow, so the same read is the same query.
    assert_eq!(edge.requests[1].flow, "");
    assert_eq!(
        edge.requests[1].body,
        json!({}),
        "no regno, no token in the args"
    );
    assert_eq!(edge.auth.as_deref(), Some("jwt-1"));
    let kept = core.device().refresh.unwrap();
    assert_eq!(
        (kept.token.as_str(), kept.regno.as_str()),
        ("vrt_1", "22BCE0001")
    );
}

#[tokio::test]
async fn a_refused_refresh_token_is_dropped_and_nothing_proves_unasked() {
    let (core, edge, server) = setup("reused").await;
    keep_token(&core, &server);
    can(
        &edge,
        "auth:token",
        401,
        json!({ "code": "refresh_reused" }),
    );
    assert!(matches!(
        view(&core, "f", false).await,
        Err(CoreError::ConsentDenied)
    ));
    assert!(core.device().refresh.is_none());
    assert_eq!(paths(&edge), ["auth:token"]);
}

#[tokio::test]
async fn another_students_refresh_token_is_never_sent() {
    let (core, edge, server) = setup("other").await;
    core.keep_refresh(Some(Refresh {
        server,
        token: "vrt_theirs".into(),
        regno: "22BCE9999".into(),
    }));
    assert!(matches!(
        view(&core, "f", false).await,
        Err(CoreError::ConsentDenied)
    ));
    assert!(paths(&edge).is_empty());
}

#[tokio::test]
async fn a_v2_account_token_is_traded_in_once() {
    let (core, edge, _server) = setup("v2").await;
    let saved = account_token::Saved {
        server: "https://vmate.aster0.dev".into(),
        registration_number: "22BCE0001".into(),
        token: "v2tok".into(),
        expires_at: 0,
    };
    core.write_plain(account_token::FILE, &saved);
    can(&edge, "account:view", 200, json!({}));
    view(&core, "f", false).await.unwrap().unwrap();
    let sent = body(&edge, 0);
    assert_eq!(sent["v2AccountToken"], "v2tok");
    assert_eq!(sent["device"]["platform"], "android");
    assert_eq!(sent["device"]["sealKey"], json!(core.device().seal_key));
    assert!(!core.at(account_token::FILE).exists());
    assert!(core.device().refresh.is_some());
}

#[tokio::test]
async fn the_demo_account_asks_for_a_demo_jwt() {
    let (core, edge, _server) = setup("demo").await;
    core.current().unwrap().write().await.account.username = crate::demo::USERNAME.into();
    // The session's own regno wins; the demo's has none.
    {
        let slot = core.current().unwrap();
        let session = slot.write().await;
        let state = vtop_core::SessionState {
            cookies: String::new(),
            csrf_token: None,
            registration_number: None,
            otp_issued_at: None,
            logged_in_at: None,
        };
        session.client.resume_session(&state).unwrap();
    }
    can(&edge, "account:view", 200, json!({}));
    view(&core, "f", false).await.unwrap().unwrap();
    assert_eq!(body(&edge, 0)["demo"], true);
    assert_eq!(core.device().refresh.unwrap().regno, "DEMO2026");
}

#[tokio::test]
async fn a_refused_jwt_is_renewed_once() {
    let (core, edge, server) = setup("renew").await;
    keep_token(&core, &server);
    can(
        &edge,
        "account:view",
        401,
        json!({ "code": "unauthenticated" }),
    );
    can(&edge, "account:view", 200, json!({ "thisPhone": true }));
    let answer = view(&core, "f", false).await.unwrap().unwrap();
    assert_eq!(answer, json!({ "thisPhone": true }));
    assert_eq!(
        paths(&edge),
        ["auth:token", "account:view", "auth:token", "account:view"]
    );
    assert_eq!(body(&edge, 2), json!({ "refreshToken": "vrt_1" }));
}

#[tokio::test]
async fn a_removed_device_forgets_its_identity_and_address() {
    let (core, edge, server) = setup("removed").await;
    keep_token(&core, &server);
    let address = json!({ "server": server, "id": "kxkjq926sc", "address": "otp+kxkjq926sc@x" });
    core.write(crate::remote::forwarding::FILE, &address);
    assert!(core.saved_mailbox().is_some());
    can(
        &edge,
        "account:view",
        403,
        json!({ "code": "device_removed" }),
    );
    let answer = view(&core, "f", false).await.unwrap();
    assert_eq!(answer.unwrap_err().code(), "device_removed");
    assert!(core.device().refresh.is_none());
    assert!(edge.lock().unwrap().auth.is_none());
    // The server took it off every address: an OTP wait now says forwarding is off.
    assert!(core.saved_mailbox().is_none());
}

#[tokio::test]
async fn signed_calls_carry_a_good_signature_and_fix_the_clock() {
    let (core, edge, _server) = setup("signed").await;
    core.set_clock_offset(-900);
    let start = core
        .signed_call("f", Kind::Mutation, "auth:proveStart", &json!({}))
        .await
        .unwrap();
    assert_eq!(start["ticket"], crate::test_edge::CHALLENGE);
    assert!(core.device().clock_offset.abs() < 60);
    let edge = edge.lock().unwrap();
    assert_eq!(edge.requests.len(), 2);
    assert!(!edge.requests[0].signed, "15 minutes off");
    assert!(edge.requests[1].signed);
}

#[tokio::test]
async fn a_missing_function_reads_as_not_found() {
    let (core, _edge, server) = setup("missing").await;
    keep_token(&core, &server);
    let answer = core
        .device_call("f", Kind::Query, "nothing:here", json!({}))
        .await;
    assert_eq!(answer.unwrap_err().code(), "not_found");
}

#[test]
fn convex_errors_keep_their_code_and_text() {
    let error = CallError::Convex {
        message: "Uncaught ConvexError".into(),
        data: json!({ "code": "key_limit", "error": "Too many keys." }),
    };
    match edge_error("keys:create", error) {
        EdgeError::Status { code, message } => {
            assert_eq!(
                (code.as_str(), message.as_str()),
                ("key_limit", "Too many keys.")
            );
        }
        EdgeError::Offline => panic!("offline"),
    }
    assert!(matches!(
        edge_error("x:y", CallError::Closed),
        EdgeError::Offline
    ));
}

#[derive(Default)]
struct Heard(Mutex<Vec<String>>);

impl LiveListener for Heard {
    fn on_change(&self) {
        self.0.lock().unwrap().push("change".into());
    }
    fn on_error(&self, error: LiveError) {
        self.0.lock().unwrap().push(format!("{error:?}"));
    }
}

#[tokio::test]
async fn live_watches_follow_as_the_student() {
    let (core, edge, server) = setup("watch").await;
    let core = Arc::new(core);
    let heard = Arc::new(Heard::default());
    // No identity, never proving: an error, and nothing sent.
    let none = core
        .clone()
        .live_watch("f".into(), LiveTopic::Account, heard.clone())
        .await;
    assert!(matches!(none, Err(CoreError::ConsentDenied)));
    keep_token(&core, &server);
    can(&edge, "account:view", 200, json!({ "thisPhone": true }));
    let watch = core
        .clone()
        .live_watch("f-w".into(), LiveTopic::Account, heard.clone())
        .await
        .unwrap();
    assert_eq!(*heard.0.lock().unwrap(), ["change"]);
    assert_eq!(paths(&edge), ["auth:token", "account:view"]);
    assert_eq!(edge.lock().unwrap().requests[1].flow, "");
    watch.stop();
    // The inbox needs an address.
    let inbox = core
        .clone()
        .live_watch("f".into(), LiveTopic::MailInbox, heard.clone())
        .await;
    assert!(matches!(
        inbox,
        Err(CoreError::ForwardingOff {
            kind: crate::ForwardingOffKind::NotSetUp,
            ..
        })
    ));
}

#[test]
fn held_watches_renew_the_jwt_a_minute_early() {
    let expires = 1_000_000;
    let due = Some(expires - RENEW_AHEAD_MS);
    // Far off: it looks again within the check, to notice the watches went.
    assert_eq!(renew_in(due, 0), Some(KEEP_CHECK));
    assert_eq!(
        renew_in(due, expires - RENEW_AHEAD_MS - 5_000),
        Some(Duration::from_secs(5))
    );
    // Due, past due, or no JWT yet: renewed now.
    assert_eq!(renew_in(due, expires - RENEW_AHEAD_MS), None);
    assert_eq!(renew_in(due, expires), None);
    assert_eq!(renew_in(None, 0), None);
}

#[tokio::test]
async fn a_sign_out_the_server_missed_is_sent_later() {
    let (core, edge, server) = setup("signout-later").await;
    keep_token(&core, &server);
    can(&edge, "auth:signOut", 503, json!({ "code": "unavailable" }));
    core.sign_out_of_server("f".into(), false).await.unwrap();
    // Not signed out on the server: the token waits, not lost.
    assert!(core.device().refresh.is_none());
    assert!(core.read::<Refresh>(LAST_STUDENT).is_some());
    can(&edge, "auth:signOut", 200, json!(null));
    core.sign_out_last_student("f").await;
    assert!(core.read::<Refresh>(LAST_STUDENT).is_none());
    assert_eq!(
        paths(&edge).last().map(String::as_str),
        Some("auth:signOut")
    );
}

#[tokio::test]
async fn a_switch_signs_out_of_the_last_students_account() {
    let (core, edge, server) = setup("switch").await;
    keep_token(&core, &server);
    core.set_aside_identity();
    assert!(core.device().refresh.is_none());
    can(&edge, "auth:signOut", 200, json!(null));
    core.sign_out_last_student("f").await;
    assert_eq!(paths(&edge), ["auth:token", "auth:signOut"]);
    assert_eq!(body(&edge, 0), json!({ "refreshToken": REFRESH }));
    // Done once: nothing kept, and the socket goes without their JWT.
    assert!(core.read::<Refresh>(LAST_STUDENT).is_none());
    assert!(edge.lock().unwrap().auth.is_none());
    core.sign_out_last_student("f").await;
    assert_eq!(paths(&edge).len(), 2);
}

#[tokio::test]
async fn an_expired_jwt_is_renewed_with_the_refresh_token_never_proved() {
    let (core, edge, server) = setup("expired").await;
    keep_token(&core, &server);
    // The first JWT has run out by the next call (as one does minutes later).
    let expired = json!({
        "jwt": "jwt-old",
        "jwtExpiresAt": (crate::test_edge::now() - 1) * 1000,
        "refreshToken": "vrt_next",
        "regno": "22BCE0001",
        "account": "acct1",
    });
    can(&edge, "auth:token", 200, expired);
    can(&edge, "account:view", 200, json!({ "thisPhone": true }));
    can(&edge, "account:view", 200, json!({ "thisPhone": true }));
    view(&core, "f1", true).await.unwrap().unwrap();
    view(&core, "f2", true).await.unwrap().unwrap();
    assert_eq!(
        paths(&edge),
        ["auth:token", "account:view", "auth:token", "account:view"]
    );
    assert_eq!(body(&edge, 2), json!({ "refreshToken": "vrt_next" }));
    assert_eq!(core.device().refresh.unwrap().token, "vrt_1");
}

#[tokio::test]
async fn a_refresh_failing_for_another_reason_keeps_the_token_and_never_proves() {
    let (core, edge, server) = setup("refresh-other").await;
    keep_token(&core, &server);
    // Not a verdict on the refresh token (the socket's old JWT, a server fault): kept.
    can(
        &edge,
        "auth:token",
        401,
        json!({ "code": "unauthenticated" }),
    );
    can(&edge, "auth:token", 500, json!({ "code": "server_error" }));
    assert!(view(&core, "f1", true).await.is_err());
    assert!(view(&core, "f2", true).await.is_err());
    assert_eq!(paths(&edge), ["auth:token", "auth:token"]);
    assert_eq!(core.device().refresh.unwrap().token, REFRESH);
}

#[tokio::test]
async fn sign_out_last_student_persists_rotated_token_and_restores_current_jwt() {
    let (core, edge, server) = setup("signout-restore").await;
    // Current student holds a live JWT on the socket.
    keep_token(&core, &server);
    can(&edge, "account:view", 200, json!({ "thisPhone": true }));
    view(&core, "f-cur", false).await.unwrap().unwrap();
    let current_jwt = edge.lock().unwrap().auth.clone();
    assert_eq!(current_jwt.as_deref(), Some("jwt-1"));

    // Last student's refresh token is waiting to sign out; auth:signOut fails once, then succeeds.
    core.write(
        LAST_STUDENT,
        &Refresh {
            server: server.clone(),
            token: "vrt_last_old".into(),
            regno: "22BCE9999".into(),
        },
    );
    can(&edge, "auth:signOut", 503, json!({ "code": "unavailable" }));
    core.sign_out_last_student("f-so1").await;
    // The rotated refresh token from auth:token was saved back to LAST_STUDENT, and the current
    // student's JWT is still on the socket.
    let kept_last = core.read::<Refresh>(LAST_STUDENT).unwrap();
    assert_eq!(kept_last.token, "vrt_2");
    assert_eq!(edge.lock().unwrap().auth, current_jwt);

    can(&edge, "auth:signOut", 200, json!(null));
    core.sign_out_last_student("f-so2").await;
    assert!(core.read::<Refresh>(LAST_STUDENT).is_none());
    assert_eq!(edge.lock().unwrap().auth, current_jwt);
}

#[tokio::test]
async fn stale_device_removed_does_not_wipe_a_newer_identity() {
    let (core, edge, server) = setup("stale-removed").await;
    keep_token(&core, &server);
    can(&edge, "account:view", 200, json!({ "thisPhone": true }));
    let old_gen = core.shared.backend_state().auth.stamp().generation;
    view(&core, "f1", false).await.unwrap().unwrap();
    // Simulate a newer identity generation (e.g. a fresh proof or re-auth).
    let (new_stamp, effects) = core.shared.backend_state().auth.on_issued(
        GrantKind::Proof,
        &server,
        "22BCE0001",
        "jwt-fresh".into(),
        (crate::test_edge::now() + 900) * 1000,
        "vrt_fresh".into(),
    );
    core.run_effects(effects);
    assert!(new_stamp.generation > old_gen);

    // Late device_removed from the old generation is ignored.
    assert!(!core.on_device_removed(old_gen));
    assert_eq!(core.device().refresh.unwrap().token, "vrt_fresh");
    assert_eq!(edge.lock().unwrap().auth.as_deref(), Some("jwt-fresh"));
}

#[tokio::test]
async fn trailing_slash_on_saved_server_still_uses_kept_refresh_token() {
    let (core, edge, server) = setup("slash").await;
    core.keep_refresh(Some(Refresh {
        server: format!("{server}/"),
        token: REFRESH.into(),
        regno: "22BCE0001".into(),
    }));
    can(&edge, "account:view", 200, json!({ "thisPhone": true }));
    view(&core, "f", false).await.unwrap().unwrap();
    assert_eq!(paths(&edge), ["auth:token", "account:view"]);
}

#[test]
fn renewal_coordinator_coalesces_requests_and_stops_when_watches_drop() {
    let mut renewal = Renewal::default();
    let now = Instant::now();
    let held_a = Arc::new(());
    let held_b = Arc::new(());
    assert!(renewal.watch(&held_a, "f1"), "first watch starts thread");
    assert!(!renewal.watch(&held_b, "f2"), "second watch reuses thread");

    // Multiple unauthenticated watch failures coalesce into one immediate run and dedupe listeners.
    let listener_a: Arc<dyn LiveListener> = Arc::new(Heard::default());
    let listener_b: Arc<dyn LiveListener> = Arc::new(Heard::default());
    assert!(!renewal.request_now("f1", listener_a.clone()));
    assert!(!renewal.request_now("f1", listener_a.clone()));
    assert!(!renewal.request_now("f2", listener_b.clone()));
    assert_eq!(renewal.pending.len(), 2);

    assert_eq!(
        renewal.decide(Some(1_000_000), 0, now),
        RenewDecision::Run { flow: "f2".into() }
    );
    assert_eq!(renewal.phase, RenewPhase::Renewing);

    // Server change pokes the coordinator without spawning a second thread.
    renewal.poke();
    assert!(renewal.running);

    // No identity stops the coordinator and returns both pending listeners to notify.
    let (stop, notify) = renewal.finish(&Err(CoreError::ConsentDenied), now);
    assert!(stop);
    assert_eq!(notify.len(), 2);
    assert!(!renewal.running);

    // Restarting then dropping all watches cleanly transitions to Stop.
    assert!(renewal.watch(&held_a, "f3"));
    drop(held_a);
    drop(held_b);
    assert_eq!(renewal.decide(Some(1_000_000), 0, now), RenewDecision::Stop);
    assert!(!renewal.running);
}

#[test]
fn live_errors_say_what_the_app_does_next() {
    let status = |code, message: &str| EdgeError::Status {
        code,
        message: message.into(),
    };
    for code in [
        ServerCode::Unauthenticated,
        ServerCode::ServerError,
        ServerCode::NotFound,
        ServerCode::BadRequest,
    ] {
        assert_eq!(LiveError::of(status(code, "x")), LiveError::Transient);
    }
    assert_eq!(
        LiveError::of(status(ServerCode::DeviceRemoved, "Removed.")),
        LiveError::NoIdentity {
            message: "Removed.".into()
        }
    );
    assert_eq!(
        LiveError::of(status(ServerCode::Unknown("mailbox_full".into()), "Full.")),
        LiveError::Stopped {
            message: "Full.".into()
        }
    );
    assert_eq!(
        LiveError::of(EdgeError::Offline),
        LiveError::Stopped {
            message: "The server can't be reached.".into()
        }
    );
}

use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use super::board::{GRACE_MS, Request, TTL_MS, notification_id};
use super::*;
use crate::events::AppEvents;
use crate::problem::Problem;
use crate::remote::consent::{self, ConsentAsk, CookieKind};
use crate::schedule::ActivityEntry;
use crate::test_edge::{Edge, keep_token, serve};

fn cancel(id: &str) -> HandoverEffect {
    HandoverEffect::Cancel {
        notification_id: notification_id(id),
    }
}

fn notify(id: &str, expires_at: i64) -> HandoverEffect {
    HandoverEffect::Notify {
        request_id: id.into(),
        notification_id: notification_id(id),
        expires_at,
        asker: None,
    }
}

/// The attempt of the one `Answer` among `effects`.
fn answer_of(effects: &[HandoverEffect], id: &str) -> u64 {
    let answers: Vec<u64> = (effects.iter())
        .filter_map(|effect| match effect {
            HandoverEffect::Answer {
                request_id,
                attempt,
            } if request_id == id => Some(*attempt),
            _ => None,
        })
        .collect();
    assert_eq!(answers.len(), 1, "{effects:?}");
    answers[0]
}

#[test]
fn expired_requests_are_dropped_before_notifying_or_answering() {
    let mut board = Board::default();
    let away = board.arrived("req-past", Some(9_999), Wake::Away, 10_000);
    assert_eq!(away, [cancel("req-past")]);
    assert_eq!(board.get("req-past"), None);

    let shown = board.arrived("req-now", Some(10_000), Wake::Shown, 10_000);
    assert_eq!(shown, [cancel("req-now")]);
    assert_eq!(board.len(), 0);
}

#[test]
fn a_notified_request_is_answered_once_the_app_shows() {
    let mut board = Board::default();
    let first = board.arrived("req-1", Some(60_000), Wake::Away, 1_000);
    assert_eq!(first, [notify("req-1", 60_000)]);
    assert_eq!(
        board.get("req-1"),
        Some(Request::Notified { expires_at: 60_000 })
    );
    // A second FCM while away posts nothing again.
    assert_eq!(board.arrived("req-1", Some(60_000), Wake::Away, 1_000), []);

    let shown = board.shown(1_000);
    let attempt = answer_of(&shown, "req-1");
    assert_eq!(shown[0], cancel("req-1"), "its notification goes first");
    assert_eq!(
        board.get("req-1"),
        Some(Request::Answering {
            expires_at: 60_000,
            attempt
        })
    );

    // Showing again, the live list or a tap while it is answered: nothing more.
    assert_eq!(board.shown(1_000), []);
    assert_eq!(board.arrived("req-1", Some(60_000), Wake::Shown, 1_000), []);
    assert_eq!(
        board.arrived("req-1", None, Wake::Tapped, 1_000),
        [cancel("req-1")]
    );

    assert!(board.finish("req-1", attempt, 1_000));
    assert_eq!(
        board.get("req-1"),
        Some(Request::Done { expires_at: 60_000 })
    );
    // Done: every way in is a duplicate.
    assert_eq!(board.arrived("req-1", Some(60_000), Wake::Shown, 1_000), []);
    assert_eq!(board.arrived("req-1", Some(60_000), Wake::Away, 1_000), []);
    assert_eq!(
        board.arrived("req-1", None, Wake::Tapped, 1_000),
        [cancel("req-1")]
    );
}

#[test]
fn a_tap_answers_a_notified_request_with_its_known_expiry() {
    let mut board = Board::default();
    board.arrived("req-tap", Some(50_000), Wake::Away, 1_000);

    let tap = board.arrived("req-tap", None, Wake::Away, 1_000);
    assert_eq!(tap, [], "away without a tap: still waiting");
    let tap = board.arrived("req-tap", None, Wake::Tapped, 1_000);
    let attempt = answer_of(&tap, "req-tap");
    assert_eq!(tap[0], cancel("req-tap"));
    assert_eq!(
        board.get("req-tap"),
        Some(Request::Answering {
            expires_at: 50_000,
            attempt
        })
    );
    assert_eq!(
        board.arrived("req-tap", Some(50_000), Wake::Shown, 1_000),
        []
    );
    assert_eq!(board.shown(1_000), []);
}

#[test]
fn on_screen_a_request_is_answered_at_once_and_its_listing_ignored() {
    let mut board = Board::default();
    let fcm = board.arrived("req-fg", Some(121_000), Wake::Shown, 1_000);
    let attempt = answer_of(&fcm, "req-fg");
    assert_eq!(
        board.get("req-fg"),
        Some(Request::Answering {
            expires_at: 121_000,
            attempt
        })
    );
    assert_eq!(
        board.arrived("req-fg", Some(120_000), Wake::Shown, 1_000),
        []
    );
}

#[test]
fn a_request_without_an_expiry_gets_the_server_s_lifetime() {
    let mut board = Board::default();
    assert_eq!(
        board.arrived("req", None, Wake::Away, 1_000),
        [notify("req", 1_000 + TTL_MS)]
    );
    // A sooner expiry from a later copy wins.
    board.arrived("req", Some(5_000), Wake::Away, 1_000);
    assert_eq!(
        board.get("req"),
        Some(Request::Notified { expires_at: 5_000 })
    );
}

#[test]
fn a_notified_request_that_expired_is_dropped_and_its_notification_goes() {
    let mut board = Board::default();

    // Tapped at expiry, with no expiry on the tap: still known as expired.
    board.arrived("req-tap", Some(5_000), Wake::Away, 1_000);
    let tap = board.arrived("req-tap", None, Wake::Tapped, 5_000);
    assert_eq!(tap, [cancel("req-tap")]);
    assert_eq!(
        board.get("req-tap"),
        Some(Request::Done { expires_at: 5_000 })
    );

    // Coming on screen clears a freshly expired one's notification.
    board.arrived("req-shown", Some(6_000), Wake::Away, 1_000);
    assert_eq!(board.shown(6_000), [cancel("req-shown")]);

    // Long past its grace, a tap with no expiry is still dropped.
    board.arrived("req-late", Some(7_000), Wake::Away, 1_000);
    let late = board.arrived("req-late", None, Wake::Tapped, 7_000 + GRACE_MS + 50_000);
    assert_eq!(late, [cancel("req-late")]);
    assert_eq!(board.get("req-late"), None);
}

#[test]
fn only_the_attempt_holding_a_request_finishes_it() {
    let mut board = Board::default();
    let a = answer_of(
        &board.arrived("req-a", Some(60_000), Wake::Shown, 1_000),
        "req-a",
    );
    assert!(!board.finish("req-a", a + 99, 1_000), "another attempt's");
    assert!(board.finish("req-a", a, 1_000));
    assert!(!board.finish("req-a", a, 1_000), "finished once");
    assert_eq!(board.start("req-a", a, 1_000), Start::Stale);
    assert_eq!(
        board.get("req-a"),
        Some(Request::Done { expires_at: 60_000 })
    );
}

#[test]
fn an_attempt_starting_after_expiry_sends_nothing() {
    let mut board = Board::default();
    let late = answer_of(
        &board.arrived("req-late", Some(10_000), Wake::Shown, 9_500),
        "req-late",
    );
    assert_eq!(board.start("req-late", late, 9_999), Start::Run);
    assert_eq!(board.start("req-late", late, 10_000), Start::Expired);
    assert_eq!(
        board.get("req-late"),
        Some(Request::Done { expires_at: 10_000 })
    );
}

#[test]
fn an_abandoned_attempt_frees_a_live_request_and_ends_an_expired_one() {
    let mut board = Board::default();
    let live = answer_of(
        &board.arrived("req-live", Some(10_000), Wake::Shown, 1_000),
        "req-live",
    );
    board.abandon("req-live", live + 1, 1_000);
    assert!(board.get("req-live").is_some(), "not that attempt's");
    board.abandon("req-live", live, 1_000);
    assert_eq!(board.get("req-live"), None);
    let again = board.arrived("req-live", Some(10_000), Wake::Shown, 1_000);
    assert!(answer_of(&again, "req-live") > live, "answered afresh");

    let expired = answer_of(
        &board.arrived("req-old", Some(10_000), Wake::Shown, 1_000),
        "req-old",
    );
    board.abandon("req-old", expired, 10_000);
    assert_eq!(
        board.get("req-old"),
        Some(Request::Done { expires_at: 10_000 })
    );
}

#[test]
fn done_requests_are_forgotten_after_their_grace() {
    let mut board = Board::default();
    for i in 1..=20 {
        let id = format!("done-{i}");
        let attempt = answer_of(&board.arrived(&id, Some(10_000), Wake::Shown, 1_000), &id);
        assert!(board.finish(&id, attempt, 1_000));
    }
    let active = answer_of(
        &board.arrived("active", Some(10_000), Wake::Shown, 1_000),
        "active",
    );
    board.arrived("notified", Some(10_000), Wake::Away, 1_000);
    assert_eq!(board.len(), 22);

    // Within the grace: done ones stay to catch duplicates; the notified one ends.
    assert_eq!(board.shown(12_000), [cancel("notified")]);
    assert_eq!(board.len(), 22);
    assert_eq!(
        board.get("notified"),
        Some(Request::Done { expires_at: 10_000 })
    );

    // Past it: only the one still answering stays.
    assert_eq!(board.shown(10_000 + GRACE_MS), []);
    assert_eq!(board.len(), 1);
    assert_eq!(
        board.get("active"),
        Some(Request::Answering {
            expires_at: 10_000,
            attempt: active
        })
    );
    // Finishing past the grace forgets it at once.
    assert!(board.finish("active", active, 10_000 + GRACE_MS));
    assert_eq!(board.len(), 0);
}

#[test]
fn notification_ids_are_java_s_string_hash_in_their_own_range() {
    // "ab".hashCode() == 97 * 31 + 98.
    assert_eq!(notification_id("ab"), 0x4000_0000 + 3105);
    // Overflowing hashes wrap and are masked as Java's `and 0x0fffffff` does.
    let long = notification_id("req-alpha-0123456789");
    assert!((0x4000_0000..=0x4fff_ffff).contains(&long));
    assert_eq!(notification_id("req-alpha"), notification_id("req-alpha"));
    assert_ne!(notification_id("req-alpha"), notification_id("req-beta"));
}

#[test]
fn fcm_expiry_takes_seconds_or_ms_and_falls_back_to_at() {
    assert_eq!(
        push_expiry(&data(&[("expiresAt", "1700000120000")])),
        Some(1_700_000_120_000)
    );
    assert_eq!(
        push_expiry(&data(&[("expiresAt", "1700000120")])),
        Some(1_700_000_120_000)
    );
    assert_eq!(
        push_expiry(&data(&[("at", "1700000000")])),
        Some(1_700_000_000_000 + TTL_MS)
    );
    assert_eq!(push_expiry(&data(&[("kind", "vtop_cookie_request")])), None);
    assert_eq!(push_expiry(&data(&[("at", "not-a-number")])), None);
}

#[test]
fn a_push_names_who_asked_in_its_notification() {
    let core = core("asker");
    let push = data(&[
        ("requestId", "req-a"),
        ("expiresAt", "4102444800000"),
        ("keyLabel", "Chrome on MacBook"),
        ("keyVia", "extension"),
        ("ip", "203.0.113.7"),
        ("ipSeen", "1"),
    ]);
    let effects = core.handover(
        "f".into(),
        HandoverEvent::Pushed {
            data: push,
            foreground: false,
        },
    );
    let asker = HandoverAsker {
        key_label: "Chrome on MacBook".into(),
        via: KeyKind::Browser,
        ip: Some(AskerIp {
            address: "203.0.113.7".into(),
            seen: true,
        }),
    };
    assert_eq!(
        effects,
        [HandoverEffect::Notify {
            request_id: "req-a".into(),
            notification_id: notification_id("req-a"),
            expires_at: 4_102_444_800_000,
            asker: Some(asker),
        }]
    );
    // An AI app's push has no address; one without a label names nobody.
    let mcp = push_asker(&data(&[("keyLabel", "Claude"), ("keyVia", "mcp")]));
    assert_eq!(
        mcp,
        Some(HandoverAsker {
            key_label: "Claude".into(),
            via: KeyKind::Mcp,
            ip: None,
        })
    );
    assert_eq!(push_asker(&data(&[("ip", "203.0.113.7")])), None);
}

#[test]
fn a_push_without_a_request_id_does_nothing() {
    let core = core("no-id");
    let data = HashMap::from([("requestId".to_string(), " ".to_string())]);
    let event = HandoverEvent::Pushed {
        data,
        foreground: true,
    };
    assert_eq!(core.handover("f".into(), event), []);
    assert_eq!(core.shared.handover().len(), 0);
}

fn data(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    (pairs.iter())
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn core(name: &str) -> Core {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-handover-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    Core::for_test(dir)
}

/// A core on a fake server that takes answers, and its server.
async fn answering(name: &str) -> (Core, Arc<Mutex<Edge>>) {
    let core = core(name);
    let edge = Arc::new(Mutex::new(Edge::default()));
    let server = serve(edge.clone()).await;
    fs::write(core.at(crate::remote::edge::SERVER_FILE), &server).unwrap();
    keep_token(&core, &server);
    (core, edge)
}

fn answers(edge: &Arc<Mutex<Edge>>) -> Vec<Value> {
    let edge = edge.lock().unwrap();
    (edge.requests.iter())
        .filter(|request| request.path == "sessions:answer")
        .map(|request| request.body.clone())
        .collect()
}

/// A request on screen, far from expiry, being answered: its attempt.
fn on_screen(core: &Core, id: &str) -> u64 {
    answer_of(
        &core
            .shared
            .handover()
            .arrived(id, Some(i64::MAX / 2), Wake::Shown, 0),
        id,
    )
}

#[tokio::test]
async fn a_request_is_answered_once() {
    let (core, edge) = answering("once").await;
    let attempt = on_screen(&core, "r1");
    // Signed out (no session in this core): the server hears why, the caller gets the error.
    let answered = core
        .answer_handover("f".into(), "r1".into(), attempt, 60)
        .await;
    assert_eq!(answered, Err(CoreError::NotSignedIn));
    // A second run of the same attempt (a duplicate effect) sends nothing.
    let again = core
        .answer_handover("f".into(), "r1".into(), attempt, 60)
        .await;
    assert_eq!(again, Ok(()));
    assert_eq!(
        answers(&edge),
        [json!({ "requestId": "r1", "error": "The phone is signed out of VTOP." })]
    );
    assert!(matches!(
        core.shared.handover().get("r1"),
        Some(Request::Done { .. })
    ));
}

#[tokio::test]
async fn an_expired_request_is_not_answered() {
    let (core, edge) = answering("expired").await;
    let attempt = answer_of(
        &core
            .shared
            .handover()
            .arrived("r1", Some(10_000), Wake::Shown, 0),
        "r1",
    );
    // The phone's clock is far past 10 s after the epoch.
    let answered = core
        .answer_handover("f".into(), "r1".into(), attempt, 60)
        .await;
    assert_eq!(answered, Ok(()));
    assert_eq!(answers(&edge), Vec::<Value>::new());
}

/// Shows the consent dialog and never answers it.
struct Unanswered(AtomicU32);

impl AppEvents for Unanswered {
    fn on_background_problem(&self, _problem: Problem) {}
    fn on_activity(&self, _entries: Vec<ActivityEntry>) {}
    fn on_consent(&self, ask: ConsentAsk) {
        if !matches!(ask, ConsentAsk::None) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn on_handover_status(&self, _flow: String, _request_id: String, _status: HandoverStatus) {}
    fn is_flow_unattended(&self, _flow: String) -> bool {
        false
    }
    fn on_semesters(&self, _state: crate::SemesterState) {}
    fn on_sync_health(&self, _health: crate::SyncHealth, _paused: Option<String>) {}
}

#[tokio::test]
async fn past_its_limit_the_answer_is_dropped_and_only_the_timeout_sent() {
    let (core, edge) = answering("timeout").await;
    consent::tests::signed_in(&core);
    let gate = Arc::new(Unanswered(AtomicU32::new(0)));
    core.events.set_shown_for_test(true);
    core.set_events(gate.clone());
    let attempt = on_screen(&core, "r1");

    let answered = core
        .answer_handover("f".into(), "r1".into(), attempt, 0)
        .await;
    assert_eq!(
        answered,
        Err(CoreError::Other {
            reason: TIMEOUT.into()
        })
    );
    assert_eq!(gate.0.load(Ordering::SeqCst), 1, "the student was asked");
    // The server hears it as any other reason the phone has no words of its own for.
    let error = format!("The phone couldn't get a session: {TIMEOUT}");
    assert_eq!(
        answers(&edge),
        [json!({ "requestId": "r1", "error": error })]
    );
    assert!(matches!(
        core.shared.handover().get("r1"),
        Some(Request::Done { .. })
    ));
    // Nothing remembered: the dialog never answered.
    assert!(
        !core
            .shared
            .consent()
            .remembered
            .contains(&CookieKind::Handover)
    );
}

#[tokio::test]
async fn an_answer_the_app_cancels_frees_the_request() {
    let (core, edge) = answering("cancelled").await;
    consent::tests::signed_in(&core);
    core.events.set_shown_for_test(true);
    core.set_events(Arc::new(Unanswered(AtomicU32::new(0))));
    let attempt = on_screen(&core, "r1");
    {
        let answer = core.answer_handover("f".into(), "r1".into(), attempt, 60);
        let mut answer = std::pin::pin!(answer);
        assert!(futures::poll!(answer.as_mut()).is_pending(), "asking");
    }
    assert_eq!(core.shared.handover().get("r1"), None, "asked again later");
    assert_eq!(answers(&edge), Vec::<Value>::new());
}

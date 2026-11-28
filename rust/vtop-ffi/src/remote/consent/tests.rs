use std::collections::VecDeque;
use std::fmt::Write as _;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, Weak};

use tracing::field::{Field, Visit};
use tracing::span::Attributes;
use tracing::{Event, Id, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use super::*;
use crate::events::AppEvents;
use crate::problem::Problem;
use crate::runtime::ManualClock;
use crate::schedule::ActivityEntry;
use crate::{ACCOUNT, Account, Session};

/// Runs `future` on this thread.
fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

/// Every log field, one per line.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<String>>);

impl Visit for Captured {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
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

/// Scripted answers (yes once the script runs out, or no with `deny`), whether flows are
/// attended, and how often it asked.
pub(crate) struct TestGate {
    shared: Weak<Shared>,
    pub(crate) attended: AtomicBool,
    pub(crate) deny: bool,
    pub(crate) answers: Mutex<VecDeque<CookieAnswer>>,
    pub(crate) payloads: Mutex<Vec<CookiePayload>>,
    pub(crate) asks: AtomicU32,
    /// The registration-number sheet's answers (Dismissed once they run out) and its asks.
    pub(crate) regno_answers: Mutex<VecDeque<RegnoAnswer>>,
    pub(crate) regno_asks: AtomicU32,
    /// What the approve sheet was asked (answered like `ask`), and the statuses it heard.
    pub(crate) handover_asks: Mutex<Vec<HandoverAsk>>,
    pub(crate) statuses: Mutex<Vec<HandoverStatus>>,
}

impl TestGate {
    pub(crate) fn asks(&self) -> u32 {
        self.asks.load(Ordering::SeqCst)
    }
}

impl AppEvents for TestGate {
    fn on_background_problem(&self, _problem: Problem) {}
    fn on_activity(&self, _entries: Vec<ActivityEntry>) {}

    fn on_consent(&self, ask: ConsentAsk) {
        let Some(shared) = self.shared.upgrade() else {
            return;
        };
        match ask {
            ConsentAsk::None => {}
            ConsentAsk::Cookie { id, payload, .. } => {
                self.asks.fetch_add(1, Ordering::SeqCst);
                self.payloads.lock().unwrap().push(payload);
                let next = self.answers.lock().unwrap().pop_front();
                let answer = next.unwrap_or(if self.deny {
                    CookieAnswer::Deny
                } else {
                    CookieAnswer::Allow { remember: false }
                });
                let mapped = match answer {
                    CookieAnswer::Deny => ConsentAnswer::Cancel,
                    CookieAnswer::Allow { remember: false } => ConsentAnswer::Send,
                    CookieAnswer::Allow { remember: true } => ConsentAnswer::DontAskAgain,
                };
                answer_shared(&shared, id, mapped);
            }
            ConsentAsk::Regno { id, .. } => {
                self.regno_asks.fetch_add(1, Ordering::SeqCst);
                let next = self
                    .regno_answers
                    .lock()
                    .unwrap()
                    .pop_front()
                    .unwrap_or(RegnoAnswer::Dismissed);
                let mapped = match next {
                    RegnoAnswer::Share => ConsentAnswer::Share,
                    RegnoAnswer::NotNow => ConsentAnswer::NotNow,
                    RegnoAnswer::Dismissed => ConsentAnswer::Cancel,
                };
                answer_shared(&shared, id, mapped);
            }
            ConsentAsk::Handover {
                id,
                ask,
                phase: HandoverSheetPhase::Asking,
                ..
            } => {
                self.asks.fetch_add(1, Ordering::SeqCst);
                self.handover_asks.lock().unwrap().push(ask);
                let next = self.answers.lock().unwrap().pop_front();
                let approve = next.map_or(!self.deny, |answer| {
                    matches!(answer, CookieAnswer::Allow { .. })
                });
                answer_shared(
                    &shared,
                    id,
                    if approve {
                        ConsentAnswer::Approve
                    } else {
                        ConsentAnswer::Deny
                    },
                );
            }
            ConsentAsk::Handover { .. } => {}
        }
    }

    fn on_handover_status(&self, _flow: String, _request_id: String, status: HandoverStatus) {
        self.statuses.lock().unwrap().push(status);
    }

    fn is_flow_unattended(&self, _flow: String) -> bool {
        !self.attended.load(Ordering::SeqCst)
    }
    fn on_semesters(&self, _state: crate::SemesterState) {}
    fn on_sync_health(&self, _health: crate::SyncHealth, _paused: Option<String>) {}
}

/// Gives `core` a fresh gate (and no remembered kinds).
pub(crate) fn install(core: &Core, attended: bool, deny: bool) -> Arc<TestGate> {
    let gate = Arc::new(TestGate {
        shared: Arc::downgrade(&core.shared),
        attended: AtomicBool::new(attended),
        deny,
        answers: Mutex::default(),
        payloads: Mutex::default(),
        asks: AtomicU32::new(0),
        regno_answers: Mutex::default(),
        regno_asks: AtomicU32::new(0),
        handover_asks: Mutex::default(),
        statuses: Mutex::default(),
    });
    *core.shared.consent() = State::default();
    core.events.set_shown_for_test(attended);
    core.set_events(gate.clone());
    gate
}

/// An attended gate that says yes to everything.
pub(crate) fn yes(core: &Core) -> Arc<TestGate> {
    install(core, true, false)
}

/// An attended gate that says no to everything.
pub(crate) fn no(core: &Core) -> Arc<TestGate> {
    install(core, true, true)
}

/// Gives `core` a session (never used against VTOP before the gate).
pub(crate) fn signed_in(core: &Core) {
    let account = Account {
        username: "22BCE0001".into(),
        password: "pw".into(),
        registration_number: Some("22BCE0001".into()),
    };
    let session = Session::new(account, &core.shared).unwrap();
    core.install(session);
    crate::remote::regno::tests::shared(core, "22BCE0001");
}

pub(crate) fn remember(core: &Core, kind: CookieKind) {
    core.shared.consent().remembered.insert(kind);
}

fn core(name: &str) -> Core {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-consent-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let core = Core::for_test(dir);
    load(&core);
    core
}

/// A core with no files: nothing it remembers is saved.
fn fresh() -> Core {
    Core::for_test(std::env::temp_dir())
}

fn check(core: &Core, flow: &str, kind: CookieKind, ask: bool) -> bool {
    let shared = &core.shared;
    block_on(allowed(
        shared,
        flow,
        kind,
        CookiePayload::Cookie,
        "Test",
        "vmate.aster0.dev",
        ask,
    ))
}

const YES_REMEMBER: CookieAnswer = CookieAnswer::Allow { remember: true };

#[test]
fn no_gate_denies() {
    let core = fresh();
    assert!(!check(&core, "f", CookieKind::Route, true));
}

#[test]
fn empty_flow_denies() {
    let core = fresh();
    let gate = yes(&core);
    assert!(!check(&core, "", CookieKind::Route, true));
    assert_eq!(gate.asks(), 0);
}

#[test]
fn unattended_denies_unless_route_remembered() {
    let core = fresh();
    let gate = install(&core, false, false);
    assert!(!check(&core, "f", CookieKind::Route, true));
    remember(&core, CookieKind::Account);
    remember(&core, CookieKind::Handover);
    assert!(!check(&core, "f", CookieKind::Account, true));
    assert!(!check(&core, "f", CookieKind::Handover, true));
    remember(&core, CookieKind::Route);
    assert!(check(&core, "f", CookieKind::Route, true));
    assert_eq!(gate.asks(), 0);
}

#[test]
fn remembered_kind_sends_without_asking() {
    let core = fresh();
    let gate = no(&core);
    remember(&core, CookieKind::Account);
    assert!(check(&core, "f", CookieKind::Account, true));
    assert!(check(&core, "f", CookieKind::Account, false));
    assert_eq!(gate.asks(), 0);
}

#[test]
fn remember_saves_only_that_kind() {
    let core = core("remember");
    let gate = yes(&core);
    load(&core);
    gate.answers.lock().unwrap().push_back(YES_REMEMBER);
    assert!(check(&core, "f1", CookieKind::Handover, true));
    assert_eq!(
        core.cookie_consent_remembered("f".into()),
        [CookieKind::Handover]
    );
    // Saved: a new start reads it back.
    core.shared.consent().remembered.clear();
    load(&core);
    assert!(check(&core, "f2", CookieKind::Handover, false));
    // Another kind still asks.
    assert!(check(&core, "f2", CookieKind::Account, true));
    assert_eq!(gate.asks(), 2);
    let _ = fs::remove_dir_all(&core.dir);
}

#[test]
fn yes_without_remember_covers_the_flow_only() {
    let core = fresh();
    let gate = yes(&core);
    assert!(check(&core, "f1", CookieKind::Route, true));
    assert!(check(&core, "f1", CookieKind::Route, true));
    assert_eq!(gate.asks(), 1);
    assert!(check(&core, "f2", CookieKind::Route, true));
    assert_eq!(gate.asks(), 2);
    // A no covers the flow too, and isn't kept past it.
    let gate = no(&core);
    assert!(!check(&core, "f3", CookieKind::Route, true));
    assert!(!check(&core, "f3", CookieKind::Route, true));
    assert_eq!(gate.asks(), 1);
    assert!(!check(&core, "f4", CookieKind::Route, true));
    assert_eq!(gate.asks(), 2);
}

#[test]
fn quiet_never_asks() {
    let core = fresh();
    let gate = yes(&core);
    assert!(!check(&core, "f", CookieKind::Handover, false));
    assert_eq!(gate.asks(), 0);
}

#[test]
fn reset_forgets_remembered_kinds() {
    let core = core("reset");
    let gate = yes(&core);
    load(&core);
    gate.answers.lock().unwrap().push_back(YES_REMEMBER);
    assert!(check(&core, "f1", CookieKind::Route, true));
    assert!(core.at(FILE).exists());
    core.reset_cookie_consent("f".into()).unwrap();
    assert!(core.cookie_consent_remembered("f".into()).is_empty());
    assert!(!core.at(FILE).exists());
    // The flow's yes went too.
    assert!(!check(&core, "f1", CookieKind::Route, false));
    let _ = fs::remove_dir_all(&core.dir);
}

#[test]
fn forgetting_one_kind_keeps_the_others() {
    let core = core("forget-kind");
    let gate = yes(&core);
    load(&core);
    gate.answers.lock().unwrap().push_back(YES_REMEMBER);
    assert!(check(&core, "f1", CookieKind::Handover, true));
    core.remember_cookie_consent("f".into(), CookieKind::Route)
        .unwrap();
    core.remember_cookie_consent("f".into(), CookieKind::Account)
        .unwrap();
    core.forget_cookie_consent("f".into(), CookieKind::Handover)
        .unwrap();
    assert_eq!(
        core.cookie_consent_remembered("f".into()),
        [CookieKind::Route, CookieKind::Account]
    );
    // The flow's yes for that kind went too; the file keeps the others, Account included.
    assert!(!check(&core, "f1", CookieKind::Handover, false));
    assert_eq!(
        fs::read_to_string(core.at(FILE)).unwrap(),
        r#"{"remembered":["route","account"]}"#
    );
    load(&core);
    assert_eq!(
        core.cookie_consent_remembered("f".into()),
        [CookieKind::Route, CookieKind::Account]
    );
    let _ = fs::remove_dir_all(&core.dir);
}

#[test]
fn another_student_forgets_remembered_kinds() {
    let core = core("switch");
    let gate = yes(&core);
    load(&core);
    gate.answers.lock().unwrap().push_back(YES_REMEMBER);
    assert!(check(&core, "f", CookieKind::Handover, true));
    fs::write(
        core.at(ACCOUNT),
        r#"{"username":"21BCE0001","password":"x"}"#,
    )
    .unwrap();
    let account = Account {
        username: "21BCE0002".into(),
        password: "pw".into(),
        registration_number: None,
    };
    let slot = core.install(Session::new(account, &core.shared).unwrap());
    core.save(&slot, &block_on(slot.read()));
    assert!(core.cookie_consent_remembered("f".into()).is_empty());
    assert!(!core.at(FILE).exists());
    // Signing out forgets them too.
    gate.answers.lock().unwrap().push_back(YES_REMEMBER);
    assert!(check(&core, "f2", CookieKind::Route, true));
    core.sign_out_here("f");
    assert!(core.cookie_consent_remembered("f".into()).is_empty());
    let _ = fs::remove_dir_all(&core.dir);
}

#[test]
fn consent_lines_carry_kind_and_decision_and_no_secret() {
    let captured = Captured::default();
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let subscriber = tracing_subscriber::registry().with(captured.clone());
    tracing::subscriber::with_default(subscriber, || {
        let core = fresh();
        let _gate = install(&core, false, false);
        block_on(allowed(
            &core.shared,
            "f",
            CookieKind::Route,
            CookiePayload::Cookie,
            "Fetch JSESSIONID=secret-cookie",
            "vmate.aster0.dev/?token=secret-query",
            true,
        ));
    });
    let logs = captured.0.lock().unwrap();
    assert!(logs.contains("kind=\"route\""), "{logs}");
    assert!(logs.contains("decision=\"denied_unattended\""), "{logs}");
    assert!(logs.contains("message=cookie consent"), "{logs}");
    for secret in ["secret-cookie", "secret-query"] {
        assert!(!logs.contains(secret), "{secret}: {logs}");
    }
}

#[test]
fn hosts_drop_the_scheme_and_path() {
    assert_eq!(host("https://vmate.aster0.dev/"), "vmate.aster0.dev");
    assert_eq!(
        host("https://convex-site.aster0.dev/auth?x=1"),
        "convex-site.aster0.dev"
    );
    assert_eq!(host("vmate.aster0.dev"), "vmate.aster0.dev");
}

#[test]
fn a_yes_given_by_choosing_server_fetch_is_kept() {
    let core = core("choose");
    let gate = no(&core);
    load(&core);
    core.remember_cookie_consent("f".into(), CookieKind::Route)
        .unwrap();
    assert!(check(&core, "f2", CookieKind::Route, true));
    assert_eq!(gate.asks(), 0);
    // Saved, as a "Don't ask again" in the dialog is.
    assert!(core.at(FILE).exists());
}

#[test]
fn ticks_replace_the_file_instead_of_rewriting_it_in_place() {
    let core = core("atomic");
    core.remember_cookie_consent("f".into(), CookieKind::Route)
        .unwrap();
    // A second name for the old file: written in place, it would change too (and a crash
    // halfway would leave half a file).
    let old = core.at("consent.old.json");
    fs::hard_link(core.at(FILE), &old).unwrap();
    let before = fs::read_to_string(&old).unwrap();
    core.remember_cookie_consent("f".into(), CookieKind::Account)
        .unwrap();
    assert_eq!(fs::read_to_string(&old).unwrap(), before);
    let saved = fs::read_to_string(core.at(FILE)).unwrap();
    assert!(
        saved.contains("route") && saved.contains("account"),
        "{saved}"
    );
    let _ = fs::remove_dir_all(&core.dir);
}

/// Captures pushed [`ConsentAsk`]s without answering them, for interactive prompt tests.
#[derive(Default)]
struct ManualEvents {
    pushed: Mutex<Vec<ConsentAsk>>,
    unattended: Mutex<BTreeSet<String>>,
}

impl ManualEvents {
    fn current(&self) -> ConsentAsk {
        self.pushed
            .lock()
            .unwrap()
            .last()
            .cloned()
            .unwrap_or(ConsentAsk::None)
    }

    fn count(&self) -> usize {
        self.pushed.lock().unwrap().len()
    }
}

impl AppEvents for ManualEvents {
    fn on_background_problem(&self, _problem: Problem) {}
    fn on_activity(&self, _entries: Vec<ActivityEntry>) {}
    fn on_consent(&self, ask: ConsentAsk) {
        self.pushed.lock().unwrap().push(ask);
    }
    fn on_handover_status(&self, _flow: String, _request_id: String, _status: HandoverStatus) {}
    fn is_flow_unattended(&self, flow: String) -> bool {
        self.unattended.lock().unwrap().contains(&flow)
    }
    fn on_semesters(&self, _state: crate::SemesterState) {}
    fn on_sync_health(&self, _health: crate::SyncHealth, _paused: Option<String>) {}
}

fn manual_core() -> (Core, ManualClock, Arc<ManualEvents>) {
    let clock = ManualClock::default();
    let core = Core::for_test_with_clock(
        std::env::temp_dir(),
        Shared::default(),
        Arc::new(clock.clone()),
    );
    let events = Arc::new(ManualEvents::default());
    core.set_events(events.clone());
    (core, clock, events)
}

#[tokio::test]
async fn concurrent_asks_for_same_flow_and_kind_coalesce_into_one_prompt() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);

    let shared_a = core.shared.clone();
    let shared_b = core.shared.clone();
    let first = tokio::spawn(async move {
        allowed(
            &shared_a,
            "f-same",
            CookieKind::Route,
            CookiePayload::Cookie,
            "Fetch A",
            "vmate.aster0.dev",
            true,
        )
        .await
    });
    while !matches!(events.current(), ConsentAsk::Cookie { .. }) {
        tokio::task::yield_now().await;
    }
    let second = tokio::spawn(async move {
        allowed(
            &shared_b,
            "f-same",
            CookieKind::Route,
            CookiePayload::Cookie,
            "Fetch B",
            "vmate.aster0.dev",
            true,
        )
        .await
    });
    tokio::task::yield_now().await;
    let ConsentAsk::Cookie { id, .. } = events.current() else {
        panic!("expected cookie prompt");
    };
    core.consent_answer("f-same".into(), id, ConsentAnswer::Send);
    assert!(first.await.unwrap());
    assert!(second.await.unwrap());
    assert_eq!(events.current(), ConsentAsk::None);
    let cookie_pushes = events
        .pushed
        .lock()
        .unwrap()
        .iter()
        .filter(|a| matches!(a, ConsentAsk::Cookie { .. }))
        .count();
    assert_eq!(cookie_pushes, 1);
}

#[test]
fn stored_format_roundtrips() {
    let json = r#"{"remembered":["route","account","handover"]}"#;
    let stored: Stored = serde_json::from_str(json).unwrap();
    assert_eq!(
        stored.remembered,
        BTreeSet::from([CookieKind::Route, CookieKind::Account, CookieKind::Handover])
    );
    assert_eq!(serde_json::to_string(&stored).unwrap(), json);
    assert_eq!(
        serde_json::from_str::<Stored>(r#"{"remembered":[]}"#).unwrap(),
        Stored::default()
    );
}

#[test]
fn decisions_log_their_old_names_and_only_sent_sends() {
    use ConsentDecision::{Denied, Sent};
    let all = [
        (
            Sent(SentWhy::RememberedUnattended),
            "sent_remembered_unattended",
        ),
        (Sent(SentWhy::Remembered), "sent_remembered"),
        (Sent(SentWhy::Flow), "sent_flow"),
        (Sent(SentWhy::Asked), "sent_asked"),
        (Denied(DeniedWhy::NoGate), "denied_no_gate"),
        (Denied(DeniedWhy::Unattended), "denied_unattended"),
        (Denied(DeniedWhy::Flow), "denied_flow"),
        (Denied(DeniedWhy::Quiet), "denied_quiet"),
        (Denied(DeniedWhy::Asked), "denied_asked"),
        (Denied(DeniedWhy::Error), "denied_error"),
    ];
    for (decision, name) in all {
        assert_eq!(decision.to_string(), name);
        assert_eq!(decision.is_sent(), name.starts_with("sent"));
    }
}

#[test]
fn a_no_is_not_remembered() {
    let core = fresh();
    let gate = yes(&core);
    gate.answers.lock().unwrap().push_back(CookieAnswer::Deny);
    assert!(!check(&core, "f1", CookieKind::Account, true));
    assert!(core.cookie_consent_remembered("f".into()).is_empty());
    // The next flow asks again (and the script's default says yes).
    assert!(check(&core, "f2", CookieKind::Account, true));
    assert_eq!(gate.asks(), 2);
}

#[test]
fn gate_receives_typed_payload() {
    let core = fresh();
    let gate = yes(&core);
    assert!(block_on(allowed(
        &core.shared,
        "f-pw",
        CookieKind::Account,
        CookiePayload::CookieAndPassword,
        "Save credentials",
        "vmate.aster0.dev",
        true,
    )));
    assert_eq!(
        *gate.payloads.lock().unwrap(),
        [CookiePayload::CookieAndPassword]
    );
}

#[test]
fn unattended_flow_is_not_attended() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    assert!(attended(&core.shared, "f"));
    events.unattended.lock().unwrap().insert("f".into());
    assert!(!attended(&core.shared, "f"));
    events.unattended.lock().unwrap().remove("f");
    assert!(attended(&core.shared, "f"));
}

#[test]
fn not_shown_is_not_attended() {
    let (core, _clock, _events) = manual_core();
    core.app_shown("f".into(), true);
    assert!(attended(&core.shared, "f"));
    core.app_shown("f".into(), false);
    assert!(!attended(&core.shared, "f"));
}

#[test]
fn asks_of_one_action_share_one_dialog() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    let mut first = Box::pin(ask_cookie(
        &core.shared,
        "f1",
        CookieKind::Route,
        CookiePayload::Cookie,
        "Fetch attendance",
        "vmate.aster0.dev",
    ));
    let mut second = Box::pin(ask_cookie(
        &core.shared,
        "f2",
        CookieKind::Route,
        CookiePayload::Cookie,
        "Fetch attendance",
        "vmate.aster0.dev",
    ));
    assert!(smol::future::block_on(smol::future::poll_once(&mut first)).is_none());
    assert!(smol::future::block_on(smol::future::poll_once(&mut second)).is_none());
    assert_eq!(core.shared.consent().cookie_prompts.len(), 1);
    let ConsentAsk::Cookie { id, .. } = events.current() else {
        panic!("expected cookie prompt");
    };
    core.consent_answer("f1".into(), id, ConsentAnswer::Send);
    assert_eq!(
        smol::future::block_on(first),
        CookieAnswer::Allow { remember: false }
    );
    assert_eq!(
        smol::future::block_on(second),
        CookieAnswer::Allow { remember: false }
    );
    assert_eq!(events.current(), ConsentAsk::None);
}

#[test]
fn payload_stays_separate_when_coalescing() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    let mut cookie_only = Box::pin(ask_cookie(
        &core.shared,
        "f1",
        CookieKind::Account,
        CookiePayload::Cookie,
        "Save",
        "vmate.aster0.dev",
    ));
    let mut with_pw = Box::pin(ask_cookie(
        &core.shared,
        "f2",
        CookieKind::Account,
        CookiePayload::CookieAndPassword,
        "Save",
        "vmate.aster0.dev",
    ));
    assert!(smol::future::block_on(smol::future::poll_once(&mut cookie_only)).is_none());
    assert!(smol::future::block_on(smol::future::poll_once(&mut with_pw)).is_none());
    assert_eq!(core.shared.consent().cookie_prompts.len(), 2);
    let ConsentAsk::Cookie { id, payload, .. } = events.current() else {
        panic!("expected first cookie prompt");
    };
    assert_eq!(payload, CookiePayload::Cookie);
    core.consent_answer("f1".into(), id, ConsentAnswer::Cancel);
    assert_eq!(smol::future::block_on(cookie_only), CookieAnswer::Deny);
    let ConsentAsk::Cookie { id, payload, .. } = events.current() else {
        panic!("expected second cookie prompt");
    };
    assert_eq!(payload, CookiePayload::CookieAndPassword);
    core.consent_answer("f2".into(), id, ConsentAnswer::Cancel);
    assert_eq!(smol::future::block_on(with_pw), CookieAnswer::Deny);
    assert_eq!(events.current(), ConsentAsk::None);
}

#[test]
fn timeout_is_cancel() {
    let (core, clock, events) = manual_core();
    core.app_shown("f".into(), true);
    core.shared.consent().wait_ms_override = Some(500);
    let mut asked = Box::pin(ask_cookie(
        &core.shared,
        "f",
        CookieKind::Account,
        CookiePayload::Cookie,
        "Load connected apps",
        "vmate.aster0.dev",
    ));
    assert!(smol::future::block_on(smol::future::poll_once(&mut asked)).is_none());
    assert!(matches!(events.current(), ConsentAsk::Cookie { .. }));
    clock.advance(499);
    assert!(smol::future::block_on(smol::future::poll_once(&mut asked)).is_none());
    clock.advance(1);
    assert_eq!(smol::future::block_on(asked), CookieAnswer::Deny);
    assert_eq!(events.current(), ConsentAsk::None);
}

#[test]
fn app_left_is_cancel() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    let mut asked = Box::pin(ask_cookie(
        &core.shared,
        "f",
        CookieKind::Handover,
        CookiePayload::Cookie,
        "Give a connected app your VTOP session",
        "vmate.aster0.dev",
    ));
    assert!(smol::future::block_on(smol::future::poll_once(&mut asked)).is_none());
    assert!(matches!(events.current(), ConsentAsk::Cookie { .. }));
    core.app_shown("f".into(), false);
    assert_eq!(smol::future::block_on(asked), CookieAnswer::Deny);
    assert_eq!(events.current(), ConsentAsk::None);
}

#[test]
fn another_action_never_rides_on_a_shown_dialog() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    let mut load = Box::pin(ask_cookie(
        &core.shared,
        "f1",
        CookieKind::Account,
        CookiePayload::Cookie,
        "Load your connected apps",
        "vmate.aster0.dev",
    ));
    let mut delete = Box::pin(ask_cookie(
        &core.shared,
        "f2",
        CookieKind::Account,
        CookiePayload::Cookie,
        "Delete your account on the server",
        "vmate.aster0.dev",
    ));
    assert!(smol::future::block_on(smol::future::poll_once(&mut load)).is_none());
    assert!(smol::future::block_on(smol::future::poll_once(&mut delete)).is_none());
    let ConsentAsk::Cookie { id, action, .. } = events.current() else {
        panic!("expected first cookie prompt");
    };
    assert_eq!(action, "Load your connected apps");
    core.consent_answer("f1".into(), id, ConsentAnswer::Send);
    assert_eq!(
        smol::future::block_on(load),
        CookieAnswer::Allow { remember: false }
    );
    let ConsentAsk::Cookie { id, action, .. } = events.current() else {
        panic!("expected second cookie prompt");
    };
    assert_eq!(action, "Delete your account on the server");
    core.consent_answer("f2".into(), id, ConsentAnswer::Cancel);
    assert_eq!(smol::future::block_on(delete), CookieAnswer::Deny);
}

#[test]
fn cancelled_ask_takes_its_dialog_away() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    {
        let mut asked = Box::pin(ask_cookie(
            &core.shared,
            "f",
            CookieKind::Account,
            CookiePayload::Cookie,
            "Load your connected apps",
            "vmate.aster0.dev",
        ));
        assert!(smol::future::block_on(smol::future::poll_once(&mut asked)).is_none());
        assert!(matches!(events.current(), ConsentAsk::Cookie { .. }));
    }
    assert_eq!(events.current(), ConsentAsk::None);
    assert!(core.shared.consent().cookie_prompts.is_empty());
}

#[test]
fn cancelled_ask_keeps_a_dialog_others_wait_on() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    let mut first = Box::pin(ask_cookie(
        &core.shared,
        "f1",
        CookieKind::Route,
        CookiePayload::Cookie,
        "Fetch attendance",
        "vmate.aster0.dev",
    ));
    let mut second = Box::pin(ask_cookie(
        &core.shared,
        "f2",
        CookieKind::Route,
        CookiePayload::Cookie,
        "Fetch attendance",
        "vmate.aster0.dev",
    ));
    assert!(smol::future::block_on(smol::future::poll_once(&mut first)).is_none());
    assert!(smol::future::block_on(smol::future::poll_once(&mut second)).is_none());
    drop(first);
    let ConsentAsk::Cookie { id, .. } = events.current() else {
        panic!("dialog should remain for second waiter");
    };
    core.consent_answer("f2".into(), id, ConsentAnswer::Send);
    assert_eq!(
        smol::future::block_on(second),
        CookieAnswer::Allow { remember: false }
    );
    assert_eq!(events.current(), ConsentAsk::None);
}

#[test]
fn ask_while_not_shown_answers_cancel_at_once() {
    let (core, _clock, events) = manual_core();
    let answer = smol::future::block_on(ask_cookie(
        &core.shared,
        "f",
        CookieKind::Account,
        CookiePayload::Cookie,
        "Load your connected apps",
        "vmate.aster0.dev",
    ));
    assert_eq!(answer, CookieAnswer::Deny);
    assert_eq!(events.count(), 0);
}

#[test]
fn unattended_without_dont_ask_again_goes_direct() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    events.unattended.lock().unwrap().insert("bg".into());
    // Unattended without "Don't ask again" denies without ever prompting (so Server falls back to Direct).
    assert!(!smol::future::block_on(allowed(
        &core.shared,
        "bg",
        CookieKind::Route,
        CookiePayload::Cookie,
        "Fetch attendance",
        "vmate.aster0.dev",
        true,
    )));
    assert_eq!(events.count(), 0);
    // With "Don't ask again" ticked for Route, unattended is allowed without prompting.
    core.remember_cookie_consent("f".into(), CookieKind::Route)
        .unwrap();
    assert!(smol::future::block_on(allowed(
        &core.shared,
        "bg",
        CookieKind::Route,
        CookiePayload::Cookie,
        "Fetch attendance",
        "vmate.aster0.dev",
        true,
    )));
    assert_eq!(events.count(), 0);
}

#[test]
fn wrong_consent_answer_variant_is_treated_as_deny() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    let mut cookie = Box::pin(ask_cookie(
        &core.shared,
        "f",
        CookieKind::Route,
        CookiePayload::Cookie,
        "Fetch",
        "vmate.aster0.dev",
    ));
    assert!(smol::future::block_on(smol::future::poll_once(&mut cookie)).is_none());
    let ConsentAsk::Cookie { id, .. } = events.current() else {
        panic!("expected cookie prompt");
    };
    core.consent_answer("f".into(), id, ConsentAnswer::Approve);
    assert_eq!(smol::future::block_on(cookie), CookieAnswer::Deny);

    let mut regno = Box::pin(ask_regno(&core.shared, "f2"));
    assert!(smol::future::block_on(smol::future::poll_once(&mut regno)).is_none());
    let ConsentAsk::Regno { id, .. } = events.current() else {
        panic!("expected regno prompt");
    };
    core.consent_answer("f2".into(), id, ConsentAnswer::Send);
    assert_eq!(smol::future::block_on(regno), RegnoAnswer::Dismissed);
}

#[test]
fn handover_expired_shows_expired() {
    let (core, clock, events) = manual_core();
    core.app_shown("f".into(), true);
    core.shared.consent().wait_ms_override = Some(60_000);
    let ask = HandoverAsk {
        request_id: "req-1".into(),
        asker: None,
        asked_at: Some(0),
        expires_at: Some(500),
        server: "vmate.aster0.dev".into(),
    };
    let mut asked = Box::pin(ask_handover(&core.shared, "f", ask.clone()));
    assert!(smol::future::block_on(smol::future::poll_once(&mut asked)).is_none());
    assert!(matches!(
        events.current(),
        ConsentAsk::Handover {
            phase: HandoverSheetPhase::Asking,
            ..
        }
    ));
    clock.advance(500);
    assert!(!smol::future::block_on(asked));
    let ConsentAsk::Handover {
        id,
        phase,
        ask: shown_ask,
        ..
    } = events.current()
    else {
        panic!("expected expired handover sheet");
    };
    assert_eq!(phase, HandoverSheetPhase::Expired);
    assert_eq!(shown_ask, ask);
    // Closing the expired sheet clears it.
    core.consent_answer("f".into(), id, ConsentAnswer::Cancel);
    assert_eq!(events.current(), ConsentAsk::None);
}

#[test]
fn regno_answer_goes_back_and_sheet_goes() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    let mut asked = Box::pin(ask_regno(&core.shared, "f"));
    assert!(smol::future::block_on(smol::future::poll_once(&mut asked)).is_none());
    let ConsentAsk::Regno { id, flow } = events.current() else {
        panic!("expected regno prompt");
    };
    assert_eq!(flow, "f");
    core.consent_answer("f".into(), id, ConsentAnswer::Share);
    assert_eq!(smol::future::block_on(asked), RegnoAnswer::Share);
    assert_eq!(events.current(), ConsentAsk::None);
}

#[test]
fn regno_not_shown_is_dismissed_at_once() {
    let (core, _clock, events) = manual_core();
    assert_eq!(
        smol::future::block_on(ask_regno(&core.shared, "f")),
        RegnoAnswer::Dismissed
    );
    assert_eq!(events.count(), 0);
}

#[test]
fn regno_timeout_is_dismissed() {
    let (core, clock, events) = manual_core();
    core.app_shown("f".into(), true);
    core.shared.consent().wait_ms_override = Some(250);
    let mut asked = Box::pin(ask_regno(&core.shared, "f"));
    assert!(smol::future::block_on(smol::future::poll_once(&mut asked)).is_none());
    clock.advance(250);
    assert_eq!(smol::future::block_on(asked), RegnoAnswer::Dismissed);
    assert_eq!(events.current(), ConsentAsk::None);
}

#[test]
fn regno_leaving_the_app_is_dismissed() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    let mut asked = Box::pin(ask_regno(&core.shared, "f"));
    assert!(smol::future::block_on(smol::future::poll_once(&mut asked)).is_none());
    core.app_shown("f".into(), false);
    assert_eq!(smol::future::block_on(asked), RegnoAnswer::Dismissed);
    assert_eq!(events.current(), ConsentAsk::None);
}

#[test]
fn regno_cancelled_ask_takes_its_sheet_away() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    {
        let mut asked = Box::pin(ask_regno(&core.shared, "f"));
        assert!(smol::future::block_on(smol::future::poll_once(&mut asked)).is_none());
        assert!(matches!(events.current(), ConsentAsk::Regno { .. }));
    }
    assert_eq!(events.current(), ConsentAsk::None);
}

#[test]
fn newer_regno_ask_closes_older() {
    let (core, _clock, events) = manual_core();
    core.app_shown("f".into(), true);
    let mut first = Box::pin(ask_regno(&core.shared, "f1"));
    assert!(smol::future::block_on(smol::future::poll_once(&mut first)).is_none());
    let mut second = Box::pin(ask_regno(&core.shared, "f2"));
    assert!(smol::future::block_on(smol::future::poll_once(&mut second)).is_none());
    assert_eq!(smol::future::block_on(first), RegnoAnswer::Dismissed);
    let ConsentAsk::Regno { id, flow } = events.current() else {
        panic!("expected second regno prompt");
    };
    assert_eq!(flow, "f2");
    core.consent_answer("f2".into(), id, ConsentAnswer::NotNow);
    assert_eq!(smol::future::block_on(second), RegnoAnswer::NotNow);
    assert_eq!(events.current(), ConsentAsk::None);
}

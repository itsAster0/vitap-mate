//! Consent before the VTOP session (cookies, and the password where a path sends it) goes to
//! our servers. Every send asks [`allowed`] first, which fails closed: no gate, no flow, a flow
//! nobody watches (the app not shown, a background run) or a gate error all count as no, except
//! that a background run follows the chosen route once the student ticked "don't ask again" for
//! [`CookieKind::Route`] (they chose that route and said not to ask). A
//! "don't ask again" tick is kept per [`CookieKind`] in [`FILE`]; an answer also covers the rest
//! of its flow for [`FLOW_GRANT_S`] (a proxied sign-in's hops ask once). Kotlin implements [`CookieGate`] (`CookieConsent.kt`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{Core, CoreError, traced_sync, unix_now};

pub(crate) const FILE: &str = "consent.json";
/// How long an answer covers the rest of its flow (one screen's pages, `sync.all`).
const FLOW_GRANT_S: i64 = 120;

/// Why the session would leave the phone; each is remembered on its own.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum CookieKind {
    /// The Server, UDP or Proxy route fetching VTOP.
    Route,
    /// The server checking who the student is (Server page, Connected apps, forwarding).
    Account,
    /// A connected app asking for a live session.
    Handover,
}

impl CookieKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Route => "route",
            Self::Account => "account",
            Self::Handover => "handover",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CookieAnswer {
    pub send: bool,
    /// Don't ask again for this kind.
    pub remember: bool,
}

/// Kotlin: whether a flow has the student on screen, and the dialog.
#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait CookieGate: Send + Sync {
    /// The app is shown and `flow` is not a background run.
    fn attended(&self, flow: String) -> bool;
    /// Shows the dialog: `action` is the why ("Set up mail forwarding"), `server` the host it goes
    /// to. A timeout or the app leaving the screen answers no.
    async fn ask(
        &self,
        flow: String,
        kind: CookieKind,
        action: String,
        server: String,
    ) -> Result<CookieAnswer, CoreError>;
}

#[derive(Default, Serialize, Deserialize)]
struct Saved {
    remembered: BTreeSet<CookieKind>,
}

/// Global, as the route is.
struct State {
    gate: Option<Arc<dyn CookieGate>>,
    /// `consent.json`, once the core opened.
    path: Option<PathBuf>,
    remembered: BTreeSet<CookieKind>,
    /// The answer in a flow, and when (unix seconds).
    answered: BTreeMap<(String, CookieKind), (i64, bool)>,
}

const NONE: State = State {
    gate: None,
    path: None,
    remembered: BTreeSet::new(),
    answered: BTreeMap::new(),
};

#[cfg(not(test))]
fn state() -> MutexGuard<'static, State> {
    static STATE: Mutex<State> = Mutex::new(NONE);
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Per test thread, so tests running at once don't share a gate.
#[cfg(test)]
fn state() -> MutexGuard<'static, State> {
    thread_local!(static STATE: &'static Mutex<State> = Box::leak(Box::new(Mutex::new(NONE))));
    STATE
        .with(|state| *state)
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// Asks `gate` before any VTOP session leaves the phone; call before [`crate::Core::new`].
#[uniffi::export]
pub fn set_cookie_gate(gate: Arc<dyn CookieGate>) {
    info!("cookie gate: foreign");
    state().gate = Some(gate);
}

/// Loads the remembered kinds; called when the core opens.
pub(crate) fn load(core: &Core) {
    let remembered = core.read::<Saved>(FILE).unwrap_or_default().remembered;
    let mut state = state();
    (state.path, state.remembered) = (Some(core.at(FILE)), remembered);
}

/// Forgets the remembered kinds and this session's answers (the file goes with `Core::forget`).
pub(crate) fn forget() {
    let mut state = state();
    state.remembered.clear();
    state.answered.clear();
}

fn save(state: &State) {
    let Some(path) = &state.path else { return };
    let saved = Saved {
        remembered: state.remembered.clone(),
    };
    let result = serde_json::to_vec(&saved)
        .map_err(std::io::Error::other)
        .and_then(|bytes| fs::write(path, bytes));
    if let Err(error) = result {
        warn!(%error, "consent not saved");
    }
}

/// Whether the session may go to `server` for `action` now. With `ask` false (a call the
/// student didn't start) only a remembered yes sends. Logs the decision, never the session.
pub(crate) async fn allowed(
    flow: &str,
    kind: CookieKind,
    action: &str,
    server: &str,
    ask: bool,
) -> bool {
    let decision = decide(flow, kind, action, server, ask).await;
    info!(kind = kind.as_str(), decision, "cookie consent");
    decision.starts_with("sent")
}

async fn decide(
    flow: &str,
    kind: CookieKind,
    action: &str,
    server: &str,
    ask: bool,
) -> &'static str {
    let gate = {
        let mut state = state();
        let now = unix_now();
        state.answered.retain(|_, (at, _)| now - *at < FLOW_GRANT_S);
        match &state.gate {
            Some(gate) if !flow.is_empty() => gate.clone(),
            _ => return "denied_no_gate",
        }
    };
    let attended = gate.attended(flow.to_string());
    {
        let state = state();
        if !attended {
            // Only the route: account checks and handovers always have the student watching.
            return if kind == CookieKind::Route && state.remembered.contains(&kind) {
                "sent_remembered_unattended"
            } else {
                "denied_unattended"
            };
        }
        if state.remembered.contains(&kind) {
            return "sent_remembered";
        }
        // Checked before `!ask`: a quiet call rides on a yes earlier in its flow. That holds
        // while one flow sends for one action only.
        match state.answered.get(&(flow.to_string(), kind)) {
            Some((_, true)) => return "sent_flow",
            Some((_, false)) => return "denied_flow",
            None => {}
        }
    }
    if !ask {
        return "denied_quiet";
    }
    let answer = gate
        .ask(
            flow.to_string(),
            kind,
            action.to_string(),
            server.to_string(),
        )
        .await;
    let send = answer.as_ref().is_ok_and(|answer| answer.send);
    let mut state = state();
    state
        .answered
        .insert((flow.to_string(), kind), (unix_now(), send));
    match answer {
        Ok(CookieAnswer {
            send: true,
            remember,
        }) => {
            if remember {
                state.remembered.insert(kind);
                save(&state);
            }
            "sent_asked"
        }
        Ok(_) => "denied_asked",
        Err(_) => "denied_error",
    }
}

/// The host of `url`, for the dialog ("vmate.aster0.dev").
pub(crate) fn host(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.split(['/', '?']).next().unwrap_or(rest).to_string()
}

#[uniffi::export]
impl Core {
    /// The kinds the student ticked "don't ask again" for.
    pub fn cookie_consent_remembered(&self, flow: String) -> Vec<CookieKind> {
        let _ = traced_sync("cookie_consent_remembered", &flow, || Ok(()));
        state().remembered.iter().copied().collect()
    }

    /// Asks every time again.
    pub fn reset_cookie_consent(&self, flow: String) -> Result<(), CoreError> {
        traced_sync("reset_cookie_consent", &flow, || {
            forget();
            match fs::remove_file(self.at(FILE)) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
                _ => {
                    info!("cookie consent reset");
                    Ok(())
                }
            }
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::VecDeque;
    use std::fmt::Write as _;
    use std::future::Future;
    use std::sync::atomic::{AtomicU32, Ordering};

    use tracing::field::{Field, Visit};
    use tracing::span::Attributes;
    use tracing::{Event, Id, Subscriber};
    use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
    use vtop_core::sync::RwLock;

    use super::*;
    use crate::{ACCOUNT, Account, Session};

    /// Runs `future` on this thread (the gate is per test thread).
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
        pub(crate) attended: bool,
        pub(crate) deny: bool,
        pub(crate) answers: Mutex<VecDeque<CookieAnswer>>,
        pub(crate) asks: AtomicU32,
    }

    impl TestGate {
        pub(crate) fn asks(&self) -> u32 {
            self.asks.load(Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl CookieGate for TestGate {
        fn attended(&self, _flow: String) -> bool {
            self.attended
        }
        async fn ask(
            &self,
            _flow: String,
            _kind: CookieKind,
            _action: String,
            _server: String,
        ) -> Result<CookieAnswer, CoreError> {
            self.asks.fetch_add(1, Ordering::SeqCst);
            let next = self.answers.lock().unwrap().pop_front();
            Ok(next.unwrap_or(CookieAnswer {
                send: !self.deny,
                remember: false,
            }))
        }
    }

    /// Installs a fresh gate (and no remembered kinds) for this test thread.
    pub(crate) fn install(attended: bool, deny: bool) -> Arc<TestGate> {
        let gate = Arc::new(TestGate {
            attended,
            deny,
            answers: Mutex::default(),
            asks: AtomicU32::new(0),
        });
        *state() = NONE;
        set_cookie_gate(gate.clone());
        gate
    }

    /// An attended gate that says yes to everything.
    pub(crate) fn yes() -> Arc<TestGate> {
        install(true, false)
    }

    /// An attended gate that says no to everything.
    pub(crate) fn no() -> Arc<TestGate> {
        install(true, true)
    }

    /// Gives `core` a session (never used against VTOP before the gate).
    pub(crate) fn signed_in(core: &Core) {
        let account = Account {
            username: "22BCE0001".into(),
            password: "pw".into(),
            rejected: false,
            registration_number: None,
        };
        let session = Session::new(account).unwrap();
        core.install(Some(Arc::new(RwLock::new(session))));
    }

    pub(crate) fn remember(kind: CookieKind) {
        state().remembered.insert(kind);
    }

    fn core(name: &str) -> Core {
        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-consent-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let core = Core {
            cache: dir.clone(),
            dir,
            slot: Mutex::new(None),
        };
        load(&core);
        core
    }

    fn check(flow: &str, kind: CookieKind, ask: bool) -> bool {
        block_on(allowed(flow, kind, "Test", "vmate.aster0.dev", ask))
    }

    const YES_REMEMBER: CookieAnswer = CookieAnswer {
        send: true,
        remember: true,
    };

    #[test]
    fn no_gate_denies() {
        *state() = NONE;
        assert!(!check("f", CookieKind::Route, true));
    }

    #[test]
    fn empty_flow_denies() {
        let gate = yes();
        assert!(!check("", CookieKind::Route, true));
        assert_eq!(gate.asks(), 0);
    }

    #[test]
    fn unattended_denies_unless_route_remembered() {
        let gate = install(false, false);
        assert!(!check("f", CookieKind::Route, true));
        remember(CookieKind::Account);
        remember(CookieKind::Handover);
        assert!(!check("f", CookieKind::Account, true));
        assert!(!check("f", CookieKind::Handover, true));
        remember(CookieKind::Route);
        assert!(check("f", CookieKind::Route, true));
        assert_eq!(gate.asks(), 0);
    }

    #[test]
    fn remembered_kind_sends_without_asking() {
        let gate = no();
        remember(CookieKind::Account);
        assert!(check("f", CookieKind::Account, true));
        assert!(check("f", CookieKind::Account, false));
        assert_eq!(gate.asks(), 0);
    }

    #[test]
    fn remember_saves_only_that_kind() {
        let core = core("remember");
        let gate = yes();
        load(&core);
        gate.answers.lock().unwrap().push_back(YES_REMEMBER);
        assert!(check("f1", CookieKind::Handover, true));
        assert_eq!(
            core.cookie_consent_remembered("f".into()),
            [CookieKind::Handover]
        );
        // Saved: a new start reads it back.
        state().remembered.clear();
        load(&core);
        assert!(check("f2", CookieKind::Handover, false));
        // Another kind still asks.
        assert!(check("f2", CookieKind::Account, true));
        assert_eq!(gate.asks(), 2);
        let _ = fs::remove_dir_all(&core.dir);
    }

    #[test]
    fn yes_without_remember_covers_the_flow_only() {
        let gate = yes();
        assert!(check("f1", CookieKind::Route, true));
        assert!(check("f1", CookieKind::Route, true));
        assert_eq!(gate.asks(), 1);
        assert!(check("f2", CookieKind::Route, true));
        assert_eq!(gate.asks(), 2);
        // A no covers the flow too, and isn't kept past it.
        let gate = no();
        assert!(!check("f3", CookieKind::Route, true));
        assert!(!check("f3", CookieKind::Route, true));
        assert_eq!(gate.asks(), 1);
        assert!(!check("f4", CookieKind::Route, true));
        assert_eq!(gate.asks(), 2);
    }

    #[test]
    fn quiet_never_asks() {
        let gate = yes();
        assert!(!check("f", CookieKind::Handover, false));
        assert_eq!(gate.asks(), 0);
    }

    #[test]
    fn reset_forgets_remembered_kinds() {
        let core = core("reset");
        let gate = yes();
        load(&core);
        gate.answers.lock().unwrap().push_back(YES_REMEMBER);
        assert!(check("f1", CookieKind::Route, true));
        assert!(core.at(FILE).exists());
        core.reset_cookie_consent("f".into()).unwrap();
        assert!(core.cookie_consent_remembered("f".into()).is_empty());
        assert!(!core.at(FILE).exists());
        // The flow's yes went too.
        assert!(!check("f1", CookieKind::Route, false));
        let _ = fs::remove_dir_all(&core.dir);
    }

    #[test]
    fn another_student_forgets_remembered_kinds() {
        let core = core("switch");
        let gate = yes();
        load(&core);
        gate.answers.lock().unwrap().push_back(YES_REMEMBER);
        assert!(check("f", CookieKind::Handover, true));
        fs::write(
            core.at(ACCOUNT),
            r#"{"username":"21BCE0001","password":"x"}"#,
        )
        .unwrap();
        let account = Account {
            username: "21BCE0002".into(),
            password: "pw".into(),
            rejected: false,
            registration_number: None,
        };
        let slot = Arc::new(RwLock::new(Session::new(account).unwrap()));
        core.install(Some(slot.clone()));
        core.save(&slot, &block_on(slot.read()));
        assert!(core.cookie_consent_remembered("f".into()).is_empty());
        assert!(!core.at(FILE).exists());
        // Signing out forgets them too.
        gate.answers.lock().unwrap().push_back(YES_REMEMBER);
        assert!(check("f2", CookieKind::Route, true));
        core.sign_out("f".into());
        assert!(core.cookie_consent_remembered("f".into()).is_empty());
        let _ = fs::remove_dir_all(&core.dir);
    }

    #[test]
    fn consent_lines_carry_kind_and_decision_and_no_secret() {
        let captured = Captured::default();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let subscriber = tracing_subscriber::registry().with(captured.clone());
        tracing::subscriber::with_default(subscriber, || {
            let _gate = install(false, false);
            block_on(allowed(
                "f",
                CookieKind::Route,
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
}

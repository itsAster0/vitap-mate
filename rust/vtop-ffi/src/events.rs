//! What the app hears from the core without asking ([`AppEvents`]), the platform services the
//! core calls ([`SecretBox`], [`UrlOpener`]), and whether the app is on screen
//! ([`Core::app_shown`]).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, PoisonError};

use tracing::{info, warn};

use crate::problem::Problem;
use crate::remote::consent::ConsentAsk;
use crate::remote::handover::HandoverStatus;
use crate::schedule::{ActivityEntry, Schedule};
use crate::semester_choice::SemesterState;
use crate::sync_health::SyncHealth;
use crate::{Core, span};

/// App-wide pushes; Kotlin implements it. Called on the core's threads, never the main one: the
/// app hops to its main thread before touching UI state. A call that panics or throws is logged
/// and dropped ([`Events::tell`]).
#[uniffi::export(with_foreign)]
pub trait AppEvents: Send + Sync {
    /// A failure of automatic work with no screen of its own (the sweep, the calendar and
    /// exam-day refreshes).
    fn on_background_problem(&self, problem: Problem);
    /// The tracked runs: running ones oldest first, then finished ones newest first.
    fn on_activity(&self, entries: Vec<ActivityEntry>);
    /// The consent prompt to show now, or [`ConsentAsk::None`] when none is open.
    fn on_consent(&self, ask: ConsentAsk);
    /// How an approved connected-app handover is going, for its notification.
    fn on_handover_status(&self, flow: String, request_id: String, status: HandoverStatus);
    /// Whether `flow` is a background (unattended) flow started in Kotlin (`flow(..., unattended = true)`).
    fn is_flow_unattended(&self, flow: String) -> bool;
    /// The semester list and pick, after every change.
    fn on_semesters(&self, state: SemesterState);
    /// How background sync stands, after every change and when the app starts listening;
    /// `paused`: why, as shown ("Paused: …").
    fn on_sync_health(&self, health: SyncHealth, paused: Option<String>);
}

/// Why a [`SecretBox`] call failed (the Keystore didn't answer); never holds the secret.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum SecretError {
    Unavailable { reason: String },
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { reason } => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for SecretError {}

impl From<uniffi::UnexpectedUniFFICallbackError> for SecretError {
    fn from(error: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Unavailable {
            reason: error.reason,
        }
    }
}

/// Small secret files sealed by the platform (the Android Keystore's AES-GCM), one per `name`.
#[uniffi::export(with_foreign)]
pub trait SecretBox: Send + Sync {
    fn seal(&self, name: String, bytes: Vec<u8>) -> Result<(), SecretError>;
    /// None when there is none, or it no longer opens (the key went with a restore).
    fn open(&self, name: String) -> Result<Option<Vec<u8>>, SecretError>;
}

/// Opens `url` in the browser (Gmail's sign-in page).
#[uniffi::export(with_foreign)]
pub trait UrlOpener: Send + Sync {
    fn open(&self, url: String);
}

/// Activities on screen and how often the app came back. A new activity can start before the
/// old one stops, so starts are counted, not flagged.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Shown {
    started: u32,
    left: bool,
    resumes: u32,
}

impl Shown {
    /// After an activity's `onStart` (`shown`) or `onStop`. Only a start after the app left
    /// counts as a resume: not the first, nor a second activity over a shown one.
    pub(crate) fn next(self, shown: bool) -> Self {
        if shown {
            Self {
                started: self.started.saturating_add(1),
                resumes: if self.started == 0 && self.left {
                    self.resumes.saturating_add(1)
                } else {
                    self.resumes
                },
                ..self
            }
        } else {
            let started = self.started.saturating_sub(1);
            Self {
                started,
                left: self.left || started == 0,
                ..self
            }
        }
    }

    pub(crate) fn on_screen(self) -> bool {
        self.started > 0
    }
}

/// A core's events and its schedule, whose activity tracker pushes through them.
pub(crate) fn wired(settings: Arc<crate::settings::SettingsStore>) -> (Arc<Events>, Schedule) {
    wired_on(Arc::new(crate::runtime::Runtime::default()), settings)
}

/// [`wired`] on `runtime` (tests give theirs a manual clock).
pub(crate) fn wired_on(
    runtime: Arc<crate::runtime::Runtime>,
    settings: Arc<crate::settings::SettingsStore>,
) -> (Arc<Events>, Schedule) {
    let events = Arc::new(Events::default());
    let pushing = events.clone();
    let schedule = Schedule::on(
        runtime,
        Arc::new(move |entries| {
            pushing.tell("activity", |listener| listener.on_activity(entries));
        }),
        settings,
    );
    (events, schedule)
}

/// [`Core`]'s listener and [`Shown`].
#[derive(Default)]
pub(crate) struct Events {
    listener: Mutex<Option<Arc<dyn AppEvents>>>,
    shown: Mutex<Shown>,
    /// Page watches, told each time the app comes back ([`Events::resumed`]).
    resumed: Mutex<Vec<smol::channel::Sender<()>>>,
    /// Flows Rust runs unattended itself (background sync), whatever Kotlin says.
    unattended: Mutex<std::collections::HashSet<String>>,
}

impl Events {
    /// `push` on the listener, if any. A panic (a Kotlin throw arrives as one) is logged and
    /// stops here, so the task that pushed carries on.
    pub(crate) fn tell(&self, what: &str, push: impl FnOnce(&dyn AppEvents)) {
        let listener = self
            .listener
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let Some(listener) = listener else { return };
        if catch_unwind(AssertUnwindSafe(|| push(listener.as_ref()))).is_err() {
            warn!(event = what, "app listener failed");
        }
    }

    pub(crate) fn has_listener(&self) -> bool {
        self.listener
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
    }

    pub(crate) fn set_listener(&self, events: Arc<dyn AppEvents>) {
        *self.listener.lock().unwrap_or_else(PoisonError::into_inner) = Some(events);
    }

    pub(crate) fn on_screen(&self) -> bool {
        self.shown
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .on_screen()
    }

    /// Fails closed (returns true / unattended) if the foreign callback panics.
    pub(crate) fn is_flow_unattended(&self, flow: &str) -> bool {
        if self
            .unattended
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(flow)
        {
            return true;
        }
        let listener = self
            .listener
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let Some(listener) = listener else {
            return false;
        };
        match catch_unwind(AssertUnwindSafe(|| {
            listener.is_flow_unattended(flow.to_string())
        })) {
            Ok(unattended) => unattended,
            Err(_) => {
                warn!(event = "is_flow_unattended", "app listener failed");
                true
            }
        }
    }

    /// Marks `flow` unattended (`on`) or not.
    pub(crate) fn set_unattended(&self, flow: &str, on: bool) {
        let mut flows = self
            .unattended
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if on {
            flows.insert(flow.to_owned());
        } else {
            flows.remove(flow);
        }
    }

    #[cfg(test)]
    pub(crate) fn set_shown_for_test(&self, shown: bool) {
        let mut state = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
        state.started = u32::from(shown);
        if !shown {
            state.left = true;
        }
    }

    fn shown(&self, shown: bool) -> (bool, Shown) {
        let (before, now) = {
            let mut state = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
            let before = *state;
            *state = state.next(shown);
            (before, *state)
        };
        if now.resumes != before.resumes {
            // Unbounded: a send fails only for a watch that closed, which is dropped here.
            self.resumed
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .retain(|watch| watch.try_send(()).is_ok());
        }
        (before.on_screen(), now)
    }

    /// Told (with nothing) each time the app comes back on screen, until dropped.
    pub(crate) fn resumed(&self) -> smol::channel::Receiver<()> {
        let (watch, resumes) = smol::channel::unbounded();
        self.resumed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(watch);
        resumes
    }
}

#[uniffi::export]
impl Core {
    /// Who hears the app-wide pushes; one listener, the last one given.
    pub fn set_events(&self, events: Arc<dyn AppEvents>) {
        self.events.set_listener(events);
        crate::remote::consent::push_current(&self.shared);
        self.push_sync_health();
    }

    /// An activity started (`shown`) or stopped.
    pub fn app_shown(&self, flow: String, shown: bool) {
        let (was_on_screen, now) = self.events.shown(shown);
        let on_screen = now.on_screen();
        span("app_shown", &flow).in_scope(|| {
            info!(shown, on_screen, resumes = now.resumes, "app_shown");
        });
        if was_on_screen && !on_screen {
            crate::remote::consent::on_app_left(&self.shared);
        }
        if was_on_screen != on_screen {
            self.on_live_app_shown(on_screen);
        }
    }

    /// Whether any activity is currently on screen.
    pub fn is_shown(&self) -> bool {
        self.events.on_screen()
    }

    /// How often the app came back on screen since it started.
    pub fn resumes(&self) -> u32 {
        self.events
            .shown
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .resumes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::ProblemKind;

    fn after(steps: &[bool]) -> Shown {
        steps.iter().fold(Shown::default(), |s, &on| s.next(on))
    }

    #[test]
    fn app_shown_counts_resumes_only_after_leaving() {
        // The first start is not a comeback.
        assert_eq!(after(&[true]).resumes, 0);
        assert!(after(&[true]).on_screen());
        assert!(!after(&[true, false]).on_screen());
        assert_eq!(after(&[true, false, true]).resumes, 1);
        // A second start without a stop doesn't count.
        assert_eq!(after(&[true, false, true, true]).resumes, 1);
    }

    #[test]
    fn an_old_activity_stopping_after_a_new_one_started_leaves_the_app_shown() {
        // A second MainActivity (from the launcher); the first one's onStop comes after.
        let both = after(&[true, true, false]);
        assert!(both.on_screen());
        assert!(!both.next(false).on_screen());
        // It never left, so the next start is no comeback... until both stopped.
        assert_eq!(after(&[true, true, false, true]).resumes, 0);
        assert_eq!(after(&[true, true, false, false, true]).resumes, 1);
    }

    struct Panics;

    impl AppEvents for Panics {
        fn on_background_problem(&self, _problem: Problem) {
            panic!("listener threw");
        }

        fn on_activity(&self, _entries: Vec<ActivityEntry>) {
            panic!("listener threw");
        }

        fn on_consent(&self, _ask: ConsentAsk) {
            panic!("listener threw");
        }

        fn on_handover_status(&self, _flow: String, _request_id: String, _status: HandoverStatus) {
            panic!("listener threw");
        }

        fn is_flow_unattended(&self, _flow: String) -> bool {
            panic!("listener threw");
        }

        fn on_semesters(&self, _state: SemesterState) {
            panic!("listener threw");
        }

        fn on_sync_health(&self, _health: SyncHealth, _paused: Option<String>) {
            panic!("listener threw");
        }
    }

    struct Counts(Mutex<u32>);

    impl AppEvents for Counts {
        fn on_background_problem(&self, _problem: Problem) {
            *self.0.lock().unwrap() += 1;
        }

        fn on_activity(&self, _entries: Vec<ActivityEntry>) {}

        fn on_consent(&self, _ask: ConsentAsk) {}

        fn on_handover_status(&self, _flow: String, _request_id: String, _status: HandoverStatus) {}

        fn is_flow_unattended(&self, _flow: String) -> bool {
            false
        }

        fn on_semesters(&self, _state: SemesterState) {}

        fn on_sync_health(&self, _health: SyncHealth, _paused: Option<String>) {}
    }

    fn offline() -> Problem {
        Problem {
            kind: ProblemKind::Offline,
            title: "Offline".into(),
            message: "Connect and try again.".into(),
            flow: "f1".into(),
            retryable: true,
            user_initiated: false,
        }
    }

    #[test]
    fn listener_panic_is_caught_and_logged() {
        let core = Arc::new(Core::for_test(std::env::temp_dir()));
        core.set_events(Arc::new(Panics));
        let pushing = core.clone();
        let task = core.schedule.runtime.spawn("f1", async move {
            pushing
                .events
                .tell("background_problem", |e| e.on_background_problem(offline()));
            "carried on"
        });
        assert_eq!(smol::future::block_on(task), "carried on");

        // The pool still runs work, and a later listener hears it.
        let counts = Arc::new(Counts(Mutex::new(0)));
        core.set_events(counts.clone());
        let pushing = core.clone();
        smol::future::block_on(core.schedule.runtime.spawn("f2", async move {
            pushing
                .events
                .tell("background_problem", |e| e.on_background_problem(offline()));
        }));
        assert_eq!(*counts.0.lock().unwrap(), 1);
    }

    #[test]
    fn resumes_follow_app_shown() {
        let core = Core::for_test(std::env::temp_dir());
        core.app_shown("f".into(), true);
        core.app_shown("f".into(), false);
        core.app_shown("f".into(), true);
        assert_eq!(core.resumes(), 1);
    }
}

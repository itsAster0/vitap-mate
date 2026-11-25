//! Consent before the VTOP session (cookies, and the password where a path sends it) goes to
//! our servers. Every send asks [`allowed`] first, which fails closed: no gate, no flow, a flow
//! nobody watches (the app not shown, a background run) or a gate error all count as no, except
//! that a background run follows the chosen route once the student ticked "don't ask again" for
//! [`CookieKind::Route`] (they chose that route and said not to ask). A
//! "don't ask again" tick is kept per [`CookieKind`] in [`FILE`]; an answer also covers the rest
//! of its flow for [`FLOW_GRANT_S`] (a proxied sign-in's hops ask once). Kotlin implements [`CookieGate`] (`CookieConsent.kt`).
//! The state is the core's ([`Shared::consent`]), so [`allowed`] takes it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, MutexGuard, PoisonError};

use futures::channel::oneshot;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::remote::regno::RegnoAnswer;
use crate::{Core, CoreError, Shared, store, traced_sync, unix_now};

pub(crate) const FILE: &str = "consent.json";
/// How long an answer covers the rest of its flow (one screen's pages, `sync.all`).
const FLOW_GRANT_S: i64 = 120;

/// Why the session would leave the phone; each is remembered on its own.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum CookieKind {
    /// The Server or UDP route fetching VTOP.
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

/// What leaves the phone when cookie consent is granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum CookiePayload {
    /// Only the VTOP session cookie.
    Cookie,
    /// The VTOP session cookie and the saved password.
    CookieAndPassword,
}

/// The student's answer to the dialog. A no is never remembered, so it carries no tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CookieAnswer {
    Deny,
    Allow {
        /// Don't ask again for this kind.
        remember: bool,
    },
}

/// Why [`allowed`] said yes or no; logged as `decision`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsentDecision {
    Sent(SentWhy),
    Denied(DeniedWhy),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SentWhy {
    /// A background run on the route the student ticked "don't ask again" for.
    RememberedUnattended,
    Remembered,
    /// A yes earlier in the same flow.
    Flow,
    Asked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeniedWhy {
    NoGate,
    Unattended,
    /// A no earlier in the same flow.
    Flow,
    /// A call the student didn't start, with nothing remembered.
    Quiet,
    Asked,
    /// The dialog failed.
    Error,
}

impl ConsentDecision {
    pub(crate) fn is_sent(self) -> bool {
        matches!(self, Self::Sent(_))
    }

    /// The log's spelling, kept stable for anyone reading old logs.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Sent(SentWhy::RememberedUnattended) => "sent_remembered_unattended",
            Self::Sent(SentWhy::Remembered) => "sent_remembered",
            Self::Sent(SentWhy::Flow) => "sent_flow",
            Self::Sent(SentWhy::Asked) => "sent_asked",
            Self::Denied(DeniedWhy::NoGate) => "denied_no_gate",
            Self::Denied(DeniedWhy::Unattended) => "denied_unattended",
            Self::Denied(DeniedWhy::Flow) => "denied_flow",
            Self::Denied(DeniedWhy::Quiet) => "denied_quiet",
            Self::Denied(DeniedWhy::Asked) => "denied_asked",
            Self::Denied(DeniedWhy::Error) => "denied_error",
        }
    }
}

impl std::fmt::Display for ConsentDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
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
        payload: CookiePayload,
        action: String,
        server: String,
    ) -> Result<CookieAnswer, CoreError>;
    /// Shows the "Share your registration number?" sheet ([`crate::remote::regno`]). A timeout
    /// or the app leaving the screen is [`RegnoAnswer::Dismissed`].
    async fn ask_regno(&self, flow: String) -> Result<RegnoAnswer, CoreError>;
}

/// [`FILE`] on disk: `{"remembered":["route","account","handover"]}`. The kinds' names are
/// [`CookieKind`]'s serde names; `stored_format_roundtrips` pins them.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Stored {
    remembered: BTreeSet<CookieKind>,
}

/// The core's consent state ([`Shared::consent`]).
#[derive(Default)]
pub(crate) struct State {
    pub(crate) gate: Option<Arc<dyn CookieGate>>,
    /// `consent.json`, once the core opened.
    path: Option<PathBuf>,
    pub(crate) remembered: BTreeSet<CookieKind>,
    /// The answer in a flow, and when (unix seconds).
    answered: BTreeMap<(String, CookieKind), (i64, bool)>,
    /// Waiters for an in-flight `gate.ask` of `(flow, kind)`, so concurrent calls share one prompt.
    in_flight: BTreeMap<(String, CookieKind), Vec<oneshot::Sender<ConsentDecision>>>,
}

impl Shared {
    pub(crate) fn consent(&self) -> MutexGuard<'_, State> {
        self.consent.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Loads the remembered kinds; called when the core opens.
pub(crate) fn load(core: &Core) {
    let remembered = core.read::<Stored>(FILE).unwrap_or_default().remembered;
    let mut state = core.shared.consent();
    (state.path, state.remembered) = (Some(core.at(FILE)), remembered);
}

/// Forgets the remembered kinds and this session's answers (the file goes with `Core::forget`).
pub(crate) fn forget(shared: &Shared) {
    let mut state = shared.consent();
    state.remembered.clear();
    state.answered.clear();
}

/// Writes the remembered kinds with `state` held, so two saves land in the order they were
/// made, through a temporary file, so a crash leaves the old ticks or the new ones.
fn save(state: &State) {
    let Some(path) = &state.path else {
        return;
    };
    let stored = Stored {
        remembered: state.remembered.clone(),
    };
    let result = serde_json::to_vec(&stored)
        .map_err(std::io::Error::other)
        .and_then(|bytes| store::write_atomic(path, &bytes));
    if let Err(error) = result {
        warn!(%error, "consent not saved");
    }
}

/// Removes an `in_flight` slot on drop if the leader future is cancelled before completing.
struct InFlightGuard<'a> {
    shared: &'a Shared,
    key: Option<(String, CookieKind)>,
}

impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            self.shared.consent().in_flight.remove(&key);
        }
    }
}

/// Whether `flow` has the student on screen (no gate or no flow: no).
pub(crate) fn attended(shared: &Shared, flow: &str) -> bool {
    let gate = shared.consent().gate.clone();
    gate.is_some_and(|gate| !flow.is_empty() && gate.attended(flow.to_string()))
}

/// Whether the session may go to `server` for `action` now. With `ask` false (a call the
/// student didn't start) only a remembered yes sends. Logs the decision, never the session.
pub(crate) async fn allowed(
    shared: &Shared,
    flow: &str,
    kind: CookieKind,
    payload: CookiePayload,
    action: &str,
    server: &str,
    ask: bool,
) -> bool {
    let decision = decide(shared, flow, kind, payload, action, server, ask).await;
    info!(
        kind = kind.as_str(),
        decision = decision.as_str(),
        "cookie consent"
    );
    decision.is_sent()
}

async fn decide(
    shared: &Shared,
    flow: &str,
    kind: CookieKind,
    payload: CookiePayload,
    action: &str,
    server: &str,
    ask: bool,
) -> ConsentDecision {
    use ConsentDecision::{Denied, Sent};
    let key = (flow.to_string(), kind);
    loop {
        let gate = {
            let mut state = shared.consent();
            let now = unix_now();
            state.answered.retain(|_, (at, _)| now - *at < FLOW_GRANT_S);
            match &state.gate {
                Some(gate) if !flow.is_empty() => gate.clone(),
                _ => return Denied(DeniedWhy::NoGate),
            }
        };
        let attended = gate.attended(flow.to_string());
        let wait_rx = {
            let mut state = shared.consent();
            if !attended {
                // Only the route: account checks and handovers always have the student watching.
                return if kind == CookieKind::Route && state.remembered.contains(&kind) {
                    Sent(SentWhy::RememberedUnattended)
                } else {
                    Denied(DeniedWhy::Unattended)
                };
            }
            if state.remembered.contains(&kind) {
                return Sent(SentWhy::Remembered);
            }
            // Checked before `!ask`: a quiet call rides on a yes earlier in its flow. That holds
            // while one flow sends for one action only.
            match state.answered.get(&key) {
                Some((_, true)) => return Sent(SentWhy::Flow),
                Some((_, false)) => return Denied(DeniedWhy::Flow),
                None => {}
            }
            if !ask {
                return Denied(DeniedWhy::Quiet);
            }
            if let Some(waiters) = state.in_flight.get_mut(&key) {
                let (tx, rx) = oneshot::channel();
                waiters.push(tx);
                Some(rx)
            } else {
                state.in_flight.insert(key.clone(), Vec::new());
                None
            }
        };
        if let Some(rx) = wait_rx {
            // Leader completed -> use its decision; leader cancelled -> loop and re-evaluate.
            if let Ok(decision) = rx.await {
                return decision;
            }
            continue;
        }
        let mut guard = InFlightGuard {
            shared,
            key: Some(key.clone()),
        };
        let answer = gate
            .ask(
                flow.to_string(),
                kind,
                payload,
                action.to_string(),
                server.to_string(),
            )
            .await;
        let send = matches!(answer, Ok(CookieAnswer::Allow { .. }));
        let (decision, waiters) = {
            let mut state = shared.consent();
            state.answered.insert(key.clone(), (unix_now(), send));
            if let Ok(CookieAnswer::Allow { remember: true }) = &answer {
                state.remembered.insert(kind);
                save(&state);
            }
            let waiters = state.in_flight.remove(&key).unwrap_or_default();
            guard.key = None;
            let decision = match answer {
                Ok(CookieAnswer::Allow { .. }) => Sent(SentWhy::Asked),
                Ok(CookieAnswer::Deny) => Denied(DeniedWhy::Asked),
                Err(_) => Denied(DeniedWhy::Error),
            };
            (decision, waiters)
        };
        let waiter_decision = if send {
            Sent(SentWhy::Flow)
        } else {
            Denied(DeniedWhy::Flow)
        };
        for tx in waiters {
            #[expect(
                clippy::let_underscore_must_use,
                reason = "waiter dropped its future before we replied"
            )]
            let _ = tx.send(waiter_decision);
        }
        return decision;
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
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("cookie_consent_remembered", &flow, || Ok(()));
        self.shared.consent().remembered.iter().copied().collect()
    }

    /// Remembers a yes for `kind` given in the app's own words, as "Don't ask again" in the
    /// dialog would (choosing Server fetch already says the cookie goes to the server).
    pub fn remember_cookie_consent(&self, flow: String, kind: CookieKind) -> Result<(), CoreError> {
        traced_sync("remember_cookie_consent", &flow, || {
            {
                let mut state = self.shared.consent();
                state.remembered.insert(kind);
                save(&state);
            }
            info!(?kind, "cookie consent remembered");
            Ok(())
        })
    }

    /// Asks every time again.
    pub fn reset_cookie_consent(&self, flow: String) -> Result<(), CoreError> {
        traced_sync("reset_cookie_consent", &flow, || {
            forget(&self.shared);
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
pub(crate) mod tests;

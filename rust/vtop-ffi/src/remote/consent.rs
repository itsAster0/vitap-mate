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
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{Core, CoreError, Shared, traced_sync, unix_now};

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

/// The core's consent state ([`Shared::consent`]).
#[derive(Default)]
pub(crate) struct State {
    pub(crate) gate: Option<Arc<dyn CookieGate>>,
    /// `consent.json`, once the core opened.
    path: Option<PathBuf>,
    pub(crate) remembered: BTreeSet<CookieKind>,
    /// The answer in a flow, and when (unix seconds).
    answered: BTreeMap<(String, CookieKind), (i64, bool)>,
}

impl Shared {
    pub(crate) fn consent(&self) -> MutexGuard<'_, State> {
        self.consent.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What [`set_cookie_gate`] set, for [`Core::new`] to take a copy of.
static GATE: Mutex<Option<Arc<dyn CookieGate>>> = Mutex::new(None);

/// Asks `gate` before any VTOP session leaves the phone; call before [`crate::Core::new`].
#[uniffi::export]
pub fn set_cookie_gate(gate: Arc<dyn CookieGate>) {
    info!("cookie gate: foreign");
    *GATE.lock().unwrap_or_else(PoisonError::into_inner) = Some(gate);
}

/// The gate [`set_cookie_gate`] set, for a new core.
pub(crate) fn handed() -> Option<Arc<dyn CookieGate>> {
    GATE.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

/// Loads the remembered kinds; called when the core opens.
pub(crate) fn load(core: &Core) {
    let remembered = core.read::<Saved>(FILE).unwrap_or_default().remembered;
    let mut state = core.shared.consent();
    (state.path, state.remembered) = (Some(core.at(FILE)), remembered);
}

/// Forgets the remembered kinds and this session's answers (the file goes with `Core::forget`).
pub(crate) fn forget(shared: &Shared) {
    let mut state = shared.consent();
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
    action: &str,
    server: &str,
    ask: bool,
) -> bool {
    let decision = decide(shared, flow, kind, action, server, ask).await;
    info!(kind = kind.as_str(), decision, "cookie consent");
    decision.starts_with("sent")
}

async fn decide(
    shared: &Shared,
    flow: &str,
    kind: CookieKind,
    action: &str,
    server: &str,
    ask: bool,
) -> &'static str {
    let gate = {
        let mut state = shared.consent();
        let now = unix_now();
        state.answered.retain(|_, (at, _)| now - *at < FLOW_GRANT_S);
        match &state.gate {
            Some(gate) if !flow.is_empty() => gate.clone(),
            _ => return "denied_no_gate",
        }
    };
    let attended = gate.attended(flow.to_string());
    {
        let state = shared.consent();
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
    let mut state = shared.consent();
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
            let mut state = self.shared.consent();
            state.remembered.insert(kind);
            save(&state);
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

//! Consent before the VTOP session (cookies, and the password where a path sends it) goes to
//! our servers, plus the registration-number and connected-app approve prompts.
//!
//! Every send asks [`allowed`] first, which fails closed: no events listener, no flow, a flow
//! nobody watches (the app not shown, a background run) all count as no, except that a background
//! run follows the chosen route once the student ticked "don't ask again" for [`CookieKind::Route`]
//! (they chose that route and said not to ask). A "don't ask again" tick is kept per [`CookieKind`]
//! in [`FILE`]; an answer also covers the rest of its flow for [`FLOW_GRANT_S`] (a proxied
//! sign-in's hops ask once).
//!
//! Rust owns the pending prompts, coalescing of identical cookie asks, timeouts (`TunableKey::CookieAsk`
//! on `Runtime`), and cancellation when the app leaves the screen (`Core::app_shown(_, false)`).
//! Prompts are pushed through [`crate::AppEvents::on_consent`] as [`ConsentAsk`], and answered
//! through [`Core::consent_answer`].

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::mem::DropGuard;
use std::path::PathBuf;
use std::sync::{MutexGuard, PoisonError};

use futures::channel::oneshot;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::remote::handover::{HandoverAsk, HandoverStatus};
use crate::remote::regno::RegnoAnswer;
use crate::settings::TunableKey;
use crate::{Core, CoreError, Shared, store, traced_sync, unix_now};

pub(crate) const FILE: &str = "consent.json";
/// How long an answer covers the rest of its flow (one screen's pages, `sync.all`).
const FLOW_GRANT_S: i64 = 120;
const DEFAULT_WAIT_SECS: i32 = 60;

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

/// The student's answer to the cookie dialog. A no is never remembered, so it carries no tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CookieAnswer {
    Deny,
    Allow {
        /// Don't ask again for this kind.
        remember: bool,
    },
}

/// The phase of a connected app's approve sheet.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum HandoverSheetPhase {
    Asking,
    Sending,
    Sent,
    Failed { reason: String },
    Expired,
}

/// The consent prompt to show now, pushed through [`crate::AppEvents::on_consent`].
#[derive(Debug, Default, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ConsentAsk {
    #[default]
    None,
    Cookie {
        id: u64,
        flow: String,
        kind: CookieKind,
        payload: CookiePayload,
        action: String,
        server: String,
    },
    Regno {
        id: u64,
        flow: String,
    },
    Handover {
        id: u64,
        flow: String,
        ask: HandoverAsk,
        phase: HandoverSheetPhase,
    },
}

/// The student's answer to a [`ConsentAsk`], sent to [`Core::consent_answer`].
/// Only the variants fitting the prompt's kind are accepted; any other is logged and treated as deny.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ConsentAnswer {
    Send,
    Cancel,
    DontAskAgain,
    Share,
    NotNow,
    Approve,
    Deny,
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
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "kept for stable decision log names")
    )]
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

/// [`FILE`] on disk: `{"remembered":["route","account","handover"]}`. The kinds' names are
/// [`CookieKind`]'s serde names; `stored_format_roundtrips` pins them.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Stored {
    remembered: BTreeSet<CookieKind>,
}

struct CookiePrompt {
    id: u64,
    flow: String,
    kind: CookieKind,
    payload: CookiePayload,
    action: String,
    server: String,
    waiters: Vec<(u64, oneshot::Sender<CookieAnswer>)>,
}

struct RegnoPrompt {
    id: u64,
    flow: String,
    tx: Option<oneshot::Sender<RegnoAnswer>>,
}

struct HandoverPrompt {
    id: u64,
    flow: String,
    ask: HandoverAsk,
    phase: HandoverSheetPhase,
    tx: Option<oneshot::Sender<bool>>,
}

/// The core's consent state ([`Shared::consent`]).
#[derive(Default)]
pub(crate) struct State {
    /// `consent.json`, once the core opened.
    path: Option<PathBuf>,
    pub(crate) remembered: BTreeSet<CookieKind>,
    /// The answer in a flow, and when (unix seconds).
    answered: BTreeMap<(String, CookieKind), (i64, bool)>,
    /// Waiters for an in-flight `ask_cookie` of `(flow, kind)`, so concurrent calls share one prompt.
    in_flight: BTreeMap<(String, CookieKind), Vec<oneshot::Sender<ConsentDecision>>>,
    /// Rust-marked unattended flows (background sync, sweep, tests).
    pub(crate) unattended: BTreeSet<String>,
    next_id: u64,
    next_waiter: u64,
    cookie_prompts: Vec<CookiePrompt>,
    regno_prompt: Option<RegnoPrompt>,
    handover_prompt: Option<HandoverPrompt>,
    last_pushed: ConsentAsk,
    #[cfg(test)]
    pub(crate) wait_ms_override: Option<u64>,
}

impl State {
    fn alloc_id(&mut self) -> u64 {
        self.next_id = self.next_id.saturating_add(1);
        self.next_id
    }

    fn alloc_waiter(&mut self) -> u64 {
        self.next_waiter = self.next_waiter.saturating_add(1);
        self.next_waiter
    }

    fn current_ask(&self) -> ConsentAsk {
        if let Some(handover) = &self.handover_prompt
            && handover.phase == HandoverSheetPhase::Asking
        {
            return ConsentAsk::Handover {
                id: handover.id,
                flow: handover.flow.clone(),
                ask: handover.ask.clone(),
                phase: handover.phase.clone(),
            };
        }
        if let Some(cookie) = self.cookie_prompts.first() {
            return ConsentAsk::Cookie {
                id: cookie.id,
                flow: cookie.flow.clone(),
                kind: cookie.kind,
                payload: cookie.payload,
                action: cookie.action.clone(),
                server: cookie.server.clone(),
            };
        }
        if let Some(regno) = &self.regno_prompt {
            return ConsentAsk::Regno {
                id: regno.id,
                flow: regno.flow.clone(),
            };
        }
        if let Some(handover) = &self.handover_prompt {
            return ConsentAsk::Handover {
                id: handover.id,
                flow: handover.flow.clone(),
                ask: handover.ask.clone(),
                phase: handover.phase.clone(),
            };
        }
        ConsentAsk::None
    }

    /// Returns the new [`ConsentAsk`] if it differs from the last pushed one.
    fn take_push(&mut self) -> Option<ConsentAsk> {
        let now = self.current_ask();
        if now == self.last_pushed {
            None
        } else {
            self.last_pushed = now.clone();
            Some(now)
        }
    }
}

impl Shared {
    pub(crate) fn consent(&self) -> MutexGuard<'_, State> {
        self.consent.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn push_consent(shared: &Shared, ask: Option<ConsentAsk>) {
    if let Some(ask) = ask {
        shared.events.tell("consent", |e| e.on_consent(ask));
    }
}

/// Pushes the active prompt (if any) when a new [`crate::AppEvents`] listener attaches.
pub(crate) fn push_current(shared: &Shared) {
    let ask = {
        let mut state = shared.consent();
        let current = state.current_ask();
        state.last_pushed = current.clone();
        (current != ConsentAsk::None).then_some(current)
    };
    push_consent(shared, ask);
}

fn wait_ms(shared: &Shared) -> u64 {
    #[cfg(test)]
    if let Some(ms) = shared.consent().wait_ms_override {
        return ms;
    }
    let secs = shared.settings.as_ref().map_or(DEFAULT_WAIT_SECS, |store| {
        store.get().tunable(TunableKey::CookieAsk)
    });
    u64::try_from(secs.max(1)).unwrap_or(60) * 1000
}

fn log_ui(src: &str, message: &str, flow: &str, fields: &[(&str, String)]) {
    let map: HashMap<String, String> = fields
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect();
    crate::obs::log_event(
        "info".into(),
        src.into(),
        Some(flow.into()),
        message.into(),
        map,
    );
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

/// A cookie waiter whose future was dropped leaves; the prompt closes with its last waiter.
fn drop_cookie_waiter(shared: &Shared, prompt_id: u64, waiter_id: u64) {
    let push = {
        let mut state = shared.consent();
        if let Some(pos) = state.cookie_prompts.iter().position(|p| p.id == prompt_id)
            && let Some(prompt) = state.cookie_prompts.get_mut(pos)
        {
            prompt.waiters.retain(|(wid, _)| *wid != waiter_id);
            if prompt.waiters.is_empty() {
                state.cookie_prompts.remove(pos);
            }
        }
        state.take_push()
    };
    push_consent(shared, push);
}

fn drop_regno_prompt(shared: &Shared, prompt_id: u64) {
    let push = {
        let mut state = shared.consent();
        if state
            .regno_prompt
            .as_ref()
            .is_some_and(|p| p.id == prompt_id)
        {
            state.regno_prompt = None;
        }
        state.take_push()
    };
    push_consent(shared, push);
}

fn drop_handover_prompt(shared: &Shared, prompt_id: u64) {
    let push = {
        let mut state = shared.consent();
        if state
            .handover_prompt
            .as_ref()
            .is_some_and(|p| p.id == prompt_id && p.phase == HandoverSheetPhase::Asking)
        {
            state.handover_prompt = None;
        }
        state.take_push()
    };
    push_consent(shared, push);
}

/// Whether `flow` has the student on screen (no events listener, no flow, backgrounded or
/// unattended: no).
pub(crate) fn attended(shared: &Shared, flow: &str) -> bool {
    if flow.is_empty() || !shared.events.has_listener() || !shared.events.on_screen() {
        return false;
    }
    if shared.consent().unattended.contains(flow) {
        return false;
    }
    !shared.events.is_flow_unattended(flow)
}

/// Cancels every open consent question when the app leaves the screen. A sending/finished
/// handover sheet stays for its result.
pub(crate) fn on_app_left(shared: &Shared) {
    let (cookie_waiters, regno_tx, handover_tx, push) = {
        let mut state = shared.consent();
        let mut cookie_waiters = Vec::new();
        for prompt in state.cookie_prompts.drain(..) {
            log_ui(
                "ui.consent",
                "answered",
                &prompt.flow,
                &[
                    ("kind", prompt.kind.as_str().into()),
                    ("send", "false".into()),
                    ("remember", "false".into()),
                ],
            );
            cookie_waiters.extend(prompt.waiters.into_iter().map(|(_, tx)| tx));
        }
        let regno_tx = state.regno_prompt.take().and_then(|mut prompt| {
            log_ui(
                "ui.regno",
                "answered",
                &prompt.flow,
                &[("answer", "dismissed".into())],
            );
            prompt.tx.take()
        });
        let handover_tx = if state
            .handover_prompt
            .as_ref()
            .is_some_and(|p| p.phase == HandoverSheetPhase::Asking)
        {
            state
                .handover_prompt
                .take()
                .and_then(|mut prompt| prompt.tx.take())
        } else {
            None
        };
        let push = state.take_push();
        (cookie_waiters, regno_tx, handover_tx, push)
    };
    for tx in cookie_waiters {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "waiter dropped its future before app left"
        )]
        let _ = tx.send(CookieAnswer::Deny);
    }
    if let Some(tx) = regno_tx {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "waiter dropped its future before app left"
        )]
        let _ = tx.send(RegnoAnswer::Dismissed);
    }
    if let Some(tx) = handover_tx {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "waiter dropped its future before app left"
        )]
        let _ = tx.send(false);
    }
    push_consent(shared, push);
}

/// Resolves a prompt by `id` from [`Core::consent_answer`].
pub(crate) fn answer_shared(shared: &Shared, id: u64, answer: ConsentAnswer) {
    let (cookie_done, regno_done, handover_done, push) = {
        let mut state = shared.consent();
        let mut cookie_done = None;
        let mut regno_done = None;
        let mut handover_done = None;

        if let Some(pos) = state.cookie_prompts.iter().position(|p| p.id == id) {
            let prompt = state.cookie_prompts.remove(pos);
            let mapped = match answer {
                ConsentAnswer::Send => CookieAnswer::Allow { remember: false },
                ConsentAnswer::DontAskAgain => CookieAnswer::Allow { remember: true },
                ConsentAnswer::Cancel => CookieAnswer::Deny,
                other => {
                    warn!(id, ?other, "unexpected consent answer for cookie prompt");
                    CookieAnswer::Deny
                }
            };
            let (send, remember) = match mapped {
                CookieAnswer::Allow { remember } => (true, remember),
                CookieAnswer::Deny => (false, false),
            };
            log_ui(
                "ui.consent",
                "answered",
                &prompt.flow,
                &[
                    ("kind", prompt.kind.as_str().into()),
                    ("send", send.to_string()),
                    ("remember", remember.to_string()),
                ],
            );
            cookie_done = Some((prompt.waiters, mapped));
        } else if state.regno_prompt.as_ref().is_some_and(|p| p.id == id) {
            if let Some(mut prompt) = state.regno_prompt.take() {
                let mapped = match answer {
                    ConsentAnswer::Share => RegnoAnswer::Share,
                    ConsentAnswer::NotNow => RegnoAnswer::NotNow,
                    ConsentAnswer::Cancel => RegnoAnswer::Dismissed,
                    other => {
                        warn!(id, ?other, "unexpected consent answer for regno prompt");
                        RegnoAnswer::Dismissed
                    }
                };
                let answer_name = match mapped {
                    RegnoAnswer::Share => "share",
                    RegnoAnswer::NotNow => "not_now",
                    RegnoAnswer::Dismissed => "dismissed",
                };
                log_ui(
                    "ui.regno",
                    "answered",
                    &prompt.flow,
                    &[("answer", answer_name.into())],
                );
                if let Some(tx) = prompt.tx.take() {
                    regno_done = Some((tx, mapped));
                }
            }
        } else if state.handover_prompt.as_ref().is_some_and(|p| p.id == id) {
            let asking = state
                .handover_prompt
                .as_ref()
                .is_some_and(|p| p.phase == HandoverSheetPhase::Asking);
            if asking {
                let approve = match answer {
                    ConsentAnswer::Approve => true,
                    ConsentAnswer::Deny | ConsentAnswer::Cancel => false,
                    other => {
                        warn!(id, ?other, "unexpected consent answer for handover prompt");
                        false
                    }
                };
                if approve {
                    if let Some(prompt) = state.handover_prompt.as_mut() {
                        prompt.phase = HandoverSheetPhase::Sending;
                        log_ui(
                            "ui.handover",
                            "answered",
                            &prompt.flow,
                            &[("approve", "true".into())],
                        );
                        if let Some(tx) = prompt.tx.take() {
                            handover_done = Some((tx, true));
                        }
                    }
                } else if let Some(mut prompt) = state.handover_prompt.take() {
                    log_ui(
                        "ui.handover",
                        "answered",
                        &prompt.flow,
                        &[("approve", "false".into())],
                    );
                    if let Some(tx) = prompt.tx.take() {
                        handover_done = Some((tx, false));
                    }
                }
            } else if matches!(answer, ConsentAnswer::Cancel | ConsentAnswer::Deny) {
                state.handover_prompt = None;
            }
        }

        let push = state.take_push();
        (cookie_done, regno_done, handover_done, push)
    };

    if let Some((waiters, mapped)) = cookie_done {
        for (_, tx) in waiters {
            #[expect(
                clippy::let_underscore_must_use,
                reason = "waiter dropped its future before we replied"
            )]
            let _ = tx.send(mapped);
        }
    }
    if let Some((tx, mapped)) = regno_done {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "waiter dropped its future before we replied"
        )]
        let _ = tx.send(mapped);
    }
    if let Some((tx, approve)) = handover_done {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "waiter dropped its future before we replied"
        )]
        let _ = tx.send(approve);
    }
    push_consent(shared, push);
}

async fn ask_cookie(
    shared: &Shared,
    flow: &str,
    kind: CookieKind,
    payload: CookiePayload,
    action: &str,
    server: &str,
) -> CookieAnswer {
    if !shared.events.on_screen() {
        return CookieAnswer::Deny;
    }
    let (prompt_id, waiter_id, rx, push) = {
        let mut state = shared.consent();
        let waiter_id = state.alloc_waiter();
        let (tx, rx) = oneshot::channel();
        let prompt_id = if let Some(existing) = state.cookie_prompts.iter_mut().find(|p| {
            p.kind == kind && p.payload == payload && p.action == action && p.server == server
        }) {
            existing.waiters.push((waiter_id, tx));
            existing.id
        } else {
            let id = state.alloc_id();
            state.cookie_prompts.push(CookiePrompt {
                id,
                flow: flow.to_string(),
                kind,
                payload,
                action: action.to_string(),
                server: server.to_string(),
                waiters: vec![(waiter_id, tx)],
            });
            log_ui(
                "ui.consent",
                "shown",
                flow,
                &[("kind", kind.as_str().into())],
            );
            id
        };
        let push = state.take_push();
        (prompt_id, waiter_id, rx, push)
    };
    // A dropped future (the caller cancelled) takes its waiter out of the prompt.
    let guard = DropGuard::new((), |()| drop_cookie_waiter(shared, prompt_id, waiter_id));
    push_consent(shared, push);
    let result = shared.runtime.timeout(wait_ms(shared), rx).await;
    DropGuard::dismiss(guard);
    match result {
        Ok(Ok(answer)) => answer,
        Ok(Err(_)) => CookieAnswer::Deny,
        Err(_) => {
            answer_shared(shared, prompt_id, ConsentAnswer::Cancel);
            CookieAnswer::Deny
        }
    }
}

/// Shows the "Share your registration number?" sheet and awaits the student's answer.
pub(crate) async fn ask_regno(shared: &Shared, flow: &str) -> RegnoAnswer {
    if !shared.events.on_screen() {
        return RegnoAnswer::Dismissed;
    }
    let (prompt_id, rx, older_tx, push) = {
        let mut state = shared.consent();
        let id = state.alloc_id();
        let (tx, rx) = oneshot::channel();
        let older_tx = state.regno_prompt.replace(RegnoPrompt {
            id,
            flow: flow.to_string(),
            tx: Some(tx),
        });
        let older_tx = older_tx.and_then(|mut old| {
            log_ui(
                "ui.regno",
                "answered",
                &old.flow,
                &[("answer", "dismissed".into())],
            );
            old.tx.take()
        });
        log_ui("ui.regno", "shown", flow, &[]);
        let push = state.take_push();
        (id, rx, older_tx, push)
    };
    if let Some(tx) = older_tx {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "older waiter dropped its future before being replaced"
        )]
        let _ = tx.send(RegnoAnswer::Dismissed);
    }
    let guard = DropGuard::new((), |()| drop_regno_prompt(shared, prompt_id));
    push_consent(shared, push);
    let result = shared.runtime.timeout(wait_ms(shared), rx).await;
    DropGuard::dismiss(guard);
    match result {
        Ok(Ok(answer)) => answer,
        Ok(Err(_)) => RegnoAnswer::Dismissed,
        Err(_) => {
            answer_shared(shared, prompt_id, ConsentAnswer::Cancel);
            RegnoAnswer::Dismissed
        }
    }
}

/// Shows the handover approve sheet and awaits Approve or Deny (or timeout -> Expired).
async fn ask_handover(shared: &Shared, flow: &str, ask: HandoverAsk) -> bool {
    if !shared.events.on_screen() {
        return false;
    }
    let wait = wait_ms(shared);
    let now_ms = shared.runtime.clock().now_ms();
    let limit = ask.expires_at.map_or(wait, |exp| {
        wait.min(u64::try_from((exp - now_ms).max(0)).unwrap_or(0))
    });
    let (prompt_id, rx, older_tx, push) = {
        let mut state = shared.consent();
        let id = state.alloc_id();
        let (tx, rx) = oneshot::channel();
        let via = ask.asker.as_ref().map_or("-", |a| match a.via {
            crate::remote::keys::KeyKind::Browser => "browser",
            crate::remote::keys::KeyKind::Mcp => "mcp",
        });
        let ip_seen = ask
            .asker
            .as_ref()
            .and_then(|a| a.ip.as_ref())
            .map_or_else(|| "null".into(), |ip| ip.seen.to_string());
        let older_tx = state
            .handover_prompt
            .replace(HandoverPrompt {
                id,
                flow: flow.to_string(),
                ask,
                phase: HandoverSheetPhase::Asking,
                tx: Some(tx),
            })
            .and_then(|mut old| old.tx.take());
        log_ui(
            "ui.handover",
            "shown",
            flow,
            &[("via", via.into()), ("ip_seen", ip_seen)],
        );
        let push = state.take_push();
        (id, rx, older_tx, push)
    };
    if let Some(tx) = older_tx {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "older handover waiter dropped its future before being replaced"
        )]
        let _ = tx.send(false);
    }
    let guard = DropGuard::new((), |()| drop_handover_prompt(shared, prompt_id));
    push_consent(shared, push);
    let result = shared.runtime.timeout(limit, rx).await;
    DropGuard::dismiss(guard);
    match result {
        Ok(Ok(approved)) => approved,
        Ok(Err(_)) => false,
        Err(_) => {
            let push = {
                let mut state = shared.consent();
                if let Some(prompt) = state.handover_prompt.as_mut()
                    && prompt.id == prompt_id
                    && prompt.phase == HandoverSheetPhase::Asking
                {
                    prompt.tx = None;
                    prompt.phase = HandoverSheetPhase::Expired;
                }
                state.take_push()
            };
            push_consent(shared, push);
            false
        }
    }
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
        {
            let mut state = shared.consent();
            let now = unix_now();
            state.answered.retain(|_, (at, _)| now - *at < FLOW_GRANT_S);
            if flow.is_empty() || !shared.events.has_listener() {
                return Denied(DeniedWhy::NoGate);
            }
        }
        let is_attended = attended(shared, flow);
        let wait_rx = {
            let mut state = shared.consent();
            if !is_attended {
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
        // A leader cancelled before it answers frees the slot, so a waiter asks again.
        let guard = DropGuard::new(key.clone(), |key| {
            shared.consent().in_flight.remove(&key);
        });
        let answer = ask_cookie(shared, flow, kind, payload, action, server).await;
        let send = matches!(answer, CookieAnswer::Allow { .. });
        let (decision, waiters) = {
            let mut state = shared.consent();
            state.answered.insert(key.clone(), (unix_now(), send));
            if let CookieAnswer::Allow { remember: true } = answer {
                state.remembered.insert(kind);
                save(&state);
            }
            let waiters = state.in_flight.remove(&key).unwrap_or_default();
            DropGuard::dismiss(guard);
            let decision = match answer {
                CookieAnswer::Allow { .. } => Sent(SentWhy::Asked),
                CookieAnswer::Deny => Denied(DeniedWhy::Asked),
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

/// Whether the session may go to a connected app for `ask`: only with the student on screen,
/// then a remembered yes for [`CookieKind::Handover`] or Approve on the sheet. Logs the decision
/// like [`allowed`], never the session.
pub(crate) async fn allowed_handover(shared: &Shared, flow: &str, ask: HandoverAsk) -> bool {
    use ConsentDecision::{Denied, Sent};
    let kind = CookieKind::Handover;
    let decision = if flow.is_empty() || !shared.events.has_listener() {
        Denied(DeniedWhy::NoGate)
    } else if !attended(shared, flow) {
        Denied(DeniedWhy::Unattended)
    } else if shared.consent().remembered.contains(&kind) {
        Sent(SentWhy::Remembered)
    } else if ask_handover(shared, flow, ask).await {
        Sent(SentWhy::Asked)
    } else {
        Denied(DeniedWhy::Asked)
    };
    info!(
        kind = kind.as_str(),
        decision = decision.as_str(),
        "cookie consent"
    );
    decision.is_sent()
}

/// Tells the app how request `request_id`'s answer is going (notification and sheet phase).
pub(crate) fn handover_status(
    shared: &Shared,
    flow: &str,
    request_id: &str,
    status: HandoverStatus,
) {
    let status_name = match &status {
        HandoverStatus::Fetching { .. } => "fetching",
        HandoverStatus::Sent => "sent",
        HandoverStatus::Failed { .. } => "failed",
    };
    log_ui(
        "ui.handover",
        "status",
        flow,
        &[("status", status_name.into())],
    );
    let push = {
        let mut state = shared.consent();
        if let Some(prompt) = state.handover_prompt.as_mut()
            && prompt.phase != HandoverSheetPhase::Asking
            && prompt.ask.request_id == request_id
        {
            prompt.phase = match &status {
                HandoverStatus::Fetching { .. } => HandoverSheetPhase::Sending,
                HandoverStatus::Sent => HandoverSheetPhase::Sent,
                HandoverStatus::Failed { reason } => HandoverSheetPhase::Failed {
                    reason: reason.clone(),
                },
            };
        }
        state.take_push()
    };
    push_consent(shared, push);
    shared.events.tell("handover_status", |e| {
        e.on_handover_status(flow.to_string(), request_id.to_string(), status);
    });
}

/// The host of `url`, for the dialog ("vmate.aster0.dev").
pub(crate) fn host(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.split(['/', '?']).next().unwrap_or(rest).to_string()
}

#[uniffi::export]
impl Core {
    /// Delivers the student's answer to consent prompt `id` (or closes a finished handover sheet).
    pub fn consent_answer(&self, flow: String, id: u64, answer: ConsentAnswer) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "traced_sync already logs the error"
        )]
        let _ = traced_sync("consent_answer", &flow, || {
            answer_shared(&self.shared, id, answer);
            Ok(())
        });
    }

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

    /// Asks every time again for `kind` only (the other kinds' ticks stay).
    pub fn forget_cookie_consent(&self, flow: String, kind: CookieKind) -> Result<(), CoreError> {
        traced_sync("forget_cookie_consent", &flow, || {
            {
                let mut state = self.shared.consent();
                state.remembered.remove(&kind);
                state.answered.retain(|(_, answered), _| *answered != kind);
                save(&state);
            }
            info!(?kind, "cookie consent forgotten");
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

//! Whether the student lets this install share their registration number with our server. A
//! server identity (the account proof, its JWT and refresh token, and everything that goes
//! with it: Connected apps, mail forwarding, push, the server routes) names the student, so
//! none is taken until they said Share: [`Core::regno_allowed`] is the one check, made where
//! the JWT is got (`backend.rs`, `take_identity`). Separate from cookie consent
//! (`consent.rs`), which is about the VTOP session.
//!
//! The answers are kept per student in [`FILE`], which a sign-out (or another student) empties
//! but keeps: no file at all is an install from before the question, whose identity counts as
//! Shared ([`migrated`]). Asked with [`CookieGate::ask_regno`]. Only a flow with the student
//! on screen that needs the identity asks; background runs and calls nobody started never do.
//! "Not now" holds for the rest of its flow, so one screen asks once.

use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::remote::consent::{self, CookieGate};
use crate::remote::route::Route;
use crate::remote::{account_token, forwarding};
use crate::{Core, CoreError, traced};

pub(crate) const FILE: &str = "regno_sharing.json";

/// Serialises read-modify-write of [`FILE`].
static LOCK: Mutex<()> = Mutex::new(());

/// The student's answer for the signed-in account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RegnoSharing {
    /// Never asked (a fresh sign-in): the first use asks.
    Unasked,
    Shared,
    /// "Not now", or turned off in Settings: a feature that needs it asks again.
    Declined,
}

/// What the sheet gave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RegnoAnswer {
    Share,
    NotNow,
    /// Closed without an answer (timed out, swiped away, the app left): nothing changes.
    Dismissed,
}

/// When [`Core::ask_regno_sharing`] shows the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RegnoAsk {
    /// After a sign-in or at an open: only while never asked.
    FirstTime,
    /// A feature that needs it (a server route picked): also after a "Not now".
    ForFeature,
}

/// [`FILE`] on disk: `{"answers":{"22BCE0001":"shared"}}`, keyed by the registration number in
/// upper case. Unasked is no entry, so it can't be written; `stored_format_roundtrips` pins the
/// spelling.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Stored {
    #[serde(default)]
    answers: BTreeMap<String, StoredAnswer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StoredAnswer {
    Shared,
    Declined,
}

impl From<StoredAnswer> for RegnoSharing {
    fn from(answer: StoredAnswer) -> Self {
        match answer {
            StoredAnswer::Shared => Self::Shared,
            StoredAnswer::Declined => Self::Declined,
        }
    }
}

/// What a transition asks the driver to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Effect {
    Save(StoredAnswer),
    /// Drop the JWT and the refresh token: no identity is used from now on.
    ForgetIdentity,
}

/// The file for an install from before the question: each student it already holds a server
/// identity for (`held`: a refresh token, a v2 token, a forwarding address) said Share then.
fn migrated(held: impl IntoIterator<Item = String>) -> Stored {
    Stored {
        answers: held
            .into_iter()
            .map(|regno| (regno.to_ascii_uppercase(), StoredAnswer::Shared))
            .collect(),
    }
}

/// `regno`'s answer. The demo account's number is no one's: always Shared.
fn resolve(stored: &Stored, regno: &str) -> RegnoSharing {
    if regno.eq_ignore_ascii_case(crate::demo::USERNAME) {
        return RegnoSharing::Shared;
    }
    stored
        .answers
        .get(&regno.to_ascii_uppercase())
        .map_or(RegnoSharing::Unasked, |answer| (*answer).into())
}

/// The state after `answer`, and what to do.
fn next(from: RegnoSharing, answer: RegnoAnswer) -> (RegnoSharing, Vec<Effect>) {
    match answer {
        RegnoAnswer::Share => (
            RegnoSharing::Shared,
            vec![Effect::Save(StoredAnswer::Shared)],
        ),
        // Whatever it was: an identity held without a Share (none should be) goes too.
        RegnoAnswer::NotNow => (
            RegnoSharing::Declined,
            vec![Effect::Save(StoredAnswer::Declined), Effect::ForgetIdentity],
        ),
        RegnoAnswer::Dismissed => (from, Vec::new()),
    }
}

/// Whether `sharing` shows the sheet for `ask`.
fn should_ask(sharing: RegnoSharing, ask: RegnoAsk) -> bool {
    match (sharing, ask) {
        (RegnoSharing::Shared, _) => false,
        (RegnoSharing::Unasked, _) => true,
        (RegnoSharing::Declined, ask) => ask == RegnoAsk::ForFeature,
    }
}

/// Empties the answers at a sign-out or another student, keeping the file (so the next
/// student isn't taken for an old install).
pub(crate) fn forget(core: &Core) {
    let _guard = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    if core.at(FILE).exists() {
        core.write_plain(FILE, &Stored::default());
    }
}

impl RegnoSharing {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unasked => "unasked",
            Self::Shared => "shared",
            Self::Declined => "declined",
        }
    }
}

impl RegnoAnswer {
    fn as_str(self) -> &'static str {
        match self {
            Self::Share => "share",
            Self::NotNow => "not_now",
            Self::Dismissed => "dismissed",
        }
    }
}

impl Core {
    /// [`FILE`], made from what this install holds the first time ([`migrated`]).
    fn stored(&self) -> Stored {
        let _guard = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        self.stored_locked()
    }

    /// [`Self::stored`] with [`LOCK`] held.
    fn stored_locked(&self) -> Stored {
        if let Some(stored) = self.read::<Stored>(FILE) {
            return stored;
        }
        let account = (self.at(forwarding::FILE).exists())
            .then(|| self.read_account())
            .flatten();
        let held = [
            self.device().refresh.map(|refresh| refresh.regno),
            self.read::<account_token::Saved>(account_token::FILE)
                .map(|saved| saved.registration_number),
            account.map(|saved| saved.registration_number.unwrap_or(saved.username)),
        ];
        let stored = migrated(held.into_iter().flatten());
        info!(
            kind = "regno",
            outcome = "migrated",
            shared = stored.answers.len(),
            "regno sharing"
        );
        self.write_plain(FILE, &stored);
        stored
    }

    fn sharing_of(&self, regno: &str) -> RegnoSharing {
        resolve(&self.stored(), regno)
    }

    fn run_regno_effects(&self, regno: &str, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Save(answer) => {
                    let _guard = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
                    let mut stored = self.stored_locked();
                    stored.answers.insert(regno.to_ascii_uppercase(), answer);
                    self.write_plain(FILE, &stored);
                }
                Effect::ForgetIdentity => self.forget_identity(),
            }
        }
    }

    fn apply_regno_answer(
        &self,
        flow: &str,
        regno: &str,
        from: RegnoSharing,
        answer: RegnoAnswer,
    ) -> RegnoSharing {
        let (to, effects) = next(from, answer);
        self.run_regno_effects(regno, effects);
        info!(
            flow,
            kind = "regno",
            answer = answer.as_str(),
            from = from.as_str(),
            to = to.as_str(),
            "regno sharing"
        );
        to
    }

    /// Whether a server identity may be used now, never asking: the student said Share, or no
    /// registration number is known to share ([`Self::regno_allowed`] without its log line).
    pub(crate) async fn identity_shared(&self) -> bool {
        let regno = self.signed_in_regno().await;
        regno.is_none_or(|regno| self.sharing_of(&regno) == RegnoSharing::Shared)
    }

    /// The signed-in student's answer; Unasked signed out.
    pub(crate) async fn regno_sharing_now(&self) -> RegnoSharing {
        match self.signed_in_regno().await {
            Some(regno) => self.sharing_of(&regno),
            None => RegnoSharing::Unasked,
        }
    }

    /// Shows the sheet when `ask` calls for it and the student is on screen, one sheet at a
    /// time; the answer after it (as it was when not asked).
    async fn ask_regno(&self, flow: &str, regno: &str, ask: RegnoAsk) -> RegnoSharing {
        let sharing = self.sharing_of(regno);
        if !should_ask(sharing, ask) {
            return sharing;
        }
        let gate = self.shared.consent().gate.clone();
        let Some(gate) = gate.filter(|_| consent::attended(&self.shared, flow)) else {
            info!(
                flow,
                kind = "regno",
                outcome = "not_asked_unattended",
                "regno sharing"
            );
            return sharing;
        };
        let mut said_no_in = self.shared.regno_ask.lock().await;
        // Another call may have asked while this one waited.
        let sharing = self.sharing_of(regno);
        if !should_ask(sharing, ask) {
            return sharing;
        }
        if said_no_in.as_deref() == Some(flow) {
            return sharing;
        }
        let answer = ask_gate(gate.as_ref(), flow).await;
        let to = self.apply_regno_answer(flow, regno, sharing, answer);
        if to != RegnoSharing::Shared {
            *said_no_in = Some(flow.to_string());
        }
        to
    }

    /// The server routes fetch as the student: once they stopped sharing, Direct is chosen.
    fn leave_server_routes(&self, flow: &str) {
        let chosen = self.route_status(flow.to_string()).chosen;
        if matches!(chosen, Route::Server | Route::Udp)
            && let Err(error) = self.set_route(flow.to_string(), Route::Direct)
        {
            warn!(%error, "route not set back to direct");
        }
    }

    /// The one check before a server identity is taken: Ok once the signed-in student said
    /// Share (signed out: Ok, no number goes; only a kept identity is used, to leave). With
    /// `may_ask`, a flow with the student on screen asks first. Else `RegnoNotShared`.
    pub(crate) async fn regno_allowed(
        &self,
        flow: &str,
        regno: Option<&str>,
        may_ask: bool,
    ) -> Result<(), CoreError> {
        let Some(regno) = regno else {
            return Ok(());
        };
        let sharing = if may_ask {
            self.ask_regno(flow, regno, RegnoAsk::ForFeature).await
        } else {
            self.sharing_of(regno)
        };
        if sharing == RegnoSharing::Shared {
            return Ok(());
        }
        info!(
            flow,
            kind = "regno",
            outcome = "not_shared",
            sharing = sharing.as_str(),
            "regno sharing"
        );
        Err(CoreError::RegnoNotShared)
    }
}

/// The sheet's answer; a failed sheet is no answer.
async fn ask_gate(gate: &dyn CookieGate, flow: &str) -> RegnoAnswer {
    gate.ask_regno(flow.to_string())
        .await
        .unwrap_or(RegnoAnswer::Dismissed)
}

#[uniffi::export]
impl Core {
    /// The signed-in student's answer (Settings → Connections); Unasked signed out.
    pub async fn regno_sharing(&self, flow: String) -> RegnoSharing {
        let sharing = traced("regno_sharing", &flow, async {
            Ok(self.regno_sharing_now().await)
        })
        .await;
        sharing.unwrap_or(RegnoSharing::Unasked)
    }

    /// Asks the signed-in student when `ask` calls for it (with them on screen), and gives the
    /// answer after. A sign-in asks [`RegnoAsk::FirstTime`] before the background proof; a
    /// server route picked asks [`RegnoAsk::ForFeature`] and is chosen only on Shared.
    pub async fn ask_regno_sharing(
        &self,
        flow: String,
        ask: RegnoAsk,
    ) -> Result<RegnoSharing, CoreError> {
        traced("ask_regno_sharing", &flow, async {
            let regno = self.signed_in_regno().await.ok_or(CoreError::NotSignedIn)?;
            Ok(self.ask_regno(&flow, &regno, ask).await)
        })
        .await
    }

    /// Settings' switch: Share, or NotNow to stop. Stopping drops this install's JWT and
    /// refresh token, so nothing goes as the student from then on; the server keeps what it
    /// has until they delete it on the Server page. A server route chosen goes back to Direct.
    pub async fn set_regno_sharing(
        &self,
        flow: String,
        answer: RegnoAnswer,
    ) -> Result<RegnoSharing, CoreError> {
        traced("set_regno_sharing", &flow, async {
            let regno = self.signed_in_regno().await.ok_or(CoreError::NotSignedIn)?;
            let from = self.sharing_of(&regno);
            let to = self.apply_regno_answer(&flow, &regno, from, answer);
            if to != RegnoSharing::Shared {
                self.leave_server_routes(&flow);
            }
            Ok(to)
        })
        .await
    }
}

#[cfg(test)]
pub(crate) mod tests;

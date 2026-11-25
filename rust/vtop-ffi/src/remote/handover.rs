//! A connected app's request for this phone's VTOP session (`vtop_cookie_request`), from FCM,
//! `sessions:pending` or its notification's tap. Rust keeps each request's lifecycle
//! ([`board`]) and answers it once, within a deadline; Android only does what
//! [`HandoverEffect`]s say: post or cancel the notification and start the answer. The answer
//! asks the student through `consent::allowed` ([`Core::session_cookies`]). No request ids,
//! cookies or notification contents are logged.

mod board;

use std::collections::HashMap;
use std::sync::{MutexGuard, PoisonError};
use std::time::{Duration, SystemTime};

use futures::future::{Either, select};
use tracing::info;
use vtop_core::sync::sleep;

use crate::remote::consent;
use crate::remote::keys::KeyKind;
use crate::{Core, CoreError, Shared, traced, traced_sync};
pub(crate) use board::Board;
use board::{Start, Wake};

/// Sent in place of the session when the answer took too long.
const TIMEOUT: &str = "The phone took too long to sign in to VTOP.";
/// Below this an FCM time is in seconds, not ms.
const SECONDS_BELOW: i64 = 10_000_000_000;

/// The key that asked for a session, and from where: the approve sheet and the notification
/// show it. The server's `asker` (`sessions:pending`) or the push's `keyLabel`, `keyVia`, `ip`
/// and `ipSeen`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct HandoverAsker {
    /// The key's name ("Chrome on MacBook", "Claude").
    pub key_label: String,
    pub via: KeyKind,
    /// The address it asked from (an AI app's as our engine saw it on `/mcp`); None when the
    /// server didn't see one.
    pub ip: Option<AskerIp>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AskerIp {
    pub address: String,
    /// The key used this address before (one of its last two).
    pub seen: bool,
}

/// What the approve sheet asks about: a connected app's request for this phone's session.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct HandoverAsk {
    pub request_id: String,
    /// None when the server no longer lists the request or didn't say.
    pub asker: Option<HandoverAsker>,
    /// Unix ms; None when the server didn't say.
    pub asked_at: Option<i64>,
    /// Unix ms; None when the server no longer lists the request.
    pub expires_at: Option<i64>,
    /// The host the session goes through ("vmate.aster0.dev").
    pub server: String,
}

/// How an approved answer is going, for its notification and the sheet.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum HandoverStatus {
    /// The student said yes (or ticked "don't ask again" in Connected apps): checking the VTOP
    /// session, signing in again if it expired, and sending it.
    Fetching { asker: Option<HandoverAsker> },
    /// The server took the session.
    Sent,
    /// It couldn't be sent; `reason` is for the student.
    Failed { reason: String },
}

/// What reached the phone about connected apps' session requests.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum HandoverEvent {
    /// FCM's `vtop_cookie_request`, its `data` as sent; `foreground` while the app is on screen.
    Pushed {
        data: HashMap<String, String>,
        foreground: bool,
    },
    /// A request `sessions:pending` lists (unix ms).
    Listed {
        request_id: String,
        expires_at: i64,
        foreground: bool,
        asker: Option<HandoverAsker>,
    },
    /// Its notification was tapped; the expiry the notification kept, if any (unix ms).
    Tapped {
        request_id: String,
        expires_at: Option<i64>,
    },
    /// The app came on screen.
    Shown,
}

/// What Android does next, in order.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum HandoverEffect {
    /// Post the request's notification (its tap brings [`HandoverEvent::Tapped`]); it times out at
    /// `expires_at` (unix ms) and names `asker` when known.
    Notify {
        request_id: String,
        notification_id: i32,
        expires_at: i64,
        asker: Option<HandoverAsker>,
    },
    /// Take a notification down.
    Cancel { notification_id: i32 },
    /// Answer with [`Core::answer_handover`], off the main thread.
    Answer { request_id: String, attempt: u64 },
}

#[uniffi::export]
impl Core {
    /// What to do about `event`: notify while the app is away, answer once on screen or on a
    /// tap, drop expired requests and duplicates. No I/O.
    pub fn handover(&self, flow: String, event: HandoverEvent) -> Vec<HandoverEffect> {
        traced_sync("handover", &flow, || {
            let now = unix_ms();
            let mut board = self.shared.handover();
            let (kind, asker, effects) = match event {
                HandoverEvent::Pushed { data, foreground } => (
                    "pushed",
                    push_asker(&data),
                    match data.get("requestId").filter(|id| !id.trim().is_empty()) {
                        Some(id) => board.arrived(id, push_expiry(&data), wake(foreground), now),
                        None => Vec::new(),
                    },
                ),
                HandoverEvent::Listed {
                    request_id,
                    expires_at,
                    foreground,
                    asker,
                } => (
                    "listed",
                    asker,
                    board.arrived(&request_id, Some(expires_at), wake(foreground), now),
                ),
                HandoverEvent::Tapped {
                    request_id,
                    expires_at,
                } => (
                    "tapped",
                    None,
                    board.arrived(&request_id, expires_at, Wake::Tapped, now),
                ),
                HandoverEvent::Shown => ("shown", None, board.shown(now)),
            };
            // One event is about one request, so its asker names the one notification it posts.
            let effects: Vec<HandoverEffect> = (effects.into_iter())
                .map(|effect| match effect {
                    HandoverEffect::Notify {
                        request_id,
                        notification_id,
                        expires_at,
                        asker: None,
                    } => HandoverEffect::Notify {
                        request_id,
                        notification_id,
                        expires_at,
                        asker: asker.clone(),
                    },
                    effect => effect,
                })
                .collect();
            let count =
                |want: fn(&HandoverEffect) -> bool| effects.iter().filter(|e| want(e)).count();
            info!(
                event = kind,
                notify = count(|e| matches!(e, HandoverEffect::Notify { .. })),
                answer = count(|e| matches!(e, HandoverEffect::Answer { .. })),
                cancel = count(|e| matches!(e, HandoverEffect::Cancel { .. })),
                "session request"
            );
            Ok(effects)
        })
        .unwrap_or_default()
    }

    /// Answers request `request_id` for `attempt` (a [`HandoverEffect::Answer`]): this phone's
    /// VTOP session with the student saying yes, or why not. Past `limit_secs` the answer is
    /// dropped and the timeout sent instead, so the server gets one answer. A request expired
    /// or answered meanwhile sends nothing. A failure is sent too, then returned.
    pub async fn answer_handover(
        &self,
        flow: String,
        request_id: String,
        attempt: u64,
        limit_secs: u32,
    ) -> Result<(), CoreError> {
        traced("answer_handover", &flow, async {
            match self
                .shared
                .handover()
                .start(&request_id, attempt, unix_ms())
            {
                Start::Run => {}
                skipped => {
                    info!(?skipped, "session request not answered");
                    return Ok(());
                }
            }
            let mut held = Held {
                shared: &self.shared,
                request_id: &request_id,
                attempt,
                finished: false,
            };
            // In a block, so a late answer is dropped before the timeout goes.
            let answered = {
                let answer = std::pin::pin!(self.answer_session_request(&flow, &request_id));
                let late = std::pin::pin!(sleep(Duration::from_secs(limit_secs.into())));
                match select(answer, late).await {
                    Either::Left((answered, _)) => Some(answered),
                    Either::Right(_) => None,
                }
            };
            let result = match answered {
                Some(answered) => answered,
                None => {
                    info!(limit_secs, "session request timed out");
                    let timeout = Err(CoreError::Other {
                        reason: TIMEOUT.into(),
                    });
                    // An approved answer's notification stops spinning.
                    consent::handover_status(
                        &self.shared,
                        &flow,
                        &request_id,
                        HandoverStatus::Failed {
                            reason: TIMEOUT.into(),
                        },
                    );
                    self.answer_with(&flow, &request_id, timeout)
                        .await
                        .map(|_| ())
                }
            };
            held.finished = true;
            self.shared
                .handover()
                .finish(&request_id, attempt, unix_ms());
            result
        })
        .await
    }
}

impl Shared {
    fn handover(&self) -> MutexGuard<'_, Board> {
        self.handover.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// An answer in progress: dropped before it finished (the app cancelled it), it lets go of the
/// request.
struct Held<'a> {
    shared: &'a Shared,
    request_id: &'a str,
    attempt: u64,
    finished: bool,
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        if !self.finished {
            info!("session request answer cancelled");
            (self.shared.handover()).abandon(self.request_id, self.attempt, unix_ms());
        }
    }
}

fn wake(foreground: bool) -> Wake {
    if foreground { Wake::Shown } else { Wake::Away }
}

/// The request's expiry (unix ms) from FCM's `data`: `expiresAt`, else `at` plus the server's
/// lifetime for a request; either in seconds or ms. None without either.
fn push_expiry(data: &HashMap<String, String>) -> Option<i64> {
    let time = |key: &str| {
        let raw = data.get(key)?.parse::<i64>().ok().filter(|raw| *raw > 0)?;
        Some(if raw < SECONDS_BELOW { raw * 1000 } else { raw })
    };
    time("expiresAt").or_else(|| Some(time("at")? + board::TTL_MS))
}

/// Who asked, from FCM's `data` (strings only): `keyLabel` and `keyVia`, then `ip` and `ipSeen`.
/// None without a label.
fn push_asker(data: &HashMap<String, String>) -> Option<HandoverAsker> {
    let key_label = data
        .get("keyLabel")
        .map(|label| label.trim())
        .filter(|label| !label.is_empty())?;
    let via = match data.get("keyVia").map(String::as_str) {
        Some("mcp") => KeyKind::Mcp,
        _ => KeyKind::Browser,
    };
    let ip = (data.get("ip").map(|ip| ip.trim()))
        .filter(|ip| !ip.is_empty())
        .map(|address| AskerIp {
            address: address.into(),
            seen: data.get("ipSeen").is_some_and(|seen| seen == "1"),
        });
    Some(HandoverAsker {
        key_label: key_label.into(),
        via,
        ip,
    })
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

#[cfg(test)]
mod tests;

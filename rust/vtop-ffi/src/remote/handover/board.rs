//! Each connected-app session request's lifecycle on this phone, with no I/O and the time passed
//! in: notified while the app is away, answered once (one attempt at a time), then done. Done
//! requests stay a while past expiry, so a late duplicate (FCM, `sessions:pending`, a tap) is
//! ignored instead of asked again.

use std::collections::BTreeMap;

use super::HandoverEffect;

/// The server's lifetime for a session request (`REQUEST_TIMEOUT_SECS`), for one that came
/// without an expiry.
pub(crate) const TTL_MS: i64 = 120_000;
/// How long a finished request is kept past its expiry to absorb late duplicates.
pub(crate) const GRACE_MS: i64 = 30_000;

/// A notification id range of its own, so session requests never collide with the account's or
/// live updates' notifications.
const NOTIFICATION_BASE: i32 = 0x4000_0000;
const NOTIFICATION_MASK: i32 = 0x0fff_ffff;

/// One request's phase; every time is unix ms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Request {
    /// Its notification is up, waiting for a tap or the app coming on screen.
    Notified { expires_at: i64 },
    /// Attempt `attempt` checks the session, asks the student and sends the answer.
    Answering { expires_at: i64, attempt: u64 },
    /// Answered, timed out or expired.
    Done { expires_at: i64 },
}

impl Request {
    fn expires_at(self) -> i64 {
        match self {
            Self::Notified { expires_at }
            | Self::Answering { expires_at, .. }
            | Self::Done { expires_at } => expires_at,
        }
    }
}

/// How a request reached the phone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wake {
    /// With the app away (FCM).
    Away,
    /// With the app on screen (FCM or `sessions:pending`).
    Shown,
    /// Its notification was tapped.
    Tapped,
}

/// What happens to the request [`Board::start`] was asked about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Start {
    Run,
    /// Another attempt has it, or it is done.
    Stale,
    /// It expired before the attempt began: done, with nothing sent.
    Expired,
}

#[derive(Debug, Default)]
pub(crate) struct Board {
    requests: BTreeMap<String, Request>,
    last_attempt: u64,
}

impl Board {
    /// A request arrived as `wake` says, expiring at `expires_at` (unix ms; when absent, as the
    /// board knew it, else `TTL_MS` from `now`).
    pub(crate) fn arrived(
        &mut self,
        id: &str,
        expires_at: Option<i64>,
        wake: Wake,
        now: i64,
    ) -> Vec<HandoverEffect> {
        // Read before pruning, so a late tap with no expiry on a pruned request still knows it.
        let before = self.requests.get(id).copied();
        let mut effects = self.prune(now);
        let expiry = expires_at
            .or(before.map(Request::expires_at))
            .unwrap_or(now + TTL_MS);
        let cancel = HandoverEffect::Cancel {
            notification_id: notification_id(id),
        };
        let answer = wake != Wake::Away;
        match self.requests.get(id).copied() {
            Some(Request::Done { .. } | Request::Answering { .. }) => {
                if wake == Wake::Tapped || matches!(before, Some(Request::Notified { .. })) {
                    push(&mut effects, cancel);
                }
            }
            Some(Request::Notified { .. }) | None if expiry <= now => {
                if self.requests.contains_key(id) {
                    self.expire(id, expiry, now);
                }
                push(&mut effects, cancel);
            }
            Some(Request::Notified { .. }) | None if answer => {
                push(&mut effects, cancel);
                effects.push(self.answer(id, expiry));
            }
            Some(Request::Notified {
                expires_at: notified,
            }) => {
                if expiry < notified {
                    self.requests
                        .insert(id.into(), Request::Notified { expires_at: expiry });
                }
            }
            None => {
                self.requests
                    .insert(id.into(), Request::Notified { expires_at: expiry });
                effects.push(HandoverEffect::Notify {
                    request_id: id.into(),
                    notification_id: notification_id(id),
                    expires_at: expiry,
                    asker: None,
                });
            }
        }
        effects
    }

    /// The app came on screen: every notified request is answered, and expired ones' notifications
    /// go.
    pub(crate) fn shown(&mut self, now: i64) -> Vec<HandoverEffect> {
        let mut effects = self.prune(now);
        let notified: Vec<(String, i64)> = (self.requests.iter())
            .filter_map(|(id, request)| match request {
                Request::Notified { expires_at } => Some((id.clone(), *expires_at)),
                _ => None,
            })
            .collect();
        for (id, expires_at) in notified {
            push(
                &mut effects,
                HandoverEffect::Cancel {
                    notification_id: notification_id(&id),
                },
            );
            effects.push(self.answer(&id, expires_at));
        }
        effects
    }

    /// Attempt `attempt` of `id` is about to send: only while it holds the request, and before
    /// it expired.
    pub(crate) fn start(&mut self, id: &str, attempt: u64, now: i64) -> Start {
        match self.requests.get(id) {
            Some(&Request::Answering {
                expires_at,
                attempt: holder,
            }) if holder == attempt => {
                if now < expires_at {
                    Start::Run
                } else {
                    self.finish(id, attempt, now);
                    Start::Expired
                }
            }
            _ => {
                self.prune(now);
                Start::Stale
            }
        }
    }

    /// Attempt `attempt` of `id` sent its answer (the session, why not, or that it timed out):
    /// done. False when it no longer held the request.
    pub(crate) fn finish(&mut self, id: &str, attempt: u64, now: i64) -> bool {
        let held = self.held(id, attempt);
        if let Some(expires_at) = held {
            self.expire(id, expires_at, now);
        }
        self.prune(now);
        held.is_some()
    }

    /// Attempt `attempt` of `id` was dropped before it answered (the app cancelled it): a request
    /// still live can be answered again; an expired one is done.
    pub(crate) fn abandon(&mut self, id: &str, attempt: u64, now: i64) {
        if let Some(expires_at) = self.held(id, attempt) {
            if now >= expires_at {
                self.expire(id, expires_at, now);
            } else {
                self.requests.remove(id);
            }
        }
        self.prune(now);
    }

    #[cfg(test)]
    pub(crate) fn get(&self, id: &str) -> Option<Request> {
        self.requests.get(id).copied()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.requests.len()
    }

    fn held(&self, id: &str, attempt: u64) -> Option<i64> {
        match self.requests.get(id) {
            Some(&Request::Answering {
                expires_at,
                attempt: holder,
            }) if holder == attempt => Some(expires_at),
            _ => None,
        }
    }

    fn answer(&mut self, id: &str, expires_at: i64) -> HandoverEffect {
        self.last_attempt += 1;
        let attempt = self.last_attempt;
        self.requests.insert(
            id.into(),
            Request::Answering {
                expires_at,
                attempt,
            },
        );
        HandoverEffect::Answer {
            request_id: id.into(),
            attempt,
        }
    }

    /// Done, kept until its grace ends; past it, forgotten.
    fn expire(&mut self, id: &str, expires_at: i64, now: i64) {
        if now < expires_at + GRACE_MS {
            self.requests
                .insert(id.into(), Request::Done { expires_at });
        } else {
            self.requests.remove(id);
        }
    }

    /// Forgets done requests past their grace and ends notified ones past expiry, cancelling
    /// their notifications. Answering ones end only with their attempt.
    fn prune(&mut self, now: i64) -> Vec<HandoverEffect> {
        let mut effects = Vec::new();
        let mut expired = Vec::new();
        self.requests.retain(|id, request| match *request {
            Request::Done { expires_at } => now < expires_at + GRACE_MS,
            Request::Notified { expires_at } if now >= expires_at => {
                expired.push(id.clone());
                effects.push(HandoverEffect::Cancel {
                    notification_id: notification_id(id),
                });
                now < expires_at + GRACE_MS
            }
            Request::Notified { .. } | Request::Answering { .. } => true,
        });
        for id in expired {
            if let Some(request) = self.requests.get_mut(&id) {
                *request = Request::Done {
                    expires_at: request.expires_at(),
                };
            }
        }
        effects
    }
}

/// `effect`, unless already there.
fn push(effects: &mut Vec<HandoverEffect>, effect: HandoverEffect) {
    if !effects.contains(&effect) {
        effects.push(effect);
    }
}

/// The request's Android notification id (and its tap's PendingIntent code): stable per request,
/// from Java's `String.hashCode`, so a notification an older app posted is still cancelled.
pub(crate) fn notification_id(id: &str) -> i32 {
    let hash = id.encode_utf16().fold(0i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    });
    NOTIFICATION_BASE + (hash & NOTIFICATION_MASK)
}

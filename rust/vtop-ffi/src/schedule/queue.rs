//! [`super::Flight`]'s state, with no I/O: a line-for-line port of Kotlin's `FetchQueue`.

use std::collections::BTreeMap;

/// How many recent fetch times decide [`FetchQueue::slow`].
const RECENT: usize = 3;

/// The queue's limits (ms), read at each event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QueueConfig {
    pub(crate) fetch_lanes: i32,
    pub(crate) queue_drop_ms: i64,
    pub(crate) slow_fetch_ms: i64,
}

/// One run of `key` started in `flow` at `at` (ms); `id` orders runs, oldest first. `user`: the
/// student asked (goes first within its priority, never dropped); `has_cache`: the page has a
/// copy to keep showing if this one is dropped; `background`: the open-app sweep (lowest
/// priority); `logged`: its wait was logged as `queued`, so its start is logged too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ticket {
    pub(crate) id: u64,
    pub(crate) key: String,
    pub(crate) flow: String,
    pub(crate) user: bool,
    pub(crate) has_cache: bool,
    pub(crate) background: bool,
    pub(crate) at: i64,
    pub(crate) logged: bool,
}

impl Ticket {
    pub(crate) fn page(&self) -> &str {
        self.key.split('/').next().unwrap_or_default()
    }
}

/// A key's run: waiting for a lane, holding one, or done with its last success at
/// `succeeded_at`. Queued and Running carry when the key's run before this one succeeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Slot {
    Queued {
        ticket: Ticket,
        succeeded_at: Option<i64>,
    },
    Running {
        ticket: Ticket,
        succeeded_at: Option<i64>,
    },
    Cooldown {
        succeeded_at: i64,
    },
}

impl Slot {
    fn succeeded_at(&self) -> Option<i64> {
        match self {
            Self::Queued { succeeded_at, .. } | Self::Running { succeeded_at, .. } => *succeeded_at,
            Self::Cooldown { succeeded_at } => Some(*succeeded_at),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QueueEvent {
    /// A new run; its key has none queued or running.
    Start(Ticket),
    /// The student asked for `key`'s run too: it goes ahead and is never dropped.
    Ask { key: String },
    /// A screen showing `page` came (`on`) or went.
    Shown { page: String, on: bool },
    /// A page fetch in `flow` took `ms` and came over UDP or not.
    Fetched { flow: String, ms: i64, udp: bool },
    /// Run `ticket_id` of `key` ended: it `succeeded`, or failed, was dropped or cancelled.
    Finished {
        ticket_id: u64,
        key: String,
        flow: String,
        succeeded: bool,
    },
    /// Sign-out: every run goes, and which succeeded is forgotten.
    CancelAll,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QueueEffect {
    /// The run got a lane.
    Go(u64),
    /// The run waited off screen too long and won't run.
    Drop(u64),
    Cancel(u64),
    /// A line for src `fetch.queue`.
    Log {
        msg: &'static str,
        flow: String,
        fields: Vec<(&'static str, String)>,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct QueueStep {
    pub(crate) queue: FetchQueue,
    pub(crate) effects: Vec<QueueEffect>,
}

/// Each key's [`Slot`], the pages on screen (with how many screens show each), the last few
/// fetch times (ms) and whether the last came over UDP, and the lanes in use. [`FetchQueue::next`]
/// gives the state after an event and what to do about it; every event drops stale waiting runs
/// and starts the next while a lane is free.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FetchQueue {
    pub(crate) slots: BTreeMap<String, Slot>,
    pub(crate) on_screen: BTreeMap<String, i32>,
    pub(crate) took: Vec<i64>,
    pub(crate) udp: bool,
    pub(crate) lanes: i32,
    pub(crate) next_id: u64,
}

impl FetchQueue {
    /// `key`'s run queued or running, if one is.
    pub(crate) fn active(&self, key: &str) -> Option<&Ticket> {
        match self.slots.get(key)? {
            Slot::Queued { ticket, .. } | Slot::Running { ticket, .. } => Some(ticket),
            Slot::Cooldown { .. } => None,
        }
    }

    /// Whether `key`'s last run succeeded at most `ms` before `now`.
    pub(crate) fn succeeded_within(&self, key: &str, ms: i64, now: i64) -> bool {
        self.slots
            .get(key)
            .and_then(Slot::succeeded_at)
            .is_some_and(|at| now - at <= ms)
    }

    pub(crate) fn is_shown(&self, page: &str) -> bool {
        self.on_screen.contains_key(page)
    }

    /// Most of the last few fetches took over `slow_fetch_ms`, or the last came over UDP.
    pub(crate) fn slow(&self, config: QueueConfig) -> bool {
        let mut took = self.took.clone();
        took.sort_unstable();
        let median = took
            .get(took.len().saturating_sub(1) / 2)
            .copied()
            .unwrap_or(0);
        self.udp || median > config.slow_fetch_ms
    }

    /// Pages of the runs holding a lane, oldest first.
    pub(crate) fn running_pages(&self) -> Vec<String> {
        let mut running: Vec<&Ticket> = self
            .slots
            .values()
            .filter_map(|slot| match slot {
                Slot::Running { ticket, .. } => Some(ticket),
                _ => None,
            })
            .collect();
        running.sort_by_key(|t| t.id);
        running.into_iter().map(|t| t.page().to_owned()).collect()
    }

    pub(crate) fn next(self, event: QueueEvent, now: i64, config: QueueConfig) -> QueueStep {
        match event {
            QueueEvent::Start(t) => {
                let before = match self.slots.get(&t.key) {
                    Some(Slot::Cooldown { succeeded_at }) => Some(*succeeded_at),
                    _ => None,
                };
                let mut q = self;
                q.next_id = q.next_id.max(t.id + 1);
                q.slots.insert(
                    t.key.clone(),
                    Slot::Queued {
                        ticket: t.clone(),
                        succeeded_at: before,
                    },
                );
                let mut step = q.pump(&t.flow, now, config);
                let q = &mut step.queue;
                let still_queued = matches!(
                    q.slots.get(&t.key),
                    Some(Slot::Queued { ticket, .. }) if ticket.id == t.id
                );
                if !still_queued {
                    return step;
                }
                let position = q
                    .waiting()
                    .iter()
                    .position(|w| w.id == t.id)
                    .map_or(0, |i| i + 1);
                let fields = vec![
                    ("page", t.page().to_owned()),
                    ("position", position.to_string()),
                    ("priority", q.priority_name(&t).to_owned()),
                    ("user", t.user.to_string()),
                    ("lanes", q.lanes.to_string()),
                ];
                if let Some(Slot::Queued { ticket, .. }) = q.slots.get_mut(&t.key) {
                    ticket.logged = true;
                }
                step.effects.push(QueueEffect::Log {
                    msg: "queued",
                    flow: t.flow.clone(),
                    fields,
                });
                step
            }
            QueueEvent::Ask { key } => {
                let flow = self.active(&key).map_or_default(|t| t.flow.clone());
                let mut q = self;
                if let Some(Slot::Queued { ticket, .. } | Slot::Running { ticket, .. }) =
                    q.slots.get_mut(&key)
                {
                    ticket.user = true;
                }
                q.pump(&flow, now, config)
            }
            QueueEvent::Shown { page, on } => {
                let mut q = self;
                let n = q.on_screen.get(&page).copied().unwrap_or(0) + if on { 1 } else { -1 };
                if n <= 0 {
                    q.on_screen.remove(&page);
                } else {
                    q.on_screen.insert(page, n);
                }
                q.pump("", now, config)
            }
            QueueEvent::Fetched { flow, ms, udp } => {
                let mut q = self;
                q.took.push(ms);
                let extra = q.took.len().saturating_sub(RECENT);
                q.took.drain(..extra);
                q.udp = udp;
                q.pump(&flow, now, config)
            }
            QueueEvent::Finished {
                ticket_id,
                key,
                flow,
                succeeded,
            } => {
                let mine = self.active(&key).is_some_and(|t| t.id == ticket_id);
                let mut q = self;
                if mine {
                    let before = q.slots.get(&key).and_then(Slot::succeeded_at);
                    if succeeded {
                        q.slots.insert(key, Slot::Cooldown { succeeded_at: now });
                    } else if let Some(succeeded_at) = before {
                        q.slots.insert(key, Slot::Cooldown { succeeded_at });
                    } else {
                        q.slots.remove(&key);
                    }
                }
                q.pump(&flow, now, config)
            }
            QueueEvent::CancelAll => {
                let mut ids: Vec<u64> = self
                    .slots
                    .keys()
                    .filter_map(|key| self.active(key).map(|t| t.id))
                    .collect();
                ids.sort_unstable();
                let mut q = self;
                q.slots.clear();
                QueueStep {
                    queue: q,
                    effects: ids.into_iter().map(QueueEffect::Cancel).collect(),
                }
            }
        }
    }

    /// Waiting runs, the one to start next first.
    fn waiting(&self) -> Vec<Ticket> {
        let mut waiting: Vec<Ticket> = self
            .slots
            .values()
            .filter_map(|slot| match slot {
                Slot::Queued { ticket, .. } => Some(ticket.clone()),
                _ => None,
            })
            .collect();
        waiting.sort_by_key(|t| (self.rank(t), !t.user, t.id));
        waiting
    }

    fn rank(&self, t: &Ticket) -> usize {
        if t.background {
            2
        } else if self.is_shown(t.page()) {
            0
        } else {
            1
        }
    }

    fn priority_name(&self, t: &Ticket) -> &'static str {
        ["on_screen", "left", "background"]
            .get(self.rank(t))
            .copied()
            .unwrap_or("background")
    }

    /// Drops stale waiting runs, then starts the next while a lane is free.
    fn pump(self, flow: &str, now: i64, config: QueueConfig) -> QueueStep {
        let mut effects = Vec::new();
        let want = if self.slow(config) {
            1
        } else {
            config.fetch_lanes
        };
        if want != self.lanes && self.lanes != 0 {
            let recent: Vec<String> = self.took.iter().map(i64::to_string).collect();
            effects.push(QueueEffect::Log {
                msg: "lanes",
                flow: flow.to_owned(),
                fields: vec![
                    ("from", self.lanes.to_string()),
                    ("to", want.to_string()),
                    ("udp", self.udp.to_string()),
                    ("recent_ms", recent.join(",")),
                ],
            });
        }
        let waiting = self.waiting();
        let mut q = self;
        for t in waiting {
            if t.user || !t.has_cache || q.is_shown(t.page()) || now - t.at <= config.queue_drop_ms
            {
                continue;
            }
            match q.slots.get(&t.key).and_then(Slot::succeeded_at) {
                Some(succeeded_at) => {
                    q.slots
                        .insert(t.key.clone(), Slot::Cooldown { succeeded_at });
                }
                None => {
                    q.slots.remove(&t.key);
                }
            }
            effects.push(QueueEffect::Drop(t.id));
            effects.push(QueueEffect::Log {
                msg: "dropped",
                flow: t.flow.clone(),
                fields: vec![
                    ("page", t.page().to_owned()),
                    ("waited_ms", (now - t.at).to_string()),
                ],
            });
        }
        q.lanes = want;
        loop {
            let running = q
                .slots
                .values()
                .filter(|slot| matches!(slot, Slot::Running { .. }))
                .count();
            if running as i64 >= i64::from(want) {
                break;
            }
            let Some(t) = q.waiting().into_iter().next() else {
                break;
            };
            let succeeded_at = q.slots.get(&t.key).and_then(Slot::succeeded_at);
            q.slots.insert(
                t.key.clone(),
                Slot::Running {
                    ticket: t.clone(),
                    succeeded_at,
                },
            );
            effects.push(QueueEffect::Go(t.id));
            if t.logged {
                effects.push(QueueEffect::Log {
                    msg: "started",
                    flow: t.flow.clone(),
                    fields: vec![
                        ("page", t.page().to_owned()),
                        ("waited_ms", (now - t.at).to_string()),
                        ("priority", q.priority_name(&t).to_owned()),
                    ],
                });
            }
        }
        QueueStep { queue: q, effects }
    }
}

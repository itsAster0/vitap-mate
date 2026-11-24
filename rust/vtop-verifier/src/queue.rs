//! The line for proof slots. Past [`Config::max_proofs`](crate::config::Config) running, a
//! phone's socket waits its turn (tokio's semaphore hands out slots in the order asked) instead
//! of being turned away; the phone's protocol does not change, it only sees pings while it
//! waits. Only a full line or a wait past `queue_wait` gets `proof_busy`.

use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures::{Sink, SinkExt};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::Instant;

/// How often a waiting socket is pinged, so neither the phone nor a proxy takes it for dead.
pub const PING_EVERY: Duration = Duration::from_secs(10);

pub struct Queue {
    slots: Arc<Semaphore>,
    waiting: AtomicUsize,
    max_waiting: usize,
    wait: Duration,
}

/// Why a socket got no slot.
#[derive(Debug, PartialEq, Eq)]
pub enum NoSlot {
    /// The line was full.
    Full,
    /// No slot came within the wait.
    TimedOut,
    /// A ping failed: the phone has gone.
    Gone,
}

impl Queue {
    pub fn new(max_proofs: usize, max_waiting: usize, wait: Duration) -> Self {
        Self {
            slots: Arc::new(Semaphore::new(max_proofs)),
            waiting: AtomicUsize::new(0),
            max_waiting,
            wait,
        }
    }

    /// Whether a new socket would get a slot or a place in line; checked before the upgrade,
    /// so a refused proof spends no ticket.
    pub fn has_room(&self) -> bool {
        self.slots.available_permits() > 0 || self.waiting() < self.max_waiting
    }

    /// Sockets in line now.
    pub fn waiting(&self) -> usize {
        self.waiting.load(Ordering::Relaxed)
    }

    /// Slots free now.
    pub fn free(&self) -> usize {
        self.slots.available_permits()
    }

    /// A slot, at once or after waiting in line, pinging `peer` every [`PING_EVERY`] meanwhile.
    pub async fn enter<S, M>(
        &self,
        peer: &mut S,
        ping: impl Fn() -> M,
    ) -> Result<OwnedSemaphorePermit, NoSlot>
    where
        S: Sink<M> + Unpin,
    {
        // No waiters hold the semaphore's permits back from this, as it gives freed ones to
        // the line first.
        if let Ok(slot) = self.slots.clone().try_acquire_owned() {
            return Ok(slot);
        }
        if self.waiting.fetch_add(1, Ordering::Relaxed) >= self.max_waiting {
            self.waiting.fetch_sub(1, Ordering::Relaxed);
            tracing::info!(waiting = self.max_waiting, "proof: the line is full");
            return Err(NoSlot::Full);
        }
        let _place = Place(&self.waiting);
        let started = Instant::now();
        tracing::info!(waiting = self.waiting(), "proof: waiting for a slot");
        let mut slot = pin!(self.slots.clone().acquire_owned());
        let mut deadline = pin!(tokio::time::sleep(self.wait));
        let mut pings = tokio::time::interval_at(started + PING_EVERY, PING_EVERY);
        let outcome = loop {
            tokio::select! {
                slot = &mut slot => break slot.map_err(|_| NoSlot::Gone),
                () = &mut deadline => break Err(NoSlot::TimedOut),
                _ = pings.tick() => {
                    if peer.send(ping()).await.is_err() {
                        break Err(NoSlot::Gone);
                    }
                }
            }
        };
        let ms = started.elapsed().as_millis() as u64;
        tracing::info!(ms, waiting = self.waiting() - 1, outcome = ?outcome.as_ref().map(|_| ()), "proof: waited {ms} ms for a slot");
        outcome
    }
}

/// A place in line, given up however the wait ends.
struct Place<'a>(&'a AtomicUsize);

impl Drop for Place<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

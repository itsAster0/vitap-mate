//! The app's executor: work that outlives the call that started it (the fetch queue's runs, the
//! sweep, sync, consent timeouts) runs here, on smol, never on the futures Kotlin polls.
//!
//! One process-wide [`smol::Executor`] on two `vtop-rt` threads. A [`smol::Task`] cancels when
//! dropped, so dropping a handle is how work stops; `.detach()` lets it run on. Time goes through
//! [`Clock`], so tests drive it by hand ([`ManualClock`]).

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Once};
use std::time::{Duration, SystemTime};

use tracing::{Instrument, info_span, warn};

static EXECUTOR: smol::Executor<'static> = smol::Executor::new();
static THREADS: Once = Once::new();
const THREAD_COUNT: usize = 2;

/// Wall time in milliseconds and sleeps, so tests can drive time.
pub(crate) trait Clock: Send + Sync {
    /// Unix milliseconds.
    fn now_ms(&self) -> i64;
    /// Ends after `ms`.
    fn sleep(&self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

/// The phone's clock and smol's timer.
pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis() as i64)
    }

    fn sleep(&self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move {
            smol::Timer::after(Duration::from_millis(ms)).await;
        })
    }
}

/// [`Runtime::timeout`] ran out first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TimedOut;

pub(crate) struct Runtime {
    clock: Arc<dyn Clock>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new(Arc::new(SystemClock))
    }
}

impl Runtime {
    /// Starts the executor's threads the first time.
    pub(crate) fn new(clock: Arc<dyn Clock>) -> Self {
        THREADS.call_once(|| {
            for _ in 0..THREAD_COUNT {
                let started = std::thread::Builder::new()
                    .name("vtop-rt".into())
                    .spawn(|| smol::block_on(EXECUTOR.run(smol::future::pending::<()>())));
                if let Err(error) = started {
                    warn!(%error, "runtime thread not started");
                }
            }
        });
        Self { clock }
    }

    pub(crate) fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }

    /// Runs `fut` on the executor in flow `flow`. Dropping the task cancels it at its next await;
    /// `.detach()` it to let it outlive the caller.
    pub(crate) fn spawn<T: Send + 'static>(
        &self,
        flow: &str,
        fut: impl Future<Output = T> + Send + 'static,
    ) -> smol::Task<T> {
        EXECUTOR.spawn(fut.instrument(info_span!("task", flow)))
    }

    /// `fut`'s output, or [`TimedOut`] once `ms` pass on the clock first.
    pub(crate) async fn timeout<F: Future>(&self, ms: u64, fut: F) -> Result<F::Output, TimedOut> {
        smol::future::or(async { Ok(fut.await) }, async {
            self.clock.sleep(ms).await;
            Err(TimedOut)
        })
        .await
    }
}

/// A clock that moves only on [`ManualClock::advance`], waking the sleeps it passes.
#[cfg(test)]
#[derive(Clone, Default)]
pub(crate) struct ManualClock {
    state: Arc<(std::sync::Mutex<Manual>, std::sync::Condvar)>,
}

#[cfg(test)]
#[derive(Default)]
struct Manual {
    now: i64,
    next_sleep: u64,
    /// Pending sleeps by id: when each ends and who to wake.
    sleepers: std::collections::BTreeMap<u64, (i64, std::task::Waker)>,
}

#[cfg(test)]
impl ManualClock {
    fn lock(&self) -> std::sync::MutexGuard<'_, Manual> {
        self.state.0.lock().unwrap()
    }

    pub(crate) fn advance(&self, ms: u64) {
        let due: Vec<_> = {
            let mut state = self.lock();
            state.now += ms as i64;
            let now = state.now;
            let ids: Vec<u64> = state
                .sleepers
                .iter()
                .filter(|(_, (at, _))| *at <= now)
                .map(|(id, _)| *id)
                .collect();
            ids.into_iter()
                .filter_map(|id| state.sleepers.remove(&id))
                .collect()
        };
        for (_, waker) in due {
            waker.wake();
        }
    }

    /// Blocks until `count` sleeps wait on this clock, so an advance can't land before them.
    pub(crate) fn wait_for_sleepers(&self, count: usize) {
        let (lock, changed) = &*self.state;
        let state = lock.lock().unwrap();
        drop(
            changed
                .wait_while(state, |state| state.sleepers.len() < count)
                .unwrap(),
        );
    }
}

#[cfg(test)]
impl Clock for ManualClock {
    fn now_ms(&self) -> i64 {
        self.lock().now
    }

    fn sleep(&self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let clock = self.clone();
        let (id, at) = {
            let mut state = self.lock();
            state.next_sleep += 1;
            (state.next_sleep, state.now + ms as i64)
        };
        Box::pin(std::future::poll_fn(move |cx| {
            let mut state = clock.lock();
            if state.now >= at {
                state.sleepers.remove(&id);
                return std::task::Poll::Ready(());
            }
            state.sleepers.insert(id, (at, cx.waker().clone()));
            drop(state);
            clock.state.1.notify_all();
            std::task::Poll::Pending
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use smol::future;

    use super::*;

    fn manual() -> (Runtime, ManualClock) {
        let clock = ManualClock::default();
        (Runtime::new(Arc::new(clock.clone())), clock)
    }

    #[test]
    fn spawn_runs_detached_from_caller() {
        let (runtime, _) = manual();
        let (tx, rx) = smol::channel::bounded(1);
        let mut caller = Box::pin(async {
            runtime
                .spawn("f1", async move { tx.send(7).await.unwrap() })
                .detach();
            future::pending::<()>().await;
        });
        assert!(future::block_on(future::poll_once(&mut caller)).is_none());
        drop(caller);
        assert_eq!(future::block_on(rx.recv()), Ok(7));
    }

    #[test]
    fn dropped_task_stops_before_next_await() {
        let (runtime, clock) = manual();
        let finished = Arc::new(AtomicBool::new(false));
        let (tx, rx) = smol::channel::bounded(1);
        let task = runtime.spawn("f1", {
            let finished = finished.clone();
            let clock = clock.clone();
            async move {
                tx.send(()).await.unwrap();
                clock.sleep(10).await;
                finished.store(true, Ordering::SeqCst);
            }
        });
        future::block_on(rx.recv()).unwrap();
        // Drops the task and waits until it is gone.
        assert_eq!(future::block_on(task.cancel()), None);
        clock.advance(10);
        assert!(!finished.load(Ordering::SeqCst));
    }

    #[test]
    fn timeout_fires_on_manual_clock() {
        let (runtime, clock) = manual();
        let mut timed = Box::pin(runtime.timeout(100, future::pending::<()>()));
        assert!(future::block_on(future::poll_once(&mut timed)).is_none());
        clock.advance(99);
        assert!(future::block_on(future::poll_once(&mut timed)).is_none());
        clock.advance(1);
        assert_eq!(future::block_on(timed), Err(TimedOut));

        let quick = runtime.timeout(100, async { 5 });
        assert_eq!(future::block_on(quick), Ok(5));
    }
}

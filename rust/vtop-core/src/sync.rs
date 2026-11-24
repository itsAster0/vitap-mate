//! Waiting that needs no runtime: an async read-write lock and a sleep. The app's library has
//! no tokio; its futures run on whatever polls them (Kotlin coroutines through UniFFI).

use std::cell::UnsafeCell;
use std::collections::BTreeMap;
use std::future::{Future, poll_fn};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::{Condvar, LazyLock, Mutex, MutexGuard, OnceLock, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

/// An async read-write lock, fair to writers: while one waits, new readers queue behind it
/// (as tokio's does), so a re-login is never starved by fetches. As a mutex: `RwLock<()>`
/// and [`RwLock::write`].
pub struct RwLock<T> {
    state: Mutex<State>,
    value: UnsafeCell<T>,
}

#[derive(Default)]
struct State {
    readers: usize,
    writer: bool,
    queued_writers: usize,
    /// Every waiter, woken together on each release; each checks again.
    wakers: Vec<Waker>,
}

// SAFETY: `state` hands out shared access to readers or exclusive access to one writer, as
// std's RwLock does; moving the lock moves the `T` it owns, so `T: Send` is enough.
unsafe impl<T: Send> Send for RwLock<T> {}
// SAFETY: as std's RwLock: readers on several threads share `&T` (so `T: Sync`), and a
// writer on any thread gets `&mut T` (so `T: Send`).
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T: Default> Default for RwLock<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            state: Mutex::default(),
            value: UnsafeCell::new(value),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Wakes every waiter, outside the state lock.
    fn release(&self, change: impl FnOnce(&mut State)) {
        let wakers = {
            let mut state = self.state();
            change(&mut state);
            std::mem::take(&mut state.wakers)
        };
        wakers.into_iter().for_each(Waker::wake);
    }

    pub async fn read(&self) -> ReadGuard<'_, T> {
        poll_fn(|context| {
            let mut state = self.state();
            if !state.writer && state.queued_writers == 0 {
                state.readers += 1;
                return Poll::Ready(ReadGuard(self));
            }
            state.park(context.waker());
            Poll::Pending
        })
        .await
    }

    pub async fn write(&self) -> WriteGuard<'_, T> {
        /// Counts the writer as queued until it holds the lock or is dropped waiting.
        struct Queued<'a, T>(&'a RwLock<T>, bool);
        impl<T> Drop for Queued<'_, T> {
            fn drop(&mut self) {
                if self.1 {
                    self.0.release(|state| state.queued_writers -= 1);
                }
            }
        }
        self.state().queued_writers += 1;
        let mut queued = Queued(self, true);
        poll_fn(|context| {
            let mut state = self.state();
            if !state.writer && state.readers == 0 {
                state.writer = true;
                state.queued_writers -= 1;
                queued.1 = false;
                return Poll::Ready(WriteGuard(self));
            }
            state.park(context.waker());
            Poll::Pending
        })
        .await
    }
}

impl State {
    fn park(&mut self, waker: &Waker) {
        if !self.wakers.iter().any(|parked| parked.will_wake(waker)) {
            self.wakers.push(waker.clone());
        }
    }
}

pub struct ReadGuard<'a, T>(&'a RwLock<T>);

impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: a reader holds the lock; no writer does.
        unsafe { &*self.0.value.get() }
    }
}

impl<T> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        self.0.release(|state| state.readers -= 1);
    }
}

pub struct WriteGuard<'a, T>(&'a RwLock<T>);

impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: this writer holds the lock alone.
        unsafe { &*self.0.value.get() }
    }
}

impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: this writer holds the lock alone.
        unsafe { &mut *self.0.value.get() }
    }
}

impl<T> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        self.0.release(|state| state.writer = false);
    }
}

/// Waits `duration` on the process's one timer thread, so any executor can poll it; dropping
/// it takes its deadline off. If the thread can't be started it doesn't wait.
pub fn sleep(duration: Duration) -> impl Future<Output = ()> + Send {
    Sleep {
        // Too far to count is never.
        deadline: Instant::now().checked_add(duration),
        id: None,
    }
}

struct Sleep {
    deadline: Option<Instant>,
    /// Its entry in [`Timers`] while one is kept.
    id: Option<u64>,
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let Some(deadline) = self.deadline else {
            return Poll::Pending;
        };
        if Instant::now() >= deadline {
            if let Some(id) = self.id.take() {
                Wheel::get().timers().remove(deadline, id);
            }
            return Poll::Ready(());
        }
        let Some(wheel) = Wheel::started() else {
            return Poll::Ready(());
        };
        let mut timers = wheel.timers();
        match self.id {
            Some(id) => timers.rewake(id, deadline, context.waker()),
            None => {
                self.id = Some(timers.add(deadline, context.waker().clone()));
                drop(timers);
                wheel.changed.notify_one();
            }
        }
        Poll::Pending
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        if let (Some(deadline), Some(id)) = (self.deadline, self.id) {
            Wheel::get().timers().remove(deadline, id);
        }
    }
}

/// The deadlines being waited for, soonest first; the id tells two at one instant apart and
/// lets a dropped [`Sleep`] take its own off.
#[derive(Default)]
struct Timers {
    due: BTreeMap<(Instant, u64), Waker>,
    next_id: u64,
}

impl Timers {
    fn add(&mut self, deadline: Instant, waker: Waker) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.due.insert((deadline, id), waker);
        id
    }

    /// Wakes the latest poll's waker instead; a deadline already taken stays taken.
    fn rewake(&mut self, id: u64, deadline: Instant, waker: &Waker) {
        if let Some(kept) = self.due.get_mut(&(deadline, id))
            && !kept.will_wake(waker)
        {
            kept.clone_from(waker);
        }
    }

    fn remove(&mut self, deadline: Instant, id: u64) {
        self.due.remove(&(deadline, id));
    }

    /// Takes the wakers due at `now`, and says when the next one is.
    fn take_due(&mut self, now: Instant) -> (Vec<Waker>, Option<Instant>) {
        let later = self.due.split_off(&(now, u64::MAX));
        let due = std::mem::replace(&mut self.due, later);
        let next = self.due.first_key_value().map(|(&(at, _), _)| at);
        (due.into_values().collect(), next)
    }
}

struct Wheel {
    timers: Mutex<Timers>,
    /// Told of each new deadline, which may be sooner than the one the thread waits for.
    changed: Condvar,
}

impl Wheel {
    fn get() -> &'static Self {
        static WHEEL: LazyLock<Wheel> = LazyLock::new(|| Wheel {
            timers: Mutex::default(),
            changed: Condvar::new(),
        });
        &WHEEL
    }

    /// The wheel, once its thread runs.
    fn started() -> Option<&'static Self> {
        static STARTED: OnceLock<bool> = OnceLock::new();
        let wheel = Self::get();
        STARTED
            .get_or_init(|| {
                std::thread::Builder::new()
                    .name("vtop-sleep".into())
                    .spawn(move || wheel.run())
                    .is_ok()
            })
            .then_some(wheel)
    }

    fn timers(&self) -> MutexGuard<'_, Timers> {
        self.timers.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn run(&self) {
        let mut timers = self.timers();
        loop {
            let now = Instant::now();
            let (due, next) = timers.take_due(now);
            if !due.is_empty() {
                drop(timers);
                due.into_iter().for_each(Waker::wake);
                timers = self.timers();
                continue;
            }
            timers = match next {
                Some(at) => {
                    self.changed
                        .wait_timeout(timers, at - now)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0
                }
                None => self
                    .changed
                    .wait(timers)
                    .unwrap_or_else(PoisonError::into_inner),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Wake;

    use super::*;

    /// Counts its wakes.
    struct Count(AtomicUsize);

    impl Wake for Count {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn counter() -> (Arc<Count>, Waker) {
        let count = Arc::new(Count(AtomicUsize::new(0)));
        (count.clone(), Waker::from(count))
    }

    #[test]
    fn timers_wake_in_deadline_order_and_say_the_next() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let mut timers = Timers::default();
        let (late, late_waker) = counter();
        let (soon, soon_waker) = counter();
        timers.add(at(30), late_waker);
        timers.add(at(10), soon_waker.clone());
        timers.add(at(10), soon_waker);
        let (due, next) = timers.take_due(at(5));
        assert!(due.is_empty());
        assert_eq!(next, Some(at(10)));
        let (due, next) = timers.take_due(at(10));
        due.into_iter().for_each(Waker::wake);
        assert_eq!(soon.0.load(Ordering::SeqCst), 2);
        assert_eq!(late.0.load(Ordering::SeqCst), 0);
        assert_eq!(next, Some(at(30)));
        let (due, next) = timers.take_due(at(31));
        assert_eq!(due.len(), 1);
        assert_eq!(next, None);
    }

    #[test]
    fn a_removed_timer_never_wakes() {
        let start = Instant::now();
        let mut timers = Timers::default();
        let (count, waker) = counter();
        let id = timers.add(start, waker.clone());
        let kept = timers.add(start + Duration::from_secs(1), waker);
        timers.remove(start, id);
        let (due, next) = timers.take_due(start);
        assert!(due.is_empty());
        assert_eq!(next, Some(start + Duration::from_secs(1)));
        timers.remove(start + Duration::from_secs(1), kept);
        assert!(timers.due.is_empty());
        assert_eq!(count.0.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn rewake_keeps_the_newest_waker() {
        let start = Instant::now();
        let mut timers = Timers::default();
        let (first, first_waker) = counter();
        let (second, second_waker) = counter();
        let id = timers.add(start, first_waker);
        timers.rewake(id, start, &second_waker);
        timers.take_due(start).0.into_iter().for_each(Waker::wake);
        assert_eq!(first.0.load(Ordering::SeqCst), 0);
        assert_eq!(second.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_dropped_sleep_leaves_no_timer() {
        let mut waiting = Box::pin(Sleep {
            deadline: Instant::now().checked_add(Duration::from_secs(60)),
            id: None,
        });
        let (_, waker) = counter();
        let polled = waiting.as_mut().poll(&mut Context::from_waker(&waker));
        assert!(polled.is_pending());
        let id = waiting.id.unwrap();
        let kept = |id| Wheel::get().timers().due.keys().any(|&(_, at)| at == id);
        assert!(kept(id));
        drop(waiting);
        assert!(!kept(id));
    }

    #[tokio::test]
    async fn a_queued_writer_goes_before_new_readers() {
        let lock = Arc::new(RwLock::new(0));
        let first = lock.read().await;
        let writer = tokio::spawn({
            let lock = lock.clone();
            async move { *lock.write().await += 1 }
        });
        tokio::task::yield_now().await;
        let reader = tokio::spawn({
            let lock = lock.clone();
            async move { *lock.read().await }
        });
        tokio::task::yield_now().await;
        drop(first);
        writer.await.unwrap();
        assert_eq!(reader.await.unwrap(), 1);
    }

    #[tokio::test]
    async fn a_dropped_writer_lets_readers_in() {
        let lock = RwLock::new(());
        let first = lock.read().await;
        let waiting = tokio::time::timeout(Duration::from_millis(20), lock.write()).await;
        assert!(waiting.is_err());
        drop(first);
        let _second = lock.read().await;
    }

    #[tokio::test]
    async fn sleep_waits() {
        let start = Instant::now();
        sleep(Duration::from_millis(30)).await;
        assert!(start.elapsed() >= Duration::from_millis(30));
    }
}

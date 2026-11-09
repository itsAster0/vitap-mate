//! Waiting that needs no runtime: an async read-write lock and a sleep. The app's library has
//! no tokio; its futures run on whatever polls them (Kotlin coroutines through UniFFI).

use std::cell::UnsafeCell;
use std::future::{poll_fn, Future};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Poll, Waker};
use std::time::Duration;

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
// std's RwLock does.
unsafe impl<T: Send> Send for RwLock<T> {}
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

/// Waits `duration` on a timer thread of its own, so any executor can poll it. One that
/// can't start the thread doesn't wait.
pub fn sleep(duration: Duration) -> impl Future<Output = ()> + Send {
    // Done, and the waker of the last poll.
    let shared = Arc::new(Mutex::new((false, None::<Waker>)));
    let mut started = false;
    poll_fn(move |context| {
        let lock = || shared.lock().unwrap_or_else(PoisonError::into_inner);
        let mut timer = lock();
        if timer.0 {
            return Poll::Ready(());
        }
        timer.1 = Some(context.waker().clone());
        if !started {
            started = true;
            let shared = shared.clone();
            let spawned = std::thread::Builder::new()
                .name("vtop-sleep".into())
                .spawn(move || {
                    std::thread::sleep(duration);
                    let mut timer = shared.lock().unwrap_or_else(PoisonError::into_inner);
                    timer.0 = true;
                    let waker = timer.1.take();
                    drop(timer);
                    if let Some(waker) = waker {
                        waker.wake();
                    }
                });
            if spawned.is_err() {
                return Poll::Ready(());
            }
        }
        Poll::Pending
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let start = std::time::Instant::now();
        sleep(Duration::from_millis(30)).await;
        assert!(start.elapsed() >= Duration::from_millis(30));
    }
}

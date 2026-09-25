//! Per-API-key token bucket.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::auth::KeyId;
use crate::error::ApiError;

struct Bucket {
    tokens: f64,
    refilled_at: Instant,
}

pub struct RateLimiter {
    capacity: f64,
    per_second: f64,
    buckets: Mutex<HashMap<KeyId, Bucket>>,
}

impl RateLimiter {
    /// Allows `per_minute` requests a minute per key, with bursts up to the
    /// same number. Zero turns limiting off.
    pub fn new(per_minute: u32) -> Self {
        Self {
            capacity: f64::from(per_minute),
            per_second: f64::from(per_minute) / 60.0,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    pub fn check(&self, key: KeyId) -> Result<(), ApiError> {
        self.check_at(key, Instant::now())
    }

    fn check_at(&self, key: KeyId, now: Instant) -> Result<(), ApiError> {
        if self.capacity <= 0.0 {
            return Ok(());
        }
        let mut buckets = self
            .buckets
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let bucket = buckets.entry(key).or_insert(Bucket {
            tokens: self.capacity,
            refilled_at: now,
        });
        let elapsed = now.saturating_duration_since(bucket.refilled_at);
        bucket.tokens =
            (bucket.tokens + elapsed.as_secs_f64() * self.per_second).min(self.capacity);
        bucket.refilled_at = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            return Ok(());
        }
        let wait_secs = ((1.0 - bucket.tokens) / self.per_second).ceil();
        Err(ApiError::RateLimited {
            retry_after_secs: (wait_secs as u64).max(1),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn limits_each_key_separately_and_refills() {
        let limiter = RateLimiter::new(2);
        let start = Instant::now();
        assert!(limiter.check_at(KeyId(0), start).is_ok());
        assert!(limiter.check_at(KeyId(0), start).is_ok());
        assert!(matches!(
            limiter.check_at(KeyId(0), start),
            Err(ApiError::RateLimited {
                retry_after_secs: 30
            })
        ));
        assert!(limiter.check_at(KeyId(1), start).is_ok());
        assert!(limiter
            .check_at(KeyId(0), start + Duration::from_secs(30))
            .is_ok());
    }

    #[test]
    fn zero_disables_limiting() {
        let limiter = RateLimiter::new(0);
        for _ in 0..1000 {
            assert!(limiter.check(KeyId(0)).is_ok());
        }
    }
}

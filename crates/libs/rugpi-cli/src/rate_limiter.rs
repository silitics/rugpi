//! Utilities for rate-limiting.

use std::error::Error;
use std::fmt;
use std::sync::{Mutex, TryLockError};
use std::time::{Duration, Instant};

/// Rate limiter.
#[derive(Debug)]
pub(crate) struct RateLimiter {
    /// Instant of the last invocation.
    last_invocation: Mutex<Option<Instant>>,
    /// Minimal time between invocations.
    min_period: Duration,
}

impl RateLimiter {
    /// Create a new [`RateLimiter`] with the given minimal period.
    pub(crate) fn new(min_period: Duration) -> Self {
        Self {
            last_invocation: Mutex::default(),
            min_period,
        }
    }

    /// Rate-limit the invocation of the given closure.
    pub(crate) fn rate_limited<F: FnOnce() -> R, R>(
        &self,
        closure: F,
    ) -> Result<R, RateLimitedError> {
        let mut last_invocation = match self.last_invocation.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(error)) => {
                /*
                    We ignore a poisoned lock. The stored state is still valid as the last
                    invocation instant is only updated after the closure is called. Maybe
                    this time, the closure succeeds?
                */
                error.into_inner()
            }
            Err(TryLockError::WouldBlock) => return Err(RateLimitedError::ConcurrentInvocation),
        };
        let passed = last_invocation
            .map(|instant| instant.elapsed())
            .unwrap_or(Duration::MAX);
        if passed < self.min_period {
            return Err(RateLimitedError::TooFast { passed });
        }
        let return_value = closure();
        *last_invocation = Some(Instant::now());
        Ok(return_value)
    }
}

/// Rate has been limited.
#[derive(Debug)]
pub(crate) enum RateLimitedError {
    /// Rate-limited due to an ongoing, concurrent invocation.
    ConcurrentInvocation,
    /// Rate-limited due to exceeding the maximal rate.
    TooFast {
        /// Time passed since the last invocation.
        #[expect(dead_code, reason = "not currently used")]
        passed: Duration,
    },
}

impl fmt::Display for RateLimitedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RateLimitedError::ConcurrentInvocation => "concurrent invocation",
            RateLimitedError::TooFast { .. } => "maximal rate has been exceeded",
        })
    }
}

impl Error for RateLimitedError {}

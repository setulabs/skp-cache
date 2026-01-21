use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};

use skp_cache_core::CacheError;

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Closed,
    Open(Instant), // Instant when it opened
    HalfOpen,
}

#[derive(Debug)]
struct Inner {
    state: State,
    failures: u32,
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    inner: Arc<RwLock<Inner>>,
    failure_threshold: u32,
    reset_timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                state: State::Closed,
                failures: 0,
            })),
            failure_threshold,
            reset_timeout,
        }
    }

    /// Check if we can execute a request
    pub fn allow_request(&self) -> bool {
        let mut inner = self.inner.write();
        match inner.state {
            State::Closed => true,
            State::Open(opened_at) => {
                if opened_at.elapsed() >= self.reset_timeout {
                    inner.state = State::HalfOpen;
                    true
                } else {
                    false
                }
            }
            State::HalfOpen => {
                 // In simple implementation, we allow 1 request?
                 // Or we allow all requests in HalfOpen (and if one fails, back to Open)?
                 // For simplified logic: return true.
                 // A strict breaker allows only 1 probe.
                 // We'll allow parallel probes for simplicity here, 
                 // understanding that we might get multiple failures before reopening.
                 // But typically HalfOpen allows 1. 
                 // Let's assume the caller will call report_result.
                 true
            }
        }
    }

    /// Report success
    pub fn report_success(&self) {
        let mut inner = self.inner.write();
        if matches!(inner.state, State::HalfOpen) {
            inner.state = State::Closed;
            inner.failures = 0;
        } else if matches!(inner.state, State::Closed) {
            // Also reset failures on success in Closed state (decay)
             inner.failures = 0;
        }
    }

    /// Report failure
    pub fn report_failure(&self) {
        let mut inner = self.inner.write();
        match inner.state {
            State::Closed => {
                inner.failures += 1;
                if inner.failures >= self.failure_threshold {
                    inner.state = State::Open(Instant::now());
                }
            }
            State::HalfOpen => {
                inner.state = State::Open(Instant::now());
            }
            State::Open(_) => {
                // Already open, do nothing (maybe update timestamp?)
            }
        }
    }
    
    /// Helper to classify if an error should trip the breaker
    pub fn is_failure(err: &CacheError) -> bool {
        matches!(
            err,
            CacheError::Connection(_) | CacheError::Backend(_) | CacheError::Timeout | CacheError::Internal(_)
        )
    }
}

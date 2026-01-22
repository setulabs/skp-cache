use crate::{CacheMetrics, CacheOperation, CacheTier, EvictionReason};
use std::time::Duration;
use tracing::{debug, info};

/// Metrics adapter that logs events via `tracing`
#[derive(Debug, Clone, Default)]
pub struct TracingMetrics {
    /// Service name/prefix (optional)
    service_name: Option<String>,
}

impl TracingMetrics {
    /// Create new tracing metrics adapter
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with service name prefix
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = Some(name.into());
        self
    }
}

impl CacheMetrics for TracingMetrics {
    fn record_hit(&self, key: &str, tier: CacheTier) {
        debug!(
            target: "skp_cache",
            event = "hit",
            key = %key,
            tier = ?tier,
            service = ?self.service_name,
            "Cache Hit"
        );
    }

    fn record_miss(&self, key: &str) {
        debug!(
            target: "skp_cache",
            event = "miss",
            key = %key,
            service = ?self.service_name,
            "Cache Miss"
        );
    }

    fn record_stale_hit(&self, key: &str) {
         debug!(
            target: "skp_cache",
            event = "stale_hit",
            key = %key,
            service = ?self.service_name,
            "Cache Stale Hit"
        );
    }

    fn record_latency(&self, operation: CacheOperation, duration: Duration) {
        tracing::trace!(
            target: "skp_cache",
            event = "latency",
            operation = ?operation,
            duration_ms = duration.as_millis(),
            service = ?self.service_name,
            "Cache Operation Latency"
        );
    }

    fn record_eviction(&self, reason: EvictionReason) {
        debug!(
            target: "skp_cache",
            event = "eviction",
            reason = ?reason,
            service = ?self.service_name,
            "Cache Eviction"
        );
    }

    fn record_size(&self, size: usize, memory_bytes: usize) {
        tracing::trace!(
            target: "skp_cache",
            event = "size",
            size = size,
            bytes = memory_bytes,
            service = ?self.service_name,
            "Cache Size Update"
        );
    }
}

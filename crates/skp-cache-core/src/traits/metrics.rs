//! Metrics trait for cache observability

use std::time::Duration;

/// Cache tier for metrics labeling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheTier {
    /// L1 in-memory cache
    L1Memory,
    /// L2 Redis or distributed cache
    L2Redis,
}

impl CacheTier {
    /// Get tier as string label
    pub fn as_str(&self) -> &'static str {
        match self {
            CacheTier::L1Memory => "l1_memory",
            CacheTier::L2Redis => "l2_redis",
        }
    }
}

/// Cache operation for latency tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheOperation {
    Get,
    Set,
    Delete,
    Serialize,
    Deserialize,
    Invalidate,
}

impl CacheOperation {
    /// Get operation as string label
    pub fn as_str(&self) -> &'static str {
        match self {
            CacheOperation::Get => "get",
            CacheOperation::Set => "set",
            CacheOperation::Delete => "delete",
            CacheOperation::Serialize => "serialize",
            CacheOperation::Deserialize => "deserialize",
            CacheOperation::Invalidate => "invalidate",
        }
    }
}

/// Reason for cache eviction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvictionReason {
    /// TTL expired
    Expired,
    /// Capacity limit reached
    Capacity,
    /// Explicitly invalidated
    Invalidated,
    /// Replaced by new value
    Replaced,
    /// Dependency was invalidated
    DependencyInvalidated,
}

impl EvictionReason {
    /// Get reason as string label
    pub fn as_str(&self) -> &'static str {
        match self {
            EvictionReason::Expired => "expired",
            EvictionReason::Capacity => "capacity",
            EvictionReason::Invalidated => "invalidated",
            EvictionReason::Replaced => "replaced",
            EvictionReason::DependencyInvalidated => "dependency",
        }
    }
}

/// Trait for cache metrics/observability
///
/// Implement this to integrate with your metrics system (Prometheus, StatsD, etc.)
pub trait CacheMetrics: Send + Sync + 'static {
    /// Record a cache hit
    fn record_hit(&self, key: &str, tier: CacheTier);

    /// Record a cache miss
    fn record_miss(&self, key: &str);

    /// Record a stale hit (served stale while revalidating)
    fn record_stale_hit(&self, key: &str);

    /// Record operation latency
    fn record_latency(&self, operation: CacheOperation, duration: Duration);

    /// Record an eviction
    fn record_eviction(&self, reason: EvictionReason);

    /// Record cache size
    fn record_size(&self, size: usize, memory_bytes: usize);
}

/// No-op metrics implementation (default)
///
/// Zero overhead when metrics are not needed.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopMetrics;

impl CacheMetrics for NoopMetrics {
    #[inline]
    fn record_hit(&self, _key: &str, _tier: CacheTier) {}

    #[inline]
    fn record_miss(&self, _key: &str) {}

    #[inline]
    fn record_stale_hit(&self, _key: &str) {}

    #[inline]
    fn record_latency(&self, _operation: CacheOperation, _duration: Duration) {}

    #[inline]
    fn record_eviction(&self, _reason: EvictionReason) {}

    #[inline]
    fn record_size(&self, _size: usize, _memory_bytes: usize) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_as_str() {
        assert_eq!(CacheTier::L1Memory.as_str(), "l1_memory");
        assert_eq!(CacheTier::L2Redis.as_str(), "l2_redis");
    }

    #[test]
    fn test_operation_as_str() {
        assert_eq!(CacheOperation::Get.as_str(), "get");
        assert_eq!(CacheOperation::Set.as_str(), "set");
    }

    #[test]
    fn test_eviction_reason_as_str() {
        assert_eq!(EvictionReason::Expired.as_str(), "expired");
        assert_eq!(EvictionReason::Capacity.as_str(), "capacity");
    }

    #[test]
    fn test_noop_metrics() {
        let metrics = NoopMetrics;
        // Just verify these don't panic
        metrics.record_hit("key", CacheTier::L1Memory);
        metrics.record_miss("key");
        metrics.record_latency(CacheOperation::Get, Duration::from_millis(1));
    }
}

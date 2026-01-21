# skp-cache: Advanced Caching Library for Rust

## 📋 Executive Summary

**skp-cache** is a modular, high-performance caching library for Rust applications featuring:

- **Multi-tier caching** (L1 Memory + L2 Redis) with circuit breaker
- **Dependency graph-based invalidation** (unique differentiator)
- **Smart eviction** with W-TinyLFU, cost-awareness, and adaptive strategies
- **Stampede protection** via request coalescing and probabilistic early refresh
- **Pluggable serialization** (JSON, MessagePack, Bincode)
- **Full observability** with metrics trait integration
- **HTTP response caching** with Cache-Control awareness
- **Native Axum/Actix middleware** + standalone usage
- **Batch operations** and negative caching support
- **TTL jitter** to prevent thundering herd on expiry

---

## 🎯 Design Goals

| Goal | Description |
|------|-------------|
| **Modular** | Pluggable backends, eviction strategies, serializers, metrics |
| **Robust** | Production-ready with circuit breaker, graceful shutdown, error handling |
| **Extensible** | Trait-based design for custom implementations |
| **Generic** | Works with any `Serialize + DeserializeOwned` types |
| **Observable** | First-class metrics support via `CacheMetrics` trait |
| **Framework-agnostic** | Standalone usage + optional Axum/Actix integration |
| **High Performance** | Lock-free where possible, async-first, SCAN over KEYS |

---

## 📦 Crate Structure

```
skp-cache/
├── Cargo.toml                    # Workspace definition
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── crates/
│   ├── skp-cache/                # Facade crate (re-exports everything)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   │
│   ├── skp-cache-core/           # Core traits, types, errors
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits/
│   │       │   ├── mod.rs
│   │       │   ├── backend.rs    # CacheBackend trait
│   │       │   ├── key.rs        # CacheKey trait
│   │       │   ├── cacheable.rs  # Cacheable marker trait
│   │       │   ├── eviction.rs   # EvictionStrategy trait
│   │       │   ├── serializer.rs # Serializer trait
│   │       │   └── metrics.rs    # CacheMetrics trait
│   │       ├── types/
│   │       │   ├── mod.rs
│   │       │   ├── entry.rs      # CacheEntry<T>
│   │       │   ├── options.rs    # CacheOptions, CacheOpts builder
│   │       │   ├── result.rs     # CacheResult, CacheStatus
│   │       │   ├── config.rs     # CacheConfig
│   │       │   ├── tags.rs       # Tag, TagSet, TagPattern
│   │       │   ├── dependency.rs # DependencyNode, DependencyGraph
│   │       │   ├── ttl_wheel.rs  # TTL-based expiration index
│   │       │   └── stats.rs      # CacheStats, Metrics
│   │       ├── errors.rs         # CacheError, Result type
│   │       ├── key.rs            # Key generation utilities
│   │       └── utils.rs          # Helpers (hashing, time, etc.)
│   │
│   ├── skp-cache-storage/        # All storage backends
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── memory/
│   │       │   ├── mod.rs
│   │       │   ├── backend.rs    # MemoryBackend (DashMap-based)
│   │       │   ├── eviction.rs   # In-memory eviction handling
│   │       │   ├── bloom.rs      # Bloom filter for negative lookups
│   │       │   └── ttl_index.rs  # Time-wheel for O(1) TTL expiration
│   │       ├── redis/
│   │       │   ├── mod.rs
│   │       │   ├── backend.rs    # RedisBackend (bb8 pool)
│   │       │   ├── scripts.rs    # Lua scripts for atomic ops
│   │       │   ├── pubsub.rs     # Distributed invalidation
│   │       │   └── tags.rs       # Redis tag index management
│   │       └── multi_tier/
│   │           ├── mod.rs
│   │           ├── backend.rs    # MultiTierBackend (L1 + L2)
│   │           ├── promotion.rs  # L2 → L1 promotion logic
│   │           └── sync.rs       # Cross-tier consistency
│   │
│   ├── skp-cache-http/           # HTTP response caching
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── response.rs       # CachedResponse type
│   │       ├── cache_control.rs  # Cache-Control header parsing
│   │       ├── etag.rs           # ETag generation/validation
│   │       ├── vary.rs           # Vary header handling
│   │       └── policy.rs         # HTTP caching policies
│   │
│   ├── skp-cache-axum/           # Axum middleware
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── layer.rs          # CacheLayer (Tower Layer)
│   │       ├── middleware.rs     # Request/Response handling
│   │       ├── extractor.rs      # Cached<T> extractor
│   │       └── manager.rs        # Per-route cache policies
│   │
│   ├── skp-cache-actix/          # Actix-web middleware
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── middleware.rs     # Actix middleware impl
│   │       ├── extractor.rs      # Cached<T> extractor
│   │       └── manager.rs        # Per-route cache policies
│   │
│   └── skp-cache-derive/         # Proc macros (optional)
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs            # #[derive(CacheKey)]
│
├── examples/
│   ├── basic_memory.rs
│   ├── redis_backend.rs
│   ├── multi_tier.rs
│   ├── dependency_graph.rs
│   ├── http_caching.rs
│   ├── axum_integration.rs
│   └── actix_integration.rs
│
└── benches/
    ├── throughput.rs
    ├── eviction.rs
    └── invalidation.rs
```

---

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Application Layer                                 │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────────────┐  │
│  │  Axum Middleware │  │ Actix Middleware │  │   Standalone Usage       │  │
│  │  (skp-cache-axum)│  │(skp-cache-actix) │  │   cache.get() / set()    │  │
│  └────────┬─────────┘  └────────┬─────────┘  └────────────┬─────────────┘  │
│           │                     │                         │                 │
│           └─────────────────────┴─────────────────────────┘                 │
│                                 │                                           │
├─────────────────────────────────┼───────────────────────────────────────────┤
│                          Cache Manager                                      │
│  ┌──────────────────────────────┴──────────────────────────────────────┐   │
│  │                      CacheManager<B: CacheBackend>                   │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐  │   │
│  │  │  Coalescer  │  │  Tag Index  │  │ Dependency  │  │   Stats    │  │   │
│  │  │(Singleflight)│  │  Registry   │  │   Graph     │  │  Collector │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └────────────┘  │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                 │                                           │
├─────────────────────────────────┼───────────────────────────────────────────┤
│                          Backend Abstraction                                │
│                    trait CacheBackend (skp-cache-core)                      │
│                                 │                                           │
│  ┌──────────────────────────────┼──────────────────────────────────────┐   │
│  │                    skp-cache-storage                                 │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌────────────────────────────┐   │   │
│  │  │   Memory    │  │    Redis    │  │       MultiTier            │   │   │
│  │  │  (DashMap)  │  │ (bb8-redis) │  │   (L1: Memory + L2: Redis) │   │   │
│  │  │             │  │             │  │                            │   │   │
│  │  │ - W-TinyLFU │  │ - Lua scrip │  │ - Promotion/Demotion       │   │   │
│  │  │ - Cost-aware│  │ - Pub/Sub   │  │ - Write-through/behind     │   │   │
│  │  │ - Bloom flt │  │ - Tag index │  │ - Consistency sync         │   │   │
│  │  └─────────────┘  └─────────────┘  └────────────────────────────┘   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                          Serialization Layer                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐    │
│  │    JSON     │  │  MessagePack │  │   Bincode   │  │  Custom impl    │    │
│  │  (default)  │  │  (optional)  │  │  (optional) │  │  Serializer     │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 🔧 Core Traits & Types

### `skp-cache-core/src/traits/backend.rs`

```rust
use async_trait::async_trait;
use crate::{CacheEntry, CacheError, CacheOptions, CacheResult, CacheStats};

/// Core trait for all cache storage backends
#[async_trait]
pub trait CacheBackend: Send + Sync + 'static {
    /// Get a value from the cache
    async fn get(&self, key: &str) -> Result<Option<CacheEntry<Vec<u8>>>, CacheError>;
    
    /// Set a value in the cache with options
    async fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        options: &CacheOptions,
    ) -> Result<(), CacheError>;
    
    /// Delete a specific key
    async fn delete(&self, key: &str) -> Result<bool, CacheError>;
    
    /// Check if a key exists (without fetching value)
    async fn exists(&self, key: &str) -> Result<bool, CacheError>;
    
    /// Delete multiple keys atomically
    async fn delete_many(&self, keys: &[&str]) -> Result<u64, CacheError>;
    
    /// Get multiple keys at once
    async fn get_many(&self, keys: &[&str]) -> Result<Vec<Option<CacheEntry<Vec<u8>>>>, CacheError>;
    
    /// Set multiple key-value pairs atomically
    async fn set_many(
        &self,
        entries: &[(&str, Vec<u8>, &CacheOptions)],
    ) -> Result<(), CacheError>;
    
    /// Clear all entries in the cache
    async fn clear(&self) -> Result<(), CacheError>;
    
    /// Get cache statistics
    async fn stats(&self) -> Result<CacheStats, CacheError>;
    
    /// Get approximate number of entries
    async fn len(&self) -> Result<usize, CacheError>;
    
    /// Check if cache is empty
    async fn is_empty(&self) -> Result<bool, CacheError> {
        Ok(self.len().await? == 0)
    }
}

/// Extended trait for backends that support tagging
#[async_trait]
pub trait TaggableBackend: CacheBackend {
    /// Get all keys associated with a tag
    async fn get_keys_by_tag(&self, tag: &str) -> Result<Vec<String>, CacheError>;
    
    /// Invalidate all entries with a specific tag
    async fn invalidate_by_tag(&self, tag: &str) -> Result<u64, CacheError>;
    
    /// Invalidate entries matching a tag pattern (supports wildcards)
    async fn invalidate_by_pattern(&self, pattern: &str) -> Result<u64, CacheError>;
    
    /// Register tags for a key
    async fn register_tags(&self, key: &str, tags: &[&str]) -> Result<(), CacheError>;
    
    /// Remove tag associations for a key
    async fn unregister_tags(&self, key: &str) -> Result<(), CacheError>;
}

/// Extended trait for backends supporting distributed operations
#[async_trait]
pub trait DistributedBackend: CacheBackend {
    /// Subscribe to invalidation events
    async fn subscribe_invalidations(&self) -> Result<InvalidationSubscriber, CacheError>;
    
    /// Publish an invalidation event
    async fn publish_invalidation(&self, event: InvalidationEvent) -> Result<(), CacheError>;
    
    /// Acquire a distributed lock
    async fn acquire_lock(&self, key: &str, ttl: Duration) -> Result<LockGuard, CacheError>;
}

/// Extended trait for backends with eviction callbacks
#[async_trait]
pub trait EvictableBackend: CacheBackend {
    /// Set eviction listener
    fn set_eviction_listener<F>(&mut self, listener: F)
    where
        F: Fn(String, Vec<u8>, EvictionReason) + Send + Sync + 'static;
}
```

### `skp-cache-core/src/traits/key.rs`

```rust
use std::fmt::Display;

/// Trait for types that can be used as cache keys
pub trait CacheKey: Send + Sync {
    /// Generate the cache key string
    fn cache_key(&self) -> String;
    
    /// Optional: namespace prefix for the key
    fn namespace(&self) -> Option<&str> {
        None
    }
    
    /// Full key including namespace
    fn full_key(&self) -> String {
        match self.namespace() {
            Some(ns) => format!("{}:{}", ns, self.cache_key()),
            None => self.cache_key(),
        }
    }
}

// Implement for common types
impl CacheKey for String {
    fn cache_key(&self) -> String {
        self.clone()
    }
}

impl CacheKey for &str {
    fn cache_key(&self) -> String {
        self.to_string()
    }
}

impl<T: Display + Send + Sync> CacheKey for (T,) {
    fn cache_key(&self) -> String {
        self.0.to_string()
    }
}

impl<T1: Display + Send + Sync, T2: Display + Send + Sync> CacheKey for (T1, T2) {
    fn cache_key(&self) -> String {
        format!("{}:{}", self.0, self.1)
    }
}

/// Composite key builder for complex cache keys
#[derive(Debug, Clone)]
pub struct CompositeKey {
    parts: Vec<String>,
    namespace: Option<String>,
}

impl CompositeKey {
    pub fn new() -> Self {
        Self {
            parts: Vec::new(),
            namespace: None,
        }
    }
    
    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }
    
    pub fn part(mut self, part: impl Display) -> Self {
        self.parts.push(part.to_string());
        self
    }
    
    pub fn parts<I, T>(mut self, parts: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Display,
    {
        self.parts.extend(parts.into_iter().map(|p| p.to_string()));
        self
    }
}

impl CacheKey for CompositeKey {
    fn cache_key(&self) -> String {
        self.parts.join(":")
    }
    
    fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }
}
```

### `skp-cache-core/src/traits/eviction.rs`

```rust
use crate::CacheEntry;

/// Reason why an entry was evicted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Eviction candidate with metadata for decision making
#[derive(Debug, Clone)]
pub struct EvictionCandidate {
    pub key: String,
    pub size: usize,
    pub cost: u64,
    pub frequency: u64,
    pub last_access: Instant,
    pub created_at: Instant,
    pub ttl_remaining: Option<Duration>,
}

/// Trait for custom eviction strategies
pub trait EvictionStrategy: Send + Sync + 'static {
    /// Name of the eviction strategy
    fn name(&self) -> &str;
    
    /// Select entries to evict to free up `required_space` bytes
    /// Returns keys to evict in order of priority
    fn select_victims(
        &self,
        candidates: &[EvictionCandidate],
        required_space: usize,
    ) -> Vec<String>;
    
    /// Record an access for frequency tracking
    fn record_access(&self, key: &str);
    
    /// Record when an entry is added
    fn record_insert(&self, key: &str, size: usize, cost: u64);
    
    /// Record when an entry is removed
    fn record_remove(&self, key: &str);
    
    /// Reset all tracking data
    fn reset(&self);
}

/// Built-in eviction strategies
pub mod strategies {
    use super::*;
    
    /// Least Recently Used
    pub struct LRU;
    
    /// Least Frequently Used
    pub struct LFU;
    
    /// Window TinyLFU (Caffeine-style, best for most workloads)
    pub struct WTinyLFU {
        window_size_percent: f32,  // typically 1%
        sample_size: usize,
    }
    
    /// Cost-Aware Multi-Queue (considers computation cost)
    pub struct CAMP {
        precision: u32,
    }
    
    /// Adaptive - switches between strategies based on workload
    pub struct Adaptive {
        strategies: Vec<Box<dyn EvictionStrategy>>,
        evaluation_window: Duration,
    }
}
```

### `skp-cache-core/src/traits/serializer.rs`

```rust
use crate::CacheError;
use serde::{de::DeserializeOwned, Serialize};

/// Trait for pluggable serialization formats
pub trait Serializer: Send + Sync + Clone + 'static {
    /// Name of the serialization format
    fn name(&self) -> &str;
    
    /// Serialize a value to bytes
    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, CacheError>;
    
    /// Deserialize bytes to a value
    fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, CacheError>;
}

/// JSON serializer using serde_json (default)
#[derive(Debug, Clone, Default)]
pub struct JsonSerializer;

impl Serializer for JsonSerializer {
    fn name(&self) -> &str {
        "json"
    }
    
    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, CacheError> {
        serde_json::to_vec(value)
            .map_err(|e| CacheError::Serialization(e.to_string()))
    }
    
    fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, CacheError> {
        serde_json::from_slice(bytes)
            .map_err(|e| CacheError::Serialization(e.to_string()))
    }
}

/// MessagePack serializer using rmp-serde (optional, faster + smaller)
#[cfg(feature = "msgpack")]
#[derive(Debug, Clone, Default)]
pub struct MsgPackSerializer;

#[cfg(feature = "msgpack")]
impl Serializer for MsgPackSerializer {
    fn name(&self) -> &str {
        "msgpack"
    }
    
    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, CacheError> {
        rmp_serde::to_vec(value)
            .map_err(|e| CacheError::Serialization(e.to_string()))
    }
    
    fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, CacheError> {
        rmp_serde::from_slice(bytes)
            .map_err(|e| CacheError::Serialization(e.to_string()))
    }
}

/// Bincode serializer (optional, fastest + smallest, not human-readable)
#[cfg(feature = "bincode")]
#[derive(Debug, Clone, Default)]
pub struct BincodeSerializer;

#[cfg(feature = "bincode")]
impl Serializer for BincodeSerializer {
    fn name(&self) -> &str {
        "bincode"
    }
    
    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, CacheError> {
        bincode::serialize(value)
            .map_err(|e| CacheError::Serialization(e.to_string()))
    }
    
    fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, CacheError> {
        bincode::deserialize(bytes)
            .map_err(|e| CacheError::Serialization(e.to_string()))
    }
}
```

### `skp-cache-core/src/traits/metrics.rs`

```rust
use std::time::Duration;
use crate::EvictionReason;

/// Trait for cache metrics/observability integration
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
    
    /// Record cache size change
    fn record_size(&self, size: usize, memory_bytes: usize);
    
    /// Record a coalesced request (stampede prevention)
    fn record_coalesce(&self, key: &str);
}

/// Cache tier for metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheTier {
    L1Memory,
    L2Redis,
    L2Distributed,
}

/// Cache operation for latency tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheOperation {
    Get,
    Set,
    Delete,
    Invalidate,
    Serialize,
    Deserialize,
}

/// No-op metrics implementation (default when metrics feature is disabled)
#[derive(Debug, Clone, Default)]
pub struct NoopMetrics;

impl CacheMetrics for NoopMetrics {
    fn record_hit(&self, _key: &str, _tier: CacheTier) {}
    fn record_miss(&self, _key: &str) {}
    fn record_stale_hit(&self, _key: &str) {}
    fn record_latency(&self, _operation: CacheOperation, _duration: Duration) {}
    fn record_eviction(&self, _reason: EvictionReason) {}
    fn record_size(&self, _size: usize, _memory_bytes: usize) {}
    fn record_coalesce(&self, _key: &str) {}
}

/// Metrics implementation using the `metrics` crate
#[cfg(feature = "metrics")]
pub struct MetricsCrateAdapter {
    prefix: String,
}

#[cfg(feature = "metrics")]
impl MetricsCrateAdapter {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self { prefix: prefix.into() }
    }
}

#[cfg(feature = "metrics")]
impl CacheMetrics for MetricsCrateAdapter {
    fn record_hit(&self, _key: &str, tier: CacheTier) {
        let tier_label = match tier {
            CacheTier::L1Memory => "l1_memory",
            CacheTier::L2Redis => "l2_redis",
            CacheTier::L2Distributed => "l2_distributed",
        };
        metrics::counter!(format!("{}_hits_total", self.prefix), "tier" => tier_label).increment(1);
    }
    
    fn record_miss(&self, _key: &str) {
        metrics::counter!(format!("{}_misses_total", self.prefix)).increment(1);
    }
    
    fn record_stale_hit(&self, _key: &str) {
        metrics::counter!(format!("{}_stale_hits_total", self.prefix)).increment(1);
    }
    
    fn record_latency(&self, operation: CacheOperation, duration: Duration) {
        let op_label = match operation {
            CacheOperation::Get => "get",
            CacheOperation::Set => "set",
            CacheOperation::Delete => "delete",
            CacheOperation::Invalidate => "invalidate",
            CacheOperation::Serialize => "serialize",
            CacheOperation::Deserialize => "deserialize",
        };
        metrics::histogram!(format!("{}_operation_duration_seconds", self.prefix), "operation" => op_label)
            .record(duration.as_secs_f64());
    }
    
    fn record_eviction(&self, reason: EvictionReason) {
        let reason_label = match reason {
            EvictionReason::Expired => "expired",
            EvictionReason::Capacity => "capacity",
            EvictionReason::Invalidated => "invalidated",
            EvictionReason::Replaced => "replaced",
            EvictionReason::DependencyInvalidated => "dependency",
        };
        metrics::counter!(format!("{}_evictions_total", self.prefix), "reason" => reason_label).increment(1);
    }
    
    fn record_size(&self, size: usize, memory_bytes: usize) {
        metrics::gauge!(format!("{}_entries", self.prefix)).set(size as f64);
        metrics::gauge!(format!("{}_memory_bytes", self.prefix)).set(memory_bytes as f64);
    }
    
    fn record_coalesce(&self, _key: &str) {
        metrics::counter!(format!("{}_coalesced_requests_total", self.prefix)).increment(1);
    }
}
```

### `skp-cache-core/src/types/entry.rs`

```rust
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime};

/// A cached entry with full metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    /// The cached value
    pub value: T,
    
    /// When the entry was created
    pub created_at: SystemTime,
    
    /// When the entry was last accessed
    pub last_accessed: SystemTime,
    
    /// Number of times this entry has been accessed
    pub access_count: u64,
    
    /// Time-to-live for this entry
    pub ttl: Option<Duration>,
    
    /// Stale-while-revalidate duration (serve stale for this long after TTL)
    pub stale_while_revalidate: Option<Duration>,
    
    /// Tags associated with this entry
    pub tags: Vec<String>,
    
    /// Keys this entry depends on
    pub dependencies: Vec<String>,
    
    /// Computation cost (for cost-aware eviction)
    pub cost: u64,
    
    /// Size in bytes (for size-aware eviction)
    pub size: usize,
    
    /// ETag for HTTP caching
    pub etag: Option<String>,
    
    /// Version for optimistic concurrency
    pub version: u64,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T) -> Self {
        let now = SystemTime::now();
        Self {
            value,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            ttl: None,
            stale_while_revalidate: None,
            tags: Vec::new(),
            dependencies: Vec::new(),
            cost: 1,
            size: 0,
            etag: None,
            version: 0,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl {
            if let Ok(elapsed) = self.created_at.elapsed() {
                return elapsed > ttl;
            }
        }
        false
    }
    
    pub fn is_stale(&self) -> bool {
        if !self.is_expired() {
            return false;
        }
        if let (Some(ttl), Some(swr)) = (self.ttl, self.stale_while_revalidate) {
            if let Ok(elapsed) = self.created_at.elapsed() {
                return elapsed <= ttl + swr;
            }
        }
        false
    }
    
    pub fn ttl_remaining(&self) -> Option<Duration> {
        self.ttl.and_then(|ttl| {
            self.created_at.elapsed().ok().and_then(|elapsed| {
                ttl.checked_sub(elapsed)
            })
        })
    }
}

/// Cache lookup result with status
#[derive(Debug, Clone)]
pub enum CacheResult<T> {
    /// Cache hit - fresh data
    Hit(CacheEntry<T>),
    
    /// Cache hit - stale data (can serve while revalidating)
    Stale(CacheEntry<T>),
    
    /// Cache miss
    Miss,
    
    /// Negative cache hit (key is known to not exist in source)
    NegativeHit,
}

impl<T> CacheResult<T> {
    pub fn is_hit(&self) -> bool {
        matches!(self, CacheResult::Hit(_))
    }
    
    pub fn is_usable(&self) -> bool {
        matches!(self, CacheResult::Hit(_) | CacheResult::Stale(_))
    }
    
    pub fn value(self) -> Option<T> {
        match self {
            CacheResult::Hit(entry) | CacheResult::Stale(entry) => Some(entry.value),
            _ => None,
        }
    }
    
    pub fn entry(self) -> Option<CacheEntry<T>> {
        match self {
            CacheResult::Hit(entry) | CacheResult::Stale(entry) => Some(entry),
            _ => None,
        }
    }
}
```

### `skp-cache-core/src/types/options.rs`

```rust
use std::time::Duration;

/// Configuration options for a cache entry
#[derive(Debug, Clone, Default)]
pub struct CacheOptions {
    /// Time-to-live
    pub ttl: Option<Duration>,
    
    /// Stale-while-revalidate duration
    pub stale_while_revalidate: Option<Duration>,
    
    /// Tags for invalidation
    pub tags: Vec<String>,
    
    /// Dependencies for cascade invalidation
    pub dependencies: Vec<String>,
    
    /// Computation cost (higher = less likely to evict)
    pub cost: Option<u64>,
    
    /// Enable early probabilistic refresh
    pub early_refresh: bool,
    
    /// Enable request coalescing for this key
    pub coalesce: bool,
    
    /// Custom ETag
    pub etag: Option<String>,
    
    /// If true, this is a negative cache entry (absence of value)
    pub negative: bool,
    
    /// Condition for conditional set (version must match)
    pub if_version: Option<u64>,
}

/// Builder for CacheOptions with fluent API
#[derive(Debug, Clone, Default)]
pub struct CacheOpts(CacheOptions);

impl CacheOpts {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set TTL
    pub fn ttl(mut self, duration: Duration) -> Self {
        self.0.ttl = Some(duration);
        self
    }
    
    /// Set TTL from seconds
    pub fn ttl_secs(self, seconds: u64) -> Self {
        self.ttl(Duration::from_secs(seconds))
    }
    
    /// Enable stale-while-revalidate
    pub fn stale_while_revalidate(mut self, duration: Duration) -> Self {
        self.0.stale_while_revalidate = Some(duration);
        self
    }
    
    /// Shorthand: swr
    pub fn swr(self, duration: Duration) -> Self {
        self.stale_while_revalidate(duration)
    }
    
    /// Add tags
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.0.tags.extend(tags.into_iter().map(Into::into));
        self
    }
    
    /// Add a single tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.0.tags.push(tag.into());
        self
    }
    
    /// Add dependencies (keys this entry depends on)
    pub fn depends_on<I, S>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.0.dependencies.extend(keys.into_iter().map(Into::into));
        self
    }
    
    /// Set computation cost
    pub fn cost(mut self, cost: u64) -> Self {
        self.0.cost = Some(cost);
        self
    }
    
    /// Measure and set computation cost from duration
    pub fn cost_from_duration(mut self, duration: Duration) -> Self {
        // Convert to microseconds as cost unit
        self.0.cost = Some(duration.as_micros() as u64);
        self
    }
    
    /// Enable early probabilistic refresh
    pub fn early_refresh(mut self) -> Self {
        self.0.early_refresh = true;
        self
    }
    
    /// Enable request coalescing
    pub fn coalesce(mut self) -> Self {
        self.0.coalesce = true;
        self
    }
    
    /// Set ETag
    pub fn etag(mut self, etag: impl Into<String>) -> Self {
        self.0.etag = Some(etag.into());
        self
    }
    
    /// Mark as negative cache entry
    pub fn negative(mut self) -> Self {
        self.0.negative = true;
        self
    }
    
    /// Conditional set: only if version matches
    pub fn if_version(mut self, version: u64) -> Self {
        self.0.if_version = Some(version);
        self
    }
    
    /// Build the options
    pub fn build(self) -> CacheOptions {
        self.0
    }
}

impl From<CacheOpts> for CacheOptions {
    fn from(opts: CacheOpts) -> Self {
        opts.0
    }
}
```

### `skp-cache-core/src/types/dependency.rs`

```rust
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;

/// A node in the dependency graph
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// The cache key
    pub key: String,
    /// Keys that this key depends on (parents)
    pub depends_on: HashSet<String>,
    /// Keys that depend on this key (children)
    pub dependents: HashSet<String>,
}

/// Manages the dependency graph for cascade invalidation
pub struct DependencyGraph {
    /// Map of key -> node
    nodes: RwLock<HashMap<String, DependencyNode>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
        }
    }
    
    /// Register dependencies for a key
    pub fn register(&self, key: &str, depends_on: &[String]) {
        let mut nodes = self.nodes.write();
        
        // Create or update the node for this key
        let node = nodes.entry(key.to_string()).or_insert_with(|| DependencyNode {
            key: key.to_string(),
            depends_on: HashSet::new(),
            dependents: HashSet::new(),
        });
        
        // Add dependencies
        for dep_key in depends_on {
            node.depends_on.insert(dep_key.clone());
            
            // Update the parent node's dependents
            let parent = nodes.entry(dep_key.clone()).or_insert_with(|| DependencyNode {
                key: dep_key.clone(),
                depends_on: HashSet::new(),
                dependents: HashSet::new(),
            });
            parent.dependents.insert(key.to_string());
        }
    }
    
    /// Remove a key from the graph
    pub fn remove(&self, key: &str) {
        let mut nodes = self.nodes.write();
        
        if let Some(node) = nodes.remove(key) {
            // Remove this key from all parents' dependents
            for parent_key in &node.depends_on {
                if let Some(parent) = nodes.get_mut(parent_key) {
                    parent.dependents.remove(key);
                }
            }
            
            // Remove this key from all children's depends_on
            for child_key in &node.dependents {
                if let Some(child) = nodes.get_mut(child_key) {
                    child.depends_on.remove(key);
                }
            }
        }
    }
    
    /// Get all keys that should be invalidated when the given key is invalidated
    /// Uses BFS to traverse all dependents
    pub fn get_cascade_invalidation(&self, key: &str) -> Vec<String> {
        let nodes = self.nodes.read();
        let mut to_invalidate = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = vec![key.to_string()];
        
        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            to_invalidate.push(current.clone());
            
            if let Some(node) = nodes.get(&current) {
                for dependent in &node.dependents {
                    if !visited.contains(dependent) {
                        queue.push(dependent.clone());
                    }
                }
            }
        }
        
        // Remove the original key from the result
        to_invalidate.retain(|k| k != key);
        to_invalidate
    }
    
    /// Check for circular dependencies
    pub fn has_cycle(&self, key: &str, depends_on: &[String]) -> bool {
        let nodes = self.nodes.read();
        
        for dep_key in depends_on {
            if dep_key == key {
                return true; // Self-dependency
            }
            
            // Check if dep_key depends on key (directly or transitively)
            let mut visited = HashSet::new();
            let mut queue = vec![dep_key.clone()];
            
            while let Some(current) = queue.pop() {
                if visited.contains(&current) {
                    continue;
                }
                visited.insert(current.clone());
                
                if let Some(node) = nodes.get(&current) {
                    for parent in &node.depends_on {
                        if parent == key {
                            return true; // Cycle detected
                        }
                        if !visited.contains(parent) {
                            queue.push(parent.clone());
                        }
                    }
                }
            }
        }
        
        false
    }
    
    /// Get stats about the dependency graph
    pub fn stats(&self) -> DependencyGraphStats {
        let nodes = self.nodes.read();
        let total_nodes = nodes.len();
        let total_edges = nodes.values().map(|n| n.depends_on.len()).sum();
        let max_depth = self.calculate_max_depth(&nodes);
        
        DependencyGraphStats {
            total_nodes,
            total_edges,
            max_depth,
        }
    }
    
    fn calculate_max_depth(&self, nodes: &HashMap<String, DependencyNode>) -> usize {
        let mut max_depth = 0;
        
        for key in nodes.keys() {
            let depth = self.get_depth(key, nodes, &mut HashSet::new());
            max_depth = max_depth.max(depth);
        }
        
        max_depth
    }
    
    fn get_depth(
        &self,
        key: &str,
        nodes: &HashMap<String, DependencyNode>,
        visited: &mut HashSet<String>,
    ) -> usize {
        if visited.contains(key) {
            return 0; // Cycle protection
        }
        visited.insert(key.to_string());
        
        let mut max_child_depth = 0;
        if let Some(node) = nodes.get(key) {
            for child in &node.dependents {
                let child_depth = self.get_depth(child, nodes, visited);
                max_child_depth = max_child_depth.max(child_depth);
            }
        }
        
        visited.remove(key);
        max_child_depth + 1
    }
}

#[derive(Debug, Clone)]
pub struct DependencyGraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub max_depth: usize,
}
```

---

## 💾 Storage Backends

### Memory Backend (`skp-cache-storage/src/memory/backend.rs`)

```rust
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};

use skp_cache_core::{
    CacheBackend, CacheEntry, CacheError, CacheOptions, CacheStats,
    EvictionStrategy, TaggableBackend,
};

/// Configuration for memory backend
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum number of entries
    pub max_capacity: usize,
    
    /// Maximum memory size in bytes (0 = unlimited)
    pub max_memory: usize,
    
    /// Eviction strategy
    pub eviction_strategy: EvictionStrategyType,
    
    /// Enable bloom filter for negative lookups
    pub bloom_filter: Option<BloomFilterConfig>,
    
    /// Cleanup interval for expired entries
    pub cleanup_interval: Duration,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            max_memory: 0,
            eviction_strategy: EvictionStrategyType::WTinyLFU,
            bloom_filter: None,
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone)]
pub enum EvictionStrategyType {
    LRU,
    LFU,
    WTinyLFU,
    CAMP,
}

/// In-memory cache backend using DashMap
pub struct MemoryBackend {
    /// Main storage
    data: DashMap<String, CacheEntry<Vec<u8>>>,
    
    /// Tag to keys index
    tag_index: DashMap<String, HashSet<String>>,
    
    /// Eviction strategy
    eviction: Arc<dyn EvictionStrategy>,
    
    /// Bloom filter for negative lookups
    bloom_filter: Option<BloomFilter>,
    
    /// Statistics
    stats: Arc<RwLock<MemoryStats>>,
    
    /// Configuration
    config: MemoryConfig,
}

impl MemoryBackend {
    pub fn new(config: MemoryConfig) -> Self {
        let eviction: Arc<dyn EvictionStrategy> = match config.eviction_strategy {
            EvictionStrategyType::LRU => Arc::new(strategies::LRU::new()),
            EvictionStrategyType::LFU => Arc::new(strategies::LFU::new()),
            EvictionStrategyType::WTinyLFU => Arc::new(strategies::WTinyLFU::new(
                config.max_capacity,
                0.01, // 1% window
            )),
            EvictionStrategyType::CAMP => Arc::new(strategies::CAMP::new(10)),
        };
        
        let bloom_filter = config.bloom_filter.as_ref().map(|cfg| {
            BloomFilter::new(cfg.expected_items, cfg.false_positive_rate)
        });
        
        Self {
            data: DashMap::with_capacity(config.max_capacity),
            tag_index: DashMap::new(),
            eviction,
            bloom_filter,
            stats: Arc::new(RwLock::new(MemoryStats::default())),
            config,
        }
    }
    
    /// Check bloom filter before expensive lookup
    fn maybe_exists(&self, key: &str) -> bool {
        match &self.bloom_filter {
            Some(bf) => bf.may_contain(key),
            None => true, // No bloom filter, assume might exist
        }
    }
    
    /// Evict entries if necessary
    async fn maybe_evict(&self, new_entry_size: usize) -> Result<(), CacheError> {
        let current_len = self.data.len();
        
        // Check capacity limit
        if current_len >= self.config.max_capacity {
            let candidates: Vec<_> = self.data.iter()
                .map(|entry| EvictionCandidate {
                    key: entry.key().clone(),
                    size: entry.value().size,
                    cost: entry.value().cost,
                    frequency: entry.value().access_count,
                    last_access: entry.value().last_accessed,
                    created_at: entry.value().created_at,
                    ttl_remaining: entry.value().ttl_remaining(),
                })
                .collect();
            
            let victims = self.eviction.select_victims(&candidates, new_entry_size);
            
            for key in victims {
                self.data.remove(&key);
                self.eviction.record_remove(&key);
                
                let mut stats = self.stats.write();
                stats.evictions += 1;
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl CacheBackend for MemoryBackend {
    async fn get(&self, key: &str) -> Result<Option<CacheEntry<Vec<u8>>>, CacheError> {
        // Check bloom filter first
        if !self.maybe_exists(key) {
            let mut stats = self.stats.write();
            stats.misses += 1;
            return Ok(None);
        }
        
        match self.data.get_mut(key) {
            Some(mut entry) => {
                // Check expiration
                if entry.is_expired() && !entry.is_stale() {
                    drop(entry);
                    self.data.remove(key);
                    
                    let mut stats = self.stats.write();
                    stats.misses += 1;
                    return Ok(None);
                }
                
                // Update access stats
                entry.last_accessed = SystemTime::now();
                entry.access_count += 1;
                self.eviction.record_access(key);
                
                let mut stats = self.stats.write();
                if entry.is_stale() {
                    stats.stale_hits += 1;
                } else {
                    stats.hits += 1;
                }
                
                Ok(Some(entry.clone()))
            }
            None => {
                let mut stats = self.stats.write();
                stats.misses += 1;
                Ok(None)
            }
        }
    }
    
    async fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        options: &CacheOptions,
    ) -> Result<(), CacheError> {
        let size = value.len();
        
        // Evict if necessary
        self.maybe_evict(size).await?;
        
        let entry = CacheEntry {
            value,
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            access_count: 0,
            ttl: options.ttl,
            stale_while_revalidate: options.stale_while_revalidate,
            tags: options.tags.clone(),
            dependencies: options.dependencies.clone(),
            cost: options.cost.unwrap_or(1),
            size,
            etag: options.etag.clone(),
            version: 0,
        };
        
        // Update tag index
        for tag in &options.tags {
            self.tag_index
                .entry(tag.clone())
                .or_insert_with(HashSet::new)
                .insert(key.to_string());
        }
        
        // Update bloom filter
        if let Some(ref bf) = self.bloom_filter {
            bf.insert(key);
        }
        
        // Insert entry
        self.data.insert(key.to_string(), entry);
        self.eviction.record_insert(key, size, options.cost.unwrap_or(1));
        
        let mut stats = self.stats.write();
        stats.writes += 1;
        
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        if let Some((_, entry)) = self.data.remove(key) {
            // Remove from tag index
            for tag in &entry.tags {
                if let Some(mut keys) = self.tag_index.get_mut(tag) {
                    keys.remove(key);
                }
            }
            
            self.eviction.record_remove(key);
            
            let mut stats = self.stats.write();
            stats.deletes += 1;
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        if !self.maybe_exists(key) {
            return Ok(false);
        }
        
        match self.data.get(key) {
            Some(entry) => Ok(!entry.is_expired() || entry.is_stale()),
            None => Ok(false),
        }
    }
    
    async fn clear(&self) -> Result<(), CacheError> {
        self.data.clear();
        self.tag_index.clear();
        self.eviction.reset();
        
        if let Some(ref bf) = self.bloom_filter {
            bf.clear();
        }
        
        Ok(())
    }
    
    async fn stats(&self) -> Result<CacheStats, CacheError> {
        let stats = self.stats.read();
        Ok(CacheStats {
            hits: stats.hits,
            misses: stats.misses,
            stale_hits: stats.stale_hits,
            writes: stats.writes,
            deletes: stats.deletes,
            evictions: stats.evictions,
            size: self.data.len(),
            memory_bytes: 0, // TODO: track actual memory
        })
    }
    
    async fn len(&self) -> Result<usize, CacheError> {
        Ok(self.data.len())
    }
}

#[async_trait]
impl TaggableBackend for MemoryBackend {
    async fn get_keys_by_tag(&self, tag: &str) -> Result<Vec<String>, CacheError> {
        Ok(self.tag_index
            .get(tag)
            .map(|keys| keys.iter().cloned().collect())
            .unwrap_or_default())
    }
    
    async fn invalidate_by_tag(&self, tag: &str) -> Result<u64, CacheError> {
        let keys = self.get_keys_by_tag(tag).await?;
        let count = keys.len() as u64;
        
        for key in keys {
            self.delete(&key).await?;
        }
        
        self.tag_index.remove(tag);
        
        Ok(count)
    }
    
    async fn invalidate_by_pattern(&self, pattern: &str) -> Result<u64, CacheError> {
        let matcher = WildcardMatcher::new(pattern);
        let mut count = 0u64;
        
        let matching_tags: Vec<String> = self.tag_index
            .iter()
            .filter(|entry| matcher.matches(entry.key()))
            .map(|entry| entry.key().clone())
            .collect();
        
        for tag in matching_tags {
            count += self.invalidate_by_tag(&tag).await?;
        }
        
        Ok(count)
    }
    
    async fn register_tags(&self, key: &str, tags: &[&str]) -> Result<(), CacheError> {
        for tag in tags {
            self.tag_index
                .entry(tag.to_string())
                .or_insert_with(HashSet::new)
                .insert(key.to_string());
        }
        Ok(())
    }
    
    async fn unregister_tags(&self, key: &str) -> Result<(), CacheError> {
        if let Some(entry) = self.data.get(key) {
            for tag in &entry.tags {
                if let Some(mut keys) = self.tag_index.get_mut(tag) {
                    keys.remove(key);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct MemoryStats {
    hits: u64,
    misses: u64,
    stale_hits: u64,
    writes: u64,
    deletes: u64,
    evictions: u64,
}
```

### Redis Backend (`skp-cache-storage/src/redis/backend.rs`)

```rust
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;
use std::sync::Arc;

use skp_cache_core::{
    CacheBackend, CacheEntry, CacheError, CacheOptions, CacheStats,
    TaggableBackend, DistributedBackend,
};

/// Configuration for Redis backend
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,
    
    /// Connection pool size
    pub pool_size: u32,
    
    /// Key prefix/namespace
    pub key_prefix: Option<String>,
    
    /// Enable pub/sub for distributed invalidation
    pub enable_pubsub: bool,
    
    /// Channel name for invalidation events
    pub invalidation_channel: String,
    
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Command timeout
    pub command_timeout: Duration,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_size: 10,
            key_prefix: None,
            enable_pubsub: true,
            invalidation_channel: "skp_cache:invalidation".to_string(),
            connection_timeout: Duration::from_secs(5),
            command_timeout: Duration::from_secs(2),
        }
    }
}

/// Redis cache backend using bb8 connection pool
pub struct RedisBackend {
    pool: Pool<RedisConnectionManager>,
    config: RedisConfig,
    stats: Arc<RwLock<RedisStats>>,
    pubsub_handle: Option<JoinHandle<()>>,
}

impl RedisBackend {
    pub async fn new(config: RedisConfig) -> Result<Self, CacheError> {
        let manager = RedisConnectionManager::new(config.url.clone())
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        let pool = Pool::builder()
            .max_size(config.pool_size)
            .connection_timeout(config.connection_timeout)
            .build(manager)
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        Ok(Self {
            pool,
            config,
            stats: Arc::new(RwLock::new(RedisStats::default())),
            pubsub_handle: None,
        })
    }
    
    fn prefixed_key(&self, key: &str) -> String {
        match &self.config.key_prefix {
            Some(prefix) => format!("{}:{}", prefix, key),
            None => key.to_string(),
        }
    }
    
    fn tag_key(&self, tag: &str) -> String {
        format!("{}:__tags__:{}", 
            self.config.key_prefix.as_deref().unwrap_or("skp"),
            tag
        )
    }
}

#[async_trait]
impl CacheBackend for RedisBackend {
    async fn get(&self, key: &str) -> Result<Option<CacheEntry<Vec<u8>>>, CacheError> {
        let mut conn = self.pool.get().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        let prefixed = self.prefixed_key(key);
        
        let data: Option<Vec<u8>> = conn.get(&prefixed).await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
        
        match data {
            Some(bytes) => {
                let entry: CacheEntry<Vec<u8>> = serde_json::from_slice(&bytes)
                    .map_err(|e| CacheError::Serialization(e.to_string()))?;
                
                let mut stats = self.stats.write();
                stats.hits += 1;
                
                Ok(Some(entry))
            }
            None => {
                let mut stats = self.stats.write();
                stats.misses += 1;
                Ok(None)
            }
        }
    }
    
    async fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        options: &CacheOptions,
    ) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        let entry = CacheEntry {
            value,
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            access_count: 0,
            ttl: options.ttl,
            stale_while_revalidate: options.stale_while_revalidate,
            tags: options.tags.clone(),
            dependencies: options.dependencies.clone(),
            cost: options.cost.unwrap_or(1),
            size: 0,
            etag: options.etag.clone(),
            version: 0,
        };
        
        let serialized = serde_json::to_vec(&entry)
            .map_err(|e| CacheError::Serialization(e.to_string()))?;
        
        let prefixed = self.prefixed_key(key);
        
        // Use pipeline for atomicity
        let mut pipe = redis::pipe();
        pipe.atomic();
        
        // Set the value with optional TTL
        if let Some(ttl) = options.ttl {
            // Add stale_while_revalidate to TTL for Redis
            let total_ttl = ttl + options.stale_while_revalidate.unwrap_or_default();
            pipe.set_ex(&prefixed, &serialized, total_ttl.as_secs());
        } else {
            pipe.set(&prefixed, &serialized);
        }
        
        // Register tags
        for tag in &options.tags {
            let tag_key = self.tag_key(tag);
            pipe.sadd(&tag_key, key);
            
            // Set TTL on tag set if entry has TTL
            if let Some(ttl) = options.ttl {
                let total_ttl = ttl + options.stale_while_revalidate.unwrap_or_default();
                pipe.expire(&tag_key, total_ttl.as_secs() as i64);
            }
        }
        
        pipe.query_async(&mut *conn).await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
        
        let mut stats = self.stats.write();
        stats.writes += 1;
        
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        let mut conn = self.pool.get().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        let prefixed = self.prefixed_key(key);
        
        let deleted: bool = conn.del(&prefixed).await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
        
        if deleted {
            let mut stats = self.stats.write();
            stats.deletes += 1;
        }
        
        Ok(deleted)
    }
    
    async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        let mut conn = self.pool.get().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        let prefixed = self.prefixed_key(key);
        
        conn.exists(&prefixed).await
            .map_err(|e| CacheError::Backend(e.to_string()))
    }
    
    async fn clear(&self) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        // Only clear keys with our prefix using SCAN (non-blocking)
        if let Some(prefix) = &self.config.key_prefix {
            let pattern = format!("{}:*", prefix);
            let mut cursor = 0u64;
            
            loop {
                let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(&pattern)
                    .arg("COUNT")
                    .arg(100)
                    .query_async(&mut *conn)
                    .await
                    .map_err(|e| CacheError::Backend(e.to_string()))?;
                
                if !keys.is_empty() {
                    // Use UNLINK for async deletion (non-blocking)
                    redis::cmd("UNLINK")
                        .arg(&keys)
                        .query_async::<_, ()>(&mut *conn)
                        .await
                        .map_err(|e| CacheError::Backend(e.to_string()))?;
                }
                
                cursor = next_cursor;
                if cursor == 0 {
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    async fn stats(&self) -> Result<CacheStats, CacheError> {
        let stats = self.stats.read();
        Ok(CacheStats {
            hits: stats.hits,
            misses: stats.misses,
            stale_hits: 0,
            writes: stats.writes,
            deletes: stats.deletes,
            evictions: 0, // Redis handles eviction internally
            size: 0,
            memory_bytes: 0,
        })
    }
    
    async fn len(&self) -> Result<usize, CacheError> {
        let mut conn = self.pool.get().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        if let Some(prefix) = &self.config.key_prefix {
            // Use SCAN to count keys (non-blocking, O(N) but distributed)
            let pattern = format!("{}:*", prefix);
            let mut cursor = 0u64;
            let mut count = 0usize;
            
            loop {
                let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(&pattern)
                    .arg("COUNT")
                    .arg(1000)
                    .query_async(&mut *conn)
                    .await
                    .map_err(|e| CacheError::Backend(e.to_string()))?;
                
                count += keys.len();
                cursor = next_cursor;
                if cursor == 0 {
                    break;
                }
            }
            
            Ok(count)
        } else {
            // No prefix - use DBSIZE (O(1) but counts all keys in DB)
            let size: i64 = redis::cmd("DBSIZE")
                .query_async(&mut *conn)
                .await
                .map_err(|e| CacheError::Backend(e.to_string()))?;
            Ok(size as usize)
        }
    }
}

#[async_trait]
impl TaggableBackend for RedisBackend {
    async fn get_keys_by_tag(&self, tag: &str) -> Result<Vec<String>, CacheError> {
        let mut conn = self.pool.get().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        let tag_key = self.tag_key(tag);
        
        conn.smembers(&tag_key).await
            .map_err(|e| CacheError::Backend(e.to_string()))
    }
    
    async fn invalidate_by_tag(&self, tag: &str) -> Result<u64, CacheError> {
        let keys = self.get_keys_by_tag(tag).await?;
        let count = keys.len() as u64;
        
        if keys.is_empty() {
            return Ok(0);
        }
        
        let mut conn = self.pool.get().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        let prefixed_keys: Vec<String> = keys.iter()
            .map(|k| self.prefixed_key(k))
            .collect();
        
        // Delete all keys atomically
        let mut pipe = redis::pipe();
        pipe.atomic();
        
        for key in &prefixed_keys {
            pipe.del(key);
        }
        
        // Delete the tag set
        pipe.del(self.tag_key(tag));
        
        pipe.query_async(&mut *conn).await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
        
        // Publish invalidation event if pub/sub is enabled
        if self.config.enable_pubsub {
            let event = InvalidationEvent::Tag(tag.to_string());
            let _ = self.publish_invalidation(event).await;
        }
        
        Ok(count)
    }
    
    async fn invalidate_by_pattern(&self, pattern: &str) -> Result<u64, CacheError> {
        let mut conn = self.pool.get().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        // Convert wildcard pattern to Redis SCAN pattern
        let tag_pattern = self.tag_key(pattern);
        
        // Use SCAN to find matching tag keys
        let mut cursor = 0u64;
        let mut total_count = 0u64;
        
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&tag_pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut *conn)
                .await
                .map_err(|e| CacheError::Backend(e.to_string()))?;
            
            for tag_key in keys {
                // Extract tag name from key
                if let Some(tag) = tag_key.strip_prefix(&format!(
                    "{}:__tags__:",
                    self.config.key_prefix.as_deref().unwrap_or("skp")
                )) {
                    total_count += self.invalidate_by_tag(tag).await?;
                }
            }
            
            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }
        
        Ok(total_count)
    }
    
    async fn register_tags(&self, key: &str, tags: &[&str]) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        let mut pipe = redis::pipe();
        
        for tag in tags {
            let tag_key = self.tag_key(tag);
            pipe.sadd(&tag_key, key);
        }
        
        pipe.query_async(&mut *conn).await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
        
        Ok(())
    }
    
    async fn unregister_tags(&self, key: &str) -> Result<(), CacheError> {
        // This requires knowing the tags for the key
        // In a full implementation, we'd store this mapping
        Ok(())
    }
}

#[async_trait]
impl DistributedBackend for RedisBackend {
    async fn subscribe_invalidations(&self) -> Result<InvalidationSubscriber, CacheError> {
        let client = redis::Client::open(self.config.url.clone())
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        let mut pubsub = client.get_async_pubsub().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        pubsub.subscribe(&self.config.invalidation_channel).await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
        
        Ok(InvalidationSubscriber { pubsub })
    }
    
    async fn publish_invalidation(&self, event: InvalidationEvent) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        let payload = serde_json::to_string(&event)
            .map_err(|e| CacheError::Serialization(e.to_string()))?;
        
        conn.publish(&self.config.invalidation_channel, &payload).await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
        
        Ok(())
    }
    
    async fn acquire_lock(&self, key: &str, ttl: Duration) -> Result<LockGuard, CacheError> {
        let mut conn = self.pool.get().await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        
        let lock_key = format!("__lock__:{}", key);
        let lock_value = uuid::Uuid::new_v4().to_string();
        
        // SET NX EX
        let acquired: bool = redis::cmd("SET")
            .arg(&lock_key)
            .arg(&lock_value)
            .arg("NX")
            .arg("PX")
            .arg(ttl.as_millis() as u64)
            .query_async(&mut *conn)
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
        
        if acquired {
            Ok(LockGuard {
                key: lock_key,
                value: lock_value,
                pool: self.pool.clone(),
            })
        } else {
            Err(CacheError::LockConflict(key.to_string()))
        }
    }
}

#[derive(Debug, Default)]
struct RedisStats {
    hits: u64,
    misses: u64,
    writes: u64,
    deletes: u64,
}
```

### Multi-Tier Backend (`skp-cache-storage/src/multi_tier/backend.rs`)

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

use skp_cache_core::{
    CacheBackend, CacheEntry, CacheError, CacheOptions, CacheResult,
    CacheStats, TaggableBackend,
};

/// Configuration for multi-tier cache
#[derive(Debug, Clone)]
pub struct MultiTierConfig {
    /// L1 (memory) TTL - typically shorter than L2
    pub l1_ttl: Option<Duration>,
    
    /// Promote L2 hits to L1
    pub promote_on_hit: bool,
    
    /// Write strategy
    pub write_strategy: WriteStrategy,
    
    /// L1 max size (entries)
    pub l1_max_size: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum WriteStrategy {
    /// Write to both L1 and L2 synchronously
    WriteThrough,
    /// Write to L1 immediately, L2 asynchronously
    WriteBehind,
    /// Write to L2 only, populate L1 on read
    WriteAround,
}

impl Default for MultiTierConfig {
    fn default() -> Self {
        Self {
            l1_ttl: Some(Duration::from_secs(60)),
            promote_on_hit: true,
            write_strategy: WriteStrategy::WriteThrough,
            l1_max_size: 10_000,
        }
    }
}

/// Multi-tier cache combining L1 (memory) and L2 (Redis)
pub struct MultiTierBackend<L1, L2>
where
    L1: CacheBackend,
    L2: CacheBackend,
{
    l1: Arc<L1>,
    l2: Arc<L2>,
    config: MultiTierConfig,
    stats: Arc<RwLock<MultiTierStats>>,
}

impl<L1, L2> MultiTierBackend<L1, L2>
where
    L1: CacheBackend,
    L2: CacheBackend,
{
    pub fn new(l1: L1, l2: L2, config: MultiTierConfig) -> Self {
        Self {
            l1: Arc::new(l1),
            l2: Arc::new(l2),
            config,
            stats: Arc::new(RwLock::new(MultiTierStats::default())),
        }
    }
    
    /// Promote an entry from L2 to L1
    async fn promote_to_l1(&self, key: &str, entry: &CacheEntry<Vec<u8>>) -> Result<(), CacheError> {
        let mut options = CacheOptions::default();
        
        // Use L1-specific TTL or remaining TTL
        options.ttl = self.config.l1_ttl.or(entry.ttl_remaining());
        options.tags = entry.tags.clone();
        
        self.l1.set(key, entry.value.clone(), &options).await
    }
}

#[async_trait]
impl<L1, L2> CacheBackend for MultiTierBackend<L1, L2>
where
    L1: CacheBackend,
    L2: CacheBackend,
{
    async fn get(&self, key: &str) -> Result<Option<CacheEntry<Vec<u8>>>, CacheError> {
        // Check L1 first (fast path)
        if let Some(entry) = self.l1.get(key).await? {
            let mut stats = self.stats.write().await;
            stats.l1_hits += 1;
            return Ok(Some(entry));
        }
        
        let mut stats = self.stats.write().await;
        stats.l1_misses += 1;
        drop(stats);
        
        // Check L2 (slow path)
        if let Some(entry) = self.l2.get(key).await? {
            let mut stats = self.stats.write().await;
            stats.l2_hits += 1;
            drop(stats);
            
            // Promote to L1
            if self.config.promote_on_hit {
                if let Err(e) = self.promote_to_l1(key, &entry).await {
                    tracing::warn!("Failed to promote to L1: {}", e);
                }
            }
            
            return Ok(Some(entry));
        }
        
        let mut stats = self.stats.write().await;
        stats.l2_misses += 1;
        
        Ok(None)
    }
    
    async fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        options: &CacheOptions,
    ) -> Result<(), CacheError> {
        match self.config.write_strategy {
            WriteStrategy::WriteThrough => {
                // Write to both L1 and L2
                let l1_options = CacheOptions {
                    ttl: self.config.l1_ttl.or(options.ttl),
                    ..options.clone()
                };
                
                // L1 write
                self.l1.set(key, value.clone(), &l1_options).await?;
                
                // L2 write
                self.l2.set(key, value, options).await?;
            }
            WriteStrategy::WriteBehind => {
                // Write to L1 immediately
                let l1_options = CacheOptions {
                    ttl: self.config.l1_ttl.or(options.ttl),
                    ..options.clone()
                };
                self.l1.set(key, value.clone(), &l1_options).await?;
                
                // Write to L2 asynchronously
                let l2 = self.l2.clone();
                let key = key.to_string();
                let options = options.clone();
                tokio::spawn(async move {
                    if let Err(e) = l2.set(&key, value, &options).await {
                        tracing::error!("Failed to write to L2: {}", e);
                    }
                });
            }
            WriteStrategy::WriteAround => {
                // Write to L2 only
                self.l2.set(key, value, options).await?;
            }
        }
        
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        let l1_deleted = self.l1.delete(key).await?;
        let l2_deleted = self.l2.delete(key).await?;
        Ok(l1_deleted || l2_deleted)
    }
    
    async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        if self.l1.exists(key).await? {
            return Ok(true);
        }
        self.l2.exists(key).await
    }
    
    async fn clear(&self) -> Result<(), CacheError> {
        self.l1.clear().await?;
        self.l2.clear().await?;
        Ok(())
    }
    
    async fn stats(&self) -> Result<CacheStats, CacheError> {
        let l1_stats = self.l1.stats().await?;
        let l2_stats = self.l2.stats().await?;
        let stats = self.stats.read().await;
        
        Ok(CacheStats {
            hits: stats.l1_hits + stats.l2_hits,
            misses: stats.l2_misses,
            stale_hits: l1_stats.stale_hits + l2_stats.stale_hits,
            writes: l1_stats.writes.max(l2_stats.writes),
            deletes: l1_stats.deletes.max(l2_stats.deletes),
            evictions: l1_stats.evictions,
            size: l1_stats.size,
            memory_bytes: l1_stats.memory_bytes,
        })
    }
    
    async fn len(&self) -> Result<usize, CacheError> {
        // L2 is source of truth
        self.l2.len().await
    }
}

#[derive(Debug, Default)]
struct MultiTierStats {
    l1_hits: u64,
    l1_misses: u64,
    l2_hits: u64,
    l2_misses: u64,
}
```

---

## 🔄 Cache Manager (High-Level API)

### `skp-cache/src/manager.rs`

```rust
use std::sync::Arc;
use std::future::Future;
use std::time::Instant;
use parking_lot::Mutex;
use dashmap::DashMap;
use tokio::sync::broadcast;

use skp_cache_core::{
    CacheBackend, CacheEntry, CacheError, CacheKey, CacheOptions, CacheOpts,
    CacheResult, DependencyGraph, TaggableBackend,
    Serializer, JsonSerializer, CacheMetrics, NoopMetrics, CacheOperation, CacheTier,
};

/// High-level cache manager with advanced features
/// 
/// Generic over:
/// - `B`: The cache backend (Memory, Redis, MultiTier)
/// - `S`: The serializer (JSON, MessagePack, Bincode)
/// - `M`: The metrics collector
pub struct CacheManager<B: CacheBackend, S: Serializer = JsonSerializer, M: CacheMetrics = NoopMetrics> {
    backend: Arc<B>,
    serializer: Arc<S>,
    metrics: Arc<M>,
    dependency_graph: Arc<DependencyGraph>,
    coalescer: Arc<Coalescer>,
    config: CacheManagerConfig,
}

/// Configuration for CacheManager
#[derive(Debug, Clone)]
pub struct CacheManagerConfig {
    /// Default TTL for entries without explicit TTL
    pub default_ttl: Option<Duration>,
    
    /// Enable request coalescing globally
    pub enable_coalescing: bool,
    
    /// Enable dependency tracking
    pub enable_dependencies: bool,
    
    /// Enable early probabilistic refresh
    pub enable_early_refresh: bool,
    
    /// Beta parameter for early refresh (higher = earlier refresh)
    pub early_refresh_beta: f64,
    
    /// Namespace prefix for all keys
    pub namespace: Option<String>,
    
    /// TTL jitter percentage (0.0 - 1.0) to prevent thundering herd
    pub ttl_jitter: f64,
}

impl Default for CacheManagerConfig {
    fn default() -> Self {
        Self {
            default_ttl: Some(Duration::from_secs(300)),
            enable_coalescing: true,
            enable_dependencies: true,
            enable_early_refresh: true,
            early_refresh_beta: 1.0,
            namespace: None,
            ttl_jitter: 0.1, // 10% jitter by default
        }
    }
}

impl<B: CacheBackend> CacheManager<B, JsonSerializer, NoopMetrics> {
    /// Create a new CacheManager with default JSON serializer and no metrics
    pub fn new(backend: B) -> Self {
        Self::with_config(backend, CacheManagerConfig::default())
    }
    
    pub fn with_config(backend: B, config: CacheManagerConfig) -> Self {
        Self {
            backend: Arc::new(backend),
            serializer: Arc::new(JsonSerializer),
            metrics: Arc::new(NoopMetrics),
            dependency_graph: Arc::new(DependencyGraph::new()),
            coalescer: Arc::new(Coalescer::new()),
            config,
        }
    }
}

impl<B: CacheBackend, S: Serializer, M: CacheMetrics> CacheManager<B, S, M> {
    /// Create a CacheManager with custom serializer and metrics
    pub fn with_serializer_and_metrics(
        backend: B,
        serializer: S,
        metrics: M,
        config: CacheManagerConfig,
    ) -> Self {
        Self {
            backend: Arc::new(backend),
            serializer: Arc::new(serializer),
            metrics: Arc::new(metrics),
            dependency_graph: Arc::new(DependencyGraph::new()),
            coalescer: Arc::new(Coalescer::new()),
            config,
        }
    }
    
    /// Apply TTL jitter to prevent thundering herd on expiry
    fn apply_ttl_jitter(&self, ttl: Duration) -> Duration {
        if self.config.ttl_jitter > 0.0 {
            let jitter_range = (ttl.as_secs_f64() * self.config.ttl_jitter) as u64;
            if jitter_range > 0 {
                let jitter = rand::random::<u64>() % jitter_range;
                return ttl + Duration::from_secs(jitter);
            }
        }
        ttl
    }
    
    fn full_key(&self, key: &str) -> String {
        match &self.config.namespace {
            Some(ns) => format!("{}:{}", ns, key),
            None => key.to_string(),
        }
    }
    
    /// Get a value from cache
    pub async fn get<T>(&self, key: impl CacheKey) -> Result<CacheResult<T>, CacheError>
    where
        T: serde::de::DeserializeOwned,
    {
        let full_key = self.full_key(&key.full_key());
        let start = Instant::now();
        
        let result = match self.backend.get(&full_key).await? {
            Some(entry) => {
                // Check for early refresh
                if self.config.enable_early_refresh && self.should_refresh_early(&entry) {
                    self.metrics.record_stale_hit(&full_key);
                    // Return stale and trigger background refresh
                    return Ok(CacheResult::Stale(self.deserialize_entry(entry)?));
                }
                
                if entry.is_expired() {
                    if entry.is_stale() {
                        self.metrics.record_stale_hit(&full_key);
                        Ok(CacheResult::Stale(self.deserialize_entry(entry)?))
                    } else {
                        self.metrics.record_miss(&full_key);
                        Ok(CacheResult::Miss)
                    }
                } else {
                    self.metrics.record_hit(&full_key, CacheTier::L1Memory);
                    Ok(CacheResult::Hit(self.deserialize_entry(entry)?))
                }
            }
            None => {
                self.metrics.record_miss(&full_key);
                Ok(CacheResult::Miss)
            }
        };
        
        self.metrics.record_latency(CacheOperation::Get, start.elapsed());
        result
    }
    
    /// Set a value in cache
    pub async fn set<T>(
        &self,
        key: impl CacheKey,
        value: T,
        options: impl Into<CacheOptions>,
    ) -> Result<(), CacheError>
    where
        T: serde::Serialize,
    {
        let full_key = self.full_key(&key.full_key());
        let mut options = options.into();
        
        // Apply default TTL if not specified
        if options.ttl.is_none() {
            options.ttl = self.config.default_ttl;
        }
        
        // Apply TTL jitter to prevent thundering herd
        if let Some(ttl) = options.ttl {
            options.ttl = Some(self.apply_ttl_jitter(ttl));
        }
        
        // Register dependencies
        if self.config.enable_dependencies && !options.dependencies.is_empty() {
            // Check for cycles
            if self.dependency_graph.has_cycle(&full_key, &options.dependencies) {
                return Err(CacheError::CyclicDependency(full_key));
            }
            self.dependency_graph.register(&full_key, &options.dependencies);
        }
        
        // Serialize using configured serializer
        let serialize_start = Instant::now();
        let serialized = self.serializer.serialize(&value)?;
        self.metrics.record_latency(CacheOperation::Serialize, serialize_start.elapsed());
        
        let set_start = Instant::now();
        self.backend.set(&full_key, serialized, &options).await?;
        self.metrics.record_latency(CacheOperation::Set, set_start.elapsed());
        
        Ok(())
    }
    
    /// Get or compute a value (cache-aside pattern)
    pub async fn get_or_compute<T, F, Fut>(
        &self,
        key: impl CacheKey,
        compute: F,
    ) -> Result<T, CacheError>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + 'static,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, CacheError>> + Send,
    {
        self.get_or_compute_with(key, CacheOpts::new(), compute).await
    }
    
    /// Get or compute with options
    pub async fn get_or_compute_with<T, F, Fut>(
        &self,
        key: impl CacheKey,
        options: CacheOpts,
        compute: F,
    ) -> Result<T, CacheError>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + 'static,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, CacheError>> + Send,
    {
        let full_key = self.full_key(&key.full_key());
        let options = options.build();
        
        // Check cache first
        match self.get::<T>(&full_key).await? {
            CacheResult::Hit(entry) => return Ok(entry.value),
            CacheResult::Stale(entry) => {
                // Trigger background refresh
                let manager = self.clone();
                let key = full_key.clone();
                let opts = options.clone();
                tokio::spawn(async move {
                    // Note: compute closure can't be used here easily
                    // In practice, you'd use a refresh callback
                });
                return Ok(entry.value);
            }
            _ => {}
        }
        
        // Use coalescing if enabled
        if self.config.enable_coalescing && options.coalesce {
            return self.coalescer.coalesce(&full_key, || async {
                let value = compute().await?;
                self.set(&full_key, &value, &options).await?;
                Ok(value)
            }).await;
        }
        
        // Compute and cache
        let value = compute().await?;
        self.set(&full_key, &value, &options).await?;
        Ok(value)
    }
    
    /// Delete a key and its dependents (cascade)
    pub async fn invalidate(&self, key: impl CacheKey) -> Result<u64, CacheError> {
        let full_key = self.full_key(&key.full_key());
        let mut count = 0u64;
        
        // Get cascade invalidation list
        if self.config.enable_dependencies {
            let dependents = self.dependency_graph.get_cascade_invalidation(&full_key);
            
            for dep_key in dependents {
                if self.backend.delete(&dep_key).await? {
                    count += 1;
                }
                self.dependency_graph.remove(&dep_key);
            }
        }
        
        // Delete the key itself
        if self.backend.delete(&full_key).await? {
            count += 1;
        }
        self.dependency_graph.remove(&full_key);
        
        Ok(count)
    }
    
    /// Invalidate by tag (if backend supports it)
    pub async fn invalidate_by_tag(&self, tag: &str) -> Result<u64, CacheError>
    where
        B: TaggableBackend,
    {
        self.backend.invalidate_by_tag(tag).await
    }
    
    /// Invalidate by tag pattern (supports wildcards)
    pub async fn invalidate_by_pattern(&self, pattern: &str) -> Result<u64, CacheError>
    where
        B: TaggableBackend,
    {
        self.backend.invalidate_by_pattern(pattern).await
    }
    
    /// Check if early refresh should be triggered
    fn should_refresh_early(&self, entry: &CacheEntry<Vec<u8>>) -> bool {
        if !self.config.enable_early_refresh {
            return false;
        }
        
        if let (Some(ttl), Some(remaining)) = (entry.ttl, entry.ttl_remaining()) {
            let beta = self.config.early_refresh_beta;
            let random: f64 = rand::random();
            
            // X-Fetch formula: refresh if remaining < TTL * random * beta * ln(random)
            let threshold = ttl.as_secs_f64() * random * beta * random.ln().abs();
            return remaining.as_secs_f64() < threshold;
        }
        
        false
    }
    
    fn deserialize_entry<T>(&self, entry: CacheEntry<Vec<u8>>) -> Result<CacheEntry<T>, CacheError>
    where
        T: serde::de::DeserializeOwned,
    {
        let deserialize_start = Instant::now();
        let value: T = self.serializer.deserialize(&entry.value)?;
        self.metrics.record_latency(CacheOperation::Deserialize, deserialize_start.elapsed());
        
        Ok(CacheEntry {
            value,
            created_at: entry.created_at,
            last_accessed: entry.last_accessed,
            access_count: entry.access_count,
            ttl: entry.ttl,
            stale_while_revalidate: entry.stale_while_revalidate,
            tags: entry.tags,
            dependencies: entry.dependencies,
            cost: entry.cost,
            size: entry.size,
            etag: entry.etag,
            version: entry.version,
        })
    }
    
    /// Graceful shutdown - flush pending operations
    pub async fn shutdown(&self) {
        // Flush any pending write-behind operations
        // Clear coalescer state
        // Close connections gracefully
        tracing::info!("CacheManager shutting down gracefully");
    }
}

impl<B: CacheBackend, S: Serializer, M: CacheMetrics> Clone for CacheManager<B, S, M> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            serializer: self.serializer.clone(),
            metrics: self.metrics.clone(),
            dependency_graph: self.dependency_graph.clone(),
            coalescer: self.coalescer.clone(),
            config: self.config.clone(),
        }
    }
}
```

### Request Coalescer (Singleflight) - Fixed Implementation

```rust
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use parking_lot::Mutex;
use tokio::sync::{broadcast, oneshot};

/// Request coalescer to prevent cache stampede
/// 
/// This implementation correctly avoids holding a sync lock during async execution.
/// When multiple requests arrive for the same key simultaneously:
/// - First request executes the computation
/// - Subsequent requests wait for the first one's result
/// - All requests receive the same computed value
pub struct Coalescer {
    inflight: Mutex<HashMap<String, Arc<CoalesceState>>>,
}

struct CoalesceState {
    /// Notifier when the computation is complete
    notifier: broadcast::Sender<()>,
    /// The serialized result (set by the leader request)
    result: Mutex<Option<Result<Vec<u8>, CacheError>>>,
}

/// Whether this request is the leader (first) or a waiter
enum CoalesceSlot {
    /// This request should execute the computation
    Leader(Arc<CoalesceState>),
    /// This request should wait for the leader
    Waiter(Arc<CoalesceState>),
}

impl Coalescer {
    pub fn new() -> Self {
        Self {
            inflight: Mutex::new(HashMap::new()),
        }
    }
    
    /// Execute a function, coalescing concurrent calls for the same key
    /// 
    /// Only the first concurrent request executes the function.
    /// All other concurrent requests wait and receive the same result.
    pub async fn coalesce<T, F, Fut>(
        &self,
        key: &str,
        f: F,
    ) -> Result<T, CacheError>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, CacheError>> + Send,
    {
        // Phase 1: Atomically check/register in the inflight map (sync, no await)
        let slot = {
            let mut inflight = self.inflight.lock();
            
            if let Some(state) = inflight.get(key) {
                // Someone else is already computing - we're a waiter
                CoalesceSlot::Waiter(state.clone())
            } else {
                // We're the first - create state and register as leader
                let (notifier, _) = broadcast::channel(16);
                let state = Arc::new(CoalesceState {
                    notifier,
                    result: Mutex::new(None),
                });
                inflight.insert(key.to_string(), state.clone());
                CoalesceSlot::Leader(state)
            }
        };
        // Lock is released here BEFORE any async work
        
        match slot {
            CoalesceSlot::Leader(state) => {
                // Execute the computation (async, no lock held)
                let result = f().await;
                
                // Serialize the result for waiters
                let serialized = match &result {
                    Ok(v) => serde_json::to_vec(v)
                        .map_err(|e| CacheError::Serialization(e.to_string())),
                    Err(e) => Err(e.clone()),
                };
                
                // Store result for waiters
                *state.result.lock() = Some(serialized);
                
                // Notify all waiters
                let _ = state.notifier.send(());
                
                // Cleanup: remove from inflight map
                {
                    let mut inflight = self.inflight.lock();
                    inflight.remove(key);
                }
                
                result
            }
            CoalesceSlot::Waiter(state) => {
                // Subscribe and wait for the leader to complete (async, no lock held)
                let mut receiver = state.notifier.subscribe();
                let _ = receiver.recv().await;
                
                // Get the cached result
                let serialized = state.result.lock().clone()
                    .ok_or_else(|| CacheError::Internal("Coalesce result missing".to_string()))?;
                
                match serialized {
                    Ok(bytes) => {
                        let value: T = serde_json::from_slice(&bytes)
                            .map_err(|e| CacheError::Serialization(e.to_string()))?;
                        Ok(value)
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }
    
    /// Get current number of inflight requests (for metrics/debugging)
    pub fn inflight_count(&self) -> usize {
        self.inflight.lock().len()
    }
}
```

---

## 🌐 HTTP Response Caching

### `skp-cache-http/src/response.rs`

```rust
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};

/// A cached HTTP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    /// HTTP status code
    pub status: u16,
    
    /// Response headers (filtered)
    pub headers: Vec<(String, String)>,
    
    /// Response body
    pub body: Vec<u8>,
    
    /// ETag for conditional requests
    pub etag: Option<String>,
    
    /// Last-Modified header
    pub last_modified: Option<String>,
    
    /// Vary headers for cache key differentiation
    pub vary: Vec<String>,
}

impl CachedResponse {
    pub fn from_response(
        status: StatusCode,
        headers: &HeaderMap,
        body: Vec<u8>,
    ) -> Self {
        let cacheable_headers: Vec<(String, String)> = headers
            .iter()
            .filter(|(name, _)| {
                let name = name.as_str().to_lowercase();
                // Keep only safe headers
                matches!(name.as_str(),
                    "content-type" | "content-language" | "content-encoding" |
                    "cache-control" | "etag" | "last-modified" | "vary" |
                    "x-cache" | "x-cache-hits"
                )
            })
            .map(|(name, value)| {
                (name.to_string(), value.to_str().unwrap_or("").to_string())
            })
            .collect();
        
        let etag = headers.get("etag")
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        
        let last_modified = headers.get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        
        let vary = headers.get("vary")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();
        
        Self {
            status: status.as_u16(),
            headers: cacheable_headers,
            body,
            etag,
            last_modified,
            vary,
        }
    }
    
    pub fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.status).unwrap_or(StatusCode::OK)
    }
}
```

### `skp-cache-http/src/cache_control.rs`

```rust
use std::time::Duration;

/// Parsed Cache-Control header
#[derive(Debug, Clone, Default)]
pub struct CacheControl {
    /// max-age directive
    pub max_age: Option<Duration>,
    
    /// s-maxage directive (for shared caches)
    pub s_maxage: Option<Duration>,
    
    /// stale-while-revalidate directive
    pub stale_while_revalidate: Option<Duration>,
    
    /// stale-if-error directive
    pub stale_if_error: Option<Duration>,
    
    /// no-cache directive
    pub no_cache: bool,
    
    /// no-store directive
    pub no_store: bool,
    
    /// private directive
    pub private: bool,
    
    /// public directive
    pub public: bool,
    
    /// must-revalidate directive
    pub must_revalidate: bool,
    
    /// immutable directive
    pub immutable: bool,
}

impl CacheControl {
    /// Parse a Cache-Control header value
    pub fn parse(value: &str) -> Self {
        let mut cc = Self::default();
        
        for directive in value.split(',').map(|s| s.trim()) {
            let parts: Vec<&str> = directive.splitn(2, '=').collect();
            let name = parts[0].to_lowercase();
            let value = parts.get(1).map(|s| s.trim_matches('"'));
            
            match name.as_str() {
                "max-age" => {
                    if let Some(v) = value.and_then(|v| v.parse().ok()) {
                        cc.max_age = Some(Duration::from_secs(v));
                    }
                }
                "s-maxage" => {
                    if let Some(v) = value.and_then(|v| v.parse().ok()) {
                        cc.s_maxage = Some(Duration::from_secs(v));
                    }
                }
                "stale-while-revalidate" => {
                    if let Some(v) = value.and_then(|v| v.parse().ok()) {
                        cc.stale_while_revalidate = Some(Duration::from_secs(v));
                    }
                }
                "stale-if-error" => {
                    if let Some(v) = value.and_then(|v| v.parse().ok()) {
                        cc.stale_if_error = Some(Duration::from_secs(v));
                    }
                }
                "no-cache" => cc.no_cache = true,
                "no-store" => cc.no_store = true,
                "private" => cc.private = true,
                "public" => cc.public = true,
                "must-revalidate" => cc.must_revalidate = true,
                "immutable" => cc.immutable = true,
                _ => {}
            }
        }
        
        cc
    }
    
    /// Check if the response is cacheable
    pub fn is_cacheable(&self) -> bool {
        !self.no_store && !self.private
    }
    
    /// Get the effective TTL for caching
    pub fn effective_ttl(&self) -> Option<Duration> {
        // s-maxage takes precedence for shared caches
        self.s_maxage.or(self.max_age)
    }
}
```

---

## 🔌 Framework Integration

### Axum Middleware (`skp-cache-axum/src/layer.rs`)

```rust
use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
    middleware::Next,
};
use std::sync::Arc;
use tower::{Layer, Service};

use skp_cache_core::{CacheBackend, CacheManager, CacheOpts};
use skp_cache_http::{CacheControl, CachedResponse, HttpCachePolicy};

/// Axum layer for HTTP response caching
#[derive(Clone)]
pub struct CacheLayer<B: CacheBackend> {
    cache: Arc<CacheManager<B>>,
    policy_manager: Arc<HttpCachePolicyManager>,
}

impl<B: CacheBackend> CacheLayer<B> {
    pub fn new(cache: CacheManager<B>) -> Self {
        Self {
            cache: Arc::new(cache),
            policy_manager: Arc::new(HttpCachePolicyManager::new()),
        }
    }
    
    /// Set default TTL for cached responses
    pub fn default_ttl(mut self, ttl: Duration) -> Self {
        self.policy_manager = Arc::new(
            self.policy_manager.as_ref().clone().default_ttl(ttl)
        );
        self
    }
    
    /// Enable Cache-Control header awareness
    pub fn cache_control_aware(mut self, enabled: bool) -> Self {
        self.policy_manager = Arc::new(
            self.policy_manager.as_ref().clone().cache_control_aware(enabled)
        );
        self
    }
    
    /// Add per-route cache policy
    pub fn route(mut self, path: &str, policy: HttpCachePolicy) -> Self {
        self.policy_manager = Arc::new(
            self.policy_manager.as_ref().clone().add_route(path, policy)
        );
        self
    }
    
    /// Vary cache by specific headers
    pub fn vary_by(mut self, headers: &[&str]) -> Self {
        self.policy_manager = Arc::new(
            self.policy_manager.as_ref().clone().vary_by(headers)
        );
        self
    }
}

impl<S, B: CacheBackend + Clone + 'static> Layer<S> for CacheLayer<B> {
    type Service = CacheMiddleware<S, B>;
    
    fn layer(&self, inner: S) -> Self::Service {
        CacheMiddleware {
            inner,
            cache: self.cache.clone(),
            policy_manager: self.policy_manager.clone(),
        }
    }
}

/// The actual middleware service
#[derive(Clone)]
pub struct CacheMiddleware<S, B: CacheBackend> {
    inner: S,
    cache: Arc<CacheManager<B>>,
    policy_manager: Arc<HttpCachePolicyManager>,
}

impl<S, B> Service<Request<Body>> for CacheMiddleware<S, B>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send,
    B: CacheBackend + Clone + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
    
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    
    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let cache = self.cache.clone();
        let policy_manager = self.policy_manager.clone();
        let mut inner = self.inner.clone();
        
        Box::pin(async move {
            let method = req.method().clone();
            let uri = req.uri().clone();
            let path = uri.path();
            
            // Only cache GET and HEAD requests
            if method != Method::GET && method != Method::HEAD {
                return inner.call(req).await;
            }
            
            // Get policy for this route
            let policy = policy_manager.get_policy(path);
            
            // Check if caching is bypassed
            if policy.bypass {
                return inner.call(req).await;
            }
            
            // Generate cache key
            let cache_key = generate_cache_key(&req, &policy);
            
            // Check cache
            if let Ok(CacheResult::Hit(entry)) = cache.get::<CachedResponse>(&cache_key).await {
                // Check conditional request headers
                if let Some(response) = handle_conditional_request(&req, &entry.value) {
                    return Ok(response);
                }
                
                // Return cached response
                let mut response = entry.value.to_response();
                response.headers_mut().insert("x-cache", "HIT".parse().unwrap());
                return Ok(response);
            }
            
            // Cache miss - call inner service
            let response = inner.call(req).await?;
            
            // Check if response is cacheable
            let cache_control = response.headers()
                .get("cache-control")
                .and_then(|v| v.to_str().ok())
                .map(CacheControl::parse)
                .unwrap_or_default();
            
            if !is_cacheable(&response, &cache_control, &policy) {
                return Ok(response);
            }
            
            // Read response body
            let (parts, body) = response.into_parts();
            let body_bytes = hyper::body::to_bytes(body).await
                .map_err(|_| /* error handling */)?;
            
            // Create cached response
            let cached = CachedResponse::from_response(
                parts.status,
                &parts.headers,
                body_bytes.to_vec(),
            );
            
            // Determine TTL
            let ttl = policy_manager.effective_ttl(&cache_control, &policy);
            let swr = cache_control.stale_while_revalidate;
            
            // Store in cache
            let opts = CacheOpts::new()
                .ttl(ttl)
                .stale_while_revalidate(swr.unwrap_or_default())
                .tags(policy.tags.iter().map(|s| s.as_str()));
            
            let _ = cache.set(&cache_key, &cached, opts).await;
            
            // Reconstruct response
            let mut response = Response::from_parts(parts, Body::from(body_bytes));
            response.headers_mut().insert("x-cache", "MISS".parse().unwrap());
            
            Ok(response)
        })
    }
}

fn generate_cache_key(req: &Request<Body>, policy: &HttpCachePolicy) -> String {
    let mut key = format!("http:{}:{}", req.method(), req.uri().path());
    
    // Add query string if present
    if let Some(query) = req.uri().query() {
        key.push_str("?");
        key.push_str(query);
    }
    
    // Add Vary headers
    for header_name in &policy.vary_headers {
        if let Some(value) = req.headers().get(header_name) {
            if let Ok(v) = value.to_str() {
                key.push_str(":");
                key.push_str(header_name);
                key.push_str("=");
                key.push_str(v);
            }
        }
    }
    
    key
}
```

### Axum Extractor (`skp-cache-axum/src/extractor.rs`)

```rust
use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::request::Parts,
};
use std::sync::Arc;

use skp_cache_core::{CacheManager, CacheBackend};

/// Extractor for accessing cache in handlers
pub struct Cache<B: CacheBackend>(pub Arc<CacheManager<B>>);

#[async_trait]
impl<S, B> FromRequestParts<S> for Cache<B>
where
    B: CacheBackend + Clone + 'static,
    S: Send + Sync,
    Arc<CacheManager<B>>: FromRef<S>,
{
    type Rejection = std::convert::Infallible;
    
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cache = Arc::<CacheManager<B>>::from_ref(state);
        Ok(Cache(cache))
    }
}

impl<B: CacheBackend> std::ops::Deref for Cache<B> {
    type Target = CacheManager<B>;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Usage example:
// async fn handler(Cache(cache): Cache<MemoryBackend>) -> impl IntoResponse {
//     let value = cache.get::<String>("key").await?;
//     ...
// }
```

---

## 🎛️ Feature Flags

### `skp-cache/Cargo.toml`

```toml
[package]
name = "skp-cache"
version = "0.1.0"
edition = "2024"
description = "Advanced, modular caching library for Rust"
license = "MIT OR Apache-2.0"
repository = "https://github.com/setulabs/skp-cache"
keywords = ["cache", "redis", "axum", "actix", "async"]
categories = ["caching", "web-programming", "asynchronous"]

[features]
default = ["memory", "json"]

# Storage backends
memory = ["skp-cache-storage/memory"]
redis = ["skp-cache-storage/redis"]
multi-tier = ["skp-cache-storage/multi-tier", "memory", "redis"]

# Serialization formats
json = ["serde_json"]
msgpack = ["rmp-serde"]
bincode = ["dep:bincode"]

# Framework integrations
axum = ["skp-cache-axum"]
actix = ["skp-cache-actix"]

# HTTP caching
http = ["skp-cache-http"]

# Advanced features
bloom-filter = ["skp-cache-storage/bloom-filter"]
compression = ["skp-cache-storage/compression"]
metrics = ["dep:metrics"]
tracing = ["dep:tracing"]

# Derive macros
derive = ["skp-cache-derive"]

# All features
full = [
    "memory", "redis", "multi-tier",
    "json", "msgpack", "bincode",
    "axum", "actix", "http",
    "bloom-filter", "compression",
    "metrics", "tracing", "derive"
]

[dependencies]
skp-cache-core = { path = "../skp-cache-core" }
skp-cache-storage = { path = "../skp-cache-storage", optional = true }
skp-cache-http = { path = "../skp-cache-http", optional = true }
skp-cache-axum = { path = "../skp-cache-axum", optional = true }
skp-cache-actix = { path = "../skp-cache-actix", optional = true }
skp-cache-derive = { path = "../skp-cache-derive", optional = true }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = { version = "1.0", optional = true }
rmp-serde = { version = "1.1", optional = true }
bincode = { version = "1.3", optional = true }

# Optional dependencies
metrics = { version = "0.22", optional = true }
tracing = { version = "0.1", optional = true }

[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
criterion = "0.5"

[[bench]]
name = "throughput"
harness = false
```

### `skp-cache-storage/Cargo.toml`

```toml
[package]
name = "skp-cache-storage"
version = "0.1.0"
edition = "2024"

[features]
default = ["memory"]

# Backends
memory = ["dashmap", "parking_lot"]
redis = ["dep:redis", "dep:bb8", "dep:bb8-redis"]
multi-tier = ["memory", "redis"]

# Optimizations
bloom-filter = ["bloomfilter"]
compression = ["zstd"]

[dependencies]
skp-cache-core = { path = "../skp-cache-core" }

async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["sync", "time"] }

# Memory backend
dashmap = { version = "5.5", optional = true }
parking_lot = { version = "0.12", optional = true }

# Redis backend
redis = { version = "0.25", features = ["tokio-comp", "connection-manager"], optional = true }
bb8 = { version = "0.8", optional = true }
bb8-redis = { version = "0.15", optional = true }

# Optimizations
bloomfilter = { version = "1.0", optional = true }
zstd = { version = "0.13", optional = true }

# Utilities
rand = "0.8"
uuid = { version = "1.6", features = ["v4"] }
```

---

## 🚀 Additional Features

### Batch Operations

```rust
impl<B: CacheBackend, S: Serializer, M: CacheMetrics> CacheManager<B, S, M> {
    /// Batch get: Retrieve multiple values at once
    pub async fn get_many<T, K>(&self, keys: &[K]) -> Result<HashMap<String, T>, CacheError>
    where
        T: serde::de::DeserializeOwned,
        K: CacheKey,
    {
        let full_keys: Vec<String> = keys.iter()
            .map(|k| self.full_key(&k.full_key()))
            .collect();
        
        let key_strs: Vec<&str> = full_keys.iter().map(|s| s.as_str()).collect();
        let entries = self.backend.get_many(&key_strs).await?;
        
        let mut result = HashMap::new();
        for (i, entry_opt) in entries.into_iter().enumerate() {
            if let Some(entry) = entry_opt {
                if !entry.is_expired() || entry.is_stale() {
                    if let Ok(value) = self.serializer.deserialize::<T>(&entry.value) {
                        result.insert(full_keys[i].clone(), value);
                    }
                }
            }
        }
        
        Ok(result)
    }
    
    /// Batch get-or-compute: Efficiently fetch multiple keys, computing missing ones
    pub async fn batch_get_or_compute<T, K, F, Fut>(
        &self,
        keys: &[K],
        compute_missing: F,
    ) -> Result<HashMap<String, T>, CacheError>
    where
        K: CacheKey,
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + 'static,
        F: FnOnce(Vec<String>) -> Fut,
        Fut: Future<Output = Result<Vec<(String, T)>, CacheError>> + Send,
    {
        // First, try to get all keys from cache
        let mut result = self.get_many::<T, K>(keys).await?;
        
        // Find missing keys
        let all_keys: Vec<String> = keys.iter()
            .map(|k| self.full_key(&k.full_key()))
            .collect();
        let missing: Vec<String> = all_keys.iter()
            .filter(|k| !result.contains_key(*k))
            .cloned()
            .collect();
        
        if missing.is_empty() {
            return Ok(result);
        }
        
        // Compute missing values
        let computed = compute_missing(missing).await?;
        
        // Store computed values and add to result
        for (key, value) in computed {
            self.set(&key, &value, CacheOpts::new()).await?;
            result.insert(key, value);
        }
        
        Ok(result)
    }
}
```

### Negative Caching

```rust
impl<B: CacheBackend, S: Serializer, M: CacheMetrics> CacheManager<B, S, M> {
    /// Cache a "not found" result to prevent repeated lookups
    pub async fn set_negative(
        &self,
        key: impl CacheKey,
        ttl: Duration,
    ) -> Result<(), CacheError> {
        let opts = CacheOpts::new()
            .ttl(ttl)
            .negative();  // Mark as negative cache entry
        
        // Store a sentinel value to indicate "known missing"
        self.set(&key.full_key(), NegativeCacheMarker, opts).await
    }
    
    /// Check if a key is negatively cached
    pub async fn is_negative(&self, key: impl CacheKey) -> Result<bool, CacheError> {
        match self.get::<NegativeCacheMarker>(&key.full_key()).await? {
            CacheResult::Hit(entry) => Ok(entry.value.is_negative),
            _ => Ok(false),
        }
    }
}

/// Marker type for negative cache entries
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NegativeCacheMarker {
    is_negative: bool,
}
```

### Circuit Breaker for L2 (Redis)

```rust
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,     // Normal operation
    Open,       // Failing, skip L2
    HalfOpen,   // Testing if L2 recovered
}

/// Circuit breaker for L2 backend resilience
pub struct CircuitBreaker {
    state: AtomicU8,
    failure_count: AtomicU64,
    last_failure: AtomicU64,
    config: CircuitBreakerConfig,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u64,
    /// Time to wait before trying L2 again (half-open)
    pub recovery_timeout: Duration,
    /// Number of successes in half-open before closing
    pub success_threshold: u64,
}

impl CircuitBreaker {
    pub fn should_allow(&self) -> bool {
        match self.state() {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if recovery timeout has elapsed
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if now - self.last_failure.load(Ordering::Relaxed) > self.config.recovery_timeout.as_secs() {
                    self.transition_to(CircuitState::HalfOpen);
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }
    
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        if self.state() == CircuitState::HalfOpen {
            self.transition_to(CircuitState::Closed);
        }
    }
    
    pub fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        self.last_failure.store(
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            Ordering::Relaxed,
        );
        
        if failures >= self.config.failure_threshold {
            self.transition_to(CircuitState::Open);
        }
    }
}
```

### Warm-up API

```rust
impl<B: CacheBackend, S: Serializer, M: CacheMetrics> CacheManager<B, S, M> {
    /// Pre-populate cache from an iterator of key-value pairs
    pub async fn warm_up<T, I>(
        &self,
        entries: I,
        options: CacheOpts,
    ) -> Result<usize, CacheError>
    where
        T: serde::Serialize,
        I: IntoIterator<Item = (String, T)>,
    {
        let mut count = 0;
        for (key, value) in entries {
            self.set(&key, value, options.clone()).await?;
            count += 1;
        }
        Ok(count)
    }
    
    /// Pre-populate cache from a data source with parallel loading
    pub async fn warm_up_parallel<T, K, F, Fut>(
        &self,
        keys: Vec<K>,
        fetch: F,
        concurrency: usize,
    ) -> Result<usize, CacheError>
    where
        K: CacheKey + Send + 'static,
        T: serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
        F: Fn(K) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, CacheError>> + Send,
    {
        use futures::stream::{self, StreamExt};
        
        let count = Arc::new(AtomicU64::new(0));
        
        stream::iter(keys)
            .map(|key| {
                let cache = self.clone();
                let fetch = &fetch;
                let count = count.clone();
                async move {
                    if let Ok(value) = fetch(key.clone()).await {
                        if cache.set(&key, &value, CacheOpts::new()).await.is_ok() {
                            count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;
        
        Ok(count.load(Ordering::Relaxed) as usize)
    }
}
```

### TTL Wheel (Time-Based Expiration Index)

```rust
use std::collections::{HashMap, VecDeque};

/// Time wheel for O(1) TTL-based expiration
/// 
/// Instead of scanning all entries to find expired ones,
/// this maintains buckets of keys organized by expiration time.
pub struct TtlWheel {
    /// Buckets of keys, indexed by time slot
    buckets: Vec<HashSet<String>>,
    /// Current bucket index
    current: usize,
    /// Tick duration (bucket resolution)
    tick_duration: Duration,
    /// Map of key -> bucket index for O(1) removal
    key_to_bucket: HashMap<String, usize>,
    /// Maximum TTL supported
    max_ttl: Duration,
}

impl TtlWheel {
    pub fn new(tick_duration: Duration, max_ttl: Duration) -> Self {
        let num_buckets = (max_ttl.as_secs() / tick_duration.as_secs()) as usize + 1;
        Self {
            buckets: vec![HashSet::new(); num_buckets],
            current: 0,
            tick_duration,
            key_to_bucket: HashMap::new(),
            max_ttl,
        }
    }
    
    /// Schedule a key for expiration after `ttl`
    pub fn schedule(&mut self, key: String, ttl: Duration) {
        let ticks = (ttl.as_secs() / self.tick_duration.as_secs()) as usize;
        let bucket_idx = (self.current + ticks) % self.buckets.len();
        
        // Remove from old bucket if exists
        if let Some(old_bucket) = self.key_to_bucket.get(&key) {
            self.buckets[*old_bucket].remove(&key);
        }
        
        self.buckets[bucket_idx].insert(key.clone());
        self.key_to_bucket.insert(key, bucket_idx);
    }
    
    /// Remove a key from the wheel (e.g., on deletion or update)
    pub fn remove(&mut self, key: &str) {
        if let Some(bucket_idx) = self.key_to_bucket.remove(key) {
            self.buckets[bucket_idx].remove(key);
        }
    }
    
    /// Advance the wheel and return expired keys
    pub fn tick(&mut self) -> Vec<String> {
        self.current = (self.current + 1) % self.buckets.len();
        let expired: Vec<String> = self.buckets[self.current].drain().collect();
        
        for key in &expired {
            self.key_to_bucket.remove(key);
        }
        
        expired
    }
}
```

---

## 📅 Implementation Phases

### Phase 1: Core Foundation (Week 1-2)

| Task | Priority | Crate |
|------|----------|-------|
| Core traits (`CacheBackend`, `CacheKey`, `Cacheable`) | P0 | skp-cache-core |
| Serializer trait + JSON/MsgPack/Bincode impls | P0 | skp-cache-core |
| CacheMetrics trait + NoopMetrics | P0 | skp-cache-core |
| Types (`CacheEntry`, `CacheOptions`, `CacheResult`) | P0 | skp-cache-core |
| Error handling | P0 | skp-cache-core |
| Memory backend (basic with DashMap) | P0 | skp-cache-storage |
| TTL Wheel for expiration | P1 | skp-cache-storage |
| CacheManager (get/set with serializer) | P0 | skp-cache |
| Unit tests | P0 | All |

### Phase 2: Redis & Multi-tier (Week 3-4)

| Task | Priority | Crate |
|------|----------|-------|
| Redis backend (SCAN-based, non-blocking) | P0 | skp-cache-storage |
| Connection pooling (bb8) | P0 | skp-cache-storage |
| Multi-tier backend (L1+L2) | P0 | skp-cache-storage |
| Tag-based invalidation | P0 | skp-cache-storage |
| Circuit breaker for L2 | P1 | skp-cache-storage |
| Integration tests | P0 | All |

### Phase 3: Advanced Features (Week 5-6)

| Task | Priority | Crate |
|------|----------|-------|
| Dependency graph | P0 | skp-cache-core |
| Cascade invalidation | P0 | skp-cache |
| Request coalescing (fixed implementation) | P0 | skp-cache |
| Stale-while-revalidate | P1 | skp-cache |
| Probabilistic early refresh | P1 | skp-cache |
| TTL jitter | P1 | skp-cache |
| Batch operations (get_many, batch_get_or_compute) | P1 | skp-cache |
| Negative caching | P2 | skp-cache |
| Warm-up API | P2 | skp-cache |
| Cost-aware eviction (CAMP) | P2 | skp-cache-storage |

### Phase 4: HTTP & Framework Integration (Week 7-8)

| Task | Priority | Crate |
|------|----------|-------|
| HTTP response types | P0 | skp-cache-http |
| Cache-Control parsing | P0 | skp-cache-http |
| Axum middleware | P0 | skp-cache-axum |
| Axum extractor | P0 | skp-cache-axum |
| Actix middleware | P2 | skp-cache-actix |
| Per-route policies | P1 | skp-cache-axum/actix |

### Phase 5: Optimizations & Polish (Week 9-10)

| Task | Priority | Crate |
|------|----------|-------|
| Bloom filter | P1 | skp-cache-storage |
| Compression (zstd) | P2 | skp-cache-storage |
| Redis pub/sub (distributed invalidation) | P1 | skp-cache-storage |
| Metrics crate adapter | P1 | skp-cache |
| Graceful shutdown | P1 | skp-cache |
| Documentation (rustdoc + README) | P0 | All |
| Benchmarks (criterion) | P1 | All |
| Examples | P0 | All |
| Publish to crates.io | P0 | All |

### Phase 6: Post-Launch Enhancements (Week 11+)

| Task | Priority | Crate |
|------|----------|-------|
| `#[derive(CacheKey)]` proc macro | P2 | skp-cache-derive |
| Read-through mode | P2 | skp-cache |
| Cache groups/namespaces | P2 | skp-cache |
| Tracing integration | P2 | skp-cache |
| Additional backends (Memcached, etc.) | P3 | skp-cache-storage |

---

## 📘 Usage Examples

### Basic Usage (Default JSON Serializer)

```rust
use skp_cache::{CacheManager, CacheOpts, MemoryBackend, MemoryConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create cache with memory backend (uses JSON serializer by default)
    let backend = MemoryBackend::new(MemoryConfig::default());
    let cache = CacheManager::new(backend);
    
    // Simple get/set
    cache.set("user:123", User { name: "Alice".into() }, CacheOpts::new()
        .ttl_secs(300)
        .tags(&["users", "user:123"])
    ).await?;
    
    let user: Option<User> = cache.get("user:123").await?.value();
    
    // Get or compute
    let user = cache.get_or_compute("user:456", || async {
        db.fetch_user(456).await
    }).await?;
    
    Ok(())
}
```

### Custom Serializer (MessagePack or Bincode)

```rust
use skp_cache::{
    CacheManager, CacheManagerConfig, CacheOpts,
    MemoryBackend, MemoryConfig,
    MsgPackSerializer, BincodeSerializer,  // Enable with features
    NoopMetrics,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = MemoryBackend::new(MemoryConfig::default());
    
    // Use MessagePack for faster serialization + smaller payloads
    let cache = CacheManager::with_serializer_and_metrics(
        backend,
        MsgPackSerializer,  // or BincodeSerializer for maximum speed
        NoopMetrics,
        CacheManagerConfig::default(),
    );
    
    // API is identical
    cache.set("key", MyData { ... }, CacheOpts::new().ttl_secs(60)).await?;
    let data: MyData = cache.get("key").await?.value().unwrap();
    
    Ok(())
}
```

### With Metrics Integration

```rust
use skp_cache::{
    CacheManager, CacheManagerConfig, CacheOpts,
    MemoryBackend, JsonSerializer, MetricsCrateAdapter,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up metrics exporter (e.g., Prometheus)
    // ... metrics recorder setup ...
    
    let backend = MemoryBackend::new(MemoryConfig::default());
    
    // Create cache with metrics adapter
    let cache = CacheManager::with_serializer_and_metrics(
        backend,
        JsonSerializer,
        MetricsCrateAdapter::new("skp_cache"),  // Prefix for all metrics
        CacheManagerConfig::default(),
    );
    
    // All operations now emit metrics:
    // - skp_cache_hits_total
    // - skp_cache_misses_total
    // - skp_cache_operation_duration_seconds
    // - skp_cache_evictions_total
    // - etc.
    
    cache.set("key", "value", CacheOpts::new()).await?;
    let _ = cache.get::<String>("key").await?;
    
    Ok(())
}
```

### Dependency Graph

```rust
use skp_cache::{CacheManager, CacheOpts};

// Set up hierarchical data with dependencies
cache.set("tenant:acme", tenant, CacheOpts::new()
    .ttl_secs(3600)
    .tags(&["tenants"])
).await?;

cache.set("user:123", user, CacheOpts::new()
    .ttl_secs(1800)
    .tags(&["users", "tenant:acme/users"])
    .depends_on(&["tenant:acme"])  // Depends on tenant
).await?;

cache.set("user:123:posts", posts, CacheOpts::new()
    .ttl_secs(600)
    .tags(&["posts", "user:123/posts"])
    .depends_on(&["user:123"])  // Depends on user
).await?;

// Invalidating tenant cascades to user and posts
let invalidated = cache.invalidate("tenant:acme").await?;
// invalidated == 3 (tenant + user + posts)
```

### Multi-tier with Redis

```rust
use skp_cache::{
    CacheManager, MemoryBackend, RedisBackend,
    MultiTierBackend, MultiTierConfig, WriteStrategy,
};

// Create backends
let l1 = MemoryBackend::new(MemoryConfig {
    max_capacity: 10_000,
    ..Default::default()
});

let l2 = RedisBackend::new(RedisConfig {
    url: "redis://localhost:6379".into(),
    ..Default::default()
}).await?;

// Create multi-tier backend
let backend = MultiTierBackend::new(l1, l2, MultiTierConfig {
    l1_ttl: Some(Duration::from_secs(60)),
    promote_on_hit: true,
    write_strategy: WriteStrategy::WriteThrough,
    ..Default::default()
});

let cache = CacheManager::new(backend);
```

### Axum Integration

```rust
use axum::{Router, routing::get};
use skp_cache::{CacheManager, MemoryBackend};
use skp_cache_axum::{CacheLayer, Cache};
use std::sync::Arc;

async fn get_user(
    Path(id): Path<u64>,
    Cache(cache): Cache<MemoryBackend>,
) -> impl IntoResponse {
    let user = cache.get_or_compute(
        format!("user:{}", id),
        || async { db.fetch_user(id).await }
    ).await?;
    
    Json(user)
}

#[tokio::main]
async fn main() {
    let cache = Arc::new(CacheManager::new(MemoryBackend::default()));
    
    let app = Router::new()
        .route("/users/:id", get(get_user))
        .layer(CacheLayer::new(cache.clone())
            .default_ttl(Duration::from_secs(300))
            .route("/api/products", HttpCachePolicy::new()
                .ttl(Duration::from_secs(600))
                .tags(&["products"]))
            .route("/api/users/*", HttpCachePolicy::bypass()))
        .with_state(cache);
    
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

### Request Coalescing

```rust
// Without coalescing: 1000 concurrent requests = 1000 DB queries
// With coalescing: 1000 concurrent requests = 1 DB query

let cache = CacheManager::with_config(backend, CacheManagerConfig {
    enable_coalescing: true,
    ..Default::default()
});

// All concurrent calls for the same key will share one computation
let result = cache.get_or_compute_with(
    "expensive:computation",
    CacheOpts::new().coalesce(),
    || async {
        // This only executes ONCE even with 1000 concurrent calls
        expensive_computation().await
    }
).await?;
```

---

## 🧪 Testing Strategy

| Test Type | Focus | Location |
|-----------|-------|----------|
| Unit tests | Individual components | `src/*/tests.rs` |
| Integration tests | Backend interactions | `tests/` |
| Property tests | Invariants (proptest) | `tests/property/` |
| Benchmark tests | Performance | `benches/` |
| Stress tests | Concurrency, stampede | `tests/stress/` |

---

## 📊 Success Metrics

| Metric | Target |
|--------|--------|
| L1 cache hit latency | < 1μs |
| L2 cache hit latency (Redis) | < 5ms |
| Throughput (memory backend) | > 1M ops/sec |
| Cache hit ratio (typical workload) | > 90% |
| Stampede protection | 100% (1 query under concurrent load) |
| Cascade invalidation | O(n) where n = dependents |

---

## 📚 References

- [TinyLFU Paper](https://arxiv.org/abs/1512.00727)
- [CAMP Algorithm](https://arxiv.org/abs/2411.01246)
- [Cache Stampede Prevention](https://en.wikipedia.org/wiki/Cache_stampede)
- [Caffeine (Java)](https://github.com/ben-manes/caffeine)
- [Moka (Rust)](https://github.com/moka-rs/moka)
- [X-Fetch Algorithm](https://cseweb.ucsd.edu/~avattani/papers/cache_stampede.pdf)

---

## 🚀 Getting Started (After Implementation)

```toml
# Cargo.toml
[dependencies]
skp-cache = { version = "0.1", features = ["redis", "axum"] }
```

```rust
use skp_cache::prelude::*;

#[tokio::main]
async fn main() -> Result<(), CacheError> {
    let cache = CacheManager::new(MemoryBackend::default());
    
    cache.set("hello", "world", CacheOpts::new().ttl_secs(60)).await?;
    
    let value: String = cache.get("hello").await?.value().unwrap();
    println!("{}", value); // "world"
    
    Ok(())
}
```

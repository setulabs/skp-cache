//! High-level cache manager

use std::sync::Arc;
use std::time::{Duration, Instant};

use skp_cache_core::{
    CacheBackend, CacheEntry, CacheKey, CacheMetrics, CacheOperation, CacheOptions,
    CacheResult, CacheTier, JsonSerializer, NoopMetrics, Result, Serializer,
};

/// Configuration for CacheManager
#[derive(Debug, Clone)]
pub struct CacheManagerConfig {
    /// Default TTL for entries without explicit TTL
    pub default_ttl: Option<Duration>,
    /// Namespace prefix for all keys
    pub namespace: Option<String>,
    /// TTL jitter percentage (0.0 - 1.0) to prevent thundering herd
    pub ttl_jitter: f64,
}

impl Default for CacheManagerConfig {
    fn default() -> Self {
        Self {
            default_ttl: Some(Duration::from_secs(300)),
            namespace: None,
            ttl_jitter: 0.1, // 10% jitter
        }
    }
}

impl CacheManagerConfig {
    /// Create config with specific default TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            default_ttl: Some(ttl),
            ..Default::default()
        }
    }

    /// Create config with namespace
    pub fn with_namespace(namespace: impl Into<String>) -> Self {
        Self {
            namespace: Some(namespace.into()),
            ..Default::default()
        }
    }

    /// Disable TTL jitter
    pub fn no_jitter(mut self) -> Self {
        self.ttl_jitter = 0.0;
        self
    }
}

/// High-level cache manager with pluggable serialization and metrics
///
/// Generic over:
/// - `B`: The cache backend (Memory, Redis, MultiTier)
/// - `S`: The serializer (JSON, MessagePack, Bincode)
/// - `M`: The metrics collector
pub struct CacheManager<B, S = JsonSerializer, M = NoopMetrics>
where
    B: CacheBackend,
    S: Serializer,
    M: CacheMetrics,
{
    backend: Arc<B>,
    serializer: Arc<S>,
    metrics: Arc<M>,
    config: CacheManagerConfig,
}

// Constructors for default serializer/metrics
impl<B: CacheBackend> CacheManager<B, JsonSerializer, NoopMetrics> {
    /// Create a new CacheManager with default JSON serializer and no metrics
    pub fn new(backend: B) -> Self {
        Self::with_config(backend, CacheManagerConfig::default())
    }

    /// Create with custom config
    pub fn with_config(backend: B, config: CacheManagerConfig) -> Self {
        Self {
            backend: Arc::new(backend),
            serializer: Arc::new(JsonSerializer),
            metrics: Arc::new(NoopMetrics),
            config,
        }
    }
}

// Full generic implementation
impl<B, S, M> CacheManager<B, S, M>
where
    B: CacheBackend,
    S: Serializer,
    M: CacheMetrics,
{
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
            config,
        }
    }

    /// Get the full key with namespace prefix
    fn full_key(&self, key: &str) -> String {
        match &self.config.namespace {
            Some(ns) => format!("{}:{}", ns, key),
            None => key.to_string(),
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

    /// Get a value from cache
    pub async fn get<T>(&self, key: impl CacheKey) -> Result<CacheResult<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let full_key = self.full_key(&key.full_key());
        let start = Instant::now();

        let result = match self.backend.get(&full_key).await? {
            Some(entry) => {
                if entry.is_expired() && !entry.is_stale() {
                    self.metrics.record_miss(&full_key);
                    CacheResult::Miss
                } else if entry.is_stale() {
                    self.metrics.record_stale_hit(&full_key);
                    CacheResult::Stale(self.deserialize_entry(entry)?)
                } else {
                    self.metrics.record_hit(&full_key, CacheTier::L1Memory);
                    CacheResult::Hit(self.deserialize_entry(entry)?)
                }
            }
            None => {
                self.metrics.record_miss(&full_key);
                CacheResult::Miss
            }
        };

        self.metrics
            .record_latency(CacheOperation::Get, start.elapsed());
        Ok(result)
    }

    /// Set a value in cache
    pub async fn set<T>(
        &self,
        key: impl CacheKey,
        value: T,
        options: impl Into<CacheOptions>,
    ) -> Result<()>
    where
        T: serde::Serialize,
    {
        let full_key = self.full_key(&key.full_key());
        let mut options = options.into();

        // Apply default TTL if not specified
        if options.ttl.is_none() {
            options.ttl = self.config.default_ttl;
        }

        // Apply jitter to prevent thundering herd
        if let Some(ttl) = options.ttl {
            options.ttl = Some(self.apply_ttl_jitter(ttl));
        }

        // Serialize
        let serialize_start = Instant::now();
        let serialized = self.serializer.serialize(&value)?;
        self.metrics
            .record_latency(CacheOperation::Serialize, serialize_start.elapsed());

        // Store
        let set_start = Instant::now();
        self.backend.set(&full_key, serialized, &options).await?;
        self.metrics
            .record_latency(CacheOperation::Set, set_start.elapsed());

        Ok(())
    }

    /// Delete a key from cache
    pub async fn delete(&self, key: impl CacheKey) -> Result<bool> {
        let full_key = self.full_key(&key.full_key());
        let start = Instant::now();
        let result = self.backend.delete(&full_key).await?;
        self.metrics
            .record_latency(CacheOperation::Delete, start.elapsed());
        Ok(result)
    }

    /// Check if key exists in cache
    pub async fn exists(&self, key: impl CacheKey) -> Result<bool> {
        let full_key = self.full_key(&key.full_key());
        self.backend.exists(&full_key).await
    }

    /// Clear all entries from cache
    pub async fn clear(&self) -> Result<()> {
        self.backend.clear().await
    }

    /// Get cache statistics
    pub async fn stats(&self) -> Result<skp_cache_core::CacheStats> {
        self.backend.stats().await
    }

    /// Get the number of entries
    pub async fn len(&self) -> Result<usize> {
        self.backend.len().await
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> Result<bool> {
        self.backend.is_empty().await
    }

    /// Deserialize a cache entry
    fn deserialize_entry<T>(&self, entry: CacheEntry<Vec<u8>>) -> Result<CacheEntry<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let deserialize_start = Instant::now();
        let value: T = self.serializer.deserialize(&entry.value)?;
        self.metrics
            .record_latency(CacheOperation::Deserialize, deserialize_start.elapsed());

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
}

impl<B, S, M> Clone for CacheManager<B, S, M>
where
    B: CacheBackend,
    S: Serializer,
    M: CacheMetrics,
{
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            serializer: self.serializer.clone(),
            metrics: self.metrics.clone(),
            config: self.config.clone(),
        }
    }
}

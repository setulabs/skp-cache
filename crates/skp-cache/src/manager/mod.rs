//! High-level cache manager

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::{HashSet, VecDeque};

use skp_cache_core::{
    CacheBackend, CacheEntry, CacheKey, CacheMetrics, CacheOperation, CacheOptions,
    CacheResult, CacheTier, DependencyBackend, JsonSerializer, NoopMetrics, Result, Serializer,
    TaggableBackend,
};

mod coalescer;
use coalescer::Coalescer;

mod read_through;
pub use read_through::{Loader, ReadThroughCache, CacheManagerReadThroughExt};

mod groups;
pub use groups::CacheGroup;

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
    B: CacheBackend + DependencyBackend,
    S: Serializer,
    M: CacheMetrics,
{
    backend: Arc<B>,
    serializer: Arc<S>,
    metrics: Arc<M>,
    config: CacheManagerConfig,
    coalescer: Coalescer,
}

// Constructors for default serializer/metrics
impl<B: CacheBackend + DependencyBackend> CacheManager<B, JsonSerializer, NoopMetrics> {
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
            coalescer: Coalescer::new(),
        }
    }
}

// Full generic implementation
impl<B, S, M> CacheManager<B, S, M>
where
    B: CacheBackend + DependencyBackend,
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
            coalescer: Coalescer::new(),
        }
    }

    /// Create a namespaced cache group
    pub fn group(&self, namespace: impl Into<String>) -> CacheGroup<'_, B, S, M> {
        CacheGroup::new(self, namespace.into())
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

        // Use coalescer to prevent thundering herd
        let backend = self.backend.clone();
        let key_clone = full_key.clone();

        let req_result = self.coalescer.do_request(&full_key, move || async move {
            backend.get(&key_clone).await
        }).await?;

        let result = match req_result {
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
        let options = options.into();

        // Serialize
        let serialize_start = Instant::now();
        let serialized = self.serializer.serialize(&value)?;
        self.metrics
            .record_latency(CacheOperation::Serialize, serialize_start.elapsed());

        self.set_raw(&full_key, serialized, options).await
    }

    /// Internal set with full logic (jitter, cascade, metrics)
    async fn set_raw(&self, full_key: &str, value: Vec<u8>, mut options: CacheOptions) -> Result<()> {
        // Apply default TTL if not specified
        if options.ttl.is_none() {
            options.ttl = self.config.default_ttl;
        }

        // Apply jitter
        if let Some(ttl) = options.ttl {
            options.ttl = Some(self.apply_ttl_jitter(ttl));
        }

        // Get dependents for cascade invalidation BEFORE setting
        // (Assuming existing key's dependents might need invalidation if value changes?)
        // Actually, usually dependents depend on the VALUE or the KEY existence.
        // If we update the value, dependents are likely stale.
        let dependents = self.backend.get_dependents(full_key).await.unwrap_or_default();

        // Store
        let set_start = Instant::now();
        self.backend.set(full_key, value, &options).await?;
        self.metrics
            .record_latency(CacheOperation::Set, set_start.elapsed());
            
        // Cascade invalidation
        for dep in dependents {
             let _ = self.invalidate_recursive(&dep).await;
        }

        Ok(())
    }

    /// Get a value from cache, or compute it if missing (coalesced)
    pub async fn get_or_compute<T, F, Fut>(
        &self,
        key: impl CacheKey,
        computer: F,
        options: Option<CacheOptions>,
    ) -> Result<CacheResult<T>>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T>> + Send + 'static,
    {
        let full_key = self.full_key(&key.full_key());
        let backend = self.backend.clone();
        let key_str = full_key.clone();
        let opts = options.unwrap_or_default();
        let manager = self.clone();
        
        // Coalesce the request
        let req_result = self.coalescer.do_request(&full_key, move || async move {
             // 1. Check Backend
             if let Some(entry) = backend.get(&key_str).await? {
                 if !entry.is_expired() {
                      return Ok(Some(entry));
                 }
                 
                 // SWR Logic: If stale, trigger background refresh
                 if entry.is_stale() {
                      let manager_bg = manager.clone();
                      let key_bg = key_str.clone();
                      let opts_bg = opts.clone();
                      
                      manager.coalescer.try_spawn_refresh(&key_str, move || async move {
                           if let Ok(val) = computer().await {
                                // Serialize depends on T. We need T to serialize!
                                // Manager has serializer.
                                // We call set_internal (set_raw).
                                // But set_raw expects Vec<u8>.
                                // CacheManager has serializer.
                                // But `computer` returns T.
                                // We need to serialize T.
                                // `manager_bg.serializer.serialize(&val)`.
                                if let Ok(serialized) = manager_bg.serializer.serialize(&val) {
                                     let _ = manager_bg.set_raw(&key_bg, serialized, opts_bg).await;
                                }
                           }
                      });
                      
                      return Ok(Some(entry));
                 }
             }
             
             // 2. Compute (Miss case)
             let val = computer().await?;
             let serialized = manager.serializer.serialize(&val)?;
             let size = serialized.len();
             
             // 3. Set (using set_raw for full logic)
             manager.set_raw(&key_str, serialized.clone(), opts).await?;
             
             Ok(Some(CacheEntry::new(serialized, size)))
        }).await?;

        match req_result {
            Some(entry) => {
                if entry.is_stale() {
                    Ok(CacheResult::Stale(self.deserialize_entry(entry)?))
                } else {
                    Ok(CacheResult::Hit(self.deserialize_entry(entry)?))
                }
            },
            None => Err(skp_cache_core::CacheError::Internal("Compute returned None".into()))
        }
    }

    /// Delete a key from cache (with cascade invalidation)
    pub async fn delete(&self, key: impl CacheKey) -> Result<bool> {
        let full_key = self.full_key(&key.full_key());
        let start = Instant::now();
        
        // Use recursive invalidation
        let result = self.invalidate_recursive(&full_key).await?;
        
        self.metrics
            .record_latency(CacheOperation::Delete, start.elapsed());
        Ok(result.0)
    }

    /// Invalidate a key and all its dependents (cascade invalidation)
    /// 
    /// Returns the number of entries invalidated
    pub async fn invalidate(&self, key: impl CacheKey) -> Result<u64> {
        let full_key = self.full_key(&key.full_key());
        let start = Instant::now();
        
        let result = self.invalidate_recursive(&full_key).await?;
        
        self.metrics
            .record_latency(CacheOperation::Invalidate, start.elapsed());
        Ok(result.1)
    }

    /// Recursive invalidation of dependents
    /// Returns (initial_key_deleted, total_count)
    async fn invalidate_recursive(&self, key: &str) -> Result<(bool, u64)> {
        let mut queue = VecDeque::new();
        queue.push_back(key.to_string());
        let mut visited = HashSet::new();
        visited.insert(key.to_string());
        
        let mut initial_deleted = false;
        let mut first = true;
        let mut count = 0u64;
        
        while let Some(k) = queue.pop_front() {
             // Get dependents first
             if let Ok(deps) = self.backend.get_dependents(&k).await {
                  for dep in deps {
                      if visited.insert(dep.clone()) {
                          queue.push_back(dep);
                      }
                  }
             }
             // Delete
             let deleted = self.backend.delete(&k).await?;
             if deleted {
                 count += 1;
             }
             if first {
                 initial_deleted = deleted;
                 first = false;
             }
        }
        Ok((initial_deleted, count))
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
    B: CacheBackend + DependencyBackend,
    S: Serializer,
    M: CacheMetrics,
{
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            serializer: self.serializer.clone(),
            metrics: self.metrics.clone(),
            config: self.config.clone(),
            coalescer: self.coalescer.clone(),
        }
    }
}

// Taggable operations
impl<B, S, M> CacheManager<B, S, M>
where
    B: CacheBackend + DependencyBackend + TaggableBackend,
    S: Serializer,
    M: CacheMetrics,
{
    /// Delete all entries with a specific tag
    pub async fn delete_by_tag(&self, tag: &str) -> Result<u64> {
        let start = Instant::now();
        let count = self.backend.delete_by_tag(tag).await?;
        self.metrics
            .record_latency(CacheOperation::Invalidate, start.elapsed());
        Ok(count)
    }

    /// Get all keys by tag
    pub async fn get_keys_by_tag(&self, tag: &str) -> Result<Vec<String>> {
        self.backend.get_by_tag(tag).await
    }
}

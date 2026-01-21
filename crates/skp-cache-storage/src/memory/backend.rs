//! In-memory cache backend using DashMap

use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use skp_cache_core::{CacheBackend, CacheEntry, CacheOptions, CacheStats, Result, TaggableBackend};

use super::ttl_index::TtlIndex;

/// Configuration for the memory backend
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum number of entries (0 = unlimited)
    pub max_capacity: usize,
    /// Cleanup interval for expired entries
    pub cleanup_interval: Duration,
    /// Maximum TTL supported (for TTL index sizing)
    pub max_ttl: Duration,
    /// Enable TTL index for efficient expiration
    pub enable_ttl_index: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            cleanup_interval: Duration::from_secs(60),
            max_ttl: Duration::from_secs(86400), // 24 hours
            enable_ttl_index: true,
        }
    }
}

impl MemoryConfig {
    /// Create config with specific capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            max_capacity: capacity,
            ..Default::default()
        }
    }

    /// Create config with unlimited capacity
    pub fn unlimited() -> Self {
        Self {
            max_capacity: 0,
            ..Default::default()
        }
    }
}

/// Internal statistics tracking
#[derive(Debug, Default)]
struct MemoryStats {
    hits: u64,
    misses: u64,
    stale_hits: u64,
    writes: u64,
    deletes: u64,
    evictions: u64,
}

/// Tag index for tag-based lookups
type TagIndex = DashMap<String, HashSet<String>>;

/// In-memory cache backend
///
/// Uses `DashMap` for concurrent access and `TtlIndex` for efficient expiration.
/// Cloning creates a new handle to the SAME underlying store.
#[derive(Clone)]
pub struct MemoryBackend {
    /// Main data store
    data: Arc<DashMap<String, CacheEntry<Vec<u8>>>>,
    /// Tag -> keys index
    tag_index: Arc<TagIndex>,
    /// TTL expiration index
    ttl_index: Arc<RwLock<TtlIndex>>,
    /// Statistics
    stats: Arc<RwLock<MemoryStats>>,
    /// Configuration
    config: MemoryConfig,
}

impl MemoryBackend {
    /// Create a new memory backend
    pub fn new(config: MemoryConfig) -> Self {
        let ttl_index = TtlIndex::new(Duration::from_secs(1), config.max_ttl);

        Self {
            data: Arc::new(DashMap::with_capacity(config.max_capacity.min(10_000))),
            tag_index: Arc::new(DashMap::new()),
            ttl_index: Arc::new(RwLock::new(ttl_index)),
            stats: Arc::new(RwLock::new(MemoryStats::default())),
            config,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(MemoryConfig::default())
    }

    /// Evict entries if at capacity
    fn maybe_evict(&self) {
        if self.config.max_capacity == 0 {
            return; // Unlimited
        }

        // Only evict if we're at or over capacity
        if self.data.len() < self.config.max_capacity {
            return;
        }

        // Simple eviction: collect keys to remove first
        let keys_to_remove: Vec<String> = self
            .data
            .iter()
            .take(self.data.len().saturating_sub(self.config.max_capacity - 1))
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys_to_remove {
            self.data.remove(&key);
            self.ttl_index.write().remove(&key);
            self.stats.write().evictions += 1;
        }
    }

    /// Remove an entry and clean up indexes
    fn remove_entry(&self, key: &str) {
        if let Some((_, entry)) = self.data.remove(key) {
            // Remove from TTL index
            self.ttl_index.write().remove(key);

            // Remove from tag index
            for tag in &entry.tags {
                if let Some(mut keys) = self.tag_index.get_mut(tag) {
                    keys.remove(key);
                }
            }
        }
    }

    /// Run TTL cleanup and return number of expired entries removed
    pub fn cleanup_expired(&self) -> usize {
        let expired = self.ttl_index.write().tick();
        let mut count = 0;

        for key in expired {
            if let Some(entry) = self.data.get(&key) {
                if entry.is_expired() && !entry.is_stale() {
                    drop(entry);
                    self.remove_entry(&key);
                    self.stats.write().evictions += 1;
                    count += 1;
                }
            }
        }

        count
    }

    /// Get approximate memory usage
    pub fn memory_usage(&self) -> usize {
        self.data
            .iter()
            .map(|entry| entry.size + entry.key().len())
            .sum()
    }
}

#[async_trait]
impl CacheBackend for MemoryBackend {
    async fn get(&self, key: &str) -> Result<Option<CacheEntry<Vec<u8>>>> {
        match self.data.get_mut(key) {
            Some(mut entry) => {
                // Check expiration
                if entry.is_expired() && !entry.is_stale() {
                    drop(entry);
                    self.remove_entry(key);
                    self.stats.write().misses += 1;
                    return Ok(None);
                }

                // Update access metadata
                entry.last_accessed = SystemTime::now();
                entry.access_count += 1;

                // Update stats
                let mut stats = self.stats.write();
                if entry.is_stale() {
                    stats.stale_hits += 1;
                } else {
                    stats.hits += 1;
                }

                Ok(Some(entry.clone()))
            }
            None => {
                self.stats.write().misses += 1;
                Ok(None)
            }
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>, options: &CacheOptions) -> Result<()> {
        self.maybe_evict();

        let size = value.len();
        let now = SystemTime::now();

        let entry = CacheEntry {
            value,
            created_at: now,
            last_accessed: now,
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

        // Schedule TTL expiration
        if self.config.enable_ttl_index {
            if let Some(ttl) = options.ttl {
                let total_ttl = ttl + options.stale_while_revalidate.unwrap_or_default();
                self.ttl_index.write().schedule(key.to_string(), total_ttl);
            }
        }

        // Update tag index
        for tag in &options.tags {
            self.tag_index
                .entry(tag.clone())
                .or_insert_with(HashSet::new)
                .insert(key.to_string());
        }

        self.data.insert(key.to_string(), entry);
        self.stats.write().writes += 1;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        if self.data.contains_key(key) {
            self.remove_entry(key);
            self.stats.write().deletes += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        match self.data.get(key) {
            Some(entry) => Ok(!entry.is_expired() || entry.is_stale()),
            None => Ok(false),
        }
    }

    async fn delete_many(&self, keys: &[&str]) -> Result<u64> {
        let mut count = 0;
        for key in keys {
            if self.delete(key).await? {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn get_many(&self, keys: &[&str]) -> Result<Vec<Option<CacheEntry<Vec<u8>>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.get(key).await?);
        }
        Ok(results)
    }

    async fn set_many(&self, entries: &[(&str, Vec<u8>, &CacheOptions)]) -> Result<()> {
        for (key, value, options) in entries {
            self.set(key, value.clone(), options).await?;
        }
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        self.data.clear();
        self.tag_index.clear();
        *self.ttl_index.write() = TtlIndex::new(Duration::from_secs(1), self.config.max_ttl);
        Ok(())
    }

    async fn stats(&self) -> Result<CacheStats> {
        let stats = self.stats.read();
        Ok(CacheStats {
            hits: stats.hits,
            misses: stats.misses,
            stale_hits: stats.stale_hits,
            writes: stats.writes,
            deletes: stats.deletes,
            evictions: stats.evictions,
            size: self.data.len(),
            memory_bytes: self.memory_usage(),
        })
    }

    async fn len(&self) -> Result<usize> {
        Ok(self.data.len())
    }
}



#[async_trait]
impl TaggableBackend for MemoryBackend {
    async fn get_by_tag(&self, tag: &str) -> Result<Vec<String>> {
        if let Some(keys) = self.tag_index.get(tag) {
             Ok(keys.iter().cloned().collect())
        } else {
             Ok(Vec::new())
        }
    }

    async fn delete_by_tag(&self, tag: &str) -> Result<u64> {
        // We get the keys and remove the tag entry first
        if let Some((_, keys)) = self.tag_index.remove(tag) {
             let mut count = 0;
             for key in keys {
                 // Check if key exists (it might have been evicted)
                 if self.data.contains_key(&key) {
                     self.remove_entry(&key);
                     self.stats.write().deletes += 1;
                     count += 1;
                 }
             }
             Ok(count)
        } else {
             Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_get_set() {
        let backend = MemoryBackend::new(MemoryConfig::default());

        let options = CacheOptions {
            ttl: Some(Duration::from_secs(60)),
            ..Default::default()
        };

        backend
            .set("key1", b"value1".to_vec(), &options)
            .await
            .unwrap();

        let result = backend.get("key1").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().value, b"value1".to_vec());
    }

    #[tokio::test]
    async fn test_delete() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let options = CacheOptions::default();

        backend
            .set("key1", b"value1".to_vec(), &options)
            .await
            .unwrap();
        assert!(backend.exists("key1").await.unwrap());

        let deleted = backend.delete("key1").await.unwrap();
        assert!(deleted);
        assert!(!backend.exists("key1").await.unwrap());
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let result = backend.get("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let options = CacheOptions::default();

        backend
            .set("key1", b"value1".to_vec(), &options)
            .await
            .unwrap();
        backend
            .set("key2", b"value2".to_vec(), &options)
            .await
            .unwrap();

        assert_eq!(backend.len().await.unwrap(), 2);

        backend.clear().await.unwrap();
        assert_eq!(backend.len().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_stats() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let options = CacheOptions::default();

        backend
            .set("key1", b"value1".to_vec(), &options)
            .await
            .unwrap();
        backend.get("key1").await.unwrap();
        backend.get("nonexistent").await.unwrap();

        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.writes, 1);
    }

    #[tokio::test]
    async fn test_capacity_eviction() {
        let config = MemoryConfig {
            max_capacity: 2,
            ..Default::default()
        };
        let backend = MemoryBackend::new(config);
        let options = CacheOptions::default();

        backend
            .set("key1", b"value1".to_vec(), &options)
            .await
            .unwrap();
        backend
            .set("key2", b"value2".to_vec(), &options)
            .await
            .unwrap();
        backend
            .set("key3", b"value3".to_vec(), &options)
            .await
            .unwrap();

        // Should have evicted one entry
        assert!(backend.len().await.unwrap() <= 2);
    }

    #[tokio::test]
    async fn test_get_many() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let options = CacheOptions::default();

        backend
            .set("key1", b"value1".to_vec(), &options)
            .await
            .unwrap();
        backend
            .set("key2", b"value2".to_vec(), &options)
            .await
            .unwrap();

        let results = backend.get_many(&["key1", "key2", "key3"]).await.unwrap();
        assert_eq!(results.len(), 3);
        assert!(results[0].is_some());
        assert!(results[1].is_some());
        assert!(results[2].is_none());
    }
}

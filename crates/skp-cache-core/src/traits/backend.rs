//! Cache backend trait

use async_trait::async_trait;
use crate::{CacheEntry, CacheError, CacheOptions, CacheStats};

/// Core trait for all cache storage backends
///
/// This trait defines the operations that any cache backend must support.
/// Implementations include in-memory caches, Redis, and multi-tier backends.
#[async_trait]
pub trait CacheBackend: Send + Sync + 'static {
    /// Get a value from the cache
    ///
    /// Returns `None` if the key doesn't exist or has expired.
    async fn get(&self, key: &str) -> Result<Option<CacheEntry<Vec<u8>>>, CacheError>;

    /// Set a value in the cache
    async fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        options: &CacheOptions,
    ) -> Result<(), CacheError>;

    /// Delete a key from the cache
    ///
    /// Returns `true` if the key existed and was deleted.
    async fn delete(&self, key: &str) -> Result<bool, CacheError>;

    /// Check if a key exists in the cache
    async fn exists(&self, key: &str) -> Result<bool, CacheError>;

    /// Delete multiple keys
    ///
    /// Returns the number of keys that were deleted.
    async fn delete_many(&self, keys: &[&str]) -> Result<u64, CacheError>;

    /// Get multiple keys at once
    ///
    /// Returns a vector of results in the same order as the input keys.
    async fn get_many(
        &self,
        keys: &[&str],
    ) -> Result<Vec<Option<CacheEntry<Vec<u8>>>>, CacheError>;

    /// Set multiple entries at once
    async fn set_many(
        &self,
        entries: &[(&str, Vec<u8>, &CacheOptions)],
    ) -> Result<(), CacheError>;

    /// Clear all entries from the cache
    async fn clear(&self) -> Result<(), CacheError>;

    /// Get cache statistics
    async fn stats(&self) -> Result<CacheStats, CacheError>;

    /// Get the number of entries in the cache
    async fn len(&self) -> Result<usize, CacheError>;

    /// Check if the cache is empty
    async fn is_empty(&self) -> Result<bool, CacheError> {
        Ok(self.len().await? == 0)
    }
}

/// Extended trait for backends that support tag-based operations
#[async_trait]
pub trait TaggableBackend: CacheBackend {
    /// Get all keys with a specific tag
    async fn get_by_tag(&self, tag: &str) -> Result<Vec<String>, CacheError>;

    /// Delete all entries with a specific tag
    async fn delete_by_tag(&self, tag: &str) -> Result<u64, CacheError>;
}

/// Extended trait for distributed backends
#[async_trait]
pub trait DistributedBackend: CacheBackend {
    /// Acquire a distributed lock
    async fn acquire_lock(&self, key: &str, ttl: std::time::Duration) -> Result<String, CacheError>;

    /// Release a distributed lock
    async fn release_lock(&self, key: &str, token: &str) -> Result<bool, CacheError>;

    /// Publish an invalidation message
    async fn publish_invalidation(&self, keys: &[&str]) -> Result<(), CacheError>;

    /// Subscribe to invalidation messages
    async fn subscribe_invalidations(&self) -> Result<(), CacheError>;
}

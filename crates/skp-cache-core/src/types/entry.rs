//! Cache entry type

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// A cached entry with full metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    /// The cached value
    pub value: T,
    /// When the entry was created
    pub created_at: SystemTime,
    /// When the entry was last accessed
    pub last_accessed: SystemTime,
    /// Number of times accessed
    pub access_count: u64,
    /// Time-to-live
    pub ttl: Option<Duration>,
    /// Stale-while-revalidate duration
    pub stale_while_revalidate: Option<Duration>,
    /// Associated tags
    pub tags: Vec<String>,
    /// Dependency keys
    pub dependencies: Vec<String>,
    /// Computation cost
    pub cost: u64,
    /// Size in bytes
    pub size: usize,
    /// ETag for HTTP caching
    pub etag: Option<String>,
    /// Version for optimistic concurrency
    pub version: u64,
}

impl<T> CacheEntry<T> {
    /// Create a new cache entry
    pub fn new(value: T, size: usize) -> Self {
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
            size,
            etag: None,
            version: 0,
        }
    }

    /// Create entry with TTL
    pub fn with_ttl(value: T, size: usize, ttl: Duration) -> Self {
        let mut entry = Self::new(value, size);
        entry.ttl = Some(ttl);
        entry
    }

    /// Check if entry has expired
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl {
            if let Ok(elapsed) = self.created_at.elapsed() {
                return elapsed > ttl;
            }
        }
        false
    }

    /// Check if entry is stale but still usable
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

    /// Get remaining TTL
    pub fn ttl_remaining(&self) -> Option<Duration> {
        self.ttl.and_then(|ttl| {
            self.created_at
                .elapsed()
                .ok()
                .and_then(|elapsed| ttl.checked_sub(elapsed))
        })
    }

    /// Get age of the entry
    pub fn age(&self) -> Duration {
        self.created_at.elapsed().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_entry() {
        let entry = CacheEntry::new("test".to_string(), 4);
        assert_eq!(entry.value, "test");
        assert_eq!(entry.access_count, 0);
        assert!(!entry.is_expired());
        assert!(!entry.is_stale());
    }

    #[test]
    fn test_entry_without_ttl_never_expires() {
        let entry = CacheEntry::new("test".to_string(), 4);
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_entry_with_ttl() {
        let entry = CacheEntry::with_ttl("test".to_string(), 4, Duration::from_secs(60));
        assert!(!entry.is_expired());
        assert!(entry.ttl_remaining().is_some());
    }
}

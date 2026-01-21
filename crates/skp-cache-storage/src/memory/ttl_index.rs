//! TTL-based expiration index for efficient expiration

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Time-wheel based TTL index for O(1) expiration lookups
///
/// Instead of scanning all entries to find expired ones,
/// this maintains buckets of keys organized by expiration time.
pub struct TtlIndex {
    /// Tick duration (bucket resolution)
    tick_duration: Duration,
    /// Buckets of keys by expiration slot
    buckets: Vec<HashSet<String>>,
    /// Current bucket index
    current: usize,
    /// Map of key -> bucket index for O(1) removal
    key_to_bucket: HashMap<String, usize>,
    /// Last tick time
    last_tick: Instant,
}

impl TtlIndex {
    /// Create a new TTL index
    ///
    /// # Arguments
    /// * `tick_duration` - Resolution of each time bucket (e.g., 1 second)
    /// * `max_ttl` - Maximum TTL to support (determines number of buckets)
    pub fn new(tick_duration: Duration, max_ttl: Duration) -> Self {
        let tick_secs = tick_duration.as_secs().max(1);
        let max_secs = max_ttl.as_secs();
        let num_buckets = ((max_secs / tick_secs) as usize + 1).max(60);

        Self {
            tick_duration,
            buckets: vec![HashSet::new(); num_buckets],
            current: 0,
            key_to_bucket: HashMap::new(),
            last_tick: Instant::now(),
        }
    }

    /// Schedule a key for expiration after `ttl`
    pub fn schedule(&mut self, key: String, ttl: Duration) {
        // Remove from old bucket if exists
        self.remove(&key);

        let tick_secs = self.tick_duration.as_secs().max(1);
        let ticks = (ttl.as_secs() / tick_secs) as usize;
        let bucket_idx = (self.current + ticks + 1) % self.buckets.len();

        self.buckets[bucket_idx].insert(key.clone());
        self.key_to_bucket.insert(key, bucket_idx);
    }

    /// Remove a key from the index
    pub fn remove(&mut self, key: &str) {
        if let Some(bucket_idx) = self.key_to_bucket.remove(key) {
            self.buckets[bucket_idx].remove(key);
        }
    }

    /// Check if a key is scheduled
    pub fn contains(&self, key: &str) -> bool {
        self.key_to_bucket.contains_key(key)
    }

    /// Advance the wheel and return expired keys
    pub fn tick(&mut self) -> Vec<String> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_tick);
        let tick_secs = self.tick_duration.as_secs().max(1);
        let ticks_to_advance = (elapsed.as_secs() / tick_secs) as usize;

        if ticks_to_advance == 0 {
            return Vec::new();
        }

        let mut expired = Vec::new();

        // Advance through buckets, collecting expired keys
        for _ in 0..ticks_to_advance.min(self.buckets.len()) {
            self.current = (self.current + 1) % self.buckets.len();
            let bucket_expired: Vec<String> = self.buckets[self.current].drain().collect();

            for key in &bucket_expired {
                self.key_to_bucket.remove(key);
            }

            expired.extend(bucket_expired);
        }

        self.last_tick = now;
        expired
    }

    /// Get the number of scheduled keys
    pub fn len(&self) -> usize {
        self.key_to_bucket.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.key_to_bucket.is_empty()
    }

    /// Clear all scheduled keys
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        self.key_to_bucket.clear();
    }
}

impl Default for TtlIndex {
    fn default() -> Self {
        Self::new(Duration::from_secs(1), Duration::from_secs(86400))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_and_remove() {
        let mut index = TtlIndex::new(Duration::from_secs(1), Duration::from_secs(60));

        index.schedule("key1".to_string(), Duration::from_secs(10));
        assert!(index.contains("key1"));
        assert_eq!(index.len(), 1);

        index.remove("key1");
        assert!(!index.contains("key1"));
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_clear() {
        let mut index = TtlIndex::new(Duration::from_secs(1), Duration::from_secs(60));

        index.schedule("key1".to_string(), Duration::from_secs(10));
        index.schedule("key2".to_string(), Duration::from_secs(20));
        assert_eq!(index.len(), 2);

        index.clear();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_reschedule() {
        let mut index = TtlIndex::new(Duration::from_secs(1), Duration::from_secs(60));

        index.schedule("key1".to_string(), Duration::from_secs(10));
        index.schedule("key1".to_string(), Duration::from_secs(20));

        // Should only be in one bucket
        assert_eq!(index.len(), 1);
    }
}

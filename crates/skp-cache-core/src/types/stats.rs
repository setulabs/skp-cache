//! Cache statistics

/// Statistics for cache operations
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of stale hits (served stale while revalidating)
    pub stale_hits: u64,
    /// Number of write operations
    pub writes: u64,
    /// Number of delete operations
    pub deletes: u64,
    /// Number of evictions
    pub evictions: u64,
    /// Current number of entries
    pub size: usize,
    /// Approximate memory usage in bytes
    pub memory_bytes: usize,
}

impl CacheStats {
    /// Calculate hit ratio (0.0 to 1.0)
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Calculate miss ratio (0.0 to 1.0)
    pub fn miss_ratio(&self) -> f64 {
        1.0 - self.hit_ratio()
    }

    /// Total requests (hits + misses)
    pub fn total_requests(&self) -> u64 {
        self.hits + self.misses
    }

    /// Merge stats from another instance
    pub fn merge(&mut self, other: &CacheStats) {
        self.hits += other.hits;
        self.misses += other.misses;
        self.stale_hits += other.stale_hits;
        self.writes += other.writes;
        self.deletes += other.deletes;
        self.evictions += other.evictions;
        self.size = other.size; // Use latest size
        self.memory_bytes = other.memory_bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_stats() {
        let stats = CacheStats::default();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_ratio(), 0.0);
    }

    #[test]
    fn test_hit_ratio() {
        let stats = CacheStats {
            hits: 80,
            misses: 20,
            ..Default::default()
        };
        assert!((stats.hit_ratio() - 0.8).abs() < f64::EPSILON);
        assert!((stats.miss_ratio() - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_total_requests() {
        let stats = CacheStats {
            hits: 100,
            misses: 50,
            ..Default::default()
        };
        assert_eq!(stats.total_requests(), 150);
    }
}

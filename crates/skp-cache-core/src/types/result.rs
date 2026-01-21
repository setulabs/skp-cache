//! Cache result type

use super::entry::CacheEntry;

/// Result of a cache lookup operation
#[derive(Debug, Clone)]
pub enum CacheResult<T> {
    /// Fresh cache hit
    Hit(CacheEntry<T>),
    /// Stale but usable (within stale-while-revalidate window)
    Stale(CacheEntry<T>),
    /// Cache miss
    Miss,
    /// Negative cache hit (known missing/not found)
    NegativeHit,
}

impl<T> CacheResult<T> {
    /// Check if this is a fresh hit
    pub fn is_hit(&self) -> bool {
        matches!(self, CacheResult::Hit(_))
    }

    /// Check if this is a miss
    pub fn is_miss(&self) -> bool {
        matches!(self, CacheResult::Miss)
    }

    /// Check if result is usable (hit or stale)
    pub fn is_usable(&self) -> bool {
        matches!(self, CacheResult::Hit(_) | CacheResult::Stale(_))
    }

    /// Check if stale (needs revalidation)
    pub fn is_stale(&self) -> bool {
        matches!(self, CacheResult::Stale(_))
    }

    /// Extract the value, consuming the result
    pub fn value(self) -> Option<T> {
        match self {
            CacheResult::Hit(entry) | CacheResult::Stale(entry) => Some(entry.value),
            _ => None,
        }
    }

    /// Extract the full entry, consuming the result
    pub fn entry(self) -> Option<CacheEntry<T>> {
        match self {
            CacheResult::Hit(entry) | CacheResult::Stale(entry) => Some(entry),
            _ => None,
        }
    }

    /// Map the value if present
    pub fn map<U, F>(self, f: F) -> CacheResult<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            CacheResult::Hit(entry) => CacheResult::Hit(CacheEntry {
                value: f(entry.value),
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
            }),
            CacheResult::Stale(entry) => CacheResult::Stale(CacheEntry {
                value: f(entry.value),
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
            }),
            CacheResult::Miss => CacheResult::Miss,
            CacheResult::NegativeHit => CacheResult::NegativeHit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hit() {
        let entry = CacheEntry::new(42, 4);
        let result = CacheResult::Hit(entry);

        assert!(result.is_hit());
        assert!(result.is_usable());
        assert!(!result.is_miss());
        assert!(!result.is_stale());
    }

    #[test]
    fn test_miss() {
        let result: CacheResult<i32> = CacheResult::Miss;

        assert!(!result.is_hit());
        assert!(!result.is_usable());
        assert!(result.is_miss());
        assert!(result.value().is_none());
    }

    #[test]
    fn test_value_extraction() {
        let entry = CacheEntry::new(42, 4);
        let result = CacheResult::Hit(entry);

        assert_eq!(result.value(), Some(42));
    }

    #[test]
    fn test_map() {
        let entry = CacheEntry::new(42, 4);
        let result = CacheResult::Hit(entry);

        let mapped = result.map(|v| v * 2);
        assert_eq!(mapped.value(), Some(84));
    }
}

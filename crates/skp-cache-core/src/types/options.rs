//! Cache options and builder

use std::time::Duration;

/// Configuration options for a cache entry
#[derive(Debug, Clone, Default)]
pub struct CacheOptions {
    /// Time-to-live
    pub ttl: Option<Duration>,
    /// Stale-while-revalidate window
    pub stale_while_revalidate: Option<Duration>,
    /// Tags for invalidation
    pub tags: Vec<String>,
    /// Dependencies (keys this entry depends on)
    pub dependencies: Vec<String>,
    /// Computation cost (for cost-aware eviction)
    pub cost: Option<u64>,
    /// Enable early refresh
    pub early_refresh: bool,
    /// Enable request coalescing
    pub coalesce: bool,
    /// ETag for HTTP caching
    pub etag: Option<String>,
    /// Mark as negative cache entry
    pub negative: bool,
    /// Conditional set: only if version matches
    pub if_version: Option<u64>,
}

/// Builder for CacheOptions with fluent API
#[derive(Debug, Clone, Default)]
pub struct CacheOpts(CacheOptions);

impl CacheOpts {
    /// Create new options builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set TTL
    pub fn ttl(mut self, duration: Duration) -> Self {
        self.0.ttl = Some(duration);
        self
    }

    /// Set TTL in seconds
    pub fn ttl_secs(self, seconds: u64) -> Self {
        self.ttl(Duration::from_secs(seconds))
    }

    /// Set TTL in minutes
    pub fn ttl_mins(self, minutes: u64) -> Self {
        self.ttl(Duration::from_secs(minutes * 60))
    }

    /// Set stale-while-revalidate window
    pub fn swr(mut self, duration: Duration) -> Self {
        self.0.stale_while_revalidate = Some(duration);
        self
    }

    /// Set stale-while-revalidate in seconds
    pub fn swr_secs(self, seconds: u64) -> Self {
        self.swr(Duration::from_secs(seconds))
    }

    /// Add multiple tags
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

    /// Add dependencies
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

impl From<Duration> for CacheOptions {
    fn from(ttl: Duration) -> Self {
        CacheOptions {
            ttl: Some(ttl),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let opts = CacheOpts::new().build();
        assert!(opts.ttl.is_none());
        assert!(opts.tags.is_empty());
    }

    #[test]
    fn test_builder_fluent() {
        let opts = CacheOpts::new()
            .ttl_secs(60)
            .swr_secs(30)
            .tags(["tag1", "tag2"])
            .tag("tag3")
            .cost(100)
            .early_refresh()
            .build();

        assert_eq!(opts.ttl, Some(Duration::from_secs(60)));
        assert_eq!(opts.stale_while_revalidate, Some(Duration::from_secs(30)));
        assert_eq!(opts.tags, vec!["tag1", "tag2", "tag3"]);
        assert_eq!(opts.cost, Some(100));
        assert!(opts.early_refresh);
    }

    #[test]
    fn test_from_duration() {
        let opts: CacheOptions = Duration::from_secs(300).into();
        assert_eq!(opts.ttl, Some(Duration::from_secs(300)));
    }
}

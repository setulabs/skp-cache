//! Configuration for Redis backend

use std::time::Duration;

/// Configuration for Redis backend connection and behavior
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis connection URL (e.g., "redis://127.0.0.1:6379")
    pub url: String,
    
    /// Connection pool size
    pub pool_size: u32,
    
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Optional key prefix for all keys (e.g., "myapp")
    pub key_prefix: Option<String>,
    
    /// Channel name for invalidation pub/sub
    pub invalidation_channel: String,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_size: 10,
            connection_timeout: Duration::from_secs(5),
            key_prefix: Some("skp".to_string()),
            invalidation_channel: "skp:invalidation".to_string(),
        }
    }
}

impl RedisConfig {
    /// Create new config with URL
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }
    
    /// Set pool size
    pub fn pool_size(mut self, size: u32) -> Self {
        self.pool_size = size;
        self
    }
    
    /// Set key prefix
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = Some(prefix.into());
        self
    }
}

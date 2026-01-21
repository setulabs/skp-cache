use crate::CacheControl;
use http::StatusCode;
use std::time::Duration;

/// Configuration for HTTP caching behavior
#[derive(Debug, Clone, Default)]
pub struct HttpCachePolicy {
    /// Ignore Cache-Control from upstream?
    pub ignore_upstream_cache_control: bool,
    /// Default TTL if none specified
    pub default_ttl: Option<Duration>,
    /// Header names to include in Vary key
    pub vary_headers: Vec<String>,
    /// Bypass cache completely?
    pub bypass: bool,
    /// Tags to apply to cached entries (for invalidation)
    pub tags: Vec<String>,
}

impl HttpCachePolicy {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = Some(ttl);
        self
    }
    
    pub fn vary_by(mut self, headers: &[&str]) -> Self {
        self.vary_headers.extend(headers.iter().map(|s| s.to_string()));
        self
    }
    
    /// Calculate effective TTL based on policy and response headers
    pub fn effective_ttl(&self, cc: &CacheControl) -> Option<Duration> {
        if self.ignore_upstream_cache_control {
             return self.default_ttl;
        }
        
        // Priority: s-maxage > max-age > default
        if let Some(ttl) = cc.s_maxage {
             return Some(ttl);
        }
        if let Some(ttl) = cc.max_age {
             return Some(ttl);
        }
        
        self.default_ttl
    }
}

/// Determine if a response is cacheable
pub fn is_cacheable(status: StatusCode, cc: &CacheControl) -> bool {
    // Only cache 200 OK for now
    if status != StatusCode::OK {
         return false;
    }
    
    if cc.no_store { return false; }
    
    // Assuming shared cache semantics by default
    if cc.private { 
        return false; 
    }
    
    true
}

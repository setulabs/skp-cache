use skp_cache_core::{
    CacheBackend, CacheKey, CacheMetrics, CacheOptions, CacheResult, DependencyBackend,
    Result, Serializer, TaggableBackend,
};
use crate::CacheManager;

/// A logical grouping of cache entries with shared namespace and invalidation
pub struct CacheGroup<'a, B, S, M>
where
    B: CacheBackend + DependencyBackend,
    S: Serializer,
    M: CacheMetrics,
{
    manager: &'a CacheManager<B, S, M>,
    namespace: String,
}

impl<'a, B, S, M> CacheGroup<'a, B, S, M>
where
    B: CacheBackend + DependencyBackend,
    S: Serializer,
    M: CacheMetrics,
{
    pub(crate) fn new(manager: &'a CacheManager<B, S, M>, namespace: String) -> Self {
        Self { manager, namespace }
    }

    /// Get the fully qualified key for this group
    pub fn group_key(&self, key: &str) -> String {
        format!("{}:{}", self.namespace, key)
    }

    /// Tag used for invalidation of entire group
    pub fn group_tag(&self) -> String {
        format!("group:{}", self.namespace)
    }

    /// Get a value from the group
    pub async fn get<T>(&self, key: impl CacheKey) -> Result<CacheResult<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        // We wrap the key to include namespace
        let composite_key = self.group_key(&key.full_key());
        self.manager.get(composite_key).await
    }

    /// Set a value in the group
    pub async fn set<T>(
        &self,
        key: impl CacheKey,
        value: T,
        options: impl Into<CacheOptions>,
    ) -> Result<()>
    where
        T: serde::Serialize,
    {
        let composite_key = self.group_key(&key.full_key());
        let mut opts = options.into();
        
        // Auto-add group tag
        let tag = self.group_tag();
        opts.tags.push(tag);

        self.manager.set(composite_key, value, opts).await
    }
    
    /// Delete a key from the group
    pub async fn delete(&self, key: impl CacheKey) -> Result<bool> {
        let composite_key = self.group_key(&key.full_key());
        self.manager.delete(composite_key).await
    }
}

// Separate impl for TaggableBackend requirements
impl<'a, B, S, M> CacheGroup<'a, B, S, M>
where
    B: CacheBackend + DependencyBackend + TaggableBackend,
    S: Serializer,
    M: CacheMetrics,
{
    /// Invalidate all entries in this group
    pub async fn invalidate_all(&self) -> Result<u64> {
         let tag = self.group_tag();
         self.manager.delete_by_tag(&tag).await
    }
    
    /// Get all keys in this group
    pub async fn keys(&self) -> Result<Vec<String>> {
        let tag = self.group_tag();
        self.manager.get_keys_by_tag(&tag).await
    }
}

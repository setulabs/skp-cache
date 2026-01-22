use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;
use std::sync::Arc;

use skp_cache_core::{
    CacheBackend, CacheError, CacheKey, CacheMetrics, CacheOptions, CacheResult, DependencyBackend,
    Result, Serializer,
};

use crate::CacheManager;

/// Trait for automatic data loading on cache miss
#[async_trait]
pub trait Loader<K, V>: Send + Sync + 'static {
    /// Load data for the given key
    async fn load(&self, key: &K) -> Result<Option<V>>;
}

/// A cache wrapper that automatically loads data on miss
pub struct ReadThroughCache<B, S, M, K, V, L> 
where
    B: CacheBackend + DependencyBackend,
    S: Serializer,
    M: CacheMetrics,
{
    manager: CacheManager<B, S, M>,
    loader: Arc<L>,
    options: CacheOptions,
    _phantom: PhantomData<(K, V)>,
}

impl<B, S, M, K, V, L> ReadThroughCache<B, S, M, K, V, L>
where
    B: CacheBackend + DependencyBackend,
    S: Serializer,
    M: CacheMetrics,
    K: CacheKey + Clone + Send + Sync + 'static,
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
    L: Loader<K, V>,
{
    /// Create a new ReadThroughCache
    pub fn new(manager: CacheManager<B, S, M>, loader: L, options: CacheOptions) -> Self {
        Self {
            manager,
            loader: Arc::new(loader),
            options,
            _phantom: PhantomData,
        }
    }

    /// Get value from cache, or load it automatically if missing
    pub async fn get(&self, key: K) -> Result<Option<V>> {
        // 1. Try to get from cache
        match self.manager.get::<V>(key.clone()).await? {
            CacheResult::Hit(entry) => Ok(Some(entry.value)),
            CacheResult::Stale(entry) => {
                // If stale, serve it but trigger background refresh
                self.refresh_background(key.clone());
                Ok(Some(entry.value))
            }
            CacheResult::Miss | CacheResult::NegativeHit => {
                // 2. Load from source (coalesced via get_or_compute)
                let loader = self.loader.clone();
                let key_clone = key.clone();
                
                let result = self.manager.get_or_compute(
                    key,
                    move || async move {
                        loader.load(&key_clone).await?
                             .ok_or_else(|| CacheError::NotFound("Loader returned None".into()))
                    },
                    Some(self.options.clone())
                ).await;

                match result {
                    Ok(CacheResult::Hit(entry)) => Ok(Some(entry.value)),
                    Ok(CacheResult::Stale(entry)) => Ok(Some(entry.value)),
                    Err(CacheError::NotFound(_)) => Ok(None),
                    Err(e) => Err(e),
                    _ => Ok(None),
                }
            }
        }
    }

    /// Force refresh a key using the loader
    pub async fn refresh(&self, key: K) -> Result<()> {
        if let Some(val) = self.loader.load(&key).await? {
            self.manager.set(key, val, self.options.clone()).await?;
        }
        Ok(())
    }

    /// Trigger background refresh
    fn refresh_background(&self, key: K) {
        let loader = self.loader.clone();
        let manager = self.manager.clone();
        let options = self.options.clone();

        tokio::spawn(async move {
            if let Ok(Some(val)) = loader.load(&key).await {
                let _ = manager.set(key, val, options).await;
            }
        });
    }
}

// Extension trait for CacheManager convenience
pub trait CacheManagerReadThroughExt<B, S, M> {
    fn read_through<K, V, L>(
        self, 
        loader: L, 
        options: CacheOptions
    ) -> ReadThroughCache<B, S, M, K, V, L>
    where
        B: CacheBackend + DependencyBackend,
        S: Serializer,
        M: CacheMetrics,
        L: Loader<K, V>,
        // Explicit bounds required for ReadThroughCache construction
        K: CacheKey + Clone + Send + Sync + 'static,
        V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static;
}

impl<B, S, M> CacheManagerReadThroughExt<B, S, M> for CacheManager<B, S, M>
where
    B: CacheBackend + DependencyBackend,
    S: Serializer,
    M: CacheMetrics,
{
    fn read_through<K, V, L>(
        self, 
        loader: L, 
        options: CacheOptions
    ) -> ReadThroughCache<B, S, M, K, V, L>
    where
        L: Loader<K, V>,
        K: CacheKey + Clone + Send + Sync + 'static,
        V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
    {
        ReadThroughCache::new(self, loader, options)
    }
}

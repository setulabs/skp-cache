use axum::{
    extract::{FromRequestParts, FromRef},
    http::request::Parts,
};
use skp_cache_core::{CacheBackend, CacheMetrics, Serializer, DependencyBackend};
use skp_cache::CacheManager;

/// Extractor to access CacheManager from Axum handlers
pub struct Cache<B, S, M>(pub CacheManager<B, S, M>)
where
    B: CacheBackend + DependencyBackend,
    S: Serializer,
    M: CacheMetrics;

impl<State, B, S, M> FromRequestParts<State> for Cache<B, S, M>
where
    B: CacheBackend + DependencyBackend + Clone + Send + Sync + 'static,
    S: Serializer + Clone + Send + Sync + 'static,
    M: CacheMetrics + Clone + Send + Sync + 'static,
    State: Send + Sync,
    CacheManager<B, S, M>: FromRef<State>,
{
    type Rejection = std::convert::Infallible;
    
    async fn from_request_parts(_parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        let cache = CacheManager::<B, S, M>::from_ref(state);
        Ok(Cache(cache))
    }
}

impl<B, S, M> std::ops::Deref for Cache<B, S, M> 
where
    B: CacheBackend + DependencyBackend,
    S: Serializer,
    M: CacheMetrics,
{
    type Target = CacheManager<B, S, M>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

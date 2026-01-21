use tower::Layer;
use skp_cache_core::{CacheBackend, CacheMetrics, Serializer, DependencyBackend};
use skp_cache::CacheManager;
use crate::middleware::CacheMiddleware;

#[derive(Clone)]
pub struct CacheLayer<B, S, M>
where
    B: CacheBackend + DependencyBackend,
    S: Serializer,
    M: CacheMetrics,
{
    pub manager: CacheManager<B, S, M>,
}

impl<B, S, M> CacheLayer<B, S, M>
where
    B: CacheBackend + DependencyBackend,
    S: Serializer,
    M: CacheMetrics,
{
    pub fn new(manager: CacheManager<B, S, M>) -> Self {
        Self { manager }
    }
}

impl<S, B, Ser, M> Layer<S> for CacheLayer<B, Ser, M>
where
    B: CacheBackend + DependencyBackend + Clone + Send + Sync + 'static,
    Ser: Serializer + Clone + Send + Sync + 'static,
    M: CacheMetrics + Clone + Send + Sync + 'static,
{
    type Service = CacheMiddleware<S, B, Ser, M>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheMiddleware::new(inner, self.manager.clone())
    }
}

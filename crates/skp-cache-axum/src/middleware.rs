use axum::{
    body::{Body},
    http::{Request, Response, Method}, 
};
use tower_service::Service;
use std::task::{Context, Poll};
use std::future::Future;
use std::pin::Pin;
use skp_cache::{CacheManager, CacheResult};
use skp_cache_core::{CacheBackend, CacheMetrics, Serializer, CacheOpts, DependencyBackend};
use skp_cache_http::{CachedResponse, policy};

#[derive(Clone)]
pub struct CacheMiddleware<S, B, Ser, M>
where
    B: CacheBackend + DependencyBackend,
    Ser: Serializer,
    M: CacheMetrics,
{
    inner: S,
    manager: CacheManager<B, Ser, M>,
}

impl<S, B, Ser, M> CacheMiddleware<S, B, Ser, M>
where
    B: CacheBackend + DependencyBackend,
    Ser: Serializer,
    M: CacheMetrics,
{
    pub fn new(inner: S, manager: CacheManager<B, Ser, M>) -> Self {
        Self { inner, manager }
    }
}

impl<S, B, Ser, M> Service<Request<Body>> for CacheMiddleware<S, B, Ser, M>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: CacheBackend + DependencyBackend + Clone + Send + Sync + 'static,
    Ser: Serializer + Send + Sync + 'static,
    M: CacheMetrics + Send + Sync + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let manager = self.manager.clone();

        Box::pin(async move {
            // 1. Only GET/HEAD
            if req.method() != Method::GET && req.method() != Method::HEAD {
                return inner.call(req).await;
            }
            
            // 2. Generate Key (Simple: full URI)
            let key = format!("http:{}", req.uri());
            
            // 3. Check Cache
            if let Ok(CacheResult::Hit(entry)) = manager.get::<CachedResponse>(&key).await {
                let mut res = Response::builder()
                    .status(entry.value.status);
                
                for (k, v) in entry.value.headers {
                     res = res.header(k, v);
                }
                
                res = res.header("x-cache", "HIT");
                
                let body = Body::from(entry.value.body);
                if let Ok(response) = res.body(body) {
                    return Ok(response);
                }
            }
            
            // 4. Cache Miss - Call Inner
            let res = inner.call(req).await?;
            
            // 5. Cache Logic
            let (parts, body) = res.into_parts();
            
            // Read bytes (ignore error for middleware robustness)
            let bytes = match axum::body::to_bytes(body, usize::MAX).await {
                Ok(b) => b,
                Err(_) => return Ok(Response::from_parts(parts, Body::empty())),
            };
            
            // Check Cache-Control
            let cc_header = parts.headers.get("cache-control").and_then(|v| v.to_str().ok()).unwrap_or("");
            let cc = skp_cache_http::CacheControl::parse(cc_header);
            
            if policy::is_cacheable(parts.status, &cc) {
                let cached = CachedResponse::from_parts(parts.status, &parts.headers, bytes.to_vec());
                let ttl = policy::HttpCachePolicy::default().effective_ttl(&cc);
                let mut opts = CacheOpts::new();
                if let Some(t) = ttl {
                    opts = opts.ttl(t);
                }
                
                // Background set
                let manager_bg = manager.clone();
                let key_bg = key.clone();
                let cached_bg = cached.clone();
                let opts_bg = opts.clone();
                
                tokio::spawn(async move {
                    let _ = manager_bg.set(&key_bg, cached_bg, opts_bg).await;
                });
            }
            
            // Reconstruct coverage
            let body = Body::from(bytes);
            let mut res = Response::from_parts(parts, body);
            res.headers_mut().insert("x-cache", "MISS".parse().unwrap());
            Ok(res)
        })
    }
}

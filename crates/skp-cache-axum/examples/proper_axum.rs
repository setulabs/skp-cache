use axum::{
    routing::get,
    Router,
    response::IntoResponse,
};
use skp_cache::{CacheManager, MemoryBackend, MemoryConfig, JsonSerializer, NoopMetrics};
use skp_cache_axum::{CacheLayer, Cache};
use tokio::net::TcpListener;

// Access simplify types
type MyCache = CacheManager<MemoryBackend, JsonSerializer, NoopMetrics>;

#[tokio::main]
async fn main() {
    let backend = MemoryBackend::new(MemoryConfig::default());
    let cache: MyCache = CacheManager::new(backend);

    // Build the application
    // 1. Create Router (starts with state `()`)
    // 2. Call `with_state` to transition to `Router<MyCache>`
    // 3. Add routes (handlers are checked against `MyCache`)
    // 4. Add CacheLayer
    let app = Router::new()
        .with_state(cache.clone())
        .route("/cached", get(cached_handler))
        .route("/manual", get(manual_handler))
        .layer(CacheLayer::new(cache));

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    
    // Uncomment to run server:
    // axum::serve(listener, app).await.unwrap();
}

/// Automatically cached by middleware if headers allow
async fn cached_handler() -> impl IntoResponse {
    // Return Cache-Control header to enable caching
    ([("cache-control", "max-age=60")], "I am cached automatically!")
}

/// Manual access using the `Cache` extractor
async fn manual_handler(
    // Extractor injects the CacheManager from State
    Cache(cache): Cache<MemoryBackend, JsonSerializer, NoopMetrics>
) -> impl IntoResponse {
    // We can interact with cache directly
    let size = cache.len().await.unwrap_or(0);
    format!("Cache currently holds {} items", size)
}

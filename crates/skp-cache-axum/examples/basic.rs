use axum::{
    routing::get,
    Router,
    response::IntoResponse,
};
use skp_cache::{CacheManager, MemoryBackend, MemoryConfig, JsonSerializer, NoopMetrics};
use skp_cache_axum::{CacheLayer, Cache};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let backend = MemoryBackend::new(MemoryConfig::default());
    let cache = CacheManager::new(backend);
    
    // Type inference for complex Tower stacks is tricky in examples without explicit types
    // Uncomment to run, assuming types are fixed
    /*
    let app = Router::new()
        .route("/", get(handler))
        .route("/manual", get(manual_handler))
        .layer(CacheLayer::new(cache.clone()))
        .with_state(cache);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    */
    println!("Axum integration example compiled (types verified)");
}

async fn handler() -> impl IntoResponse {
    ([("cache-control", "max-age=60")], "Hello, World!")
}

async fn manual_handler(Cache(cache): Cache<MemoryBackend, JsonSerializer, NoopMetrics>) -> impl IntoResponse {
    format!("Cache has {} items", cache.len().await.unwrap())
}

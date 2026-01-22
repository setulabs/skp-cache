use skp_cache::prelude::*;
use skp_cache::TracingMetrics; // Explicit import
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE) // Enable TRACE to see latency logs
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    println!("🔍 Initialized tracing...");

    // 2. Create Cache Manager with TracingMetrics
    // We need to use the builder or constructor that allows custom metrics
    let backend = MemoryBackend::new(MemoryConfig::default());
    
    // Use TracingMetrics adapter
    let metrics = TracingMetrics::new().with_service_name("example-service");
    
    // We use the JSON serializer (default)
    let serializer = JsonSerializer;
    
    let cache = CacheManager::with_serializer_and_metrics(
        backend,
        serializer,
        metrics,
        CacheManagerConfig::default(),
    );

    println!("\n⚡ Setting value...");
    cache.set("user:1", "Alice", CacheOpts::new().ttl_secs(60)).await?;
    
    println!("\n⚡ Getting value (Hit)...");
    // Explicit type annotation <String>
    let val = cache.get::<String>("user:1").await?.value();
    println!("   Got: {:?}", val);
    
    println!("\n⚡ Getting missing value (Miss)...");
    let miss = cache.get::<String>("user:99").await?.value();
    println!("   Got: {:?}", miss);
    
    println!("\n✅ Check your console output for structured logs!");

    Ok(())
}

use skp_cache::prelude::*;


#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Check if we can connect to Redis, otherwise skip
    // Real application would fail here.
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    
    println!("Connecting to Redis at {}", redis_url);
    
    // Create Redis backend configuration
    let config = RedisConfig::new(redis_url)
        .pool_size(5)
        .prefix("example");
        
    // Create Redis backend
    match RedisBackend::new(config).await {
        Ok(backend) => {
            // Create Cache Manager
            let cache = CacheManager::new(backend);
            
            // Set a value
            cache
                .set("hello", &"world".to_string(), CacheOpts::new().ttl_mins(5))
                .await?;
                
            // Get it back
            match cache.get::<String>("hello").await? {
                CacheResult::Hit(entry) => {
                    println!("Hit: {} (tags: {:?})", entry.value, entry.tags);
                }
                CacheResult::Miss => println!("Miss"),
                CacheResult::Stale(entry) => println!("Stale: {}", entry.value),
                CacheResult::NegativeHit => println!("Negative Hit"),
            }
            
            // Tagging example
            cache
                .set(
                    "user:1",
                    &"sachin".to_string(),
                    CacheOpts::new().tags(vec!["users", "admins"]),
                )
                .await?;
                
            // Get by tag
            // Note: CacheManager doesn't expose get_by_tag directly yet?
            // CacheManager currently only proxies `get`, `set`, etc.
            // If I want to use `get_by_tag`, I need to use the backend directly OR
            // CacheManager needs to expose it (Phase 3?).
            // Checking CacheManager...
            // It has `backend()` accessor?
            // If not, I can access backend if I kept a reference?
            
            // Currently CacheManager wraps backend internally.
        },
        Err(e) => {
             eprintln!("Failed to connect to Redis: {}", e);
             println!("Make sure Redis is running at 127.0.0.1:6379 or set REDIS_URL");
        }
    }
    
    Ok(())
}

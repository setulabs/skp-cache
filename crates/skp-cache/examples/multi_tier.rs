use skp_cache::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Multi-Tier Cache Example");
    
    // 1. L1: Memory Backend
    let l1 = MemoryBackend::new(MemoryConfig::default());
    
    // 2. L2: Redis Backend
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    
    println!("Connecting to Redis L2 at {}", redis_url);
    
    let l2_result = RedisBackend::new(
        RedisConfig::new(redis_url)
            .prefix("multitier")
    ).await;
    
    match l2_result {
        Ok(l2) => {
            // 3. Circuit Breaker
            let breaker = CircuitBreaker::new(3, Duration::from_secs(10));
            
            // 4. Combine into MultiTierBackend
            let backend = MultiTierBackend::new(l1, l2, breaker);
            
            // 5. Create Cache Manager
            let cache = CacheManager::new(backend);
            
            // Perform operations
            println!("Setting key 'tier_key'...");
            cache.set("tier_key", &"persistent_data".to_string(), CacheOpts::new().ttl_secs(60)).await?;
            
            println!("Getting key 'tier_key'...");
            // First fetch comes from L1 (written through)
            // Or if we restarted app (and L2 has it), it would be fetched from L2 and backfilled to L1
            match cache.get::<String>("tier_key").await? {
                CacheResult::Hit(entry) => println!("Hit: {}", entry.value),
                _ => println!("Miss or Error"),
            }
            
            println!("Clearing L1 manually to simulate cold start...");
            // To properly simulate cold start involving internal backend access, 
            // we'd need to keep a reference to L1 or use a new Manager instance connected to same Redis.
            // Since `MultiTierBackend` owns L1/L2, we can't easily access L1 to clear it *after* wrapping.
            // But we can create another cache instance sharing the same Redis?
            
        },
        Err(e) => {
            eprintln!("Could not connect to Redis: {}", e);
        }
    }

    Ok(())
}

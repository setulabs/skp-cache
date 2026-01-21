use skp_cache::prelude::*;
use skp_cache_core::{CacheOpts};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let backend = MemoryBackend::new(MemoryConfig::default());
    let cache = CacheManager::new(backend);
    
    let key = "swr_key";
    
    // 1. Set entry with SWR
    // TTL = 1s, SWR = 5s (Total 6s validity, 1-6s is Stale window)
    let opts = CacheOpts::new()
        .ttl_secs(1)
        .swr_secs(5);
        
    cache.set(key, &"initial", opts.clone()).await?;
    println!("Set 'initial'. Waiting 2s (TTL expired, SWR active)...");
    sleep(Duration::from_secs(2)).await;
    
    // 2. get_or_compute
    // Should return STALE "initial" immediately, and trigger background compute
    println!("Requesting key...");
    let start = std::time::Instant::now();
    
    let res = cache.get_or_compute(key, || async {
         println!("  Background: Computing fresh value (taking 500ms)...");
         sleep(Duration::from_millis(500)).await;
         println!("  Background: Compute done.");
         Ok("fresh".to_string())
    }, Some(opts.build())).await?;
    
    let elapsed = start.elapsed();
    println!("Request returned in {:?} (should be instant)", elapsed);
    
    match res {
        CacheResult::Stale(v) => {
            println!("Got Stale value: '{}'", v.value);
            assert_eq!(v.value, "initial");
        },
        CacheResult::Hit(_) => panic!("Expected Stale, got Hit (Fresh)"),
        CacheResult::Miss => panic!("Expected Stale, got Miss"),
        _ => panic!("Unexpected result"),
    }
    
    // 3. Wait for background refresh to complete
    println!("Waiting for background refresh...");
    sleep(Duration::from_secs(1)).await;
    
    // 4. Get again -> Should be fresh
    let res2 = cache.get::<String>(key).await?;
    match res2 {
        CacheResult::Hit(v) => {
            println!("Got Fresh value: '{}'", v.value);
            assert_eq!(v.value, "fresh");
        },
        CacheResult::Stale(_) => panic!("Expected Fresh, got Stale"),
        CacheResult::Miss => panic!("Expected Fresh, got Miss"),
        _ => panic!("Unexpected result"),
    }
    
    println!("SWR Success!");
    Ok(())
}

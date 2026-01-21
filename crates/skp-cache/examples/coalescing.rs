use skp_cache::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // 1. Setup cache
    let backend = MemoryBackend::new(MemoryConfig::default());
    let cache = CacheManager::new(backend);
    
    // 2. Shared counter to track actual computations
    let compute_count = Arc::new(Mutex::new(0));
    
    let mut handles = Vec::new();
    let key = "expensive_data";
    
    println!("Spawning 10 concurrent requests for key '{}'...", key);
    
    // 3. Launch concurrent requests
    for _ in 0..10 {
        let cache = cache.clone();
        let compute_count = compute_count.clone();
        
        handles.push(tokio::spawn(async move {
            let result: CacheResult<String> = cache
                .get_or_compute(key, || async move {
                    // Simulate expensive computation (100ms)
                    sleep(Duration::from_millis(100)).await;
                    
                    let mut count = compute_count.lock().unwrap();
                    *count += 1;
                    println!("Computing... (count: {})", *count);
                    
                    Ok("computed_value".to_string())
                }, None)
                .await
                .unwrap();
                
            match result {
                CacheResult::Hit(v) => {
                     assert_eq!(v.value, "computed_value");
                },
                _ => panic!("Expected hit"),
            }
        }));
    }
    
    // 4. Wait for all to complete
    for h in handles {
        h.await?;
    }
    
    // 5. Verify coalescing
    let total_computations = *compute_count.lock().unwrap();
    println!("Total computations performed: {}", total_computations);
    
    if total_computations != 1 {
        panic!("Coalescing failed! Expected 1 computation, got {}", total_computations);
    } else {
        println!("SUCCESS: Request coalescing worked correctly.");
    }
    
    Ok(())
}

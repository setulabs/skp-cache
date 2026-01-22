//! Request Coalescing (Singleflight) Example
//!
//! Demonstrates how concurrent cache misses for the same key
//! are coalesced into a single computation, preventing stampedes.

use skp_cache::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let backend = MemoryBackend::new(MemoryConfig::default());
    let cache = CacheManager::new(backend);

    println!("=== Request Coalescing Demo ===\n");

    // Track how many times our "expensive" computation runs
    let computation_count = Arc::new(AtomicU32::new(0));

    // Simulate 10 concurrent requests for the same key
    let mut handles = vec![];

    for i in 0..10 {
        let cache = cache.clone();
        let count = computation_count.clone();

        handles.push(tokio::spawn(async move {
            let result = cache
                .get_or_compute(
                    "expensive:report",
                    move || {
                        let count = count.clone();
                        async move {
                            // This "expensive" computation should only run ONCE
                            count.fetch_add(1, Ordering::SeqCst);
                            println!("  → Computing expensive report...");
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            Ok::<_, CacheError>(format!("Report data"))
                        }
                    },
                    Some(CacheOpts::new().ttl_secs(60).into()),
                )
                .await;

            println!("  Request {} completed: {:?}", i, result.is_ok());
        }));
    }

    // Wait for all requests
    for handle in handles {
        handle.await.unwrap();
    }

    let total_computations = computation_count.load(Ordering::SeqCst);
    println!("\n📊 Results:");
    println!("   Total concurrent requests: 10");
    println!("   Actual computations: {}", total_computations);

    if total_computations == 1 {
        println!("\n✅ Coalescing works! Only 1 computation despite 10 requests.");
    } else {
        println!("\n⚠️  Expected 1 computation, got {}", total_computations);
    }

    Ok(())
}

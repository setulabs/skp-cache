//! Stale-While-Revalidate (SWR) Example
//!
//! Demonstrates serving stale data immediately while refreshing
//! in the background, providing optimal latency for users.

use skp_cache::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let backend = MemoryBackend::new(MemoryConfig::default());
    let cache = CacheManager::new(backend);

    println!("=== Stale-While-Revalidate Demo ===\n");

    // Set an entry with short TTL and SWR window
    cache
        .set(
            "dashboard",
            &"Dashboard v1".to_string(),
            CacheOpts::new()
                .ttl_secs(1)  // Fresh for 1 second
                .swr_secs(5), // Stale but usable for 5 more seconds
        )
        .await?;
    println!("✓ Set 'dashboard' with TTL=1s, SWR=5s");

    // Immediately: should be fresh
    match cache.get::<String>("dashboard").await? {
        CacheResult::Hit(entry) => println!("T+0s: HIT (fresh) - {}", entry.value),
        CacheResult::Stale(entry) => println!("T+0s: STALE - {}", entry.value),
        CacheResult::Miss => println!("T+0s: MISS"),
        _ => {}
    }

    // Wait for TTL to expire
    println!("\n⏳ Waiting 2 seconds for TTL to expire...\n");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // After TTL: should be stale (but still returned)
    match cache.get::<String>("dashboard").await? {
        CacheResult::Hit(entry) => println!("T+2s: HIT (fresh) - {}", entry.value),
        CacheResult::Stale(entry) => {
            println!("T+2s: STALE (serving old data) - {}", entry.value);
            println!("       → Background refresh would trigger here");
        }
        CacheResult::Miss => println!("T+2s: MISS"),
        _ => {}
    }

    // Wait for SWR window to expire
    println!("\n⏳ Waiting 5 more seconds for SWR window to expire...\n");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // After SWR: should be gone
    match cache.get::<String>("dashboard").await? {
        CacheResult::Hit(entry) => println!("T+7s: HIT - {}", entry.value),
        CacheResult::Stale(entry) => println!("T+7s: STALE - {}", entry.value),
        CacheResult::Miss => println!("T+7s: MISS (entry fully expired)"),
        _ => {}
    }

    println!("\n✅ SWR behavior demonstrated!");

    Ok(())
}

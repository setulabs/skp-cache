//! Basic example demonstrating skp-cache with memory backend

use skp_cache::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== skp-cache Basic Example ===\n");

    // Create cache with memory backend
    let config = MemoryConfig::default();
    let backend = MemoryBackend::new(config);
    let cache = CacheManager::new(backend);

    // Create a user
    let user = User {
        id: 123,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    // Store in cache with tags
    println!("Storing user in cache...");
    cache
        .set(
            "user:123",
            &user,
            CacheOpts::new()
                .ttl_secs(300)
                .tags(["users", "user:123"]),
        )
        .await?;

    // Retrieve from cache
    println!("Retrieving user from cache...");
    match cache.get::<User>("user:123").await? {
        CacheResult::Hit(entry) => {
            println!("✅ Cache HIT!");
            println!("   User: {} <{}>", entry.value.name, entry.value.email);
            println!("   TTL remaining: {:?}", entry.ttl_remaining());
        }
        CacheResult::Miss => {
            println!("❌ Cache MISS");
        }
        CacheResult::Stale(entry) => {
            println!("⚠️ Cache STALE (serving while revalidating)");
            println!("   User: {}", entry.value.name);
        }
        CacheResult::NegativeHit => {
            println!("🚫 Negative cache hit (known missing)");
        }
    }

    // Using tuple keys
    println!("\nUsing tuple keys...");
    cache
        .set(("session", "abc123"), &"user_data".to_string(), CacheOpts::new().ttl_mins(30))
        .await?;

    if cache.exists(("session", "abc123")).await? {
        println!("✅ Session exists in cache");
    }

    // Check cache stats
    let stats = cache.stats().await?;
    println!("\n📊 Cache Statistics:");
    println!("   Hits: {}", stats.hits);
    println!("   Misses: {}", stats.misses);
    println!("   Writes: {}", stats.writes);
    println!("   Hit Ratio: {:.2}%", stats.hit_ratio() * 100.0);
    println!("   Size: {} entries", stats.size);

    // Delete entry
    println!("\nDeleting user from cache...");
    let deleted = cache.delete("user:123").await?;
    println!("   Deleted: {}", deleted);
    println!("   Exists after delete: {}", cache.exists("user:123").await?);

    // Demonstrate cache with namespace
    println!("\n--- Cache with Namespace ---");
    let backend2 = MemoryBackend::new(MemoryConfig::default());
    let namespaced_cache = CacheManager::with_config(
        backend2,
        CacheManagerConfig::with_namespace("myapp"),
    );

    namespaced_cache
        .set("config", &"value123".to_string(), CacheOpts::new())
        .await?;

    match namespaced_cache.get::<String>("config").await? {
        CacheResult::Hit(entry) => {
            println!("✅ Namespaced cache HIT: {}", entry.value);
        }
        _ => println!("❌ Unexpected result"),
    }

    // Demonstrate custom TTL
    println!("\n--- Custom TTL Example ---");
    cache
        .set(
            "temp_data",
            &"expires soon".to_string(),
            CacheOpts::new()
                .ttl(Duration::from_secs(60))
                .swr(Duration::from_secs(30)),
        )
        .await?;

    match cache.get::<String>("temp_data").await? {
        CacheResult::Hit(entry) => {
            println!("✅ Temp data cached with TTL: {:?}", entry.ttl);
            println!("   Stale-while-revalidate: {:?}", entry.stale_while_revalidate);
        }
        _ => {}
    }

    println!("\n=== Example Complete ===");
    Ok(())
}

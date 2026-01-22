//! Dependency Graph Example
//!
//! Demonstrates how cache entries can depend on other entries,
//! enabling automatic cascade invalidation when parent entries change.

use skp_cache::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Tenant {
    id: u64,
    name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct User {
    id: u64,
    tenant_id: u64,
    name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UserPosts {
    user_id: u64,
    posts: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let backend = MemoryBackend::new(MemoryConfig::default());
    let cache = CacheManager::new(backend);

    println!("=== Dependency Graph Demo ===\n");

    // 1. Create a tenant (root entity)
    let tenant = Tenant {
        id: 1,
        name: "Acme Corp".to_string(),
    };
    cache
        .set(
            "tenant:1",
            &tenant,
            CacheOpts::new()
                .ttl(Duration::from_secs(3600))
                .tags(["tenants"]),
        )
        .await?;
    println!("✓ Created tenant:1");

    // 2. Create a user that DEPENDS on the tenant
    let user = User {
        id: 123,
        tenant_id: 1,
        name: "Alice".to_string(),
    };
    cache
        .set(
            "user:123",
            &user,
            CacheOpts::new()
                .ttl(Duration::from_secs(1800))
                .tags(["users", "tenant:1/users"])
                .depends_on(["tenant:1"]), // <-- Dependency!
        )
        .await?;
    println!("✓ Created user:123 (depends on tenant:1)");

    // 3. Create user posts that DEPEND on the user
    let posts = UserPosts {
        user_id: 123,
        posts: vec!["Hello World".to_string(), "Rust is great".to_string()],
    };
    cache
        .set(
            "user:123:posts",
            &posts,
            CacheOpts::new()
                .ttl(Duration::from_secs(600))
                .tags(["posts", "user:123/posts"])
                .depends_on(["user:123"]), // <-- Dependency!
        )
        .await?;
    println!("✓ Created user:123:posts (depends on user:123)");

    // Verify all entries exist
    println!("\n--- Before Invalidation ---");
    println!(
        "tenant:1 exists: {}",
        matches!(cache.get::<Tenant>("tenant:1").await?, CacheResult::Hit(_))
    );
    println!(
        "user:123 exists: {}",
        matches!(cache.get::<User>("user:123").await?, CacheResult::Hit(_))
    );
    println!(
        "user:123:posts exists: {}",
        matches!(
            cache.get::<UserPosts>("user:123:posts").await?,
            CacheResult::Hit(_)
        )
    );

    // 4. Invalidate the tenant - this should cascade!
    println!("\n🗑️  Invalidating tenant:1...");
    let count = cache.invalidate("tenant:1").await?;
    println!("   Invalidated {} entries (tenant + user + posts)", count);

    // Verify cascade invalidation
    println!("\n--- After Invalidation ---");
    println!(
        "tenant:1 exists: {}",
        matches!(cache.get::<Tenant>("tenant:1").await?, CacheResult::Hit(_))
    );
    println!(
        "user:123 exists: {}",
        matches!(cache.get::<User>("user:123").await?, CacheResult::Hit(_))
    );
    println!(
        "user:123:posts exists: {}",
        matches!(
            cache.get::<UserPosts>("user:123:posts").await?,
            CacheResult::Hit(_)
        )
    );

    println!("\n✅ Dependency graph invalidation works correctly!");

    Ok(())
}

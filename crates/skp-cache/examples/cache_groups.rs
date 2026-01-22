use skp_cache::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Enable TTL index + tag index (default)
    let backend = MemoryBackend::new(MemoryConfig::default());
    let cache = CacheManager::new(backend);

    // Create groups
    let users = cache.group("users");
    let products = cache.group("products");

    // Set keys
    println!("Setting users:1 = Alice");
    users.set("1", "Alice", CacheOpts::default()).await?;
    
    println!("Setting products:1 = Laptop");
    products.set("1", "Laptop", CacheOpts::default()).await?;

    // Verify isolation
    let u1: Option<String> = users.get("1").await?.value();
    let p1: Option<String> = products.get("1").await?.value();
    
    println!("Users:1 = {:?}", u1);
    println!("Products:1 = {:?}", p1);
    
    assert_eq!(u1, Some("Alice".to_string()));
    assert_eq!(p1, Some("Laptop".to_string()));
    
    // Invalidate users group
    println!("\nInvalidating users group...");
    let count = users.invalidate_all().await?;
    println!("Invalidated {} entries", count);
    assert!(count >= 1);
    
    // Verify users gone, products remain
    let u1_after: Option<String> = users.get("1").await?.value();
    let p1_after: Option<String> = products.get("1").await?.value();
    
    println!("Users:1 after invalidate = {:?}", u1_after);
    println!("Products:1 after invalidate = {:?}", p1_after);
    
    assert_eq!(u1_after, None);
    assert_eq!(p1_after, Some("Laptop".to_string()));
    
    // Check internal keys
    let keys = cache.get_keys_by_tag("group:products").await?;
    println!("\nActual keys in 'products' group (via tag): {:?}", keys);
    assert!(keys.len() >= 1);
    
    println!("\n✅ Cache groups isolation and invalidation verified!");
    
    Ok(())
}

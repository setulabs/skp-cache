use skp_cache::prelude::*;
use skp_cache_core::{CacheOpts};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let backend = MemoryBackend::new(MemoryConfig::default());
    let cache = CacheManager::new(backend);
    
    println!("Creating dependency chain: A <- B <- C");
    // Create A
    cache.set("A", &"value_A", CacheOpts::new()).await?;
    
    // Create B depends on A
    cache.set("B", &"value_B", CacheOpts::new().depends_on(["A"])).await?;
    
    // Create C depends on B
    cache.set("C", &"value_C", CacheOpts::new().depends_on(["B"])).await?;
    
    // Check all exist
    assert!(cache.exists("A").await?);
    assert!(cache.exists("B").await?);
    assert!(cache.exists("C").await?);
    
    // Delete A
    println!("Deleting A (root of chain)...");
    cache.delete("A").await?;
    
    // Verify cascade
    assert!(!cache.exists("A").await?, "A should be deleted");
    assert!(!cache.exists("B").await?, "B should be deleted (depended on A)");
    assert!(!cache.exists("C").await?, "C should be deleted (depended on B)");
    
    println!("Cascade success: A, B, C deleted.");
    
    // Test Set cascade
    println!("Testing Set invalidation...");
    cache.set("X", &"val_X", CacheOpts::new()).await?;
    cache.set("Y", &"val_Y", CacheOpts::new().depends_on(["X"])).await?;
    
    assert!(cache.exists("X").await?);
    assert!(cache.exists("Y").await?);
    
    println!("Updating X...");
    cache.set("X", &"new_val_X", CacheOpts::new()).await?;
    
    // Y should be invalidated
    assert!(cache.exists("X").await?);
    assert!(!cache.exists("Y").await?, "Y should be invalidated when X changed");
    
    println!("Set cascade success: Y invalidated.");
    
    Ok(())
}

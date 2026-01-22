use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use skp_cache::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    id: u64,
    name: String,
    role: String,
}

struct UserLoader {
    // Simulate database
    db: Arc<Mutex<HashMap<String, User>>>,
}

impl UserLoader {
    fn new() -> Self {
        let mut db = HashMap::new();
        db.insert("1".to_string(), User { id: 1, name: "Alice".into(), role: "Admin".into() });
        db.insert("2".to_string(), User { id: 2, name: "Bob".into(), role: "User".into() });
        
        Self {
            db: Arc::new(Mutex::new(db)),
        }
    }
}

#[async_trait]
impl Loader<String, User> for UserLoader {
    async fn load(&self, key: &String) -> Result<Option<User>> {
        println!("  -> Loading user {} from DB...", key);
        // Simulate latency
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let db = self.db.lock().unwrap();
        Ok(db.get(key).cloned())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Create base cache
    let backend = MemoryBackend::new(MemoryConfig::default());
    let cache = CacheManager::new(backend);
    
    // 2. Create loader
    let loader = UserLoader::new();
    
    // 3. Create read-through cache
    // Cache User objects keyed by String, with 60s TTL
    // NOTE: Explicit type annotation helps compiler infer bounds
    let user_cache = cache.read_through::<String, User, UserLoader>(
        loader,
        CacheOpts::new().ttl_secs(60).into()
    );
    
    println!("1. Fetching user 1 (first time - should load from DB)");
    let user = user_cache.get("1".to_string()).await?;
    println!("Got: {:?}", user);
    
    println!("\n2. Fetching user 1 (second time - should be cached)");
    let user = user_cache.get("1".to_string()).await?;
    println!("Got: {:?}", user);
    
    println!("\n3. Fetching non-existent user 99");
    let user = user_cache.get("99".to_string()).await?;
    println!("Got: {:?}", user);

    Ok(())
}

use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use redis::{AsyncCommands, Value};
use std::sync::Arc;
use parking_lot::RwLock as SyncRwLock;
use skp_cache_core::{
    CacheBackend, CacheEntry, CacheError, CacheOptions, CacheStats, DependencyBackend, Result, TaggableBackend,
};
use std::time::SystemTime;

use super::config::RedisConfig;

/// Redis backend implementation
#[derive(Clone)]
pub struct RedisBackend {
    pool: Pool<RedisConnectionManager>,
    config: RedisConfig,
    stats: Arc<SyncRwLock<CacheStats>>,
}

impl RedisBackend {
    /// Create a new Redis backend
    pub async fn new(config: RedisConfig) -> Result<Self> {
        let manager = RedisConnectionManager::new(config.url.as_str())
            .map_err(|e| CacheError::Connection(e.to_string()))?;
            
        let pool = Pool::builder()
            .max_size(config.pool_size)
            .connection_timeout(config.connection_timeout)
            .build(manager)
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
            
        Ok(Self {
            pool,
            config,
            stats: Arc::new(SyncRwLock::new(CacheStats::default())),
        })
    }
    
    /// Get prefix for a key
    fn prefixed_key(&self, key: &str) -> String {
        match &self.config.key_prefix {
            Some(prefix) => format!("{}:{}", prefix, key),
            None => key.to_string(),
        }
    }
    
    /// Get tag key
    fn tag_key(&self, tag: &str) -> String {
        match &self.config.key_prefix {
            Some(prefix) => format!("{}:__tags__:{}", prefix, tag),
            None => format!("__tags__:{}", tag),
        }
    }

    /// Get dependency key
    fn dep_key(&self, dep: &str) -> String {
        match &self.config.key_prefix {
            Some(prefix) => format!("{}:__deps__:{}", prefix, dep),
            None => format!("__deps__:{}", dep),
        }
    }

    /// Get connection from pool
    async fn get_connection(&self) -> Result<PooledConnection<'_, RedisConnectionManager>> {
        self.pool.get().await.map_err(|e| CacheError::Connection(e.to_string()))
    }
}

#[async_trait]
impl CacheBackend for RedisBackend {
    async fn get(&self, key: &str) -> Result<Option<CacheEntry<Vec<u8>>>> {
        let mut conn = self.get_connection().await?;
        let prefixed = self.prefixed_key(key);
        
        let bytes: Option<Vec<u8>> = conn.get(&prefixed).await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
            
        match bytes {
            Some(data) => {
                let entry: CacheEntry<Vec<u8>> = serde_json::from_slice(&data)
                    .map_err(|e| CacheError::Deserialization(e.to_string()))?;
                
                // Update hit stats
                self.stats.write().hits += 1;
                Ok(Some(entry))
            },
            None => {
                // Update miss stats
                self.stats.write().misses += 1;
                Ok(None)
            }
        }
    }

    async fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        options: &CacheOptions,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;
        
        // Create entry wrapper
        let entry = CacheEntry {
            value,
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            access_count: 0,
            ttl: options.ttl,
            stale_while_revalidate: options.stale_while_revalidate,
            tags: options.tags.clone(),
            dependencies: options.dependencies.clone(),
            cost: options.cost.unwrap_or(1),
            size: 0, // Not easily calculable here without serialization first, but we will serialize next
            etag: options.etag.clone(),
            version: 0,
        };
        
        // Serialize
        let serialized = serde_json::to_vec(&entry)
            .map_err(|e| CacheError::Serialization(e.to_string()))?;
            
        let prefixed = self.prefixed_key(key);
        
        // Use pipeline for atomicity (set key + update tags)
        let mut pipe = redis::pipe();
        pipe.atomic();
        
        // Set with TTL
        if let Some(ttl) = options.ttl {
             let total_ttl = ttl + options.stale_while_revalidate.unwrap_or_default();
             pipe.set_ex(&prefixed, &serialized, total_ttl.as_secs());
        } else {
             pipe.set(&prefixed, &serialized);
        }
        
        
        // Add to tags
        for tag in &options.tags {
            let tag_k = self.tag_key(tag);
            pipe.sadd(&tag_k, key);
        }

        // Add to dependencies
        for dep in &options.dependencies {
            let dep_k = self.dep_key(dep);
            pipe.sadd(&dep_k, key);
        }
        
        pipe.query_async::<Vec<Value>>(&mut *conn).await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
            
        self.stats.write().writes += 1;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let prefixed = self.prefixed_key(key);
        
        let deleted: bool = conn.del(&prefixed).await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
            
        if deleted {
            self.stats.write().deletes += 1;
        }
        Ok(deleted)
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let prefixed = self.prefixed_key(key);
        
        conn.exists(&prefixed).await
            .map_err(|e| CacheError::Backend(e.to_string()))
    }
    
    async fn delete_many(&self, keys: &[&str]) -> Result<u64> {
        let mut conn = self.get_connection().await?;
        if keys.is_empty() {
             return Ok(0);
        }
        
        let prefixed_keys: Vec<String> = keys.iter().map(|k| self.prefixed_key(k)).collect();
        let count: u64 = conn.del(&prefixed_keys).await
             .map_err(|e| CacheError::Backend(e.to_string()))?;
             
        self.stats.write().deletes += count;
        Ok(count)
    }

    async fn get_many(
        &self,
        keys: &[&str],
    ) -> Result<Vec<Option<CacheEntry<Vec<u8>>>>> {
        let mut conn = self.get_connection().await?;
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let prefixed_keys: Vec<String> = keys.iter().map(|k| self.prefixed_key(k)).collect();
        let raw_results: Vec<Option<Vec<u8>>> = conn.mget(&prefixed_keys).await
             .map_err(|e| CacheError::Backend(e.to_string()))?;
             
        let mut results = Vec::with_capacity(raw_results.len());
        let mut hits = 0;
        let mut misses = 0;
        
        for raw in raw_results {
            match raw {
                Some(data) => {
                    let entry: CacheEntry<Vec<u8>> = serde_json::from_slice(&data)
                        .map_err(|e| CacheError::Deserialization(e.to_string()))?;
                    results.push(Some(entry));
                    hits += 1;
                },
                None => {
                    results.push(None);
                    misses += 1;
                }
            }
        }
        
        {
            let mut stats = self.stats.write();
            stats.hits += hits;
            stats.misses += misses;
        }
        
        Ok(results)
    }

    async fn set_many(
        &self,
        entries: &[(&str, Vec<u8>, &CacheOptions)],
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let mut pipe = redis::pipe();
        pipe.atomic();
        
        for (key, value, options) in entries {
            let entry = CacheEntry {
                value: value.clone(),
                created_at: SystemTime::now(),
                last_accessed: SystemTime::now(),
                access_count: 0,
                ttl: options.ttl,
                stale_while_revalidate: options.stale_while_revalidate,
                tags: options.tags.clone(),
                dependencies: options.dependencies.clone(),
                cost: options.cost.unwrap_or(1),
                size: 0,
                etag: options.etag.clone(),
                version: 0,
            };
            
            let serialized = serde_json::to_vec(&entry)
                .map_err(|e| CacheError::Serialization(e.to_string()))?;
            let prefixed = self.prefixed_key(key);
            
             if let Some(ttl) = options.ttl {
                 let total_ttl = ttl + options.stale_while_revalidate.unwrap_or_default();
                 pipe.set_ex(&prefixed, &serialized, total_ttl.as_secs());
            } else {
                 pipe.set(&prefixed, &serialized);
            }
            
            for tag in &options.tags {
                let tag_k = self.tag_key(tag);
                pipe.sadd(&tag_k, key);
            }

            for dep in &options.dependencies {
                let dep_k = self.dep_key(dep);
                pipe.sadd(&dep_k, key);
            }
        }
        
        pipe.query_async::<Vec<Value>>(&mut *conn).await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
            
        self.stats.write().writes += entries.len() as u64;
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;
        
        let match_pattern = match &self.config.key_prefix {
             Some(prefix) => format!("{}:*", prefix),
             None => "*".to_string(),
        };
        
        // Scan and delete
        let mut cursor = 0u64;
        let count_per_scan = 1000;
        
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .cursor_arg(cursor)
                .arg("MATCH")
                .arg(&match_pattern)
                .arg("COUNT")
                .arg(count_per_scan)
                .query_async(&mut *conn)
                .await
                .map_err(|e| CacheError::Backend(e.to_string()))?;
                
            if !keys.is_empty() {
                let _: usize = conn.unlink(&keys).await
                    .map_err(|e| CacheError::Backend(e.to_string()))?;
            }
            
            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }
        
        Ok(())
    }

    async fn stats(&self) -> Result<CacheStats> {
        Ok(self.stats.read().clone())
    }

    async fn len(&self) -> Result<usize> {
        let mut conn = self.get_connection().await?;
        
        // Exact count is expensive in Redis unless we track it
        // Or we use DBSIZE if we own the whole DB
        // If we use prefix, we must scan to count, which is O(N)
        // For now, let's implement O(N) scan count as len() is widely used for debugging/metrics
        // But warning: this is slow on large datasets
        
        if self.config.key_prefix.is_some() {
             let match_pattern = format!("{}:*", self.config.key_prefix.as_ref().unwrap());
             let mut cursor = 0u64;
             let mut count = 0;
             loop {
                 let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                    .cursor_arg(cursor)
                    .arg("MATCH")
                    .arg(&match_pattern)
                    .arg("COUNT")
                    .arg(1000)
                    .query_async(&mut *conn)
                    .await
                    .map_err(|e| CacheError::Backend(e.to_string()))?;
                    
                 count += keys.len();
                 cursor = next_cursor;
                 if cursor == 0 {
                     break;
                 }
             }
             Ok(count)
        } else {
             let size: usize = redis::cmd("DBSIZE")
                .query_async(&mut *conn)
                .await
                .map_err(|e| CacheError::Backend(e.to_string()))?;
             Ok(size)
        }
    }
}

#[async_trait]
impl TaggableBackend for RedisBackend {
    async fn get_by_tag(&self, tag: &str) -> Result<Vec<String>> {
        let mut conn = self.get_connection().await?;
        let tag_k = self.tag_key(tag);
        
        let keys: Vec<String> = conn.smembers(&tag_k).await
             .map_err(|e| CacheError::Backend(e.to_string()))?;
             
        Ok(keys)
    }

    async fn delete_by_tag(&self, tag: &str) -> Result<u64> {
        let mut conn = self.get_connection().await?;
        let tag_k = self.tag_key(tag);
        
        // 1. Get keys
        let keys: Vec<String> = conn.smembers(&tag_k).await
             .map_err(|e| CacheError::Backend(e.to_string()))?;
             
        if keys.is_empty() {
             return Ok(0);
        }
        
        let prefixed_keys: Vec<String> = keys.iter().map(|k| self.prefixed_key(k)).collect();
        
        // 2. Delete keys and the tag key itself in a transaction?
        // But we need to make sure we prefix them correctly.
        // Wait, stored members in SET are raw keys or prefixed keys?
        // In set(): `pipe.sadd(&tag_k, key);` <- stores raw key WITHOUT prefix.
        // So `prefixed_keys` above requires prefixing.
        
        let mut pipe = redis::pipe();
        pipe.atomic();
        
        for k in &prefixed_keys {
             pipe.del(k);
        }
        pipe.del(&tag_k);
        
        pipe.query_async::<Vec<Value>>(&mut *conn).await
            .map_err(|e| CacheError::Backend(e.to_string()))?;
            
        self.stats.write().deletes += keys.len() as u64;
        Ok(keys.len() as u64)
    }
}

#[async_trait]
impl DependencyBackend for RedisBackend {
    async fn get_dependents(&self, key: &str) -> Result<Vec<String>> {
        let mut conn = self.get_connection().await?;
        let dep_k = self.dep_key(key);
        
        let keys: Vec<String> = conn.smembers(&dep_k).await
             .map_err(|e| CacheError::Backend(e.to_string()))?;
             
        Ok(keys)
    }
}

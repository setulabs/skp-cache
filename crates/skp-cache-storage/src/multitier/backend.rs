use async_trait::async_trait;
use skp_cache_core::{
    CacheBackend, CacheEntry, CacheError, CacheOptions, CacheStats, Result, TaggableBackend,
};
use super::circuit_breaker::CircuitBreaker;

/// Multi-tier backend combining L1 (fast, local) and L2 (slow, remote) caches
pub struct MultiTierBackend<L1, L2> {
    l1: L1,
    l2: L2,
    circuit_breaker: CircuitBreaker,
}

impl<L1, L2> MultiTierBackend<L1, L2> {
    /// Create a new multi-tier backend
    pub fn new(l1: L1, l2: L2, circuit_breaker: CircuitBreaker) -> Self {
        Self {
            l1,
            l2,
            circuit_breaker,
        }
    }
}

#[async_trait]
impl<L1, L2> CacheBackend for MultiTierBackend<L1, L2>
where
    L1: CacheBackend,
    L2: CacheBackend,
{
    async fn get(&self, key: &str) -> Result<Option<CacheEntry<Vec<u8>>>> {
        // 1. Try L1 (Memory) first
        match self.l1.get(key).await {
            Ok(Some(entry)) => {
                // Buffer hit
                return Ok(Some(entry));
            }
            Err(_e) => {
                // Log warning but continue to L2?
                // For now, we ignore L1 errors (treat as miss) to prioritize availability
                // In production, you'd want logging here
            }
            Ok(None) => {} // Miss
        }

        // 2. Check Circuit Breaker for L2
        if !self.circuit_breaker.allow_request() {
            // Circuit open - return miss (degraded mode)
            return Ok(None);
        }

        // 3. Try L2 (Redis)
        match self.l2.get(key).await {
            Ok(Some(entry)) => {
                self.circuit_breaker.report_success();
                
                // 4. Backfill L1
                // We recreate options from the entry roughly
                let opts = CacheOptions {
                    ttl: entry.ttl,
                    stale_while_revalidate: entry.stale_while_revalidate,
                    tags: entry.tags.clone(),
                    dependencies: entry.dependencies.clone(),
                    cost: Some(entry.cost),
                    etag: entry.etag.clone(),
                    ..Default::default()
                };
                
                // Ignore L1 set errors (it's just an optimization)
                let _ = self.l1.set(key, entry.value.clone(), &opts).await;
                
                Ok(Some(entry))
            }
            Ok(None) => {
                self.circuit_breaker.report_success();
                Ok(None)
            }
            Err(e) => {
                if CircuitBreaker::is_failure(&e) {
                    self.circuit_breaker.report_failure();
                }
                Err(e)
            }
        }
    }

    async fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        options: &CacheOptions,
    ) -> Result<()> {
        // Write-through: Set L2 then L1
        
        // Check breaker before L2 write?
        // Usually writes should fail if backend is down to ensure consistency.
        if !self.circuit_breaker.allow_request() {
             return Err(CacheError::Backend("Circuit breaker open".to_string()));
        }

        match self.l2.set(key, value.clone(), options).await {
            Ok(_) => {
                self.circuit_breaker.report_success();
                // L2 success, now update L1
                // We want L1 to reflect L2.
                self.l1.set(key, value, options).await?;
                Ok(())
            }
            Err(e) => {
                if CircuitBreaker::is_failure(&e) {
                    self.circuit_breaker.report_failure();
                }
                Err(e)
            }
        }
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        // Delete from both. L2 first.
        let l2_res = self.l2.delete(key).await;
        // Even if L2 fails, we should delete from L1 to avoid stale data?
        // But if L2 fails, we might still have data in L2. L1 deleted + L2 present = inconsistency.
        // Cache consistency is hard.
        // Best effort: delete both.
        
        let l1_res = self.l1.delete(key).await;
        
        match l2_res {
             Ok(deleted) => {
                 l1_res?; // Propagate L1 error?
                 Ok(deleted)
             }
             Err(e) => {
                 // L2 failed.
                 if CircuitBreaker::is_failure(&e) {
                     self.circuit_breaker.report_failure();
                 }
                 Err(e)
             }
        }
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        // Check L1 first
        if self.l1.exists(key).await? {
            return Ok(true);
        }
        
        if !self.circuit_breaker.allow_request() {
            return Ok(false);
        }
        
        match self.l2.exists(key).await {
             Ok(exists) => {
                 self.circuit_breaker.report_success();
                 Ok(exists)
             }
             Err(e) => {
                 if CircuitBreaker::is_failure(&e) {
                     self.circuit_breaker.report_failure();
                 }
                 Err(e)
             }
        }
    }
    
    async fn delete_many(&self, keys: &[&str]) -> Result<u64> {
        let l2_res = self.l2.delete_many(keys).await;
        let _ = self.l1.delete_many(keys).await;
        
        l2_res.map_err(|e| {
             if CircuitBreaker::is_failure(&e) {
                 self.circuit_breaker.report_failure();
             }
             e
        })
    }

    async fn get_many(
        &self,
        keys: &[&str],
    ) -> Result<Vec<Option<CacheEntry<Vec<u8>>>>> {
        // Naive implementation: iterate.
        // Optimized: 
        // 1. get_many from L1.
        // 2. Identify misses.
        // 3. get_many from L2 for misses.
        // 4. Backfill L2 hits to L1.
        // 5. Merge results.
        
        let l1_results = self.l1.get_many(keys).await?;
        let mut final_results = Vec::with_capacity(keys.len());
        let mut missing_indices = Vec::new();
        let mut missing_keys = Vec::new();
        
        for (i, res) in l1_results.into_iter().enumerate() {
            if res.is_some() {
                final_results.push(res);
            } else {
                final_results.push(None); // Placeholder
                missing_indices.push(i);
                missing_keys.push(keys[i]);
            }
        }
        
        if missing_keys.is_empty() {
            return Ok(final_results);
        }
        
        if !self.circuit_breaker.allow_request() {
            return Ok(final_results); // Return partial results (L1 hits only)
        }
        
        match self.l2.get_many(&missing_keys).await {
            Ok(l2_results) => {
                self.circuit_breaker.report_success();
                
                for (i, l2_res) in l2_results.into_iter().enumerate() {
                    let original_idx = missing_indices[i];
                    if let Some(entry) = l2_res {
                         // Backfill
                         let opts = CacheOptions {
                            ttl: entry.ttl,
                            stale_while_revalidate: entry.stale_while_revalidate,
                            tags: entry.tags.clone(),
                            dependencies: entry.dependencies.clone(),
                            cost: Some(entry.cost),
                            etag: entry.etag.clone(),
                            ..Default::default()
                        };
                        let _ = self.l1.set(keys[original_idx], entry.value.clone(), &opts).await;
                        final_results[original_idx] = Some(entry);
                    }
                }
                Ok(final_results)
            }
            Err(e) => {
                if CircuitBreaker::is_failure(&e) {
                    self.circuit_breaker.report_failure();
                }
                // If L2 fails, return current partial results? Or error?
                // Returning partial (L1 only) results is safer for resilience.
                // But caller expects Ok implies complete result attempt.
                // MultiTier strategy usually degrades gracefully.
                Ok(final_results)
            }
        }
    }

    async fn set_many(
        &self,
        entries: &[(&str, Vec<u8>, &CacheOptions)],
    ) -> Result<()> {
        if !self.circuit_breaker.allow_request() {
             return Err(CacheError::Backend("Circuit breaker open".to_string()));
        }
        
        match self.l2.set_many(entries).await {
             Ok(_) => {
                 self.circuit_breaker.report_success();
                 self.l1.set_many(entries).await?;
                 Ok(())
             }
             Err(e) => {
                 if CircuitBreaker::is_failure(&e) {
                    self.circuit_breaker.report_failure();
                }
                Err(e)
             }
        }
    }

    async fn clear(&self) -> Result<()> {
        let l2_res = self.l2.clear().await;
        let _ = self.l1.clear().await;
        l2_res
    }

    async fn stats(&self) -> Result<CacheStats> {
        // Aggregate stats? Or return L2 stats?
        // Only L2 stats are persistent.
        // But L1 stats are useful for hit ratio.
        // CacheBackend returns single CacheStats.
        // We could sum them up.
        let l1_stats = self.l1.stats().await?;
        let l2_stats = match self.l2.stats().await {
             Ok(s) => s,
             Err(_) => CacheStats::default(),
        };
        
        Ok(CacheStats {
            hits: l1_stats.hits + l2_stats.hits,
            misses: l2_stats.misses, // True misses are L2 misses
            stale_hits: l1_stats.stale_hits + l2_stats.stale_hits,
            writes: l2_stats.writes,
            deletes: l2_stats.deletes,
            evictions: l1_stats.evictions + l2_stats.evictions,
            size: l2_stats.size, // L2 size is total size
            memory_bytes: l1_stats.memory_bytes, // L1 usage is relevant RAM usage
        })
    }

    async fn len(&self) -> Result<usize> {
        self.l2.len().await
    }
}

#[async_trait]
impl<L1, L2> TaggableBackend for MultiTierBackend<L1, L2>
where
    L1: TaggableBackend,
    L2: TaggableBackend,
{
    async fn get_by_tag(&self, tag: &str) -> Result<Vec<String>> {
        // L2 is authority
        if !self.circuit_breaker.allow_request() {
             return self.l1.get_by_tag(tag).await;
        }
        match self.l2.get_by_tag(tag).await {
             Ok(keys) => {
                 self.circuit_breaker.report_success();
                 Ok(keys)
             },
             Err(e) => {
                 if CircuitBreaker::is_failure(&e) {
                    self.circuit_breaker.report_failure();
                }
                // Fallback to L1?
                self.l1.get_by_tag(tag).await
             }
        }
    }

    async fn delete_by_tag(&self, tag: &str) -> Result<u64> {
        let l2_res = self.l2.delete_by_tag(tag).await;
        let _ = self.l1.delete_by_tag(tag).await;
        
        match l2_res {
             Ok(count) => {
                 self.circuit_breaker.report_success();
                 Ok(count)
             }
             Err(e) => {
                 if CircuitBreaker::is_failure(&e) {
                    self.circuit_breaker.report_failure();
                }
                Err(e)
             }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryBackend, MemoryConfig};
    use std::time::Duration;
    use parking_lot::RwLock;
    use std::sync::Arc;

    /// Helper to create memory backend
    fn create_memory() -> MemoryBackend {
        MemoryBackend::new(MemoryConfig::default())
    }

    #[tokio::test]
    async fn test_multitier_flow() {
        let l1 = create_memory();
        let l2 = create_memory();
        let breaker = CircuitBreaker::new(3, Duration::from_secs(10));
        let backend = MultiTierBackend::new(l1.clone(), l2.clone(), breaker);

        let opts = CacheOptions::default();

        // 1. Set (Write through)
        backend.set("key", b"val".to_vec(), &opts).await.unwrap();

        // Check L1 and L2
        assert!(l1.exists("key").await.unwrap());
        assert!(l2.exists("key").await.unwrap());

        // 2. Get (L1 Hit)
        let res = backend.get("key").await.unwrap();
        assert!(res.is_some());
        assert_eq!(res.unwrap().value, b"val".to_vec());

        // 3. Simulate L1 Eviction/Miss
        l1.delete("key").await.unwrap();
        assert!(!l1.exists("key").await.unwrap());
        
        // Get should hit L2 and backfill L1
        let res = backend.get("key").await.unwrap();
        assert!(res.is_some());
        assert_eq!(res.unwrap().value, b"val".to_vec());
        
        // Check L1 backfill
        assert!(l1.exists("key").await.unwrap());
    }

    #[derive(Clone)]
    struct FailingBackend {
        failures: Arc<RwLock<usize>>,
    }

    #[async_trait]
    impl CacheBackend for FailingBackend {
        async fn get(&self, _key: &str) -> Result<Option<CacheEntry<Vec<u8>>>> {
            *self.failures.write() += 1;
            Err(CacheError::Backend("Fail".to_string()))
        }
        async fn set(&self, _key: &str, _value: Vec<u8>, _opts: &CacheOptions) -> Result<()> {
            *self.failures.write() += 1;
            Err(CacheError::Backend("Fail".to_string()))
        }
        async fn delete(&self, _key: &str) -> Result<bool> { Err(CacheError::Backend("Fail".to_string())) }
        async fn exists(&self, _key: &str) -> Result<bool> { Err(CacheError::Backend("Fail".to_string())) }
        async fn delete_many(&self, _keys: &[&str]) -> Result<u64> { Err(CacheError::Backend("Fail".to_string())) }
        async fn get_many(&self, _keys: &[&str]) -> Result<Vec<Option<CacheEntry<Vec<u8>>>>> { Err(CacheError::Backend("Fail".to_string())) }
        async fn set_many(&self, _entries: &[(&str, Vec<u8>, &CacheOptions)]) -> Result<()> { Err(CacheError::Backend("Fail".to_string())) }
        async fn clear(&self) -> Result<()> { Err(CacheError::Backend("Fail".to_string())) }
        async fn stats(&self) -> Result<CacheStats> { Ok(CacheStats::default()) }
        async fn len(&self) -> Result<usize> { Ok(0) }
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let l1 = create_memory();
        let l2_fails = Arc::new(RwLock::new(0));
        let l2 = FailingBackend { failures: l2_fails.clone() };
        
        let breaker = CircuitBreaker::new(2, Duration::from_millis(100)); // 2 failures to trip
        let backend = MultiTierBackend::new(l1, l2, breaker);

        // 1. Fail L2 on get (Miss L1 -> Fail L2)
        assert!(backend.get("key").await.is_err()); // Fail 1
        assert!(backend.get("key").await.is_err()); // Fail 2 -> Trip

        // 2. Circuit should be Open now
        // Next request should return None (Degraded mode) or fail fast?
        // Implementation returns Ok(None) on get.
        let res = backend.get("key").await;
        assert!(res.is_ok()); 
        assert!(res.unwrap().is_none());
        
        // Assert we didn't call backend again (failures count should be 2)
        assert_eq!(*l2_fails.read(), 2);
        
        // 3. Wait for reset timeout
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // 4. HalfOpen -> Try again -> Fail -> Trip again
        assert!(backend.get("key").await.is_err());
        assert_eq!(*l2_fails.read(), 3);
        
        // 5. Open again
        let res = backend.get("key").await;
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
        assert_eq!(*l2_fails.read(), 3);
    }
}

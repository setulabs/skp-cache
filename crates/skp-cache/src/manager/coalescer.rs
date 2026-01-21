use std::sync::Arc;
use tokio::sync::broadcast;
use dashmap::DashMap;
use skp_cache_core::{CacheEntry, Result, CacheError};

#[derive(Clone, Default)]
pub struct Coalescer {
    // Map key -> Broadcast channel sender
    // The sender transmits the result of the cache fetch
    inflight: Arc<DashMap<String, broadcast::Sender<Result<Option<CacheEntry<Vec<u8>>>>>>>,
    // Set of keys currently being refreshed in background (SWR)
    refreshing: Arc<DashMap<String, ()>>,
}

impl Coalescer {
    pub fn new() -> Self {
        Self {
            inflight: Arc::new(DashMap::new()),
            refreshing: Arc::new(DashMap::new()),
        }
    }

    /// Execute a request with coalescing for the given key.
    /// If a request for this key is already running, wait for its result.
    /// Otherwise, run the request and broadcast the result.
    pub async fn do_request<F, Fut>(&self, key: &str, f: F) -> Result<Option<CacheEntry<Vec<u8>>>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Option<CacheEntry<Vec<u8>>>>> + Send + 'static,
    {
        // Try to join existing request or become leader
        // We use a scope here to ensure the DashMap entry lock is dropped immediately
        let action = {
             let entry = self.inflight.entry(key.to_string());
             match entry {
                 dashmap::mapref::entry::Entry::Occupied(o) => {
                     // Join existing request
                     Ok(o.get().subscribe())
                 },
                 dashmap::mapref::entry::Entry::Vacant(v) => {
                     // Become leader
                     let (tx, _rx) = broadcast::channel(1);
                     v.insert(tx.clone());
                     Err(tx)
                 }
             }
        };

        match action {
            Ok(mut rx) => {
                // Follower: wait for result
                match rx.recv().await {
                    Ok(res) => res,
                    Err(_) => {
                        // Leader dropped/failed without sending (e.g. panic)
                        // We cannot easily retry because F is FnOnce and consumed.
                        Err(CacheError::Internal("In-flight request failed".to_string()))
                    }
                }
            },
            Err(tx) => {
                // Leader: execute request
                let result = f().await;
                
                // Cleanup map entry first
                self.inflight.remove(key);
                
                // Send result to followers if any
                if tx.receiver_count() > 0 {
                    // Clone result (expensive but necessary for owned return)
                    let _ = tx.send(result.clone());
                }
                
                result
            }
        }
    }

    /// Try to spawn a background refresh task for the given key.
    /// If a refresh is already running for this key, this is a no-op.
    pub fn try_spawn_refresh<F, Fut>(&self, key: &str, task_factory: F)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let key_str = key.to_string();
        // Use entry API to atomically check and insert
        // DashMap entry locking ensures atomicity
        let should_run = match self.refreshing.entry(key_str.clone()) {
            dashmap::mapref::entry::Entry::Vacant(v) => {
                v.insert(());
                true
            },
            dashmap::mapref::entry::Entry::Occupied(_) => false
        };

        if should_run {
            let task = task_factory();
            let map = self.refreshing.clone();
            tokio::spawn(async move {
                task.await;
                map.remove(&key_str);
            });
        }
    }
}

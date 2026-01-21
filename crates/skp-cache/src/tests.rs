//! Integration tests for CacheManager

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use std::time::Duration;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct TestData {
        id: u64,
        name: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_basic_get_set() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let cache = CacheManager::new(backend);

        let data = TestData {
            id: 1,
            name: "test".to_string(),
            value: 42,
        };

        cache
            .set("test_key", &data, CacheOpts::new())
            .await
            .unwrap();

        match cache.get::<TestData>("test_key").await.unwrap() {
            CacheResult::Hit(entry) => {
                assert_eq!(entry.value, data);
            }
            _ => panic!("Expected cache hit"),
        }
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let cache = CacheManager::new(backend);

        match cache.get::<TestData>("nonexistent").await.unwrap() {
            CacheResult::Miss => {}
            _ => panic!("Expected cache miss"),
        }
    }

    #[tokio::test]
    async fn test_delete() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let cache = CacheManager::new(backend);

        cache.set("key", &42i32, CacheOpts::new()).await.unwrap();
        assert!(cache.exists("key").await.unwrap());

        let deleted = cache.delete("key").await.unwrap();
        assert!(deleted);
        assert!(!cache.exists("key").await.unwrap());
    }

    #[tokio::test]
    async fn test_with_namespace() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let config = CacheManagerConfig {
            namespace: Some("myapp".to_string()),
            ..Default::default()
        };
        let cache = CacheManager::with_config(backend, config);

        cache.set("key", &42i32, CacheOpts::new()).await.unwrap();
        assert!(cache.exists("key").await.unwrap());

        // Value should be stored with namespace prefix
        match cache.get::<i32>("key").await.unwrap() {
            CacheResult::Hit(entry) => {
                assert_eq!(entry.value, 42);
            }
            _ => panic!("Expected cache hit"),
        }
    }

    #[tokio::test]
    async fn test_with_ttl() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let config = CacheManagerConfig {
            default_ttl: Some(Duration::from_secs(60)),
            ttl_jitter: 0.0, // Disable jitter for test
            ..Default::default()
        };
        let cache = CacheManager::with_config(backend, config);

        cache.set("key", &42i32, CacheOpts::new()).await.unwrap();

        match cache.get::<i32>("key").await.unwrap() {
            CacheResult::Hit(entry) => {
                assert!(entry.ttl.is_some());
                assert_eq!(entry.ttl.unwrap(), Duration::from_secs(60));
            }
            _ => panic!("Expected cache hit"),
        }
    }

    #[tokio::test]
    async fn test_custom_ttl() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let config = CacheManagerConfig {
            ttl_jitter: 0.0,
            ..Default::default()
        };
        let cache = CacheManager::with_config(backend, config);

        cache
            .set("key", &42i32, CacheOpts::new().ttl_secs(120))
            .await
            .unwrap();

        match cache.get::<i32>("key").await.unwrap() {
            CacheResult::Hit(entry) => {
                assert_eq!(entry.ttl.unwrap(), Duration::from_secs(120));
            }
            _ => panic!("Expected cache hit"),
        }
    }

    #[tokio::test]
    async fn test_stats() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let cache = CacheManager::new(backend);

        cache.set("key1", &1i32, CacheOpts::new()).await.unwrap();
        cache.set("key2", &2i32, CacheOpts::new()).await.unwrap();

        let _ = cache.get::<i32>("key1").await.unwrap(); // Hit
        let _ = cache.get::<i32>("key3").await.unwrap(); // Miss

        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.writes, 2);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_clear() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let cache = CacheManager::new(backend);

        cache.set("key1", &1i32, CacheOpts::new()).await.unwrap();
        cache.set("key2", &2i32, CacheOpts::new()).await.unwrap();

        assert_eq!(cache.len().await.unwrap(), 2);

        cache.clear().await.unwrap();
        assert_eq!(cache.len().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_tuple_keys() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let cache = CacheManager::new(backend);

        // 2-tuple key
        cache
            .set(("user", 123), &"Alice".to_string(), CacheOpts::new())
            .await
            .unwrap();

        match cache.get::<String>(("user", 123)).await.unwrap() {
            CacheResult::Hit(entry) => {
                assert_eq!(entry.value, "Alice");
            }
            _ => panic!("Expected cache hit"),
        }
    }

    #[tokio::test]
    async fn test_clone() {
        let backend = MemoryBackend::new(MemoryConfig::default());
        let cache1 = CacheManager::new(backend);

        cache1.set("key", &42i32, CacheOpts::new()).await.unwrap();

        let cache2 = cache1.clone();

        // Both should see the same data (shared backend)
        assert!(cache2.exists("key").await.unwrap());
    }
}

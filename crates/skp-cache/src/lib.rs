//! skp-cache: Advanced, modular caching library for Rust
//!
//! # Features
//!
//! - **Multi-tier caching** (L1 Memory + L2 Redis)
//! - **Dependency graph-based invalidation**
//! - **Pluggable serialization** (JSON, MessagePack, Bincode)
//! - **Metrics integration**
//! - **Stampede protection**
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use skp_cache::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//!     let backend = MemoryBackend::new(MemoryConfig::default());
//!     let cache = CacheManager::new(backend);
//!
//!     cache.set("key", &42i32, CacheOpts::new().ttl_secs(60)).await?;
//!     
//!     match cache.get::<i32>("key").await? {
//!         CacheResult::Hit(entry) => println!("Got: {}", entry.value),
//!         CacheResult::Miss => println!("Cache miss"),
//!         _ => {}
//!     }
//!     
//!     Ok(())
//! }
//! ```

mod manager;

// Re-export core
pub use skp_cache_core::*;

// Re-export storage
#[cfg(feature = "memory")]
pub use skp_cache_storage::{MemoryBackend, MemoryConfig};

#[cfg(feature = "redis")]
pub use skp_cache_storage::{RedisBackend, RedisConfig};

#[cfg(feature = "multitier")]
pub use skp_cache_storage::{MultiTierBackend, CircuitBreaker};

#[cfg(feature = "derive")]
pub use skp_cache_derive::CacheKey;

// Export manager
pub use manager::{CacheManager, CacheManagerConfig};
pub use manager::{Loader, ReadThroughCache, CacheManagerReadThroughExt};
pub use manager::CacheGroup;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{
        CacheError, CacheKey, CacheManager, CacheManagerConfig, CacheOpts, CacheResult,
        JsonSerializer, Result, Serializer, Loader, ReadThroughCache, CacheManagerReadThroughExt,
        CacheGroup,
    };

    #[cfg(feature = "memory")]
    pub use crate::{MemoryBackend, MemoryConfig};

    #[cfg(feature = "redis")]
    pub use crate::{RedisBackend, RedisConfig};

    #[cfg(feature = "multitier")]
    pub use crate::{MultiTierBackend, CircuitBreaker};

    #[cfg(feature = "msgpack")]
    pub use crate::MsgPackSerializer;

    #[cfg(feature = "bincode")]
    pub use crate::BincodeSerializer;

    #[cfg(feature = "derive")]
    pub use crate::CacheKey as DeriveCacheKey;
}

#[cfg(test)]
mod tests;

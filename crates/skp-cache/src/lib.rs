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

// Export manager
pub use manager::{CacheManager, CacheManagerConfig};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{
        CacheError, CacheKey, CacheManager, CacheManagerConfig, CacheOpts, CacheResult,
        JsonSerializer, Result, Serializer,
    };

    #[cfg(feature = "memory")]
    pub use crate::{MemoryBackend, MemoryConfig};

    #[cfg(feature = "msgpack")]
    pub use crate::MsgPackSerializer;

    #[cfg(feature = "bincode")]
    pub use crate::BincodeSerializer;
}

#[cfg(test)]
mod tests;

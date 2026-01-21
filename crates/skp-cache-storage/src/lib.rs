//! skp-cache-storage: Storage backends for skp-cache

#[cfg(feature = "memory")]
pub mod memory;

#[cfg(feature = "memory")]
pub use memory::{MemoryBackend, MemoryConfig};

#[cfg(feature = "redis")]
pub mod redis;

#[cfg(feature = "redis")]
pub use redis::{RedisBackend, RedisConfig};

#[cfg(feature = "multitier")]
pub mod multitier;

#[cfg(feature = "multitier")]
pub use multitier::{MultiTierBackend, CircuitBreaker};

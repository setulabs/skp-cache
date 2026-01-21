//! skp-cache-storage: Storage backends for skp-cache

#[cfg(feature = "memory")]
pub mod memory;

#[cfg(feature = "memory")]
pub use memory::{MemoryBackend, MemoryConfig};

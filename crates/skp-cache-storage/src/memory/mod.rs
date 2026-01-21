//! In-memory cache backend

mod backend;
mod ttl_index;

pub use backend::{MemoryBackend, MemoryConfig};

//! Core types for cache operations

mod entry;
mod options;
mod result;
mod stats;

pub use entry::CacheEntry;
pub use options::{CacheOptions, CacheOpts};
pub use result::CacheResult;
pub use stats::CacheStats;

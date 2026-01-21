//! skp-cache-core: Core traits and types for the skp-cache library
//!
//! This crate provides the foundational types and traits used throughout
//! the skp-cache ecosystem.

mod error;
mod traits;
mod types;

pub use error::{CacheError, Result};
pub use traits::*;
pub use types::*;

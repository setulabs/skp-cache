//! Core traits for cache operations

mod backend;
mod key;
mod metrics;
mod serializer;

pub use backend::{CacheBackend, DependencyBackend, DistributedBackend, TaggableBackend};
pub use key::{CacheKey, CompositeKey};
pub use metrics::{CacheMetrics, CacheOperation, CacheTier, EvictionReason, NoopMetrics};
pub use serializer::{JsonSerializer, Serializer};

#[cfg(feature = "msgpack")]
pub use serializer::MsgPackSerializer;

#[cfg(feature = "bincode")]
pub use serializer::BincodeSerializer;

#[cfg(feature = "metrics")]
pub use metrics::MetricsCrateAdapter;

#[cfg(feature = "tracing")]
mod tracing;
#[cfg(feature = "tracing")]
pub use tracing::TracingMetrics;


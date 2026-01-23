# Changelog

All notable changes to skp-cache will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-01-22

### Added

#### Core Library (`skp-cache-core`)
- **CacheBackend trait** - Async trait for all cache storage backends with `get`, `set`, `delete`, `exists`, `get_many`, `set_many`, `clear`, `stats`, and `len` methods
- **TaggableBackend trait** - Extension trait for tag-based cache operations (`get_by_tag`, `delete_by_tag`)
- **DependencyBackend trait** - Extension trait for dependency tracking and cascade invalidation
- **DistributedBackend trait** - Extension trait for distributed operations (locks, pub/sub)
- **CacheKey trait** - Trait for types that can be used as cache keys with namespace support
- **Serializer trait** - Pluggable serialization with built-in implementations:
  - `JsonSerializer` (default) - Human-readable, widely compatible
  - `MsgPackSerializer` (feature: `msgpack`) - Faster, more compact
  - `BincodeSerializer` (feature: `bincode`) - Fastest, smallest
- **CacheMetrics trait** - Pluggable observability integration
- **TracingMetrics** - Metrics adapter using `tracing` crate
- **CacheEntry** - Cache entry with value, TTL, tags, dependencies, cost, and version
- **CacheResult** - Enum for cache lookups (`Hit`, `Stale`, `Miss`, `NegativeHit`)
- **CacheOptions** / **CacheOpts** - Builder for cache entry configuration
- **CacheStats** - Statistics structure for cache operations
- **Compression support** - Optional compression utilities

#### Storage Backends (`skp-cache-storage`)
- **MemoryBackend** - High-performance in-memory cache using DashMap
  - Configurable capacity limits
  - LRU-based eviction when at capacity
  - Tag index for tag-based lookups
  - Dependency index for cascade invalidation
  - Bloom filter for negative lookups optimization
  - TTL index for efficient expiration
- **RedisBackend** - Redis-backed cache using bb8 connection pool
  - Connection pooling with bb8
  - Key prefixing/namespacing
  - Tag-based operations using Redis Sets
  - Dependency tracking
  - TTL support via Redis EXPIRE
- **MultiTierBackend** - Two-tier caching (L1 Memory + L2 Redis)
  - Automatic L2 → L1 promotion on hit
  - Write-through strategy
  - Circuit breaker for L2 failures

#### Cache Manager (`skp-cache`)
- **CacheManager** - High-level API with pluggable serialization and metrics
  - Generic over backend, serializer, and metrics
  - Namespace support with key prefixing
  - TTL jitter to prevent thundering herd
  - Cascade invalidation via dependency graph
- **Request Coalescing** - Prevents cache stampede by coalescing concurrent requests for the same key
- **Stale-While-Revalidate** - Serve stale content while refreshing in background
- **Read-Through Cache** - Automatic data loading on cache miss via `Loader` trait
- **Cache Groups** - Namespaced cache operations with group-level invalidation

#### HTTP Caching (`skp-cache-http`)
- **CachedResponse** - Serializable HTTP response for caching
- **CacheControl** - HTTP Cache-Control header parsing
- **HttpCachePolicy** - HTTP caching policy with configurable defaults

#### Axum Integration (`skp-cache-axum`)
- **CacheMiddleware** - Tower Service for automatic HTTP response caching
- **CacheLayer** - Tower Layer for easy integration
- **Cache Extractor** - Axum extractor for accessing cache in handlers

#### Derive Macro (`skp-cache-derive`)
- **#[derive(CacheKey)]** - Procedural macro for automatic CacheKey implementation
  - `#[cache_key(namespace = "...")]` - Set key namespace
  - `#[cache_key(separator = "...")]` - Custom field separator (default: `:`)
  - `#[cache_key(skip)]` - Skip field in key generation

### Examples
- `basic_memory.rs` - Basic in-memory cache usage
- `redis_backend.rs` - Redis backend demonstration
- `multi_tier.rs` - L1 Memory + L2 Redis setup
- `dependency_graph.rs` - Cascade invalidation demo
- `coalescing.rs` - Request coalescing example
- `swr.rs` - Stale-while-revalidate pattern
- `read_through.rs` - Automatic loading on miss
- `cache_groups.rs` - Namespaced cache groups
- `derive_key.rs` - CacheKey derive macro usage
- `tracing_integration.rs` - Tracing observability setup

### Benchmarks
- Serialization benchmark (`benches/serialization.rs`)
- Throughput benchmark (`benches/throughput.rs`)

---

## Future Roadmap

### Planned Features
- [ ] W-TinyLFU eviction strategy
- [ ] CAMP (Cost-Aware Multi-Queue) eviction
- [ ] Adaptive eviction strategy switching
- [ ] Memcached backend
- [ ] Actix-web middleware
- [ ] Distributed lock improvements
- [ ] Redis Cluster support
- [ ] Metrics crate adapter (`metrics` crate integration)
- [ ] Graceful shutdown handling
- [ ] Property-based testing with proptest

---

[0.1.0]: https://github.com/setulab/skp-cache/releases/tag/v0.1.0

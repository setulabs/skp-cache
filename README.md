# skp-cache

[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

> Advanced, modular caching library for Rust with dependency graph invalidation, stampede protection, and framework integrations.

## ✨ Features

- **Multi-tier caching** — L1 Memory + L2 Redis with automatic promotion
- **Dependency graph invalidation** — Cascade invalidation when parent entries change
- **Stampede protection** — Request coalescing (singleflight) prevents thundering herd
- **Stale-while-revalidate** — Serve stale data while refreshing in background
- **Pluggable serialization** — JSON (default), MessagePack, Bincode
- **Metrics integration** — First-class observability via `CacheMetrics` trait
- **Framework support** — Native Axum middleware + extractors
- **TTL jitter** — Prevents synchronized expiration storms

## 📦 Installation

```toml
[dependencies]
skp-cache = "0.1"

# Optional features
skp-cache = { version = "0.1", features = ["redis", "axum", "msgpack"] }
```

### Available Features

| Feature | Description |
|---------|-------------|
| `memory` | In-memory backend (default) |
| `redis` | Redis backend with connection pooling |
| `multitier` | L1 + L2 multi-tier caching |
| `json` | JSON serialization (default) |
| `msgpack` | MessagePack serialization |
| `bincode` | Bincode serialization |
| `metrics` | Metrics crate integration |

## 🚀 Quick Start

```rust
use skp_cache::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create cache with memory backend
    let backend = MemoryBackend::new(MemoryConfig::default());
    let cache = CacheManager::new(backend);

    // Set with TTL
    cache.set("user:123", &User { name: "Alice" }, 
        CacheOpts::new().ttl_secs(300)
    ).await?;

    // Get
    match cache.get::<User>("user:123").await? {
        CacheResult::Hit(entry) => println!("Found: {:?}", entry.value),
        CacheResult::Miss => println!("Not found"),
        _ => {}
    }

    Ok(())
}
```

## 🔗 Dependency Graph Invalidation

Link cache entries so invalidating a parent cascades to dependents:

```rust
// User depends on tenant
cache.set("user:123", &user, 
    CacheOpts::new()
        .depends_on(["tenant:1"])
).await?;

// Posts depend on user
cache.set("user:123:posts", &posts,
    CacheOpts::new()
        .depends_on(["user:123"])
).await?;

// Invalidating tenant cascades to user and posts!
let count = cache.invalidate("tenant:1").await?;
// count == 3 (tenant + user + posts)
```

## 🛡️ Stampede Protection

Concurrent requests for the same missing key trigger only ONE computation:

```rust
// 1000 concurrent calls = 1 database query
let user = cache.get_or_compute(
    format!("user:{}", id),
    || async { db.fetch_user(id).await },
    Some(CacheOpts::new().ttl_secs(300).into())
).await?;
```

## ⏱️ Stale-While-Revalidate

Serve slightly stale data instantly while refreshing in background:

```rust
cache.set("dashboard", &data,
    CacheOpts::new()
        .ttl_secs(60)   // Fresh for 60s
        .swr_secs(300)  // Stale but usable for 5 more minutes
).await?;
```

## 🌐 Axum Integration

```rust
use skp_cache_axum::{CacheLayer, Cache};

let app = Router::new()
    .with_state(cache.clone())
    .route("/users/:id", get(get_user))
    .layer(CacheLayer::new(cache));

async fn get_user(
    Path(id): Path<u64>,
    Cache(cache): Cache<MemoryBackend, JsonSerializer, NoopMetrics>,
) -> impl IntoResponse {
    // Use cache directly in handlers
    let user = cache.get::<User>(&format!("user:{}", id)).await?;
    Json(user)
}
```

## 📊 Metrics

Integrate with any metrics system:

```rust
use skp_cache::{CacheManager, MetricsCrateAdapter};

let cache = CacheManager::with_serializer_and_metrics(
    backend,
    JsonSerializer,
    MetricsCrateAdapter::new("skp_cache"),
    config,
);

// Emits: skp_cache_hits_total, skp_cache_misses_total, etc.
```

## 📁 Examples

Run the examples to see features in action:

```bash
# Dependency graph invalidation
cargo run -p skp-cache --example dependency_graph

# Request coalescing
cargo run -p skp-cache --example coalescing

# Stale-while-revalidate
cargo run -p skp-cache --example swr

# Axum integration
cargo run -p skp-cache-axum --example proper_axum
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Application                        │
│  (Axum Middleware / Standalone / Actix)             │
└───────────────────────┬─────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────┐
│                  CacheManager                        │
│  ┌─────────┐  ┌───────────┐  ┌──────────────────┐  │
│  │Coalescer│  │ Serializer│  │ Metrics Collector│  │
│  └─────────┘  └───────────┘  └──────────────────┘  │
└───────────────────────┬─────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────┐
│                    Backends                          │
│  ┌─────────┐  ┌─────────┐  ┌───────────────────┐   │
│  │ Memory  │  │  Redis  │  │    MultiTier      │   │
│  │(DashMap)│  │(bb8+Lua)│  │ (L1 Mem + L2 Red) │   │
│  └─────────┘  └─────────┘  └───────────────────┘   │
└─────────────────────────────────────────────────────┘
```

## 📜 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
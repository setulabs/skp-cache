# skp-cache Benchmark Results

> Benchmarks run on Linux with `cargo bench --all-features`  
> Last updated: January 2026

## Table of Contents

- [Cache Throughput](#1-cache-throughput-benchmarks)
- [Serialization](#2-serializer-benchmarks)
- [Compression](#3-compression-benchmarks)
- [Recommendations](#-recommended-configurations)

---

## 1. Cache Throughput Benchmarks

Core cache operations performance:

| Operation | Time | Throughput | Notes |
|-----------|------|------------|-------|
| `set/small_value` | ~900 ns | 1.08 M ops/s | Small struct caching |
| `set/medium_value` | ~1.6 µs | 608 K ops/s | 1KB value caching |
| `get/hit` | ~717 ns | 1.39 M ops/s | Cache hit latency |
| `get/miss` | ~550 ns | 1.82 M ops/s | Cache miss (faster - no deserialization) |
| `mixed/80_read_20_write` | ~82 µs | 1.21 M ops/s | Realistic workload (100 ops) |

### Running Throughput Benchmarks

```bash
cargo bench -p skp-cache
```

---

## 2. Serializer Benchmarks

### Serialization Speed

Time to serialize data of various sizes:

| Serializer | Small (~50B) | Medium (~500B) | Large (~15KB) | Winner |
|------------|--------------|----------------|---------------|--------|
| **JSON** | 118 ns | 929 ns | 13.3 µs | - |
| **MsgPack** | 80 ns | 1.44 µs | 11.9 µs | Large data |
| **Bincode** | **53 ns** | **599 ns** | **4.5 µs** | ⭐ Overall fastest |

### Deserialization Speed

Time to deserialize data:

| Serializer | Small | Medium | Large | Throughput (Large) |
|------------|-------|--------|-------|-------------------|
| **JSON** | 225 ns | 2.88 µs | 30.9 µs | 468 MiB/s |
| **MsgPack** | 93 ns | 1.52 µs | 20.0 µs | 645 MiB/s |
| **Bincode** | **100 ns** | 1.56 µs | **19.2 µs** | **677 MiB/s** ⭐ |

### Serialized Size Comparison

For a large test data structure (~15KB logical size):

| Format | Serialized Size | Reduction vs JSON |
|--------|-----------------|-------------------|
| JSON | 15,172 bytes | baseline |
| MsgPack | 13,564 bytes | 10.6% smaller |
| Bincode | 13,592 bytes | 10.4% smaller |
| JSON + Zstd | 2,069 bytes | **86.4% smaller** ⭐ |

### Running Serialization Benchmarks

```bash
cargo bench -p skp-cache-core --all-features
```

---

## 3. Compression Benchmarks

### Compression Speed by Zstd Level

| Level | 1KB | 10KB | 100KB | Trade-off |
|-------|-----|------|-------|-----------|
| **Noop** | 27 ns (36 GiB/s) | 108 ns (88 GiB/s) | 2.2 µs (43 GiB/s) | No compression |
| **Zstd L1** | 2.4 µs (403 MiB/s) | 3.1 µs (3.0 GiB/s) | 10.8 µs (8.8 GiB/s) | ⭐ Best speed/ratio |
| **Zstd L3** | 12 µs (81 MiB/s) | 13.2 µs (741 MiB/s) | 20.4 µs (4.7 GiB/s) | Balanced |
| **Zstd L9** | 299 µs (3.3 MiB/s) | 302 µs (32 MiB/s) | 334 µs (289 MiB/s) | Max compression |

### Decompression Speed

Decompression is consistently fast regardless of compression level used:

| Data Size | Time | Throughput |
|-----------|------|------------|
| 1KB | 757 ns | 1.26 GiB/s |
| 10KB | 2.0 µs | 4.73 GiB/s |
| 100KB | 15.0 µs | 6.37 GiB/s |

---

## 📊 Recommended Configurations

| Use Case | Serializer | Compression | Rationale |
|----------|------------|-------------|-----------|
| **🚀 High Performance** | Bincode | None/L1 | 2-3x faster than JSON, minimal overhead |
| **⚖️ Balanced (Default)** | Bincode | Zstd L1 | Fast serialize + 60-80% size reduction |
| **💾 Storage Optimized** | Bincode | Zstd L3 | Good compression, reasonable speed |
| **🌐 API/Cross-platform** | JSON | Zstd L1 | Human readable, debuggable, compressed |
| **📡 Network Transfer** | MsgPack | Zstd L1 | Compact + cross-language compatible |
| **🏗️ Maximum Compression** | Bincode | Zstd L9 | Archival/cold storage (slow but tiny) |

---

## 🏆 Best Combinations Summary

| Priority | Recommendation | Expected Performance |
|----------|----------------|---------------------|
| **Speed First** | `Bincode` + `NoopCompressor` | ~53-4500 ns serialize, 677 MiB/s deserialize |
| **Ideal Balance** | `Bincode` + `ZstdCompressor::new(1)` | ~3-11 µs overhead, 3-9 GiB/s compress, 86% size reduction |
| **Size First** | `Bincode` + `ZstdCompressor::new(3)` | ~13-20 µs overhead, 86%+ size reduction |
| **Interoperability** | `MsgPack` + `ZstdCompressor::new(1)` | Cross-language safe, fast, compact |

---

## Key Insights

1. **Bincode is ~2-3x faster** than JSON for both serialization and deserialization
2. **Zstd L1 provides excellent compression** (60-80%) with minimal latency (~3-11 µs for typical payloads)
3. **Decompression is always fast** (~1.3-6.4 GiB/s) regardless of compression level
4. **Avoid Zstd L9** for real-time caching (10-100x slower than L1)
5. **JSON+Zstd** achieves 86.4% compression - ideal for network/storage constrained environments

---

## Code Examples

### High Performance Setup (Bincode, No Compression)

```rust
use skp_cache::{CacheManager, BincodeSerializer, NoopCompressor};

let cache = CacheManager::builder()
    .serializer(BincodeSerializer)
    .compressor(NoopCompressor)
    .build();
```

### Balanced Setup (Bincode + Zstd L1)

```rust
use skp_cache::{CacheManager, BincodeSerializer, ZstdCompressor};

let cache = CacheManager::builder()
    .serializer(BincodeSerializer)
    .compressor(ZstdCompressor::new(1))
    .build();
```

### Cross-Platform Setup (MsgPack + Zstd L1)

```rust
use skp_cache::{CacheManager, MsgPackSerializer, ZstdCompressor};

let cache = CacheManager::builder()
    .serializer(MsgPackSerializer)
    .compressor(ZstdCompressor::new(1))
    .build();
```

---

## Running All Benchmarks

```bash
# All benchmarks
cargo bench --all-features

# Throughput only
cargo bench -p skp-cache

# Serialization & compression only
cargo bench -p skp-cache-core --all-features
```

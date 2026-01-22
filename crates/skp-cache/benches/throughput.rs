//! Benchmarks for skp-cache throughput and operations

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use skp_cache::{CacheManager, CacheOpts, CacheResult, MemoryBackend, MemoryConfig};
use std::hint::black_box;
use tokio::runtime::Runtime;

fn create_cache() -> CacheManager<MemoryBackend> {
    let backend = MemoryBackend::new(MemoryConfig::default());
    CacheManager::new(backend)
}

fn bench_set(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = create_cache();

    let mut group = c.benchmark_group("set");
    group.throughput(Throughput::Elements(1));

    group.bench_function("small_value", |b| {
        b.iter(|| {
            rt.block_on(async {
                cache
                    .set(black_box("key"), black_box(&42i32), CacheOpts::new())
                    .await
                    .unwrap();
            });
        });
    });

    group.bench_function("medium_value", |b| {
        let value = "x".repeat(1024); // 1KB
        b.iter(|| {
            rt.block_on(async {
                cache
                    .set(black_box("key"), black_box(&value), CacheOpts::new())
                    .await
                    .unwrap();
            });
        });
    });

    group.finish();
}

fn bench_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = create_cache();

    // Pre-populate
    rt.block_on(async {
        cache
            .set("key", &42i32, CacheOpts::new().ttl_secs(3600))
            .await
            .unwrap();
    });

    let mut group = c.benchmark_group("get");
    group.throughput(Throughput::Elements(1));

    group.bench_function("hit", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result: CacheResult<i32> = cache.get(black_box("key")).await.unwrap();
                black_box(result);
            });
        });
    });

    group.bench_function("miss", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result: CacheResult<i32> = cache.get(black_box("nonexistent")).await.unwrap();
                black_box(result);
            });
        });
    });

    group.finish();
}

fn bench_mixed_workload(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = create_cache();

    // Pre-populate some keys
    rt.block_on(async {
        for i in 0..100 {
            cache
                .set(&format!("key:{}", i), &i, CacheOpts::new().ttl_secs(3600))
                .await
                .unwrap();
        }
    });

    let mut group = c.benchmark_group("mixed");
    group.throughput(Throughput::Elements(100));

    group.bench_function("80_read_20_write", |b| {
        let mut i = 0u64;
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..100 {
                    i = i.wrapping_add(1);
                    if i % 5 == 0 {
                        // 20% writes
                        cache
                            .set(&format!("key:{}", i % 100), &i, CacheOpts::new())
                            .await
                            .unwrap();
                    } else {
                        // 80% reads
                        let _: CacheResult<u64> =
                            cache.get(&format!("key:{}", i % 100)).await.unwrap();
                    }
                }
            });
        });
    });

    group.finish();
}

criterion_group!(benches, bench_set, bench_get, bench_mixed_workload);
criterion_main!(benches);

//! Benchmarks for serialization and compression methods

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde::{Deserialize, Serialize};
use skp_cache_core::{Compressor, JsonSerializer, NoopCompressor, Serializer};
use std::hint::black_box;

#[cfg(feature = "msgpack")]
use skp_cache_core::MsgPackSerializer;

#[cfg(feature = "bincode")]
use skp_cache_core::BincodeSerializer;

#[cfg(feature = "compression")]
use skp_cache_core::ZstdCompressor;

/// Test data structure for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestData {
    id: u64,
    name: String,
    values: Vec<i32>,
    metadata: std::collections::HashMap<String, String>,
}

impl TestData {
    fn small() -> Self {
        Self {
            id: 12345,
            name: "test".to_string(),
            values: vec![1, 2, 3],
            metadata: std::collections::HashMap::new(),
        }
    }

    fn medium() -> Self {
        let mut metadata = std::collections::HashMap::new();
        for i in 0..10 {
            metadata.insert(format!("key_{}", i), format!("value_{}", i));
        }
        Self {
            id: 12345,
            name: "test medium data structure".to_string(),
            values: (0..100).collect(),
            metadata,
        }
    }

    fn large() -> Self {
        let mut metadata = std::collections::HashMap::new();
        for i in 0..100 {
            metadata.insert(format!("key_{}", i), "x".repeat(100));
        }
        Self {
            id: 12345,
            name: "test large data structure with lots of content".to_string(),
            values: (0..1000).collect(),
            metadata,
        }
    }
}

fn bench_serializers(c: &mut Criterion) {
    let test_cases = vec![
        ("small", TestData::small()),
        ("medium", TestData::medium()),
        ("large", TestData::large()),
    ];

    let mut group = c.benchmark_group("serialize");

    for (name, data) in &test_cases {
        // JSON
        group.bench_with_input(BenchmarkId::new("json", name), data, |b, data| {
            let serializer = JsonSerializer;
            b.iter(|| {
                let bytes = serializer.serialize(black_box(data)).unwrap();
                black_box(bytes);
            });
        });

        // MessagePack
        #[cfg(feature = "msgpack")]
        group.bench_with_input(BenchmarkId::new("msgpack", name), data, |b, data| {
            let serializer = MsgPackSerializer;
            b.iter(|| {
                let bytes = serializer.serialize(black_box(data)).unwrap();
                black_box(bytes);
            });
        });

        // Bincode
        #[cfg(feature = "bincode")]
        group.bench_with_input(BenchmarkId::new("bincode", name), data, |b, data| {
            let serializer = BincodeSerializer;
            b.iter(|| {
                let bytes = serializer.serialize(black_box(data)).unwrap();
                black_box(bytes);
            });
        });
    }

    group.finish();
}

fn bench_deserializers(c: &mut Criterion) {
    let test_cases = vec![
        ("small", TestData::small()),
        ("medium", TestData::medium()),
        ("large", TestData::large()),
    ];

    let mut group = c.benchmark_group("deserialize");

    for (name, data) in &test_cases {
        // JSON
        let json_serializer = JsonSerializer;
        let json_bytes = json_serializer.serialize(&data).unwrap();
        group.throughput(Throughput::Bytes(json_bytes.len() as u64));
        group.bench_with_input(BenchmarkId::new("json", name), &json_bytes, |b, bytes| {
            b.iter(|| {
                let result: TestData = json_serializer.deserialize(black_box(bytes)).unwrap();
                black_box(result);
            });
        });

        // MessagePack
        #[cfg(feature = "msgpack")]
        {
            let msgpack_serializer = MsgPackSerializer;
            let msgpack_bytes = msgpack_serializer.serialize(&data).unwrap();
            group.throughput(Throughput::Bytes(msgpack_bytes.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("msgpack", name),
                &msgpack_bytes,
                |b, bytes| {
                    b.iter(|| {
                        let result: TestData =
                            msgpack_serializer.deserialize(black_box(bytes)).unwrap();
                        black_box(result);
                    });
                },
            );
        }

        // Bincode
        #[cfg(feature = "bincode")]
        {
            let bincode_serializer = BincodeSerializer;
            let bincode_bytes = bincode_serializer.serialize(&data).unwrap();
            group.throughput(Throughput::Bytes(bincode_bytes.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("bincode", name),
                &bincode_bytes,
                |b, bytes| {
                    b.iter(|| {
                        let result: TestData =
                            bincode_serializer.deserialize(black_box(bytes)).unwrap();
                        black_box(result);
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_compression(c: &mut Criterion) {
    // Generate test data of various sizes
    let sizes = vec![
        ("1KB", vec![0u8; 1024]),
        ("10KB", vec![0u8; 10 * 1024]),
        ("100KB", vec![0u8; 100 * 1024]),
    ];

    // Also test with realistic data (JSON serialized)
    let json_serializer = JsonSerializer;
    let large_data = TestData::large();
    let json_bytes = json_serializer.serialize(&large_data).unwrap();

    let mut group = c.benchmark_group("compress");

    // Test with zeroed data (highly compressible)
    for (name, data) in &sizes {
        group.throughput(Throughput::Bytes(data.len() as u64));

        // Noop (baseline)
        group.bench_with_input(BenchmarkId::new("noop", name), data, |b, data| {
            let compressor = NoopCompressor;
            b.iter(|| {
                let result = compressor.compress(black_box(data)).unwrap();
                black_box(result);
            });
        });

        // Zstd level 1 (fastest)
        #[cfg(feature = "compression")]
        group.bench_with_input(BenchmarkId::new("zstd_l1", name), data, |b, data| {
            let compressor = ZstdCompressor::new(1);
            b.iter(|| {
                let result = compressor.compress(black_box(data)).unwrap();
                black_box(result);
            });
        });

        // Zstd level 3 (default)
        #[cfg(feature = "compression")]
        group.bench_with_input(BenchmarkId::new("zstd_l3", name), data, |b, data| {
            let compressor = ZstdCompressor::new(3);
            b.iter(|| {
                let result = compressor.compress(black_box(data)).unwrap();
                black_box(result);
            });
        });

        // Zstd level 9 (high compression)
        #[cfg(feature = "compression")]
        group.bench_with_input(BenchmarkId::new("zstd_l9", name), data, |b, data| {
            let compressor = ZstdCompressor::new(9);
            b.iter(|| {
                let result = compressor.compress(black_box(data)).unwrap();
                black_box(result);
            });
        });
    }

    // Test with realistic JSON data
    group.throughput(Throughput::Bytes(json_bytes.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("zstd_l3", "json_data"),
        &json_bytes,
        |b, data| {
            #[cfg(feature = "compression")]
            {
                let compressor = ZstdCompressor::new(3);
                b.iter(|| {
                    let result = compressor.compress(black_box(data)).unwrap();
                    black_box(result);
                });
            }
            #[cfg(not(feature = "compression"))]
            {
                let _ = data;
                b.iter(|| {});
            }
        },
    );

    group.finish();
}

fn bench_decompression(c: &mut Criterion) {
    #[cfg(feature = "compression")]
    {
        let sizes = vec![
            ("1KB", vec![0u8; 1024]),
            ("10KB", vec![0u8; 10 * 1024]),
            ("100KB", vec![0u8; 100 * 1024]),
        ];

        let mut group = c.benchmark_group("decompress");

        for (name, data) in &sizes {
            let compressor = ZstdCompressor::new(3);
            let compressed = compressor.compress(data).unwrap();

            group.throughput(Throughput::Bytes(data.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("zstd_l3", name),
                &compressed,
                |b, compressed| {
                    b.iter(|| {
                        let result = compressor.decompress(black_box(compressed)).unwrap();
                        black_box(result);
                    });
                },
            );
        }

        group.finish();
    }

    #[cfg(not(feature = "compression"))]
    {
        // Empty benchmark when compression feature is not enabled
        let _ = c;
    }
}

fn bench_serialized_size(c: &mut Criterion) {
    // This benchmark just reports sizes, not timing
    let data = TestData::large();

    let json_serializer = JsonSerializer;
    let json_bytes = json_serializer.serialize(&data).unwrap();
    println!("\n=== Serialized Sizes (large TestData) ===");
    println!("JSON:     {} bytes", json_bytes.len());

    #[cfg(feature = "msgpack")]
    {
        let msgpack_serializer = MsgPackSerializer;
        let msgpack_bytes = msgpack_serializer.serialize(&data).unwrap();
        println!("MsgPack:  {} bytes", msgpack_bytes.len());
    }

    #[cfg(feature = "bincode")]
    {
        let bincode_serializer = BincodeSerializer;
        let bincode_bytes = bincode_serializer.serialize(&data).unwrap();
        println!("Bincode:  {} bytes", bincode_bytes.len());
    }

    #[cfg(feature = "compression")]
    {
        let compressor = ZstdCompressor::new(3);
        let compressed = compressor.compress(&json_bytes).unwrap();
        println!(
            "JSON+Zstd: {} bytes ({:.1}% of original)",
            compressed.len(),
            (compressed.len() as f64 / json_bytes.len() as f64) * 100.0
        );
    }

    // Dummy benchmark to satisfy criterion
    let mut group = c.benchmark_group("size_report");
    group.bench_function("noop", |b| b.iter(|| {}));
    group.finish();
}

criterion_group!(
    benches,
    bench_serializers,
    bench_deserializers,
    bench_compression,
    bench_decompression,
    bench_serialized_size,
);
criterion_main!(benches);

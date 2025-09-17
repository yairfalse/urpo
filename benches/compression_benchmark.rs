//! Compression benchmark for trace storage performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::{Duration, SystemTime};
use urpo_lib::core::{ServiceName, SpanBuilder, SpanId, SpanStatus, TraceId};
use urpo_lib::storage::compression::{CompressionEngine, CompressionLevel};

fn generate_test_spans(count: usize) -> Vec<urpo_lib::core::Span> {
    let mut spans = Vec::with_capacity(count);
    let base_time = SystemTime::now();

    for i in 0..count {
        let span = SpanBuilder::default()
            .trace_id(TraceId::new(format!("trace-{}", i % 100)).unwrap()) // 100 unique traces
            .span_id(SpanId::new(format!("span-{}", i)).unwrap())
            .service_name(ServiceName::new(format!("service-{}", i % 10)).unwrap()) // 10 unique services
            .operation_name(&format!("operation-{}", i % 20)) // 20 unique operations
            .start_time(base_time + Duration::from_millis(i as u64))
            .duration(Duration::from_millis(10 + (i % 1000) as u64))
            .status(if i % 100 == 0 {
                SpanStatus::Error("test error".to_string())
            } else {
                SpanStatus::Ok
            })
            .attribute("http.method", "GET")
            .attribute("http.status_code", "200")
            .attribute("service.version", "1.0.0")
            .build_default();

        spans.push(span);
    }

    spans
}

fn bench_compression_levels(c: &mut Criterion) {
    let engine = CompressionEngine::new();
    let spans = generate_test_spans(1000);

    let mut group = c.benchmark_group("compression_levels");

    for level in [
        CompressionLevel::None,
        CompressionLevel::Fast,
        CompressionLevel::Balanced,
        CompressionLevel::Maximum,
    ] {
        group.bench_with_input(
            BenchmarkId::new("compress", format!("{:?}", level)),
            &level,
            |b, &level| {
                b.iter(|| {
                    let compressed = engine
                        .compress_spans(black_box(&spans), black_box(level))
                        .unwrap();
                    black_box(compressed);
                })
            },
        );
    }

    group.finish();
}

fn bench_compression_ratios(c: &mut Criterion) {
    let engine = CompressionEngine::new();

    c.bench_function("compression_ratio_analysis", |b| {
        b.iter(|| {
            let spans = black_box(generate_test_spans(1000));

            // Test all compression levels
            for level in [
                CompressionLevel::None,
                CompressionLevel::Fast,
                CompressionLevel::Balanced,
                CompressionLevel::Maximum,
            ] {
                let compressed = engine.compress_spans(&spans, level).unwrap();
                let ratio = compressed.compression_ratio();
                black_box(ratio);
            }
        })
    });
}

fn bench_batch_sizes(c: &mut Criterion) {
    let engine = CompressionEngine::new();
    let mut group = c.benchmark_group("batch_sizes");

    for size in [100, 500, 1000, 5000, 10000] {
        group.bench_with_input(BenchmarkId::new("fast_compression", size), &size, |b, &size| {
            let spans = generate_test_spans(size);
            b.iter(|| {
                let compressed = engine
                    .compress_spans(black_box(&spans), CompressionLevel::Fast)
                    .unwrap();
                black_box(compressed);
            })
        });

        group.bench_with_input(
            BenchmarkId::new("balanced_compression", size),
            &size,
            |b, &size| {
                let spans = generate_test_spans(size);
                b.iter(|| {
                    let compressed = engine
                        .compress_spans(black_box(&spans), CompressionLevel::Balanced)
                        .unwrap();
                    black_box(compressed);
                })
            },
        );
    }

    group.finish();
}

fn bench_decompression(c: &mut Criterion) {
    let engine = CompressionEngine::new();
    let spans = generate_test_spans(1000);

    let mut group = c.benchmark_group("decompression");

    // Pre-compress with different levels
    let none_compressed = engine
        .compress_spans(&spans, CompressionLevel::None)
        .unwrap();
    let fast_compressed = engine
        .compress_spans(&spans, CompressionLevel::Fast)
        .unwrap();

    group.bench_function("none", |b| {
        b.iter(|| {
            let decompressed = engine
                .decompress_spans(black_box(&none_compressed))
                .unwrap();
            black_box(decompressed);
        })
    });

    group.bench_function("fast", |b| {
        b.iter(|| {
            let decompressed = engine
                .decompress_spans(black_box(&fast_compressed))
                .unwrap();
            black_box(decompressed);
        })
    });

    group.finish();
}

fn bench_compression_throughput(c: &mut Criterion) {
    let engine = CompressionEngine::new();

    c.bench_function("compression_throughput_1mb", |b| {
        // Generate approximately 1MB of span data
        let spans = generate_test_spans(2000); // Rough estimate for 1MB

        b.iter(|| {
            let compressed = engine
                .compress_spans(black_box(&spans), CompressionLevel::Fast)
                .unwrap();
            black_box(compressed);
        })
    });
}

fn bench_memory_efficiency(c: &mut Criterion) {
    let engine = CompressionEngine::new();

    c.bench_function("memory_efficiency_test", |b| {
        b.iter(|| {
            // Generate spans with lots of repeated data (good for compression)
            let mut spans = Vec::new();
            for i in 0..1000 {
                let span = SpanBuilder::default()
                    .trace_id(TraceId::new("repeated-trace-id".to_string()).unwrap())
                    .span_id(SpanId::new(format!("span-{}", i)).unwrap())
                    .service_name(ServiceName::new("repeated-service".to_string()).unwrap())
                    .operation_name("repeated-operation")
                    .build_default();
                spans.push(span);
            }

            // Compress with string pooling (balanced level)
            let compressed = engine
                .compress_spans(black_box(&spans), CompressionLevel::Balanced)
                .unwrap();

            black_box(compressed);
        })
    });
}

criterion_group!(
    benches,
    bench_compression_levels,
    bench_compression_ratios,
    bench_batch_sizes,
    bench_decompression,
    bench_compression_throughput,
    bench_memory_efficiency
);
criterion_main!(benches);

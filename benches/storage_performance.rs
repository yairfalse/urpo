//! Comprehensive performance benchmarks for the ultra-fast storage engine.
//!
//! Run with: cargo bench --bench storage_performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use urpo_lib::core::{ServiceName, Span, SpanId, SpanKind, SpanStatus, TraceId};
use urpo_lib::storage::{
    CompactSpan, HotTraceRing, StringIntern, TieredConfig, TieredStorageEngine, UltraFastStorage,
};

/// Generate a test span with specific parameters.
fn generate_test_span(trace_num: usize, span_num: usize) -> Span {
    Span::builder()
        .trace_id(TraceId::new(format!("{:032x}", trace_num)).unwrap())
        .span_id(SpanId::new(format!("{:016x}", span_num)).unwrap())
        .service_name(ServiceName::new(format!("service-{}", trace_num % 100)).unwrap())
        .operation_name(format!("operation-{}", span_num % 50))
        .start_time(SystemTime::now())
        .duration(Duration::from_micros((span_num % 1000) as u64))
        .kind(match span_num % 5 {
            0 => SpanKind::Server,
            1 => SpanKind::Client,
            2 => SpanKind::Producer,
            3 => SpanKind::Consumer,
            _ => SpanKind::Internal,
        })
        .status(if span_num % 20 == 0 {
            SpanStatus::Error("test error".to_string())
        } else {
            SpanStatus::Ok
        })
        .build()
        .unwrap()
}

/// Benchmark CompactSpan conversion performance.
fn bench_compact_span_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("compact_span");
    let string_intern = StringIntern::new();

    group.bench_function("from_span", |b| {
        let span = generate_test_span(1, 1);
        b.iter(|| {
            let compact = CompactSpan::from_span(black_box(&span), &string_intern);
            black_box(compact);
        })
    });

    group.bench_function("size_check", |b| {
        b.iter(|| {
            assert_eq!(std::mem::size_of::<CompactSpan>(), 64);
            assert_eq!(std::mem::align_of::<CompactSpan>(), 64);
        })
    });

    group.finish();
}

/// Benchmark string interning performance.
fn bench_string_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_interning");

    group.bench_function("intern_new_service", |b| {
        let intern = StringIntern::new();
        let mut counter = 0;
        b.iter(|| {
            let service = ServiceName::new(format!("service-{}", counter)).unwrap();
            let idx = intern.intern_service(black_box(&service));
            counter += 1;
            black_box(idx);
        })
    });

    group.bench_function("intern_existing_service", |b| {
        let intern = StringIntern::new();
        let service = ServiceName::new("test-service".to_string()).unwrap();
        intern.intern_service(&service); // Pre-intern

        b.iter(|| {
            let idx = intern.intern_service(black_box(&service));
            black_box(idx);
        })
    });

    group.finish();
}

/// Benchmark hot ring buffer performance.
fn bench_hot_ring_buffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("hot_ring_buffer");

    for capacity in [1000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*capacity as u64));

        group.bench_with_input(BenchmarkId::from_parameter(capacity), capacity, |b, &capacity| {
            let ring = HotTraceRing::new(capacity);
            let string_intern = StringIntern::new();
            let span = generate_test_span(1, 1);
            let compact = CompactSpan::from_span(&span, &string_intern);

            b.iter(|| {
                let success = ring.try_push(black_box(compact.clone()));
                black_box(success);
            })
        });
    }

    group.finish();
}

/// Benchmark ultra-fast storage ingestion.
fn bench_ultra_fast_ingestion(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultra_fast_ingestion");

    for batch_size in [100, 1000, 10_000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("batch", batch_size),
            batch_size,
            |b, &batch_size| {
                let storage = UltraFastStorage::new(100_000);
                let spans: Vec<Span> = (0..batch_size)
                    .map(|i| generate_test_span(i / 100, i))
                    .collect();

                b.iter(|| {
                    for span in &spans {
                        let _ = storage.ingest_span(black_box(span.clone()));
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark tiered storage engine performance.
fn bench_tiered_storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("tiered_storage");

    let temp_dir = TempDir::new().unwrap();
    let mut config = TieredConfig::default();
    config.storage_dir = temp_dir.path().to_path_buf();
    config.hot_capacity = 10_000;

    let engine = TieredStorageEngine::new(config).unwrap();

    group.bench_function("ingest_single", |b| {
        let span = generate_test_span(1, 1);
        b.iter(|| {
            let _ = engine.ingest(black_box(span.clone()));
        })
    });

    group.bench_function("query_by_service", |b| {
        // Pre-populate with test data
        for i in 0..1000 {
            let span = generate_test_span(i / 10, i);
            let _ = engine.ingest(span);
        }

        let service = ServiceName::new("service-1".to_string()).unwrap();
        b.iter(|| {
            let results = engine
                .query(Some(black_box(&service)), None, None, 100)
                .unwrap();
            black_box(results);
        })
    });

    group.finish();
}

/// Benchmark query performance across different data sizes.
fn bench_query_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_scaling");

    for span_count in [1000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*span_count as u64));

        group.bench_with_input(
            BenchmarkId::new("spans", span_count),
            span_count,
            |b, &span_count| {
                let storage = UltraFastStorage::new(span_count + 1000);

                // Pre-populate
                for i in 0..span_count {
                    let span = generate_test_span(i / 100, i);
                    let _ = storage.ingest_span(span);
                }

                let service = ServiceName::new("service-1".to_string()).unwrap();
                b.iter(|| {
                    let results = storage.query_by_service(black_box(&service));
                    black_box(results);
                })
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage patterns.
fn bench_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");

    group.bench_function("memory_per_span", |b| {
        b.iter(|| {
            // CompactSpan is exactly 64 bytes
            let span_size = std::mem::size_of::<CompactSpan>();

            // Calculate memory for 1 million spans
            let million_spans_mb = (span_size * 1_000_000) / (1024 * 1024);

            // Should be ~61 MB for 1M spans (meeting <100MB target)
            assert!(million_spans_mb < 100);
            black_box(million_spans_mb);
        })
    });

    group.finish();
}

/// Benchmark concurrent ingestion performance.
fn bench_concurrent_ingestion(c: &mut Criterion) {
    use std::sync::Arc;
    use std::thread;

    let mut group = c.benchmark_group("concurrent_ingestion");

    group.bench_function("4_threads_10k_spans", |b| {
        b.iter(|| {
            let storage = Arc::new(UltraFastStorage::new(100_000));
            let mut handles = vec![];

            for thread_id in 0..4 {
                let storage_clone = storage.clone();
                let handle = thread::spawn(move || {
                    for i in 0..2500 {
                        let span = generate_test_span(thread_id * 2500 + i, i);
                        let _ = storage_clone.ingest_span(span);
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_compact_span_conversion,
    bench_string_interning,
    bench_hot_ring_buffer,
    bench_ultra_fast_ingestion,
    bench_tiered_storage,
    bench_query_scaling,
    bench_memory_efficiency,
    bench_concurrent_ingestion
);

criterion_main!(benches);

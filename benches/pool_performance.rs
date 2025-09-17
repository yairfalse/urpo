//! Pool performance benchmark showing zero-allocation benefits

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use urpo_lib::core::{ServiceName, Span, SpanId, TraceId};
use urpo_lib::storage::simple_pool::{get_span, SimpleSpanPool};

fn bench_allocation_vs_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_vs_pool");

    // Benchmark regular allocation
    group.bench_function("allocation", |b| {
        b.iter(|| {
            let span = Span::builder()
                .trace_id(TraceId::new("trace_123".to_string()).unwrap())
                .span_id(SpanId::new("span_456".to_string()).unwrap())
                .service_name(ServiceName::new("test-service".to_string()).unwrap())
                .operation_name("test-operation")
                .build()
                .unwrap();
            black_box(span);
        })
    });

    // Benchmark pool usage
    let pool = SimpleSpanPool::new(1000);
    group.bench_function("pool", |b| {
        b.iter(|| {
            if let Some(pooled) = pool.get() {
                let span = pooled.as_ref();
                black_box(span);
                // Automatic return to pool on drop
            }
        })
    });

    // Benchmark global pool
    group.bench_function("global_pool", |b| {
        b.iter(|| {
            if let Some(pooled) = get_span() {
                let span = pooled.as_ref();
                black_box(span);
            }
        })
    });

    group.finish();
}

fn bench_batch_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_processing");

    for size in [100, 1000, 10000].iter() {
        // With allocation
        group.bench_with_input(BenchmarkId::new("allocation", size), size, |b, &size| {
            b.iter(|| {
                let mut spans = Vec::with_capacity(size);
                for i in 0..size {
                    let span = Span::builder()
                        .trace_id(TraceId::new(format!("trace_{}", i)).unwrap())
                        .span_id(SpanId::new(format!("span_{}", i)).unwrap())
                        .service_name(ServiceName::new("service".to_string()).unwrap())
                        .operation_name("operation")
                        .build()
                        .unwrap();
                    spans.push(span);
                }
                black_box(&spans);
            })
        });

        // With pool
        let pool = SimpleSpanPool::new(size + 100);
        group.bench_with_input(BenchmarkId::new("pool", size), size, |b, &size| {
            b.iter(|| {
                let mut spans = Vec::new();
                for _ in 0..size {
                    if let Some(pooled) = pool.get() {
                        spans.push(pooled);
                    }
                }
                black_box(&spans);
                // Spans return to pool when dropped
            })
        });
    }

    group.finish();
}

fn bench_pool_performance(c: &mut Criterion) {
    let pool = SimpleSpanPool::new(10000);

    c.bench_function("pool_hit_rate", |b| {
        b.iter(|| {
            // Test sustained pool usage
            let mut spans = Vec::new();
            for _ in 0..100 {
                if let Some(pooled) = pool.get() {
                    spans.push(pooled);
                }
            }
            black_box(&spans);
            // All spans return to pool
        })
    });

    // Test pool exhaustion behavior
    c.bench_function("pool_exhaustion", |b| {
        b.iter(|| {
            let mut spans = Vec::new();
            // Try to get more spans than pool capacity
            for _ in 0..20000 {
                if let Some(pooled) = pool.get() {
                    spans.push(pooled);
                } else {
                    break; // Pool exhausted
                }
            }
            black_box(&spans);
        })
    });
}

criterion_group!(
    benches,
    bench_allocation_vs_pool,
    bench_batch_processing,
    bench_pool_performance
);
criterion_main!(benches);

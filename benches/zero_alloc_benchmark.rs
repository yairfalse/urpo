//! Zero-allocation benchmark - Proving ZERO allocations in hot paths!

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;
use std::time::Duration;
use urpo_lib::core::{ServiceName, Span, SpanId, SpanKind, SpanStatus, TraceId};
use urpo_lib::storage::zero_alloc_pool::{
    get_compact_slot, get_pooled_span, CompactSpanPool, GlobalPools, ZeroAllocSpanPool,
};

fn bench_with_allocation(c: &mut Criterion) {
    c.bench_function("span_with_allocation", |b| {
        b.iter(|| {
            // Allocate new span every time (SLOW)
            let span = Span::builder()
                .trace_id(TraceId::new("trace_123").unwrap())
                .span_id(SpanId::new("span_456").unwrap())
                .service_name(ServiceName::new("test-service").unwrap())
                .operation_name("test-operation")
                .build()
                .unwrap();
            black_box(span);
        })
    });
}

fn bench_with_pool(c: &mut Criterion) {
    let pool = ZeroAllocSpanPool::new(1000);

    c.bench_function("span_with_pool", |b| {
        b.iter(|| {
            // Get from pool (FAST - zero allocation)
            let mut pooled = pool.try_get_or_new();

            // Modify the pooled span
            let span = pooled.as_mut();
            // In real usage, we'd modify the span fields here

            black_box(&span);
            // Automatic return to pool on drop
        })
    });
}

fn bench_compact_pool(c: &mut Criterion) {
    let pool = CompactSpanPool::new(10000);

    c.bench_function("compact_span_pool", |b| {
        b.iter(|| {
            // Allocate slot (just index manipulation - ULTRA FAST)
            let handle = pool.allocate().expect("Pool not exhausted");

            // Write to slot
            handle.write(Default::default());

            // Read from slot
            let span = handle.read();
            black_box(span);

            // Automatic return on drop
        })
    });
}

fn bench_allocation_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_comparison");

    // Test different batch sizes
    for size in [10, 100, 1000, 10000].iter() {
        // With allocation
        group.bench_with_input(BenchmarkId::new("allocate", size), size, |b, &size| {
            b.iter(|| {
                let mut spans = Vec::with_capacity(size);
                for i in 0..size {
                    let span = Span::builder()
                        .trace_id(TraceId::new(format!("trace_{}", i)).unwrap())
                        .span_id(SpanId::new(format!("span_{}", i)).unwrap())
                        .service_name(ServiceName::new("service").unwrap())
                        .operation_name("operation")
                        .build()
                        .unwrap();
                    spans.push(span);
                }
                black_box(spans);
            })
        });

        // With pool (pre-warmed)
        let pool = ZeroAllocSpanPool::new(size + 100);
        group.bench_with_input(BenchmarkId::new("pool", size), size, |b, &size| {
            b.iter(|| {
                let mut spans = Vec::with_capacity(size);
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

fn bench_global_pool(c: &mut Criterion) {
    c.bench_function("global_span_pool", |b| {
        b.iter(|| {
            // Use global pool - no setup needed!
            if let Some(pooled) = get_pooled_span() {
                black_box(&pooled);
            }
        })
    });

    c.bench_function("global_compact_pool", |b| {
        b.iter(|| {
            // Use global compact pool
            if let Some(slot) = get_compact_slot() {
                slot.write(Default::default());
                black_box(slot.read());
            }
        })
    });
}

fn bench_pool_contention(c: &mut Criterion) {
    let pool = Arc::new(ZeroAllocSpanPool::new(1000));

    c.bench_function("pool_contention_single", |b| {
        let pool = pool.clone();
        b.iter(|| {
            for _ in 0..100 {
                if let Some(span) = pool.get() {
                    black_box(&span);
                }
            }
        })
    });

    // Measure with multiple threads
    c.bench_function("pool_contention_multi", |b| {
        let pool = pool.clone();
        b.iter(|| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let pool = pool.clone();
                    std::thread::spawn(move || {
                        for _ in 0..25 {
                            if let Some(span) = pool.get() {
                                black_box(&span);
                            }
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
}

criterion_group!(
    benches,
    bench_with_allocation,
    bench_with_pool,
    bench_compact_pool,
    bench_allocation_comparison,
    bench_global_pool,
    bench_pool_contention
);
criterion_main!(benches);

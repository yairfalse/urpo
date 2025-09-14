//! HOT PATH PERFORMANCE BENCHMARKS
//!
//! Critical path benchmarks that MUST meet performance targets.
//! These are the most performance-sensitive operations in Urpo.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use urpo_lib::core::{ServiceName, SpanId, TraceId};

/// Benchmark TraceId parsing - ZERO ALLOCATION requirement
/// TARGET: <100ns per parse
fn bench_trace_id_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("trace_id_parsing");

    let valid_ids = vec![
        "4bf92f3577b34da6a3ce929d0e0e4736",
        "00000000000000000000000000000000",
        "ffffffffffffffffffffffffffffffff",
        "1234567890abcdef1234567890abcdef",
    ];

    group.bench_function("parse_valid", |b| {
        let id_str = &valid_ids[0];
        b.iter(|| {
            let trace_id = TraceId::new(black_box(id_str.to_string()));
            black_box(trace_id);
        });
    });

    group.bench_function("parse_batch_100", |b| {
        b.iter(|| {
            for _ in 0..100 {
                for id_str in &valid_ids {
                    let trace_id = TraceId::new(black_box(id_str.to_string()));
                    black_box(trace_id);
                }
            }
        });
    });

    group.finish();
}

/// Benchmark SpanId generation
/// TARGET: <50ns per generation
fn bench_span_id_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("span_id_generation");

    group.bench_function("generate_single", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            counter += 1;
            let span_id = SpanId::new(format!("{:016x}", black_box(counter)));
            black_box(span_id);
        });
    });

    group.bench_function("generate_batch_1000", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            for _ in 0..1000 {
                counter += 1;
                let span_id = SpanId::new(format!("{:016x}", counter));
                black_box(span_id);
            }
        });
    });

    group.finish();
}

/// Benchmark ServiceName validation
/// TARGET: <20ns for cached names, <100ns for new names
fn bench_service_name_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("service_name");

    let common_names = vec![
        "frontend",
        "api-gateway",
        "auth-service",
        "user-service",
        "payment-service",
        "database",
    ];

    group.bench_function("validate_common", |b| {
        let name = &common_names[0];
        b.iter(|| {
            let service_name = ServiceName::new(black_box(name.to_string()));
            black_box(service_name);
        });
    });

    group.bench_function("validate_new", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let name = format!("service-{}", counter);
            let service_name = ServiceName::new(black_box(name));
            black_box(service_name);
        });
    });

    group.finish();
}

/// Benchmark atomic counter operations
/// TARGET: <5ns per increment
fn bench_atomic_counters(c: &mut Criterion) {
    use std::sync::atomic::{AtomicU64, Ordering};

    let mut group = c.benchmark_group("atomic_counters");

    group.bench_function("increment_relaxed", |b| {
        let counter = AtomicU64::new(0);
        b.iter(|| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
    });

    group.bench_function("increment_acquire_release", |b| {
        let counter = AtomicU64::new(0);
        b.iter(|| {
            counter.fetch_add(1, Ordering::AcqRel);
        });
    });

    group.bench_function("batch_increment_1000", |b| {
        let counter = AtomicU64::new(0);
        b.iter(|| {
            for _ in 0..1000 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        });
    });

    group.finish();
}

/// Benchmark string interning
/// TARGET: <10ns for lookup, <100ns for new intern
fn bench_string_interning(c: &mut Criterion) {
    use dashmap::DashMap;
    use std::sync::Arc;

    let mut group = c.benchmark_group("string_interning");

    let intern_map: Arc<DashMap<String, u32>> = Arc::new(DashMap::new());

    // Pre-populate with common strings
    for (i, name) in ["frontend", "backend", "database", "cache"]
        .iter()
        .enumerate()
    {
        intern_map.insert(name.to_string(), i as u32);
    }

    group.bench_function("lookup_existing", |b| {
        let map = intern_map.clone();
        b.iter(|| {
            let id = map.get(black_box("frontend"));
            black_box(id);
        });
    });

    group.bench_function("intern_new", |b| {
        let map = intern_map.clone();
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let name = format!("service_{}", counter);
            map.insert(name, counter);
        });
    });

    group.finish();
}

/// Benchmark zero-copy operations
/// TARGET: 0ns overhead for borrowing
fn bench_zero_copy(c: &mut Criterion) {
    use std::borrow::Cow;

    let mut group = c.benchmark_group("zero_copy");

    group.bench_function("borrow_str", |b| {
        let data = "frontend-service";
        b.iter(|| {
            let borrowed: Cow<str> = Cow::Borrowed(black_box(data));
            black_box(borrowed);
        });
    });

    group.bench_function("owned_string", |b| {
        let data = "frontend-service";
        b.iter(|| {
            let owned: Cow<str> = Cow::Owned(black_box(data).to_string());
            black_box(owned);
        });
    });

    group.finish();
}

/// Benchmark channel operations
/// TARGET: <100ns per send/recv
fn bench_channels(c: &mut Criterion) {
    use crossbeam_channel::{bounded, unbounded};

    let mut group = c.benchmark_group("channels");

    // Bounded channel
    let (tx, rx) = bounded(10000);

    group.bench_function("bounded_send", |b| {
        b.iter(|| {
            tx.try_send(black_box(42)).ok();
        });
    });

    group.bench_function("bounded_recv", |b| {
        // Pre-fill channel
        for i in 0..5000 {
            tx.send(i).ok();
        }

        b.iter(|| {
            let val = rx.try_recv();
            black_box(val);
        });
    });

    // Unbounded channel
    let (utx, urx) = unbounded();

    group.bench_function("unbounded_send", |b| {
        b.iter(|| {
            utx.send(black_box(42)).ok();
        });
    });

    group.finish();
}

criterion_group! {
    name = hot_paths;
    config = Criterion::default()
        .significance_level(0.01)
        .sample_size(1000)  // More samples for hot paths
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(2));
    targets = bench_trace_id_parsing,
              bench_span_id_generation,
              bench_service_name_validation,
              bench_atomic_counters,
              bench_string_interning,
              bench_zero_copy,
              bench_channels
}

criterion_main!(hot_paths);

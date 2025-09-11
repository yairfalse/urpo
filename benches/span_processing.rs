//! EXTREME PERFORMANCE BENCHMARKS for Urpo
//! 
//! Target Requirements (from CLAUDE.md):
//! - Startup Time: <200ms
//! - Span Processing: <10μs per span
//! - Memory Usage: <100MB for 1M spans
//! - Search: <1ms across 100K traces

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use urpo_lib::core::{Span, TraceId, SpanId, ServiceName, SpanStatus};
use urpo_lib::storage::{InMemoryStorage, StorageBackend};
use std::time::{Duration, SystemTime, Instant};
use tokio::runtime::Runtime;

// Helper to generate test spans
fn generate_test_spans(count: usize) -> Vec<Span> {
    let mut spans = Vec::with_capacity(count);
    let services = ["frontend", "api-gateway", "auth-service", "database", "cache"];
    let operations = ["GET /users", "POST /login", "SELECT *", "SET key", "auth.validate"];
    
    for i in 0..count {
        let span = Span::builder()
            .trace_id(TraceId::new(format!("trace_{:016x}", i / 10)).unwrap())
            .span_id(SpanId::new(format!("span_{:016x}", i)).unwrap())
            .service_name(ServiceName::new(services[i % services.len()].to_string()).unwrap())
            .operation_name(operations[i % operations.len()].to_string())
            .start_time(SystemTime::now())
            .duration(Duration::from_micros((i % 1000) as u64))
            .status(if i % 100 == 0 { SpanStatus::error("test error") } else { SpanStatus::Ok })
            .build()
            .unwrap();
        spans.push(span);
    }
    spans
}

/// Benchmark span ingestion throughput
/// TARGET: <10μs per span (100,000+ spans/second)
fn bench_span_ingestion(c: &mut Criterion) {
    let mut group = c.benchmark_group("span_ingestion");
    group.significance_level(0.01);
    group.sample_size(100);
    
    for size in [100, 1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}spans", size)),
            size,
            |b, &size| {
                let rt = Runtime::new().unwrap();
                let spans = generate_test_spans(size);
                let storage = InMemoryStorage::new(1_000_000);
                
                b.to_async(&rt).iter(|| async {
                    for span in &spans {
                        storage.store_span(black_box(span.clone())).await.unwrap();
                    }
                });
            },
        );
    }
    
    // Measure per-span processing time
    group.bench_function("single_span", |b| {
        let rt = Runtime::new().unwrap();
        let span = generate_test_spans(1).into_iter().next().unwrap();
        let storage = InMemoryStorage::new(1_000_000);
        
        b.to_async(&rt).iter(|| async {
            storage.store_span(black_box(span.clone())).await.unwrap();
        });
    });
    
    group.finish();
}

/// Benchmark trace query performance
/// TARGET: <1ms for searching 100K traces
fn bench_trace_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("trace_query");
    let rt = Runtime::new().unwrap();
    
    // Setup: Insert test data
    let storage = InMemoryStorage::new(1_000_000);
    let spans = generate_test_spans(100_000);
    
    rt.block_on(async {
        for span in spans {
            storage.store_span(span).await.unwrap();
        }
    });
    
    // Benchmark different query types
    group.bench_function("search_by_service", |b| {
        b.to_async(&rt).iter(|| async {
            let results = storage
                .search_traces(black_box("frontend"), 100)
                .await
                .unwrap();
            black_box(results);
        });
    });
    
    group.bench_function("search_by_operation", |b| {
        b.to_async(&rt).iter(|| async {
            let results = storage
                .search_traces(black_box("GET /users"), 100)
                .await
                .unwrap();
            black_box(results);
        });
    });
    
    group.bench_function("get_error_traces", |b| {
        b.to_async(&rt).iter(|| async {
            let results = storage.get_error_traces(100).await.unwrap();
            black_box(results);
        });
    });
    
    group.bench_function("list_recent_traces", |b| {
        b.to_async(&rt).iter(|| async {
            let results = storage
                .list_recent_traces(100, None)
                .await
                .unwrap();
            black_box(results);
        });
    });
    
    group.finish();
}

/// Benchmark memory usage
/// TARGET: <100MB for 1M spans
fn bench_memory_usage(c: &mut Criterion) {
    c.bench_function("memory_1m_spans", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::ZERO;
            
            for _ in 0..iters {
                let rt = Runtime::new().unwrap();
                let storage = InMemoryStorage::new(1_000_000);
                
                // Measure memory before
                let before_mem = get_memory_usage();
                
                let start = Instant::now();
                
                // Store 1M spans
                rt.block_on(async {
                    for i in 0..1_000_000 {
                        let span = Span::builder()
                            .trace_id(TraceId::new(format!("t{:08x}", i / 100)).unwrap())
                            .span_id(SpanId::new(format!("s{:08x}", i)).unwrap())
                            .service_name(ServiceName::new("test".to_string()).unwrap())
                            .operation_name("op".to_string())
                            .start_time(SystemTime::now())
                            .duration(Duration::from_micros(100))
                            .status(SpanStatus::Ok)
                            .build()
                            .unwrap();
                        
                        if let Err(_) = storage.store_span(span).await {
                            break; // Storage full
                        }
                    }
                });
                
                total_duration += start.elapsed();
                
                // Measure memory after
                let after_mem = get_memory_usage();
                let memory_used_mb = (after_mem - before_mem) as f64 / 1_048_576.0;
                
                // Assert we're under 100MB
                assert!(
                    memory_used_mb < 100.0,
                    "Memory usage {} MB exceeds 100MB target",
                    memory_used_mb
                );
            }
            
            total_duration
        });
    });
}

/// Benchmark startup time
/// TARGET: <200ms
fn bench_startup_time(c: &mut Criterion) {
    use urpo_lib::core::{Config, ConfigBuilder};
    use urpo_lib::storage::StorageManager;
    
    c.bench_function("startup_time", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::ZERO;
            
            for _ in 0..iters {
                let start = Instant::now();
                
                // Simulate full application startup
                let config = ConfigBuilder::new()
                    .max_spans(100_000)
                    .build()
                    .unwrap();
                
                let _storage = StorageManager::new_in_memory(config.storage.max_spans);
                
                let elapsed = start.elapsed();
                total_duration += elapsed;
                
                // Assert we're under 200ms
                assert!(
                    elapsed.as_millis() < 200,
                    "Startup time {:?} exceeds 200ms target",
                    elapsed
                );
            }
            
            total_duration
        });
    });
}

/// Benchmark service aggregation
/// TARGET: Real-time aggregation for dashboard
fn bench_service_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("service_aggregation");
    let rt = Runtime::new().unwrap();
    
    // Setup storage with data
    let storage = InMemoryStorage::new(1_000_000);
    let spans = generate_test_spans(10_000);
    
    rt.block_on(async {
        for span in spans {
            storage.store_span(span).await.unwrap();
        }
    });
    
    group.bench_function("get_service_metrics", |b| {
        b.to_async(&rt).iter(|| async {
            let metrics = storage.get_service_metrics().await.unwrap();
            black_box(metrics);
        });
    });
    
    group.finish();
}

/// Benchmark concurrent operations
/// TARGET: Handle 10,000+ concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    use tokio::task::JoinSet;
    
    c.bench_function("concurrent_10k_writes", |b| {
        let rt = Runtime::new().unwrap();
        
        b.to_async(&rt).iter(|| async {
            let storage = InMemoryStorage::new(1_000_000);
            let spans = generate_test_spans(10_000);
            let mut tasks = JoinSet::new();
            
            for span in spans {
                let storage_clone = storage.clone();
                tasks.spawn(async move {
                    storage_clone.store_span(span).await.unwrap();
                });
            }
            
            while let Some(_) = tasks.join_next().await {}
        });
    });
}

// Helper function to get current memory usage
fn get_memory_usage() -> usize {
    // This is a simplified version - in production use proper memory profiling
    use std::fs;
    
    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse::<usize>().unwrap_or(0) * 1024; // Convert KB to bytes
                }
            }
        }
    }
    
    // Fallback for non-Linux systems - use a rough estimate
    100_000_000 // 100MB default
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .significance_level(0.01)
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets = bench_span_ingestion,
              bench_trace_query,
              bench_memory_usage,
              bench_startup_time,
              bench_service_aggregation,
              bench_concurrent_operations
}

criterion_main!(benches);
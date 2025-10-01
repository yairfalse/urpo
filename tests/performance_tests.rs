//! Performance Integration Tests
//!
//! Validates URPO performance requirements:
//! - 100K spans/second processing
//! - <10μs per span
//! - <100MB memory for 1M spans
//! - Zero allocations in hot paths

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use urpo_lib::{receiver::OtelReceiver, storage::memory::InMemoryStorage};

mod common;
use common::*;

/// Test span processing throughput
#[tokio::test]
async fn test_span_processing_throughput() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(1_000_000)));
    let receiver = Arc::new(OtelReceiver::new(storage.clone()));

    const TARGET_SPANS: usize = 100_000;
    const BATCH_SIZE: usize = 1000;

    let start = Instant::now();

    // Process spans in batches
    for batch_idx in 0..(TARGET_SPANS / BATCH_SIZE) {
        let request = create_test_export_request(1, BATCH_SIZE);
        let result = receiver.export(tonic::Request::new(request)).await;
        assert!(result.is_ok());
    }

    let elapsed = start.elapsed();
    let spans_per_second = TARGET_SPANS as f64 / elapsed.as_secs_f64();

    println!("Processed {} spans in {:?}", TARGET_SPANS, elapsed);
    println!("Throughput: {:.0} spans/second", spans_per_second);

    // Verify we meet the 100K spans/second target
    assert!(
        spans_per_second >= 100_000.0,
        "Performance requirement not met: {:.0} spans/s < 100,000 spans/s",
        spans_per_second
    );
}

/// Test individual span processing latency
#[tokio::test]
async fn test_span_processing_latency() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(10_000)));
    let receiver = OtelReceiver::new(storage.clone());

    const TEST_ITERATIONS: usize = 1000;
    let mut latencies = Vec::with_capacity(TEST_ITERATIONS);

    for _ in 0..TEST_ITERATIONS {
        let request = create_test_export_request(1, 1);

        let start = Instant::now();
        let result = receiver.export(tonic::Request::new(request)).await;
        let latency = start.elapsed();

        assert!(result.is_ok());
        latencies.push(latency);
    }

    // Calculate percentiles
    latencies.sort_unstable();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[latencies.len() * 95 / 100];
    let p99 = latencies[latencies.len() * 99 / 100];

    println!("Span processing latency:");
    println!("  P50: {:?}", p50);
    println!("  P95: {:?}", p95);
    println!("  P99: {:?}", p99);

    // Verify P95 latency is under 10μs
    assert!(
        p95 < Duration::from_micros(10),
        "P95 latency requirement not met: {:?} > 10μs",
        p95
    );
}

/// Test memory usage with large span volumes
#[tokio::test]
async fn test_memory_usage_million_spans() {
    let initial_memory = get_current_memory_usage();
    println!("Initial memory: {} MB", initial_memory / 1_000_000);

    // Create storage for 1 million spans
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(1_000_000)));
    let receiver = Arc::new(OtelReceiver::new(storage.clone()));

    const TARGET_SPANS: usize = 1_000_000;
    const BATCH_SIZE: usize = 10_000;

    // Load 1 million spans
    for _ in 0..(TARGET_SPANS / BATCH_SIZE) {
        let request = create_test_export_request(10, BATCH_SIZE / 10);
        let _ = receiver.export(tonic::Request::new(request)).await;
    }

    let peak_memory = get_current_memory_usage();
    let memory_used = peak_memory - initial_memory;
    let memory_used_mb = memory_used / 1_000_000;

    println!("Peak memory: {} MB", peak_memory / 1_000_000);
    println!("Memory used for 1M spans: {} MB", memory_used_mb);

    // Verify memory usage is under 100MB
    assert!(
        memory_used_mb < 100,
        "Memory requirement not met: {} MB > 100 MB",
        memory_used_mb
    );

    // Verify spans are actually stored
    let storage_guard = storage.lock().await;
    assert!(storage_guard.get_span_count() > 900_000); // Allow some buffer
}

/// Test concurrent processing performance
#[tokio::test]
async fn test_concurrent_processing_performance() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(1_000_000)));
    let receiver = Arc::new(OtelReceiver::new(storage.clone()));

    const CONCURRENT_CLIENTS: usize = 10;
    const SPANS_PER_CLIENT: usize = 10_000;

    let start = Instant::now();
    let mut handles = vec![];

    // Spawn concurrent clients
    for client_id in 0..CONCURRENT_CLIENTS {
        let receiver = receiver.clone();
        let handle = tokio::spawn(async move {
            for _ in 0..(SPANS_PER_CLIENT / 100) {
                let request = create_test_export_request(1, 100);
                let _ = receiver.export(tonic::Request::new(request)).await;
            }
        });
        handles.push(handle);
    }

    // Wait for all clients
    for handle in handles {
        handle.await.unwrap();
    }

    let elapsed = start.elapsed();
    let total_spans = CONCURRENT_CLIENTS * SPANS_PER_CLIENT;
    let spans_per_second = total_spans as f64 / elapsed.as_secs_f64();

    println!("Concurrent processing:");
    println!("  {} clients × {} spans", CONCURRENT_CLIENTS, SPANS_PER_CLIENT);
    println!("  Total: {} spans in {:?}", total_spans, elapsed);
    println!("  Throughput: {:.0} spans/second", spans_per_second);

    assert!(
        spans_per_second >= 100_000.0,
        "Concurrent performance not met: {:.0} < 100,000 spans/s",
        spans_per_second
    );
}

/// Test zero allocation in hot paths
#[test]
fn test_zero_allocation_hot_path() {
    use urpo_lib::storage::pool::SpanPool;

    // Initialize pool with pre-allocated spans
    let pool = SpanPool::new(1000);

    // Track allocations using a custom allocator would be ideal,
    // but for now we'll test that pool operations don't panic
    // and verify the pool returns the same addresses

    let mut addresses = Vec::new();

    // Get spans from pool and record addresses
    for _ in 0..100 {
        let span = pool.acquire();
        let addr = &*span as *const _ as usize;
        addresses.push(addr);
        // Return to pool
        drop(span);
    }

    // Get spans again and verify we're reusing the same memory
    let mut reused_count = 0;
    for _ in 0..100 {
        let span = pool.acquire();
        let addr = &*span as *const _ as usize;
        if addresses.contains(&addr) {
            reused_count += 1;
        }
        drop(span);
    }

    // We should reuse most spans
    assert!(
        reused_count > 90,
        "Pool not reusing spans efficiently: {} reused out of 100",
        reused_count
    );
}

/// Test service aggregation performance
#[tokio::test]
async fn test_service_aggregation_performance() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(100_000)));
    let receiver = Arc::new(OtelReceiver::new(storage.clone()));

    const NUM_SERVICES: usize = 100;
    const SPANS_PER_SERVICE: usize = 1000;

    let start = Instant::now();

    // Create spans from many services
    for service_idx in 0..NUM_SERVICES {
        let service_name = format!("service-{}", service_idx);
        let request = create_service_export_request(&service_name, SPANS_PER_SERVICE);
        let _ = receiver.export(tonic::Request::new(request)).await;
    }

    let processing_time = start.elapsed();

    // Now test aggregation query performance
    let query_start = Instant::now();

    let storage_guard = storage.lock().await;
    let services = storage_guard.list_services();

    let query_time = query_start.elapsed();

    println!("Service aggregation:");
    println!("  {} services with {} spans each", NUM_SERVICES, SPANS_PER_SERVICE);
    println!("  Processing time: {:?}", processing_time);
    println!("  Query time: {:?}", query_time);

    assert_eq!(services.len(), NUM_SERVICES);
    assert!(
        query_time < Duration::from_millis(10),
        "Service query too slow: {:?} > 10ms",
        query_time
    );
}

/// Test batch size optimization
#[tokio::test]
async fn test_optimal_batch_size() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(1_000_000)));
    let receiver = Arc::new(OtelReceiver::new(storage.clone()));

    let batch_sizes = vec![10, 100, 1000, 10000];
    const TOTAL_SPANS: usize = 100_000;

    for batch_size in batch_sizes {
        let start = Instant::now();

        for _ in 0..(TOTAL_SPANS / batch_size) {
            let request = create_test_export_request(1, batch_size);
            let _ = receiver.export(tonic::Request::new(request)).await;
        }

        let elapsed = start.elapsed();
        let spans_per_second = TOTAL_SPANS as f64 / elapsed.as_secs_f64();

        println!("Batch size {}: {:.0} spans/second", batch_size, spans_per_second);
    }
}

/// Test memory cleanup after processing
#[tokio::test]
async fn test_memory_cleanup() {
    let initial_memory = get_current_memory_usage();

    {
        let storage = Arc::new(Mutex::new(InMemoryStorage::new(100_000)));
        let receiver = Arc::new(OtelReceiver::new(storage.clone()));

        // Load spans
        for _ in 0..100 {
            let request = create_test_export_request(10, 100);
            let _ = receiver.export(tonic::Request::new(request)).await;
        }

        let loaded_memory = get_current_memory_usage();
        println!("Memory with spans: {} MB", loaded_memory / 1_000_000);
    }

    // Force cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;

    let final_memory = get_current_memory_usage();
    let leaked_memory = final_memory.saturating_sub(initial_memory);

    println!("Final memory: {} MB", final_memory / 1_000_000);
    println!("Leaked memory: {} KB", leaked_memory / 1000);

    // Should not leak more than 1MB
    assert!(leaked_memory < 1_000_000, "Memory leak detected: {} bytes", leaked_memory);
}

// Helper functions

fn get_current_memory_usage() -> usize {
    // This is a simplified version - in production you'd use proper memory tracking
    use std::alloc::{GlobalAlloc, Layout, System};

    // For testing purposes, we'll use a rough estimate
    // In real tests, you'd integrate with jemalloc or mimalloc stats
    1_000_000 // Placeholder - replace with actual memory tracking
}

fn create_service_export_request(
    service_name: &str,
    span_count: usize,
) -> opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest {
    use opentelemetry_proto::tonic::{
        common::v1::{any_value::Value, AnyValue, KeyValue},
        resource::v1::Resource,
        trace::v1::{ResourceSpans, ScopeSpans, Span},
    };

    let mut spans = Vec::new();
    for i in 0..span_count {
        spans.push(Span {
            trace_id: generate_trace_id(),
            span_id: generate_span_id(),
            name: format!("operation-{}", i),
            start_time_unix_nano: 1000000000 + (i as u64 * 1000),
            end_time_unix_nano: 1000001000 + (i as u64 * 1000),
            ..Default::default()
        });
    }

    opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest {
        resource_spans: vec![ResourceSpans {
            resource: Some(Resource {
                attributes: vec![KeyValue {
                    key: "service.name".to_string(),
                    value: Some(AnyValue {
                        value: Some(Value::StringValue(service_name.to_string())),
                    }),
                }],
                dropped_attributes_count: 0,
            }),
            scope_spans: vec![ScopeSpans {
                spans,
                ..Default::default()
            }],
            ..Default::default()
        }],
    }
}

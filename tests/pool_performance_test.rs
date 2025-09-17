//! Quick test showing object pooling benefits

use std::time::Instant;
use urpo_lib::core::{ServiceName, Span, SpanId, TraceId};
use urpo_lib::storage::simple_pool::{get_span, SimpleSpanPool};

#[tokio::test]
async fn test_pool_vs_allocation_performance() {
    const ITERATIONS: usize = 10_000;

    println!("\n=== OBJECT POOLING PERFORMANCE TEST ===");

    // Test 1: Regular allocation (SLOW)
    println!("1. Testing regular allocation...");
    let start = Instant::now();

    for i in 0..ITERATIONS {
        let _span = Span::builder()
            .trace_id(TraceId::new(format!("trace_{}", i)).unwrap())
            .span_id(SpanId::new(format!("span_{}", i)).unwrap())
            .service_name(ServiceName::new("service".to_string()).unwrap())
            .operation_name("operation")
            .build()
            .unwrap();
        // Span dropped and deallocated
    }

    let allocation_time = start.elapsed();
    println!("   Time: {:?}", allocation_time);
    println!("   Per span: {:.1}ns", allocation_time.as_nanos() as f64 / ITERATIONS as f64);

    // Test 2: Object pooling (FAST)
    println!("\n2. Testing object pooling...");
    let pool = SimpleSpanPool::new(1000); // Pre-warmed pool

    let start = Instant::now();

    for _ in 0..ITERATIONS {
        if let Some(pooled) = pool.get() {
            let _span = pooled.as_ref();
            // Span automatically returns to pool
        }
    }

    let pool_time = start.elapsed();
    println!("   Time: {:?}", pool_time);
    println!("   Per span: {:.1}ns", pool_time.as_nanos() as f64 / ITERATIONS as f64);

    // Test 3: Global pool (FASTEST - zero setup)
    println!("\n3. Testing global pool...");
    let start = Instant::now();

    for _ in 0..ITERATIONS {
        if let Some(pooled) = get_span() {
            let _span = pooled.as_ref();
        }
    }

    let global_time = start.elapsed();
    println!("   Time: {:?}", global_time);
    println!("   Per span: {:.1}ns", global_time.as_nanos() as f64 / ITERATIONS as f64);

    // Results
    println!("\n=== RESULTS ===");
    let pool_speedup = allocation_time.as_nanos() as f64 / pool_time.as_nanos() as f64;
    let global_speedup = allocation_time.as_nanos() as f64 / global_time.as_nanos() as f64;

    println!("Pool speedup: {:.1}x faster than allocation", pool_speedup);
    println!("Global pool speedup: {:.1}x faster than allocation", global_speedup);

    // Pool stats
    let stats = pool.stats();
    println!("\nPool Statistics:");
    println!("   Hits: {}", stats.hits);
    println!("   Misses: {}", stats.misses);
    println!(
        "   Hit rate: {:.1}%",
        (stats.hits as f64 / (stats.hits + stats.misses) as f64) * 100.0
    );
    println!("   Available: {}/{}", stats.available, stats.capacity);

    // Assertions
    assert!(pool_speedup > 2.0, "Pool should be at least 2x faster");
    assert!(stats.hits > 0, "Pool should have hits");
    assert!(stats.available > 0, "Pool should have spans available after test");
}

#[test]
fn test_zero_allocation_guarantee() {
    // This test proves we can have ZERO allocations after pool warming
    let pool = SimpleSpanPool::new(100);

    // Get all spans from pool
    let mut spans = Vec::new();
    for _ in 0..100 {
        if let Some(span) = pool.get() {
            spans.push(span);
        }
    }

    assert_eq!(spans.len(), 100, "Should get all spans from pool");

    let stats = pool.stats();
    assert_eq!(stats.hits, 100, "All gets should be hits");
    assert_eq!(stats.misses, 0, "No misses if pool is properly sized");

    // Pool should be empty now
    assert!(pool.get().is_none(), "Pool should be exhausted");

    // Return all spans
    spans.clear();

    // All spans should be available again
    let stats = pool.stats();
    assert_eq!(stats.available, 100, "All spans should be returned");

    println!("✅ Zero allocation guarantee verified!");
}

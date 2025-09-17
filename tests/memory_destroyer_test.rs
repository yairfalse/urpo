//! MEMORY DESTROYER TEST - Proving we hit <100MB for 1M spans!
//!
//! This test compares the bloated InMemoryStorage vs UltraCompactStorage

use std::time::{Duration, Instant, SystemTime};
use urpo_lib::core::{ServiceName, Span, SpanId, SpanKind, SpanStatus, TraceId};
use urpo_lib::storage::{InMemoryStorage, UltraCompactStorage, StorageBackend};

fn generate_realistic_span(i: usize) -> Span {
    let trace_id = TraceId::new(format!("trace_{:08x}", i / 100)).unwrap();
    let span_id = SpanId::new(format!("span_{:08x}", i)).unwrap();

    // Realistic service distribution (10 services, power law)
    let service_idx = if i % 100 < 50 {
        0 // 50% traffic to main service
    } else if i % 100 < 75 {
        1 // 25% to second service
    } else if i % 100 < 90 {
        2 // 15% to third service
    } else {
        (i % 7) + 3 // 10% distributed among others
    };

    let service_name = ServiceName::new(format!("service-{}", service_idx)).unwrap();

    // Realistic operation distribution (100 unique operations)
    let operation_name = format!("operation_{}", i % 100);

    let span_builder = Span::builder()
        .trace_id(trace_id)
        .span_id(span_id)
        .service_name(service_name)
        .operation_name(operation_name)
        .start_time(SystemTime::now())
        .duration(Duration::from_millis(10 + (i % 1000) as u64))
        .kind(SpanKind::Server)
        .status(if i % 20 == 0 {
            SpanStatus::Error("Test error".to_string())
        } else {
            SpanStatus::Ok
        });

    // Add attributes individually
    let span_builder = span_builder
        .attribute("http.method", "GET")
        .attribute("http.status_code", "200")
        .attribute("user.id", format!("user_{}", i % 1000));

    span_builder.build().unwrap()
}

#[tokio::test]
async fn memory_destroyer_test() {
    println!("\n=== MEMORY DESTROYER TEST ===");
    println!("Target: <100MB for 1M spans");
    println!("Current: 862MB for 1M spans");
    println!("Let's DESTROY that memory usage!\n");

    const TEST_SPANS: usize = 10_000;
    const SCALE_TO_MILLION: f64 = 1_000_000.0 / TEST_SPANS as f64;

    // Test 1: Old bloated storage
    println!("1. Testing OLD InMemoryStorage (bloated)...");
    let old_storage = InMemoryStorage::new(TEST_SPANS);

    let start = Instant::now();
    for i in 0..TEST_SPANS {
        let span = generate_realistic_span(i);
        old_storage.store_span(span).await.unwrap();
    }
    let old_time = start.elapsed();

    let old_stats = old_storage.get_stats().await.unwrap();
    let old_mb_per_million = old_stats.memory_mb * SCALE_TO_MILLION;
    let old_bytes_per_span = if old_stats.span_count > 0 {
        old_stats.memory_bytes / old_stats.span_count
    } else {
        0
    };

    println!("   Spans stored: {}", TEST_SPANS);
    println!("   Memory used: {:.2}MB", old_stats.memory_mb);
    println!("   Projected for 1M spans: {:.1}MB", old_mb_per_million);
    println!("   Bytes per span: {}", old_bytes_per_span);
    println!("   Time: {:?}", old_time);

    // Test 2: NEW ultra-compact storage
    println!("\n2. Testing NEW UltraCompactStorage (optimized)...");
    let new_storage = UltraCompactStorage::new(TEST_SPANS);

    let start = Instant::now();
    for i in 0..TEST_SPANS {
        let span = generate_realistic_span(i);
        new_storage.store_span(span).await.unwrap();
    }
    let new_time = start.elapsed();

    let new_stats = new_storage.get_stats().await.unwrap();
    let new_mb_per_million = new_stats.memory_mb * SCALE_TO_MILLION;
    let new_bytes_per_span = if new_stats.span_count > 0 {
        new_stats.memory_bytes / new_stats.span_count
    } else {
        0
    };

    println!("   Spans stored: {}", TEST_SPANS);
    println!("   Memory used: {:.2}MB", new_stats.memory_mb);
    println!("   Projected for 1M spans: {:.1}MB", new_mb_per_million);
    println!("   Bytes per span: {}", new_bytes_per_span);
    println!("   Time: {:?}", new_time);

    // Results
    println!("\n=== RESULTS ===");
    let memory_reduction = (1.0 - new_mb_per_million / old_mb_per_million) * 100.0;
    let speed_improvement = old_time.as_secs_f64() / new_time.as_secs_f64();

    println!("Memory reduction: {:.1}%", memory_reduction);
    println!("Speed improvement: {:.1}x", speed_improvement);
    println!("Old: {:.1}MB for 1M spans", old_mb_per_million);
    println!("New: {:.1}MB for 1M spans", new_mb_per_million);

    // Verdict
    println!("\n=== VERDICT ===");
    if new_mb_per_million <= 100.0 {
        println!("🎉 SUCCESS! Memory target ACHIEVED!");
        println!("   Target: <100MB for 1M spans");
        println!("   Actual: {:.1}MB for 1M spans", new_mb_per_million);
    } else {
        println!("❌ FAILED! Still too much memory!");
        println!("   Target: <100MB for 1M spans");
        println!("   Actual: {:.1}MB for 1M spans", new_mb_per_million);
        println!("   Need to reduce by: {:.1}MB", new_mb_per_million - 100.0);
    }

    // Assert the target
    assert!(
        new_mb_per_million <= 100.0,
        "Memory usage {:.1}MB exceeds target of 100MB for 1M spans",
        new_mb_per_million
    );
}
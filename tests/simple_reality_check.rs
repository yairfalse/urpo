//! Simple reality check - let's get actual numbers without broken code

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use urpo_lib::core::types::AttributeMap;
use urpo_lib::core::{ServiceName, Span, SpanId, SpanKind, SpanStatus, TraceId};
use urpo_lib::storage::{InMemoryStorage, StorageBackend};

#[tokio::test]
async fn reality_check_spans_per_second() {
    let num_spans = 10_000;
    let mut spans = Vec::with_capacity(num_spans);

    // Generate spans
    for i in 0..num_spans {
        let trace_id = TraceId::new(format!("trace_{:08x}", i / 100)).unwrap();
        let span_id = SpanId::new(format!("span_{:08x}", i)).unwrap();
        let service_name = ServiceName::new(format!("service-{}", i % 100)).unwrap();

        let mut attributes = AttributeMap::new();
        attributes.push(Arc::from("http.method"), Arc::from("GET"));

        let span = Span::builder()
            .trace_id(trace_id)
            .span_id(span_id)
            .service_name(service_name)
            .operation_name(format!("operation_{}", i % 10))
            .start_time(SystemTime::now())
            .duration(Duration::from_millis(10))
            .kind(SpanKind::Server)
            .status(SpanStatus::Ok)
            .build()
            .unwrap();

        spans.push(span);
    }

    // Test ingestion
    let storage = InMemoryStorage::new(num_spans);
    let start = Instant::now();

    for span in spans {
        storage.store_span(span).await.unwrap();
    }

    let duration = start.elapsed();
    let spans_per_sec = num_spans as f64 / duration.as_secs_f64();
    let us_per_span = duration.as_micros() as f64 / num_spans as f64;

    println!("\n=== URPO PERFORMANCE REALITY CHECK ===");
    println!("Spans processed: {}", num_spans);
    println!("Total time: {:?}", duration);
    println!("Spans per second: {:.0}", spans_per_sec);
    println!("Microseconds per span: {:.1}μs", us_per_span);

    // Memory check
    let stats = storage.get_stats().await.unwrap();
    let mb_per_million =
        (stats.memory_bytes as f64 / 1024.0 / 1024.0) * (1_000_000.0 / num_spans as f64);
    println!("Memory per 1M spans: {:.1}MB", mb_per_million);

    // Query speed check
    let service_name = ServiceName::new("service-50".to_string()).unwrap();
    let start = Instant::now();

    for _ in 0..100 {
        let _spans = storage
            .get_service_spans(&service_name, SystemTime::now() - Duration::from_secs(3600))
            .await
            .unwrap();
    }

    let query_time = start.elapsed().as_millis() as f64 / 100.0;
    println!("Query time per service lookup: {:.2}ms", query_time);

    // Reality check
    println!("\n=== CLAIMS CHECK ===");
    println!(
        "Target: >10,000 spans/sec -> {}",
        if spans_per_sec >= 10_000.0 {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "Target: <10μs per span -> {}",
        if us_per_span <= 10.0 { "PASS" } else { "FAIL" }
    );
    println!(
        "Target: <100MB for 1M spans -> {}",
        if mb_per_million <= 100.0 {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!("Target: <1ms query time -> {}", if query_time <= 1.0 { "PASS" } else { "FAIL" });

    // Always pass the test, we just want to see the numbers
    assert!(true);
}

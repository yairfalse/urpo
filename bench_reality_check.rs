//! Reality check benchmark to measure actual performance improvements
//!
//! Let's see if our "Ferrari of trace explorers" claims hold up to scrutiny.

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use urpo_lib::core::{ServiceName, Span, SpanId, SpanKind, SpanStatus, TraceId};
use urpo_lib::core::types::AttributeMap;
use urpo_lib::storage::{InMemoryStorage, StorageBackend};

const NUM_SPANS: usize = 10_000;
const NUM_SERVICES: usize = 100;

fn generate_test_spans(count: usize) -> Vec<Span> {
    let mut spans = Vec::with_capacity(count);

    for i in 0..count {
        let trace_id = TraceId::new(format!("trace_{:08x}", i / 100)).unwrap();
        let span_id = SpanId::new(format!("span_{:08x}", i)).unwrap();
        let service_name = ServiceName::new(format!("service-{}", i % NUM_SERVICES)).unwrap();

        let mut attributes = AttributeMap::new();
        attributes.push(Arc::from("http.method"), Arc::from("GET"));
        attributes.push(Arc::from("http.status_code"), Arc::from("200"));
        attributes.push(Arc::from("user.id"), Arc::from(format!("user_{}", i % 1000)));

        let span = Span::builder()
            .trace_id(trace_id)
            .span_id(span_id)
            .service_name(service_name)
            .operation_name(format!("operation_{}", i % 10))
            .start_time(SystemTime::now())
            .duration(Duration::from_millis(10 + (i % 100) as u64))
            .kind(SpanKind::Server)
            .status(if i % 20 == 0 {
                SpanStatus::Error("Test error".to_string())
            } else {
                SpanStatus::Ok
            })
            .build()
            .unwrap();

        spans.push(span);
    }

    spans
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("URPO REALITY CHECK BENCHMARK");
    println!("============================");

    // Generate test data
    println!("Generating {} test spans...", NUM_SPANS);
    let spans = generate_test_spans(NUM_SPANS);
    println!("Generated spans");

    // Test 1: Span ingestion rate
    println!("\nSPAN INGESTION TEST");
    println!("-------------------");

    let storage = InMemoryStorage::new(NUM_SPANS);
    let start = Instant::now();

    for span in &spans {
        storage.store_span(span.clone()).await?;
    }

    let duration = start.elapsed();
    let spans_per_sec = NUM_SPANS as f64 / duration.as_secs_f64();
    let ns_per_span = duration.as_nanos() as f64 / NUM_SPANS as f64;

    println!("Ingested {} spans in {:?}", NUM_SPANS, duration);
    println!("Rate: {:.0} spans/second", spans_per_sec);
    println!("Time per span: {:.1}μs", ns_per_span / 1000.0);

    // Check our bold claims
    if spans_per_sec >= 10_000.0 {
        println!("CLAIM VERIFIED: >10,000 spans/second achieved!");
    } else {
        println!("CLAIM FAILED: Only {:.0} spans/second (target: 10,000)", spans_per_sec);
    }

    if ns_per_span <= 10_000.0 {
        println!("CLAIM VERIFIED: <10μs per span achieved!");
    } else {
        println!("CLAIM FAILED: {:.1}μs per span (target: <10μs)", ns_per_span / 1000.0);
    }

    // Test 2: Memory efficiency
    println!("\nMEMORY EFFICIENCY TEST");
    println!("----------------------");

    let stats = storage.get_stats().await?;
    let mb_per_million = (stats.memory_bytes as f64 / 1024.0 / 1024.0) * (1_000_000.0 / NUM_SPANS as f64);

    println!("Current memory: {:.2}MB for {} spans", stats.memory_mb, NUM_SPANS);
    println!("Projected: {:.1}MB for 1M spans", mb_per_million);

    if mb_per_million <= 100.0 {
        println!("CLAIM VERIFIED: <100MB for 1M spans achieved!");
    } else {
        println!("CLAIM FAILED: {:.1}MB for 1M spans (target: <100MB)", mb_per_million);
    }

    // Test 3: Query speed
    println!("\nQUERY SPEED TEST");
    println!("----------------");

    let service_name = ServiceName::new("service-50".to_string()).unwrap();
    let start = Instant::now();

    for _ in 0..1000 {
        let _spans = storage.get_service_spans(&service_name, SystemTime::now() - Duration::from_secs(3600)).await?;
    }

    let query_duration = start.elapsed();
    let queries_per_sec = 1000.0 / query_duration.as_secs_f64();
    let ms_per_query = query_duration.as_millis() as f64 / 1000.0;

    println!("Executed 1000 service queries in {:?}", query_duration);
    println!("Rate: {:.0} queries/second", queries_per_sec);
    println!("Time per query: {:.2}ms", ms_per_query);

    if ms_per_query <= 1.0 {
        println!("CLAIM VERIFIED: <1ms query time achieved!");
    } else {
        println!("CLAIM FAILED: {:.2}ms per query (target: <1ms)", ms_per_query);
    }

    // Final verdict
    println!("\nFINAL VERDICT");
    println!("=============");

    let claims_verified = (spans_per_sec >= 10_000.0) as i32 +
                         (ns_per_span <= 10_000.0) as i32 +
                         (mb_per_million <= 100.0) as i32 +
                         (ms_per_query <= 1.0) as i32;

    match claims_verified {
        4 => println!("ALL CLAIMS VERIFIED! Urpo is indeed fast!"),
        3 => println!("MOSTLY VERIFIED! Urpo is pretty fast!"),
        2 => println!("PARTIALLY VERIFIED. Room for improvement."),
        1 => println!("MOSTLY FAILED. Back to the drawing board."),
        0 => println!("ALL CLAIMS FAILED! Time for humble pie."),
        _ => unreachable!(),
    }

    println!("\nClaims verified: {}/4", claims_verified);

    Ok(())
}
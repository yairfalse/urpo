//! Standalone OTEL Sender - no dependencies on urpo_lib
//!
//! Sends real OTEL spans to any OTLP endpoint

use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Simple OTEL Sender");
    println!("📡 Target: http://localhost:4317");
    println!("⚡ Sending 5000 spans at 1000/sec...");
    println!("{}", "─".repeat(40));

    // Simple HTTP client approach (no gRPC complexity)
    let client = reqwest::Client::new();
    let target = "http://localhost:4318/v1/traces"; // HTTP endpoint

    let mut sent = 0;
    let start = Instant::now();

    for batch in 0..50 { // 50 batches of 100 spans each = 5000 total
        let batch_start = Instant::now();

        // Generate simple JSON payload
        let payload = generate_otel_json_batch(100, batch);

        // Send to OTLP HTTP endpoint
        match client
            .post(target)
            .header("Content-Type", "application/json")
            .body(payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    sent += 100;
                    let rate = sent as f64 / start.elapsed().as_secs_f64();
                    println!("✅ Batch {}: {} spans sent ({:.0}/s)", batch + 1, sent, rate);
                } else {
                    println!("❌ Batch {} failed: {}", batch + 1, response.status());
                }
            }
            Err(e) => {
                println!("❌ Batch {} error: {}", batch + 1, e);
            }
        }

        // Rate limiting - 1000/sec = 100ms per 100-span batch
        let elapsed = batch_start.elapsed();
        if elapsed < Duration::from_millis(100) {
            tokio::time::sleep(Duration::from_millis(100) - elapsed).await;
        }
    }

    let total_time = start.elapsed();
    let final_rate = sent as f64 / total_time.as_secs_f64();

    println!("{}", "─".repeat(40));
    println!("🎉 Complete! {} spans in {:.1}s ({:.0}/s)", sent, total_time.as_secs_f64(), final_rate);

    Ok(())
}

fn generate_otel_json_batch(count: usize, batch_id: usize) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let mut spans = Vec::new();

    for i in 0..count {
        let span_id = format!("{:016x}", (batch_id * 1000 + i) as u64);
        let trace_id = format!("{:032x}", (batch_id * 100 + i / 10) as u128);
        let duration_ns = 1_000_000 + (i * 10_000) as u64; // 1-10ms

        let span = format!(
            r#"{{
                "traceId": "{}",
                "spanId": "{}",
                "name": "operation-{}",
                "kind": 2,
                "startTimeUnixNano": {},
                "endTimeUnixNano": {},
                "attributes": [
                    {{"key": "service.name", "value": {{"stringValue": "service-{}"}}}},
                    {{"key": "http.method", "value": {{"stringValue": "GET"}}}},
                    {{"key": "http.status_code", "value": {{"intValue": 200}}}}
                ],
                "status": {{"code": 1}}
            }}"#,
            trace_id,
            span_id,
            i % 10,
            now - duration_ns,
            now,
            i % 5
        );
        spans.push(span);
    }

    format!(
        r#"{{
            "resourceSpans": [
                {{
                    "resource": {{
                        "attributes": [
                            {{"key": "service.name", "value": {{"stringValue": "load-generator"}}}}
                        ]
                    }},
                    "scopeSpans": [
                        {{
                            "scope": {{
                                "name": "otel-sender",
                                "version": "1.0.0"
                            }},
                            "spans": [
                                {}
                            ]
                        }}
                    ]
                }}
            ]
        }}"#,
        spans.join(",")
    )
}

// Cargo.toml dependencies needed:
// [dependencies]
// tokio = { version = "1", features = ["full"] }
// reqwest = { version = "0.11", features = ["json"] }
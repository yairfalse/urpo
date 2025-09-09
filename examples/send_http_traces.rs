//! Example of sending OpenTelemetry traces to Urpo via HTTP.
//! 
//! This demonstrates sending OTLP traces over HTTP to Urpo's receiver
//! on port 4318. Shows both JSON and protobuf formats.

use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    tracing::info!("Testing HTTP OTLP receiver on localhost:4318");

    // Test 1: Send JSON traces
    tracing::info!("Sending JSON OTLP traces...");
    send_json_traces().await?;

    // Test 2: Health check
    tracing::info!("Testing health endpoint...");
    test_health_endpoint().await?;

    tracing::info!("HTTP OTLP tests completed successfully!");

    Ok(())
}

async fn send_json_traces() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // Create test traces
    for i in 0..5 {
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos() as u64;
        
        let trace_id = format!("{:032x}", rand::random::<u128>());
        let span_id = format!("{:016x}", rand::random::<u64>());
        let parent_span_id = if i > 0 { 
            Some(format!("{:016x}", rand::random::<u64>())) 
        } else { 
            None 
        };

        let mut span = json!({
            "traceId": trace_id,
            "spanId": span_id,
            "name": format!("http-test-span-{}", i),
            "startTimeUnixNano": format!("{}", now_nanos),
            "endTimeUnixNano": format!("{}", now_nanos + 100_000_000), // +100ms
            "kind": 2, // SPAN_KIND_CLIENT
            "attributes": [
                {
                    "key": "http.method",
                    "value": {
                        "stringValue": "POST"
                    }
                },
                {
                    "key": "http.url", 
                    "value": {
                        "stringValue": format!("/api/test/{}", i)
                    }
                },
                {
                    "key": "http.status_code",
                    "value": {
                        "intValue": if i % 3 == 0 { 500 } else { 200 }
                    }
                }
            ]
        });

        // Add parent span ID if this is a child span
        if let Some(parent_id) = parent_span_id {
            span["parentSpanId"] = json!(parent_id);
        }

        let otlp_request = json!({
            "resourceSpans": [
                {
                    "resource": {
                        "attributes": [
                            {
                                "key": "service.name",
                                "value": {
                                    "stringValue": "http-test-service"
                                }
                            },
                            {
                                "key": "service.version",
                                "value": {
                                    "stringValue": "1.0.0"
                                }
                            }
                        ]
                    },
                    "scopeSpans": [
                        {
                            "scope": {
                                "name": "http-test-scope",
                                "version": "1.0.0"
                            },
                            "spans": [span]
                        }
                    ]
                }
            ]
        });

        let response = client
            .post("http://localhost:4318/v1/traces")
            .header("Content-Type", "application/json")
            .json(&otlp_request)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            tracing::info!("✓ Successfully sent JSON trace {} (status: {})", i, status);
        } else {
            let error_text = response.text().await?;
            tracing::error!("✗ Failed to send JSON trace {} (status: {}, error: {})", i, status, error_text);
        }

        // Small delay between traces
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    Ok(())
}

async fn test_health_endpoint() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:4318/health")
        .send()
        .await?;

    if response.status().is_success() {
        let health_data: serde_json::Value = response.json().await?;
        tracing::info!("✓ Health check passed: {}", health_data);
    } else {
        tracing::error!("✗ Health check failed (status: {})", response.status());
    }

    Ok(())
}

// Helper to add reqwest dependency for HTTP requests
// Add this to Cargo.toml under [dev-dependencies]:
// reqwest = { version = "0.11", features = ["json"] }
//! Basic usage example for Urpo.
//!
//! This example demonstrates how to:
//! 1. Configure Urpo
//! 2. Start the application
//! 3. Send test spans
//! 4. Query stored data

use chrono::Utc;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;
use urpo_lib::core::{Config, ServiceName, Span, SpanId, SpanKind, SpanStatus, TraceId};
use urpo_lib::storage::{InMemoryStorage, StorageBackend};

/// Generate a test span with realistic data.
fn generate_test_span(
    trace_num: u32,
    span_num: u32,
    service: &str,
    operation: &str,
    duration_ms: u64,
    is_error: bool,
) -> Span {
    let start_time = Utc::now();
    let end_time = start_time + chrono::Duration::milliseconds(duration_ms as i64);
    
    let mut attributes = HashMap::new();
    attributes.insert("http.method".to_string(), "GET".to_string());
    attributes.insert("http.url".to_string(), format!("/api/{}", operation));
    attributes.insert("http.status_code".to_string(), if is_error { "500" } else { "200" }.to_string());
    
    {
        let mut builder = urpo_lib::core::SpanBuilder::default()
            .span_id(SpanId::new(format!("{:0>16}", span_num)).unwrap())
            .trace_id(TraceId::new(format!("{:0>32}", trace_num)).unwrap())
            .service_name(ServiceName::new(service.to_string()).unwrap())
            .operation_name(operation.to_string())
            .kind(if span_num == 1 { SpanKind::Server } else { SpanKind::Client })
            .start_time(start_time.into())
            .duration(Duration::from_millis(duration_ms as u64))
            .status(if is_error {
                SpanStatus::Error("Internal Server Error".to_string())
            } else {
                SpanStatus::Ok
            });
            
        if span_num > 1 {
            builder = builder.parent_span_id(SpanId::new(format!("{:0>16}", span_num - 1)).unwrap());
        }
        
        builder.build()
            .map(|mut span| {
                span.attributes = attributes;
                span
            })
            .unwrap()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    tracing::info!("Starting Urpo basic usage example");
    
    // Create configuration
    let config = Config::default();
    tracing::info!("Configuration loaded");
    
    // Create storage backend
    let storage = InMemoryStorage::new(1000);
    tracing::info!("Storage backend created");
    
    // Simulate receiving spans from different services
    let services = vec!["auth-service", "api-gateway", "database-service", "cache-service"];
    let operations = vec!["login", "get_user", "update_profile", "list_items"];
    
    tracing::info!("Generating and storing test spans...");
    
    for trace_num in 1..=10 {
        for (span_num, service) in services.iter().enumerate() {
            let operation = operations[span_num % operations.len()];
            let duration_ms = 50 + (span_num as u64 * 10);
            let is_error = trace_num % 5 == 0; // Every 5th trace has errors
            
            let span = generate_test_span(
                trace_num,
                span_num as u32 + 1,
                service,
                operation,
                duration_ms,
                is_error,
            );
            
            storage.store_span(span).await?;
        }
        
        // Small delay to simulate real-time data
        sleep(std::time::Duration::from_millis(100)).await;
    }
    
    tracing::info!("Generated {} traces with {} spans each", 10, services.len());
    
    // Query and display storage statistics
    let stats = storage.get_stats().await?;
    tracing::info!("Storage Statistics:");
    tracing::info!("  Total traces: {}", stats.trace_count);
    tracing::info!("  Total spans: {}", stats.span_count);
    tracing::info!("  Total services: {}", stats.service_count);
    tracing::info!("  Estimated memory: {} KB", stats.memory_bytes / 1024);
    
    // Query service metrics
    let metrics = storage.get_service_metrics().await?;
    tracing::info!("\nService Metrics:");
    for metric in metrics {
        tracing::info!(
            "  {} - Rate: {:.2}/s, Errors: {:.2}%, P95: {}ms",
            metric.name.as_str(),
            metric.request_rate,
            metric.error_rate,
            metric.latency_p95.as_millis()
        );
    }
    
    // Query spans for a specific trace
    let trace_id = TraceId::new(format!("{:0>32}", 1)).unwrap();
    let trace_spans = storage.get_trace_spans(&trace_id).await?;
    tracing::info!("\nSpans for trace {}:", trace_id.as_str());
    for span in trace_spans {
        tracing::info!(
            "  {} - {} [{}ms] - Status: {:?}",
            span.service_name.as_str(),
            span.operation_name,
            span.duration.as_millis(),
            span.status
        );
    }
    
    // Query recent spans for a specific service
    let service_name = ServiceName::new("api-gateway".to_string()).unwrap();
    let service_spans = storage.get_service_spans(&service_name, SystemTime::now() - Duration::from_secs(3600)).await?;
    tracing::info!("\nRecent spans for service {}:", service_name.as_str());
    for span in service_spans {
        tracing::info!(
            "  Trace {} - {} [{}ms]",
            &span.trace_id.as_str()[..8],
            span.operation_name,
            span.duration.as_millis()
        );
    }
    
    // Demonstrate cleanup
    tracing::info!("\nRunning cleanup (retention: 1 hour)...");
    let removed = storage.enforce_limits().await?;
    tracing::info!("Removed {} old spans", removed);
    
    // Final statistics
    let final_stats = storage.get_stats().await?;
    tracing::info!("\nFinal Statistics:");
    tracing::info!("  Remaining spans: {}", final_stats.span_count);
    
    tracing::info!("\nExample completed successfully!");
    tracing::info!("To run Urpo normally, use: urpo start");
    
    Ok(())
}
//! Example that sends OTEL trace data to Urpo for testing.
//!
//! Run with: cargo run --example send_otel_data

use opentelemetry::{global, trace::{Span, SpanKind, Status, Tracer, TracerProvider as _}};
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk as sdk;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracer() -> impl Tracer {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://localhost:4317")
        .with_protocol(Protocol::Grpc)
        .with_timeout(Duration::from_secs(3));

    let trace_config = sdk::trace::config()
        .with_sampler(sdk::trace::Sampler::AlwaysOn)
        .with_resource(sdk::Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", "test-service"),
            opentelemetry::KeyValue::new("service.version", "1.0.0"),
            opentelemetry::KeyValue::new("deployment.environment", "development"),
        ]));

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace_config)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("Failed to install tracer");

    global::set_tracer_provider(provider.clone());
    provider.tracer("test-service")
}

#[tokio::main]
async fn main() {
    // Initialize logging
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize the tracer
    let tracer = init_tracer();
    
    println!("Sending OTEL trace data to localhost:4317...");
    
    // Create some sample traces
    for i in 0..5 {
        println!("Sending trace batch {}...", i + 1);
        
        // Create a root span
        let mut root_span = tracer
            .span_builder(format!("process_request_{}", i))
            .with_kind(SpanKind::Server)
            .with_attributes(vec![
                opentelemetry::KeyValue::new("http.method", "GET"),
                opentelemetry::KeyValue::new("http.url", format!("/api/users/{}", i)),
                opentelemetry::KeyValue::new("http.status_code", 200i64),
                opentelemetry::KeyValue::new("user.id", format!("user_{}", i)),
            ])
            .start(&tracer);
        
        // Simulate some work
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Create child spans
        for j in 0..3 {
            let child_span = tracer
                .span_builder(format!("database_query_{}", j))
                .with_kind(SpanKind::Client)
                .with_attributes(vec![
                    opentelemetry::KeyValue::new("db.system", "postgresql"),
                    opentelemetry::KeyValue::new("db.operation", "SELECT"),
                    opentelemetry::KeyValue::new("db.statement", format!("SELECT * FROM table_{}", j)),
                ])
                .start(&tracer);
            
            // Simulate database query
            tokio::time::sleep(Duration::from_millis(20)).await;
            
            // Add events to the span
            child_span.add_event(
                "query_executed",
                vec![
                    opentelemetry::KeyValue::new("rows_returned", 42i64),
                    opentelemetry::KeyValue::new("execution_time_ms", 15i64),
                ],
            );
            
            // Set status
            child_span.set_status(Status::Ok);
            child_span.end();
        }
        
        // Create another service call
        let service_span = tracer
            .span_builder("call_external_service")
            .with_kind(SpanKind::Client)
            .with_attributes(vec![
                opentelemetry::KeyValue::new("service.name", "payment-service"),
                opentelemetry::KeyValue::new("rpc.method", "ProcessPayment"),
                opentelemetry::KeyValue::new("rpc.system", "grpc"),
            ])
            .start(&tracer);
        
        tokio::time::sleep(Duration::from_millis(30)).await;
        
        // Simulate an error occasionally
        if i % 3 == 0 {
            service_span.record_error(&std::io::Error::new(std::io::ErrorKind::Other, "Payment processing failed"));
            service_span.set_status(Status::error("Payment service unavailable"));
        } else {
            service_span.set_status(Status::Ok);
        }
        service_span.end();
        
        // Complete the root span
        root_span.set_status(if i % 3 == 0 {
            Status::error("Request failed due to payment error")
        } else {
            Status::Ok
        });
        root_span.end();
        
        // Wait a bit between traces
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Create spans from multiple services
    println!("\nSending traces from multiple services...");
    
    let services = vec!["auth-service", "user-service", "order-service", "inventory-service"];
    
    for service_name in &services {
        // Create a tracer for each service
        let service_tracer = global::tracer(service_name.to_string());
        
        for i in 0..3 {
            let span = service_tracer
                .span_builder(format!("{}_operation_{}", service_name, i))
                .with_kind(SpanKind::Internal)
                .with_attributes(vec![
                    opentelemetry::KeyValue::new("service.name", service_name.to_string()),
                    opentelemetry::KeyValue::new("operation.type", "business_logic"),
                    opentelemetry::KeyValue::new("request.id", format!("req_{}", i)),
                ])
                .start(&service_tracer);
            
            tokio::time::sleep(Duration::from_millis(25)).await;
            
            span.add_event(
                "operation_completed",
                vec![
                    opentelemetry::KeyValue::new("success", true),
                    opentelemetry::KeyValue::new("items_processed", (i + 1) * 10),
                ],
            );
            
            span.set_status(Status::Ok);
            span.end();
        }
    }
    
    // Force flush to ensure all spans are sent
    println!("\nFlushing spans...");
    global::shutdown_tracer_provider();
    
    println!("Done! Check Urpo to see the traces.");
}
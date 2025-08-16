//! Simple test to send a single trace to Urpo
//!
//! Run with: cargo run --example send_test_trace

use opentelemetry_proto::tonic::{
    collector::trace::v1::{
        trace_service_client::TraceServiceClient, ExportTraceServiceRequest,
    },
    common::v1::{any_value, AnyValue, InstrumentationScope, KeyValue},
    resource::v1::Resource,
    trace::v1::{span, ResourceSpans, ScopeSpans, Span, Status},
};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Urpo at localhost:4317...");
    
    let mut client = TraceServiceClient::connect("http://localhost:4317").await?;
    
    // Create a simple span
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let start_time_nanos = now.as_nanos() as u64;
    let end_time_nanos = start_time_nanos + 100_000_000; // 100ms later
    
    let span = Span {
        trace_id: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], // 16 bytes
        span_id: vec![1, 2, 3, 4, 5, 6, 7, 8], // 8 bytes
        trace_state: String::new(),
        parent_span_id: vec![],
        flags: 1, // Sampled
        name: "test_operation".to_string(),
        kind: span::SpanKind::Server as i32,
        start_time_unix_nano: start_time_nanos,
        end_time_unix_nano: end_time_nanos,
        attributes: vec![
            KeyValue {
                key: "http.method".to_string(),
                value: Some(AnyValue {
                    value: Some(any_value::Value::StringValue("GET".to_string())),
                }),
            },
            KeyValue {
                key: "http.url".to_string(),
                value: Some(AnyValue {
                    value: Some(any_value::Value::StringValue("/api/test".to_string())),
                }),
            },
            KeyValue {
                key: "http.status_code".to_string(),
                value: Some(AnyValue {
                    value: Some(any_value::Value::IntValue(200)),
                }),
            },
        ],
        dropped_attributes_count: 0,
        events: vec![],
        dropped_events_count: 0,
        links: vec![],
        dropped_links_count: 0,
        status: Some(Status {
            code: 1, // OK
            message: String::new(),
        }),
    };
    
    // Create resource with service name
    let resource = Resource {
        attributes: vec![
            KeyValue {
                key: "service.name".to_string(),
                value: Some(AnyValue {
                    value: Some(any_value::Value::StringValue("test-service".to_string())),
                }),
            },
            KeyValue {
                key: "service.version".to_string(),
                value: Some(AnyValue {
                    value: Some(any_value::Value::StringValue("1.0.0".to_string())),
                }),
            },
        ],
        dropped_attributes_count: 0,
    };
    
    // Create the scope spans
    let scope_spans = ScopeSpans {
        scope: Some(InstrumentationScope {
            name: "test-instrumentation".to_string(),
            version: "1.0.0".to_string(),
            attributes: vec![],
            dropped_attributes_count: 0,
        }),
        spans: vec![span],
        schema_url: String::new(),
    };
    
    // Create resource spans
    let resource_spans = ResourceSpans {
        resource: Some(resource),
        scope_spans: vec![scope_spans],
        schema_url: String::new(),
    };
    
    // Create the export request
    let request = ExportTraceServiceRequest {
        resource_spans: vec![resource_spans],
    };
    
    println!("Sending test trace to Urpo...");
    let response = client.export(request).await?;
    
    println!("Response: {:?}", response);
    println!("Success! Trace sent to Urpo.");
    
    Ok(())
}
//! Continuously sends OTEL trace data to Urpo for testing the dashboard
//!
//! Run with: cargo run --example continuous_sender

use opentelemetry_proto::tonic::{
    collector::trace::v1::{
        trace_service_client::TraceServiceClient, ExportTraceServiceRequest,
    },
    common::v1::{any_value, AnyValue, InstrumentationScope, KeyValue},
    resource::v1::Resource,
    trace::v1::{span, ResourceSpans, ScopeSpans, Span, Status},
};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Urpo at localhost:4317...");
    println!("Press Ctrl+C to stop sending traces");
    
    let mut client = TraceServiceClient::connect("http://localhost:4317").await?;
    
    let services = vec![
        "frontend-service",
        "auth-service", 
        "user-service",
        "order-service",
        "payment-service",
        "inventory-service",
        "notification-service"
    ];
    
    let operations = vec![
        "GET /api/users",
        "POST /api/orders",
        "GET /api/products",
        "PUT /api/cart",
        "DELETE /api/sessions",
        "GET /api/health",
        "POST /api/payment",
    ];
    
    let mut rng = rand::thread_rng();
    let mut trace_counter = 0u64;
    
    loop {
        // Generate batch of spans
        let batch_size = rng.gen_range(1..=5);
        
        for _ in 0..batch_size {
            trace_counter += 1;
            
            // Random service and operation
            let service_name = services[rng.gen_range(0..services.len())];
            let operation = operations[rng.gen_range(0..operations.len())];
            
            // Generate trace with multiple spans
            let trace_id = generate_trace_id(trace_counter);
            let root_span_id = generate_span_id(1);
            
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let start_time_nanos = now.as_nanos() as u64;
            
            // Random duration between 10ms and 500ms
            let duration_ms = rng.gen_range(10..=500);
            let end_time_nanos = start_time_nanos + (duration_ms * 1_000_000);
            
            // Random status - 90% success, 10% error
            let status = if rng.gen_range(0..100) < 90 {
                Status { code: 1, message: String::new() } // OK
            } else {
                Status { 
                    code: 2, // ERROR
                    message: format!("Error in {}", operation) 
                }
            };
            
            // Random HTTP status code
            let http_status = if status.code == 1 {
                if rng.gen_bool(0.5) { 200 } else { 201 }
            } else {
                if rng.gen_bool(0.5) { 500 } else { 503 }
            };
            
            let span = Span {
                trace_id: trace_id.clone(),
                span_id: root_span_id,
                trace_state: String::new(),
                parent_span_id: vec![],
                flags: 1,
                name: operation.to_string(),
                kind: span::SpanKind::Server as i32,
                start_time_unix_nano: start_time_nanos,
                end_time_unix_nano: end_time_nanos,
                attributes: vec![
                    KeyValue {
                        key: "http.method".to_string(),
                        value: Some(AnyValue {
                            value: Some(any_value::Value::StringValue(
                                operation.split(' ').next().unwrap_or("GET").to_string()
                            )),
                        }),
                    },
                    KeyValue {
                        key: "http.url".to_string(),
                        value: Some(AnyValue {
                            value: Some(any_value::Value::StringValue(
                                operation.split(' ').nth(1).unwrap_or("/").to_string()
                            )),
                        }),
                    },
                    KeyValue {
                        key: "http.status_code".to_string(),
                        value: Some(AnyValue {
                            value: Some(any_value::Value::IntValue(http_status)),
                        }),
                    },
                    KeyValue {
                        key: "trace.id".to_string(),
                        value: Some(AnyValue {
                            value: Some(any_value::Value::IntValue(trace_counter as i64)),
                        }),
                    },
                ],
                dropped_attributes_count: 0,
                events: vec![],
                dropped_events_count: 0,
                links: vec![],
                dropped_links_count: 0,
                status: Some(status),
            };
            
            // Create child spans
            let mut spans = vec![span];
            
            // Add 1-3 child spans
            let child_count = rng.gen_range(1..=3);
            for i in 0..child_count {
                let child_span_id = generate_span_id((i + 2) as u64);
                let child_start = start_time_nanos + (i as u64 * 10_000_000);
                let child_end = child_start + rng.gen_range(5_000_000..50_000_000);
                
                let child_span = Span {
                    trace_id: trace_id.clone(),
                    span_id: child_span_id,
                    trace_state: String::new(),
                    parent_span_id: root_span_id.clone(),
                    flags: 1,
                    name: format!("database_query_{}", i),
                    kind: span::SpanKind::Client as i32,
                    start_time_unix_nano: child_start,
                    end_time_unix_nano: child_end,
                    attributes: vec![
                        KeyValue {
                            key: "db.system".to_string(),
                            value: Some(AnyValue {
                                value: Some(any_value::Value::StringValue("postgresql".to_string())),
                            }),
                        },
                        KeyValue {
                            key: "db.operation".to_string(),
                            value: Some(AnyValue {
                                value: Some(any_value::Value::StringValue("SELECT".to_string())),
                            }),
                        },
                    ],
                    dropped_attributes_count: 0,
                    events: vec![],
                    dropped_events_count: 0,
                    links: vec![],
                    dropped_links_count: 0,
                    status: Some(Status { code: 1, message: String::new() }),
                };
                
                spans.push(child_span);
            }
            
            // Create resource with service name
            let resource = Resource {
                attributes: vec![
                    KeyValue {
                        key: "service.name".to_string(),
                        value: Some(AnyValue {
                            value: Some(any_value::Value::StringValue(service_name.to_string())),
                        }),
                    },
                    KeyValue {
                        key: "service.version".to_string(),
                        value: Some(AnyValue {
                            value: Some(any_value::Value::StringValue("1.0.0".to_string())),
                        }),
                    },
                    KeyValue {
                        key: "deployment.environment".to_string(),
                        value: Some(AnyValue {
                            value: Some(any_value::Value::StringValue("production".to_string())),
                        }),
                    },
                ],
                dropped_attributes_count: 0,
            };
            
            // Create the scope spans
            let scope_spans = ScopeSpans {
                scope: Some(InstrumentationScope {
                    name: "urpo-test".to_string(),
                    version: "1.0.0".to_string(),
                    attributes: vec![],
                    dropped_attributes_count: 0,
                }),
                spans,
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
            
            // Send the trace
            match client.export(request).await {
                Ok(_) => {
                    if trace_counter % 10 == 0 {
                        println!("Sent {} traces so far...", trace_counter);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to send trace: {}", e);
                }
            }
        }
        
        // Wait before sending next batch
        sleep(Duration::from_millis(rng.gen_range(100..=500))).await;
    }
}

fn generate_trace_id(counter: u64) -> Vec<u8> {
    let mut trace_id = vec![0u8; 16];
    let bytes = counter.to_be_bytes();
    trace_id[8..16].copy_from_slice(&bytes);
    trace_id[0] = 1; // Make sure it's not all zeros
    trace_id
}

fn generate_span_id(counter: u64) -> Vec<u8> {
    let mut span_id = vec![0u8; 8];
    let bytes = counter.to_be_bytes();
    span_id.copy_from_slice(&bytes);
    span_id[0] = 1; // Make sure it's not all zeros
    span_id
}
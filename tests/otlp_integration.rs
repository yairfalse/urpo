//! OTLP Protocol Integration Tests
//!
//! Tests full OTLP gRPC receiver functionality including:
//! - Protocol compliance
//! - Resource extraction
//! - Batch processing
//! - Error handling

use opentelemetry_proto::tonic::{
    collector::trace::v1::{
        trace_service_server::TraceService, ExportTraceServiceRequest, ExportTraceServiceResponse,
    },
    common::v1::{any_value::Value, AnyValue, KeyValue},
    resource::v1::Resource,
    trace::v1::{span, ResourceSpans, ScopeSpans, Span, Status},
};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response};
use urpo_lib::{
    receiver::{create_trace_service_server, OtelReceiver},
    storage::memory::InMemoryStorage,
};

mod common;
use common::*;

/// Test basic OTLP span reception
#[tokio::test]
async fn test_otlp_basic_span_reception() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(10000)));
    let receiver = OtelReceiver::new(storage.clone());

    let request = create_test_export_request(1, 10);
    let result = receiver.export(Request::new(request)).await;

    assert!(result.is_ok());

    let storage_guard = storage.lock().await;
    let spans = storage_guard.get_all_spans();
    assert_eq!(spans.len(), 10);
}

/// Test OTLP batch processing with multiple services
#[tokio::test]
async fn test_otlp_batch_multiple_services() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(10000)));
    let receiver = OtelReceiver::new(storage.clone());

    let mut resource_spans = Vec::new();

    // Create spans from 3 different services
    for service_idx in 0..3 {
        let service_name = format!("service-{}", service_idx);
        resource_spans.push(create_resource_spans(&service_name, 100));
    }

    let request = ExportTraceServiceRequest { resource_spans };
    let result = receiver.export(Request::new(request)).await;

    assert!(result.is_ok());

    let storage_guard = storage.lock().await;
    assert_eq!(storage_guard.get_all_spans().len(), 300); // 3 services * 100 spans

    // Verify service names were extracted correctly
    let services = storage_guard.list_services();
    assert_eq!(services.len(), 3);
}

/// Test OTLP protocol validation
#[tokio::test]
async fn test_otlp_invalid_span_handling() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(10000)));
    let receiver = OtelReceiver::new(storage.clone());

    // Create span with invalid timing (start_time = 0)
    let invalid_span = Span {
        trace_id: generate_trace_id(),
        span_id: generate_span_id(),
        name: "invalid-span".to_string(),
        start_time_unix_nano: 0, // Invalid!
        end_time_unix_nano: 1000,
        ..Default::default()
    };

    let request = ExportTraceServiceRequest {
        resource_spans: vec![ResourceSpans {
            resource: Some(create_test_resource("test-service")),
            scope_spans: vec![ScopeSpans {
                spans: vec![invalid_span],
                ..Default::default()
            }],
            ..Default::default()
        }],
    };

    let result = receiver.export(Request::new(request)).await;

    // Should succeed but not store invalid span
    assert!(result.is_ok());

    let storage_guard = storage.lock().await;
    assert_eq!(storage_guard.get_all_spans().len(), 0);
}

/// Test resource attribute extraction
#[tokio::test]
async fn test_otlp_resource_extraction() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(10000)));
    let receiver = OtelReceiver::new(storage.clone());

    let mut resource = create_test_resource("payment-service");
    resource.attributes.push(KeyValue {
        key: "service.version".to_string(),
        value: Some(AnyValue {
            value: Some(Value::StringValue("1.2.3".to_string())),
        }),
    });
    resource.attributes.push(KeyValue {
        key: "deployment.environment".to_string(),
        value: Some(AnyValue {
            value: Some(Value::StringValue("production".to_string())),
        }),
    });

    let request = ExportTraceServiceRequest {
        resource_spans: vec![ResourceSpans {
            resource: Some(resource),
            scope_spans: vec![create_test_scope_spans(5)],
            ..Default::default()
        }],
    };

    let result = receiver.export(Request::new(request)).await;
    assert!(result.is_ok());

    let storage_guard = storage.lock().await;
    let spans = storage_guard.get_all_spans();
    assert_eq!(spans.len(), 5);

    // All spans should have the same service name
    for span in spans {
        assert_eq!(span.service_name, "payment-service");
    }
}

/// Test span timing overflow protection
#[tokio::test]
async fn test_otlp_timing_overflow_protection() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(10000)));
    let receiver = OtelReceiver::new(storage.clone());

    // Create span with potential overflow
    let span = Span {
        trace_id: generate_trace_id(),
        span_id: generate_span_id(),
        name: "overflow-test".to_string(),
        start_time_unix_nano: u64::MAX - 1000,
        end_time_unix_nano: u64::MAX,
        ..Default::default()
    };

    let request = create_request_with_spans(vec![span]);
    let result = receiver.export(Request::new(request)).await;

    // Should handle gracefully
    assert!(result.is_ok());
}

/// Test concurrent OTLP requests
#[tokio::test]
async fn test_otlp_concurrent_requests() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(100000)));
    let receiver = Arc::new(OtelReceiver::new(storage.clone()));

    let mut handles = vec![];

    // Send 10 concurrent requests
    for i in 0..10 {
        let receiver = receiver.clone();
        let handle = tokio::spawn(async move {
            let request = create_test_export_request(1, 100);
            receiver.export(Request::new(request)).await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    let storage_guard = storage.lock().await;
    assert_eq!(storage_guard.get_all_spans().len(), 1000); // 10 requests * 100 spans
}

/// Test span status and error handling
#[tokio::test]
async fn test_otlp_span_status_processing() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(10000)));
    let receiver = OtelReceiver::new(storage.clone());

    let error_span = Span {
        trace_id: generate_trace_id(),
        span_id: generate_span_id(),
        name: "error-operation".to_string(),
        start_time_unix_nano: 1000,
        end_time_unix_nano: 2000,
        status: Some(Status {
            code: 2, // ERROR
            message: "Database connection failed".to_string(),
        }),
        ..Default::default()
    };

    let ok_span = Span {
        trace_id: generate_trace_id(),
        span_id: generate_span_id(),
        name: "success-operation".to_string(),
        start_time_unix_nano: 1000,
        end_time_unix_nano: 2000,
        status: Some(Status {
            code: 1, // OK
            message: String::new(),
        }),
        ..Default::default()
    };

    let request = create_request_with_spans(vec![error_span, ok_span]);
    let result = receiver.export(Request::new(request)).await;

    assert!(result.is_ok());

    let storage_guard = storage.lock().await;
    let spans = storage_guard.get_all_spans();
    assert_eq!(spans.len(), 2);

    // Verify error span
    let error_span = spans.iter().find(|s| s.name == "error-operation").unwrap();
    assert!(error_span.is_error);

    // Verify success span
    let ok_span = spans
        .iter()
        .find(|s| s.name == "success-operation")
        .unwrap();
    assert!(!ok_span.is_error);
}

/// Test memory limit enforcement
#[tokio::test]
async fn test_otlp_memory_limit_enforcement() {
    // Create storage with small limit
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(100)));
    let receiver = OtelReceiver::new(storage.clone());

    // Try to send 1000 spans
    let request = create_test_export_request(1, 1000);
    let result = receiver.export(Request::new(request)).await;

    assert!(result.is_ok());

    let storage_guard = storage.lock().await;
    // Should only store up to limit
    assert!(storage_guard.get_all_spans().len() <= 100);
}

/// Test span parent-child relationships
#[tokio::test]
async fn test_otlp_trace_hierarchy() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(10000)));
    let receiver = OtelReceiver::new(storage.clone());

    let trace_id = generate_trace_id();
    let root_span_id = generate_span_id();

    // Create root span
    let root_span = Span {
        trace_id: trace_id.clone(),
        span_id: root_span_id.clone(),
        name: "root".to_string(),
        start_time_unix_nano: 1000,
        end_time_unix_nano: 5000,
        ..Default::default()
    };

    // Create child span
    let child_span = Span {
        trace_id: trace_id.clone(),
        span_id: generate_span_id(),
        parent_span_id: root_span_id.clone(),
        name: "child".to_string(),
        start_time_unix_nano: 2000,
        end_time_unix_nano: 4000,
        ..Default::default()
    };

    let request = create_request_with_spans(vec![root_span, child_span]);
    let result = receiver.export(Request::new(request)).await;

    assert!(result.is_ok());

    let storage_guard = storage.lock().await;
    let spans = storage_guard.get_trace_spans(&format_trace_id(&trace_id));

    assert_eq!(spans.len(), 2);

    // Verify parent-child relationship
    let child = spans.iter().find(|s| s.name == "child").unwrap();
    assert!(child.parent_span_id.is_some());
}

/// Test OTLP export partial success response
#[tokio::test]
async fn test_otlp_partial_success_response() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new(5)));
    let receiver = OtelReceiver::new(storage.clone());

    // Send more spans than storage can hold
    let request = create_test_export_request(1, 10);
    let result = receiver.export(Request::new(request)).await;

    assert!(result.is_ok());

    let response = result.unwrap().into_inner();

    // Check if partial success is reported
    if let Some(partial) = response.partial_success {
        assert!(partial.rejected_spans > 0);
        assert!(!partial.error_message.is_empty());
    }
}

// Helper functions

fn create_test_resource(service_name: &str) -> Resource {
    Resource {
        attributes: vec![KeyValue {
            key: "service.name".to_string(),
            value: Some(AnyValue {
                value: Some(Value::StringValue(service_name.to_string())),
            }),
        }],
        dropped_attributes_count: 0,
    }
}

fn create_test_scope_spans(count: usize) -> ScopeSpans {
    let mut spans = Vec::new();
    for i in 0..count {
        spans.push(Span {
            trace_id: generate_trace_id(),
            span_id: generate_span_id(),
            name: format!("span-{}", i),
            start_time_unix_nano: 1000 + (i as u64 * 100),
            end_time_unix_nano: 1100 + (i as u64 * 100),
            ..Default::default()
        });
    }

    ScopeSpans {
        spans,
        ..Default::default()
    }
}

fn create_resource_spans(service_name: &str, span_count: usize) -> ResourceSpans {
    ResourceSpans {
        resource: Some(create_test_resource(service_name)),
        scope_spans: vec![create_test_scope_spans(span_count)],
        ..Default::default()
    }
}

fn create_request_with_spans(spans: Vec<Span>) -> ExportTraceServiceRequest {
    ExportTraceServiceRequest {
        resource_spans: vec![ResourceSpans {
            resource: Some(create_test_resource("test-service")),
            scope_spans: vec![ScopeSpans {
                spans,
                ..Default::default()
            }],
            ..Default::default()
        }],
    }
}

fn create_test_export_request(
    service_count: usize,
    spans_per_service: usize,
) -> ExportTraceServiceRequest {
    let mut resource_spans = Vec::new();

    for i in 0..service_count {
        let service_name = format!("service-{}", i);
        resource_spans.push(create_resource_spans(&service_name, spans_per_service));
    }

    ExportTraceServiceRequest { resource_spans }
}

fn generate_trace_id() -> Vec<u8> {
    (0..16).map(|i| i as u8).collect()
}

fn generate_span_id() -> Vec<u8> {
    (0..8).map(|i| (i + 100) as u8).collect()
}

fn format_trace_id(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

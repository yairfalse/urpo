//! Integration tests for Urpo.

use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use urpo::core::{
    Config, ServiceName, Span, SpanId, SpanKind, SpanStatus, TraceId,
};
use urpo::storage::{InMemoryStorage, StorageBackend};

/// Create a test span with specified parameters.
fn create_test_span(trace_id: &str, span_id: &str, service: &str) -> Span {
    Span {
        span_id: SpanId::new(format!("{:0>16}", span_id)).unwrap(),
        trace_id: TraceId::new(format!("{:0>32}", trace_id)).unwrap(),
        parent_span_id: None,
        service_name: ServiceName::new(service.to_string()).unwrap(),
        operation_name: "test-operation".to_string(),
        kind: SpanKind::Server,
        start_time: Utc::now(),
        end_time: Utc::now() + chrono::Duration::milliseconds(100),
        status: SpanStatus::Ok,
        attributes: HashMap::new(),
        events: Vec::new(),
    }
}

#[tokio::test]
async fn test_end_to_end_span_storage() {
    // Create storage
    let storage = InMemoryStorage::new(100);
    
    // Store multiple spans
    let span1 = create_test_span("1", "1", "service-a");
    let span2 = create_test_span("1", "2", "service-a");
    let span3 = create_test_span("2", "3", "service-b");
    
    storage.store_span(span1.clone()).await.unwrap();
    storage.store_span(span2.clone()).await.unwrap();
    storage.store_span(span3.clone()).await.unwrap();
    
    // Verify trace spans
    let trace1_spans = storage
        .get_trace_spans(&TraceId::new(format!("{:0>32}", "1")).unwrap())
        .await
        .unwrap();
    assert_eq!(trace1_spans.len(), 2);
    
    let trace2_spans = storage
        .get_trace_spans(&TraceId::new(format!("{:0>32}", "2")).unwrap())
        .await
        .unwrap();
    assert_eq!(trace2_spans.len(), 1);
    
    // Verify service spans
    let service_a_spans = storage
        .get_service_spans(&ServiceName::new("service-a".to_string()).unwrap(), 10)
        .await
        .unwrap();
    assert_eq!(service_a_spans.len(), 2);
    
    let service_b_spans = storage
        .get_service_spans(&ServiceName::new("service-b".to_string()).unwrap(), 10)
        .await
        .unwrap();
    assert_eq!(service_b_spans.len(), 1);
    
    // Verify metrics
    let metrics = storage.get_service_metrics().await.unwrap();
    assert_eq!(metrics.len(), 2);
    
    // Verify stats
    let stats = storage.get_stats().await.unwrap();
    assert_eq!(stats.span_count, 3);
    assert_eq!(stats.trace_count, 2);
    assert_eq!(stats.service_count, 2);
}

#[tokio::test]
async fn test_storage_cleanup() {
    let storage = InMemoryStorage::new(100);
    
    // Create old span
    let mut old_span = create_test_span("1", "1", "service-a");
    old_span.end_time = Utc::now() - chrono::Duration::hours(2);
    old_span.start_time = old_span.end_time - chrono::Duration::milliseconds(100);
    
    // Create recent span
    let recent_span = create_test_span("2", "2", "service-b");
    
    storage.store_span(old_span).await.unwrap();
    storage.store_span(recent_span).await.unwrap();
    
    // Verify both spans exist
    let stats_before = storage.get_stats().await.unwrap();
    assert_eq!(stats_before.span_count, 2);
    
    // Run cleanup with 1 hour retention
    let removed = storage
        .cleanup(chrono::Duration::hours(1))
        .await
        .unwrap();
    assert_eq!(removed, 1);
    
    // Verify only recent span remains
    let stats_after = storage.get_stats().await.unwrap();
    assert_eq!(stats_after.span_count, 1);
}

#[tokio::test]
async fn test_config_validation() {
    // Valid config
    let valid_config = Config::default();
    assert!(valid_config.validate().is_ok());
    
    // Invalid sampling rate
    let mut invalid_config = Config::default();
    invalid_config.sampling_rate = 1.5;
    assert!(invalid_config.validate().is_err());
    
    // Invalid memory limit
    let mut invalid_config = Config::default();
    invalid_config.max_memory_mb = 0;
    assert!(invalid_config.validate().is_err());
    
    // Invalid max traces
    let mut invalid_config = Config::default();
    invalid_config.max_traces = 0;
    assert!(invalid_config.validate().is_err());
}

#[tokio::test]
async fn test_error_span_metrics() {
    let storage = InMemoryStorage::new(100);
    
    // Create spans with mixed status
    let mut error_span = create_test_span("1", "1", "service-a");
    error_span.status = SpanStatus::Error("test error".to_string());
    
    let ok_span1 = create_test_span("2", "2", "service-a");
    let ok_span2 = create_test_span("3", "3", "service-a");
    
    storage.store_span(error_span).await.unwrap();
    storage.store_span(ok_span1).await.unwrap();
    storage.store_span(ok_span2).await.unwrap();
    
    // Check metrics
    let metrics = storage.get_service_metrics().await.unwrap();
    assert_eq!(metrics.len(), 1);
    
    let service_metrics = &metrics[0];
    assert_eq!(service_metrics.span_count, 3);
    assert_eq!(service_metrics.error_count, 1);
    assert!((service_metrics.error_rate() - 33.33).abs() < 0.01);
}

#[test]
fn test_span_duration_calculation() {
    let start = Utc::now();
    let end = start + chrono::Duration::milliseconds(500);
    
    let span = Span {
        span_id: SpanId::new(format!("{:0>16}", "1")).unwrap(),
        trace_id: TraceId::new(format!("{:0>32}", "1")).unwrap(),
        parent_span_id: None,
        service_name: ServiceName::new("test".to_string()).unwrap(),
        operation_name: "test".to_string(),
        kind: SpanKind::Server,
        start_time: start,
        end_time: end,
        status: SpanStatus::Ok,
        attributes: HashMap::new(),
        events: Vec::new(),
    };
    
    let duration = span.duration();
    assert!(duration >= Duration::from_millis(499));
    assert!(duration <= Duration::from_millis(501));
}

#[test]
fn test_span_hierarchy() {
    let parent_span_id = SpanId::new(format!("{:0>16}", "1")).unwrap();
    
    let root_span = Span {
        span_id: parent_span_id.clone(),
        trace_id: TraceId::new(format!("{:0>32}", "1")).unwrap(),
        parent_span_id: None,
        service_name: ServiceName::new("test".to_string()).unwrap(),
        operation_name: "root".to_string(),
        kind: SpanKind::Server,
        start_time: Utc::now(),
        end_time: Utc::now() + chrono::Duration::milliseconds(100),
        status: SpanStatus::Ok,
        attributes: HashMap::new(),
        events: Vec::new(),
    };
    
    let child_span = Span {
        span_id: SpanId::new(format!("{:0>16}", "2")).unwrap(),
        trace_id: TraceId::new(format!("{:0>32}", "1")).unwrap(),
        parent_span_id: Some(parent_span_id),
        service_name: ServiceName::new("test".to_string()).unwrap(),
        operation_name: "child".to_string(),
        kind: SpanKind::Client,
        start_time: Utc::now(),
        end_time: Utc::now() + chrono::Duration::milliseconds(50),
        status: SpanStatus::Ok,
        attributes: HashMap::new(),
        events: Vec::new(),
    };
    
    assert!(root_span.is_root());
    assert!(!child_span.is_root());
}
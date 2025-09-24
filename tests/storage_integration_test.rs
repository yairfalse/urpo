//! Integration tests for storage backend with real test spans.

use std::time::Duration;
use urpo_lib::core::{ServiceMetrics, ServiceName, Span, SpanBuilder, SpanId, SpanStatus, TraceId};
use urpo_lib::storage::{InMemoryStorage, StorageBackend};

#[tokio::test]
async fn test_storage_with_real_spans() {
    // Create storage backend
    let storage = InMemoryStorage::new(1000);

    // Create real test spans
    let mut spans = Vec::new();
    for i in 0..100 {
        let span = SpanBuilder::default()
            .trace_id(TraceId::new(format!("trace_{:04}", i / 10)).unwrap())
            .span_id(SpanId::new(format!("span_{:04}", i)).unwrap())
            .service_name(ServiceName::new(format!("service_{}", i % 5)).unwrap())
            .operation_name(format!("operation_{}", i % 3))
            .start_time(std::time::SystemTime::now())
            .duration(Duration::from_millis(i as u64 * 10))
            .status(if i % 10 == 0 { SpanStatus::Error("test error".to_string()) } else { SpanStatus::Ok })
            .build_default();
        spans.push(span);
    }

    // Store all spans
    for span in spans {
        storage.store_span(span).await.unwrap();
    }

    // Get metrics
    let metrics = storage.get_service_metrics().await.unwrap();

    // Verify we have metrics for all services
    assert!(!metrics.is_empty());
    assert!(metrics.len() >= 5); // Should have at least 5 services

    // Check that metrics have reasonable values
    for metric in &metrics {
        assert!(metric.span_count > 0);
        assert!(metric.request_rate >= 0.0);
        assert!(metric.error_rate >= 0.0 && metric.error_rate <= 1.0);
        assert!(metric.latency_p50.as_millis() > 0);
        assert!(metric.latency_p95 >= metric.latency_p50);
        assert!(metric.latency_p99 >= metric.latency_p95);
    }

    // Test specific service metrics
    let api_gateway = metrics
        .iter()
        .find(|m| m.name.as_str() == "api-gateway")
        .expect("Should have api-gateway metrics");

    assert!(api_gateway.error_rate < 0.1); // API gateway should have low error rate

    let payment_service = metrics
        .iter()
        .find(|m| m.name.as_str() == "payment-service")
        .expect("Should have payment-service metrics");

    assert!(payment_service.error_rate > 0.0); // Payment service should have some errors
}

#[tokio::test]
async fn test_storage_limits() {
    // Create storage with small limit
    let storage = InMemoryStorage::new(10);

    // Store more spans than the limit
    for i in 0..50 {
        let span = SpanBuilder::default()
            .trace_id(TraceId::new(format!("trace_{:04}", i / 10)).unwrap())
            .span_id(SpanId::new(format!("span_{:04}", i)).unwrap())
            .service_name(ServiceName::new("test-service".to_string()).unwrap())
            .operation_name("test-op")
            .build_default();
        let _ = storage.store_span(span).await; // May fail when over limit
    }

    // Check that storage enforced limits
    let count = storage.get_span_count().await.unwrap();
    assert!(count <= 10, "Storage should enforce max span limit");
}

#[tokio::test]
async fn test_metrics_calculation_accuracy() {
    let storage = InMemoryStorage::new(10000);

    // Generate many spans to get stable metrics
    for i in 0..1000 {
        let span = SpanBuilder::default()
            .trace_id(TraceId::new(format!("trace_{:04}", i / 10)).unwrap())
            .span_id(SpanId::new(format!("span_{:04}", i)).unwrap())
            .service_name(ServiceName::new(format!("service_{}", i % 5)).unwrap())
            .operation_name(format!("operation_{}", i % 3))
            .status(if i % 10 == 0 { SpanStatus::Error("test error".to_string()) } else { SpanStatus::Ok })
            .build_default();
        storage.store_span(span).await.unwrap();
    }

    let metrics = storage.get_service_metrics().await.unwrap();

    // Verify all expected services are present
    let service_names: Vec<&str> = metrics.iter().map(|m| m.name.as_str()).collect();

    assert!(service_names.contains(&"api-gateway"));
    assert!(service_names.contains(&"user-service"));
    assert!(service_names.contains(&"order-service"));
    assert!(service_names.contains(&"payment-service"));
    assert!(service_names.contains(&"inventory-service"));

    // Check that metrics are internally consistent
    for metric in metrics {
        // Error count should not exceed span count
        assert!(metric.error_count <= metric.span_count);

        // Percentiles should be ordered correctly
        assert!(metric.latency_p50 <= metric.latency_p95);
        assert!(metric.latency_p95 <= metric.latency_p99);

        // Min/max should bound the percentiles
        assert!(metric.min_duration <= metric.latency_p50);
        assert!(metric.max_duration >= metric.latency_p99);

        // Average should be between min and max
        assert!(metric.avg_duration >= metric.min_duration);
        assert!(metric.avg_duration <= metric.max_duration);
    }
}

#[tokio::test]
async fn test_service_health_status() {
    let storage = InMemoryStorage::new(1000);

    // Generate spans with known error rates
    for i in 0..200 {
        let service_num = i % 5;
        let has_error = match service_num {
            3 => i % 5 == 0,  // payment-service: 20% error rate
            _ => i % 50 == 0, // others: 2% error rate
        };

        let span = SpanBuilder::default()
            .trace_id(TraceId::new(format!("trace_{:04}", i)).unwrap())
            .span_id(SpanId::new(format!("span_{:04}", i)).unwrap())
            .service_name(ServiceName::new(match service_num {
                0 => "api-gateway",
                1 => "user-service",
                2 => "order-service",
                3 => "payment-service",
                _ => "inventory-service",
            }.to_string()).unwrap())
            .operation_name("test-op")
            .status(if has_error { SpanStatus::Error("error".to_string()) } else { SpanStatus::Ok })
            .build_default();
        storage.store_span(span).await.unwrap();
    }

    let metrics = storage.get_service_metrics().await.unwrap();

    for metric in metrics {
        // Determine health status based on error rate
        let health_status = if metric.error_rate > 0.1 {
            "unhealthy"
        } else if metric.error_rate > 0.02 {
            "degraded"
        } else {
            "healthy"
        };

        println!(
            "Service: {} | Error Rate: {:.2}% | Status: {}",
            metric.name.as_str(),
            metric.error_rate * 100.0,
            health_status
        );

        // Verify the health determination makes sense
        match metric.name.as_str() {
            "api-gateway" | "user-service" | "inventory-service" => {
                assert!(metric.error_rate < 0.02, "{} should be healthy", metric.name.as_str());
            },
            "payment-service" => {
                // Payment service has higher error rate by design
                assert!(metric.error_rate > 0.01, "payment-service should have some errors");
            },
            _ => {},
        }
    }
}

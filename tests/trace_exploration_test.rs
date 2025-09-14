//! Integration tests for trace exploration functionality.

use std::time::{Duration, SystemTime};
use urpo_lib::core::{ServiceName, Span, SpanId, SpanStatus, TraceId};
use urpo_lib::storage::{InMemoryStorage, StorageBackend};

#[tokio::test]
async fn test_list_recent_traces() {
    let storage = InMemoryStorage::new(1000);

    // Create multiple traces with different services
    for trace_num in 0..5 {
        let trace_id = TraceId::new(format!("trace_{:04}", trace_num)).unwrap();

        // Root span
        let root_span = Span::builder()
            .trace_id(trace_id.clone())
            .span_id(SpanId::new(format!("span_root_{:04}", trace_num)).unwrap())
            .service_name(ServiceName::new("api-gateway".to_string()).unwrap())
            .operation_name("POST /api/order")
            .start_time(SystemTime::now() - Duration::from_secs(trace_num as u64 * 10))
            .duration(Duration::from_millis(100))
            .status(SpanStatus::Ok)
            .build()
            .unwrap();

        storage.store_span(root_span).await.unwrap();

        // Child spans
        for span_num in 0..3 {
            let child_span = Span::builder()
                .trace_id(trace_id.clone())
                .span_id(SpanId::new(format!("span_{:04}_{:02}", trace_num, span_num)).unwrap())
                .parent_span_id(SpanId::new(format!("span_root_{:04}", trace_num)).unwrap())
                .service_name(ServiceName::new(format!("service-{}", span_num)).unwrap())
                .operation_name(format!("operation-{}", span_num))
                .start_time(SystemTime::now() - Duration::from_secs(trace_num as u64 * 10))
                .duration(Duration::from_millis(30))
                .status(if span_num == 1 && trace_num == 2 {
                    SpanStatus::Error("Test error".to_string())
                } else {
                    SpanStatus::Ok
                })
                .build()
                .unwrap();

            storage.store_span(child_span).await.unwrap();
        }
    }

    // Test listing recent traces
    let traces = storage.list_recent_traces(10, None).await.unwrap();
    assert_eq!(traces.len(), 5, "Should have 5 traces");

    // Verify traces are sorted by start time (most recent first)
    for i in 1..traces.len() {
        assert!(
            traces[i - 1].start_time >= traces[i].start_time,
            "Traces should be sorted by start time descending"
        );
    }

    // Verify trace properties
    let first_trace = &traces[0];
    assert_eq!(first_trace.span_count, 4, "Each trace should have 4 spans");
    assert_eq!(first_trace.root_service.as_str(), "api-gateway");
    assert_eq!(first_trace.root_operation, "POST /api/order");
    assert!(
        first_trace.services.len() >= 3,
        "Should have multiple services"
    );

    // Test filtering by service
    let api_traces = storage
        .list_recent_traces(
            10,
            Some(&ServiceName::new("api-gateway".to_string()).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(api_traces.len(), 5, "All traces have api-gateway service");

    let service_1_traces = storage
        .list_recent_traces(
            10,
            Some(&ServiceName::new("service-1".to_string()).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(service_1_traces.len(), 5, "All traces have service-1");
}

#[tokio::test]
async fn test_get_error_traces() {
    let storage = InMemoryStorage::new(1000);

    // Create traces with and without errors
    for trace_num in 0..5 {
        let trace_id = TraceId::new(format!("trace_{:04}", trace_num)).unwrap();
        let has_error = trace_num % 2 == 0;

        let span = Span::builder()
            .trace_id(trace_id.clone())
            .span_id(SpanId::new(format!("span_{:04}", trace_num)).unwrap())
            .service_name(ServiceName::new("test-service".to_string()).unwrap())
            .operation_name("test-op")
            .start_time(SystemTime::now())
            .duration(Duration::from_millis(100))
            .status(if has_error {
                SpanStatus::Error(format!("Error in trace {}", trace_num))
            } else {
                SpanStatus::Ok
            })
            .build()
            .unwrap();

        storage.store_span(span).await.unwrap();
    }

    // Get error traces
    let error_traces = storage.get_error_traces(10).await.unwrap();
    assert_eq!(error_traces.len(), 3, "Should have 3 error traces");

    // Verify all returned traces have errors
    for trace in error_traces {
        assert!(trace.has_error, "All returned traces should have errors");
    }
}

#[tokio::test]
async fn test_get_slow_traces() {
    let storage = InMemoryStorage::new(1000);

    // Create traces with varying durations
    let durations = [50, 150, 300, 600, 1000]; // milliseconds

    for (i, &duration_ms) in durations.iter().enumerate() {
        let trace_id = TraceId::new(format!("trace_{:04}", i)).unwrap();

        let span = Span::builder()
            .trace_id(trace_id.clone())
            .span_id(SpanId::new(format!("span_{:04}", i)).unwrap())
            .service_name(ServiceName::new("test-service".to_string()).unwrap())
            .operation_name("test-op")
            .start_time(SystemTime::now())
            .duration(Duration::from_millis(duration_ms))
            .status(SpanStatus::Ok)
            .build()
            .unwrap();

        storage.store_span(span).await.unwrap();
    }

    // Get slow traces (> 500ms)
    let slow_traces = storage
        .get_slow_traces(Duration::from_millis(500), 10)
        .await
        .unwrap();
    assert_eq!(slow_traces.len(), 2, "Should have 2 slow traces");

    // Verify all returned traces are slow
    for trace in slow_traces {
        assert!(
            trace.duration >= Duration::from_millis(500),
            "All returned traces should be slower than threshold"
        );
    }
}

#[tokio::test]
async fn test_search_traces() {
    let storage = InMemoryStorage::new(1000);

    // Create traces with different operations and attributes
    let operations = [
        "GET /users",
        "POST /orders",
        "GET /products",
        "DELETE /sessions",
    ];

    for (i, &op) in operations.iter().enumerate() {
        let trace_id = TraceId::new(format!("trace_{:04}", i)).unwrap();

        let mut span_builder = Span::builder()
            .trace_id(trace_id.clone())
            .span_id(SpanId::new(format!("span_{:04}", i)).unwrap())
            .service_name(ServiceName::new("test-service".to_string()).unwrap())
            .operation_name(op)
            .start_time(SystemTime::now())
            .duration(Duration::from_millis(100))
            .status(SpanStatus::Ok);

        // Add searchable attributes
        if op.contains("users") {
            span_builder = span_builder.attribute("user.id", "12345");
        }
        if op.contains("orders") {
            span_builder = span_builder.attribute("order.id", "ORD-789");
            span_builder = span_builder.tag("priority", "high");
        }

        storage
            .store_span(span_builder.build().unwrap())
            .await
            .unwrap();
    }

    // Search by operation name
    let user_traces = storage.search_traces("users", 10).await.unwrap();
    assert_eq!(
        user_traces.len(),
        1,
        "Should find 1 trace with 'users' in operation"
    );

    // Search by attribute value
    let order_traces = storage.search_traces("ORD-789", 10).await.unwrap();
    assert_eq!(order_traces.len(), 1, "Should find 1 trace with order ID");

    // Search by tag
    let priority_traces = storage.search_traces("high", 10).await.unwrap();
    assert_eq!(
        priority_traces.len(),
        1,
        "Should find 1 trace with high priority tag"
    );

    // Search with no matches
    let no_matches = storage.search_traces("nonexistent", 10).await.unwrap();
    assert_eq!(no_matches.len(), 0, "Should find no traces");
}

#[tokio::test]
async fn test_span_hierarchy() {
    let storage = InMemoryStorage::new(1000);
    let trace_id = TraceId::new("test_trace".to_string()).unwrap();

    // Create a multi-level span hierarchy
    // Root
    let root = Span::builder()
        .trace_id(trace_id.clone())
        .span_id(SpanId::new("root".to_string()).unwrap())
        .service_name(ServiceName::new("gateway".to_string()).unwrap())
        .operation_name("handleRequest")
        .start_time(SystemTime::now())
        .duration(Duration::from_millis(100))
        .status(SpanStatus::Ok)
        .build()
        .unwrap();

    storage.store_span(root).await.unwrap();

    // Level 1 children
    for i in 0..2 {
        let child = Span::builder()
            .trace_id(trace_id.clone())
            .span_id(SpanId::new(format!("child_{}", i)).unwrap())
            .parent_span_id(SpanId::new("root".to_string()).unwrap())
            .service_name(ServiceName::new(format!("service_{}", i)).unwrap())
            .operation_name(format!("operation_{}", i))
            .start_time(SystemTime::now())
            .duration(Duration::from_millis(40))
            .status(SpanStatus::Ok)
            .build()
            .unwrap();

        storage.store_span(child).await.unwrap();

        // Level 2 grandchildren
        for j in 0..2 {
            let grandchild = Span::builder()
                .trace_id(trace_id.clone())
                .span_id(SpanId::new(format!("grandchild_{}_{}", i, j)).unwrap())
                .parent_span_id(SpanId::new(format!("child_{}", i)).unwrap())
                .service_name(ServiceName::new(format!("subservice_{}_{}", i, j)).unwrap())
                .operation_name(format!("subop_{}_{}", i, j))
                .start_time(SystemTime::now())
                .duration(Duration::from_millis(15))
                .status(SpanStatus::Ok)
                .build()
                .unwrap();

            storage.store_span(grandchild).await.unwrap();
        }
    }

    // Get all spans for the trace
    let spans = storage.get_trace_spans(&trace_id).await.unwrap();
    assert_eq!(
        spans.len(),
        7,
        "Should have 7 spans total (1 root + 2 children + 4 grandchildren)"
    );

    // Verify hierarchy
    let root_spans: Vec<_> = spans
        .iter()
        .filter(|s| s.parent_span_id.is_none())
        .collect();
    assert_eq!(root_spans.len(), 1, "Should have exactly 1 root span");

    let child_spans: Vec<_> = spans
        .iter()
        .filter(|s| s.parent_span_id.as_ref().map(|p| p.as_str()) == Some("root"))
        .collect();
    assert_eq!(
        child_spans.len(),
        2,
        "Should have 2 direct children of root"
    );

    for child in &child_spans {
        let grandchildren: Vec<_> = spans
            .iter()
            .filter(|s| s.parent_span_id.as_ref() == Some(&child.span_id))
            .collect();
        assert_eq!(
            grandchildren.len(),
            2,
            "Each child should have 2 grandchildren"
        );
    }
}

#[tokio::test]
async fn test_trace_aggregation_and_statistics() {
    let storage = InMemoryStorage::new(1000);

    // Create multiple traces with different characteristics
    for trace_num in 0..10 {
        let trace_id = TraceId::new(format!("trace_{:04}", trace_num)).unwrap();
        let base_time = SystemTime::now() - Duration::from_secs(trace_num as u64 * 60);

        // Create spans with varying durations and error rates
        for span_num in 0..5 {
            let span = Span::builder()
                .trace_id(trace_id.clone())
                .span_id(SpanId::new(format!("span_{:04}_{:02}", trace_num, span_num)).unwrap())
                .service_name(ServiceName::new(format!("service-{}", span_num % 3)).unwrap())
                .operation_name(format!("op-{}", span_num))
                .start_time(base_time + Duration::from_millis(span_num as u64 * 10))
                .duration(Duration::from_millis(
                    (trace_num + 1) as u64 * 50 + span_num as u64 * 10,
                ))
                .status(if trace_num == 3 && span_num == 2 {
                    SpanStatus::Error("Simulated error".to_string())
                } else {
                    SpanStatus::Ok
                })
                .attribute("trace.num", &trace_num.to_string())
                .attribute("span.num", &span_num.to_string())
                .build()
                .unwrap();

            storage.store_span(span).await.unwrap();
        }
    }

    // Test trace count aggregation
    let all_traces = storage.list_recent_traces(100, None).await.unwrap();
    assert_eq!(all_traces.len(), 10, "Should have 10 traces");

    // Test service filtering
    let service_0_traces = storage
        .list_recent_traces(
            100,
            Some(&ServiceName::new("service-0".to_string()).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(
        service_0_traces.len(),
        10,
        "All traces should have service-0"
    );

    // Test trace statistics
    for trace in &all_traces {
        assert_eq!(trace.span_count, 5, "Each trace should have 5 spans");
        assert_eq!(
            trace.services.len(),
            3,
            "Each trace should touch 3 services"
        );
    }

    // Test error trace filtering
    let error_traces = storage.get_error_traces(100).await.unwrap();
    assert_eq!(error_traces.len(), 1, "Should have 1 error trace");
    assert_eq!(error_traces[0].trace_id.as_str(), "trace_0003");

    // Test duration statistics
    let slow_threshold = Duration::from_millis(250);
    let slow_traces = storage.get_slow_traces(slow_threshold, 100).await.unwrap();
    assert!(slow_traces.len() >= 5, "Should have multiple slow traces");

    // Verify all slow traces exceed threshold
    for trace in &slow_traces {
        assert!(
            trace.duration >= slow_threshold,
            "Trace {:?} duration {:?} should exceed threshold {:?}",
            trace.trace_id,
            trace.duration,
            slow_threshold
        );
    }
}

#[tokio::test]
async fn test_memory_pressure_and_cleanup() {
    use urpo_lib::storage::CleanupConfig;

    // Create storage with small capacity to trigger cleanup
    let small_storage = InMemoryStorage::with_cleanup_config(
        50, // Small max_spans to trigger cleanup
        CleanupConfig {
            max_memory_bytes: 1024 * 1024, // 1MB
            warning_threshold: 0.7,
            critical_threshold: 0.85,
            emergency_threshold: 0.95,
            retention_period: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(1),
            min_spans_per_service: 10,
        },
    );

    // Fill storage beyond capacity
    for i in 0..100 {
        let trace_id = TraceId::new(format!("trace_{:04}", i)).unwrap();
        let span = Span::builder()
            .trace_id(trace_id.clone())
            .span_id(SpanId::new(format!("span_{:04}", i)).unwrap())
            .service_name(ServiceName::new("test-service".to_string()).unwrap())
            .operation_name("test-op")
            .start_time(SystemTime::now() - Duration::from_secs(i as u64))
            .duration(Duration::from_millis(100))
            .status(SpanStatus::Ok)
            .build()
            .unwrap();

        let _ = small_storage.store_span(span).await; // May fail at high capacity
    }

    // Verify storage respects max capacity
    let span_count = small_storage.get_span_count().await.unwrap();
    assert!(
        span_count <= 50,
        "Storage should not exceed max capacity: {} > 50",
        span_count
    );

    // Test memory pressure calculation
    let memory_pressure = small_storage.get_memory_pressure();
    assert!(
        memory_pressure > 0.0 && memory_pressure <= 1.0,
        "Memory pressure should be between 0 and 1: {}",
        memory_pressure
    );

    // Test cleanup enforcement
    let evicted = small_storage.enforce_limits().await.unwrap();
    let final_count = small_storage.get_span_count().await.unwrap();
    assert!(
        final_count <= 50,
        "After cleanup, storage should be within limits"
    );

    // Test that oldest spans were evicted (newer spans remain)
    let recent_traces = small_storage.list_recent_traces(10, None).await.unwrap();
    assert!(
        !recent_traces.is_empty(),
        "Should have recent traces after cleanup"
    );

    // Verify newer traces are retained
    for trace in recent_traces {
        let trace_num: usize = trace
            .trace_id
            .as_str()
            .strip_prefix("trace_")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        assert!(
            trace_num >= 50,
            "Should retain newer traces (num >= 50): {}",
            trace_num
        );
    }

    // Test emergency cleanup
    let emergency_evicted = small_storage.emergency_cleanup().await.unwrap();
    let after_emergency = small_storage.get_span_count().await.unwrap();
    assert!(
        after_emergency < 50,
        "Emergency cleanup should remove significant spans"
    );
}

#[tokio::test]
async fn test_service_metrics_calculation() {
    let storage = InMemoryStorage::new(1000);

    // Create spans with known metrics for multiple services
    let services = vec!["auth-service", "api-gateway", "database"];
    let base_time = SystemTime::now() - Duration::from_secs(60);

    for service_name in &services {
        let service = ServiceName::new(service_name.to_string()).unwrap();

        // Create spans with varying characteristics
        for i in 0..20 {
            let trace_id = TraceId::new(format!("trace_{}_{}", service_name, i)).unwrap();

            // Vary duration: some fast, some slow
            let duration = if i < 10 {
                Duration::from_millis(50 + i as u64 * 10) // Fast spans: 50-140ms
            } else {
                Duration::from_millis(500 + i as u64 * 50) // Slow spans: 500-950ms
            };

            // Add some errors for testing error rate
            let status = if service_name == &"database" && i % 5 == 0 {
                SpanStatus::Error(format!("Database error {}", i))
            } else if service_name == &"auth-service" && i == 15 {
                SpanStatus::Error("Authentication failed".to_string())
            } else {
                SpanStatus::Ok
            };

            let span = Span::builder()
                .trace_id(trace_id.clone())
                .span_id(SpanId::new(format!("span_{}_{}", service_name, i)).unwrap())
                .service_name(service.clone())
                .operation_name(format!("operation_{}", i % 3))
                .start_time(base_time + Duration::from_secs(i as u64))
                .duration(duration)
                .status(status)
                .build()
                .unwrap();

            storage.store_span(span).await.unwrap();
        }
    }

    // Get service metrics
    let metrics = storage.get_service_metrics().await.unwrap();
    assert_eq!(metrics.len(), 3, "Should have metrics for 3 services");

    // Find metrics for each service
    let auth_metrics = metrics
        .iter()
        .find(|m| m.name.as_str() == "auth-service")
        .expect("Should have auth-service metrics");

    let api_metrics = metrics
        .iter()
        .find(|m| m.name.as_str() == "api-gateway")
        .expect("Should have api-gateway metrics");

    let db_metrics = metrics
        .iter()
        .find(|m| m.name.as_str() == "database")
        .expect("Should have database metrics");

    // Verify request rates
    assert!(
        auth_metrics.request_rate > 0.0,
        "Auth service should have requests"
    );
    assert!(
        api_metrics.request_rate > 0.0,
        "API gateway should have requests"
    );
    assert!(
        db_metrics.request_rate > 0.0,
        "Database should have requests"
    );

    // Verify error rates
    assert!(
        auth_metrics.error_rate > 0.0,
        "Auth service should have some errors"
    );
    assert_eq!(
        api_metrics.error_rate, 0.0,
        "API gateway error rate should be 0%"
    );
    assert!(
        db_metrics.error_rate > 0.0,
        "Database should have some errors"
    );

    // Verify latency statistics exist and are reasonable
    for metrics in &[auth_metrics, api_metrics, db_metrics] {
        assert!(
            metrics.latency_p50 > Duration::from_millis(0),
            "P50 latency should be positive"
        );
        assert!(
            metrics.latency_p95 >= metrics.latency_p50,
            "P95 should be >= P50"
        );
        assert!(
            metrics.latency_p99 >= metrics.latency_p95,
            "P99 should be >= P95"
        );
    }
}

#[tokio::test]
async fn test_concurrent_access_and_thread_safety() {
    use std::sync::Arc;
    use tokio::task;

    let storage = Arc::new(InMemoryStorage::new(10000));
    let num_writers = 10;
    let spans_per_writer = 100;

    // Spawn multiple concurrent writers
    let mut write_handles = Vec::new();
    for writer_id in 0..num_writers {
        let storage_clone = Arc::clone(&storage);
        let handle = task::spawn(async move {
            for span_id in 0..spans_per_writer {
                let trace_id = TraceId::new(format!("trace_w{}_s{}", writer_id, span_id)).unwrap();
                let span = Span::builder()
                    .trace_id(trace_id.clone())
                    .span_id(SpanId::new(format!("span_w{}_s{}", writer_id, span_id)).unwrap())
                    .service_name(ServiceName::new(format!("service-{}", writer_id % 3)).unwrap())
                    .operation_name(format!("op-{}", span_id % 5))
                    .start_time(SystemTime::now())
                    .duration(Duration::from_millis(10 + span_id as u64))
                    .status(if span_id % 20 == 0 {
                        SpanStatus::Error("Concurrent error".to_string())
                    } else {
                        SpanStatus::Ok
                    })
                    .build()
                    .unwrap();

                storage_clone.store_span(span).await.unwrap();

                // Small delay to increase contention
                if span_id % 10 == 0 {
                    tokio::time::sleep(Duration::from_micros(10)).await;
                }
            }
        });
        write_handles.push(handle);
    }

    // Spawn concurrent readers
    let mut read_handles = Vec::new();
    for reader_id in 0..5 {
        let storage_clone = Arc::clone(&storage);
        let handle = task::spawn(async move {
            let mut successful_reads = 0;
            for _ in 0..50 {
                // Try various read operations
                let traces = storage_clone.list_recent_traces(10, None).await.unwrap();
                if !traces.is_empty() {
                    successful_reads += 1;
                }

                let services = storage_clone.list_services().await.unwrap();
                if !services.is_empty() {
                    successful_reads += 1;
                }

                // Try to get a specific span
                let test_span_id = SpanId::new(format!("span_w0_s{}", reader_id)).unwrap();
                if let Ok(Some(_)) = storage_clone.get_span(&test_span_id).await {
                    successful_reads += 1;
                }

                tokio::time::sleep(Duration::from_micros(50)).await;
            }
            successful_reads
        });
        read_handles.push(handle);
    }

    // Wait for all writers to complete
    for handle in write_handles {
        handle.await.unwrap();
    }

    // Wait for all readers to complete
    let mut total_reads = 0;
    for handle in read_handles {
        total_reads += handle.await.unwrap();
    }

    // Verify data integrity after concurrent access
    let final_count = storage.get_span_count().await.unwrap();
    assert_eq!(
        final_count,
        num_writers * spans_per_writer,
        "Should have all spans: expected {}, got {}",
        num_writers * spans_per_writer,
        final_count
    );

    // Verify readers had successful reads
    assert!(
        total_reads > 0,
        "Readers should have successful reads during concurrent access"
    );

    // Verify service metrics are consistent
    let metrics = storage.get_service_metrics().await.unwrap();
    assert_eq!(metrics.len(), 3, "Should have 3 services");

    // Verify all services have metrics
    assert_eq!(metrics.len(), 3, "Should have metrics for all 3 services");
    for metric in &metrics {
        assert!(
            metric.request_rate > 0.0,
            "All services should have request rates"
        );
    }

    // Test concurrent trace queries
    let mut query_handles = Vec::new();
    for _ in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let handle = task::spawn(async move {
            let error_traces = storage_clone.get_error_traces(100).await.unwrap();
            let slow_traces = storage_clone
                .get_slow_traces(Duration::from_millis(50), 100)
                .await
                .unwrap();
            let search_results = storage_clone.search_traces("op-1", 100).await.unwrap();
            (error_traces.len(), slow_traces.len(), search_results.len())
        });
        query_handles.push(handle);
    }

    // Verify all queries return consistent results
    let mut query_results = Vec::new();
    for handle in query_handles {
        query_results.push(handle.await.unwrap());
    }

    // All queries should return the same results
    let first_result = query_results[0];
    for result in &query_results[1..] {
        assert_eq!(
            result.0, first_result.0,
            "Error trace counts should be consistent"
        );
        assert_eq!(
            result.1, first_result.1,
            "Slow trace counts should be consistent"
        );
        assert_eq!(
            result.2, first_result.2,
            "Search results should be consistent"
        );
    }
}

#[tokio::test]
async fn test_span_attributes_and_tags_filtering() {
    let storage = InMemoryStorage::new(1000);

    // Create spans with various attributes and tags
    let test_data = vec![
        (
            "user-service",
            vec![("user.id", "12345"), ("user.email", "test@example.com")],
            vec!["production", "critical"],
        ),
        (
            "payment-service",
            vec![("payment.amount", "99.99"), ("payment.currency", "USD")],
            vec!["production", "financial"],
        ),
        (
            "notification-service",
            vec![
                ("notification.type", "email"),
                ("notification.recipient", "admin"),
            ],
            vec!["staging", "low-priority"],
        ),
        (
            "analytics-service",
            vec![("event.type", "page_view"), ("event.source", "mobile")],
            vec!["production", "analytics"],
        ),
        (
            "auth-service",
            vec![("auth.method", "oauth"), ("auth.provider", "google")],
            vec!["production", "security"],
        ),
    ];

    for (service_name, attributes, tags) in test_data {
        for i in 0..5 {
            let trace_id = TraceId::new(format!("trace_{}_{}", service_name, i)).unwrap();
            let mut span_builder = Span::builder()
                .trace_id(trace_id.clone())
                .span_id(SpanId::new(format!("span_{}_{}", service_name, i)).unwrap())
                .service_name(ServiceName::new(service_name.to_string()).unwrap())
                .operation_name(format!("{}_operation", service_name))
                .start_time(SystemTime::now())
                .duration(Duration::from_millis(100))
                .status(SpanStatus::Ok);

            // Add attributes
            for (key, value) in &attributes {
                span_builder = span_builder.attribute(*key, *value);
            }

            // Add tags
            for tag in &tags {
                span_builder = span_builder.tag(*tag, *tag);
            }

            // Add some conditional attributes
            if i % 2 == 0 {
                span_builder = span_builder.attribute("request.id", &format!("req_{}", i));
            }
            if i % 3 == 0 {
                span_builder = span_builder.tag("slow", "true");
            }

            storage
                .store_span(span_builder.build().unwrap())
                .await
                .unwrap();
        }
    }

    // Test searching by attribute values
    let user_search = storage.search_traces("12345", 10).await.unwrap();
    assert_eq!(
        user_search.len(),
        5,
        "Should find all traces with user.id=12345"
    );

    let email_search = storage.search_traces("test@example.com", 10).await.unwrap();
    assert_eq!(
        email_search.len(),
        5,
        "Should find all traces with the email"
    );

    let payment_search = storage.search_traces("99.99", 10).await.unwrap();
    assert_eq!(payment_search.len(), 5, "Should find all payment traces");

    let currency_search = storage.search_traces("USD", 10).await.unwrap();
    assert_eq!(
        currency_search.len(),
        5,
        "Should find all USD payment traces"
    );

    // Test searching by tags
    let production_search = storage.search_traces("production", 100).await.unwrap();
    assert!(
        production_search.len() >= 15,
        "Should find multiple production traces"
    );

    let staging_search = storage.search_traces("staging", 10).await.unwrap();
    assert_eq!(staging_search.len(), 5, "Should find staging traces");

    let critical_search = storage.search_traces("critical", 10).await.unwrap();
    assert_eq!(critical_search.len(), 5, "Should find critical traces");

    let financial_search = storage.search_traces("financial", 10).await.unwrap();
    assert_eq!(financial_search.len(), 5, "Should find financial traces");

    // Test searching by operation names
    let user_op_search = storage
        .search_traces("user-service_operation", 10)
        .await
        .unwrap();
    assert_eq!(
        user_op_search.len(),
        5,
        "Should find user service operations"
    );

    // Test searching by conditional attributes
    let request_search = storage.search_traces("req_", 100).await.unwrap();
    assert!(
        request_search.len() > 0,
        "Should find traces with request IDs"
    );

    let slow_tag_search = storage.search_traces("slow", 100).await.unwrap();
    assert!(
        slow_tag_search.len() > 0,
        "Should find traces tagged as slow"
    );

    // Test that search is case-sensitive for exact matches
    let no_match_search = storage.search_traces("PRODUCTION", 10).await.unwrap();
    assert_eq!(
        no_match_search.len(),
        0,
        "Should not find uppercase when lowercase was stored"
    );

    // Test searching for non-existent values
    let not_found = storage
        .search_traces("nonexistent_value", 10)
        .await
        .unwrap();
    assert_eq!(
        not_found.len(),
        0,
        "Should return empty for non-existent values"
    );

    // Verify span details include attributes and tags
    let trace_id = TraceId::new("trace_user-service_0".to_string()).unwrap();
    let spans = storage.get_trace_spans(&trace_id).await.unwrap();
    assert!(!spans.is_empty(), "Should have spans for the trace");

    let first_span = &spans[0];
    assert!(
        first_span.attributes.contains_key("user.id"),
        "Span should have user.id attribute"
    );
    assert!(
        first_span.attributes.contains_key("user.email"),
        "Span should have user.email attribute"
    );
    assert!(
        first_span.tags.contains_key("production"),
        "Span should have production tag"
    );
    assert!(
        first_span.tags.contains_key("critical"),
        "Span should have critical tag"
    );
}

#[tokio::test]
async fn test_trace_waterfall_visualization_data() {
    let storage = InMemoryStorage::new(1000);
    let base_time = SystemTime::now();
    let trace_id = TraceId::new("waterfall_trace".to_string()).unwrap();

    // Create a realistic trace with overlapping and sequential spans
    // Simulating: API Gateway -> (Auth Service || User Service) -> Database

    // Root span: API Gateway
    let gateway_span = Span::builder()
        .trace_id(trace_id.clone())
        .span_id(SpanId::new("gateway_root".to_string()).unwrap())
        .service_name(ServiceName::new("api-gateway".to_string()).unwrap())
        .operation_name("POST /api/user/profile")
        .start_time(base_time)
        .duration(Duration::from_millis(250))
        .status(SpanStatus::Ok)
        .attribute("http.method", "POST")
        .attribute("http.path", "/api/user/profile")
        .build()
        .unwrap();

    storage.store_span(gateway_span).await.unwrap();

    // Auth Service span (parallel with User Service)
    let auth_span = Span::builder()
        .trace_id(trace_id.clone())
        .span_id(SpanId::new("auth_check".to_string()).unwrap())
        .parent_span_id(SpanId::new("gateway_root".to_string()).unwrap())
        .service_name(ServiceName::new("auth-service".to_string()).unwrap())
        .operation_name("validateToken")
        .start_time(base_time + Duration::from_millis(10))
        .duration(Duration::from_millis(50))
        .status(SpanStatus::Ok)
        .attribute("auth.method", "JWT")
        .build()
        .unwrap();

    storage.store_span(auth_span).await.unwrap();

    // User Service span (parallel with Auth Service)
    let user_span = Span::builder()
        .trace_id(trace_id.clone())
        .span_id(SpanId::new("user_fetch".to_string()).unwrap())
        .parent_span_id(SpanId::new("gateway_root".to_string()).unwrap())
        .service_name(ServiceName::new("user-service".to_string()).unwrap())
        .operation_name("getUserProfile")
        .start_time(base_time + Duration::from_millis(15))
        .duration(Duration::from_millis(180))
        .status(SpanStatus::Ok)
        .attribute("user.id", "user123")
        .build()
        .unwrap();

    storage.store_span(user_span).await.unwrap();

    // Database span (child of User Service)
    let db_span = Span::builder()
        .trace_id(trace_id.clone())
        .span_id(SpanId::new("db_query".to_string()).unwrap())
        .parent_span_id(SpanId::new("user_fetch".to_string()).unwrap())
        .service_name(ServiceName::new("database".to_string()).unwrap())
        .operation_name("SELECT * FROM users")
        .start_time(base_time + Duration::from_millis(30))
        .duration(Duration::from_millis(120))
        .status(SpanStatus::Ok)
        .attribute("db.type", "postgresql")
        .attribute("db.statement", "SELECT * FROM users WHERE id = ?")
        .build()
        .unwrap();

    storage.store_span(db_span).await.unwrap();

    // Cache check span (another child of User Service, after DB)
    let cache_span = Span::builder()
        .trace_id(trace_id.clone())
        .span_id(SpanId::new("cache_update".to_string()).unwrap())
        .parent_span_id(SpanId::new("user_fetch".to_string()).unwrap())
        .service_name(ServiceName::new("cache".to_string()).unwrap())
        .operation_name("updateCache")
        .start_time(base_time + Duration::from_millis(155))
        .duration(Duration::from_millis(20))
        .status(SpanStatus::Ok)
        .attribute("cache.type", "redis")
        .build()
        .unwrap();

    storage.store_span(cache_span).await.unwrap();

    // Notification span (parallel, starts later)
    let notification_span = Span::builder()
        .trace_id(trace_id.clone())
        .span_id(SpanId::new("send_notification".to_string()).unwrap())
        .parent_span_id(SpanId::new("gateway_root".to_string()).unwrap())
        .service_name(ServiceName::new("notification-service".to_string()).unwrap())
        .operation_name("sendEmail")
        .start_time(base_time + Duration::from_millis(200))
        .duration(Duration::from_millis(40))
        .status(SpanStatus::Ok)
        .attribute("notification.type", "email")
        .build()
        .unwrap();

    storage.store_span(notification_span).await.unwrap();

    // Get all spans for waterfall visualization
    let spans = storage.get_trace_spans(&trace_id).await.unwrap();
    assert_eq!(spans.len(), 6, "Should have 6 spans in the trace");

    // Verify spans are sorted by start time (important for waterfall)
    for i in 1..spans.len() {
        assert!(
            spans[i].start_time >= spans[i - 1].start_time,
            "Spans should be sorted by start time for waterfall visualization"
        );
    }

    // Build parent-child relationships for waterfall
    let mut children_map: std::collections::HashMap<Option<SpanId>, Vec<&Span>> =
        std::collections::HashMap::new();
    for span in &spans {
        children_map
            .entry(span.parent_span_id.clone())
            .or_insert_with(Vec::new)
            .push(span);
    }

    // Verify root span
    let root_spans = children_map.get(&None).unwrap();
    assert_eq!(root_spans.len(), 1, "Should have exactly one root span");
    assert_eq!(root_spans[0].span_id.as_str(), "gateway_root");

    // Verify gateway children (should have auth, user, and notification)
    let gateway_children = children_map
        .get(&Some(SpanId::new("gateway_root".to_string()).unwrap()))
        .unwrap();
    assert_eq!(
        gateway_children.len(),
        3,
        "Gateway should have 3 child spans"
    );

    // Verify user service children (should have db and cache)
    let user_children = children_map
        .get(&Some(SpanId::new("user_fetch".to_string()).unwrap()))
        .unwrap();
    assert_eq!(
        user_children.len(),
        2,
        "User service should have 2 child spans"
    );

    // Calculate waterfall metrics
    let trace_start = spans.iter().map(|s| s.start_time).min().unwrap();
    let trace_end = spans
        .iter()
        .map(|s| s.start_time + s.duration)
        .max()
        .unwrap();
    let total_duration = trace_end.duration_since(trace_start).unwrap();

    assert_eq!(
        total_duration,
        Duration::from_millis(250),
        "Total trace duration should be 250ms"
    );

    // Verify parallel execution detection
    let auth_start = base_time + Duration::from_millis(10);
    let auth_end = auth_start + Duration::from_millis(50);
    let user_start = base_time + Duration::from_millis(15);

    assert!(
        user_start < auth_end && user_start > auth_start,
        "Auth and User services should execute in parallel"
    );

    // Verify sequential execution detection
    let db_end = base_time + Duration::from_millis(30) + Duration::from_millis(120);
    let cache_start = base_time + Duration::from_millis(155);

    assert!(
        cache_start > db_end,
        "Cache update should start after database query completes"
    );

    // Test trace info for waterfall header
    let trace_info = storage.list_recent_traces(1, None).await.unwrap();
    assert_eq!(trace_info.len(), 1);

    let info = &trace_info[0];
    assert_eq!(info.trace_id.as_str(), "waterfall_trace");
    assert_eq!(info.span_count, 6);
    assert_eq!(info.duration, Duration::from_millis(250));
    assert_eq!(info.root_service.as_str(), "api-gateway");
    assert_eq!(info.root_operation, "POST /api/user/profile");
    assert_eq!(info.services.len(), 6, "Should have 6 unique services");
}

#[tokio::test]
async fn test_edge_cases_and_error_conditions() {
    let storage = InMemoryStorage::new(1000);

    // Test 1: Empty storage queries
    let empty_traces = storage.list_recent_traces(10, None).await.unwrap();
    assert_eq!(
        empty_traces.len(),
        0,
        "Empty storage should return no traces"
    );

    let empty_error_traces = storage.get_error_traces(10).await.unwrap();
    assert_eq!(
        empty_error_traces.len(),
        0,
        "Empty storage should return no error traces"
    );

    let empty_slow_traces = storage
        .get_slow_traces(Duration::from_millis(100), 10)
        .await
        .unwrap();
    assert_eq!(
        empty_slow_traces.len(),
        0,
        "Empty storage should return no slow traces"
    );

    let empty_search = storage.search_traces("anything", 10).await.unwrap();
    assert_eq!(
        empty_search.len(),
        0,
        "Empty storage search should return nothing"
    );

    let empty_metrics = storage.get_service_metrics().await.unwrap();
    assert_eq!(
        empty_metrics.len(),
        0,
        "Empty storage should have no service metrics"
    );

    // Test 2: Trace with only root span (no children)
    let single_span_trace = TraceId::new("single_span_trace".to_string()).unwrap();
    let single_span = Span::builder()
        .trace_id(single_span_trace.clone())
        .span_id(SpanId::new("single_span".to_string()).unwrap())
        .service_name(ServiceName::new("single-service".to_string()).unwrap())
        .operation_name("singleOp")
        .start_time(SystemTime::now())
        .duration(Duration::from_millis(10))
        .status(SpanStatus::Ok)
        .build()
        .unwrap();

    storage.store_span(single_span).await.unwrap();

    let single_trace_info = storage.list_recent_traces(10, None).await.unwrap();
    assert_eq!(single_trace_info.len(), 1);
    assert_eq!(
        single_trace_info[0].span_count, 1,
        "Single span trace should have span_count of 1"
    );

    // Test 3: Orphaned spans (parent doesn't exist)
    let orphan_trace = TraceId::new("orphan_trace".to_string()).unwrap();
    let orphan_span = Span::builder()
        .trace_id(orphan_trace.clone())
        .span_id(SpanId::new("orphan_span".to_string()).unwrap())
        .parent_span_id(SpanId::new("non_existent_parent".to_string()).unwrap())
        .service_name(ServiceName::new("orphan-service".to_string()).unwrap())
        .operation_name("orphanOp")
        .start_time(SystemTime::now())
        .duration(Duration::from_millis(20))
        .status(SpanStatus::Ok)
        .build()
        .unwrap();

    storage.store_span(orphan_span).await.unwrap();

    let orphan_spans = storage.get_trace_spans(&orphan_trace).await.unwrap();
    assert_eq!(orphan_spans.len(), 1, "Should store orphaned span");
    assert!(
        orphan_spans[0].parent_span_id.is_some(),
        "Orphan span should have parent_span_id set"
    );

    // Test 4: Very long operation names and attribute values
    let long_trace = TraceId::new("long_trace".to_string()).unwrap();
    let very_long_string = "a".repeat(10000);
    let long_span = Span::builder()
        .trace_id(long_trace.clone())
        .span_id(SpanId::new("long_span".to_string()).unwrap())
        .service_name(ServiceName::new("long-service".to_string()).unwrap())
        .operation_name(&very_long_string[..1000]) // Use first 1000 chars
        .start_time(SystemTime::now())
        .duration(Duration::from_millis(30))
        .status(SpanStatus::Ok)
        .attribute("long_attr", &very_long_string[..5000])
        .build()
        .unwrap();

    storage.store_span(long_span).await.unwrap();

    let long_trace_spans = storage.get_trace_spans(&long_trace).await.unwrap();
    assert_eq!(
        long_trace_spans.len(),
        1,
        "Should store span with long strings"
    );
    assert!(
        long_trace_spans[0].operation_name.len() >= 1000,
        "Should preserve long operation name"
    );

    // Test 5: Spans with zero duration
    let zero_duration_trace = TraceId::new("zero_duration".to_string()).unwrap();
    let zero_span = Span::builder()
        .trace_id(zero_duration_trace.clone())
        .span_id(SpanId::new("zero_span".to_string()).unwrap())
        .service_name(ServiceName::new("zero-service".to_string()).unwrap())
        .operation_name("instantOp")
        .start_time(SystemTime::now())
        .duration(Duration::from_millis(0))
        .status(SpanStatus::Ok)
        .build()
        .unwrap();

    storage.store_span(zero_span).await.unwrap();

    let zero_duration_spans = storage.get_trace_spans(&zero_duration_trace).await.unwrap();
    assert_eq!(
        zero_duration_spans.len(),
        1,
        "Should store zero-duration span"
    );
    assert_eq!(zero_duration_spans[0].duration, Duration::from_millis(0));

    // Test 6: Duplicate span IDs (should replace)
    let duplicate_trace = TraceId::new("duplicate_trace".to_string()).unwrap();
    let span_id = SpanId::new("duplicate_span_id".to_string()).unwrap();

    let first_span = Span::builder()
        .trace_id(duplicate_trace.clone())
        .span_id(span_id.clone())
        .service_name(ServiceName::new("first-service".to_string()).unwrap())
        .operation_name("firstOp")
        .start_time(SystemTime::now())
        .duration(Duration::from_millis(40))
        .status(SpanStatus::Ok)
        .build()
        .unwrap();

    storage.store_span(first_span).await.unwrap();

    let second_span = Span::builder()
        .trace_id(duplicate_trace.clone())
        .span_id(span_id.clone())
        .service_name(ServiceName::new("second-service".to_string()).unwrap())
        .operation_name("secondOp")
        .start_time(SystemTime::now())
        .duration(Duration::from_millis(50))
        .status(SpanStatus::Error("Replaced span".to_string()))
        .build()
        .unwrap();

    storage.store_span(second_span).await.unwrap();

    let duplicate_spans = storage.get_trace_spans(&duplicate_trace).await.unwrap();
    assert_eq!(
        duplicate_spans.len(),
        1,
        "Duplicate span ID should replace existing"
    );
    assert_eq!(
        duplicate_spans[0].service_name.as_str(),
        "second-service",
        "Should have the second span's data"
    );
    assert!(
        duplicate_spans[0].status.is_error(),
        "Should have the second span's error status"
    );

    // Test 7: Non-existent trace/span queries
    let non_existent_trace = TraceId::new("non_existent".to_string()).unwrap();
    let non_existent_spans = storage.get_trace_spans(&non_existent_trace).await.unwrap();
    assert_eq!(
        non_existent_spans.len(),
        0,
        "Non-existent trace should return empty spans"
    );

    let non_existent_span_id = SpanId::new("non_existent_span".to_string()).unwrap();
    let non_existent_span = storage.get_span(&non_existent_span_id).await.unwrap();
    assert!(
        non_existent_span.is_none(),
        "Non-existent span should return None"
    );

    // Test 8: Service filter with non-existent service
    let non_existent_service = ServiceName::new("non-existent-service".to_string()).unwrap();
    let filtered_traces = storage
        .list_recent_traces(10, Some(&non_existent_service))
        .await
        .unwrap();
    assert_eq!(
        filtered_traces.len(),
        0,
        "Non-existent service filter should return no traces"
    );

    // Test 9: Extremely old spans (test time filtering)
    let old_trace = TraceId::new("old_trace".to_string()).unwrap();
    let very_old_time = SystemTime::now() - Duration::from_secs(86400 * 365); // 1 year ago
    let old_span = Span::builder()
        .trace_id(old_trace.clone())
        .span_id(SpanId::new("old_span".to_string()).unwrap())
        .service_name(ServiceName::new("old-service".to_string()).unwrap())
        .operation_name("oldOp")
        .start_time(very_old_time)
        .duration(Duration::from_millis(60))
        .status(SpanStatus::Ok)
        .build()
        .unwrap();

    storage.store_span(old_span).await.unwrap();

    // Get service spans from recent time (should not include old span)
    let recent_time = SystemTime::now() - Duration::from_secs(3600); // 1 hour ago
    let recent_service_spans = storage
        .get_service_spans(
            &ServiceName::new("old-service".to_string()).unwrap(),
            recent_time,
        )
        .await
        .unwrap();
    assert_eq!(
        recent_service_spans.len(),
        0,
        "Should not return spans older than requested time"
    );

    // Test 10: Special characters in strings
    let special_trace = TraceId::new("special_!@#$%^&*()_trace".to_string()).unwrap();
    let special_span = Span::builder()
        .trace_id(special_trace.clone())
        .span_id(SpanId::new("special_span_!@#$".to_string()).unwrap())
        .service_name(ServiceName::new("special-service-!@#".to_string()).unwrap())
        .operation_name("operation with spaces and !@#$%^&*()")
        .start_time(SystemTime::now())
        .duration(Duration::from_millis(70))
        .status(SpanStatus::Ok)
        .attribute(
            "special.chars",
            "value with \n newlines \t tabs and 🎉 emoji",
        )
        .tag("env", "dev-test-🚀")
        .build()
        .unwrap();

    storage.store_span(special_span).await.unwrap();

    let special_spans = storage.get_trace_spans(&special_trace).await.unwrap();
    assert_eq!(special_spans.len(), 1, "Should handle special characters");
    assert!(
        special_spans[0]
            .attributes
            .get("special.chars")
            .unwrap()
            .contains("🎉"),
        "Should preserve emoji and special characters"
    );
}

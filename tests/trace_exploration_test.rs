//! Integration tests for trace exploration functionality.

mod common;

use common::*;
use std::time::{Duration, SystemTime};
use urpo_lib::core::{ServiceName, Span, SpanId, SpanStatus, TraceId};
use urpo_lib::storage::{InMemoryStorage, StorageBackend};

#[tokio::test]
async fn test_list_recent_traces() {
    let storage = InMemoryStorage::new(1000);

    // Create test traces
    create_test_traces(&storage, 5, 3).await;

    // Test listing recent traces
    let traces = test_storage_query!(storage, recent_traces(10));
    assert_eq!(traces.len(), 5);
    assert_traces_sorted_by_time(&traces);

    // Verify trace properties
    assert_trace_properties!(traces[0], spans: 4, service: "api-gateway");

    // Test filtering by service
    let api_traces = test_storage_query!(
        storage,
        recent_traces(10, &ServiceName::new("api-gateway".to_string()).unwrap())
    );
    assert_eq!(api_traces.len(), 5);

    let service_traces = test_storage_query!(
        storage,
        recent_traces(10, &ServiceName::new("service-1".to_string()).unwrap())
    );
    assert_eq!(service_traces.len(), 5);
}

#[tokio::test]
async fn test_get_error_traces() {
    let storage = InMemoryStorage::new(1000);

    // Create traces with alternating errors
    create_test_traces(&storage, 5, 2).await;

    // Test getting error traces
    let error_traces = test_storage_query!(storage, error_traces(10));
    assert_eq!(error_traces.len(), 3); // Traces 0, 2, 4 have errors

    for trace in &error_traces {
        assert_trace_properties!(trace, spans: 3, service: "api-gateway", has_error: true);
    }

    assert_traces_sorted_by_time(&error_traces);
}

#[tokio::test]
async fn test_get_slow_traces() {
    let storage = InMemoryStorage::new(1000);

    // Create traces with varying durations
    for i in 0..5 {
        let root = TestSpanBuilder::new(i, 0)
            .as_root()
            .duration_ms((i + 1) as u64 * 50)
            .service("api-gateway")
            .build();
        storage.store_span(root).await.unwrap();

        // Add child spans
        for j in 0..2 {
            let child = TestSpanBuilder::new(i, j + 1)
                .duration_ms(20)
                .service(&format!("service-{}", j))
                .build();
            storage.store_span(child).await.unwrap();
        }
    }

    // Test getting slow traces
    let slow_traces = test_storage_query!(storage, slow_traces(Duration::from_millis(100), 10));
    assert!(slow_traces.len() >= 3); // Traces with duration >= 100ms
    assert_traces_sorted_by_duration(&slow_traces);
}

#[tokio::test]
async fn test_search_traces() {
    let storage = InMemoryStorage::new(1000);

    // Create traces with specific operations
    for i in 0..3 {
        let root = TestSpanBuilder::new(i, 0)
            .as_root()
            .service("api-gateway")
            .build();
        storage.store_span(root).await.unwrap();

        // Add spans with searchable attributes
        let mut builder = Span::builder()
            .trace_id(TraceId::new(format!("trace_{:04}", i)).unwrap())
            .span_id(SpanId::new(format!("span_search_{}", i)).unwrap())
            .parent_span_id(SpanId::new(format!("span_root_{:04}", i)).unwrap())
            .service_name(ServiceName::new("search-service".to_string()).unwrap())
            .operation_name(if i == 1 { "special-op" } else { "normal-op" }.to_string())
            .start_time(SystemTime::now())
            .duration(Duration::from_millis(50))
            .status(SpanStatus::Ok);

        builder = builder.attribute("user.id", format!("user{}", i));
        if i == 1 {
            builder = builder.attribute("special", "true");
        }

        storage.store_span(builder.build().unwrap()).await.unwrap();
    }

    // Test searching
    let results = test_storage_query!(storage, search("special", 10));
    assert_eq!(results.len(), 1);

    let user_results = test_storage_query!(storage, search("user1", 10));
    assert_eq!(user_results.len(), 1);
}

#[tokio::test]
async fn test_trace_with_multiple_services() {
    let storage = InMemoryStorage::new(1000);

    // Create a complex trace
    let trace_id = TraceId::new("complex_trace".to_string()).unwrap();

    // Root span
    storage
        .store_span(
            Span::builder()
                .trace_id(trace_id.clone())
                .span_id(SpanId::new("root".to_string()).unwrap())
                .service_name(ServiceName::new("gateway".to_string()).unwrap())
                .operation_name("GET /api/data")
                .start_time(SystemTime::now())
                .duration(Duration::from_millis(200))
                .status(SpanStatus::Ok)
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

    // Service chain
    let services = ["auth", "database", "cache", "processor"];
    for (i, service) in services.iter().enumerate() {
        storage
            .store_span(
                Span::builder()
                    .trace_id(trace_id.clone())
                    .span_id(SpanId::new(format!("span_{}", service)).unwrap())
                    .parent_span_id(SpanId::new("root".to_string()).unwrap())
                    .service_name(ServiceName::new(service.to_string()).unwrap())
                    .operation_name(format!("{}_operation", service))
                    .start_time(SystemTime::now())
                    .duration(Duration::from_millis(30))
                    .status(if i == 2 {
                        SpanStatus::Error("Cache miss".to_string())
                    } else {
                        SpanStatus::Ok
                    })
                    .build()
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Verify the trace
    let traces = test_storage_query!(storage, recent_traces(1));
    assert_eq!(traces.len(), 1);

    let trace = &traces[0];
    assert_trace_properties!(trace, spans: 5, service: "gateway", has_error: true);
    assert!(trace.services.len() >= 4);
}

#[tokio::test]
async fn test_concurrent_trace_storage() {
    let storage = InMemoryStorage::new(10000);

    // Spawn multiple tasks to store traces concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let storage_clone = storage.clone();
        let handle = tokio::spawn(async move {
            create_test_trace(&storage_clone, i, 5, false).await;
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all traces were stored
    let traces = test_storage_query!(storage, recent_traces(20));
    assert_eq!(traces.len(), 10);

    for trace in &traces {
        assert_trace_properties!(trace, spans: 6);
    }
}

#[tokio::test]
async fn test_storage_limits() {
    let storage = InMemoryStorage::new(10); // Small limit

    // Try to store more spans than the limit
    for i in 0..20 {
        let span = test_span!(i, 0, root);
        let _ = storage.store_span(span).await; // May fail when over limit
    }

    // Check that storage respects the limit
    let count = storage.get_span_count().await.unwrap();
    assert!(count <= 10, "Storage should respect max span limit");
}

#[tokio::test]
async fn test_trace_duration_calculation() {
    let storage = InMemoryStorage::new(1000);
    let trace_id = TraceId::new("duration_test".to_string()).unwrap();

    // Create spans with different start times and durations
    let base_time = SystemTime::now();

    storage
        .store_span(
            Span::builder()
                .trace_id(trace_id.clone())
                .span_id(SpanId::new("early".to_string()).unwrap())
                .service_name(ServiceName::new("service1".to_string()).unwrap())
                .operation_name("early_op")
                .start_time(base_time)
                .duration(Duration::from_millis(50))
                .status(SpanStatus::Ok)
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

    storage
        .store_span(
            Span::builder()
                .trace_id(trace_id.clone())
                .span_id(SpanId::new("late".to_string()).unwrap())
                .service_name(ServiceName::new("service2".to_string()).unwrap())
                .operation_name("late_op")
                .start_time(base_time + Duration::from_millis(100))
                .duration(Duration::from_millis(50))
                .status(SpanStatus::Ok)
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

    // Check trace duration calculation
    let traces = test_storage_query!(storage, recent_traces(1));
    assert_eq!(traces.len(), 1);

    let trace = &traces[0];
    assert!(trace.duration >= Duration::from_millis(150));
}

#[tokio::test]
async fn test_orphan_span_handling() {
    let storage = InMemoryStorage::new(1000);

    // Store orphan spans (no root)
    for i in 0..3 {
        let span = TestSpanBuilder::new(99, i)
            .service("orphan-service")
            .build();
        storage.store_span(span).await.unwrap();
    }

    // Verify orphan trace is still accessible
    let traces = test_storage_query!(storage, recent_traces(10));
    assert_eq!(traces.len(), 1);

    let trace = &traces[0];
    assert_eq!(trace.span_count, 3);
    // Root service should be from the first span since no parent exists
    assert_eq!(trace.root_service.as_str(), "orphan-service");
}

#[tokio::test]
async fn test_service_filtering_performance() {
    let storage = InMemoryStorage::new(10000);

    // Create many traces across different services
    for i in 0..100 {
        let service_num = i % 10;
        let root = TestSpanBuilder::new(i, 0)
            .as_root()
            .service(&format!("service-{}", service_num))
            .build();
        storage.store_span(root).await.unwrap();
    }

    // Test filtering performance
    let start = SystemTime::now();
    let filtered = test_storage_query!(
        storage,
        recent_traces(100, &ServiceName::new("service-5".to_string()).unwrap())
    );
    let duration = SystemTime::now().duration_since(start).unwrap();

    assert_eq!(filtered.len(), 10);
    assert!(duration < Duration::from_millis(100), "Filtering should be fast");
}

#[tokio::test]
async fn test_trace_cleanup() {
    let storage = InMemoryStorage::new(100);

    // Fill storage beyond capacity
    create_test_traces(&storage, 20, 5).await;

    // Get initial count
    let initial_count = storage.get_span_count().await.unwrap();

    // Trigger cleanup
    let removed = storage.emergency_cleanup().await.unwrap_or(0);

    // Either spans were removed OR storage was already at limit
    let after_count = storage.get_span_count().await.unwrap();
    assert!(
        removed > 0 || after_count <= 100,
        "Cleanup should remove spans or maintain limit"
    );

    // Verify storage still works after cleanup
    let traces = test_storage_query!(storage, recent_traces(50));
    assert!(traces.len() <= 20, "Traces should be limited");

    // Can still add new traces
    create_test_trace(&storage, 100, 2, false).await;
    let new_traces = test_storage_query!(storage, recent_traces(1));
    assert_eq!(new_traces.len(), 1);
}

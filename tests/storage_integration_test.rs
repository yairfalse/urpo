//! Integration tests for storage backend with fake spans and metrics aggregation.

use urpo_lib::core::{ServiceMetrics, ServiceName};
use urpo_lib::storage::{StorageManager, fake_spans::FakeSpanGenerator};
use std::time::Duration;

#[tokio::test]
async fn test_storage_with_fake_spans() {
    // Create storage manager
    let storage_manager = StorageManager::new_in_memory(1000);
    let storage = storage_manager.backend();
    
    // Generate fake spans
    let generator = FakeSpanGenerator::new();
    let spans = generator.generate_batch(100).await.unwrap();
    
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
    let api_gateway = metrics.iter()
        .find(|m| m.name.as_str() == "api-gateway")
        .expect("Should have api-gateway metrics");
    
    assert!(api_gateway.error_rate < 0.1); // API gateway should have low error rate
    
    let payment_service = metrics.iter()
        .find(|m| m.name.as_str() == "payment-service")
        .expect("Should have payment-service metrics");
    
    assert!(payment_service.error_rate > 0.0); // Payment service should have some errors
}

#[tokio::test]
async fn test_storage_limits() {
    // Create storage with small limit
    let storage_manager = StorageManager::new_in_memory(10);
    let storage = storage_manager.backend();
    
    // Generate and store more spans than the limit
    let generator = FakeSpanGenerator::new();
    let spans = generator.generate_batch(50).await.unwrap();
    
    for span in spans {
        storage.store_span(span).await.unwrap();
    }
    
    // Check that storage enforced limits
    let count = storage.get_span_count().await.unwrap();
    assert!(count <= 10, "Storage should enforce max span limit");
}

#[tokio::test]
async fn test_metrics_calculation_accuracy() {
    let storage_manager = StorageManager::new_in_memory(10000);
    let storage = storage_manager.backend();
    
    // Generate spans with known characteristics
    let generator = FakeSpanGenerator::new();
    
    // Generate many spans to get stable metrics
    for _ in 0..10 {
        let spans = generator.generate_batch(100).await.unwrap();
        for span in spans {
            storage.store_span(span).await.unwrap();
        }
    }
    
    let metrics = storage.get_service_metrics().await.unwrap();
    
    // Verify all expected services are present
    let service_names: Vec<&str> = metrics.iter()
        .map(|m| m.name.as_str())
        .collect();
    
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
    let storage_manager = StorageManager::new_in_memory(1000);
    let storage = storage_manager.backend();
    
    // Generate spans
    let generator = FakeSpanGenerator::new();
    let spans = generator.generate_batch(200).await.unwrap();
    
    for span in spans {
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
            }
            "payment-service" => {
                // Payment service has higher error rate by design
                assert!(metric.error_rate > 0.01, "payment-service should have some errors");
            }
            _ => {}
        }
    }
}
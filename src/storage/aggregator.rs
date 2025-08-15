//! Metrics aggregation from stored spans.
//!
//! This module calculates service metrics from spans stored in the storage backend,
//! computing real-time statistics like RPS, error rates, and latency percentiles.

use crate::core::{Result, ServiceMetrics, ServiceName};
use crate::storage::StorageBackend;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Time window for metric calculation (60 seconds).
const METRIC_WINDOW_SECS: u64 = 60;

/// Calculate service metrics from spans in storage.
///
/// This function queries spans from the storage backend for the last 60 seconds
/// and calculates metrics including RPS, error rate, and latency percentiles.
pub async fn calculate_service_metrics(
    storage: &dyn StorageBackend,
) -> Result<Vec<ServiceMetrics>> {
    let window_start = SystemTime::now() - Duration::from_secs(METRIC_WINDOW_SECS);
    
    // Get all unique service names from storage
    // In a real implementation, we'd have a method to list services
    // For now, we'll use the known service names
    let service_names = vec![
        ServiceName::new("api-gateway".to_string())?,
        ServiceName::new("user-service".to_string())?,
        ServiceName::new("order-service".to_string())?,
        ServiceName::new("payment-service".to_string())?,
        ServiceName::new("inventory-service".to_string())?,
    ];
    
    let mut metrics = Vec::new();
    
    for service_name in service_names {
        // Get spans for this service in the time window
        let spans = storage.get_service_spans(&service_name, window_start).await?;
        
        if spans.is_empty() {
            // Create metrics with zero values for services with no recent activity
            metrics.push(ServiceMetrics::new(service_name));
            continue;
        }
        
        // Calculate metrics from spans
        let span_count = spans.len() as u64;
        let error_count = spans.iter().filter(|s| s.status.is_error()).count() as u64;
        
        // Calculate RPS (requests per second)
        let request_rate = span_count as f64 / METRIC_WINDOW_SECS as f64;
        
        // Calculate error rate
        let error_rate = if span_count > 0 {
            error_count as f64 / span_count as f64
        } else {
            0.0
        };
        
        // Collect and sort latencies for percentile calculation
        let mut latencies: Vec<u64> = spans.iter()
            .map(|s| s.duration.as_millis() as u64)
            .collect();
        latencies.sort_unstable();
        
        // Calculate percentiles
        let p50 = calculate_percentile(&latencies, 0.50);
        let p95 = calculate_percentile(&latencies, 0.95);
        let p99 = calculate_percentile(&latencies, 0.99);
        
        // Calculate average duration
        let total_duration: u64 = latencies.iter().sum();
        let avg_duration = if !latencies.is_empty() {
            Duration::from_millis(total_duration / latencies.len() as u64)
        } else {
            Duration::from_millis(0)
        };
        
        // Find min and max durations
        let min_duration = latencies.first()
            .map(|&ms| Duration::from_millis(ms))
            .unwrap_or(Duration::from_millis(0));
        let max_duration = latencies.last()
            .map(|&ms| Duration::from_millis(ms))
            .unwrap_or(Duration::from_millis(0));
        
        // Create the service metrics
        let mut service_metrics = ServiceMetrics::new(service_name);
        service_metrics.request_rate = request_rate;
        service_metrics.error_rate = error_rate;
        service_metrics.latency_p50 = Duration::from_millis(p50);
        service_metrics.latency_p95 = Duration::from_millis(p95);
        service_metrics.latency_p99 = Duration::from_millis(p99);
        service_metrics.span_count = span_count;
        service_metrics.error_count = error_count;
        service_metrics.avg_duration = avg_duration;
        service_metrics.min_duration = min_duration;
        service_metrics.max_duration = max_duration;
        service_metrics.last_seen = SystemTime::now();
        
        metrics.push(service_metrics);
    }
    
    // Sort by service name for consistent display
    metrics.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
    
    Ok(metrics)
}

/// Calculate a percentile value from a sorted list of values.
fn calculate_percentile(sorted_values: &[u64], percentile: f64) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }
    
    if sorted_values.len() == 1 {
        return sorted_values[0];
    }
    
    let index = ((sorted_values.len() - 1) as f64 * percentile) as usize;
    sorted_values[index]
}

/// Aggregated metrics over multiple time windows.
#[derive(Debug, Clone)]
pub struct WindowedMetrics {
    /// Service name
    pub service_name: ServiceName,
    /// Metrics for different time windows
    pub windows: HashMap<String, ServiceMetrics>,
}

impl WindowedMetrics {
    /// Create new windowed metrics for a service.
    pub fn new(service_name: ServiceName) -> Self {
        Self {
            service_name,
            windows: HashMap::new(),
        }
    }
    
    /// Add metrics for a specific time window.
    pub fn add_window(&mut self, window_name: String, metrics: ServiceMetrics) {
        self.windows.insert(window_name, metrics);
    }
    
    /// Get metrics for a specific window.
    pub fn get_window(&self, window_name: &str) -> Option<&ServiceMetrics> {
        self.windows.get(window_name)
    }
}

/// Calculate metrics for multiple time windows.
pub async fn calculate_windowed_metrics(
    storage: &dyn StorageBackend,
    service_name: &ServiceName,
) -> Result<WindowedMetrics> {
    let mut windowed = WindowedMetrics::new(service_name.clone());
    
    // Calculate for different time windows
    let windows = vec![
        ("1m", 60),
        ("5m", 300),
        ("15m", 900),
        ("1h", 3600),
    ];
    
    for (name, seconds) in windows {
        let window_start = SystemTime::now() - Duration::from_secs(seconds);
        let spans = storage.get_service_spans(service_name, window_start).await?;
        
        if !spans.is_empty() {
            // Calculate metrics for this window
            let span_count = spans.len() as u64;
            let error_count = spans.iter().filter(|s| s.status.is_error()).count() as u64;
            let request_rate = span_count as f64 / seconds as f64;
            let error_rate = error_count as f64 / span_count as f64;
            
            let mut latencies: Vec<u64> = spans.iter()
                .map(|s| s.duration.as_millis() as u64)
                .collect();
            latencies.sort_unstable();
            
            let mut metrics = ServiceMetrics::new(service_name.clone());
            metrics.request_rate = request_rate;
            metrics.error_rate = error_rate;
            metrics.latency_p50 = Duration::from_millis(calculate_percentile(&latencies, 0.50));
            metrics.latency_p95 = Duration::from_millis(calculate_percentile(&latencies, 0.95));
            metrics.latency_p99 = Duration::from_millis(calculate_percentile(&latencies, 0.99));
            metrics.span_count = span_count;
            metrics.error_count = error_count;
            
            windowed.add_window(name.to_string(), metrics);
        }
    }
    
    Ok(windowed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Span, SpanId, SpanStatus, TraceId};
    use crate::storage::InMemoryStorage;

    async fn create_test_span(
        service: &str,
        duration_ms: u64,
        is_error: bool,
    ) -> Span {
        let status = if is_error {
            SpanStatus::Error("Test error".to_string())
        } else {
            SpanStatus::Ok
        };
        
        Span::builder()
            .trace_id(TraceId::new(format!("trace_{}", rand::random::<u32>())).unwrap())
            .span_id(SpanId::new(format!("span_{}", rand::random::<u32>())).unwrap())
            .service_name(ServiceName::new(service.to_string()).unwrap())
            .operation_name("test_op")
            .start_time(SystemTime::now())
            .duration(Duration::from_millis(duration_ms))
            .status(status)
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_calculate_percentile() {
        let values = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        
        assert_eq!(calculate_percentile(&values, 0.0), 10);
        assert_eq!(calculate_percentile(&values, 0.5), 50);
        assert_eq!(calculate_percentile(&values, 0.9), 90);
        assert_eq!(calculate_percentile(&values, 1.0), 100);
    }

    #[tokio::test]
    async fn test_calculate_service_metrics() {
        let storage = InMemoryStorage::new(1000);
        
        // Add test spans
        for i in 0..100 {
            let duration = 10 + (i % 50) * 2; // Vary duration
            let is_error = i % 20 == 0; // 5% error rate
            let span = create_test_span("api-gateway", duration, is_error).await;
            storage.store_span(span).await.unwrap();
        }
        
        let metrics = calculate_service_metrics(&storage).await.unwrap();
        
        // Find api-gateway metrics
        let api_metrics = metrics.iter()
            .find(|m| m.name.as_str() == "api-gateway")
            .expect("Should have api-gateway metrics");
        
        assert_eq!(api_metrics.span_count, 100);
        assert_eq!(api_metrics.error_count, 5);
        assert!((api_metrics.error_rate - 0.05).abs() < 0.001);
        assert!(api_metrics.request_rate > 0.0);
        assert!(api_metrics.latency_p50.as_millis() > 0);
        assert!(api_metrics.latency_p95.as_millis() >= api_metrics.latency_p50.as_millis());
        assert!(api_metrics.latency_p99.as_millis() >= api_metrics.latency_p95.as_millis());
    }

    #[tokio::test]
    async fn test_empty_service_metrics() {
        let storage = InMemoryStorage::new(1000);
        
        let metrics = calculate_service_metrics(&storage).await.unwrap();
        
        // Should have metrics for all services even with no data
        assert!(!metrics.is_empty());
        
        for metric in metrics {
            assert_eq!(metric.span_count, 0);
            assert_eq!(metric.error_count, 0);
            assert_eq!(metric.request_rate, 0.0);
            assert_eq!(metric.error_rate, 0.0);
        }
    }

    #[tokio::test]
    async fn test_windowed_metrics() {
        let storage = InMemoryStorage::new(1000);
        let service_name = ServiceName::new("test-service".to_string()).unwrap();
        
        // Add spans
        for _ in 0..50 {
            let span = create_test_span("test-service", 100, false).await;
            storage.store_span(span).await.unwrap();
        }
        
        let windowed = calculate_windowed_metrics(&storage, &service_name).await.unwrap();
        
        assert_eq!(windowed.service_name, service_name);
        
        // Should have 1m window with data
        let one_min = windowed.get_window("1m");
        assert!(one_min.is_some());
        
        let metrics = one_min.unwrap();
        assert_eq!(metrics.span_count, 50);
        assert!(metrics.request_rate > 0.0);
    }
}
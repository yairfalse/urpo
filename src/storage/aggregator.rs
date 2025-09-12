//! Metrics aggregation from stored spans.
//!
//! This module calculates service metrics from spans stored in the storage backend,
//! computing real-time statistics like RPS, error rates, and latency percentiles.

use crate::core::{Result, ServiceMetrics, ServiceName};
use crate::storage::StorageBackend;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// Time window for metric calculation (60 seconds).
const METRIC_WINDOW_SECS: u64 = 60;

/// Number of histogram buckets for latency tracking.
const LATENCY_BUCKETS: usize = 50;

/// Maximum number of data points per sliding window.
const MAX_WINDOW_SIZE: usize = 10000;

/// Calculate service metrics from spans in storage.
///
/// This function queries spans from the storage backend for the last 60 seconds
/// and calculates metrics including RPS, error rate, and latency percentiles.
pub async fn calculate_service_metrics(
    storage: &dyn StorageBackend,
) -> Result<Vec<ServiceMetrics>> {
    let window_start = SystemTime::now() - Duration::from_secs(METRIC_WINDOW_SECS);
    
    // BLAZING FAST: Get service names from storage without hardcoding
    let service_names = storage.list_services().await?;
    
    let mut metrics = Vec::new();
    
    for service_name in service_names {
        // Get spans for this service in the time window
        let spans = storage.get_service_spans(&service_name, window_start).await?;
        
        if spans.is_empty() {
            // Create metrics with zero values for services with no recent activity
            metrics.push(ServiceMetrics::new(service_name));
            continue;
        }
        
        // Batch process spans for efficiency
        let (span_count, error_count, mut latencies) = process_spans_batch(&spans);
        
        // Calculate RPS (requests per second)
        let request_rate = span_count as f64 / METRIC_WINDOW_SECS as f64;
        
        // Calculate error rate
        let error_rate = if span_count > 0 {
            error_count as f64 / span_count as f64
        } else {
            0.0
        };
        
        // BLAZING FAST: Calculate percentiles WITHOUT cloning
        let (p50, p95, p99) = if latencies.len() > 1000 {
            // Use histogram approximation for large datasets
            calculate_percentiles_histogram(&latencies)
        } else {
            // ZERO ALLOCATION: Sort in-place for percentiles
            calculate_percentiles_exact(&mut latencies)
        };
        
        // Calculate average duration
        let total_duration: u64 = latencies.iter().sum();
        let avg_duration = if !latencies.is_empty() {
            Duration::from_millis(total_duration / latencies.len() as u64)
        } else {
            Duration::from_millis(0)
        };
        
        // BLAZING FAST: Find min/max WITHOUT sorting or cloning
        let (min_ms, max_ms) = latencies.iter()
            .fold((u64::MAX, 0u64), |(min, max), &val| {
                (min.min(val), max.max(val))
            });
        let min_duration = if latencies.is_empty() { 
            Duration::from_millis(0) 
        } else { 
            Duration::from_millis(min_ms) 
        };
        let max_duration = Duration::from_millis(max_ms);
        
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

/// BLAZING FAST: Calculate exact percentiles WITHOUT cloning
/// This modifies the input array in-place for zero allocations
#[inline]
fn calculate_percentiles_exact(latencies: &mut Vec<u64>) -> (u64, u64, u64) {
    if latencies.is_empty() {
        return (0, 0, 0);
    }
    
    // ZERO ALLOCATION: Sort in-place
    latencies.sort_unstable();
    
    let len = latencies.len();
    let p50 = latencies[len / 2];
    let p95 = latencies[len * 95 / 100];
    let p99 = latencies[len * 99 / 100];
    
    (p50, p95, p99)
}

/// Calculate percentiles using histogram approximation for large datasets.
fn calculate_percentiles_histogram(latencies: &[u64]) -> (u64, u64, u64) {
    if latencies.is_empty() {
        return (0, 0, 0);
    }
    
    // BLAZING FAST: Find min/max in single pass without panic
    let (min_latency, max_latency) = latencies.iter()
        .fold((u64::MAX, 0u64), |(min, max), &val| {
            (min.min(val), max.max(val))
        });
    
    if min_latency == max_latency {
        return (min_latency, min_latency, min_latency);
    }
    
    // Create histogram buckets
    let bucket_size = (max_latency - min_latency + LATENCY_BUCKETS as u64 - 1) / LATENCY_BUCKETS as u64;
    let mut histogram = vec![0u64; LATENCY_BUCKETS];
    
    // Fill histogram
    for &latency in latencies {
        let bucket = ((latency - min_latency) / bucket_size).min(LATENCY_BUCKETS as u64 - 1) as usize;
        histogram[bucket] += 1;
    }
    
    // Calculate cumulative distribution
    let total_count = latencies.len() as u64;
    let mut cumulative = 0u64;
    let mut p50 = min_latency;
    let mut p95 = min_latency;
    let mut p99 = min_latency;
    
    for (i, &count) in histogram.iter().enumerate() {
        cumulative += count;
        let percentile = cumulative as f64 / total_count as f64;
        
        let bucket_start = min_latency + i as u64 * bucket_size;
        
        if percentile >= 0.50 && p50 == min_latency {
            p50 = bucket_start;
        }
        if percentile >= 0.95 && p95 == min_latency {
            p95 = bucket_start;
        }
        if percentile >= 0.99 && p99 == min_latency {
            p99 = bucket_start;
        }
    }
    
    (p50, p95, p99)
}

/// Efficiently process spans in batches.
fn process_spans_batch(spans: &[crate::core::Span]) -> (u64, u64, Vec<u64>) {
    let mut span_count = 0u64;
    let mut error_count = 0u64;
    let mut latencies = Vec::with_capacity(spans.len());
    
    for span in spans {
        span_count += 1;
        
        if span.status.is_error() {
            error_count += 1;
        }
        
        latencies.push(span.duration.as_millis() as u64);
    }
    
    (span_count, error_count, latencies)
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
            // Batch process spans for efficiency
            let (span_count, error_count, mut latencies) = process_spans_batch(&spans);
            let request_rate = span_count as f64 / seconds as f64;
            let error_rate = error_count as f64 / span_count as f64;
            
            // Calculate percentiles using efficient method
            let (p50, p95, p99) = if latencies.len() > 1000 {
                calculate_percentiles_histogram(&latencies)
            } else {
                let mut sorted_latencies = latencies;
                sorted_latencies.sort_unstable();
                (
                    calculate_percentile(&sorted_latencies, 0.50),
                    calculate_percentile(&sorted_latencies, 0.95),
                    calculate_percentile(&sorted_latencies, 0.99),
                )
            };
            
            let mut metrics = ServiceMetrics::new(service_name.clone());
            metrics.request_rate = request_rate;
            metrics.error_rate = error_rate;
            metrics.latency_p50 = Duration::from_millis(p50);
            metrics.latency_p95 = Duration::from_millis(p95);
            metrics.latency_p99 = Duration::from_millis(p99);
            metrics.span_count = span_count;
            metrics.error_count = error_count;
            
            windowed.add_window(name.to_string(), metrics);
        }
    }
    
    Ok(windowed)
}

/// Sliding window for efficient real-time metrics.
#[derive(Debug)]
pub struct SlidingWindow {
    /// Service name.
    service_name: ServiceName,
    /// Data points with timestamps.
    data_points: VecDeque<(SystemTime, MetricDataPoint)>,
    /// Window duration.
    window_duration: Duration,
    /// Cached metrics (updated incrementally).
    cached_metrics: Option<ServiceMetrics>,
    /// Last update time.
    last_update: SystemTime,
}

/// Individual metric data point.
#[derive(Debug, Clone)]
struct MetricDataPoint {
    /// Request count.
    requests: u64,
    /// Error count.
    errors: u64,
    /// Latency value in milliseconds.
    latency_ms: u64,
}

impl SlidingWindow {
    /// Create a new sliding window.
    pub fn new(service_name: ServiceName, window_duration: Duration) -> Self {
        Self {
            service_name,
            data_points: VecDeque::new(),
            window_duration,
            cached_metrics: None,
            last_update: SystemTime::now(),
        }
    }
    
    /// Add a data point to the window.
    pub fn add_data_point(&mut self, timestamp: SystemTime, requests: u64, errors: u64, latency_ms: u64) {
        // Remove expired data points
        let cutoff = timestamp - self.window_duration;
        while let Some((ts, _)) = self.data_points.front() {
            if *ts < cutoff {
                self.data_points.pop_front();
            } else {
                break;
            }
        }
        
        // Add new data point
        let data_point = MetricDataPoint {
            requests,
            errors,
            latency_ms,
        };
        
        self.data_points.push_back((timestamp, data_point));
        
        // Limit window size to prevent memory growth
        while self.data_points.len() > MAX_WINDOW_SIZE {
            self.data_points.pop_front();
        }
        
        // Invalidate cache
        self.cached_metrics = None;
        self.last_update = timestamp;
    }
    
    /// Get current metrics from the sliding window.
    pub fn get_metrics(&mut self) -> ServiceMetrics {
        // Use cached metrics if available and recent
        if let Some(ref metrics) = self.cached_metrics {
            if self.last_update.duration_since(SystemTime::now()).unwrap_or(Duration::ZERO) < Duration::from_secs(1) {
                return metrics.clone();
            }
        }
        
        // Calculate metrics from data points
        let mut total_requests = 0u64;
        let mut total_errors = 0u64;
        let mut latencies = Vec::new();
        
        for (_, data_point) in &self.data_points {
            total_requests += data_point.requests;
            total_errors += data_point.errors;
            latencies.push(data_point.latency_ms);
        }
        
        let request_rate = total_requests as f64 / self.window_duration.as_secs() as f64;
        let error_rate = if total_requests > 0 {
            total_errors as f64 / total_requests as f64
        } else {
            0.0
        };
        
        // Calculate percentiles
        let (p50, p95, p99) = if latencies.len() > 1000 {
            calculate_percentiles_histogram(&latencies)
        } else {
            latencies.sort_unstable();
            (
                calculate_percentile(&latencies, 0.50),
                calculate_percentile(&latencies, 0.95),
                calculate_percentile(&latencies, 0.99),
            )
        };
        
        // Calculate other stats
        let avg_latency = if !latencies.is_empty() {
            latencies.iter().sum::<u64>() / latencies.len() as u64
        } else {
            0
        };
        
        let min_latency = latencies.iter().min().copied().unwrap_or(0);
        let max_latency = latencies.iter().max().copied().unwrap_or(0);
        
        // Create metrics
        let mut metrics = ServiceMetrics::new(self.service_name.clone());
        metrics.request_rate = request_rate;
        metrics.error_rate = error_rate;
        metrics.latency_p50 = Duration::from_millis(p50);
        metrics.latency_p95 = Duration::from_millis(p95);
        metrics.latency_p99 = Duration::from_millis(p99);
        metrics.span_count = total_requests;
        metrics.error_count = total_errors;
        metrics.avg_duration = Duration::from_millis(avg_latency);
        metrics.min_duration = Duration::from_millis(min_latency);
        metrics.max_duration = Duration::from_millis(max_latency);
        metrics.last_seen = self.last_update;
        
        // Cache the result
        self.cached_metrics = Some(metrics.clone());
        
        metrics
    }
}

/// Real-time metrics aggregator with sliding windows.
#[derive(Debug)]
pub struct RealtimeAggregator {
    /// Sliding windows per service.
    windows: Arc<RwLock<HashMap<ServiceName, SlidingWindow>>>,
    /// Window duration.
    window_duration: Duration,
}

impl RealtimeAggregator {
    /// Create a new real-time aggregator.
    pub fn new(window_duration: Duration) -> Self {
        Self {
            windows: Arc::new(RwLock::new(HashMap::new())),
            window_duration,
        }
    }
    
    /// Add spans to the aggregator.
    pub async fn add_spans(&self, spans: &[crate::core::Span]) {
        let timestamp = SystemTime::now();
        let mut windows = self.windows.write().await;
        
        // Group spans by service
        let mut service_data: HashMap<ServiceName, (u64, u64, Vec<u64>)> = HashMap::new();
        
        for span in spans {
            let entry = service_data.entry(span.service_name.clone()).or_default();
            entry.0 += 1; // requests
            if span.status.is_error() {
                entry.1 += 1; // errors
            }
            entry.2.push(span.duration.as_millis() as u64); // latencies
        }
        
        // Update windows
        for (service_name, (requests, errors, latencies)) in service_data {
            let window = windows.entry(service_name.clone())
                .or_insert_with(|| SlidingWindow::new(service_name, self.window_duration));
            
            // Use average latency for the data point
            let avg_latency = if !latencies.is_empty() {
                latencies.iter().sum::<u64>() / latencies.len() as u64
            } else {
                0
            };
            
            window.add_data_point(timestamp, requests, errors, avg_latency);
        }
    }
    
    /// Get metrics for all services.
    pub async fn get_all_metrics(&self) -> Vec<ServiceMetrics> {
        let mut windows = self.windows.write().await;
        let mut metrics = Vec::new();
        
        for window in windows.values_mut() {
            metrics.push(window.get_metrics());
        }
        
        // Sort by service name for consistency
        metrics.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        
        metrics
    }
    
    /// Get metrics for a specific service.
    pub async fn get_service_metrics(&self, service_name: &ServiceName) -> Option<ServiceMetrics> {
        let mut windows = self.windows.write().await;
        windows.get_mut(service_name).map(|window| window.get_metrics())
    }
    
    /// Clean up old windows for inactive services.
    pub async fn cleanup_inactive(&self, cutoff: SystemTime) {
        let mut windows = self.windows.write().await;
        windows.retain(|_, window| window.last_update >= cutoff);
    }
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
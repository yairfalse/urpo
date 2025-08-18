//! Fake span generator for testing and demo purposes.
//!
//! This module generates realistic OTEL span data for demonstration
//! and testing of the storage and aggregation systems.

use crate::core::{Result, ServiceName, Span, SpanId, SpanStatus, TraceId};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::time;

/// Service configuration for realistic span generation.
#[derive(Debug, Clone)]
struct ServiceConfig {
    /// Service name
    name: ServiceName,
    /// Base request rate (spans per second)
    base_rps: f64,
    /// Variation in RPS (0.0 to 1.0)
    rps_variation: f64,
    /// Base error rate (0.0 to 1.0)
    base_error_rate: f64,
    /// Base P50 latency in milliseconds
    base_p50_ms: u64,
    /// Base P95 latency in milliseconds
    base_p95_ms: u64,
    /// Base P99 latency in milliseconds
    base_p99_ms: u64,
    /// Common operations for this service
    operations: Vec<String>,
}

impl ServiceConfig {
    /// Create a new service configuration.
    fn new(
        name: &str,
        base_rps: f64,
        base_error_rate: f64,
        base_p50_ms: u64,
        base_p95_ms: u64,
        base_p99_ms: u64,
        operations: Vec<&str>,
    ) -> Self {
        Self {
            name: ServiceName::new(name.to_string()).unwrap(),
            base_rps,
            rps_variation: 0.2, // 20% variation
            base_error_rate,
            base_p50_ms,
            base_p95_ms,
            base_p99_ms,
            operations: operations.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Generate a random latency based on percentiles.
    fn generate_latency(&self) -> Duration {
        let mut rng = thread_rng();
        let percentile: f64 = rng.gen();
        
        let ms = if percentile < 0.5 {
            // P0-P50: Linear between 0 and P50
            (self.base_p50_ms as f64 * percentile * 2.0) as u64
        } else if percentile < 0.95 {
            // P50-P95: Linear between P50 and P95
            let range = self.base_p95_ms - self.base_p50_ms;
            let position = (percentile - 0.5) / 0.45;
            self.base_p50_ms + (range as f64 * position) as u64
        } else {
            // P95-P100: Linear between P95 and P99 (with some outliers)
            let range = self.base_p99_ms - self.base_p95_ms;
            let position = (percentile - 0.95) / 0.05;
            let base = self.base_p95_ms + (range as f64 * position) as u64;
            
            // Add occasional outliers (1% chance of 2x latency)
            if rng.gen::<f64>() < 0.01 {
                base * 2
            } else {
                base
            }
        };
        
        // Add some jitter
        let jitter = rng.gen_range(0.9..1.1);
        Duration::from_millis((ms as f64 * jitter) as u64)
    }

    /// Check if this request should error based on error rate.
    fn should_error(&self) -> bool {
        thread_rng().gen::<f64>() < self.base_error_rate
    }

    /// Get a random operation name for this service.
    fn random_operation(&self) -> String {
        let idx = thread_rng().gen_range(0..self.operations.len());
        self.operations[idx].clone()
    }

    /// Calculate current RPS with variation.
    fn current_rps(&self) -> f64 {
        let variation = thread_rng().gen_range(-self.rps_variation..self.rps_variation);
        self.base_rps * (1.0 + variation)
    }
}

/// Fake span generator that produces realistic OTEL spans.
pub struct SpanGenerator {
    /// Service configurations
    services: Vec<ServiceConfig>,
    /// Counter for generating unique IDs
    id_counter: Arc<RwLock<u64>>,
    /// Whether the generator is running
    running: Arc<RwLock<bool>>,
}

impl SpanGenerator {
    /// Create a new fake span generator with default services.
    pub fn new() -> Self {
        let services = vec![
            ServiceConfig::new(
                "api-gateway",
                50.0,  // 50 RPS
                0.001, // 0.1% error rate
                20,    // P50: 20ms
                50,    // P95: 50ms
                100,   // P99: 100ms
                vec!["GET /api/v1/users", "POST /api/v1/orders", "GET /api/v1/products"],
            ),
            ServiceConfig::new(
                "user-service",
                30.0,  // 30 RPS
                0.002, // 0.2% error rate
                15,    // P50: 15ms
                40,    // P95: 40ms
                80,    // P99: 80ms
                vec!["getUserById", "updateUser", "listUsers", "authenticateUser"],
            ),
            ServiceConfig::new(
                "order-service",
                25.0,  // 25 RPS
                0.005, // 0.5% error rate
                30,    // P50: 30ms
                80,    // P95: 80ms
                150,   // P99: 150ms
                vec!["createOrder", "getOrder", "updateOrderStatus", "cancelOrder"],
            ),
            ServiceConfig::new(
                "payment-service",
                15.0,  // 15 RPS
                0.02,  // 2% error rate (payment failures)
                100,   // P50: 100ms (external API calls)
                300,   // P95: 300ms
                500,   // P99: 500ms
                vec!["processPayment", "refundPayment", "validateCard", "getPaymentStatus"],
            ),
            ServiceConfig::new(
                "inventory-service",
                40.0,  // 40 RPS
                0.003, // 0.3% error rate
                10,    // P50: 10ms (cached responses)
                25,    // P95: 25ms
                50,    // P99: 50ms
                vec!["checkStock", "reserveItems", "updateInventory", "getProductInfo"],
            ),
        ];

        Self {
            services,
            id_counter: Arc::new(RwLock::new(0)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Generate a unique ID.
    async fn generate_id(&self, prefix: &str) -> String {
        let mut counter = self.id_counter.write().await;
        *counter += 1;
        
        // Generate a random suffix for uniqueness
        let suffix: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        
        format!("{}_{:08x}_{}", prefix, *counter, suffix)
    }

    /// Generate a single span for a service.
    async fn generate_span(&self, service: &ServiceConfig) -> Result<Span> {
        let trace_id = TraceId::new(self.generate_id("trace").await)?;
        let span_id = SpanId::new(self.generate_id("span").await.chars().take(16).collect())?;
        
        let operation = service.random_operation();
        let duration = service.generate_latency();
        
        let status = if service.should_error() {
            let error_messages = vec![
                "Connection timeout",
                "Database error",
                "Invalid request",
                "Service unavailable",
                "Rate limit exceeded",
                "Authentication failed",
                "Resource not found",
            ];
            let msg = error_messages[thread_rng().gen_range(0..error_messages.len())];
            SpanStatus::Error(msg.to_string())
        } else {
            SpanStatus::Ok
        };
        
        // Add some realistic attributes
        let mut attributes = HashMap::new();
        attributes.insert("http.method".to_string(), "GET".to_string());
        attributes.insert("http.status_code".to_string(), 
            if status.is_ok() { "200" } else { "500" }.to_string());
        attributes.insert("span.kind".to_string(), "server".to_string());
        
        // Add some tags
        let mut tags = HashMap::new();
        tags.insert("environment".to_string(), "production".to_string());
        tags.insert("version".to_string(), "1.0.0".to_string());
        
        // Add resource attributes
        let mut resource_attributes = HashMap::new();
        resource_attributes.insert("host.name".to_string(), 
            format!("{}-host-{}", service.name.as_str(), thread_rng().gen_range(1..=5)));
        resource_attributes.insert("container.id".to_string(), 
            self.generate_id("container").await.chars().take(12).collect());
        
        Ok(Span::builder()
            .trace_id(trace_id)
            .span_id(span_id)
            .service_name(service.name.clone())
            .operation_name(operation)
            .start_time(SystemTime::now() - duration)
            .duration(duration)
            .status(status)
            .attribute("http.method", "GET")
            .attribute("span.kind", "server")
            .tag("environment", "production")
            .resource_attribute("host.name", format!("{}-host", service.name.as_str()))
            .build()?)
    }

    /// Generate spans continuously at the configured rate.
    pub async fn generate_spans_continuous<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(Span) + Send + 'static,
    {
        *self.running.write().await = true;
        
        // Calculate total RPS across all services
        let total_rps: f64 = self.services.iter().map(|s| s.base_rps).sum();
        let interval_ms = 1000.0 / total_rps;
        
        let mut interval = time::interval(Duration::from_millis(interval_ms as u64));
        
        while *self.running.read().await {
            interval.tick().await;
            
            // Select a service weighted by its RPS
            let service = self.select_service_weighted();
            
            // Generate a span for this service
            match self.generate_span(&service).await {
                Ok(span) => callback(span),
                Err(e) => tracing::warn!("Failed to generate span: {}", e),
            }
        }
        
        Ok(())
    }

    /// Select a service weighted by its RPS.
    fn select_service_weighted(&self) -> ServiceConfig {
        let total_rps: f64 = self.services.iter().map(|s| s.current_rps()).sum();
        let mut selection = thread_rng().gen::<f64>() * total_rps;
        
        for service in &self.services {
            selection -= service.current_rps();
            if selection <= 0.0 {
                return service.clone();
            }
        }
        
        // Fallback to first service
        self.services[0].clone()
    }

    /// Generate a batch of spans for testing.
    pub async fn generate_batch(&self, count: usize) -> Result<Vec<Span>> {
        let mut spans = Vec::with_capacity(count);
        
        for _ in 0..count {
            let service = self.select_service_weighted();
            spans.push(self.generate_span(&service).await?);
        }
        
        Ok(spans)
    }

    /// Stop the continuous generation.
    pub async fn stop(&self) {
        *self.running.write().await = false;
    }

    /// Check if the generator is running.
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

impl Default for SpanGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_single_span() {
        let generator = SpanGenerator::new();
        let service = &generator.services[0];
        let span = generator.generate_span(service).await.unwrap();
        
        assert_eq!(span.service_name, service.name);
        assert!(!span.operation_name.is_empty());
        assert!(span.duration.as_millis() > 0);
    }

    #[tokio::test]
    async fn test_generate_batch() {
        let generator = SpanGenerator::new();
        let spans = generator.generate_batch(100).await.unwrap();
        
        assert_eq!(spans.len(), 100);
        
        // Check that we have spans from multiple services
        let mut services = std::collections::HashSet::new();
        for span in &spans {
            services.insert(span.service_name.as_str());
        }
        assert!(services.len() > 1);
    }

    #[tokio::test]
    async fn test_error_rate() {
        let generator = SpanGenerator::new();
        let spans = generator.generate_batch(1000).await.unwrap();
        
        // Find payment service spans (highest error rate)
        let payment_spans: Vec<_> = spans.iter()
            .filter(|s| s.service_name.as_str() == "payment-service")
            .collect();
        
        assert!(!payment_spans.is_empty());
        
        let error_count = payment_spans.iter()
            .filter(|s| s.status.is_error())
            .count();
        
        let error_rate = error_count as f64 / payment_spans.len() as f64;
        
        // Should be roughly 2% with some variance
        assert!(error_rate > 0.0);
        assert!(error_rate < 0.1); // Should be less than 10%
    }

    #[tokio::test]
    async fn test_latency_distribution() {
        let generator = SpanGenerator::new();
        let service = &generator.services[0]; // api-gateway
        
        let mut latencies = Vec::new();
        for _ in 0..1000 {
            let span = generator.generate_span(service).await.unwrap();
            latencies.push(span.duration.as_millis() as u64);
        }
        
        latencies.sort_unstable();
        
        let p50 = latencies[500];
        let p95 = latencies[950];
        let p99 = latencies[990];
        
        // Check that percentiles are in reasonable ranges
        assert!(p50 <= service.base_p50_ms * 2);
        assert!(p95 <= service.base_p95_ms * 2);
        assert!(p99 <= service.base_p99_ms * 3); // Allow more variance for P99
    }
}
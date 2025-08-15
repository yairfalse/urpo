//! Storage backend for trace data.
//!
//! This module provides the storage interface and implementations
//! for storing and querying trace data efficiently.

pub mod fake_spans;
pub mod aggregator;

use crate::core::{Result, ServiceMetrics, ServiceName, Span, SpanId, TraceId};
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

/// Trait for storage backend implementations.
#[async_trait::async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store a span.
    async fn store_span(&self, span: Span) -> Result<()>;

    /// Get a span by ID.
    async fn get_span(&self, span_id: &SpanId) -> Result<Option<Span>>;

    /// Get all spans for a trace.
    async fn get_trace_spans(&self, trace_id: &TraceId) -> Result<Vec<Span>>;

    /// Get spans for a service within a time window.
    async fn get_service_spans(
        &self,
        service: &ServiceName,
        since: SystemTime,
    ) -> Result<Vec<Span>>;

    /// Get service metrics calculated from stored spans.
    async fn get_service_metrics(&self) -> Result<Vec<ServiceMetrics>>;

    /// Get the total number of stored spans.
    async fn get_span_count(&self) -> Result<usize>;

    /// Remove old spans to enforce storage limits.
    async fn enforce_limits(&self) -> Result<usize>;
}

/// Storage statistics.
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Total number of traces.
    pub trace_count: usize,
    /// Total number of spans.
    pub span_count: usize,
    /// Total number of services.
    pub service_count: usize,
    /// Estimated memory usage in bytes.
    pub memory_bytes: usize,
    /// Oldest span timestamp.
    pub oldest_span: Option<SystemTime>,
    /// Newest span timestamp.
    pub newest_span: Option<SystemTime>,
}

/// In-memory storage implementation with bounded capacity.
pub struct InMemoryStorage {
    /// Spans indexed by span ID.
    spans: Arc<DashMap<SpanId, Span>>,
    /// Trace ID to span IDs mapping.
    traces: Arc<DashMap<TraceId, Vec<SpanId>>>,
    /// Service to span IDs mapping with timestamps for efficient querying.
    services: Arc<DashMap<ServiceName, VecDeque<(SystemTime, SpanId)>>>,
    /// Ordered list of span IDs by insertion time for LRU eviction.
    span_order: Arc<RwLock<VecDeque<(SystemTime, SpanId)>>>,
    /// Maximum number of spans to store.
    max_spans: usize,
    /// Maximum spans per service.
    max_spans_per_service: usize,
}

impl InMemoryStorage {
    /// Create a new in-memory storage with specified limits.
    pub fn new(max_spans: usize) -> Self {
        Self {
            spans: Arc::new(DashMap::new()),
            traces: Arc::new(DashMap::new()),
            services: Arc::new(DashMap::new()),
            span_order: Arc::new(RwLock::new(VecDeque::new())),
            max_spans,
            max_spans_per_service: max_spans / 10, // Allow each service ~10% of total capacity
        }
    }

    /// Remove the oldest spans to stay within limits.
    async fn evict_oldest_spans(&self, count: usize) -> usize {
        let mut span_order = self.span_order.write().await;
        let mut removed = 0;

        for _ in 0..count {
            if let Some((_, span_id)) = span_order.pop_front() {
                // Remove from main storage
                if let Some((_, span)) = self.spans.remove(&span_id) {
                    // Remove from trace index
                    if let Some(mut trace_spans) = self.traces.get_mut(&span.trace_id) {
                        trace_spans.retain(|id| id != &span_id);
                        if trace_spans.is_empty() {
                            drop(trace_spans);
                            self.traces.remove(&span.trace_id);
                        }
                    }

                    // Remove from service index
                    if let Some(mut service_spans) = self.services.get_mut(&span.service_name) {
                        service_spans.retain(|(_, id)| id != &span_id);
                        if service_spans.is_empty() {
                            drop(service_spans);
                            self.services.remove(&span.service_name);
                        }
                    }

                    removed += 1;
                }
            } else {
                break;
            }
        }

        removed
    }

    /// Enforce per-service limits.
    async fn enforce_service_limits(&self) {
        for mut entry in self.services.iter_mut() {
            let service_spans = entry.value_mut();
            while service_spans.len() > self.max_spans_per_service {
                if let Some((_, old_span_id)) = service_spans.pop_front() {
                    // Remove from main storage
                    if let Some((_, span)) = self.spans.remove(&old_span_id) {
                        // Remove from trace index
                        if let Some(mut trace_spans) = self.traces.get_mut(&span.trace_id) {
                            trace_spans.retain(|id| id != &old_span_id);
                        }
                        
                        // Remove from span order
                        let mut span_order = self.span_order.write().await;
                        span_order.retain(|(_, id)| id != &old_span_id);
                    }
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl StorageBackend for InMemoryStorage {
    async fn store_span(&self, span: Span) -> Result<()> {
        let span_id = span.span_id.clone();
        let trace_id = span.trace_id.clone();
        let service_name = span.service_name.clone();
        let start_time = span.start_time;

        // Check if we need to evict spans before storing
        if self.spans.len() >= self.max_spans {
            let to_evict = (self.max_spans / 10).max(1); // Evict 10% when at capacity
            self.evict_oldest_spans(to_evict).await;
        }

        // Store the span
        self.spans.insert(span_id.clone(), span);

        // Update trace index
        self.traces
            .entry(trace_id)
            .or_insert_with(Vec::new)
            .push(span_id.clone());

        // Update service index with timestamp for efficient time-based queries
        self.services
            .entry(service_name)
            .or_insert_with(VecDeque::new)
            .push_back((start_time, span_id.clone()));

        // Add to span order for LRU eviction
        {
            let mut span_order = self.span_order.write().await;
            span_order.push_back((start_time, span_id));
        }

        // Enforce per-service limits
        self.enforce_service_limits().await;

        Ok(())
    }

    async fn get_span(&self, span_id: &SpanId) -> Result<Option<Span>> {
        Ok(self.spans.get(span_id).map(|entry| entry.clone()))
    }

    async fn get_trace_spans(&self, trace_id: &TraceId) -> Result<Vec<Span>> {
        if let Some(span_ids) = self.traces.get(trace_id) {
            let mut spans = Vec::with_capacity(span_ids.len());
            for span_id in span_ids.iter() {
                if let Some(span) = self.spans.get(span_id) {
                    spans.push(span.clone());
                }
            }
            // Sort by start time
            spans.sort_by_key(|s| s.start_time);
            Ok(spans)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_service_spans(
        &self,
        service: &ServiceName,
        since: SystemTime,
    ) -> Result<Vec<Span>> {
        if let Some(service_spans) = self.services.get(service) {
            let mut spans = Vec::new();
            for (timestamp, span_id) in service_spans.iter() {
                if *timestamp >= since {
                    if let Some(span) = self.spans.get(span_id) {
                        spans.push(span.clone());
                    }
                }
            }
            Ok(spans)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_service_metrics(&self) -> Result<Vec<ServiceMetrics>> {
        // Delegate to the aggregator module
        aggregator::calculate_service_metrics(self).await
    }

    async fn get_span_count(&self) -> Result<usize> {
        Ok(self.spans.len())
    }

    async fn enforce_limits(&self) -> Result<usize> {
        let current_count = self.spans.len();
        if current_count > self.max_spans {
            let to_remove = current_count - self.max_spans;
            Ok(self.evict_oldest_spans(to_remove).await)
        } else {
            Ok(0)
        }
    }
}

/// Storage manager for coordinating storage operations.
pub struct StorageManager {
    backend: Arc<dyn StorageBackend>,
}

impl StorageManager {
    /// Create a new storage manager with in-memory backend.
    pub fn new_in_memory(max_spans: usize) -> Self {
        let backend = Arc::new(InMemoryStorage::new(max_spans));
        Self { backend }
    }

    /// Get the storage backend.
    pub fn backend(&self) -> Arc<dyn StorageBackend> {
        self.backend.clone()
    }

    /// Store a span.
    pub async fn store_span(&self, span: Span) -> Result<()> {
        self.backend.store_span(span).await
    }

    /// Get service metrics.
    pub async fn get_service_metrics(&self) -> Result<Vec<ServiceMetrics>> {
        self.backend.get_service_metrics().await
    }

    /// Run periodic cleanup to enforce storage limits.
    pub async fn run_cleanup(&self) -> Result<()> {
        let removed = self.backend.enforce_limits().await?;
        if removed > 0 {
            tracing::debug!("Cleaned up {} old spans", removed);
        }
        Ok(())
    }

    /// Get storage statistics.
    pub async fn get_stats(&self) -> Result<StorageStats> {
        let span_count = self.backend.get_span_count().await?;
        
        // Simple approximation for now
        // In a real implementation, this would be part of the trait
        let avg_span_size = 1024; // bytes per span
        let memory_bytes = span_count * avg_span_size;
        
        Ok(StorageStats {
            trace_count: 0, // Would need a trait method
            span_count,
            service_count: 0, // Would need a trait method
            memory_bytes,
            oldest_span: None,
            newest_span: None,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn create_test_span(trace_num: u32, span_num: u32, service: &str) -> Span {
        Span::builder()
            .trace_id(TraceId::new(format!("trace_{:04}", trace_num)).unwrap())
            .span_id(SpanId::new(format!("span_{:04}", span_num)).unwrap())
            .service_name(ServiceName::new(service.to_string()).unwrap())
            .operation_name(format!("operation_{}", span_num))
            .start_time(SystemTime::now())
            .duration(Duration::from_millis(100))
            .status(crate::core::SpanStatus::Ok)
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_store_and_retrieve_span() {
        let storage = InMemoryStorage::new(100);
        let span = create_test_span(1, 1, "test-service").await;
        let span_id = span.span_id.clone();

        storage.store_span(span.clone()).await.unwrap();
        
        let retrieved = storage.get_span(&span_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().span_id, span_id);
    }

    #[tokio::test]
    async fn test_get_trace_spans() {
        let storage = InMemoryStorage::new(100);
        let trace_id = TraceId::new("trace_0001".to_string()).unwrap();
        
        for i in 1..=3 {
            let mut span = create_test_span(1, i, "test-service").await;
            span.trace_id = trace_id.clone();
            storage.store_span(span).await.unwrap();
        }

        let spans = storage.get_trace_spans(&trace_id).await.unwrap();
        assert_eq!(spans.len(), 3);
    }

    #[tokio::test]
    async fn test_storage_limits() {
        let storage = InMemoryStorage::new(5); // Max 5 spans
        
        for i in 1..=10 {
            let span = create_test_span(i, i, "test-service").await;
            storage.store_span(span).await.unwrap();
        }
        
        // Should have enforced limit
        assert!(storage.spans.len() <= 5);
    }

    #[tokio::test]
    async fn test_service_spans_time_filter() {
        let storage = InMemoryStorage::new(100);
        let service_name = ServiceName::new("test-service".to_string()).unwrap();
        
        // Store some spans
        for i in 1..=5 {
            let span = create_test_span(i, i, "test-service").await;
            storage.store_span(span).await.unwrap();
        }
        
        // Query spans from now (should get all)
        let since = SystemTime::now() - Duration::from_secs(60);
        let spans = storage.get_service_spans(&service_name, since).await.unwrap();
        assert_eq!(spans.len(), 5);
        
        // Query spans from future (should get none)
        let future = SystemTime::now() + Duration::from_secs(60);
        let spans = storage.get_service_spans(&service_name, future).await.unwrap();
        assert_eq!(spans.len(), 0);
    }
}
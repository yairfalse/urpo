//! Storage backend for trace data.
//!
//! This module provides the storage interface and implementations
//! for storing and querying trace data efficiently.

use crate::core::{Result, ServiceMetrics, ServiceName, Span, SpanId, TraceId, UrpoError};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
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

    /// Get spans for a service.
    async fn get_service_spans(&self, service: &ServiceName, limit: usize) -> Result<Vec<Span>>;

    /// Get service metrics.
    async fn get_service_metrics(&self) -> Result<Vec<ServiceMetrics>>;

    /// Remove old traces based on retention policy.
    async fn cleanup(&self, retention: Duration) -> Result<usize>;

    /// Get storage statistics.
    async fn get_stats(&self) -> Result<StorageStats>;
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
    pub oldest_span: Option<DateTime<Utc>>,
    /// Newest span timestamp.
    pub newest_span: Option<DateTime<Utc>>,
}

/// In-memory storage implementation.
pub struct InMemoryStorage {
    /// Spans indexed by span ID.
    spans: Arc<DashMap<SpanId, Span>>,
    /// Trace ID to span IDs mapping.
    traces: Arc<DashMap<TraceId, Vec<SpanId>>>,
    /// Service to span IDs mapping.
    services: Arc<DashMap<ServiceName, VecDeque<SpanId>>>,
    /// Service metrics.
    metrics: Arc<RwLock<HashMap<ServiceName, ServiceMetrics>>>,
    /// Maximum number of traces to store.
    max_traces: usize,
    /// Maximum spans per service.
    max_spans_per_service: usize,
}

impl InMemoryStorage {
    /// Create a new in-memory storage.
    pub fn new(max_traces: usize) -> Self {
        Self {
            spans: Arc::new(DashMap::new()),
            traces: Arc::new(DashMap::new()),
            services: Arc::new(DashMap::new()),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            max_traces,
            max_spans_per_service: 1000,
        }
    }

    /// Update service metrics with a new span.
    async fn update_metrics(&self, span: &Span) {
        let mut metrics = self.metrics.write().await;
        let entry = metrics
            .entry(span.service_name.clone())
            .or_insert_with(|| ServiceMetrics {
                service_name: span.service_name.clone(),
                span_count: 0,
                error_count: 0,
                avg_duration_ms: 0,
                p50_latency_ms: 0,
                p95_latency_ms: 0,
                p99_latency_ms: 0,
                rps: 0.0,
                last_updated: Utc::now(),
            });

        entry.span_count += 1;
        if span.is_error() {
            entry.error_count += 1;
        }

        // Update average duration (simplified - in production, use proper statistics)
        let span_duration_ms = span.duration().as_millis() as u64;
        let current_total = entry.avg_duration_ms * (entry.span_count - 1);
        let new_total = current_total + span_duration_ms;
        entry.avg_duration_ms = new_total / entry.span_count;

        // Update percentiles (simplified - in production, use proper percentile calculation)
        entry.p50_latency_ms = entry.avg_duration_ms;
        entry.p95_latency_ms = (entry.avg_duration_ms as f64 * 1.5) as u64;
        entry.p99_latency_ms = (entry.avg_duration_ms as f64 * 2.0) as u64;

        // Update RPS (simplified - in production, track time windows)
        let time_window = (Utc::now() - entry.last_updated).num_seconds().max(1) as f64;
        entry.rps = entry.span_count as f64 / time_window;
        
        entry.last_updated = Utc::now();
    }

    /// Enforce storage limits.
    async fn enforce_limits(&self) {
        // Limit number of traces
        if self.traces.len() > self.max_traces {
            let to_remove = self.traces.len() - self.max_traces;
            let mut removed = 0;
            
            // Remove oldest traces (simplified - in production, track insertion order)
            for item in self.traces.iter() {
                if removed >= to_remove {
                    break;
                }
                
                let trace_id = item.key().clone();
                if let Some((_, span_ids)) = self.traces.remove(&trace_id) {
                    for span_id in span_ids {
                        self.spans.remove(&span_id);
                    }
                    removed += 1;
                }
            }
        }

        // Limit spans per service
        for mut item in self.services.iter_mut() {
            let spans = item.value_mut();
            while spans.len() > self.max_spans_per_service {
                if let Some(old_span_id) = spans.pop_front() {
                    self.spans.remove(&old_span_id);
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

        // Update metrics
        self.update_metrics(&span).await;

        // Store the span
        self.spans.insert(span_id.clone(), span);

        // Update trace index
        self.traces
            .entry(trace_id)
            .or_insert_with(Vec::new)
            .push(span_id.clone());

        // Update service index
        self.services
            .entry(service_name)
            .or_insert_with(VecDeque::new)
            .push_back(span_id);

        // Enforce limits
        self.enforce_limits().await;

        Ok(())
    }

    async fn get_span(&self, span_id: &SpanId) -> Result<Option<Span>> {
        Ok(self.spans.get(span_id).map(|entry| entry.clone()))
    }

    async fn get_trace_spans(&self, trace_id: &TraceId) -> Result<Vec<Span>> {
        if let Some(span_ids) = self.traces.get(trace_id) {
            let mut spans = Vec::new();
            for span_id in span_ids.iter() {
                if let Some(span) = self.spans.get(span_id) {
                    spans.push(span.clone());
                }
            }
            Ok(spans)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_service_spans(&self, service: &ServiceName, limit: usize) -> Result<Vec<Span>> {
        if let Some(span_ids) = self.services.get(service) {
            let mut spans = Vec::new();
            for (i, span_id) in span_ids.iter().rev().enumerate() {
                if i >= limit {
                    break;
                }
                if let Some(span) = self.spans.get(span_id) {
                    spans.push(span.clone());
                }
            }
            Ok(spans)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_service_metrics(&self) -> Result<Vec<ServiceMetrics>> {
        let metrics = self.metrics.read().await;
        Ok(metrics.values().cloned().collect())
    }

    async fn cleanup(&self, retention: Duration) -> Result<usize> {
        let cutoff = Utc::now() - retention;
        let mut removed = 0;

        // Find and remove old spans
        let mut to_remove = Vec::new();
        for item in self.spans.iter() {
            if item.value().end_time < cutoff {
                to_remove.push(item.key().clone());
            }
        }

        for span_id in to_remove {
            if let Some((_, span)) = self.spans.remove(&span_id) {
                // Remove from trace index
                if let Some(mut trace_spans) = self.traces.get_mut(&span.trace_id) {
                    trace_spans.retain(|id| id != &span_id);
                }

                // Remove from service index
                if let Some(mut service_spans) = self.services.get_mut(&span.service_name) {
                    service_spans.retain(|id| id != &span_id);
                }

                removed += 1;
            }
        }

        // Clean up empty traces
        self.traces.retain(|_, spans| !spans.is_empty());

        // Clean up empty services
        self.services.retain(|_, spans| !spans.is_empty());

        Ok(removed)
    }

    async fn get_stats(&self) -> Result<StorageStats> {
        let span_count = self.spans.len();
        let trace_count = self.traces.len();
        let service_count = self.services.len();

        // Estimate memory usage (rough approximation)
        let avg_span_size = 1024; // bytes
        let memory_bytes = span_count * avg_span_size;

        // Find oldest and newest spans
        let mut oldest: Option<DateTime<Utc>> = None;
        let mut newest: Option<DateTime<Utc>> = None;

        for item in self.spans.iter() {
            let span = item.value();
            match oldest {
                None => oldest = Some(span.start_time),
                Some(current) if span.start_time < current => oldest = Some(span.start_time),
                _ => {}
            }
            match newest {
                None => newest = Some(span.end_time),
                Some(current) if span.end_time > current => newest = Some(span.end_time),
                _ => {}
            }
        }

        Ok(StorageStats {
            trace_count,
            span_count,
            service_count,
            memory_bytes,
            oldest_span: oldest,
            newest_span: newest,
        })
    }
}

/// Storage manager for coordinating storage operations.
pub struct StorageManager {
    backend: Arc<dyn StorageBackend>,
    max_memory_mb: usize,
}

impl StorageManager {
    /// Create a new storage manager with in-memory backend.
    pub fn new_in_memory(max_traces: usize, max_memory_mb: usize) -> Self {
        let backend = Arc::new(InMemoryStorage::new(max_traces));
        Self {
            backend,
            max_memory_mb,
        }
    }

    /// Get the storage backend.
    pub fn backend(&self) -> Arc<dyn StorageBackend> {
        self.backend.clone()
    }

    /// Check if memory limit is exceeded.
    pub async fn check_memory_limit(&self) -> Result<()> {
        let stats = self.backend.get_stats().await?;
        let memory_mb = stats.memory_bytes / (1024 * 1024);
        
        if memory_mb > self.max_memory_mb {
            return Err(UrpoError::MemoryLimitExceeded {
                current: memory_mb,
                limit: self.max_memory_mb,
            });
        }
        
        Ok(())
    }

    /// Run periodic cleanup.
    pub async fn run_cleanup(&self, retention: Duration) -> Result<()> {
        let removed = self.backend.cleanup(retention).await?;
        if removed > 0 {
            tracing::info!("Cleaned up {} old spans", removed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{SpanKind, SpanStatus};
    use std::collections::HashMap;

    async fn create_test_span(trace_num: u32, span_num: u32) -> Span {
        Span {
            span_id: SpanId::new(format!("{:0>16}", span_num)).unwrap(),
            trace_id: TraceId::new(format!("{:0>32}", trace_num)).unwrap(),
            parent_span_id: None,
            service_name: ServiceName::new("test-service".to_string()).unwrap(),
            operation_name: format!("operation-{}", span_num),
            kind: SpanKind::Server,
            start_time: Utc::now(),
            end_time: Utc::now() + Duration::milliseconds(100),
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_store_and_retrieve_span() {
        let storage = InMemoryStorage::new(100);
        let span = create_test_span(1, 1).await;
        let span_id = span.span_id.clone();

        storage.store_span(span.clone()).await.unwrap();
        
        let retrieved = storage.get_span(&span_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().span_id, span_id);
    }

    #[tokio::test]
    async fn test_get_trace_spans() {
        let storage = InMemoryStorage::new(100);
        let trace_id = TraceId::new(format!("{:0>32}", 1)).unwrap();
        
        for i in 1..=3 {
            let mut span = create_test_span(1, i).await;
            span.trace_id = trace_id.clone();
            storage.store_span(span).await.unwrap();
        }

        let spans = storage.get_trace_spans(&trace_id).await.unwrap();
        assert_eq!(spans.len(), 3);
    }

    #[tokio::test]
    async fn test_service_metrics() {
        let storage = InMemoryStorage::new(100);
        let service_name = ServiceName::new("test-service".to_string()).unwrap();
        
        for i in 1..=5 {
            let mut span = create_test_span(i, i).await;
            if i == 3 {
                span.status = SpanStatus::Error("test error".to_string());
            }
            storage.store_span(span).await.unwrap();
        }

        let metrics = storage.get_service_metrics().await.unwrap();
        assert_eq!(metrics.len(), 1);
        
        let service_metrics = &metrics[0];
        assert_eq!(service_metrics.span_count, 5);
        assert_eq!(service_metrics.error_count, 1);
    }

    #[tokio::test]
    async fn test_cleanup() {
        let storage = InMemoryStorage::new(100);
        
        // Create old span
        let mut old_span = create_test_span(1, 1).await;
        old_span.end_time = Utc::now() - Duration::hours(2);
        storage.store_span(old_span).await.unwrap();
        
        // Create recent span
        let recent_span = create_test_span(2, 2).await;
        storage.store_span(recent_span).await.unwrap();
        
        // Cleanup with 1 hour retention
        let removed = storage.cleanup(Duration::hours(1)).await.unwrap();
        assert_eq!(removed, 1);
        
        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.span_count, 1);
    }

    #[tokio::test]
    async fn test_storage_limits() {
        let storage = InMemoryStorage::new(2); // Max 2 traces
        
        for i in 1..=5 {
            let span = create_test_span(i, i).await;
            storage.store_span(span).await.unwrap();
        }
        
        // Should have enforced limit
        assert!(storage.traces.len() <= 2);
    }

    #[tokio::test]
    async fn test_storage_manager_memory_check() {
        let manager = StorageManager::new_in_memory(100, 1); // 1MB limit
        
        // Store many spans to exceed limit
        for i in 1..=1000 {
            let span = create_test_span(i, i).await;
            manager.backend().store_span(span).await.unwrap();
        }
        
        // Memory limit should be exceeded
        let result = manager.check_memory_limit().await;
        assert!(result.is_err());
    }
}
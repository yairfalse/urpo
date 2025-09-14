//! Storage backend trait and implementations.

use super::{StorageHealth, StorageStats, TraceInfo};
use crate::core::{Result, ServiceMetrics, ServiceName, Span, SpanId, TraceId};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

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

    /// Get list of active service names.
    async fn list_services(&self) -> Result<Vec<ServiceName>>;

    /// Get detailed storage statistics.
    async fn get_storage_stats(&self) -> Result<StorageStats>;

    /// Perform emergency cleanup.
    async fn emergency_cleanup(&self) -> Result<usize>;

    /// Check storage health.
    fn get_health(&self) -> StorageHealth;

    /// Enable downcasting for concrete types.
    fn as_any(&self) -> &dyn std::any::Any;

    /// List recent traces with optional filtering.
    async fn list_recent_traces(
        &self,
        limit: usize,
        service_filter: Option<&ServiceName>,
    ) -> Result<Vec<TraceInfo>>;

    /// Search traces by operation name or attributes.
    async fn search_traces(&self, query: &str, limit: usize) -> Result<Vec<TraceInfo>>;

    /// Get traces with errors.
    async fn get_error_traces(&self, limit: usize) -> Result<Vec<TraceInfo>>;

    /// Get slow traces (P99 latency).
    async fn get_slow_traces(&self, threshold: Duration, limit: usize) -> Result<Vec<TraceInfo>>;

    /// List traces with filtering options.
    async fn list_traces(
        &self,
        service: Option<&str>,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: usize,
    ) -> Result<Vec<TraceInfo>>;

    /// Get service metrics as a map.
    async fn get_service_metrics_map(&self) -> Result<HashMap<ServiceName, ServiceMetrics>>;

    /// Search spans by query with filters.
    async fn search_spans(
        &self,
        query: &str,
        service: Option<&str>,
        attribute_key: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Span>>;

    /// Get storage statistics for health check.
    async fn get_stats(&self) -> Result<StorageStats>;
}

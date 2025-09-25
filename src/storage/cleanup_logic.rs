//! Memory cleanup and management utilities for storage backends.

use super::StorageHealth;
use crate::core::{ServiceName, Span, SpanId, TraceId};
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime};

/// Memory cleanup configuration.
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    /// Maximum memory usage in bytes before cleanup.
    pub max_memory_bytes: usize,
    /// Warning threshold (0.0 - 1.0).
    pub warning_threshold: f64,
    /// Critical threshold (0.0 - 1.0).
    pub critical_threshold: f64,
    /// Emergency threshold (0.0 - 1.0).
    pub emergency_threshold: f64,
    /// Span retention period.
    pub retention_period: Duration,
    /// Cleanup interval.
    pub cleanup_interval: Duration,
    /// Minimum spans to keep per service.
    pub min_spans_per_service: usize,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 512 * 1024 * 1024, // 512MB
            warning_threshold: 0.7,
            critical_threshold: 0.85,
            emergency_threshold: 0.95,
            retention_period: Duration::from_secs(3600), // 1 hour
            cleanup_interval: Duration::from_secs(30),
            min_spans_per_service: 100,
        }
    }
}

/// Performance and monitoring counters.
#[derive(Debug)]
pub struct StorageCounters {
    /// Total spans processed.
    pub spans_processed: AtomicU64,
    /// Processing errors.
    pub processing_errors: AtomicU64,
    /// Cleanup operations performed.
    pub cleanup_operations: AtomicU64,
    /// Memory bytes estimate.
    pub memory_bytes: AtomicUsize,
    /// Spans evicted.
    pub spans_evicted: AtomicU64,
    /// Start time for rate calculations.
    pub start_time: Instant,
}

impl Default for StorageCounters {
    fn default() -> Self {
        Self {
            spans_processed: AtomicU64::new(0),
            processing_errors: AtomicU64::new(0),
            cleanup_operations: AtomicU64::new(0),
            memory_bytes: AtomicUsize::new(0),
            spans_evicted: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }
}

/// Helper trait for cleanup operations.
pub trait CleanupOperations {
    /// Get current memory pressure.
    fn get_memory_pressure(&self) -> f64;

    /// Determine storage health based on memory usage.
    fn get_health_status(&self) -> StorageHealth;

    /// Check if cleanup is needed.
    fn should_cleanup(&self, last_cleanup: Instant, cleanup_config: &CleanupConfig) -> bool;
}

/// Memory estimation for spans.
pub fn estimate_span_memory(span: &Span) -> usize {
    // Base size (fields)
    let mut size = std::mem::size_of::<Span>();

    // String allocations
    size += span.trace_id.as_str().len();
    size += span.span_id.as_str().len();
    size += span.service_name.as_str().len();
    size += span.operation_name.len();

    // Attributes (AttributeMap is a HashMap internally)
    size += span.attributes.len() * std::mem::size_of::<(String, String)>();
    for (k, v) in span.attributes.iter() {
        size += k.len() + v.len();
    }

    // Tags
    size += span.tags.len() * std::mem::size_of::<(String, String)>();
    for (k, v) in span.tags.iter() {
        size += k.len() + v.len();
    }

    size
}

/// Batch remove helper for efficient cleanup.
pub fn batch_remove_spans(
    spans: &DashMap<SpanId, Span>,
    traces: &DashMap<TraceId, Vec<SpanId>>,
    services: &DashMap<ServiceName, VecDeque<(SystemTime, SpanId)>>,
    span_ids: &[SpanId],
    counters: &StorageCounters,
) -> usize {
    let mut removed = 0;

    for span_id in span_ids {
        if let Some((_, span)) = spans.remove(span_id) {
            // Update memory estimate
            let span_memory = estimate_span_memory(&span);
            counters
                .memory_bytes
                .fetch_sub(span_memory, Ordering::Relaxed);

            // Remove from trace index
            if let Some(mut trace_spans) = traces.get_mut(&span.trace_id) {
                trace_spans.retain(|id| id != span_id);
                if trace_spans.is_empty() {
                    drop(trace_spans);
                    traces.remove(&span.trace_id);
                }
            }

            // Remove from service index
            if let Some(mut service_spans) = services.get_mut(&span.service_name) {
                service_spans.retain(|(_, id)| id != span_id);
            }

            removed += 1;
        }
    }

    counters
        .spans_evicted
        .fetch_add(removed as u64, Ordering::Relaxed);
    removed
}

/// Macro for creating common metric aggregation functions.
#[macro_export]
macro_rules! aggregate_metrics {
    ($spans:expr, $field:ident, $op:tt) => {{
        $spans.iter().$op(|span| span.$field).unwrap_or_default()
    }};
}

/// Macro for atomic counter operations.
#[macro_export]
macro_rules! update_counter {
    ($counter:expr, add $value:expr) => {
        $counter.fetch_add($value, std::sync::atomic::Ordering::Relaxed)
    };
    ($counter:expr, sub $value:expr) => {
        $counter.fetch_sub($value, std::sync::atomic::Ordering::Relaxed)
    };
    ($counter:expr, set $value:expr) => {
        $counter.store($value, std::sync::atomic::Ordering::Relaxed)
    };
    ($counter:expr, get) => {
        $counter.load(std::sync::atomic::Ordering::Relaxed)
    };
}

/// Macro for creating trace info from spans.
#[macro_export]
macro_rules! create_trace_info {
    ($trace_id:expr, $spans:expr) => {{
        use $crate::core::ServiceName;
        use $crate::storage::TraceInfo;

        if $spans.is_empty() {
            None
        } else {
            let start_time = $spans.iter().map(|s| s.start_time).min().unwrap();
            let duration = $spans
                .iter()
                .map(|s| s.duration)
                .max()
                .unwrap_or_else(|| Duration::from_secs(0));
            let has_error = $spans.iter().any(|s| s.is_error());
            let root_span = $spans.iter().find(|s| s.parent_span_id.is_none());
            let services: Vec<ServiceName> = $spans
                .iter()
                .map(|s| s.service_name.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            Some(TraceInfo {
                trace_id: $trace_id.clone(),
                root_service: root_span
                    .map(|s| s.service_name.clone())
                    .unwrap_or_else(|| ServiceName::new("unknown".to_string()).unwrap()),
                root_operation: root_span
                    .map(|s| s.operation_name.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
                span_count: $spans.len(),
                duration,
                start_time,
                has_error,
                services,
            })
        }
    }};
}

/// Macro for repetitive search implementations.
#[macro_export]
macro_rules! impl_search {
    ($self:expr, $filter:expr, $limit:expr) => {{
        use $crate::storage::TraceInfo;

        let traces = $self
            .traces
            .iter()
            .filter_map(|entry| {
                let trace_id = entry.key();
                let span_ids = entry.value();

                let spans: Vec<_> = span_ids
                    .iter()
                    .filter_map(|id| $self.spans.get(id).map(|s| s.clone()))
                    .collect();

                if spans.is_empty() || !$filter(&spans) {
                    return None;
                }

                create_trace_info!(trace_id, spans)
            })
            .collect::<Vec<TraceInfo>>();

        // Sort by start time (newest first)
        let mut sorted = traces;
        sorted.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        Ok::<Vec<TraceInfo>, $crate::core::UrpoError>(
            sorted.into_iter().take($limit).collect::<Vec<TraceInfo>>(),
        )
    }};
}

/// Macro for removing span from indices
#[macro_export]
macro_rules! remove_span_indices {
    ($self:expr, $span:expr, $span_id:expr) => {{
        // Update memory tracking
        let memory_freed = $self.estimate_span_memory($span);
        update_counter!($self.counters.memory_bytes, sub memory_freed);

        // Remove from trace index
        if let Some(mut trace_spans) = $self.traces.get_mut(&$span.trace_id) {
            trace_spans.retain(|id| id != $span_id);
            if trace_spans.is_empty() {
                drop(trace_spans);
                $self.traces.remove(&$span.trace_id);
            }
        }

        // Remove from service index
        if let Some(mut service_spans) = $self.services.get_mut(&$span.service_name) {
            service_spans.retain(|(_, id)| id != $span_id);
            if service_spans.is_empty() {
                drop(service_spans);
                $self.services.remove(&$span.service_name);
            }
        }
    }};
}

/// Macro for batch processing with yield
#[macro_export]
macro_rules! batch_process {
    ($items:expr, $batch_size:expr, $process:expr) => {{
        for chunk in $items.chunks($batch_size) {
            for item in chunk {
                $process(item);
            }
            tokio::task::yield_now().await;
        }
    }};
}

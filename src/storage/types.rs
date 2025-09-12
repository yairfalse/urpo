//! Storage data types and structures.

use crate::core::{ServiceName, TraceId};
use std::time::{SystemTime, Duration};

/// Information about a trace for listing purposes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceInfo {
    /// Unique trace identifier.
    pub trace_id: TraceId,
    /// Root service name.
    pub root_service: ServiceName,
    /// Root operation name.
    pub root_operation: String,
    /// Total number of spans in the trace.
    pub span_count: usize,
    /// Total duration of the trace.
    pub duration: Duration,
    /// Start time of the trace.
    pub start_time: SystemTime,
    /// Whether the trace contains errors.
    pub has_error: bool,
    /// Services involved in the trace.
    pub services: Vec<ServiceName>,
}

/// Storage statistics with comprehensive monitoring.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageStats {
    /// Total number of traces.
    pub trace_count: usize,
    /// Total number of spans.
    pub span_count: usize,
    /// Total number of services.
    pub service_count: usize,
    /// Estimated memory usage in bytes.
    pub memory_bytes: usize,
    /// Memory usage in MB for display.
    pub memory_mb: f64,
    /// Memory pressure level (0.0 = normal, 1.0 = critical).
    pub memory_pressure: f64,
    /// Oldest span timestamp.
    pub oldest_span: Option<SystemTime>,
    /// Newest span timestamp.
    pub newest_span: Option<SystemTime>,
    /// Spans processed per second.
    pub processing_rate: f64,
    /// Error rate for processing.
    pub error_rate: f64,
    /// Number of cleanup operations performed.
    pub cleanup_count: u64,
    /// Last cleanup timestamp.
    pub last_cleanup: Option<SystemTime>,
    /// Storage health status.
    pub health_status: StorageHealth,
    /// Uptime in seconds.
    pub uptime_seconds: u64,
}

/// Health status of the storage system.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StorageHealth {
    /// Storage is operating normally.
    Healthy,
    /// Storage is experiencing degraded performance.
    Degraded,
    /// Storage is experiencing critical issues.
    Critical,
    /// Storage is offline or unavailable.
    Offline,
}

/// Configuration for cleanup operations.
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    /// Maximum number of spans to keep in storage.
    pub max_spans: usize,
    /// Maximum memory usage in bytes.
    pub max_memory_bytes: usize,
    /// Maximum age of spans before cleanup.
    pub max_age: Duration,
    /// Whether to enable aggressive cleanup under memory pressure.
    pub aggressive_cleanup: bool,
    /// Minimum number of spans to keep per service.
    pub min_spans_per_service: usize,
    /// Warning threshold for memory usage (0.0-1.0).
    pub warning_threshold: f64,
    /// Critical threshold for memory usage (0.0-1.0).
    pub critical_threshold: f64,
    /// Emergency threshold for memory usage (0.0-1.0).
    pub emergency_threshold: f64,
    /// How long to retain traces.
    pub retention_period: Duration,
    /// How often to run cleanup operations.
    pub cleanup_interval: Duration,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            max_spans: 1_000_000,
            max_memory_bytes: 2_000_000_000, // 2GB
            max_age: Duration::from_secs(3600), // 1 hour
            aggressive_cleanup: true,
            min_spans_per_service: 100,
            warning_threshold: 0.7,
            critical_threshold: 0.85,
            emergency_threshold: 0.95,
            retention_period: Duration::from_secs(24 * 3600), // 24 hours
            cleanup_interval: Duration::from_secs(60), // 1 minute
        }
    }
}
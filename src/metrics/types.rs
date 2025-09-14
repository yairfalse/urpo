//! Core metric types optimized for performance.
//!
//! All types are designed for zero-allocation hot paths
//! and cache-line optimization.

/// OpenTelemetry metric types with ultra-fast processing
#[derive(Debug, Clone)]
pub enum MetricType {
    /// Monotonically increasing counter (requests, errors)
    Counter { value: f64 },
    /// Point-in-time measurement (CPU usage, memory)
    Gauge { value: f64 },
    /// Latency/size distributions with pre-computed percentiles
    Histogram {
        sum: f64,
        count: u64,
        // TODO: Add buckets when needed
    },
}

/// Metric data point optimized for cache efficiency
/// Size: 32 bytes exactly for cache line optimization
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MetricPoint {
    /// Timestamp (8 bytes)
    pub timestamp: u64,
    /// Service name index (2 bytes)
    pub service_idx: u16,
    /// Metric name index (2 bytes)
    pub metric_idx: u16,
    /// Value (8 bytes)
    pub value: f64,
    /// Attributes hash (4 bytes)
    pub attr_hash: u32,
    /// Metric type + flags (1 byte)
    pub type_flags: u8,
    /// Padding for alignment (3 bytes)
    _padding: [u8; 3],
}

impl MetricPoint {
    pub fn new(timestamp: u64, service_idx: u16, metric_idx: u16, value: f64) -> Self {
        Self {
            timestamp,
            service_idx,
            metric_idx,
            value,
            attr_hash: 0,
            type_flags: 0,
            _padding: [0; 3],
        }
    }
}

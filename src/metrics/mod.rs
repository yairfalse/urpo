//! Ultra-fast OpenTelemetry metrics implementation.
//!
//! Performance targets:
//! - <100ns per metric ingestion (10x faster with SIMD)
//! - <25MB memory for 1M metric points
//! - <1ms service health queries
//! - Lock-free operations throughout

pub mod aggregator;
pub mod ring_buffer;
pub mod storage;
pub mod string_pool;
pub mod types;

pub use aggregator::{AggregationResult, MetricsAggregator};
pub use ring_buffer::MetricRingBuffer;
pub use storage::{MetricStorage, ServiceHealth};
pub use types::{HistogramBucket, MetricPoint, MetricType, Quantile};

#[cfg(test)]
mod integration_test;

#[cfg(test)]
mod tests {
    use super::types::*;

    #[test]
    fn test_metric_type_counter() {
        let counter = MetricType::Counter { value: 42.0 };

        match counter {
            MetricType::Counter { value } => assert_eq!(value, 42.0),
            _ => panic!("Expected counter"),
        }
    }

    #[test]
    fn test_metric_type_gauge() {
        let gauge = MetricType::Gauge { value: 73.5 };

        match gauge {
            MetricType::Gauge { value } => assert_eq!(value, 73.5),
            _ => panic!("Expected gauge"),
        }
    }

    #[test]
    fn test_metric_point_size() {
        // Ensure struct is cache-line optimized (32 bytes or less)
        assert!(std::mem::size_of::<MetricPoint>() <= 32);
    }

    #[test]
    fn test_metric_point_creation() {
        let point = MetricPoint::new(
            1234567890, // timestamp
            1,          // service_idx
            2,          // metric_idx
            99.9,       // value
        );

        assert_eq!(point.timestamp, 1234567890);
        assert_eq!(point.value, 99.9);
        assert_eq!(point.service_idx, 1);
        assert_eq!(point.metric_idx, 2);
    }
}

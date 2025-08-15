//! Core business logic and domain models for Urpo.
//!
//! This module contains the fundamental types and logic that power
//! the OTEL trace exploration functionality.

pub mod error;

pub use error::{Result, UrpoError};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a unique trace identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(String);

impl TraceId {
    /// Create a new TraceId with validation.
    pub fn new(id: String) -> Result<Self> {
        if id.is_empty() {
            return Err(UrpoError::parse("TraceId cannot be empty"));
        }
        if id.len() != 32 {
            return Err(UrpoError::parse(format!(
                "TraceId must be 32 characters, got {}",
                id.len()
            )));
        }
        Ok(TraceId(id))
    }

    /// Get the underlying string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Represents a unique span identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(String);

impl SpanId {
    /// Create a new SpanId with validation.
    pub fn new(id: String) -> Result<Self> {
        if id.is_empty() {
            return Err(UrpoError::parse("SpanId cannot be empty"));
        }
        if id.len() != 16 {
            return Err(UrpoError::parse(format!(
                "SpanId must be 16 characters, got {}",
                id.len()
            )));
        }
        Ok(SpanId(id))
    }

    /// Get the underlying string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Service name wrapper for type safety.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceName(String);

impl ServiceName {
    /// Create a new ServiceName with validation.
    pub fn new(name: String) -> Result<Self> {
        if name.is_empty() {
            return Err(UrpoError::parse("ServiceName cannot be empty"));
        }
        if name.len() > 256 {
            return Err(UrpoError::parse(format!(
                "ServiceName too long: {} characters (max 256)",
                name.len()
            )));
        }
        Ok(ServiceName(name))
    }

    /// Get the underlying string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Represents the kind of a span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanKind {
    /// Internal operation within an application.
    Internal,
    /// Server-side handling of a synchronous RPC or HTTP request.
    Server,
    /// Client-side of a synchronous RPC or HTTP request.
    Client,
    /// Producer of an asynchronous message.
    Producer,
    /// Consumer of an asynchronous message.
    Consumer,
}

/// Status of a span execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStatus {
    /// Operation completed successfully.
    Ok,
    /// Operation failed with an error.
    Error(String),
    /// Status is not set.
    Unset,
}

/// Represents a single span in a distributed trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Unique identifier for this span.
    pub span_id: SpanId,
    /// Trace this span belongs to.
    pub trace_id: TraceId,
    /// Parent span ID, if this is not a root span.
    pub parent_span_id: Option<SpanId>,
    /// Service that generated this span.
    pub service_name: ServiceName,
    /// Operation name.
    pub operation_name: String,
    /// Kind of span.
    pub kind: SpanKind,
    /// Start time of the span.
    pub start_time: DateTime<Utc>,
    /// End time of the span.
    pub end_time: DateTime<Utc>,
    /// Span execution status.
    pub status: SpanStatus,
    /// Span attributes.
    pub attributes: HashMap<String, String>,
    /// Span events.
    pub events: Vec<SpanEvent>,
}

impl Span {
    /// Calculate the duration of this span.
    pub fn duration(&self) -> std::time::Duration {
        let nanos = (self.end_time - self.start_time).num_nanoseconds().unwrap_or(0);
        std::time::Duration::from_nanos(nanos.max(0) as u64)
    }

    /// Check if this span represents an error.
    pub fn is_error(&self) -> bool {
        matches!(self.status, SpanStatus::Error(_))
    }

    /// Check if this is a root span.
    pub fn is_root(&self) -> bool {
        self.parent_span_id.is_none()
    }
}

/// Represents an event within a span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    /// Event name.
    pub name: String,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Event attributes.
    pub attributes: HashMap<String, String>,
}

/// Aggregated service metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    /// Service name.
    pub service_name: ServiceName,
    /// Total number of spans.
    pub span_count: u64,
    /// Number of error spans.
    pub error_count: u64,
    /// Average span duration in milliseconds.
    pub avg_duration_ms: u64,
    /// P50 latency in milliseconds.
    pub p50_latency_ms: u64,
    /// P95 latency in milliseconds.
    pub p95_latency_ms: u64,
    /// P99 latency in milliseconds.
    pub p99_latency_ms: u64,
    /// Requests per second.
    pub rps: f64,
    /// Last update timestamp.
    pub last_updated: DateTime<Utc>,
}

impl ServiceMetrics {
    /// Calculate error rate as a percentage.
    pub fn error_rate(&self) -> f64 {
        if self.span_count == 0 {
            0.0
        } else {
            (self.error_count as f64 / self.span_count as f64) * 100.0
        }
    }

    /// Check if the service is healthy based on error rate.
    pub fn is_healthy(&self, error_threshold: f64) -> bool {
        self.error_rate() < error_threshold
    }
}

/// Configuration for the Urpo application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// GRPC port for OTEL receiver.
    pub grpc_port: u16,
    /// HTTP port for OTEL receiver.
    pub http_port: u16,
    /// Maximum memory usage in MB.
    pub max_memory_mb: usize,
    /// Maximum number of traces to store.
    pub max_traces: usize,
    /// Sampling rate (0.0 to 1.0).
    pub sampling_rate: f64,
    /// Enable debug logging.
    pub debug: bool,
    /// Retention period for traces in seconds.
    pub retention_seconds: u64,
}

impl Config {
    /// Create a new configuration with validation.
    pub fn new() -> Result<Self> {
        Self::default_config()
    }
    
    /// Get the retention period as a chrono::Duration.
    pub fn retention(&self) -> chrono::Duration {
        chrono::Duration::seconds(self.retention_seconds as i64)
    }

    /// Get the default configuration.
    pub fn default_config() -> Result<Self> {
        let config = Config {
            grpc_port: 4317,
            http_port: 4318,
            max_memory_mb: 512,
            max_traces: 10000,
            sampling_rate: 1.0,
            debug: false,
            retention_seconds: 3600, // 1 hour
        };
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        if self.sampling_rate < 0.0 || self.sampling_rate > 1.0 {
            return Err(UrpoError::InvalidSamplingRate(self.sampling_rate));
        }
        if self.max_memory_mb == 0 {
            return Err(UrpoError::config("max_memory_mb must be greater than 0"));
        }
        if self.max_traces == 0 {
            return Err(UrpoError::config("max_traces must be greater than 0"));
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::default_config().expect("default config should be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_id_validation() {
        let valid_id = "a".repeat(32);
        assert!(TraceId::new(valid_id).is_ok());

        let empty_id = String::new();
        assert!(TraceId::new(empty_id).is_err());

        let wrong_length = "a".repeat(31);
        assert!(TraceId::new(wrong_length).is_err());
    }

    #[test]
    fn test_span_id_validation() {
        let valid_id = "b".repeat(16);
        assert!(SpanId::new(valid_id).is_ok());

        let empty_id = String::new();
        assert!(SpanId::new(empty_id).is_err());

        let wrong_length = "b".repeat(15);
        assert!(SpanId::new(wrong_length).is_err());
    }

    #[test]
    fn test_service_name_validation() {
        let valid_name = "my-service".to_string();
        assert!(ServiceName::new(valid_name).is_ok());

        let empty_name = String::new();
        assert!(ServiceName::new(empty_name).is_err());

        let too_long = "a".repeat(257);
        assert!(ServiceName::new(too_long).is_err());
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        config.sampling_rate = 1.5;
        assert!(config.validate().is_err());

        config.sampling_rate = -0.1;
        assert!(config.validate().is_err());

        config.sampling_rate = 0.5;
        config.max_memory_mb = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_service_metrics_error_rate() {
        let metrics = ServiceMetrics {
            service_name: ServiceName::new("test".to_string()).unwrap(),
            span_count: 100,
            error_count: 10,
            avg_duration_ms: 100,
            p50_latency_ms: 80,
            p95_latency_ms: 150,
            p99_latency_ms: 200,
            rps: 10.0,
            last_updated: Utc::now(),
        };

        assert_eq!(metrics.error_rate(), 10.0);
        assert!(metrics.is_healthy(15.0));
        assert!(!metrics.is_healthy(5.0));
    }
}
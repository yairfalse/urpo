//! Core business logic and domain models for Urpo.
//!
//! This module contains the fundamental types and logic that power
//! the OTEL trace exploration functionality.

pub mod error;
pub mod types;

// Re-export commonly used types
pub use error::{Result, UrpoError};
pub use types::{
    ServiceMetrics, ServiceName, Span, SpanBuilder, SpanId, SpanStatus, 
    Trace, TraceId,
};

use serde::{Deserialize, Serialize};

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

    /// Get the default configuration.
    pub fn default_config() -> Result<Self> {
        let config = Config {
            grpc_port: 4317,
            http_port: 4318,
            max_memory_mb: 512,
            max_traces: 100000,  // Updated to match storage limits
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
}
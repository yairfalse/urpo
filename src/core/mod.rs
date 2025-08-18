//! Core business logic and domain models for Urpo.
//!
//! This module contains the fundamental types and logic that power
//! the OTEL trace exploration functionality.

pub mod config;
pub mod diagnostics;
pub mod error;
pub mod retry;
pub mod types;

// Re-export commonly used types
pub use config::{Config, ConfigBuilder, ConfigWatcher};
pub use error::{Result, UrpoError};
pub use types::{
    ServiceMetrics, ServiceName, Span, SpanBuilder, SpanId, SpanStatus, 
    Trace, TraceId,
};


//! Core business logic and domain models for Urpo.
//!
//! This module contains the fundamental types and logic that power
//! the OTEL trace exploration functionality.

#![warn(missing_docs)]

pub mod config;
pub mod diagnostics;
pub mod error;
pub mod otel_compliance;
pub mod retry;
pub mod string_intern;
pub mod types;

// Re-export commonly used types
pub use config::{Config, ConfigBuilder, ConfigWatcher};
pub use error::{Result, UrpoError};
pub use types::{
    ServiceMetrics, ServiceName, Span, SpanBuilder, SpanId, SpanKind, SpanStatus, Trace, TraceId,
};

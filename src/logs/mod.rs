//! OpenTelemetry logs processing and storage
//!
//! High-performance log storage with full-text search and trace correlation.

pub mod storage;
pub mod types;

pub use storage::LogStorage;
pub use types::{LogRecord, LogSeverity};
//! Log data types and structures

use crate::core::{SpanId, TraceId};
use std::collections::HashMap;

/// Log severity levels per OpenTelemetry specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LogSeverity {
    Trace = 1,
    Debug = 5,
    Info = 9,
    Warn = 13,
    Error = 17,
    Fatal = 21,
}

impl LogSeverity {
    /// Convert from OTLP severity number
    pub fn from_otlp(severity: i32) -> Self {
        match severity {
            1..=4 => Self::Trace,
            5..=8 => Self::Debug,
            9..=12 => Self::Info,
            13..=16 => Self::Warn,
            17..=20 => Self::Error,
            21..=24 => Self::Fatal,
            _ => Self::Info, // Default
        }
    }

    /// Get display string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
        }
    }
}

/// Compact log record optimized for storage
#[derive(Debug, Clone)]
pub struct LogRecord {
    /// Timestamp in nanoseconds
    pub timestamp: u64,
    /// Service identifier (interned)
    pub service_id: u16,
    /// Optional trace ID for correlation
    pub trace_id: Option<TraceId>,
    /// Optional span ID for correlation
    pub span_id: Option<SpanId>,
    /// Log severity level
    pub severity: LogSeverity,
    /// Log message body
    pub body: String,
    /// Additional attributes
    pub attributes: HashMap<String, String>,
}

impl LogRecord {
    /// Create new log record
    pub fn new(timestamp: u64, service_id: u16, severity: LogSeverity, body: String) -> Self {
        Self {
            timestamp,
            service_id,
            severity,
            body,
            trace_id: None,
            span_id: None,
            attributes: HashMap::new(),
        }
    }

    /// Set trace ID for correlation
    pub fn with_trace_id(mut self, trace_id: TraceId) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    /// Set span ID for correlation
    pub fn with_span_id(mut self, span_id: SpanId) -> Self {
        self.span_id = Some(span_id);
        self
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }

    /// Estimated memory size in bytes
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.body.len()
            + self
                .attributes
                .iter()
                .map(|(k, v)| k.len() + v.len())
                .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_from_otlp() {
        assert_eq!(LogSeverity::from_otlp(1), LogSeverity::Trace);
        assert_eq!(LogSeverity::from_otlp(5), LogSeverity::Debug);
        assert_eq!(LogSeverity::from_otlp(9), LogSeverity::Info);
        assert_eq!(LogSeverity::from_otlp(13), LogSeverity::Warn);
        assert_eq!(LogSeverity::from_otlp(17), LogSeverity::Error);
        assert_eq!(LogSeverity::from_otlp(21), LogSeverity::Fatal);

        // Edge cases
        assert_eq!(LogSeverity::from_otlp(0), LogSeverity::Info);
        assert_eq!(LogSeverity::from_otlp(100), LogSeverity::Info);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(LogSeverity::Trace < LogSeverity::Debug);
        assert!(LogSeverity::Debug < LogSeverity::Info);
        assert!(LogSeverity::Info < LogSeverity::Warn);
        assert!(LogSeverity::Warn < LogSeverity::Error);
        assert!(LogSeverity::Error < LogSeverity::Fatal);
    }

    #[test]
    fn test_log_record_creation() {
        let record =
            LogRecord::new(1234567890, 42, LogSeverity::Info, "Test log message".to_string());

        assert_eq!(record.timestamp, 1234567890);
        assert_eq!(record.service_id, 42);
        assert_eq!(record.severity, LogSeverity::Info);
        assert_eq!(record.body, "Test log message");
        assert!(record.trace_id.is_none());
        assert!(record.span_id.is_none());
    }

    #[test]
    fn test_log_record_with_trace() {
        let trace_id = TraceId::new("abcd1234".to_string()).unwrap();
        let span_id = SpanId::new("ef567890".to_string()).unwrap();

        let record =
            LogRecord::new(1234567890, 42, LogSeverity::Error, "Error occurred".to_string())
                .with_trace_id(trace_id.clone())
                .with_span_id(span_id.clone());

        assert_eq!(record.trace_id, Some(trace_id));
        assert_eq!(record.span_id, Some(span_id));
    }

    #[test]
    fn test_log_record_with_attributes() {
        let record = LogRecord::new(1234567890, 42, LogSeverity::Info, "Test".to_string())
            .with_attribute("http.method".to_string(), "GET".to_string())
            .with_attribute("http.status".to_string(), "200".to_string());

        assert_eq!(record.attributes.len(), 2);
        assert_eq!(record.attributes.get("http.method"), Some(&"GET".to_string()));
        assert_eq!(record.attributes.get("http.status"), Some(&"200".to_string()));
    }

    #[test]
    fn test_memory_size_estimation() {
        let record = LogRecord::new(
            1234567890,
            42,
            LogSeverity::Info,
            "A".repeat(100), // 100 bytes
        )
        .with_attribute("key".to_string(), "value".to_string()); // 8 bytes

        let size = record.memory_size();
        assert!(size > 100); // At least the body size
        assert!(size < 1000); // Reasonable upper bound
    }
}

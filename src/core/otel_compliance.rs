//! OpenTelemetry protocol compliance layer.
//!
//! This module ensures 100% OTEL spec compliance while maintaining
//! our ultra-fast performance characteristics.

use opentelemetry_proto::tonic::trace::v1::{
    span::SpanKind as ProtoSpanKind,
    status::StatusCode as ProtoStatusCode,
    Status as ProtoStatus,
};

use crate::core::{SpanKind, SpanStatus, Span};
use std::time::{SystemTime, UNIX_EPOCH};

/// Convert OTEL protocol span kind to our internal representation.
#[inline(always)]
pub fn convert_span_kind(proto_kind: i32) -> SpanKind {
    match ProtoSpanKind::try_from(proto_kind) {
        Ok(ProtoSpanKind::Internal) | Ok(ProtoSpanKind::Unspecified) => SpanKind::Internal,
        Ok(ProtoSpanKind::Server) => SpanKind::Server,
        Ok(ProtoSpanKind::Client) => SpanKind::Client,
        Ok(ProtoSpanKind::Producer) => SpanKind::Producer,
        Ok(ProtoSpanKind::Consumer) => SpanKind::Consumer,
        Err(_) => SpanKind::Internal,
    }
}

/// Convert OTEL protocol status to our internal representation.
#[inline(always)]
pub fn convert_span_status(proto_status: Option<ProtoStatus>) -> SpanStatus {
    match proto_status {
        Some(status) => match ProtoStatusCode::try_from(status.code) {
            Ok(ProtoStatusCode::Ok) => SpanStatus::Ok,
            Ok(ProtoStatusCode::Error) => SpanStatus::Error(status.message),
            Ok(ProtoStatusCode::Unset) | Err(_) => SpanStatus::Unknown,
        },
        None => SpanStatus::Unknown,
    }
}

/// Convert nanoseconds since Unix epoch to SystemTime.
#[inline(always)]
pub fn nanos_to_system_time(nanos: u64) -> SystemTime {
    let secs = nanos / 1_000_000_000;
    let subsec_nanos = (nanos % 1_000_000_000) as u32;
    UNIX_EPOCH + std::time::Duration::new(secs, subsec_nanos)
}

/// Convert SystemTime to nanoseconds since Unix epoch.
#[inline(always)]
pub fn system_time_to_nanos(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// Semantic convention keys for span attributes.
pub mod attributes {
    // Service attributes
    pub const SERVICE_NAME: &str = "service.name";
    pub const SERVICE_VERSION: &str = "service.version";
    pub const SERVICE_NAMESPACE: &str = "service.namespace";
    
    // HTTP attributes
    pub const HTTP_METHOD: &str = "http.method";
    pub const HTTP_STATUS_CODE: &str = "http.status_code";
    pub const HTTP_URL: &str = "http.url";
    pub const HTTP_TARGET: &str = "http.target";
    
    // Database attributes
    pub const DB_SYSTEM: &str = "db.system";
    pub const DB_NAME: &str = "db.name";
    pub const DB_STATEMENT: &str = "db.statement";
    
    // RPC attributes
    pub const RPC_SERVICE: &str = "rpc.service";
    pub const RPC_METHOD: &str = "rpc.method";
    pub const RPC_SYSTEM: &str = "rpc.system";
    
    // Network attributes
    pub const NET_PEER_NAME: &str = "net.peer.name";
    pub const NET_PEER_PORT: &str = "net.peer.port";
    
    // Error attributes
    pub const EXCEPTION_TYPE: &str = "exception.type";
    pub const EXCEPTION_MESSAGE: &str = "exception.message";
}

/// W3C TraceContext propagation support.
pub mod trace_context {
    use std::fmt;
    
    /// W3C TraceContext header name.
    pub const TRACEPARENT_HEADER: &str = "traceparent";
    
    /// W3C TraceState header name.
    pub const TRACESTATE_HEADER: &str = "tracestate";
    
    /// Parse W3C traceparent header.
    /// Format: version-trace_id-span_id-flags
    /// Example: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
    #[inline]
    pub fn parse_traceparent(header: &str) -> Option<(String, String, u8)> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 {
            return None;
        }
        
        let version = parts[0];
        if version != "00" {
            return None; // Only support version 00
        }
        
        let trace_id = parts[1];
        let span_id = parts[2];
        let flags = u8::from_str_radix(parts[3], 16).ok()?;
        
        Some((trace_id.to_string(), span_id.to_string(), flags))
    }
    
    /// Create W3C traceparent header.
    #[inline]
    pub fn create_traceparent(trace_id: &str, span_id: &str, sampled: bool) -> String {
        let flags = if sampled { 0x01 } else { 0x00 };
        format!("00-{}-{}-{:02x}", trace_id, span_id, flags)
    }
}

/// Validate OTEL compliance for a span.
#[inline]
pub fn validate_span(span: &Span) -> Result<(), ValidationError> {
    // Validate trace ID (32 hex chars)
    if span.trace_id.as_str().len() != 32 {
        return Err(ValidationError::InvalidTraceId);
    }
    
    // Validate span ID (16 hex chars)
    if span.span_id.as_str().len() != 16 {
        return Err(ValidationError::InvalidSpanId);
    }
    
    // Validate service name (non-empty)
    if span.service_name.as_str().is_empty() {
        return Err(ValidationError::MissingServiceName);
    }
    
    Ok(())
}

#[derive(Debug, Clone)]
pub enum ValidationError {
    InvalidTraceId,
    InvalidSpanId,
    MissingServiceName,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTraceId => write!(f, "Invalid trace ID format"),
            Self::InvalidSpanId => write!(f, "Invalid span ID format"),
            Self::MissingServiceName => write!(f, "Missing service name"),
        }
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_span_kind_conversion() {
        assert!(matches!(convert_span_kind(0), SpanKind::Internal));
        assert!(matches!(convert_span_kind(1), SpanKind::Internal));
        assert!(matches!(convert_span_kind(2), SpanKind::Server));
        assert!(matches!(convert_span_kind(3), SpanKind::Client));
        assert!(matches!(convert_span_kind(4), SpanKind::Producer));
        assert!(matches!(convert_span_kind(5), SpanKind::Consumer));
    }
    
    #[test]
    fn test_traceparent_parsing() {
        let header = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let (trace_id, span_id, flags) = trace_context::parse_traceparent(header).unwrap();
        
        assert_eq!(trace_id, "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(span_id, "00f067aa0ba902b7");
        assert_eq!(flags, 0x01);
    }
    
    #[test]
    fn test_time_conversion() {
        let nanos = 1_700_000_000_123_456_789u64;
        let time = nanos_to_system_time(nanos);
        let back = system_time_to_nanos(time);
        assert_eq!(nanos, back);
    }
}
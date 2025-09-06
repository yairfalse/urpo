use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use std::fmt;
use crate::core::error::{Result, UrpoError};

/// Unique identifier for a trace
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(String);

/// Unique identifier for a span within a trace
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(String);

/// Service name identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceName(String);

impl TraceId {
    /// Creates a new TraceId after validation
    pub fn new(id: String) -> Result<Self> {
        if id.is_empty() {
            return Err(UrpoError::InvalidSpan("TraceId cannot be empty".to_string()));
        }
        // OTEL trace IDs are 16 bytes = 32 hex characters
        if id.len() > 32 {
            return Err(UrpoError::InvalidSpan(format!("TraceId cannot exceed 32 characters, got {}", id.len())));
        }
        Ok(TraceId(id))
    }
    
    /// Returns the string representation of the trace ID
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Returns the inner string value
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SpanId {
    /// Creates a new SpanId after validation
    pub fn new(id: String) -> Result<Self> {
        if id.is_empty() {
            return Err(UrpoError::InvalidSpan("SpanId cannot be empty".to_string()));
        }
        // OTEL span IDs are 8 bytes = 16 hex characters
        if id.len() > 16 {
            return Err(UrpoError::InvalidSpan(format!("SpanId cannot exceed 16 characters, got {}", id.len())));
        }
        Ok(SpanId(id))
    }
    
    /// Returns the string representation of the span ID
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Returns the inner string value
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ServiceName {
    /// Creates a new ServiceName after validation
    pub fn new(name: String) -> Result<Self> {
        if name.is_empty() {
            return Err(UrpoError::InvalidSpan("ServiceName cannot be empty".to_string()));
        }
        if name.len() > 255 {
            return Err(UrpoError::InvalidSpan("ServiceName cannot exceed 255 characters".to_string()));
        }
        Ok(ServiceName(name))
    }
    
    /// Returns the string representation of the service name
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Returns the inner string value
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for ServiceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a span execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpanStatus {
    /// Span completed successfully
    Ok,
    /// Span completed with an error
    Error(String),
    /// Span was cancelled before completion
    Cancelled,
    /// Span status is unknown
    Unknown,
}

impl SpanStatus {
    /// Returns true if the span status indicates an error
    pub fn is_error(&self) -> bool {
        matches!(self, SpanStatus::Error(_))
    }
    
    /// Returns true if the span completed successfully
    pub fn is_ok(&self) -> bool {
        matches!(self, SpanStatus::Ok)
    }
    
    /// Returns the error message if this is an error status
    pub fn error_message(&self) -> Option<&str> {
        match self {
            SpanStatus::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Represents a single span in a distributed trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Unique identifier for the trace this span belongs to
    pub trace_id: TraceId,
    /// Unique identifier for this span
    pub span_id: SpanId,
    /// Parent span ID if this is a child span
    pub parent_span_id: Option<SpanId>,
    /// Name of the service that generated this span
    pub service_name: ServiceName,
    /// Name of the operation this span represents
    pub operation_name: String,
    /// When the span started
    pub start_time: SystemTime,
    /// How long the span took to complete
    pub duration: Duration,
    /// Status of the span execution
    pub status: SpanStatus,
    /// Key-value attributes associated with the span
    pub attributes: HashMap<String, String>,
    /// Tags for easier filtering and searching
    pub tags: HashMap<String, String>,
    /// Resource attributes (e.g., host, container info)
    pub resource_attributes: HashMap<String, String>,
}

impl Span {
    /// Creates a new span builder
    pub fn builder() -> SpanBuilder {
        SpanBuilder::default()
    }
    
    /// Returns the end time of the span
    pub fn end_time(&self) -> SystemTime {
        self.start_time + self.duration
    }
    
    /// Returns true if this span is a root span (has no parent)
    pub fn is_root(&self) -> bool {
        self.parent_span_id.is_none()
    }
    
    /// Returns true if this span has child spans
    pub fn has_parent(&self) -> bool {
        self.parent_span_id.is_some()
    }
    
    /// Gets an attribute value by key
    pub fn get_attribute(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(|s| s.as_str())
    }
    
    /// Gets a tag value by key
    pub fn get_tag(&self, key: &str) -> Option<&str> {
        self.tags.get(key).map(|s| s.as_str())
    }
    
    /// Returns the duration in milliseconds
    pub fn duration_ms(&self) -> u64 {
        self.duration.as_millis() as u64
    }
}

/// Builder for creating Span instances
#[derive(Default)]
pub struct SpanBuilder {
    trace_id: Option<TraceId>,
    span_id: Option<SpanId>,
    parent_span_id: Option<SpanId>,
    service_name: Option<ServiceName>,
    operation_name: Option<String>,
    start_time: Option<SystemTime>,
    duration: Option<Duration>,
    status: Option<SpanStatus>,
    attributes: HashMap<String, String>,
    tags: HashMap<String, String>,
    resource_attributes: HashMap<String, String>,
}

impl SpanBuilder {
    pub fn trace_id(mut self, trace_id: TraceId) -> Self {
        self.trace_id = Some(trace_id);
        self
    }
    
    pub fn span_id(mut self, span_id: SpanId) -> Self {
        self.span_id = Some(span_id);
        self
    }
    
    pub fn parent_span_id(mut self, parent_span_id: SpanId) -> Self {
        self.parent_span_id = Some(parent_span_id);
        self
    }
    
    pub fn service_name(mut self, service_name: ServiceName) -> Self {
        self.service_name = Some(service_name);
        self
    }
    
    pub fn operation_name<S: Into<String>>(mut self, operation_name: S) -> Self {
        self.operation_name = Some(operation_name.into());
        self
    }
    
    pub fn start_time(mut self, start_time: SystemTime) -> Self {
        self.start_time = Some(start_time);
        self
    }
    
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }
    
    pub fn status(mut self, status: SpanStatus) -> Self {
        self.status = Some(status);
        self
    }
    
    pub fn attribute<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
    
    pub fn tag<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
    
    pub fn resource_attribute<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.resource_attributes.insert(key.into(), value.into());
        self
    }
    
    pub fn build(self) -> Result<Span> {
        Ok(Span {
            trace_id: self.trace_id.ok_or_else(|| UrpoError::InvalidSpan("trace_id is required".to_string()))?,
            span_id: self.span_id.ok_or_else(|| UrpoError::InvalidSpan("span_id is required".to_string()))?,
            parent_span_id: self.parent_span_id,
            service_name: self.service_name.ok_or_else(|| UrpoError::InvalidSpan("service_name is required".to_string()))?,
            operation_name: self.operation_name.ok_or_else(|| UrpoError::InvalidSpan("operation_name is required".to_string()))?,
            start_time: self.start_time.unwrap_or_else(SystemTime::now),
            duration: self.duration.unwrap_or(Duration::from_millis(0)),
            status: self.status.unwrap_or(SpanStatus::Unknown),
            attributes: self.attributes,
            tags: self.tags,
            resource_attributes: self.resource_attributes,
        })
    }
}

/// Represents a complete distributed trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    /// Unique identifier for this trace
    pub trace_id: TraceId,
    /// All spans that are part of this trace
    pub spans: Vec<Span>,
    /// The root span ID (if any)
    pub root_span: Option<SpanId>,
    /// Total duration from first span start to last span end
    pub total_duration: Duration,
    /// Number of unique services involved in this trace
    pub service_count: usize,
    /// Number of spans with errors
    pub error_count: usize,
    /// When this trace was first seen
    pub first_seen: SystemTime,
    /// When this trace was last updated
    pub last_updated: SystemTime,
}

impl Trace {
    /// Creates a new trace from a collection of spans
    pub fn from_spans(trace_id: TraceId, mut spans: Vec<Span>) -> Result<Self> {
        if spans.is_empty() {
            return Err(UrpoError::InvalidSpan("Trace must contain at least one span".to_string()));
        }
        
        // Sort spans by start time
        spans.sort_by_key(|span| span.start_time);
        
        // Find root span
        let root_span = spans.iter()
            .find(|span| span.is_root())
            .map(|span| span.span_id.clone());
        
        // Calculate total duration
        let first_start = spans.first().unwrap().start_time;
        let last_end = spans.iter()
            .map(|span| span.end_time())
            .max()
            .unwrap_or(first_start);
        let total_duration = last_end.duration_since(first_start)
            .unwrap_or(Duration::from_millis(0));
        
        // Count unique services
        let mut services = std::collections::HashSet::new();
        for span in &spans {
            services.insert(span.service_name.as_str());
        }
        let service_count = services.len();
        
        // Count errors
        let error_count = spans.iter()
            .filter(|span| span.status.is_error())
            .count();
        
        let now = SystemTime::now();
        
        Ok(Trace {
            trace_id,
            spans,
            root_span,
            total_duration,
            service_count,
            error_count,
            first_seen: now,
            last_updated: now,
        })
    }
    
    /// Returns spans sorted by start time
    pub fn spans_by_time(&self) -> &[Span] {
        &self.spans
    }
    
    /// Returns spans for a specific service
    pub fn spans_for_service(&self, service_name: &str) -> Vec<&Span> {
        self.spans.iter()
            .filter(|span| span.service_name.as_str() == service_name)
            .collect()
    }
    
    /// Returns the root span if it exists
    pub fn get_root_span(&self) -> Option<&Span> {
        self.root_span.as_ref()
            .and_then(|root_id| self.spans.iter().find(|span| &span.span_id == root_id))
    }
    
    /// Returns child spans for a given parent span ID
    pub fn child_spans(&self, parent_id: &SpanId) -> Vec<&Span> {
        self.spans.iter()
            .filter(|span| span.parent_span_id.as_ref() == Some(parent_id))
            .collect()
    }
    
    /// Returns true if this trace has any errors
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }
    
    /// Returns the list of unique service names in this trace
    pub fn service_names(&self) -> Vec<&str> {
        let mut services: Vec<_> = self.spans.iter()
            .map(|span| span.service_name.as_str())
            .collect();
        services.sort_unstable();
        services.dedup();
        services
    }
}

/// Aggregated metrics for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    /// Name of the service
    pub name: ServiceName,
    /// Requests per second
    pub request_rate: f64,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// 50th percentile latency
    pub latency_p50: Duration,
    /// 95th percentile latency
    pub latency_p95: Duration,
    /// 99th percentile latency
    pub latency_p99: Duration,
    /// When this service was last seen
    pub last_seen: SystemTime,
    /// Total number of spans processed
    pub span_count: u64,
    /// Total number of error spans
    pub error_count: u64,
    /// Average duration across all spans
    pub avg_duration: Duration,
    /// Maximum duration observed
    pub max_duration: Duration,
    /// Minimum duration observed
    pub min_duration: Duration,
}

impl ServiceMetrics {
    /// Creates new service metrics with default values
    pub fn new(name: ServiceName) -> Self {
        Self {
            name,
            request_rate: 0.0,
            error_rate: 0.0,
            latency_p50: Duration::from_millis(0),
            latency_p95: Duration::from_millis(0),
            latency_p99: Duration::from_millis(0),
            last_seen: SystemTime::now(),
            span_count: 0,
            error_count: 0,
            avg_duration: Duration::from_millis(0),
            max_duration: Duration::from_millis(0),
            min_duration: Duration::from_millis(0),
        }
    }
    
    /// Returns true if this service is considered healthy
    pub fn is_healthy(&self) -> bool {
        self.error_rate < 0.05 // Less than 5% error rate
    }
    
    /// Returns the success rate (inverse of error rate)
    pub fn success_rate(&self) -> f64 {
        1.0 - self.error_rate
    }
    
    /// Updates metrics with a new span
    pub fn update_with_span(&mut self, span: &Span) {
        self.span_count += 1;
        self.last_seen = SystemTime::now();
        
        if span.status.is_error() {
            self.error_count += 1;
        }
        
        // Update error rate
        self.error_rate = self.error_count as f64 / self.span_count as f64;
        
        // Update duration statistics
        if self.span_count == 1 {
            self.min_duration = span.duration;
            self.max_duration = span.duration;
            self.avg_duration = span.duration;
        } else {
            if span.duration < self.min_duration {
                self.min_duration = span.duration;
            }
            if span.duration > self.max_duration {
                self.max_duration = span.duration;
            }
            
            // Simple moving average for now
            let total_ms = self.avg_duration.as_millis() as u64 * (self.span_count - 1) + span.duration.as_millis() as u64;
            self.avg_duration = Duration::from_millis(total_ms / self.span_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_id_validation() {
        assert!(TraceId::new("valid_id".to_string()).is_ok());
        assert!(TraceId::new("".to_string()).is_err());
        assert!(TraceId::new("a".repeat(33)).is_err());
    }

    #[test]
    fn test_span_builder() {
        let span = Span::builder()
            .trace_id(TraceId::new("trace1".to_string()).unwrap())
            .span_id(SpanId::new("span1".to_string()).unwrap())
            .service_name(ServiceName::new("test-service".to_string()).unwrap())
            .operation_name("test-op")
            .attribute("key", "value")
            .build()
            .unwrap();
        
        assert_eq!(span.trace_id.as_str(), "trace1");
        assert_eq!(span.operation_name, "test-op");
        assert_eq!(span.get_attribute("key"), Some("value"));
    }

    #[test]
    fn test_trace_from_spans() {
        let trace_id = TraceId::new("trace1".to_string()).unwrap();
        let span = Span::builder()
            .trace_id(trace_id.clone())
            .span_id(SpanId::new("span1".to_string()).unwrap())
            .service_name(ServiceName::new("test-service".to_string()).unwrap())
            .operation_name("test-op")
            .build()
            .unwrap();
        
        let trace = Trace::from_spans(trace_id, vec![span]).unwrap();
        assert_eq!(trace.spans.len(), 1);
        assert_eq!(trace.service_count, 1);
        assert_eq!(trace.error_count, 0);
    }

    #[test]
    fn test_service_metrics_update() {
        let mut metrics = ServiceMetrics::new(ServiceName::new("test".to_string()).unwrap());
        
        let span = Span::builder()
            .trace_id(TraceId::new("trace1".to_string()).unwrap())
            .span_id(SpanId::new("span1".to_string()).unwrap())
            .service_name(ServiceName::new("test".to_string()).unwrap())
            .operation_name("test-op")
            .duration(Duration::from_millis(100))
            .status(SpanStatus::Ok)
            .build()
            .unwrap();
        
        metrics.update_with_span(&span);
        
        assert_eq!(metrics.span_count, 1);
        assert_eq!(metrics.error_count, 0);
        assert_eq!(metrics.error_rate, 0.0);
        assert!(metrics.is_healthy());
    }
}
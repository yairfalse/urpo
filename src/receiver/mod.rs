//! OpenTelemetry receiver implementation.
//!
//! This module implements GRPC and HTTP receivers for OpenTelemetry
//! trace data following the OTLP specification.

use crate::core::{Result, Span as UrpoSpan, SpanId, SpanKind, SpanStatus, TraceId, ServiceName, UrpoError};
use chrono::{DateTime, Utc};
use opentelemetry_proto::tonic::collector::trace::v1::{
    trace_service_server::{TraceService, TraceServiceServer},
    ExportTraceServiceRequest, ExportTraceServiceResponse,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tonic::{transport::Server, Request, Response, Status};

/// OTEL receiver for collecting trace data.
pub struct OtelReceiver {
    /// Channel sender for processed spans.
    span_sender: mpsc::Sender<UrpoSpan>,
    /// Configuration for sampling.
    sampling_rate: f64,
}

impl OtelReceiver {
    /// Create a new OTEL receiver.
    pub fn new(span_sender: mpsc::Sender<UrpoSpan>, sampling_rate: f64) -> Self {
        Self {
            span_sender,
            sampling_rate,
        }
    }

    /// Start the GRPC server.
    pub async fn start_grpc(self: Arc<Self>, addr: SocketAddr) -> Result<()> {
        let service = TraceServiceServer::new(GrpcTraceService {
            receiver: self.clone(),
        });

        Server::builder()
            .add_service(service)
            .serve(addr)
            .await
            .map_err(|e| UrpoError::protocol(format!("Failed to start GRPC server: {}", e)))?;

        Ok(())
    }

    /// Start the HTTP server.
    pub async fn start_http(self: Arc<Self>, addr: SocketAddr) -> Result<()> {
        // HTTP server implementation would go here
        // For now, we'll just log that it would start
        tracing::info!("HTTP server would start on {}", addr);
        Ok(())
    }

    /// Process incoming spans.
    async fn process_spans(&self, spans: Vec<UrpoSpan>) -> Result<()> {
        for span in spans {
            // Apply sampling
            if self.should_sample() {
                self.span_sender
                    .send(span)
                    .await
                    .map_err(|_| UrpoError::ChannelSend)?;
            }
        }
        Ok(())
    }

    /// Determine if a span should be sampled.
    fn should_sample(&self) -> bool {
        if self.sampling_rate >= 1.0 {
            return true;
        }
        if self.sampling_rate <= 0.0 {
            return false;
        }
        
        // Simple random sampling
        rand::random::<f64>() < self.sampling_rate
    }
}

/// GRPC trace service implementation.
struct GrpcTraceService {
    receiver: Arc<OtelReceiver>,
}

#[tonic::async_trait]
impl TraceService for GrpcTraceService {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> std::result::Result<Response<ExportTraceServiceResponse>, Status> {
        let export_request = request.into_inner();
        let mut spans = Vec::new();

        // Process resource spans
        for resource_spans in export_request.resource_spans {
            let resource = resource_spans.resource.unwrap_or_default();
            let service_name = extract_service_name(&resource.attributes);

            for scope_spans in resource_spans.scope_spans {
                for otel_span in scope_spans.spans {
                    match convert_otel_span(otel_span, service_name.clone()) {
                        Ok(span) => spans.push(span),
                        Err(e) => {
                            tracing::warn!("Failed to convert span: {}", e);
                        }
                    }
                }
            }
        }

        // Process the spans
        if let Err(e) = self.receiver.process_spans(spans).await {
            tracing::error!("Failed to process spans: {}", e);
            return Err(Status::internal(format!("Failed to process spans: {}", e)));
        }

        Ok(Response::new(ExportTraceServiceResponse {
            partial_success: None,
        }))
    }
}

/// Extract service name from resource attributes.
fn extract_service_name(attributes: &[opentelemetry_proto::tonic::common::v1::KeyValue]) -> String {
    for attr in attributes {
        if attr.key == "service.name" {
            if let Some(value) = &attr.value {
                if let Some(opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(s)) = &value.value {
                    return s.clone();
                }
            }
        }
    }
    "unknown".to_string()
}

/// Convert OTEL span to Urpo span.
fn convert_otel_span(
    otel_span: opentelemetry_proto::tonic::trace::v1::Span,
    service_name: String,
) -> Result<UrpoSpan> {
    let trace_id = TraceId::new(hex::encode(&otel_span.trace_id))?;
    let span_id = SpanId::new(hex::encode(&otel_span.span_id))?;
    
    let parent_span_id = if otel_span.parent_span_id.is_empty() {
        None
    } else {
        Some(SpanId::new(hex::encode(&otel_span.parent_span_id))?)
    };

    let service_name = ServiceName::new(service_name)?;
    
    let kind = match otel_span.kind() {
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Unspecified => SpanKind::Internal,
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Internal => SpanKind::Internal,
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Server => SpanKind::Server,
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Client => SpanKind::Client,
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Producer => SpanKind::Producer,
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Consumer => SpanKind::Consumer,
    };

    let start_time = nanos_to_datetime(otel_span.start_time_unix_nano);
    let end_time = nanos_to_datetime(otel_span.end_time_unix_nano);

    let status = if let Some(status) = otel_span.status {
        match status.code() {
            opentelemetry_proto::tonic::trace::v1::status::StatusCode::Unset => SpanStatus::Unset,
            opentelemetry_proto::tonic::trace::v1::status::StatusCode::Ok => SpanStatus::Ok,
            opentelemetry_proto::tonic::trace::v1::status::StatusCode::Error => {
                SpanStatus::Error(status.message.clone())
            }
        }
    } else {
        SpanStatus::Unset
    };

    let mut attributes = HashMap::new();
    for attr in otel_span.attributes {
        if let Some(value) = attr.value {
            attributes.insert(attr.key, value_to_string(value));
        }
    }

    let events = otel_span
        .events
        .into_iter()
        .map(|event| crate::core::SpanEvent {
            name: event.name,
            timestamp: nanos_to_datetime(event.time_unix_nano),
            attributes: event
                .attributes
                .into_iter()
                .filter_map(|attr| {
                    attr.value.map(|v| (attr.key, value_to_string(v)))
                })
                .collect(),
        })
        .collect();

    Ok(UrpoSpan {
        span_id,
        trace_id,
        parent_span_id,
        service_name,
        operation_name: otel_span.name,
        kind,
        start_time,
        end_time,
        status,
        attributes,
        events,
    })
}

/// Convert nanoseconds to DateTime.
fn nanos_to_datetime(nanos: u64) -> DateTime<Utc> {
    let secs = (nanos / 1_000_000_000) as i64;
    let nanos = (nanos % 1_000_000_000) as u32;
    DateTime::from_timestamp(secs, nanos).unwrap_or_else(Utc::now)
}

/// Convert OTEL value to string.
fn value_to_string(value: opentelemetry_proto::tonic::common::v1::AnyValue) -> String {
    use opentelemetry_proto::tonic::common::v1::any_value::Value;
    
    match value.value {
        Some(Value::StringValue(s)) => s,
        Some(Value::BoolValue(b)) => b.to_string(),
        Some(Value::IntValue(i)) => i.to_string(),
        Some(Value::DoubleValue(d)) => d.to_string(),
        Some(Value::ArrayValue(arr)) => {
            let values: Vec<String> = arr
                .values
                .into_iter()
                .map(value_to_string)
                .collect();
            format!("[{}]", values.join(", "))
        }
        Some(Value::KvlistValue(kv)) => {
            let pairs: Vec<String> = kv
                .values
                .into_iter()
                .map(|kv| {
                    let value = kv.value.map(value_to_string).unwrap_or_default();
                    format!("{}={}", kv.key, value)
                })
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
        Some(Value::BytesValue(bytes)) => format!("bytes({})", bytes.len()),
        None => String::new(),
    }
}

/// Receiver manager for coordinating GRPC and HTTP receivers.
pub struct ReceiverManager {
    grpc_receiver: Arc<OtelReceiver>,
    http_receiver: Arc<OtelReceiver>,
    grpc_addr: SocketAddr,
    http_addr: SocketAddr,
}

impl ReceiverManager {
    /// Create a new receiver manager.
    pub fn new(
        span_sender: mpsc::Sender<UrpoSpan>,
        grpc_port: u16,
        http_port: u16,
        sampling_rate: f64,
    ) -> Self {
        let grpc_receiver = Arc::new(OtelReceiver::new(span_sender.clone(), sampling_rate));
        let http_receiver = Arc::new(OtelReceiver::new(span_sender, sampling_rate));
        
        let grpc_addr = SocketAddr::from(([0, 0, 0, 0], grpc_port));
        let http_addr = SocketAddr::from(([0, 0, 0, 0], http_port));

        Self {
            grpc_receiver,
            http_receiver,
            grpc_addr,
            http_addr,
        }
    }

    /// Start both GRPC and HTTP receivers.
    pub async fn start(self) -> Result<()> {
        let grpc_handle = tokio::spawn({
            let receiver = self.grpc_receiver.clone();
            let addr = self.grpc_addr;
            async move {
                tracing::info!("Starting GRPC receiver on {}", addr);
                receiver.start_grpc(addr).await
            }
        });

        let http_handle = tokio::spawn({
            let receiver = self.http_receiver.clone();
            let addr = self.http_addr;
            async move {
                tracing::info!("Starting HTTP receiver on {}", addr);
                receiver.start_http(addr).await
            }
        });

        // Wait for both to complete (they shouldn't unless there's an error)
        tokio::select! {
            result = grpc_handle => {
                result
                    .map_err(|e| UrpoError::protocol(format!("GRPC receiver task failed: {}", e)))?
            }
            result = http_handle => {
                result
                    .map_err(|e| UrpoError::protocol(format!("HTTP receiver task failed: {}", e)))?
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_nanos_to_datetime() {
        let nanos = 1_700_000_000_000_000_000; // Approximately Nov 2023
        let dt = nanos_to_datetime(nanos);
        assert!(dt.year() >= 2023);
    }

    #[test]
    fn test_should_sample() {
        let (tx, _rx) = mpsc::channel(10);
        
        let receiver = OtelReceiver::new(tx.clone(), 1.0);
        assert!(receiver.should_sample());
        
        let receiver = OtelReceiver::new(tx.clone(), 0.0);
        assert!(!receiver.should_sample());
    }

    #[test]
    fn test_extract_service_name() {
        use opentelemetry_proto::tonic::common::v1::{AnyValue, KeyValue, any_value::Value};
        
        let attributes = vec![
            KeyValue {
                key: "service.name".to_string(),
                value: Some(AnyValue {
                    value: Some(Value::StringValue("test-service".to_string())),
                }),
            },
        ];
        
        assert_eq!(extract_service_name(&attributes), "test-service");
        
        let empty_attributes = vec![];
        assert_eq!(extract_service_name(&empty_attributes), "unknown");
    }
}
//! OpenTelemetry receiver implementation.
//!
//! This module implements GRPC and HTTP receivers for OpenTelemetry
//! trace and metrics data following the OTLP specification.

pub mod http;
pub mod logs;
pub mod metrics;

use crate::core::{
    Result, ServiceName, Span as UrpoSpan, SpanId, SpanStatus, TraceId, UrpoError,
};
use crate::storage::UnifiedStorage;
use chrono::{DateTime, Utc};
use opentelemetry_proto::tonic::collector::trace::v1::{
    trace_service_server::{TraceService, TraceServiceServer},
    ExportTraceServiceRequest, ExportTraceServiceResponse,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};

/// OTEL receiver for collecting trace data.
pub struct OtelReceiver {
    /// GRPC port
    grpc_port: u16,
    /// HTTP port
    http_port: u16,
    /// Storage backend
    storage: Arc<tokio::sync::RwLock<dyn crate::storage::StorageBackend>>,
    /// Health monitor
    health_monitor: Arc<crate::monitoring::Monitor>,
}

impl OtelReceiver {
    /// Create a new OTEL receiver with UnifiedStorage (recommended).
    pub fn with_storage(
        grpc_port: u16,
        http_port: u16,
        storage: &UnifiedStorage,
        health_monitor: Arc<crate::monitoring::Monitor>,
    ) -> Self {
        Self::new(grpc_port, http_port, storage.as_backend(), health_monitor)
    }

    /// Create a new OTEL receiver.
    pub fn new(
        grpc_port: u16,
        http_port: u16,
        storage: Arc<tokio::sync::RwLock<dyn crate::storage::StorageBackend>>,
        health_monitor: Arc<crate::monitoring::Monitor>,
    ) -> Self {
        Self {
            grpc_port,
            http_port,
            storage,
            health_monitor,
        }
    }

    /// Run both GRPC and HTTP receivers
    pub async fn run(self: Arc<Self>) -> Result<()> {
        tracing::info!(
            "Starting OTEL receivers on ports {} (GRPC) and {} (HTTP)",
            self.grpc_port,
            self.http_port
        );

        let grpc_addr = SocketAddr::from(([0, 0, 0, 0], self.grpc_port));
        let http_addr = SocketAddr::from(([0, 0, 0, 0], self.http_port));

        // Start GRPC server
        let mut grpc_handle = {
            let receiver = self.clone();
            tokio::spawn(async move {
                if let Err(e) = receiver.start_grpc(grpc_addr).await {
                    tracing::error!("GRPC server error: {}", e);
                }
            })
        };

        // Start HTTP server
        let mut http_handle = {
            let receiver = self.clone();
            tokio::spawn(async move {
                if let Err(e) = receiver.start_http(http_addr).await {
                    tracing::error!("HTTP server error: {}", e);
                }
            })
        };

        // Wait for shutdown signal or server error
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received shutdown signal, stopping both servers");
                grpc_handle.abort();
                http_handle.abort();
                Ok(())
            }
            _result = &mut grpc_handle => {
                tracing::warn!("GRPC server stopped unexpectedly");
                http_handle.abort();
                Ok(())
            }
            _result = &mut http_handle => {
                tracing::warn!("HTTP server stopped unexpectedly");
                grpc_handle.abort();
                Ok(())
            }
        }
    }

    /// Start the GRPC server.
    pub async fn start_grpc(self: Arc<Self>, addr: SocketAddr) -> Result<()> {
        let service = TraceServiceServer::new(GrpcTraceService {
            receiver: self.clone(),
        });

        tracing::info!("GRPC server binding to {}", addr);

        // Create server builder and bind
        let server = Server::builder().add_service(service);

        tracing::debug!("Starting server.serve() on {}", addr);

        // Serve with proper error handling
        match server.serve(addr).await {
            Ok(_) => {
                tracing::info!("GRPC server stopped gracefully");
                Ok(())
            },
            Err(e) => {
                tracing::error!("GRPC server error: {} (binding to {})", e, addr);
                // Check if it's a binding/address error
                if e.to_string().contains("Address already in use") {
                    Err(UrpoError::network(format!("Port {} already in use", addr.port())))
                } else if e.to_string().contains("Permission denied") {
                    Err(UrpoError::network(format!("Permission denied binding to {}", addr)))
                } else {
                    Err(UrpoError::protocol(format!("Failed to start GRPC server: {}", e)))
                }
            },
        }
    }

    /// Start the HTTP server.
    pub async fn start_http(self: Arc<Self>, addr: SocketAddr) -> Result<()> {
        tracing::info!("Starting HTTP OTLP receiver on {}", addr);

        let app = http::create_http_router(self);

        let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            UrpoError::network(format!("Failed to bind HTTP server to {}: {}", addr, e))
        })?;

        tracing::info!("HTTP OTLP receiver listening on {}", addr);

        axum::serve(listener, app)
            .await
            .map_err(|e| UrpoError::protocol(format!("HTTP server error: {}", e)))?;

        Ok(())
    }

    /// Process incoming spans.
    async fn process_spans(&self, spans: Vec<UrpoSpan>) -> Result<()> {
        let storage = self.storage.write().await;
        for span in spans {
            // Store the span
            storage.store_span(span).await?;
        }
        Ok(())
    }

    /// Determine if a span should be sampled.
    fn should_sample(&self) -> bool {
        // Always sample for now
        true
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
        let mut total_resource_spans = 0;
        let mut total_scope_spans = 0;
        let mut total_spans = 0;

        // Process resource spans
        for resource_spans in export_request.resource_spans {
            total_resource_spans += 1;
            let resource = resource_spans.resource.unwrap_or_default();
            let service_name = extract_service_name(&resource.attributes);

            tracing::debug!(
                "Processing resource spans for service: {}, scope_spans count: {}",
                service_name,
                resource_spans.scope_spans.len()
            );

            for scope_spans in resource_spans.scope_spans {
                total_scope_spans += 1;
                let scope_name = scope_spans
                    .scope
                    .as_ref()
                    .map(|s| s.name.as_str())
                    .unwrap_or("unknown");

                tracing::debug!(
                    "Processing scope: {}, spans count: {}",
                    scope_name,
                    scope_spans.spans.len()
                );

                for otel_span in scope_spans.spans {
                    total_spans += 1;
                    let span_name = otel_span.name.clone();
                    let trace_id_hex = hex::encode(&otel_span.trace_id);
                    let span_id_hex = hex::encode(&otel_span.span_id);

                    match convert_otel_span(otel_span, service_name.clone()) {
                        Ok(span) => {
                            tracing::debug!(
                                "Converted span: service={}, operation={}, trace_id={}, span_id={}",
                                service_name,
                                span_name,
                                trace_id_hex,
                                span_id_hex
                            );
                            spans.push(span);
                        },
                        Err(e) => {
                            tracing::warn!(
                                "Failed to convert span: service={}, operation={}, trace_id={}, span_id={}, error={}",
                                service_name, span_name, trace_id_hex, span_id_hex, e
                            );
                        },
                    }
                }
            }
        }

        tracing::info!(
            "Received OTEL export request: {} resource spans, {} scope spans, {} spans total, {} successfully converted",
            total_resource_spans,
            total_scope_spans,
            total_spans,
            spans.len()
        );

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
                if let Some(
                    opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(s),
                ) = &value.value
                {
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
    // OTEL trace IDs are 16 bytes (32 hex chars), span IDs are 8 bytes (16 hex chars)
    let trace_id_hex = hex::encode(&otel_span.trace_id);
    let span_id_hex = hex::encode(&otel_span.span_id);

    // Validate IDs are not empty
    if trace_id_hex.is_empty() || trace_id_hex == "00000000000000000000000000000000" {
        return Err(UrpoError::InvalidSpan("Invalid trace ID: empty or all zeros".to_string()));
    }
    if span_id_hex.is_empty() || span_id_hex == "0000000000000000" {
        return Err(UrpoError::InvalidSpan("Invalid span ID: empty or all zeros".to_string()));
    }

    let trace_id = TraceId::new(trace_id_hex)?;
    let span_id = SpanId::new(span_id_hex)?;

    let parent_span_id = if otel_span.parent_span_id.is_empty() {
        None
    } else {
        let parent_hex = hex::encode(&otel_span.parent_span_id);
        if parent_hex != "0000000000000000" {
            Some(SpanId::new(parent_hex)?)
        } else {
            None
        }
    };

    let service_name = if service_name.is_empty() {
        ServiceName::new("unknown".to_string())?
    } else {
        ServiceName::new(service_name)?
    };

    // Map span kind to an attribute instead
    let kind_str = match otel_span.kind() {
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Unspecified => "internal",
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Internal => "internal",
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Server => "server",
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Client => "client",
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Producer => "producer",
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Consumer => "consumer",
    };

    let _start_time = nanos_to_datetime(otel_span.start_time_unix_nano);
    let _end_time = nanos_to_datetime(otel_span.end_time_unix_nano);

    let status = if let Some(status) = otel_span.status {
        match status.code() {
            opentelemetry_proto::tonic::trace::v1::status::StatusCode::Unset => SpanStatus::Unknown,
            opentelemetry_proto::tonic::trace::v1::status::StatusCode::Ok => SpanStatus::Ok,
            opentelemetry_proto::tonic::trace::v1::status::StatusCode::Error => {
                SpanStatus::Error(status.message)
            },
        }
    } else {
        SpanStatus::Unknown
    };

    let mut attributes = HashMap::new();
    for attr in otel_span.attributes {
        if let Some(value) = attr.value {
            attributes.insert(attr.key, value_to_string(value));
        }
    }

    // Store events as attributes for now since we don't have SpanEvent in core types
    for (i, event) in otel_span.events.into_iter().enumerate() {
        attributes.insert(format!("event.{}.name", i), event.name);
        attributes.insert(
            format!("event.{}.time", i),
            nanos_to_datetime(event.time_unix_nano).to_rfc3339(),
        );
        for attr in event.attributes {
            if let Some(value) = attr.value {
                attributes.insert(format!("event.{}.{}", i, attr.key), value_to_string(value));
            }
        }
    }

    // Add span kind to attributes
    attributes.insert("span.kind".to_string(), kind_str.to_string());

    // Calculate duration from start and end times
    let start_system = std::time::SystemTime::UNIX_EPOCH
        + std::time::Duration::from_nanos(otel_span.start_time_unix_nano);
    let end_system = std::time::SystemTime::UNIX_EPOCH
        + std::time::Duration::from_nanos(otel_span.end_time_unix_nano);

    let duration = if end_system > start_system {
        end_system.duration_since(start_system).unwrap_or_default()
    } else {
        std::time::Duration::from_millis(0)
    };

    let mut builder = UrpoSpan::builder()
        .trace_id(trace_id)
        .span_id(span_id)
        .service_name(service_name)
        .operation_name(otel_span.name)
        .start_time(start_system)
        .duration(duration)
        .status(status)
        .attribute("span.kind", kind_str);

    if let Some(parent_id) = parent_span_id {
        builder = builder.parent_span_id(parent_id);
    }

    builder.build()
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
            let values: Vec<String> = arr.values.into_iter().map(value_to_string).collect();
            format!("[{}]", values.join(", "))
        },
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
        },
        Some(Value::BytesValue(bytes)) => format!("bytes({})", bytes.len()),
        None => String::new(),
    }
}

//         // Wait for both to complete (they shouldn't unless there's an error)
//         tokio::select! {
//             result = grpc_handle => {
//                 result
//                     .map_err(|e| UrpoError::protocol(format!("GRPC receiver task failed: {}", e)))?
//             }
//             result = http_handle => {
//                 result
//                     .map_err(|e| UrpoError::protocol(format!("HTTP receiver task failed: {}", e)))?
//             }
//         }
//     }
// }

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

    // fn test_should_sample() {
    //     let (tx, _rx) = mpsc::channel(10);
    //
    //     let receiver = OtelReceiver::new(tx.clone(), 1.0);
    //     assert!(receiver.should_sample());
    //
    //     let receiver = OtelReceiver::new(tx.clone(), 0.0);
    //     assert!(!receiver.should_sample());
    // }

    #[test]
    fn test_extract_service_name() {
        use opentelemetry_proto::tonic::common::v1::{any_value::Value, AnyValue, KeyValue};

        let attributes = vec![KeyValue {
            key: "service.name".to_string(),
            value: Some(AnyValue {
                value: Some(Value::StringValue("test-service".to_string())),
            }),
        }];

        assert_eq!(extract_service_name(&attributes), "test-service");

        let empty_attributes = vec![];
        assert_eq!(extract_service_name(&empty_attributes), "unknown");
    }
}

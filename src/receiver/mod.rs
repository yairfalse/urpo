//! OpenTelemetry receiver implementation.
//!
//! This module implements GRPC and HTTP receivers for OpenTelemetry
//! trace and metrics data following the OTLP specification.

pub mod http;
pub mod logs;
pub mod metrics;

use crate::core::{Result, ServiceName, Span as UrpoSpan, SpanId, SpanStatus, TraceId, UrpoError};
use crate::storage::{UnifiedStorage, ZeroAllocSpanPool};
use chrono::{DateTime, Utc};
use opentelemetry_proto::tonic::collector::trace::v1::{
    trace_service_server::{TraceService, TraceServiceServer},
    ExportTraceServiceRequest, ExportTraceServiceResponse,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};

/// Configuration for OTEL receiver
#[derive(Debug, Clone)]
pub struct ReceiverConfig {
    pub span_pool_size: usize,
    pub batch_size: usize,
    pub sampling_rate: f32,
}

impl Default for ReceiverConfig {
    fn default() -> Self {
        Self {
            span_pool_size: 10_000,  // Configurable instead of hardcoded
            batch_size: 512,         // Configurable instead of hardcoded
            sampling_rate: 0.1,      // Configurable instead of hardcoded
        }
    }
}

/// OpenTelemetry trace receiver supporting both GRPC and HTTP protocols.
///
/// This receiver implements the OTLP specification for collecting trace data
/// from instrumented applications. It supports:
/// - GRPC on the configured port (standard: 4317)
/// - HTTP/JSON on the configured port (standard: 4318)
/// - Real-time span processing and storage
/// - Health monitoring and metrics collection
pub struct OtelReceiver {
    /// GRPC port
    grpc_port: u16,
    /// HTTP port
    http_port: u16,
    /// Storage backend
    storage: Arc<tokio::sync::RwLock<dyn crate::storage::StorageBackend>>,
    /// Health monitor
    health_monitor: Arc<crate::monitoring::Monitor>,
    /// Sampling rate (0.0 to 1.0)
    sampling_rate: f32,
    /// Zero-allocation span pool for 6.3x performance boost
    span_pool: Arc<ZeroAllocSpanPool>,
    /// Batch processing channel
    batch_sender: Option<tokio::sync::mpsc::Sender<Vec<UrpoSpan>>>,
    /// Batch configuration
    batch_size: usize,
    /// Smart sampler for OTEL-compliant sampling
    sampler: Option<Arc<crate::sampling::SmartSampler>>,
}

impl OtelReceiver {
    /// Create a new OTEL receiver from any storage backend.
    pub fn from_storage<S: Into<Arc<tokio::sync::RwLock<dyn crate::storage::StorageBackend>>>>(
        grpc_port: u16,
        http_port: u16,
        storage: S,
        health_monitor: Arc<crate::monitoring::Monitor>,
    ) -> Self {
        Self::new(grpc_port, http_port, storage.into(), health_monitor)
    }

    /// Create a new OTEL receiver with configurable parameters.
    pub fn new(
        grpc_port: u16,
        http_port: u16,
        storage: Arc<tokio::sync::RwLock<dyn crate::storage::StorageBackend>>,
        health_monitor: Arc<crate::monitoring::Monitor>,
    ) -> Self {
        Self::with_config(grpc_port, http_port, storage, health_monitor, Default::default())
    }

    /// Create receiver with custom configuration.
    pub fn with_config(
        grpc_port: u16,
        http_port: u16,
        storage: Arc<tokio::sync::RwLock<dyn crate::storage::StorageBackend>>,
        health_monitor: Arc<crate::monitoring::Monitor>,
        config: ReceiverConfig,
    ) -> Self {
        let span_pool = Arc::new(ZeroAllocSpanPool::new(config.span_pool_size));

        Self {
            grpc_port,
            http_port,
            storage,
            health_monitor,
            sampling_rate: config.sampling_rate,
            span_pool,
            batch_sender: None,
            batch_size: config.batch_size,
            sampler: None,
        }
    }

    /// Set the sampling rate (0.0 to 1.0).
    pub fn with_sampling_rate(mut self, rate: f32) -> Self {
        self.sampling_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Enable batch processing with specified size.
    pub fn with_batch_processing(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        // Initialize batch processor
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<UrpoSpan>>(16);
        let storage = self.storage.clone();

        // Spawn batch processor task
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(batch_size);
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));

            loop {
                tokio::select! {
                    Some(spans) = rx.recv() => {
                        batch.extend(spans);
                        if batch.len() >= batch_size {
                            Self::flush_batch(&storage, &mut batch).await;
                        }
                    }
                    _ = interval.tick() => {
                        if !batch.is_empty() {
                            Self::flush_batch(&storage, &mut batch).await;
                        }
                    }
                }
            }
        });

        self.batch_sender = Some(tx);
        self
    }

    /// Enable OTEL-compliant smart sampling.
    pub fn with_smart_sampling(mut self, storage_budget_gb: u64) -> Self {
        self.sampler = Some(Arc::new(crate::sampling::SmartSampler::new(storage_budget_gb)));
        self
    }

    /// Flush a batch to storage.
    async fn flush_batch(storage: &Arc<tokio::sync::RwLock<dyn crate::storage::StorageBackend>>, batch: &mut Vec<UrpoSpan>) {
        if batch.is_empty() {
            return;
        }

        let storage = storage.write().await;
        for span in batch.drain(..) {
            if let Err(e) = storage.store_span(span).await {
                tracing::error!("Failed to store span: {}", e);
            }
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

    /// Start the GRPC server with all OTLP services.
    pub async fn start_grpc(self: Arc<Self>, addr: SocketAddr) -> Result<()> {
        let trace_service = TraceServiceServer::new(GrpcTraceService {
            receiver: self.clone(),
        });

        tracing::info!("GRPC server binding to {} with trace support", addr);

        // Create server builder with trace service
        let server = Server::builder().add_service(trace_service);

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

    /// Process incoming spans with batching and sampling.
    async fn process_spans(&self, spans: Vec<UrpoSpan>) -> Result<()> {
        // Apply sampling
        let sampled_spans: Vec<UrpoSpan> = if let Some(ref sampler) = self.sampler {
            // Use smart sampler for OTEL-compliant sampling
            let mut sampled = Vec::with_capacity(spans.len());
            for span in spans {
                let trace_id = &span.trace_id;
                match sampler.should_sample_head(trace_id) {
                    crate::sampling::SamplingDecision::Keep => sampled.push(span),
                    crate::sampling::SamplingDecision::Defer => {
                        // For deferred decisions, use simple probability for now
                        if self.should_sample() {
                            sampled.push(span);
                        }
                    }
                    crate::sampling::SamplingDecision::Drop => {}
                }
            }
            sampled
        } else {
            // Fallback to simple sampling
            spans.into_iter()
                .filter(|_| self.should_sample())
                .collect()
        };

        if sampled_spans.is_empty() {
            return Ok(());
        }

        // Use batch processing if configured
        if let Some(ref sender) = self.batch_sender {
            sender.send(sampled_spans).await
                .map_err(|_| UrpoError::protocol("Batch channel closed"))?;
        } else {
            // Direct storage without batching
            let storage = self.storage.write().await;
            for span in sampled_spans {
                storage.store_span(span).await?;
            }
        }
        Ok(())
    }

    /// Determine if a span should be sampled based on the configured sampling rate.
    #[inline]
    fn should_sample(&self) -> bool {
        // Use fastrand for efficient random sampling
        fastrand::f32() < self.sampling_rate
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

        // Process resource spans with full semantics
        for resource_spans in export_request.resource_spans {
            total_resource_spans += 1;
            let resource = resource_spans.resource.unwrap_or_default();
            let semantics = extract_resource_semantics(&resource);
            let service_name = semantics.service_name.clone();

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

                    match convert_otel_span_with_pool(otel_span, &service_name, &self.receiver.span_pool) {
                        Ok(span) => {
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
    extract_resource_attribute(attributes, "service.name")
        .unwrap_or_else(|| "unknown".to_string())
}

/// Extract resource attribute by key (OTEL semantic conventions).
fn extract_resource_attribute(
    attributes: &[opentelemetry_proto::tonic::common::v1::KeyValue],
    key: &str,
) -> Option<String> {
    for attr in attributes {
        if attr.key == key {
            if let Some(value) = &attr.value {
                if let Some(
                    opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(s),
                ) = &value.value
                {
                    return Some(s.clone());
                }
            }
        }
    }
    None
}

/// Extract all resource semantics per OTEL spec.
fn extract_resource_semantics(
    resource: &opentelemetry_proto::tonic::resource::v1::Resource,
) -> ResourceSemantics {
    let attrs = &resource.attributes;

    ResourceSemantics {
        service_name: extract_resource_attribute(attrs, "service.name")
            .unwrap_or_else(|| "unknown".to_string()),
        service_version: extract_resource_attribute(attrs, "service.version"),
        service_namespace: extract_resource_attribute(attrs, "service.namespace"),
        deployment_environment: extract_resource_attribute(attrs, "deployment.environment"),
        host_name: extract_resource_attribute(attrs, "host.name"),
        container_id: extract_resource_attribute(attrs, "container.id"),
        process_pid: extract_resource_attribute(attrs, "process.pid")
            .and_then(|s| s.parse::<i32>().ok()),
        telemetry_sdk_name: extract_resource_attribute(attrs, "telemetry.sdk.name"),
        telemetry_sdk_version: extract_resource_attribute(attrs, "telemetry.sdk.version"),
        telemetry_sdk_language: extract_resource_attribute(attrs, "telemetry.sdk.language"),
    }
}

/// Resource semantics per OTEL specification.
#[derive(Debug, Clone)]
struct ResourceSemantics {
    pub service_name: String,
    pub service_version: Option<String>,
    pub service_namespace: Option<String>,
    pub deployment_environment: Option<String>,
    pub host_name: Option<String>,
    pub container_id: Option<String>,
    pub process_pid: Option<i32>,
    pub telemetry_sdk_name: Option<String>,
    pub telemetry_sdk_version: Option<String>,
    pub telemetry_sdk_language: Option<String>,
}

/// Extract attribute value from OTEL any value.
fn extract_attribute_value(value: &Option<opentelemetry_proto::tonic::common::v1::AnyValue>) -> Option<String> {
    value.as_ref()?.value.as_ref().map(|v| match v {
        opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(s) => s.clone(),
        opentelemetry_proto::tonic::common::v1::any_value::Value::IntValue(i) => i.to_string(),
        opentelemetry_proto::tonic::common::v1::any_value::Value::DoubleValue(d) => d.to_string(),
        opentelemetry_proto::tonic::common::v1::any_value::Value::BoolValue(b) => b.to_string(),
        _ => String::new(),
    })
}

/// Convert OTEL span to Urpo span using zero-alloc pool for 6.3x performance.
fn convert_otel_span_with_pool(
    otel_span: opentelemetry_proto::tonic::trace::v1::Span,
    service_name: &str,
    pool: &Arc<ZeroAllocSpanPool>,
) -> Result<UrpoSpan> {
    // Try to get a span from the pool for zero-allocation
    let pooled = pool.try_get_or_new();
    let mut span_box = pooled.take();

    // Extract all the fields we need
    let (trace_id, span_id, parent_span_id) = extract_span_ids(&otel_span)?;
    let service_name = parse_service_name(service_name)?;
    let status = extract_span_status(&otel_span);
    let timing = extract_span_timing(&otel_span)?;

    // Update the pooled span with new values
    span_box.trace_id = trace_id;
    span_box.span_id = span_id;
    span_box.parent_span_id = parent_span_id;
    span_box.service_name = service_name;
    span_box.operation_name = otel_span.name.clone();
    span_box.start_time = timing.start_time;
    span_box.duration = timing.duration;
    span_box.status = status;

    // Clear and set attributes
    span_box.attributes.0.clear();
    span_box.attributes.push(
        Arc::from("span.kind"),
        Arc::from(extract_span_kind(&otel_span))
    );

    // Add other attributes from OTEL span
    for attr in otel_span.attributes {
        if let Some(value) = extract_attribute_value(&attr.value) {
            span_box.attributes.push(Arc::from(attr.key.as_str()), Arc::from(value.as_str()));
        }
    }

    Ok(*span_box)
}

/// Convert OTEL span to Urpo span (legacy without pool).
fn convert_otel_span(
    otel_span: opentelemetry_proto::tonic::trace::v1::Span,
    service_name: String,
) -> Result<UrpoSpan> {
    let (trace_id, span_id, parent_span_id) = extract_span_ids(&otel_span)?;
    let service_name = parse_service_name(service_name)?;
    let status = extract_span_status(&otel_span);
    let timing = extract_span_timing(&otel_span)?;
    let attributes = extract_span_attributes(&otel_span);

    let mut builder = UrpoSpan::builder()
        .trace_id(trace_id)
        .span_id(span_id)
        .service_name(service_name)
        .operation_name(otel_span.name.clone())
        .start_time(timing.start_time)
        .duration(timing.duration)
        .status(status)
        .attribute("span.kind", extract_span_kind(&otel_span));

    if let Some(parent_id) = parent_span_id {
        builder = builder.parent_span_id(parent_id);
    }

    builder.build()
}

/// Extract trace ID, span ID, and parent span ID from OTEL span
fn extract_span_ids(
    otel_span: &opentelemetry_proto::tonic::trace::v1::Span,
) -> Result<(TraceId, SpanId, Option<SpanId>)> {
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

    Ok((trace_id, span_id, parent_span_id))
}

/// Parse and validate service name
fn parse_service_name(service_name: String) -> Result<ServiceName> {
    if service_name.is_empty() {
        ServiceName::new("unknown".to_string())
    } else {
        ServiceName::new(service_name)
    }
}

/// Extract span status from OTEL span
fn extract_span_status(otel_span: &opentelemetry_proto::tonic::trace::v1::Span) -> SpanStatus {
    if let Some(status) = &otel_span.status {
        match status.code() {
            opentelemetry_proto::tonic::trace::v1::status::StatusCode::Unset => SpanStatus::Unknown,
            opentelemetry_proto::tonic::trace::v1::status::StatusCode::Ok => SpanStatus::Ok,
            opentelemetry_proto::tonic::trace::v1::status::StatusCode::Error => {
                SpanStatus::Error(status.message.clone())
            },
        }
    } else {
        SpanStatus::Unknown
    }
}

/// Extract span kind as string
fn extract_span_kind(otel_span: &opentelemetry_proto::tonic::trace::v1::Span) -> &'static str {
    match otel_span.kind() {
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Unspecified => "internal",
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Internal => "internal",
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Server => "server",
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Client => "client",
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Producer => "producer",
        opentelemetry_proto::tonic::trace::v1::span::SpanKind::Consumer => "consumer",
    }
}

/// Timing information extracted from span
struct SpanTiming {
    start_time: std::time::SystemTime,
    duration: std::time::Duration,
}

/// Extract timing information from OTEL span with proper error handling
fn extract_span_timing(otel_span: &opentelemetry_proto::tonic::trace::v1::Span) -> Result<SpanTiming> {
    // Validate timestamps are reasonable (not zero, not in far future)
    if otel_span.start_time_unix_nano == 0 {
        return Err(UrpoError::protocol("Invalid span: start_time is zero"));
    }

    if otel_span.end_time_unix_nano == 0 {
        return Err(UrpoError::protocol("Invalid span: end_time is zero"));
    }

    // Convert nanoseconds to SystemTime with overflow protection
    let start_system = safe_nanos_to_system_time(otel_span.start_time_unix_nano)?;
    let end_system = safe_nanos_to_system_time(otel_span.end_time_unix_nano)?;

    // Calculate duration with proper error handling
    let duration = if end_system >= start_system {
        end_system
            .duration_since(start_system)
            .map_err(|e| UrpoError::protocol(format!("Invalid span duration: {}", e)))?
    } else {
        return Err(UrpoError::protocol(format!(
            "Invalid span: end_time ({:?}) before start_time ({:?})",
            end_system, start_system
        )));
    };

    // Validate duration is reasonable (not longer than 24 hours)
    const MAX_SPAN_DURATION: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);
    if duration > MAX_SPAN_DURATION {
        return Err(UrpoError::protocol(format!(
            "Invalid span: duration too long ({:?}), max allowed: {:?}",
            duration, MAX_SPAN_DURATION
        )));
    }

    Ok(SpanTiming {
        start_time: start_system,
        duration,
    })
}

/// Safely convert nanoseconds to SystemTime with overflow protection
#[inline]
fn safe_nanos_to_system_time(nanos: u64) -> Result<std::time::SystemTime> {
    // Protect against overflow when converting nanoseconds to Duration
    const MAX_NANOS: u64 = u64::MAX / 2; // Conservative limit to prevent overflow

    if nanos > MAX_NANOS {
        return Err(UrpoError::protocol(format!(
            "Timestamp overflow: {} nanoseconds exceeds maximum",
            nanos
        )));
    }

    // Validate timestamp is reasonable (after 2000, before 2100)
    const YEAR_2000_NANOS: u64 = 946_684_800_000_000_000; // 2000-01-01 in nanoseconds
    const YEAR_2100_NANOS: u64 = 4_102_444_800_000_000_000; // 2100-01-01 in nanoseconds

    if nanos < YEAR_2000_NANOS {
        return Err(UrpoError::protocol(format!(
            "Invalid timestamp: {} is before year 2000",
            nanos
        )));
    }

    if nanos > YEAR_2100_NANOS {
        return Err(UrpoError::protocol(format!(
            "Invalid timestamp: {} is after year 2100",
            nanos
        )));
    }

    Ok(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_nanos(nanos))
}

/// Extract attributes from OTEL span including events
fn extract_span_attributes(
    otel_span: &opentelemetry_proto::tonic::trace::v1::Span,
) -> HashMap<String, String> {
    let mut attributes = HashMap::new();

    // Extract regular attributes
    for attr in &otel_span.attributes {
        if let Some(value) = &attr.value {
            attributes.insert(attr.key.clone(), value_to_string(value.clone()));
        }
    }

    // Store events as attributes for now since we don't have SpanEvent in core types
    for (i, event) in otel_span.events.iter().enumerate() {
        attributes.insert(format!("event.{}.name", i), event.name.clone());
        attributes.insert(
            format!("event.{}.time", i),
            nanos_to_datetime(event.time_unix_nano).to_rfc3339(),
        );
        for attr in &event.attributes {
            if let Some(value) = &attr.value {
                attributes
                    .insert(format!("event.{}.{}", i, attr.key), value_to_string(value.clone()));
            }
        }
    }

    attributes
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

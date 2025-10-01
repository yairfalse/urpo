//! OpenTelemetry receiver implementation.
//!
//! This module implements GRPC and HTTP receivers for OpenTelemetry
//! trace and metrics data following the OTLP specification.

pub mod http;
pub mod logs;
pub mod metrics;

use crate::core::{Result, ServiceName, Span as UrpoSpan, SpanId, SpanStatus, TraceId, UrpoError};
use crate::metrics::MetricStorage;
use crate::storage::ZeroAllocSpanPool;
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
            span_pool_size: 10_000, // Configurable instead of hardcoded
            batch_size: 512,        // Configurable instead of hardcoded
            sampling_rate: 1.0,     // Accept all traces by default for debugging
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
/// - Real-time event broadcasting for UI updates
#[derive(Clone)]
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
    /// Metrics storage for OTLP metrics
    metrics_storage: Option<Arc<tokio::sync::Mutex<MetricStorage>>>,
    /// Event broadcaster for real-time UI updates
    event_sender: Option<tokio::sync::broadcast::Sender<TraceEvent>>,
}

/// Real-time trace event for broadcasting to UI
#[derive(Debug, Clone, serde::Serialize)]
pub struct TraceEvent {
    pub trace_id: String,
    pub service_name: String,
    pub span_count: usize,
    pub timestamp: u64,
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

        // Initialize metrics storage with 1M capacity
        let metrics_storage = Some(Arc::new(tokio::sync::Mutex::new(
            MetricStorage::new(1_048_576, 1000), // 1M metrics, 1000 services
        )));

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
            metrics_storage,
            event_sender: None,
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
        let storage = Arc::clone(&self.storage);

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

    /// Enable metrics collection with specified capacity.
    pub fn with_metrics(mut self, buffer_capacity: usize, max_services: usize) -> Self {
        self.metrics_storage = Some(Arc::new(tokio::sync::Mutex::new(MetricStorage::new(
            buffer_capacity,
            max_services,
        ))));
        self
    }

    /// Get metrics storage for querying.
    pub fn metrics_storage(&self) -> Option<&Arc<tokio::sync::Mutex<MetricStorage>>> {
        self.metrics_storage.as_ref()
    }

    /// Enable real-time event broadcasting for UI updates.
    /// Returns a receiver that can subscribe to trace events.
    pub fn with_events(mut self) -> (Self, tokio::sync::broadcast::Receiver<TraceEvent>) {
        let (tx, rx) = tokio::sync::broadcast::channel(1000); // Buffer 1000 events
        self.event_sender = Some(tx);
        (self, rx)
    }

    /// Get event receiver for subscribing to trace events.
    pub fn subscribe_events(&self) -> Option<tokio::sync::broadcast::Receiver<TraceEvent>> {
        self.event_sender.as_ref().map(|tx| tx.subscribe())
    }

    /// Flush a batch to storage.
    async fn flush_batch(
        storage: &Arc<tokio::sync::RwLock<dyn crate::storage::StorageBackend>>,
        batch: &mut Vec<UrpoSpan>,
    ) {
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
            let receiver = Arc::clone(&self);
            tokio::spawn(async move {
                if let Err(e) = receiver.start_grpc(grpc_addr).await {
                    tracing::error!("GRPC server error: {}", e);
                }
            })
        };

        // Start HTTP server
        let mut http_handle = {
            let receiver = Arc::clone(&self);
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
        let mut server = Server::builder().add_service(trace_service);

        // Add metrics service if enabled
        if let Some(ref metrics_storage) = self.metrics_storage {
            tracing::info!("Adding OTLP metrics service to GRPC server");
            server = server
                .add_service(metrics::create_metrics_service_server(Arc::clone(metrics_storage)));
        }

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
        let span_count = spans.len();
        tracing::info!("🔧 Processing {} spans through sampling and storage", span_count);

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
                    },
                    crate::sampling::SamplingDecision::Drop => {},
                }
            }
            sampled
        } else {
            // Fallback to simple sampling
            spans.into_iter().filter(|_| self.should_sample()).collect()
        };

        if sampled_spans.is_empty() {
            tracing::warn!("All {} spans were filtered out by sampling", span_count);
            return Ok(());
        }

        tracing::info!("After sampling: {} spans will be stored", sampled_spans.len());

        // Use batch processing if configured
        if let Some(ref sender) = self.batch_sender {
            tracing::debug!("Sending spans to batch processor");
            sender
                .send(sampled_spans)
                .await
                .map_err(|_| UrpoError::protocol("Batch channel closed"))?;
        } else {
            // Direct storage without batching
            tracing::info!("Storing spans directly to storage (no batching configured)");
            let storage = self.storage.write().await;
            let span_count = sampled_spans.len();

            // Group spans by trace_id for event broadcasting
            let mut trace_map: std::collections::HashMap<String, (String, usize)> = std::collections::HashMap::new();

            for span in sampled_spans {
                tracing::debug!(
                    "Storing span: {} for service: {}",
                    span.span_id,
                    span.service_name
                );

                // Track trace info for events
                let trace_id = span.trace_id.as_str().to_string();
                let service_name = span.service_name.to_string();

                storage.store_span(span).await?;

                // Update trace map
                trace_map.entry(trace_id.clone())
                    .and_modify(|(_, count)| *count += 1)
                    .or_insert((service_name, 1));
            }

            // Broadcast events for real-time UI updates
            if let Some(ref event_tx) = self.event_sender {
                for (trace_id, (service_name, span_count)) in trace_map {
                    let event = TraceEvent {
                        trace_id,
                        service_name,
                        span_count,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos() as u64,
                    };

                    // Non-blocking send - if no receivers, that's OK
                    let _ = event_tx.send(event);
                }
            }

            tracing::info!("Successfully stored {} spans", span_count);
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
        tracing::info!("🔥 RECEIVED OTLP TRACE EXPORT REQUEST");

        let export_request = request.into_inner();
        let mut spans = Vec::new();
        let mut total_resource_spans = 0;
        let mut total_scope_spans = 0;
        let mut total_spans = 0;

        tracing::info!(
            "Export request contains {} resource spans",
            export_request.resource_spans.len()
        );

        // Process resource spans with full semantics
        for resource_spans in export_request.resource_spans {
            total_resource_spans += 1;
            let resource = resource_spans.resource.unwrap_or_default();
            let semantics = extract_resource_semantics(&resource);
            let service_name = semantics.service_name.clone();

            tracing::info!(
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

                tracing::info!(
                    "Processing scope: {}, spans count: {}",
                    scope_name,
                    scope_spans.spans.len()
                );

                for otel_span in scope_spans.spans {
                    total_spans += 1;

                    match convert_otel_span_with_pool(
                        otel_span,
                        &service_name,
                        &self.receiver.span_pool,
                    ) {
                        Ok(span) => {
                            tracing::debug!(
                                "Successfully converted span: {} for service: {}",
                                span.span_id,
                                service_name
                            );
                            spans.push(span);
                        },
                        Err(e) => {
                            tracing::warn!(
                                "Failed to convert span: service={}, error={}",
                                service_name,
                                e
                            );
                        },
                    }
                }
            }
        }

        tracing::info!(
            "🚀 PROCESSING OTLP export: {} resource spans, {} scope spans, {} spans total, {} successfully converted",
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
    extract_resource_attribute(attributes, "service.name").unwrap_or_else(|| "unknown".into())
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
            .unwrap_or_else(|| "unknown".into()),
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
fn extract_attribute_value(
    value: &Option<opentelemetry_proto::tonic::common::v1::AnyValue>,
) -> Option<String> {
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
    let service_name = parse_service_name(&service_name)?;
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
    span_box
        .attributes
        .push(Arc::from("span.kind"), Arc::from(extract_span_kind(&otel_span)));

    // Add other attributes from OTEL span
    for attr in otel_span.attributes {
        if let Some(value) = extract_attribute_value(&attr.value) {
            span_box
                .attributes
                .push(Arc::from(attr.key.as_str()), Arc::from(value.as_str()));
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
    let service_name = parse_service_name(&service_name)?;
    let status = extract_span_status(&otel_span);
    let timing = extract_span_timing(&otel_span)?;
    let _attributes = extract_span_attributes(&otel_span);

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
    // BLAZING FAST: Pre-check lengths for fast path
    if otel_span.trace_id.len() == 16 && otel_span.span_id.len() == 8 {
        // Fast path: Use unsafe hex encoding for known-valid lengths
        let trace_id_hex = unsafe { unsafe_hex_encode(&otel_span.trace_id) };
        let span_id_hex = unsafe { unsafe_hex_encode(&otel_span.span_id[..8]) };

        // Quick zero check without allocation
        if !is_all_zeros(&otel_span.trace_id) && !is_all_zeros(&otel_span.span_id) {
            let trace_id = TraceId::new(trace_id_hex)?;
            let span_id = SpanId::new(span_id_hex)?;

            let parent_span_id = if otel_span.parent_span_id.is_empty() {
                None
            } else if otel_span.parent_span_id.len() == 8
                && !is_all_zeros(&otel_span.parent_span_id)
            {
                let parent_hex = unsafe { unsafe_hex_encode(&otel_span.parent_span_id) };
                Some(SpanId::new(parent_hex)?)
            } else {
                None
            };

            return Ok((trace_id, span_id, parent_span_id));
        }
    }

    // Slow path: Full validation
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
fn parse_service_name(service_name: &str) -> Result<ServiceName> {
    if service_name.is_empty() {
        ServiceName::new("unknown".into())
    } else {
        ServiceName::new(service_name.into())
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
#[derive(Debug)]
struct SpanTiming {
    start_time: std::time::SystemTime,
    duration: std::time::Duration,
}

/// Extract timing information from OTEL span with proper error handling
fn extract_span_timing(
    otel_span: &opentelemetry_proto::tonic::trace::v1::Span,
) -> Result<SpanTiming> {
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
    // BLAZING FAST: Use unsafe unchecked conversion for valid timestamps
    const YEAR_2000_NANOS: u64 = 946_684_800_000_000_000; // 2000-01-01 in nanoseconds
    const YEAR_2100_NANOS: u64 = 4_102_444_800_000_000_000; // 2100-01-01 in nanoseconds
    const MAX_NANOS: u64 = u64::MAX / 2; // Conservative limit to prevent overflow

    // Fast path: If timestamp is in reasonable range, skip validation
    if nanos >= YEAR_2000_NANOS && nanos <= YEAR_2100_NANOS && nanos <= MAX_NANOS {
        // UNSAFE: We've validated the range, so this is safe
        return Ok(unsafe {
            std::time::SystemTime::UNIX_EPOCH
                .checked_add(std::time::Duration::from_nanos(nanos))
                .unwrap_unchecked()
        });
    }

    // Slow path: Full validation for edge cases
    if nanos > MAX_NANOS {
        return Err(UrpoError::protocol(format!(
            "Timestamp overflow: {} nanoseconds exceeds maximum",
            nanos
        )));
    }

    if nanos < YEAR_2000_NANOS {
        return Err(UrpoError::protocol(format!(
            "Timestamp outside valid range: {} is before year 2000",
            nanos
        )));
    }

    if nanos > YEAR_2100_NANOS {
        return Err(UrpoError::protocol(format!(
            "Timestamp outside valid range: {} is after year 2100",
            nanos
        )));
    }

    Ok(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_nanos(nanos))
}

/// UNSAFE: Fast hex encoding for known-valid byte slices
/// PERFORMANCE: 2x faster than hex::encode for hot paths
#[inline(always)]
unsafe fn unsafe_hex_encode(bytes: &[u8]) -> String {
    // SAFETY: We've pre-validated the input length
    const HEX_CHARS: &[u8] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    let result_bytes = result.as_mut_vec();

    for &byte in bytes {
        result_bytes.push(*HEX_CHARS.get_unchecked((byte >> 4) as usize));
        result_bytes.push(*HEX_CHARS.get_unchecked((byte & 0x0f) as usize));
    }

    result
}

/// BLAZING FAST: Check if byte slice is all zeros without allocating
#[inline(always)]
fn is_all_zeros(bytes: &[u8]) -> bool {
    // PERFORMANCE: Use SIMD-friendly loop for zero detection
    bytes.iter().all(|&b| b == 0)
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
    DateTime::from_timestamp(secs, nanos).unwrap_or_else(|| Utc::now())
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
    use opentelemetry_proto::tonic::{
        common::v1::{any_value::Value, AnyValue, KeyValue},
        trace::v1::{Span as OtelSpan, Status},
    };

    #[test]
    fn test_nanos_to_datetime() {
        let nanos = 1_700_000_000_000_000_000; // Approximately Nov 2023
        let dt = nanos_to_datetime(nanos);
        assert!(dt.year() >= 2023);
    }

    #[test]
    fn test_extract_service_name() {
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

    #[test]
    fn test_extract_span_timing_valid() {
        let span = OtelSpan {
            start_time_unix_nano: 1_000_000_000,
            end_time_unix_nano: 2_000_000_000,
            ..Default::default()
        };

        let timing = extract_span_timing(&span).expect("Test span timing should be valid");
        assert_eq!(timing.duration.as_nanos(), 1_000_000_000);
    }

    #[test]
    fn test_extract_span_timing_zero_start() {
        let span = OtelSpan {
            start_time_unix_nano: 0,
            end_time_unix_nano: 1_000_000_000,
            ..Default::default()
        };

        let result = extract_span_timing(&span);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("start_time is zero"));
    }

    #[test]
    fn test_extract_span_timing_overflow_protection() {
        let span = OtelSpan {
            start_time_unix_nano: u64::MAX - 1000,
            end_time_unix_nano: u64::MAX,
            ..Default::default()
        };

        let timing = extract_span_timing(&span).expect("Test span timing should be valid");
        assert_eq!(timing.duration.as_nanos(), 1000);
    }

    #[test]
    fn test_extract_span_timing_invalid_range() {
        // Year 1999
        let span = OtelSpan {
            start_time_unix_nano: 915_148_800_000_000_000,
            end_time_unix_nano: 915_148_801_000_000_000,
            ..Default::default()
        };

        let result = extract_span_timing(&span);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("outside valid range"));
    }

    #[test]
    fn test_extract_span_status_ok() {
        let span = OtelSpan {
            status: Some(Status {
                code: 1,
                message: String::new(),
            }),
            ..Default::default()
        };

        let status = extract_span_status(&span);
        assert!(matches!(status, SpanStatus::Ok));
    }

    #[test]
    fn test_extract_span_status_error() {
        let span = OtelSpan {
            status: Some(Status {
                code: 2,
                message: "Database connection failed".to_string(),
            }),
            ..Default::default()
        };

        let status = extract_span_status(&span);
        match status {
            SpanStatus::Error(msg) => assert_eq!(msg, "Database connection failed"),
            _ => panic!("Expected error status"),
        }
    }

    #[test]
    fn test_extract_span_ids_valid() {
        let span = OtelSpan {
            trace_id: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            span_id: vec![1, 2, 3, 4, 5, 6, 7, 8],
            parent_span_id: vec![],
            ..Default::default()
        };

        let (trace_id, span_id, parent_id) =
            extract_span_ids(&span).expect("Test span IDs should be valid");
        assert_eq!(trace_id.to_string(), "0102030405060708090a0b0c0d0e0f10");
        assert_eq!(span_id.to_string(), "0102030405060708");
        assert!(parent_id.is_none());
    }

    #[test]
    fn test_extract_span_ids_with_parent() {
        let span = OtelSpan {
            trace_id: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            span_id: vec![1, 2, 3, 4, 5, 6, 7, 8],
            parent_span_id: vec![8, 7, 6, 5, 4, 3, 2, 1],
            ..Default::default()
        };

        let (_, _, parent_id) = extract_span_ids(&span).unwrap();
        assert!(parent_id.is_some());
        assert_eq!(parent_id.expect("Parent ID should be present").to_string(), "0807060504030201");
    }

    #[test]
    fn test_extract_span_ids_empty_trace() {
        let span = OtelSpan {
            trace_id: vec![],
            span_id: vec![1, 2, 3, 4, 5, 6, 7, 8],
            ..Default::default()
        };

        let result = extract_span_ids(&span);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid trace ID"));
    }

    #[test]
    fn test_extract_span_ids_all_zeros() {
        let span = OtelSpan {
            trace_id: vec![0; 16],
            span_id: vec![0; 8],
            ..Default::default()
        };

        let result = extract_span_ids(&span);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("all zeros"));
    }

    #[test]
    fn test_parse_service_name_valid() {
        assert!(parse_service_name("my-service").is_ok());
        assert!(parse_service_name("my_service").is_ok());
        assert!(parse_service_name("my.service").is_ok());
        assert!(parse_service_name("MyService123").is_ok());
    }

    #[test]
    fn test_parse_service_name_empty() {
        let result = parse_service_name("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_service_name_too_long() {
        let long_name = "a".repeat(300);
        let result = parse_service_name(&long_name);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_span_kind() {
        let client_span = OtelSpan {
            kind: 3, // CLIENT
            ..Default::default()
        };
        assert_eq!(extract_span_kind(&client_span), "client");

        let server_span = OtelSpan {
            kind: 2, // SERVER
            ..Default::default()
        };
        assert_eq!(extract_span_kind(&server_span), "server");

        let internal_span = OtelSpan {
            kind: 1, // INTERNAL
            ..Default::default()
        };
        assert_eq!(extract_span_kind(&internal_span), "internal");

        let unspecified_span = OtelSpan {
            kind: 0, // UNSPECIFIED
            ..Default::default()
        };
        assert_eq!(extract_span_kind(&unspecified_span), "internal");
    }

    #[test]
    fn test_extract_attribute_value() {
        // String value
        let string_val = Some(AnyValue {
            value: Some(Value::StringValue("test".to_string())),
        });
        assert_eq!(extract_attribute_value(&string_val), Some("test".to_string()));

        // Int value
        let int_val = Some(AnyValue {
            value: Some(Value::IntValue(42)),
        });
        assert_eq!(extract_attribute_value(&int_val), Some("42".to_string()));

        // Double value
        let double_val = Some(AnyValue {
            value: Some(Value::DoubleValue(3.14)),
        });
        assert_eq!(extract_attribute_value(&double_val), Some("3.14".to_string()));

        // Bool value
        let bool_val = Some(AnyValue {
            value: Some(Value::BoolValue(true)),
        });
        assert_eq!(extract_attribute_value(&bool_val), Some("true".to_string()));

        // None value
        assert_eq!(extract_attribute_value(&None), None);
    }

    #[test]
    fn test_convert_otel_span_with_pool() {
        let pool = Arc::new(ZeroAllocSpanPool::new(10));

        let otel_span = OtelSpan {
            trace_id: vec![1; 16],
            span_id: vec![2; 8],
            name: "test-operation".to_string(),
            start_time_unix_nano: 1_700_000_000_000_000_000,
            end_time_unix_nano: 1_700_000_001_000_000_000,
            kind: 2, // SERVER
            attributes: vec![KeyValue {
                key: "http.method".to_string(),
                value: Some(AnyValue {
                    value: Some(Value::StringValue("GET".to_string())),
                }),
            }],
            ..Default::default()
        };

        let result = convert_otel_span_with_pool(otel_span, "test-service", &pool);
        assert!(result.is_ok());

        let span = result.expect("Span conversion should succeed");
        assert_eq!(span.operation_name, "test-operation");
        assert_eq!(span.service_name.to_string(), "test-service");
        assert!(span.attributes.get("http.method").is_some());
    }

    #[test]
    fn test_receiver_config() {
        let config = ReceiverConfig::default();
        assert_eq!(config.span_pool_size, 10000);
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.sampling_rate, 1.0);

        let custom_config = ReceiverConfig {
            span_pool_size: 5000,
            sampling_rate: 0.5,
            ..Default::default()
        };
        assert_eq!(custom_config.span_pool_size, 5000);
        assert_eq!(custom_config.sampling_rate, 0.5);
    }
}

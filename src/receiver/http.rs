//! HTTP OTLP receiver implementation.
//!
//! Implements the OTLP/HTTP protocol specification for receiving traces
//! over HTTP on port 4318. Supports both JSON and protobuf formats.

use crate::receiver::{convert_otel_span, extract_service_name};
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use prost::Message;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// HTTP OTLP server state.
#[derive(Clone)]
pub struct HttpOtelState {
    pub receiver: Arc<super::OtelReceiver>,
}

/// Create HTTP router for OTLP endpoints.
pub fn create_http_router(receiver: Arc<super::OtelReceiver>) -> Router {
    let state = HttpOtelState { receiver };

    Router::new()
        // OTLP trace endpoints
        .route("/v1/traces", post(handle_traces_v1))
        .route("/v1/trace", post(handle_traces_v1)) // Alternative endpoint
        // OTLP metrics endpoint
        .route("/v1/metrics", post(handle_metrics_v1))
        // Health check
        .route("/health", get(health_check))
        .route("/", get(root_handler))
        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(tower_http::cors::Any)
                        .allow_methods(tower_http::cors::Any)
                        .allow_headers(tower_http::cors::Any),
                ),
        )
        .with_state(state)
}

/// Handle OTLP trace export requests.
async fn handle_traces_v1(
    State(state): State<HttpOtelState>,
    headers: HeaderMap,
    body: Bytes,
) -> std::result::Result<impl IntoResponse, HttpError> {
    tracing::debug!("Received HTTP trace export request, {} bytes", body.len());

    // Determine content type
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    tracing::debug!("Content-Type: {}", content_type);

    // Parse the request based on content type
    let export_request = if content_type.contains("application/x-protobuf")
        || content_type.contains("application/octet-stream")
    {
        // Protobuf format
        parse_protobuf_request(&body)?
    } else {
        // Assume JSON format
        parse_json_request(&body)?
    };

    // Process the spans using the same logic as gRPC
    let spans = process_export_request(export_request)?;

    // Store spans
    if let Err(e) = state.receiver.process_spans(spans).await {
        tracing::error!("Failed to process spans: {}", e);
        return Err(HttpError::Internal(format!("Failed to process spans: {}", e)));
    }

    tracing::debug!("Successfully processed HTTP trace export request");

    // Return OTLP response
    Ok(Json(serde_json::json!({
        "partialSuccess": null
    })))
}

/// Parse protobuf OTLP request.
fn parse_protobuf_request(
    body: &[u8],
) -> std::result::Result<ExportTraceServiceRequest, HttpError> {
    ExportTraceServiceRequest::decode(body)
        .map_err(|e| HttpError::BadRequest(format!("Failed to parse protobuf: {}", e)))
}

/// Parse JSON OTLP request.
fn parse_json_request(body: &[u8]) -> std::result::Result<ExportTraceServiceRequest, HttpError> {
    // First parse as generic JSON to validate structure
    let json_value: Value = serde_json::from_slice(body)
        .map_err(|e| HttpError::BadRequest(format!("Invalid JSON: {}", e)))?;

    // Convert JSON to protobuf message
    // This is a simplified conversion - in production you'd want more robust JSON->protobuf conversion
    json_to_otlp_request(json_value)
}

/// Convert JSON Value to OTLP ExportTraceServiceRequest.
fn json_to_otlp_request(json: Value) -> std::result::Result<ExportTraceServiceRequest, HttpError> {
    use opentelemetry_proto::tonic::{
        collector::trace::v1::ExportTraceServiceRequest,
        common::v1::{AnyValue, InstrumentationScope, KeyValue},
        resource::v1::Resource,
        trace::v1::{ResourceSpans, ScopeSpans},
    };

    // Extract resourceSpans array
    let resource_spans_array = json
        .get("resourceSpans")
        .ok_or_else(|| HttpError::BadRequest("Missing 'resourceSpans' field".to_string()))?
        .as_array()
        .ok_or_else(|| HttpError::BadRequest("'resourceSpans' must be an array".to_string()))?;

    let mut resource_spans_vec = Vec::new();

    for resource_spans_json in resource_spans_array {
        // Parse resource
        let resource = if let Some(resource_json) = resource_spans_json.get("resource") {
            let mut attributes = Vec::new();

            if let Some(attrs) = resource_json.get("attributes").and_then(|v| v.as_array()) {
                for attr in attrs {
                    if let (Some(key), Some(value)) =
                        (attr.get("key").and_then(|k| k.as_str()), attr.get("value"))
                    {
                        let any_value = if let Some(str_val) =
                            value.get("stringValue").and_then(|v| v.as_str())
                        {
                            AnyValue {
                                value: Some(opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(str_val.to_string()))
                            }
                        } else if let Some(int_val) = value.get("intValue").and_then(|v| v.as_i64())
                        {
                            AnyValue {
                                value: Some(opentelemetry_proto::tonic::common::v1::any_value::Value::IntValue(int_val))
                            }
                        } else {
                            continue;
                        };

                        attributes.push(KeyValue {
                            key: key.to_string(),
                            value: Some(any_value),
                        });
                    }
                }
            }

            Some(Resource {
                attributes,
                dropped_attributes_count: 0,
            })
        } else {
            None
        };

        // Parse scope spans
        let empty_vec = Vec::new();
        let scope_spans_array = resource_spans_json
            .get("scopeSpans")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let mut scope_spans_vec = Vec::new();

        for scope_spans_json in scope_spans_array {
            // Parse scope
            let scope = scope_spans_json.get("scope").map(|scope_json| {
                InstrumentationScope {
                    name: scope_json
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    version: scope_json
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    attributes: Vec::new(), // Simplified - could parse attributes here too
                    dropped_attributes_count: 0,
                }
            });

            // Parse spans
            let empty_spans_vec = Vec::new();
            let spans_array = scope_spans_json
                .get("spans")
                .and_then(|v| v.as_array())
                .unwrap_or(&empty_spans_vec);

            let mut spans_vec = Vec::new();

            for span_json in spans_array {
                let span = json_to_span(span_json)?;
                spans_vec.push(span);
            }

            scope_spans_vec.push(ScopeSpans {
                scope,
                spans: spans_vec,
                schema_url: "".to_string(),
            });
        }

        resource_spans_vec.push(ResourceSpans {
            resource,
            scope_spans: scope_spans_vec,
            schema_url: "".to_string(),
        });
    }

    Ok(ExportTraceServiceRequest {
        resource_spans: resource_spans_vec,
    })
}

/// Convert JSON span to protobuf Span.
fn json_to_span(
    span_json: &Value,
) -> std::result::Result<opentelemetry_proto::tonic::trace::v1::Span, HttpError> {
    use opentelemetry_proto::tonic::{
        common::v1::{AnyValue, KeyValue},
        trace::v1::Span,
    };

    // Extract required fields
    let trace_id = span_json
        .get("traceId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| HttpError::BadRequest("Missing 'traceId' in span".to_string()))?;

    let span_id = span_json
        .get("spanId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| HttpError::BadRequest("Missing 'spanId' in span".to_string()))?;

    let name = span_json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Convert hex strings to bytes
    let trace_id_bytes = hex::decode(trace_id)
        .map_err(|_| HttpError::BadRequest("Invalid trace ID format".to_string()))?;
    let span_id_bytes = hex::decode(span_id)
        .map_err(|_| HttpError::BadRequest("Invalid span ID format".to_string()))?;

    let parent_span_id_bytes =
        if let Some(parent_id) = span_json.get("parentSpanId").and_then(|v| v.as_str()) {
            hex::decode(parent_id)
                .map_err(|_| HttpError::BadRequest("Invalid parent span ID format".to_string()))?
        } else {
            Vec::new()
        };

    // Parse timestamps
    let start_time_unix_nano = span_json
        .get("startTimeUnixNano")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let end_time_unix_nano = span_json
        .get("endTimeUnixNano")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Parse attributes
    let mut attributes = Vec::new();
    if let Some(attrs) = span_json.get("attributes").and_then(|v| v.as_array()) {
        for attr in attrs {
            if let (Some(key), Some(value)) =
                (attr.get("key").and_then(|k| k.as_str()), attr.get("value"))
            {
                let any_value =
                    if let Some(str_val) = value.get("stringValue").and_then(|v| v.as_str()) {
                        AnyValue {
                        value: Some(
                            opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(
                                str_val.to_string(),
                            ),
                        ),
                    }
                    } else {
                        continue;
                    };

                attributes.push(KeyValue {
                    key: key.to_string(),
                    value: Some(any_value),
                });
            }
        }
    }

    // Parse kind
    let kind = span_json.get("kind").and_then(|v| v.as_u64()).unwrap_or(0) as i32;

    Ok(Span {
        trace_id: trace_id_bytes,
        span_id: span_id_bytes,
        parent_span_id: parent_span_id_bytes,
        name,
        kind,
        start_time_unix_nano,
        end_time_unix_nano,
        attributes,
        dropped_attributes_count: 0,
        events: Vec::new(),
        dropped_events_count: 0,
        links: Vec::new(),
        dropped_links_count: 0,
        status: None,
        trace_state: "".to_string(),
        flags: 0,
    })
}

/// Process OTLP export request and convert to Urpo spans.
fn process_export_request(
    export_request: ExportTraceServiceRequest,
) -> std::result::Result<Vec<crate::core::Span>, HttpError> {
    let mut spans = Vec::new();
    let mut total_resource_spans = 0;
    let mut total_scope_spans = 0;
    let mut total_spans = 0;

    // Process resource spans (same logic as gRPC implementation)
    for resource_spans in export_request.resource_spans {
        total_resource_spans += 1;
        let resource = resource_spans.resource.unwrap_or_default();
        let service_name = extract_service_name(&resource.attributes);

        tracing::debug!(
            "Processing HTTP resource spans for service: {}, scope_spans count: {}",
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
                "Processing HTTP scope: {}, spans count: {}",
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
                            "Converted HTTP span: service={}, operation={}, trace_id={}, span_id={}",
                            service_name, span_name, trace_id_hex, span_id_hex
                        );
                        spans.push(span);
                    },
                    Err(e) => {
                        tracing::warn!(
                            "Failed to convert HTTP span: service={}, operation={}, trace_id={}, span_id={}, error={}",
                            service_name, span_name, trace_id_hex, span_id_hex, e
                        );
                    },
                }
            }
        }
    }

    tracing::info!(
        "Received HTTP OTEL export request: {} resource spans, {} scope spans, {} spans total, {} successfully converted",
        total_resource_spans,
        total_scope_spans,
        total_spans,
        spans.len()
    );

    Ok(spans)
}

/// Health check endpoint.
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "urpo-http-receiver",
        "endpoints": ["/v1/traces", "/health"]
    }))
}

/// Root handler.
async fn root_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "service": "Urpo OTLP HTTP Receiver",
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": {
            "/v1/traces": "POST - OTLP trace export",
            "/health": "GET - Health check"
        }
    }))
}

/// Handle OTLP metrics export requests - minimal implementation.
#[inline(always)]
async fn handle_metrics_v1() -> impl IntoResponse {
    tracing::debug!("Received HTTP metrics export request");

    // Zero-allocation OTLP success response
    const OTLP_SUCCESS: &str = r#"{"partialSuccess":null}"#;

    (StatusCode::OK, [("content-type", "application/json")], OTLP_SUCCESS)
}

/// HTTP-specific error type.
#[derive(Debug)]
pub enum HttpError {
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            HttpError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            HttpError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            HttpError::Internal(msg) => write!(f, "Internal Error: {}", msg),
        }
    }
}

impl std::error::Error for HttpError {}

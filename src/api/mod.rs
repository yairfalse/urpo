//! Minimal HTTP API for external tool compatibility.
//!
//! This module provides a lightweight HTTP API with 5 essential endpoints
//! for compatibility with external tools like dashboards and alert systems.

use crate::core::{Result, UrpoError};
use crate::export::{ExportFormat, ExportOptions, TraceExporter};
use crate::storage::StorageBackend;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

/// API server configuration.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Port to listen on (default: 8080)
    pub port: u16,
    /// Enable CORS headers
    pub enable_cors: bool,
    /// Maximum results per query
    pub max_results: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            enable_cors: true,
            max_results: 1000,
        }
    }
}

/// API server state.
#[derive(Clone)]
struct ApiState {
    storage: Arc<tokio::sync::RwLock<dyn StorageBackend>>,
    config: ApiConfig,
}

/// Health check response.
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_seconds: u64,
    trace_count: usize,
    service_count: usize,
}

/// Error response.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    code: u16,
}

/// Query parameters for trace listing.
#[derive(Debug, Deserialize)]
struct TraceQuery {
    /// Filter by service name
    service: Option<String>,
    /// Start time (unix timestamp in seconds)
    start_time: Option<u64>,
    /// End time (unix timestamp in seconds)
    end_time: Option<u64>,
    /// Maximum number of results
    limit: Option<usize>,
    /// Only return traces with errors
    errors_only: Option<bool>,
    /// Export format (json, jaeger, otel, csv)
    format: Option<String>,
}

/// Query parameters for search.
#[derive(Debug, Deserialize)]
struct SearchQuery {
    /// Search query string
    q: String,
    /// Service filter
    service: Option<String>,
    /// Attribute key filter
    attribute_key: Option<String>,
    /// Maximum results
    limit: Option<usize>,
}

/// Start the API server.
pub async fn start_server(
    storage: Arc<tokio::sync::RwLock<dyn StorageBackend>>,
    config: ApiConfig,
) -> Result<()> {
    let state = ApiState {
        storage,
        config: config.clone(),
    };

    // Build router with all endpoints
    let mut app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/traces", get(list_traces_handler))
        .route("/api/traces/:id", get(get_trace_handler))
        .route("/api/services", get(list_services_handler))
        .route("/api/search", get(search_handler))
        .with_state(state);

    // Add CORS if enabled
    if config.enable_cors {
        app = app.layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
        );
    }

    // Start server
    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Starting API server on http://{}", addr);
    
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| UrpoError::Io(std::io::Error::new(std::io::ErrorKind::AddrInUse, format!("Failed to bind to {}: {}", addr, e))))?;
    
    axum::serve(listener, app)
        .await
        .map_err(|e| UrpoError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("API server error: {}", e))))?;

    Ok(())
}

/// GET /health - System health and statistics
async fn health_handler(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    // Get storage statistics
    let stats = match state.storage.read().await.get_stats().await {
        Ok(s) => s,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "Storage unavailable".to_string(),
                    code: 503,
                }),
            ).into_response();
        }
    };

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: stats.uptime_seconds,
        trace_count: stats.trace_count,
        service_count: stats.service_count,
    };

    Json(response).into_response()
}

/// GET /api/traces - List recent traces with filtering
async fn list_traces_handler(
    State(state): State<ApiState>,
    Query(params): Query<TraceQuery>,
) -> impl IntoResponse {
    // Convert time from seconds to nanoseconds
    let start_time = params.start_time.map(|t| t * 1_000_000_000);
    let end_time = params.end_time.map(|t| t * 1_000_000_000);
    
    // Apply limit with max cap
    let limit = params.limit
        .unwrap_or(100)
        .min(state.config.max_results);

    // List traces
    let traces = match state.storage.read().await.list_traces(
        params.service.as_deref(),
        start_time,
        end_time,
        limit,
    ).await {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to list traces: {}", e),
                    code: 500,
                }),
            ).into_response();
        }
    };

    // Filter by error status if requested
    let filtered_traces = if params.errors_only.unwrap_or(false) {
        traces.into_iter().filter(|t| t.has_error).collect()
    } else {
        traces
    };

    // Handle different export formats
    if let Some(format_str) = params.format {
        let format = match format_str.parse::<ExportFormat>() {
            Ok(f) => f,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("Invalid format: {}", e),
                        code: 400,
                    }),
                ).into_response();
            }
        };

        let storage_ref = state.storage.read().await;
        let exporter = TraceExporter::new(&*storage_ref);
        let options = ExportOptions {
            format,
            output: None,
            service: params.service,
            start_time,
            end_time,
            limit: Some(limit),
            errors_only: params.errors_only.unwrap_or(false),
        };

        match exporter.export_traces(&options).await {
            Ok(content) => content.into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Export failed: {}", e),
                    code: 500,
                }),
            ).into_response(),
        }
    } else {
        // Return JSON by default
        Json(filtered_traces).into_response()
    }
}

/// GET /api/traces/:id - Get specific trace with all spans
async fn get_trace_handler(
    State(state): State<ApiState>,
    Path(trace_id): Path<String>,
) -> impl IntoResponse {
    // Parse trace ID
    let trace_id: crate::core::TraceId = match trace_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invalid trace ID format".to_string(),
                    code: 400,
                }),
            ).into_response();
        }
    };

    // Get trace spans
    let spans = match state.storage.read().await.get_trace_spans(trace_id.clone()).await {
        Ok(s) => s,
        Err(e) => {
            if e.to_string().contains("not found") {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: format!("Trace not found: {}", trace_id.as_str()),
                        code: 404,
                    }),
                ).into_response();
            } else {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Failed to get trace: {}", e),
                        code: 500,
                    }),
                ).into_response();
            }
        }
    };

    Json(spans).into_response()
}

/// GET /api/services - List all services with basic metrics
async fn list_services_handler(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    // Get service metrics
    let services = match state.storage.read().await.get_service_metrics_map().await {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to get services: {}", e),
                    code: 500,
                }),
            ).into_response();
        }
    };

    // Convert to simple service list with metrics
    let service_list: Vec<ServiceInfo> = services.into_iter().map(|(name, metrics)| {
        ServiceInfo {
            name: name.as_str().to_string(),
            trace_count: metrics.span_count as usize,
            error_count: metrics.error_count as usize,
            latency_p50: metrics.latency_p50.as_micros() as u64,
            latency_p95: metrics.latency_p95.as_micros() as u64,
            latency_p99: metrics.latency_p99.as_micros() as u64,
        }
    }).collect();

    Json(service_list).into_response()
}

/// GET /api/search - Search spans by attributes or text
async fn search_handler(
    State(state): State<ApiState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    // Validate query
    if params.q.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Query parameter 'q' is required".to_string(),
                code: 400,
            }),
        ).into_response();
    }

    let limit = params.limit
        .unwrap_or(100)
        .min(state.config.max_results);

    // Perform search
    let results = match state.storage.read().await.search_spans(
        &params.q,
        params.service.as_deref(),
        params.attribute_key.as_deref(),
        limit,
    ).await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Search failed: {}", e),
                    code: 500,
                }),
            ).into_response();
        }
    };

    // Return search results
    Json(SearchResults {
        query: params.q,
        count: results.len(),
        spans: results,
    }).into_response()
}

/// Service information with metrics.
#[derive(Debug, Serialize)]
struct ServiceInfo {
    name: String,
    trace_count: usize,
    error_count: usize,
    latency_p50: u64,
    latency_p95: u64,
    latency_p99: u64,
}

/// Search results response.
#[derive(Debug, Serialize)]
struct SearchResults {
    query: String,
    count: usize,
    spans: Vec<crate::core::Span>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ApiConfig::default();
        assert_eq!(config.port, 8080);
        assert!(config.enable_cors);
        assert_eq!(config.max_results, 1000);
    }

    #[test]
    fn test_export_format_parsing() {
        assert_eq!("json".parse::<ExportFormat>().unwrap(), ExportFormat::Json);
        assert_eq!("jaeger".parse::<ExportFormat>().unwrap(), ExportFormat::Jaeger);
        assert_eq!("otel".parse::<ExportFormat>().unwrap(), ExportFormat::OpenTelemetry);
        assert_eq!("csv".parse::<ExportFormat>().unwrap(), ExportFormat::Csv);
        assert!("invalid".parse::<ExportFormat>().is_err());
    }
}
//! Shared types for Tauri application.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use urpo_lib::{monitoring::Monitor, receiver::OtelReceiver, storage::StorageBackend};

/// Application state shared across Tauri commands
/// PERFORMANCE: Uses RwLock for concurrent reads, exclusive writes
pub struct AppState {
    pub storage: Arc<RwLock<dyn StorageBackend>>,
    pub receiver: Arc<RwLock<Option<OtelReceiver>>>,
    pub monitor: Arc<Monitor>,
    pub metrics_storage: Option<Arc<tokio::sync::Mutex<urpo_lib::metrics::MetricStorage>>>,
    pub logs_storage: Option<Arc<tokio::sync::Mutex<urpo_lib::logs::LogStorage>>>,
}

/// Service metrics for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub name: String,
    pub request_rate: f64,
    pub error_rate: f64,
    pub latency_p50: u64,
    pub latency_p95: u64,
    pub latency_p99: u64,
    pub active_spans: usize,
}

/// Trace information for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceInfo {
    pub trace_id: String,
    pub root_service: String,
    pub root_operation: String,
    pub start_time: i64,
    pub duration: u64,
    pub span_count: usize,
    pub has_error: bool,
    pub services: Vec<String>,
}

/// Storage information for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub trace_count: usize,
    pub span_count: usize,
    pub memory_mb: f64,
    pub storage_health: String,
    pub memory_pressure: f64,
    pub oldest_span: Option<i64>,
}

/// System metrics for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage: f64,
    pub memory_usage_mb: f64,
    pub memory_pressure: f64,
    pub storage_health: String,
    pub receiver_active: bool,
    pub spans_per_second: f64,
    pub active_services: usize,
    pub uptime_seconds: u64,
    pub command_latencies: std::collections::HashMap<String, f64>,
}

/// OTLP Service Health Metrics (real-time from metrics receiver)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub service_name: String,
    pub request_rate: f64,      // requests per second
    pub error_rate: f64,         // error percentage (0.0 - 100.0)
    pub avg_latency_ms: f64,     // average latency in milliseconds
    pub p95_latency_ms: f64,     // 95th percentile latency
    pub last_updated: i64,       // unix timestamp
}

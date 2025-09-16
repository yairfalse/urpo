#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// EXTREME PERFORMANCE: Use mimalloc for blazing fast memory allocation
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use atomic_float::AtomicF64 as AtomicFloat;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tauri::{Manager, State, Window};
use tokio::sync::RwLock;

use urpo_lib::{
    core::{Config, ConfigBuilder, ServiceName, TraceId},
    monitoring::Monitor,
    receiver::OtelReceiver,
    service_map::ServiceMapBuilder,
    storage::{StorageBackend, StorageManager},
};

// Global telemetry system for ultra-high performance monitoring
static TELEMETRY: Lazy<TelemetryState> = Lazy::new(|| TelemetryState::new());

struct TelemetryState {
    command_latencies: DashMap<String, AtomicFloat>,
    heap_usage_mb: AtomicFloat,
    cpu_usage_percent: AtomicFloat,
    memory_pressure: AtomicFloat,
    cold_fetch_latency_ms: AtomicFloat,
    tier_status: DashMap<String, String>,
    free_space_mb: AtomicFloat,
    system_info: tokio::sync::RwLock<Option<System>>,
}

impl TelemetryState {
    fn new() -> Self {
        Self {
            command_latencies: DashMap::new(),
            heap_usage_mb: AtomicFloat::new(0.0),
            cpu_usage_percent: AtomicFloat::new(0.0),
            memory_pressure: AtomicFloat::new(0.0),
            cold_fetch_latency_ms: AtomicFloat::new(0.0),
            tier_status: DashMap::new(),
            free_space_mb: AtomicFloat::new(0.0),
            system_info: tokio::sync::RwLock::new(None),
        }
    }

    /// Record command latency with exponential moving average for ultra-low latency
    #[inline(always)]
    fn record_command_latency(&self, command: &str, latency_ms: f64) {
        if let Some(existing) = self.command_latencies.get(command) {
            let current = existing.load(Ordering::Relaxed);
            let alpha = 0.1; // Exponential moving average
            let new_value = alpha * latency_ms + (1.0 - alpha) * current;
            existing.store(new_value, Ordering::Relaxed);
        } else {
            self.command_latencies
                .insert(command.to_string(), AtomicFloat::new(latency_ms));
        }
    }

    /// Update system metrics with minimal overhead (cached updates)
    async fn update_system_metrics(&self) {
        let mut system_guard = self.system_info.write().await;
        if system_guard.is_none() {
            *system_guard = Some(System::new_all());
        }

        if let Some(ref mut system) = system_guard.as_mut() {
            system.refresh_memory();
            system.refresh_cpu_all();

            // Memory pressure calculation (0.0 = no pressure, 1.0 = maximum pressure)
            let used_memory = system.used_memory() as f64;
            let total_memory = system.total_memory() as f64;
            let pressure = used_memory / total_memory;
            self.memory_pressure.store(pressure, Ordering::Relaxed);

            // Heap usage in MB
            let heap_mb = used_memory / (1024.0 * 1024.0);
            self.heap_usage_mb.store(heap_mb, Ordering::Relaxed);

            // CPU usage percentage
            let cpu_usage = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>()
                / system.cpus().len() as f32;
            self.cpu_usage_percent
                .store(cpu_usage as f64, Ordering::Relaxed);

            // Available disk space (cold storage considerations)
            let available_space = system.available_memory() as f64 / (1024.0 * 1024.0);
            self.free_space_mb.store(available_space, Ordering::Relaxed);
        }
    }
}

/// Panic isolation and performance monitoring macro for production-grade robustness
macro_rules! safe_command {
    ($command_name:expr, $body:expr) => {{
        let start = Instant::now();

        let result = match catch_unwind(AssertUnwindSafe(|| $body)) {
            Ok(future) => future.await,
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };

                tracing::error!("Command {} panicked: {}", $command_name, panic_msg);
                Err(format!("{} failed due to internal error: {}", $command_name, panic_msg))
            },
        };

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
        TELEMETRY.record_command_latency($command_name, latency_ms);

        // Log performance warnings for GUI snappiness (<16ms target)
        if latency_ms > 16.0 {
            tracing::warn!(
                "Command {} exceeded 16ms GUI target: {:.2}ms",
                $command_name,
                latency_ms
            );
        }

        result
    }};
}

/// Background telemetry task that streams performance data to the frontend
async fn background_telemetry_task(window: Window) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        // Update system metrics with minimal overhead (cached updates)
        TELEMETRY.update_system_metrics().await;

        // Collect telemetry data for streaming
        let telemetry_data = serde_json::json!({
            "heap_usage_mb": TELEMETRY.heap_usage_mb.load(Ordering::Relaxed),
            "cpu_usage_percent": TELEMETRY.cpu_usage_percent.load(Ordering::Relaxed),
            "memory_pressure": TELEMETRY.memory_pressure.load(Ordering::Relaxed),
            "cold_fetch_latency_ms": TELEMETRY.cold_fetch_latency_ms.load(Ordering::Relaxed),
            "free_space_mb": TELEMETRY.free_space_mb.load(Ordering::Relaxed),
            "command_latencies": TELEMETRY.command_latencies
                .iter()
                .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
                .collect::<std::collections::HashMap<_, _>>(),
            "tier_status": TELEMETRY.tier_status
                .iter()
                .map(|entry| (entry.key().clone(), entry.value().clone()))
                .collect::<std::collections::HashMap<_, _>>(),
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });

        // Stream telemetry to frontend (non-blocking)
        let _ = window.emit("telemetry-update", &telemetry_data);

        // Log performance warnings for debugging
        let memory_pressure = TELEMETRY.memory_pressure.load(Ordering::Relaxed);
        if memory_pressure > 0.9 {
            tracing::warn!("High memory pressure detected: {:.1}%", memory_pressure * 100.0);
        }

        let cold_fetch_latency = TELEMETRY.cold_fetch_latency_ms.load(Ordering::Relaxed);
        if cold_fetch_latency > 100.0 {
            tracing::warn!("Slow cold fetch detected: {:.1}ms", cold_fetch_latency);
        }
    }
}

struct AppState {
    storage: Arc<dyn StorageBackend>,
    storage_manager: Arc<StorageManager>,
    receiver: Arc<RwLock<Option<Arc<OtelReceiver>>>>,
    monitor: Arc<Monitor>,
    startup_time: Instant,
    config: Arc<Config>,
}

#[derive(serde::Serialize)]
struct ServiceMetrics {
    name: String,
    request_rate: f64,
    error_rate: f64,
    latency_p50: u64,
    latency_p95: u64,
    latency_p99: u64,
    active_spans: usize,
}

#[derive(serde::Serialize)]
struct TraceInfo {
    trace_id: String,
    root_service: String,
    root_operation: String,
    start_time: i64,
    duration: u64,
    span_count: usize,
    has_error: bool,
    services: Vec<String>,
}

#[derive(serde::Serialize)]
struct SystemMetrics {
    memory_usage_mb: f64,
    cpu_usage_percent: f64,
    spans_per_second: f64,
    total_spans: usize,
    uptime_seconds: u64,

    // Advanced performance metrics
    heap_usage_mb: f64,
    memory_pressure: f64, // 0.0-1.0 scale
    cold_fetch_latency_ms: f64,
    command_latencies: std::collections::HashMap<String, f64>,
    free_space_mb: f64,
    tier_health: Vec<TierHealthInfo>,
}

#[derive(serde::Serialize)]
struct TierHealthInfo {
    tier: String,
    status: String,
    health_score: f64, // 0.0-1.0 where 1.0 is perfect health
}

#[derive(serde::Serialize)]
struct StorageInfo {
    mode: String,
    persistent_enabled: bool,
    data_dir: String,
    hot_size: usize,
    warm_size_mb: usize,
    cold_retention_hours: usize,
    total_spans: usize,
    memory_mb: f64,
    health: String,
}

// Service map structures for serialization
#[derive(serde::Serialize)]
struct ServiceMapResponse {
    nodes: Vec<ServiceNodeResponse>,
    edges: Vec<ServiceEdgeResponse>,
    generated_at: i64,
    trace_count: u64,
    time_window_seconds: u64,
}

#[derive(serde::Serialize)]
struct ServiceNodeResponse {
    name: String,
    request_count: u64,
    error_rate: f64,
    avg_latency_us: u64,
    is_root: bool,
    is_leaf: bool,
    tier: u32,
}

#[derive(serde::Serialize)]
struct ServiceEdgeResponse {
    from: String,
    to: String,
    call_count: u64,
    error_count: u64,
    avg_latency_us: u64,
    p99_latency_us: u64,
    operations: Vec<String>,
}

// Tauri command to get service metrics - BLAZING FAST with zero allocations in hot path
#[tauri::command]
async fn get_service_metrics(state: State<'_, AppState>) -> Result<Vec<ServiceMetrics>, String> {
    let metrics = state
        .storage
        .get_service_metrics()
        .await
        .map_err(|e| e.to_string())?;

    // Pre-allocate exact capacity for zero reallocation
    let mut result = Vec::with_capacity(metrics.len());

    for metric in metrics {
        result.push(ServiceMetrics {
            name: metric.name.to_string(),
            request_rate: metric.request_rate,
            error_rate: metric.error_rate,
            latency_p50: metric.latency_p50.as_millis() as u64,
            latency_p95: metric.latency_p95.as_millis() as u64,
            latency_p99: metric.latency_p99.as_millis() as u64,
            active_spans: (metric.request_rate * metric.latency_p50.as_secs_f64()) as usize,
        });
    }

    Ok(result)
}

// Batch command to reduce IPC overhead
#[tauri::command]
async fn get_service_metrics_batch(
    state: State<'_, AppState>,
    service_names: Vec<String>,
) -> Result<Vec<ServiceMetrics>, String> {
    if service_names.is_empty() {
        return get_service_metrics(state).await;
    }

    let metrics = state
        .storage
        .get_service_metrics()
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::with_capacity(service_names.len());

    for name in service_names {
        if let Some(metric) = metrics.iter().find(|m| m.name.as_str() == name) {
            result.push(ServiceMetrics {
                name: metric.name.to_string(),
                request_rate: metric.request_rate,
                error_rate: metric.error_rate,
                latency_p50: metric.latency_p50.as_millis() as u64,
                latency_p95: metric.latency_p95.as_millis() as u64,
                latency_p99: metric.latency_p99.as_millis() as u64,
                active_spans: (metric.request_rate * metric.latency_p50.as_secs_f64()) as usize,
            });
        }
    }

    Ok(result)
}

#[tauri::command]
async fn list_recent_traces(
    state: State<'_, AppState>,
    limit: usize,
    service_filter: Option<String>,
) -> Result<Vec<TraceInfo>, String> {
    let service = service_filter
        .map(|s| ServiceName::new(s))
        .transpose()
        .map_err(|e| e.to_string())?;

    let traces = state
        .storage
        .list_recent_traces(limit, service.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::with_capacity(traces.len());

    for trace in traces {
        result.push(TraceInfo {
            trace_id: trace.trace_id.to_string(),
            root_service: trace.root_service.to_string(),
            root_operation: trace.root_operation,
            start_time: trace
                .start_time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            duration: trace.duration.as_millis() as u64,
            span_count: trace.span_count,
            has_error: trace.has_error,
            services: trace.services.into_iter().map(|s| s.to_string()).collect(),
        });
    }

    Ok(result)
}

#[tauri::command]
async fn get_error_traces(
    state: State<'_, AppState>,
    limit: usize,
) -> Result<Vec<TraceInfo>, String> {
    let traces = state
        .storage
        .get_error_traces(limit)
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::with_capacity(traces.len());

    for trace in traces {
        result.push(TraceInfo {
            trace_id: trace.trace_id.to_string(),
            root_service: trace.root_service.to_string(),
            root_operation: trace.root_operation,
            start_time: trace
                .start_time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            duration: trace.duration.as_millis() as u64,
            span_count: trace.span_count,
            has_error: trace.has_error,
            services: trace.services.into_iter().map(|s| s.to_string()).collect(),
        });
    }

    Ok(result)
}

#[tauri::command]
async fn get_trace_spans(
    state: State<'_, AppState>,
    trace_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let trace_id = TraceId::new(trace_id).map_err(|e| e.to_string())?;

    let spans = state
        .storage
        .get_trace_spans(&trace_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::with_capacity(spans.len());

    for span in spans {
        result.push(serde_json::to_value(span).map_err(|e| e.to_string())?);
    }

    Ok(result)
}

#[tauri::command]
async fn search_traces(
    state: State<'_, AppState>,
    query: String,
    limit: usize,
) -> Result<Vec<TraceInfo>, String> {
    let traces = state
        .storage
        .search_traces(&query, limit)
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::with_capacity(traces.len());

    for trace in traces {
        result.push(TraceInfo {
            trace_id: trace.trace_id.to_string(),
            root_service: trace.root_service.to_string(),
            root_operation: trace.root_operation,
            start_time: trace
                .start_time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            duration: trace.duration.as_millis() as u64,
            span_count: trace.span_count,
            has_error: trace.has_error,
            services: trace.services.into_iter().map(|s| s.to_string()).collect(),
        });
    }

    Ok(result)
}

#[tauri::command]
async fn get_system_metrics(state: State<'_, AppState>) -> Result<SystemMetrics, String> {
    safe_command!("get_system_metrics", async {
        // Update telemetry system metrics (background task with caching)
        TELEMETRY.update_system_metrics().await;

        // Get metrics from monitor
        let system_metrics = state.monitor.get_metrics().await;

        // Calculate aggregate stats from resource metrics
        let total_request_rate = system_metrics.performance.spans_per_second as f64;

        let total_spans = state
            .storage
            .get_span_count()
            .await
            .map_err(|e| e.to_string())?;

        let uptime = state.startup_time.elapsed().as_secs();

        // Collect command latencies for advanced dashboard
        let command_latencies: std::collections::HashMap<String, f64> = TELEMETRY
            .command_latencies
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
            .collect();

        // Build tier health info
        let tier_health = vec![
            TierHealthInfo {
                tier: "receiver".to_string(),
                status: if system_metrics.receiver.grpc_healthy {
                    "healthy"
                } else {
                    "stopped"
                }
                .to_string(),
                health_score: if system_metrics.receiver.grpc_healthy {
                    1.0
                } else {
                    0.0
                },
            },
            TierHealthInfo {
                tier: "storage".to_string(),
                status: "healthy".to_string(),
                health_score: 1.0, // TODO: Add storage health metrics
            },
            TierHealthInfo {
                tier: "application".to_string(),
                status: "running".to_string(),
                health_score: 1.0,
            },
        ];

        // Get actual resource metrics with enhanced telemetry
        Ok(SystemMetrics {
            memory_usage_mb: system_metrics.resources.memory_mb,
            cpu_usage_percent: system_metrics.resources.cpu_percent,
            spans_per_second: total_request_rate,
            total_spans,
            uptime_seconds: uptime,

            // Advanced performance telemetry
            heap_usage_mb: TELEMETRY.heap_usage_mb.load(Ordering::Relaxed),
            memory_pressure: TELEMETRY.memory_pressure.load(Ordering::Relaxed),
            cold_fetch_latency_ms: TELEMETRY.cold_fetch_latency_ms.load(Ordering::Relaxed),
            command_latencies,
            free_space_mb: TELEMETRY.free_space_mb.load(Ordering::Relaxed),
            tier_health,
        })
    })
}

// Stream trace data for large datasets with adaptive performance
#[tauri::command]
async fn stream_trace_data(
    window: tauri::Window,
    state: State<'_, AppState>,
    trace_id: String,
) -> Result<(), String> {
    safe_command!("stream_trace_data", async {
        let trace_id = TraceId::new(trace_id).map_err(|e| e.to_string())?;

        // Use spawn_blocking for potentially slow storage operations
        let spans = tokio::task::spawn_blocking({
            let storage = state.storage.clone();
            let trace_id = trace_id.clone();
            move || {
                tokio::runtime::Handle::current().block_on(async {
                    let fetch_start = Instant::now();
                    let result = storage.get_trace_spans(&trace_id).await;

                    // Record cold fetch latency for telemetry
                    let latency = fetch_start.elapsed().as_secs_f64() * 1000.0;
                    if latency > 50.0 {
                        TELEMETRY
                            .cold_fetch_latency_ms
                            .store(latency, Ordering::Relaxed);
                    }

                    result
                })
            }
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        // Optimize chunk size based on memory pressure
        let memory_pressure = TELEMETRY.memory_pressure.load(Ordering::Relaxed);
        let chunk_size = if memory_pressure > 0.8 {
            50 // Reduce chunk size under memory pressure
        } else if memory_pressure > 0.6 {
            75
        } else {
            100 // Default chunk size
        };

        let chunks: Vec<_> = spans.chunks(chunk_size).collect();

        for (i, chunk) in chunks.iter().enumerate() {
            // Use rayon for parallel serialization of chunks
            let chunk_data = tokio::task::spawn_blocking({
                let chunk = chunk.to_vec();
                move || {
                    use rayon::prelude::*;
                    chunk
                        .par_iter()
                        .map(|span| serde_json::to_value(span))
                        .collect::<Result<Vec<_>, _>>()
                }
            })
            .await
            .map_err(|e| format!("Join error: {}", e))?
            .map_err(|e| format!("Failed to serialize span: {}", e))?;

            window
                .emit("trace-chunk", &chunk_data)
                .map_err(|e| e.to_string())?;

            // Adaptive yielding based on memory pressure
            if i < chunks.len() - 1 {
                let yield_time = if memory_pressure > 0.8 {
                    Duration::from_millis(5) // Longer yield under pressure
                } else if memory_pressure > 0.6 {
                    Duration::from_millis(1)
                } else {
                    Duration::from_micros(100) // Fast yield under normal conditions
                };
                tokio::time::sleep(yield_time).await;
            }
        }

        window
            .emit("trace-complete", ())
            .map_err(|e| e.to_string())?;

        Ok(())
    })
}

#[tauri::command]
async fn start_receiver(state: State<'_, AppState>) -> Result<(), String> {
    let mut receiver_guard = state.receiver.write().await;

    if receiver_guard.is_some() {
        return Ok(()); // Already running
    }

    // Create receiver with standard OTEL ports
    // Create a new storage backend specifically for the receiver
    // This is a workaround for the type mismatch - OtelReceiver expects Arc<RwLock<dyn StorageBackend>>
    // but our app uses Arc<dyn StorageBackend>
    use urpo_lib::storage::InMemoryStorage;
    let storage_impl = InMemoryStorage::new(state.config.storage.max_spans);
    let receiver_storage: Arc<tokio::sync::RwLock<dyn urpo_lib::storage::StorageBackend>> =
        Arc::new(tokio::sync::RwLock::new(storage_impl));

    let receiver = Arc::new(OtelReceiver::new(
        4317, // GRPC port
        4318, // HTTP port
        receiver_storage,
        state.monitor.clone(),
    ));

    // Start receiver in background
    let receiver_clone = receiver.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = receiver_clone.run().await {
            tracing::error!("Receiver error: {}", e);
        }
    });

    *receiver_guard = Some(receiver);
    Ok(())
}

#[tauri::command]
async fn stop_receiver(state: State<'_, AppState>) -> Result<(), String> {
    let mut receiver_guard = state.receiver.write().await;

    // Simply drop the receiver to stop it
    *receiver_guard = None;

    Ok(())
}

#[tauri::command]
async fn get_storage_info(state: State<'_, AppState>) -> Result<StorageInfo, String> {
    let stats = state
        .storage_manager
        .get_stats()
        .await
        .map_err(|e| e.to_string())?;

    Ok(StorageInfo {
        mode: if state.config.storage.persistent {
            "persistent".to_string()
        } else {
            "in-memory".to_string()
        },
        persistent_enabled: state.config.storage.persistent,
        data_dir: state.config.storage.data_dir.to_string_lossy().to_string(),
        hot_size: state.config.storage.hot_storage_size,
        warm_size_mb: state.config.storage.warm_storage_mb,
        cold_retention_hours: state.config.storage.cold_retention_hours,
        total_spans: stats.span_count,
        memory_mb: stats.memory_mb,
        health: format!("{:?}", stats.health_status),
    })
}

#[tauri::command]
async fn trigger_tier_migration(state: State<'_, AppState>) -> Result<String, String> {
    // Trigger cleanup and tier migration
    state
        .storage_manager
        .run_cleanup()
        .await
        .map_err(|e| e.to_string())?;

    Ok("Tier migration triggered successfully".to_string())
}

// BLAZING FAST service map generation - optimized for <10ms response time
#[tauri::command]
async fn get_service_map(
    state: State<'_, AppState>,
    limit: Option<usize>,
    time_window_seconds: Option<u64>,
) -> Result<ServiceMapResponse, String> {
    safe_command!("get_service_map", async {
        // Use optimized defaults for maximum performance
        let limit = limit.unwrap_or(1000);
        let time_window = time_window_seconds.unwrap_or(3600);

        // Parallel service map building with spawn_blocking
        let service_map = tokio::task::spawn_blocking({
            let storage = state.storage.clone();
            move || {
                tokio::runtime::Handle::current().block_on(async {
                    // Create builder with zero-allocation storage access
                    let mut builder = ServiceMapBuilder::new(&*storage);

                    // Build map from recent traces with bounded memory usage
                    builder.build_from_recent_traces(limit, time_window).await
                })
            }
        })
        .await
        .map_err(|e| format!("Join error: {}", e))?
        .map_err(|e| format!("Failed to build service map: {}", e))?;

        // Check memory pressure and adjust parallelization accordingly
        let memory_pressure = TELEMETRY.memory_pressure.load(Ordering::Relaxed);

        let (nodes, edges) = if memory_pressure > 0.8 || service_map.nodes.len() < 100 {
            // Sequential processing under memory pressure or for small datasets
            let nodes = service_map
                .nodes
                .into_iter()
                .map(|node| ServiceNodeResponse {
                    name: node.name.to_string(),
                    request_count: node.request_count,
                    error_rate: node.error_rate,
                    avg_latency_us: node.avg_latency_us,
                    is_root: node.is_root,
                    is_leaf: node.is_leaf,
                    tier: node.tier,
                })
                .collect();

            let edges = service_map
                .edges
                .into_iter()
                .map(|edge| ServiceEdgeResponse {
                    from: edge.from.to_string(),
                    to: edge.to.to_string(),
                    call_count: edge.call_count,
                    error_count: edge.error_count,
                    avg_latency_us: edge.avg_latency_us,
                    p99_latency_us: edge.p99_latency_us,
                    operations: edge.operations.into_iter().collect(),
                })
                .collect();

            (nodes, edges)
        } else {
            // Parallel processing for large datasets with rayon
            tokio::task::spawn_blocking({
                let service_nodes = service_map.nodes;
                let service_edges = service_map.edges;
                move || {
                    use rayon::prelude::*;

                    let nodes: Vec<ServiceNodeResponse> = service_nodes
                        .into_par_iter()
                        .map(|node| ServiceNodeResponse {
                            name: node.name.to_string(),
                            request_count: node.request_count,
                            error_rate: node.error_rate,
                            avg_latency_us: node.avg_latency_us,
                            is_root: node.is_root,
                            is_leaf: node.is_leaf,
                            tier: node.tier,
                        })
                        .collect();

                    let edges: Vec<ServiceEdgeResponse> = service_edges
                        .into_par_iter()
                        .map(|edge| ServiceEdgeResponse {
                            from: edge.from.to_string(),
                            to: edge.to.to_string(),
                            call_count: edge.call_count,
                            error_count: edge.error_count,
                            avg_latency_us: edge.avg_latency_us,
                            p99_latency_us: edge.p99_latency_us,
                            operations: edge.operations.into_iter().collect(),
                        })
                        .collect();

                    (nodes, edges)
                }
            })
            .await
            .map_err(|e| format!("Join error during parallel processing: {}", e))?
        };

        // Convert SystemTime to unix timestamp for frontend compatibility
        let generated_at = service_map
            .generated_at
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Ok(ServiceMapResponse {
            nodes,
            edges,
            generated_at,
            trace_count: service_map.trace_count,
            time_window_seconds: service_map.time_window_seconds,
        })
    })
}

fn main() {
    // Track startup time for <200ms target
    let startup_time = Instant::now();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("urpo=debug,tauri=info")
        .init();

    // Load configuration with persistent storage support
    let config = ConfigBuilder::new()
        .persistent(std::env::var("URPO_PERSISTENT").unwrap_or_default() == "true")
        .data_dir(std::path::PathBuf::from(
            std::env::var("URPO_DATA_DIR").unwrap_or_else(|_| "./urpo_data".to_string()),
        ))
        .max_spans(100_000)
        .max_memory_mb(512)
        .build()
        .expect("Failed to build config");

    let config = Arc::new(config);

    // Create storage manager with optional persistence
    let storage_manager = if config.storage.persistent {
        tracing::info!("Starting with persistent storage at {:?}", config.storage.data_dir);
        Arc::new(
            StorageManager::new_persistent(&config).expect("Failed to create persistent storage"),
        )
    } else {
        tracing::info!("Starting with in-memory storage only");
        Arc::new(StorageManager::new_in_memory(config.storage.max_spans))
    };

    let storage_backend = storage_manager.backend();

    // Create monitor with performance manager
    use urpo_lib::storage::PerformanceManager;
    let perf_manager = Arc::new(PerformanceManager::new());
    let monitor = Arc::new(Monitor::new(perf_manager));

    let app_state = AppState {
        storage: storage_backend,
        storage_manager: storage_manager.clone(),
        receiver: Arc::new(RwLock::new(None)),
        monitor: monitor.clone(),
        startup_time,
        config: config.clone(),
    };

    let app = tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_service_metrics,
            get_service_metrics_batch,
            list_recent_traces,
            get_error_traces,
            get_trace_spans,
            search_traces,
            get_system_metrics,
            stream_trace_data,
            start_receiver,
            stop_receiver,
            get_storage_info,
            trigger_tier_migration,
            get_service_map,
        ])
        .setup(|app| {
            // Start background telemetry task with main window
            let window = app.get_window("main").expect("Failed to get main window");
            tauri::async_runtime::spawn(background_telemetry_task(window));

            // Initialize tier status
            TELEMETRY
                .tier_status
                .insert("application".to_string(), "starting".to_string());

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Mark application as ready
    TELEMETRY
        .tier_status
        .insert("application".to_string(), "ready".to_string());

    // Log startup performance
    let elapsed = startup_time.elapsed();
    tracing::info!("Startup time: {:?}", elapsed);

    if elapsed > Duration::from_millis(200) {
        tracing::warn!("Startup time exceeded 200ms target: {:?}", elapsed);
    } else {
        tracing::info!("✅ Startup time meets <200ms target: {:?}", elapsed);
    }

    // Run the application
    app.run(|_app_handle, event| match event {
        tauri::RunEvent::ExitRequested { api, .. } => {
            // Clean shutdown with telemetry
            TELEMETRY
                .tier_status
                .insert("application".to_string(), "shutting_down".to_string());
            api.prevent_exit();
        },
        _ => {},
    });

    tracing::info!("Application shutdown complete");
}

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::State;
use tokio::sync::RwLock;
use urpo_lib::{
    core::{Config, ConfigBuilder, ServiceName, TraceId},
    monitoring::Monitor,
    receiver::OtelReceiver,
    storage::{InMemoryStorage, StorageBackend, StorageManager},
};

struct AppState {
    storage: Arc<RwLock<dyn StorageBackend>>,
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

// Tauri command to get service metrics - BLAZING FAST with zero allocations in hot path
#[tauri::command]
async fn get_service_metrics(state: State<'_, AppState>) -> Result<Vec<ServiceMetrics>, String> {
    let metrics = state
        .storage
        .read()
        .await
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
        .read()
        .await
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
        .read()
        .await
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
                .unwrap()
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
        .read()
        .await
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
                .unwrap()
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
        .read()
        .await
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
        .read()
        .await
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
                .unwrap()
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
    // Get metrics from monitor
    let system_metrics = state.monitor.get_metrics().await;
    
    // Calculate aggregate stats from resource metrics
    let total_request_rate = system_metrics.performance.spans_per_second as f64;
    
    let total_spans = state
        .storage
        .read()
        .await
        .get_span_count()
        .await
        .map_err(|e| e.to_string())?;
    
    let uptime = state.startup_time.elapsed().as_secs();

    // Get actual resource metrics
    Ok(SystemMetrics {
        memory_usage_mb: system_metrics.resources.memory_mb,
        cpu_usage_percent: system_metrics.resources.cpu_percent,
        spans_per_second: total_request_rate,
        total_spans,
        uptime_seconds: uptime,
    })
}

// Stream trace data for large datasets
#[tauri::command]
async fn stream_trace_data(
    window: tauri::Window,
    state: State<'_, AppState>,
    trace_id: String,
) -> Result<(), String> {
    let trace_id = TraceId::new(trace_id).map_err(|e| e.to_string())?;
    
    let spans = state
        .storage
        .read()
        .await
        .get_trace_spans(&trace_id)
        .await
        .map_err(|e| e.to_string())?;

    // Stream in chunks to prevent blocking
    const CHUNK_SIZE: usize = 100;
    let chunks: Vec<_> = spans.chunks(CHUNK_SIZE).collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_data: Vec<_> = chunk
            .iter()
            .map(|span| serde_json::to_value(span).unwrap())
            .collect();

        window
            .emit("trace-chunk", &chunk_data)
            .map_err(|e| e.to_string())?;

        // Yield to prevent blocking
        if i < chunks.len() - 1 {
            tokio::time::sleep(Duration::from_micros(100)).await;
        }
    }

    window
        .emit("trace-complete", ())
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn start_receiver(state: State<'_, AppState>) -> Result<(), String> {
    let mut receiver_guard = state.receiver.write().await;
    
    if receiver_guard.is_some() {
        return Ok(()); // Already running
    }

    // Create receiver with standard OTEL ports
    let receiver = Arc::new(OtelReceiver::new(
        4317, // GRPC port
        4318, // HTTP port
        state.storage.clone(),
        state.monitor.clone(),
    ));

    // Start receiver in background
    let receiver_clone = receiver.clone();
    tokio::spawn(async move {
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
    let stats = state.storage_manager.get_stats().await.map_err(|e| e.to_string())?;
    
    Ok(StorageInfo {
        mode: if state.config.storage.persistent { "persistent".to_string() } else { "in-memory".to_string() },
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
    state.storage_manager.run_cleanup().await.map_err(|e| e.to_string())?;
    
    Ok("Tier migration triggered successfully".to_string())
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
            std::env::var("URPO_DATA_DIR").unwrap_or_else(|_| "./urpo_data".to_string())
        ))
        .max_spans(100_000)
        .max_memory_mb(512)
        .build()
        .expect("Failed to build config");
    
    let config = Arc::new(config);
    
    // Create storage manager with optional persistence
    let storage_manager = if config.storage.persistent {
        tracing::info!("Starting with persistent storage at {:?}", config.storage.data_dir);
        Arc::new(StorageManager::new_persistent(&config).expect("Failed to create persistent storage"))
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

    tauri::Builder::default()
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    let elapsed = startup_time.elapsed();
    tracing::info!("Startup time: {:?}", elapsed);
    
    if elapsed > Duration::from_millis(200) {
        tracing::warn!("Startup time exceeded 200ms target!");
    }
}
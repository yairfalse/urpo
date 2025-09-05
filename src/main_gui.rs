#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::State;
use tokio::sync::RwLock;
use urpo_lib::{
    core::{ServiceName, TraceId},
    monitoring::SystemMonitor,
    receiver::OtelReceiver,
    storage::{InMemoryStorage, StorageBackend},
};

struct AppState {
    storage: Arc<InMemoryStorage>,
    receiver: Arc<RwLock<Option<OtelReceiver>>>,
    monitor: Arc<SystemMonitor>,
    startup_time: Instant,
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
            active_spans: metric.active_spans,
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
                active_spans: metric.active_spans,
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
    let memory_usage = state.monitor.get_memory_usage();
    let cpu_usage = state.monitor.get_cpu_usage();
    let spans_per_second = state.monitor.get_spans_per_second();
    let total_spans = state
        .storage
        .get_span_count()
        .await
        .map_err(|e| e.to_string())?;
    let uptime = state.startup_time.elapsed().as_secs();

    Ok(SystemMetrics {
        memory_usage_mb: memory_usage as f64 / (1024.0 * 1024.0),
        cpu_usage_percent: cpu_usage,
        spans_per_second,
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

    let receiver = OtelReceiver::new(state.storage.clone())
        .map_err(|e| e.to_string())?;

    // Start receiver in background
    let receiver_clone = receiver.clone();
    tokio::spawn(async move {
        if let Err(e) = receiver_clone.start().await {
            tracing::error!("Receiver error: {}", e);
        }
    });

    *receiver_guard = Some(receiver);
    Ok(())
}

#[tauri::command]
async fn stop_receiver(state: State<'_, AppState>) -> Result<(), String> {
    let mut receiver_guard = state.receiver.write().await;
    
    if let Some(receiver) = receiver_guard.take() {
        receiver.shutdown().await.map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

fn main() {
    // Track startup time for <200ms target
    let startup_time = Instant::now();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("urpo=debug,tauri=info")
        .init();

    // Create storage with performance settings
    let storage = Arc::new(InMemoryStorage::new(100_000));
    let monitor = Arc::new(SystemMonitor::new());
    
    let app_state = AppState {
        storage: storage.clone(),
        receiver: Arc::new(RwLock::new(None)),
        monitor: monitor.clone(),
        startup_time,
    };

    // Start monitoring in background
    let monitor_clone = monitor.clone();
    let storage_clone = storage.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            loop {
                monitor_clone.update_metrics(&storage_clone).await;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    });

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    let elapsed = startup_time.elapsed();
    tracing::info!("Startup time: {:?}", elapsed);
    
    if elapsed > Duration::from_millis(200) {
        tracing::warn!("Startup time exceeded 200ms target!");
    }
}
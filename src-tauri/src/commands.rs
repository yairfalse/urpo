//! Optimized Tauri command handlers with performance-focused macros.

use serde_json::Value;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{State, Window};

use crate::{AppState, ServiceMetrics, StorageInfo, TraceInfo};
use urpo_lib::core::{ServiceName, TraceId};

/// Macro for efficient error conversion with zero allocation where possible
#[macro_export]
macro_rules! map_err_str {
    ($expr:expr) => {
        $expr.map_err(|e| e.to_string())
    };
}

/// Macro for pre-allocated vector creation
#[macro_export]
macro_rules! preallocated_vec {
    ($capacity:expr) => {
        Vec::with_capacity($capacity)
    };
}

/// Macro for efficient timestamp conversion
#[inline(always)]
fn to_unix_secs(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Macro for converting storage TraceInfo to Tauri TraceInfo with zero-copy where possible
#[inline(always)]
fn convert_trace_info(trace: urpo_lib::storage::TraceInfo) -> TraceInfo {
    TraceInfo {
        trace_id: trace.trace_id.to_string(),
        root_service: trace.root_service.to_string(),
        root_operation: trace.root_operation,
        start_time: to_unix_secs(trace.start_time),
        duration: trace.duration.as_millis() as u64,
        span_count: trace.span_count,
        has_error: trace.has_error,
        services: trace.services.into_iter().map(|s| s.to_string()).collect(),
    }
}

/// Macro for common command pattern with timing and error handling
#[macro_export]
macro_rules! timed_command {
    ($name:expr, $body:expr) => {{
        let start = Instant::now();
        let result = $body;
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Record telemetry
        crate::TELEMETRY.record_command_latency($name, latency_ms);

        result
    }};
}

/// Macro for batch processing with pre-allocation
#[macro_export]
macro_rules! batch_convert {
    ($items:expr, $converter:expr) => {{
        let mut result = preallocated_vec!($items.len());
        for item in $items {
            result.push($converter(item));
        }
        result
    }};
}

// ============= Optimized Command Handlers =============

#[tauri::command]
#[inline]
pub async fn get_service_metrics(
    state: State<'_, AppState>,
) -> Result<Vec<ServiceMetrics>, String> {
    timed_command!("get_service_metrics", {
        let storage = state.storage.read().await;
        let metrics = map_err_str!(storage.get_service_metrics().await)?;

        Ok(batch_convert!(metrics, |metric: urpo_lib::core::ServiceMetrics| {
            ServiceMetrics {
                name: metric.name.to_string(),
                request_rate: metric.request_rate,
                error_rate: metric.error_rate,
                latency_p50: metric.latency_p50.as_millis() as u64,
                latency_p95: metric.latency_p95.as_millis() as u64,
                latency_p99: metric.latency_p99.as_millis() as u64,
                active_spans: (metric.request_rate * metric.latency_p50.as_secs_f64()) as usize,
            }
        }))
    })
}

#[tauri::command]
#[inline]
pub async fn get_service_metrics_batch(
    state: State<'_, AppState>,
    service_names: Vec<String>,
) -> Result<Vec<ServiceMetrics>, String> {
    if service_names.is_empty() {
        return get_service_metrics(state).await;
    }

    timed_command!("get_service_metrics_batch", {
        let storage = state.storage.read().await;
        let all_metrics = map_err_str!(storage.get_service_metrics().await)?;

        // Use HashMap for O(1) lookup instead of O(n) find
        let metrics_map: std::collections::HashMap<_, _> = all_metrics
            .into_iter()
            .map(|m| (m.name.as_str().to_string(), m))
            .collect();

        let mut result = preallocated_vec!(service_names.len());

        for name in service_names {
            if let Some(metric) = metrics_map.get(&name) {
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
    })
}

#[tauri::command]
#[inline]
pub async fn list_recent_traces(
    state: State<'_, AppState>,
    limit: usize,
    service_filter: Option<String>,
) -> Result<Vec<TraceInfo>, String> {
    timed_command!("list_recent_traces", {
        let service = service_filter
            .map(|s| ServiceName::new(s))
            .transpose()
            .map_err(|e| e.to_string())?;

        let storage = state.storage.read().await;
        let traces = map_err_str!(
            storage.list_recent_traces(limit, service.as_ref()).await
        )?;

        Ok(batch_convert!(traces, convert_trace_info))
    })
}

#[tauri::command]
#[inline]
pub async fn get_error_traces(
    state: State<'_, AppState>,
    limit: usize,
) -> Result<Vec<TraceInfo>, String> {
    timed_command!("get_error_traces", {
        let storage = state.storage.read().await;
        let traces = map_err_str!(storage.get_error_traces(limit).await)?;
        Ok(batch_convert!(traces, convert_trace_info))
    })
}

#[tauri::command]
#[inline]
pub async fn get_trace_spans(
    state: State<'_, AppState>,
    trace_id: String,
) -> Result<Vec<Value>, String> {
    timed_command!("get_trace_spans", {
        let trace_id = map_err_str!(TraceId::new(trace_id))?;
        let storage = state.storage.read().await;
        let spans = map_err_str!(storage.get_trace_spans(&trace_id).await)?;

        let mut result = preallocated_vec!(spans.len());
        for span in spans {
            result.push(map_err_str!(serde_json::to_value(span))?);
        }

        Ok(result)
    })
}

#[tauri::command]
#[inline]
pub async fn search_traces(
    state: State<'_, AppState>,
    query: String,
    limit: usize,
) -> Result<Vec<TraceInfo>, String> {
    timed_command!("search_traces", {
        let storage = state.storage.read().await;
        let traces = map_err_str!(storage.search_traces(&query, limit).await)?;
        Ok(batch_convert!(traces, convert_trace_info))
    })
}

#[tauri::command]
#[inline]
pub async fn get_storage_info(state: State<'_, AppState>) -> Result<StorageInfo, String> {
    timed_command!("get_storage_info", {
        let storage = state.storage.read().await;
        let stats = map_err_str!(storage.get_storage_stats().await)?;

        Ok(StorageInfo {
            trace_count: stats.trace_count,
            span_count: stats.span_count,
            memory_mb: stats.memory_mb,
            storage_health: match stats.health_status {
                urpo_lib::storage::StorageHealth::Healthy => "healthy".to_string(),
                urpo_lib::storage::StorageHealth::Degraded => "degraded".to_string(),
                urpo_lib::storage::StorageHealth::Critical => "critical".to_string(),
                urpo_lib::storage::StorageHealth::Offline => "offline".to_string(),
            },
            memory_pressure: stats.memory_pressure,
            oldest_span: stats.oldest_span.map(to_unix_secs),
        })
    })
}

#[tauri::command]
#[inline]
pub async fn start_receiver(state: State<'_, AppState>) -> Result<bool, String> {
    timed_command!("start_receiver", {
        let mut receiver_guard = state.receiver.write().await;

        if receiver_guard.is_none() {
            let receiver = urpo_lib::receiver::OtelReceiver::new(
                4327, // gRPC port (temporary change to avoid conflicts)
                4328, // HTTP port (temporary change to avoid conflicts)
                Arc::clone(&state.storage),
                Arc::clone(&state.monitor),
            );

            // Start receiver in background - BLAZING FAST
            let receiver_arc = Arc::new(receiver.clone()); // Note: This clone is necessary as receiver is OtelReceiver
            tokio::spawn(async move {
                tracing::info!("Starting OTLP receiver on ports 4317/4318");
                if let Err(e) = receiver_arc.run().await {
                    tracing::error!("OTLP receiver error: {}", e);
                }
            });

            *receiver_guard = Some(receiver);
            Ok(true) // Started
        } else {
            Ok(false) // Already running
        }
    })
}

/// Check if receiver is running - ZERO ALLOCATION
#[tauri::command]
#[inline]
pub async fn is_receiver_running(state: State<'_, AppState>) -> Result<bool, String> {
    let receiver_guard = state.receiver.read().await;
    Ok(receiver_guard.is_some())
}

#[tauri::command]
#[inline]
pub async fn stop_receiver(state: State<'_, AppState>) -> Result<(), String> {
    timed_command!("stop_receiver", {
        let mut receiver_guard = state.receiver.write().await;

        // Simply drop the receiver to stop it
        *receiver_guard = None;

        Ok(())
    })
}

#[tauri::command]
#[inline]
pub async fn trigger_tier_migration(state: State<'_, AppState>) -> Result<String, String> {
    timed_command!("trigger_tier_migration", {
        let storage = state.storage.write().await;
        let removed = map_err_str!(storage.emergency_cleanup().await)?;
        Ok(format!("Migrated {} spans to cold storage", removed))
    })
}

/// Stream trace data with chunking for large traces
#[tauri::command]
pub async fn stream_trace_data(
    window: Window,
    state: State<'_, AppState>,
    trace_id: String,
    chunk_size: usize,
) -> Result<(), String> {
    timed_command!("stream_trace_data", {
        let trace_id = map_err_str!(TraceId::new(trace_id))?;
        let storage = state.storage.read().await;
        let spans = map_err_str!(storage.get_trace_spans(&trace_id).await)?;

        // Stream in chunks for better performance
        for chunk in spans.chunks(chunk_size) {
            let chunk_data: Vec<Value> = chunk
                .iter()
                .filter_map(|span| serde_json::to_value(span).ok())
                .collect();

            map_err_str!(window.emit("trace-chunk", &chunk_data))?;

            // Yield to prevent blocking
            tokio::task::yield_now().await;
        }

        map_err_str!(window.emit("trace-complete", ()))?;
        Ok(())
    })
}

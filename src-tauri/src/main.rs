#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// EXTREME PERFORMANCE: Use mimalloc for blazing fast memory allocation
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod commands;
mod telemetry;
mod types;
mod device_auth;

use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Manager;
use tokio::sync::RwLock;

use types::{AppState, SystemMetrics};
use urpo_lib::{
    monitoring::Monitor,
    storage::{InMemoryStorage, StorageBackend},
};

// Re-export for commands
pub use telemetry::TELEMETRY;
pub use types::*;

// Configuration constants (matching Config defaults in urpo_lib)
const MAX_METRICS: usize = 1_048_576; // 1M metrics
const MAX_SERVICES: usize = 1000;      // 1000 services
const MAX_LOGS: usize = 100_000;       // 100K logs

/// Initialize application state
async fn init_app_state() -> (AppState, tokio::sync::broadcast::Receiver<urpo_lib::receiver::TraceEvent>) {
    // Create optimized storage with aggressive limits
    let storage: Arc<RwLock<dyn StorageBackend>> = Arc::new(RwLock::new(InMemoryStorage::new(100_000)));

    // Create monitor
    let monitor = Arc::new(Monitor::new());

    // Spawn background monitoring task
    let monitor_clone = Arc::clone(&monitor);
    let _storage_clone = Arc::clone(&storage);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            // Update telemetry
            TELEMETRY.update_system_metrics().await;

            // Get monitor health status
            let health = monitor_clone.get_health().await;
            if !matches!(health, urpo_lib::monitoring::SystemHealth::Healthy) {
                tracing::warn!("System health: {:?}", health);
            }
        }
    });

    // Initialize metrics storage
    let metrics_storage = Some(Arc::new(tokio::sync::Mutex::new(
        urpo_lib::metrics::MetricStorage::new(MAX_METRICS, MAX_SERVICES),
    )));

    // Auto-start OTLP receiver for BLAZING FAST trace ingestion
    let mut otel_receiver = urpo_lib::receiver::OtelReceiver::new(
        4327, // gRPC port (temporary change to avoid conflicts)
        4328, // HTTP port (temporary change to avoid conflicts)
        Arc::clone(&storage),
        Arc::clone(&monitor),
    );

    // Wire up metrics storage to receiver
    if let Some(ref metrics_storage) = metrics_storage {
        otel_receiver = otel_receiver.with_metrics(MAX_METRICS, MAX_SERVICES);
    }

    // Enable logs collection
    otel_receiver = otel_receiver.with_logs(MAX_LOGS);

    // Enable real-time event broadcasting
    let (otel_receiver, mut event_rx) = otel_receiver.with_events();

    let receiver = Arc::new(RwLock::new(Some(otel_receiver.clone())));

    // Start receiver in background - ZERO BLOCKING
    let receiver_arc = Arc::new(otel_receiver);
    tokio::spawn(async move {
        tracing::info!("🚀 Auto-starting OTLP receiver on ports 4327 (gRPC) and 4328 (HTTP)");
        if let Err(e) = receiver_arc.run().await {
            tracing::error!("OTLP receiver error: {}", e);
        }
    });

    (
        AppState {
            storage,
            receiver,
            monitor,
            metrics_storage,
        },
        event_rx,
    )
}

/// Get system metrics command with caching
#[tauri::command]
async fn get_system_metrics(state: tauri::State<'_, AppState>) -> Result<SystemMetrics, String> {
    // Update telemetry
    TELEMETRY.update_system_metrics().await;

    let storage = state.storage.read().await;
    let storage_stats = storage
        .get_storage_stats()
        .await
        .map_err(|e| e.to_string())?;

    let receiver_guard = state.receiver.read().await;
    let receiver_active = receiver_guard.is_some();

    Ok(SystemMetrics {
        cpu_usage: TELEMETRY.get_cpu_usage(),
        memory_usage_mb: TELEMETRY.get_heap_usage_mb(),
        memory_pressure: TELEMETRY.get_memory_pressure(),
        storage_health: format!("{:?}", storage_stats.health_status),
        receiver_active,
        spans_per_second: storage_stats.processing_rate,
        active_services: storage_stats.service_count,
        uptime_seconds: storage_stats.uptime_seconds,
        command_latencies: TELEMETRY.get_command_latencies(),
    })
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("urpo=debug,tauri=info")
        .init();

    // Initialize application state
    let (app_state, mut event_rx) = init_app_state().await;

    // Initialize device auth state
    let device_auth_state = device_auth::DeviceAuthState::new();

    // Build and run Tauri application
    tauri::Builder::default()
        .manage(app_state)
        .manage(device_auth_state)
        .invoke_handler(tauri::generate_handler![
            // System
            get_system_metrics,
            // Device Flow Authentication
            device_auth::start_device_login,
            device_auth::poll_device_login,
            device_auth::open_device_login_page,
            device_auth::get_device_user,
            device_auth::device_logout,
            // Commands from module
            commands::get_service_metrics,
            commands::get_service_metrics_batch,
            commands::list_recent_traces,
            commands::get_error_traces,
            commands::get_trace_spans,
            commands::search_traces,
            commands::get_storage_info,
            commands::start_receiver,
            commands::stop_receiver,
            commands::is_receiver_running,
            commands::trigger_tier_migration,
            commands::stream_trace_data,
            commands::get_service_health_metrics,
        ])
        .setup(move |app| {
            // Log startup time for performance tracking
            let start = Instant::now();

            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_window("main") {
                    window.open_devtools();
                }
            }

            let startup_ms = start.elapsed().as_millis();
            tracing::info!("🚀 Urpo started in {}ms", startup_ms);

            if startup_ms > 200 {
                tracing::warn!("⚠️ Startup time {}ms exceeds 200ms target!", startup_ms);
            }

            // Spawn task to broadcast trace events to frontend
            let app_handle = app.handle();
            tokio::spawn(async move {
                while let Ok(event) = event_rx.recv().await {
                    tracing::debug!("Broadcasting trace event: {:?}", event);
                    // Emit to all windows
                    if let Err(e) = app_handle.emit_all("trace_received", &event) {
                        tracing::warn!("Failed to emit trace event: {}", e);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

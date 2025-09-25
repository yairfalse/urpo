//! Main application entry point for Urpo.

use crate::core::{Config, Result, UrpoError};
use crate::monitoring::Monitor;
use crate::receiver::OtelReceiver;
use crate::storage::UnifiedStorage;
use crate::tui;
use std::sync::Arc;

/// Main application struct that coordinates all components of Urpo.
pub struct Application {
    /// The OpenTelemetry receiver for trace ingestion
    receiver: Arc<OtelReceiver>,
    /// Unified storage backend
    storage: Arc<UnifiedStorage>,
    /// Health monitor
    monitor: Arc<Monitor>,
    /// Application configuration
    config: Config,
}

impl Application {
    /// Create a new Application with the given configuration.
    pub fn new(config: Config) -> Result<Self> {
        // Initialize health monitor
        let monitor = Arc::new(Monitor::new());

        // Initialize storage with configuration
        let storage =
            Arc::new(UnifiedStorage::new(config.storage.max_spans, config.storage.max_memory_mb));

        // Initialize receiver with storage and monitor
        let receiver = Arc::new(
            OtelReceiver::with_storage(
                config.receiver.grpc_port,
                config.receiver.http_port,
                &storage,
                monitor.clone(),
            )
            .with_sampling_rate(config.sampling.rate),
        );

        Ok(Self {
            receiver,
            storage,
            monitor,
            config,
        })
    }

    /// Run the application (starts receivers and TUI if enabled).
    pub async fn run(self) -> Result<()> {
        tracing::info!("Starting Urpo application");

        // Start the receiver in the background
        let receiver = self.receiver.clone();
        let receiver_handle = tokio::spawn(async move {
            if let Err(e) = receiver.run().await {
                tracing::error!("Receiver error: {}", e);
            }
        });

        // If TUI is enabled, run it; otherwise just wait for shutdown
        if self.config.ui.enabled {
            // Run TUI in foreground
            let result = tui::run_tui(self.storage.as_backend(), self.monitor.clone()).await;

            // Shutdown receiver when TUI exits
            receiver_handle.abort();
            result
        } else {
            // Headless mode - just run until shutdown signal
            tracing::info!("Running in headless mode (TUI disabled)");
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Received shutdown signal");
                    receiver_handle.abort();
                    Ok(())
                }
                _ = receiver_handle => {
                    Err(UrpoError::internal("Receiver terminated unexpectedly"))
                }
            }
        }
    }

    /// Get a reference to the storage backend.
    pub fn storage(&self) -> &Arc<UnifiedStorage> {
        &self.storage
    }

    /// Get a reference to the health monitor.
    pub fn monitor(&self) -> &Arc<Monitor> {
        &self.monitor
    }

    /// Get a reference to the receiver.
    pub fn receiver(&self) -> &Arc<OtelReceiver> {
        &self.receiver
    }
}

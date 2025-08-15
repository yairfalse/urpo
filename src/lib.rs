//! Urpo - Terminal-native OTEL trace explorer.
//!
//! Urpo is a high-performance OpenTelemetry trace explorer designed for
//! terminal environments. It provides real-time service health monitoring
//! and individual trace debugging capabilities.
//!
//! # Features
//!
//! - **OTEL Protocol Support**: GRPC (port 4317) and HTTP (port 4318) receivers
//! - **Real-time Aggregation**: Service metrics with sub-second latency
//! - **Terminal UI**: Interactive dashboard with vim-like navigation
//! - **Memory Efficient**: Bounded memory usage with automatic eviction
//! - **Zero Configuration**: Works out of the box with sensible defaults
//!
//! # Architecture
//!
//! Urpo is built with a modular architecture:
//! - `receiver`: OTEL protocol implementation
//! - `storage`: Pluggable storage backends
//! - `core`: Domain models and business logic
//! - `ui`: Terminal user interface
//! - `cli`: Command-line interface
//!
//! # Example
//!
//! ```no_run
//! use urpo::core::Config;
//! use urpo::Application;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::default();
//!     let app = Application::new(config)?;
//!     app.run().await?;
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

pub mod cli;
pub mod core;
pub mod receiver;
pub mod storage;
pub mod ui;

use crate::core::{Config, Result, Span, UrpoError};
use crate::receiver::ReceiverManager;
use crate::storage::StorageManager;
use crate::ui::{App as UIApp, TerminalUI};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

/// Main application coordinator.
pub struct Application {
    config: Config,
    storage_manager: Arc<StorageManager>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Application {
    /// Create a new application instance.
    pub fn new(config: Config) -> Result<Self> {
        config.validate()?;
        
        let storage_manager = Arc::new(StorageManager::new_in_memory(
            config.max_traces,
        ));

        Ok(Self {
            config,
            storage_manager,
            shutdown_tx: None,
        })
    }

    /// Run the application with UI.
    pub async fn run(mut self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        // Create span processing channel
        let (span_tx, mut span_rx) = mpsc::channel::<Span>(1000);

        // Start receivers
        let receiver_manager = ReceiverManager::new(
            span_tx,
            self.config.grpc_port,
            self.config.http_port,
            self.config.sampling_rate,
        );

        let receiver_handle = tokio::spawn(async move {
            receiver_manager.start().await
        });

        // Start span processor
        let storage = self.storage_manager.backend();
        let processor_handle = tokio::spawn(async move {
            while let Some(span) = span_rx.recv().await {
                if let Err(e) = storage.store_span(span).await {
                    tracing::error!("Failed to store span: {}", e);
                }
            }
        });

        // Start cleanup task
        let cleanup_storage = self.storage_manager.clone();
        let cleanup_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Err(e) = cleanup_storage.run_cleanup().await {
                    tracing::error!("Cleanup failed: {}", e);
                }
            }
        });

        // Start UI
        let ui_storage = self.storage_manager.backend();
        let ui_handle = tokio::spawn(async move {
            let mut terminal_ui = TerminalUI::new()?;
            let mut app = UIApp::new();

            // Update app with data periodically
            let mut update_interval = tokio::time::interval(std::time::Duration::from_secs(1));
            
            loop {
                tokio::select! {
                    _ = update_interval.tick() => {
                        // Update service metrics
                        if let Ok(metrics) = ui_storage.get_service_metrics().await {
                            app.update_services(metrics);
                        }
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                        // Run UI update
                        terminal_ui.run(app).await?;
                        break;
                    }
                }
            }
            
            Ok::<(), UrpoError>(())
        });

        // Wait for shutdown signal
        tokio::select! {
            _ = shutdown_rx => {
                tracing::info!("Shutdown signal received");
            }
            result = receiver_handle => {
                if let Err(e) = result {
                    tracing::error!("Receiver task failed: {}", e);
                }
            }
            result = processor_handle => {
                if let Err(e) = result {
                    tracing::error!("Processor task failed: {}", e);
                }
            }
            result = cleanup_handle => {
                if let Err(e) = result {
                    tracing::error!("Cleanup task failed: {}", e);
                }
            }
            result = ui_handle => {
                match result {
                    Ok(Ok(())) => tracing::info!("UI exited normally"),
                    Ok(Err(e)) => tracing::error!("UI error: {}", e),
                    Err(e) => tracing::error!("UI task failed: {}", e),
                }
            }
        }

        Ok(())
    }

    /// Run the application in headless mode (no UI).
    pub async fn run_headless(mut self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        // Create span processing channel
        let (span_tx, mut span_rx) = mpsc::channel::<Span>(1000);

        // Start receivers
        let receiver_manager = ReceiverManager::new(
            span_tx,
            self.config.grpc_port,
            self.config.http_port,
            self.config.sampling_rate,
        );

        let receiver_handle = tokio::spawn(async move {
            receiver_manager.start().await
        });

        // Start span processor
        let storage = self.storage_manager.backend();
        let processor_handle = tokio::spawn(async move {
            while let Some(span) = span_rx.recv().await {
                if let Err(e) = storage.store_span(span).await {
                    tracing::error!("Failed to store span: {}", e);
                }
            }
        });

        // Start metrics reporter
        let metrics_storage = self.storage_manager.backend();
        let metrics_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                match metrics_storage.get_service_metrics().await {
                    Ok(metrics) => {
                        for metric in metrics {
                            tracing::info!(
                                "Service: {} | RPS: {:.1} | Error Rate: {:.2}% | P95: {}ms",
                                metric.name.as_str(),
                                metric.request_rate,
                                metric.error_rate * 100.0,
                                metric.latency_p95.as_millis()
                            );
                        }
                    }
                    Err(e) => tracing::error!("Failed to get metrics: {}", e),
                }
            }
        });

        // Wait for shutdown
        tokio::select! {
            _ = shutdown_rx => {
                tracing::info!("Shutdown signal received");
            }
            result = receiver_handle => {
                if let Err(e) = result {
                    tracing::error!("Receiver task failed: {}", e);
                }
            }
            result = processor_handle => {
                if let Err(e) = result {
                    tracing::error!("Processor task failed: {}", e);
                }
            }
            result = metrics_handle => {
                if let Err(e) = result {
                    tracing::error!("Metrics task failed: {}", e);
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Ctrl-C received, shutting down");
            }
        }

        Ok(())
    }

    /// Shutdown the application gracefully.
    pub fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_creation() {
        let config = Config::default();
        let app = Application::new(config);
        assert!(app.is_ok());
    }

    #[test]
    fn test_invalid_config() {
        let mut config = Config::default();
        config.sampling_rate = 2.0; // Invalid
        let app = Application::new(config);
        assert!(app.is_err());
    }
}
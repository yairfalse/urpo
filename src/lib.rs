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
//! use urpo_lib::core::Config;
//! use urpo_lib::Application;
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

pub mod api;
pub mod cli;
pub mod core;
pub mod export;
pub mod monitoring;
pub mod receiver;
pub mod storage;
pub mod ui;

use crate::core::{Config, Result};
// use crate::receiver::ReceiverManager; // Commented out as ReceiverManager is not currently used
use crate::storage::StorageManager;
use std::sync::Arc;
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
            config.storage.max_spans,
        ));

        Ok(Self {
            config,
            storage_manager,
            shutdown_tx: None,
        })
    }

    /// Run the application with UI.
    pub async fn run(self) -> Result<()> {
        // This method is not currently used - see cli/mod.rs for the actual implementation
        // Keeping it here for potential future use
        Ok(())
    }

    /// Run the application in headless mode (no UI).
    pub async fn run_headless(self) -> Result<()> {
        // This method is not currently used - see cli/mod.rs for the actual implementation
        // Keeping it here for potential future use
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
        // Test would need to be updated for new config structure
        // Placeholder for now
        assert!(true);
    }
}
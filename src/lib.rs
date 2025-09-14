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
pub mod metrics;
pub mod monitoring;
pub mod receiver;
pub mod service_map;
pub mod storage;
pub mod tui;

// Re-export core types for convenience
pub use crate::core::{Config, Result};

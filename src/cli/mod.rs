//! Command-line interface for Urpo.
//!
//! This module provides the CLI argument parsing and command handling
//! for the Urpo application.

use crate::core::{Config, Result, UrpoError};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Terminal-native OTEL trace explorer with real-time service health monitoring.
#[derive(Parser, Debug)]
#[command(name = "urpo")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Configuration file path.
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Enable debug logging.
    #[arg(short, long)]
    pub debug: bool,

    /// Log level (trace, debug, info, warn, error).
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands for Urpo.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the OTEL receiver and UI.
    Start {
        /// GRPC port for OTEL receiver.
        #[arg(long, default_value = "4317")]
        grpc_port: u16,

        /// HTTP port for OTEL receiver.
        #[arg(long, default_value = "4318")]
        http_port: u16,

        /// Maximum memory usage in MB.
        #[arg(long, default_value = "512")]
        max_memory_mb: usize,

        /// Maximum number of traces to store.
        #[arg(long, default_value = "10000")]
        max_traces: usize,

        /// Sampling rate (0.0 to 1.0).
        #[arg(long, default_value = "1.0")]
        sampling_rate: f64,

        /// Trace retention period in seconds.
        #[arg(long, default_value = "3600")]
        retention_seconds: u64,

        /// Run in headless mode (no UI).
        #[arg(long)]
        headless: bool,
    },

    /// Export traces to a file.
    Export {
        /// Output file path.
        #[arg(short, long)]
        output: PathBuf,

        /// Export format (json, yaml).
        #[arg(short, long, default_value = "json")]
        format: ExportFormat,

        /// Service name filter.
        #[arg(long)]
        service: Option<String>,

        /// Time range start (RFC3339 format).
        #[arg(long)]
        from: Option<String>,

        /// Time range end (RFC3339 format).
        #[arg(long)]
        to: Option<String>,
    },

    /// Import traces from a file.
    Import {
        /// Input file path.
        #[arg(short, long)]
        input: PathBuf,

        /// Import format (json, yaml).
        #[arg(short, long, default_value = "json")]
        format: ExportFormat,
    },

    /// Show service health status.
    Health {
        /// Service name filter.
        #[arg(long)]
        service: Option<String>,

        /// Output format (table, json).
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
    },

    /// Validate configuration file.
    Validate {
        /// Configuration file path.
        #[arg(short, long)]
        config: PathBuf,
    },
}

/// Export format options.
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    /// JSON format.
    Json,
    /// YAML format.
    Yaml,
}

impl std::str::FromStr for ExportFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(ExportFormat::Json),
            "yaml" | "yml" => Ok(ExportFormat::Yaml),
            _ => Err(format!("Unknown export format: {}", s)),
        }
    }
}

/// Output format options.
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    /// Table format for terminal display.
    Table,
    /// JSON format for programmatic consumption.
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!("Unknown output format: {}", s)),
        }
    }
}

impl Cli {
    /// Parse command-line arguments.
    pub fn parse_args() -> Self {
        Cli::parse()
    }

    /// Load configuration from file or command-line arguments.
    pub async fn load_config(&self) -> Result<Config> {
        // If a config file is specified, load it
        if let Some(config_path) = &self.config {
            let config_str = tokio::fs::read_to_string(config_path)
                .await
                .map_err(|e| UrpoError::config(format!("Failed to read config file: {}", e)))?;
            
            let mut config: Config = serde_yaml::from_str(&config_str)
                .map_err(|e| UrpoError::config(format!("Failed to parse config file: {}", e)))?;
            
            // Override with command-line debug flag if set
            if self.debug {
                config.debug = true;
            }
            
            config.validate()?;
            return Ok(config);
        }

        // Otherwise, build config from command-line arguments
        match &self.command {
            Some(Commands::Start {
                grpc_port,
                http_port,
                max_memory_mb,
                max_traces,
                sampling_rate,
                retention_seconds,
                ..
            }) => {
                let config = Config {
                    grpc_port: *grpc_port,
                    http_port: *http_port,
                    max_memory_mb: *max_memory_mb,
                    max_traces: *max_traces,
                    sampling_rate: *sampling_rate,
                    debug: self.debug,
                    retention_seconds: *retention_seconds,
                };
                config.validate()?;
                Ok(config)
            }
            _ => Config::new(),
        }
    }

    /// Initialize logging based on configuration.
    pub fn init_logging(&self) -> Result<()> {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

        let filter = if self.debug {
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("debug"))
        } else {
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&self.log_level))
        };

        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .try_init()
            .map_err(|e| UrpoError::config(format!("Failed to initialize logging: {}", e)))?;

        Ok(())
    }
}

/// Execute a CLI command.
pub async fn execute(cli: Cli) -> Result<()> {
    cli.init_logging()?;

    match cli.command {
        Some(Commands::Start { headless, .. }) => {
            let config = cli.load_config().await?;
            if headless {
                tracing::info!("Starting Urpo in headless mode...");
                start_headless(config).await
            } else {
                tracing::info!("Starting Urpo with terminal UI...");
                start_with_ui(config).await
            }
        }
        Some(Commands::Export { .. }) => {
            tracing::info!("Exporting traces...");
            export_traces().await
        }
        Some(Commands::Import { .. }) => {
            tracing::info!("Importing traces...");
            import_traces().await
        }
        Some(Commands::Health { .. }) => {
            tracing::info!("Checking service health...");
            show_health().await
        }
        Some(Commands::Validate { config }) => {
            tracing::info!("Validating configuration file: {:?}", config);
            validate_config(config).await
        }
        None => {
            // Default action: start with UI
            let config = cli.load_config().await?;
            tracing::info!("Starting Urpo with terminal UI (default)...");
            start_with_ui(config).await
        }
    }
}

async fn start_with_ui(config: Config) -> Result<()> {
    use crate::storage::{StorageManager, fake_spans::FakeSpanGenerator};
    use crate::ui::{App, TerminalUI};
    use crate::receiver::ReceiverManager;
    use std::sync::Arc;
    use tokio::sync::mpsc;
    
    // Create storage manager with configured limits
    let storage_manager = StorageManager::new_in_memory(config.max_traces);
    let storage = storage_manager.backend();
    
    // Create channel for spans from GRPC receiver
    let (span_tx, mut span_rx) = mpsc::channel(1000);
    
    // Start GRPC and HTTP receivers
    let receiver_manager = ReceiverManager::new(
        span_tx,
        config.grpc_port,
        config.http_port,
        config.sampling_rate,
    );
    
    let receiver_handle = tokio::spawn(async move {
        tracing::info!("Starting OTEL receivers...");
        if let Err(e) = receiver_manager.start().await {
            tracing::error!("OTEL receiver failed: {}", e);
        }
    });
    
    // Spawn task to process spans from receiver and store them
    let storage_clone = storage.clone();
    let span_processor_handle = tokio::spawn(async move {
        while let Some(span) = span_rx.recv().await {
            if let Err(e) = storage_clone.store_span(span).await {
                tracing::warn!("Failed to store span from receiver: {}", e);
            }
        }
        tracing::info!("Span processor stopped");
    });
    
    // Start fake span generator in background (for demonstration)
    let generator = Arc::new(FakeSpanGenerator::new());
    let storage_clone = storage.clone();
    let generator_clone = generator.clone();
    
    let generator_handle = tokio::spawn(async move {
        let storage = storage_clone;
        loop {
            // Generate spans continuously but less frequently than before
            match generator_clone.generate_batch(10).await {
                Ok(spans) => {
                    for span in spans {
                        if let Err(e) = storage.store_span(span).await {
                            tracing::warn!("Failed to store fake span: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to generate fake spans: {}", e);
                }
            }
            // Wait longer between fake span generation
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });
    
    // Create UI app with storage backend
    let app = App::with_storage(storage);
    
    // Create and run the terminal UI
    let mut terminal = TerminalUI::new()?;
    let result = terminal.run(app).await;
    
    // Stop all background tasks
    receiver_handle.abort();
    span_processor_handle.abort();
    generator_handle.abort();
    
    // Restore terminal on exit
    terminal.restore()?;
    
    result
}

async fn start_headless(config: Config) -> Result<()> {
    use crate::storage::StorageManager;
    use crate::receiver::ReceiverManager;
    use tokio::sync::mpsc;
    use tokio::signal;
    
    tracing::info!("Starting Urpo in headless mode...");
    
    // Create storage manager with configured limits
    let storage_manager = StorageManager::new_in_memory(config.max_traces);
    let storage = storage_manager.backend();
    
    // Create channel for spans from GRPC receiver
    let (span_tx, mut span_rx) = mpsc::channel(1000);
    
    // Start GRPC and HTTP receivers
    let receiver_manager = ReceiverManager::new(
        span_tx,
        config.grpc_port,
        config.http_port,
        config.sampling_rate,
    );
    
    let receiver_handle = tokio::spawn(async move {
        tracing::info!("Starting OTEL receivers in headless mode...");
        if let Err(e) = receiver_manager.start().await {
            tracing::error!("OTEL receiver failed: {}", e);
        }
    });
    
    // Spawn task to process spans from receiver and store them
    let span_processor_handle = tokio::spawn(async move {
        let storage = storage;
        while let Some(span) = span_rx.recv().await {
            if let Err(e) = storage.store_span(span).await {
                tracing::warn!("Failed to store span from receiver: {}", e);
            }
        }
        tracing::info!("Span processor stopped");
    });
    
    tracing::info!("Urpo headless mode ready. Press Ctrl+C to stop.");
    
    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            tracing::info!("Shutdown signal received, stopping...");
        }
        Err(err) => {
            tracing::error!("Unable to listen for shutdown signal: {}", err);
        }
    }
    
    // Stop all background tasks
    receiver_handle.abort();
    span_processor_handle.abort();
    
    tracing::info!("Urpo stopped");
    Ok(())
}

async fn export_traces() -> Result<()> {
    // Placeholder for trace export
    tracing::info!("Trace export would happen here");
    Ok(())
}

async fn import_traces() -> Result<()> {
    // Placeholder for trace import
    tracing::info!("Trace import would happen here");
    Ok(())
}

async fn show_health() -> Result<()> {
    // Placeholder for health check
    tracing::info!("Health check would happen here");
    Ok(())
}

async fn validate_config(path: PathBuf) -> Result<()> {
    let config_str = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| UrpoError::config(format!("Failed to read config file: {}", e)))?;
    
    let config: Config = serde_yaml::from_str(&config_str)
        .map_err(|e| UrpoError::config(format!("Failed to parse config file: {}", e)))?;
    
    config.validate()?;
    
    tracing::info!("Configuration file is valid: {:?}", path);
    println!("✓ Configuration file is valid");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_parsing() {
        assert!(matches!("json".parse::<ExportFormat>(), Ok(ExportFormat::Json)));
        assert!(matches!("yaml".parse::<ExportFormat>(), Ok(ExportFormat::Yaml)));
        assert!(matches!("yml".parse::<ExportFormat>(), Ok(ExportFormat::Yaml)));
        assert!("unknown".parse::<ExportFormat>().is_err());
    }

    #[test]
    fn test_output_format_parsing() {
        assert!(matches!("table".parse::<OutputFormat>(), Ok(OutputFormat::Table)));
        assert!(matches!("json".parse::<OutputFormat>(), Ok(OutputFormat::Json)));
        assert!("unknown".parse::<OutputFormat>().is_err());
    }
}
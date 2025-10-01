//! Command-line interface for Urpo.
//!
//! This module provides a simple, htop-like CLI for Urpo.
//! Just run `urpo` to start with sensible defaults!

use crate::core::{Config, Result, UrpoError};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Terminal-native OTEL trace explorer - simple as htop!
#[derive(Parser, Debug)]
#[command(name = "urpo")]
#[command(version, about, long_about = None)]
#[command(disable_version_flag = true)]
pub struct Cli {
    /// Optional subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// GRPC port for OTEL receiver
    #[arg(long, env = "URPO_GRPC_PORT", default_value = "4317")]
    pub grpc_port: Option<u16>,

    /// HTTP port for OTEL receiver
    #[arg(long, env = "URPO_HTTP_PORT", default_value = "4318")]
    pub http_port: Option<u16>,

    /// Maximum memory usage in MB
    #[arg(long, env = "URPO_MEMORY_LIMIT")]
    pub memory_limit: Option<usize>,

    /// Configuration file path (default: ~/.config/urpo/config.yaml)
    #[arg(short, long, env = "URPO_CONFIG")]
    pub config: Option<PathBuf>,

    /// Disable fake span generation for demo
    #[arg(long, env = "URPO_NO_FAKE")]
    pub no_fake: bool,

    /// Enable debug logging
    #[arg(short, long, env = "URPO_DEBUG")]
    pub debug: bool,

    /// Run in headless mode (no UI, just receivers)
    #[arg(long, env = "URPO_HEADLESS")]
    pub headless: bool,

    /// Use terminal UI instead of GUI (default: GUI if available)
    #[arg(long, env = "URPO_TERMINAL")]
    pub terminal: bool,

    /// Validate configuration and exit
    #[arg(long)]
    pub check_config: bool,

    /// Show version information
    #[arg(short = 'V', long = "show-version")]
    pub version: bool,

    /// Enable HTTP API server (port 8080)
    #[arg(long, env = "URPO_API")]
    pub api: bool,

    /// HTTP API server port (default: 8080)
    #[arg(long, env = "URPO_API_PORT", default_value = "8080")]
    pub api_port: u16,
}

/// Available subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Export traces to various formats
    Export {
        /// Trace ID to export (if not specified, exports based on filters)
        trace_id: Option<String>,

        /// Export format (json, jaeger, otel, csv)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Filter by service name
        #[arg(short, long)]
        service: Option<String>,

        /// Export traces from the last duration (e.g., "1h", "30m", "24h")
        #[arg(short, long)]
        last: Option<String>,

        /// Start time for export (ISO 8601 or Unix timestamp)
        #[arg(long)]
        start: Option<String>,

        /// End time for export (ISO 8601 or Unix timestamp)
        #[arg(long)]
        end: Option<String>,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Only export traces with errors
        #[arg(long)]
        errors_only: bool,

        /// Maximum number of traces to export
        #[arg(long, default_value = "1000")]
        limit: usize,
    },
}

impl Cli {
    /// Parse command-line arguments.
    pub fn parse_args() -> Self {
        Cli::parse()
    }

    /// Load configuration with proper precedence:
    /// 1. CLI arguments (highest priority)
    /// 2. Environment variables
    /// 3. Config file
    /// 4. Defaults (lowest priority)
    pub async fn load_config(&self) -> Result<Config> {
        use crate::core::config::ConfigBuilder;

        let mut builder = ConfigBuilder::new();

        // 1. Load from config file if specified or default location
        let config_path = if let Some(path) = &self.config {
            path.clone()
        } else {
            // Check default config location
            let default_path = dirs::config_dir()
                .map(|d| d.join("urpo").join("config.yaml"))
                .unwrap_or_else(|| PathBuf::from("~/.config/urpo/config.yaml"));

            if default_path.exists() {
                default_path
            } else {
                // No config file, use defaults
                return self.build_config_from_args(builder);
            }
        };

        // Try to load config file
        match tokio::fs::read_to_string(&config_path).await {
            Ok(content) => {
                builder = builder.from_yaml(&content)?;
                tracing::info!("Loaded configuration from: {:?}", config_path);
            },
            Err(e) if self.config.is_some() => {
                // User explicitly specified a config file that doesn't exist
                return Err(UrpoError::config(format!(
                    "Failed to read config file {:?}: {}",
                    config_path, e
                )));
            },
            Err(_) => {
                // Default config file doesn't exist, that's OK
                tracing::debug!("No config file found at {:?}, using defaults", config_path);
            },
        }

        // 2. Apply CLI overrides
        self.build_config_from_args(builder)
    }

    fn build_config_from_args(
        &self,
        mut builder: crate::core::config::ConfigBuilder,
    ) -> Result<Config> {
        // Apply CLI arguments (these override everything)
        if let Some(port) = self.grpc_port {
            builder = builder.grpc_port(port);
        }
        if let Some(port) = self.http_port {
            builder = builder.http_port(port);
        }
        if let Some(limit) = self.memory_limit {
            builder = builder.max_memory_mb(limit);
        }

        builder = builder.debug(self.debug);

        builder.build()
    }

    /// Initialize logging based on configuration.
    pub fn init_logging(&self) -> Result<()> {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

        // Determine log level
        let env_log_level = std::env::var("URPO_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        let log_level = if self.debug {
            "debug"
        } else {
            env_log_level.as_str()
        };

        let filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

        // Configure logging format
        let fmt_layer = if self.headless {
            // Structured logging for headless mode
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .compact()
        } else {
            // Simpler format for interactive mode
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .compact()
        };

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .try_init()
            .map_err(|e| UrpoError::config(format!("Failed to initialize logging: {}", e)))?;

        Ok(())
    }
}

/// Execute the Urpo application.
pub async fn execute(cli: Cli) -> Result<()> {
    // Handle version flag first
    if cli.version {
        println!("urpo {}", env!("CARGO_PKG_VERSION"));
        println!("Terminal-native OTEL trace explorer");
        return Ok(());
    }

    // Handle subcommands
    if let Some(ref command) = cli.command {
        return execute_subcommand(command.clone(), &cli).await;
    }

    // Initialize logging
    cli.init_logging()?;

    // Load and validate configuration
    let config = cli.load_config().await?;

    // Handle config validation flag
    if cli.check_config {
        config.validate()?;
        println!("Configuration is valid!");
        println!("  GRPC port: {}", config.server.grpc_port);
        println!("  HTTP port: {}", config.server.http_port);
        println!("  Memory limit: {}MB", config.storage.max_memory_mb);
        println!("  Max spans: {}", config.storage.max_spans);
        return Ok(());
    }

    // Start the application
    if cli.headless {
        tracing::info!("Starting Urpo in headless mode...");
        start_headless(config, &cli).await
    } else {
        tracing::info!("Starting Urpo with terminal UI...");
        start_with_ui(config, &cli).await
    }
}

/// Execute a subcommand
async fn execute_subcommand(command: Commands, cli: &Cli) -> Result<()> {
    // Initialize logging for subcommands
    cli.init_logging()?;

    match command {
        Commands::Export {
            trace_id,
            format,
            service,
            last,
            start,
            end,
            output,
            errors_only,
            limit,
        } => {
            execute_export(
                trace_id,
                format,
                service,
                last,
                start,
                end,
                output,
                errors_only,
                limit,
                cli,
            )
            .await
        },
    }
}

/// Execute the export command
async fn execute_export(
    trace_id: Option<String>,
    format: String,
    service: Option<String>,
    last: Option<String>,
    start: Option<String>,
    end: Option<String>,
    output: Option<PathBuf>,
    errors_only: bool,
    limit: usize,
    cli: &Cli,
) -> Result<()> {
    use crate::{
        core::TraceId,
        export::{ExportFormat, ExportOptions, TraceExporter},
        storage::{InMemoryStorage, StorageBackend},
    };
    use std::sync::Arc;
    use std::time::SystemTime;
    use tokio::sync::RwLock;

    // Load configuration
    let config = cli.load_config().await?;

    // Initialize storage (read-only for export)
    let storage: Arc<RwLock<dyn StorageBackend>> =
        Arc::new(RwLock::new(InMemoryStorage::with_config(&config)));
    let storage_trait = Arc::clone(&storage);

    // Parse export format
    let export_format = format
        .parse::<ExportFormat>()
        .map_err(|_| UrpoError::config(format!("Invalid export format: {}", format)))?;

    // Handle time filtering
    let (start_time, end_time) = if let Some(last_str) = last {
        // Parse duration string (e.g., "1h", "30m", "24h")
        let duration = parse_duration(&last_str)
            .ok_or_else(|| UrpoError::config(format!("Invalid duration: {}", last_str)))?;

        let now = SystemTime::now();
        let start = now - duration;
        let start_nanos = start
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| UrpoError::config(format!("Invalid system time: {}", e)))?
            .as_nanos() as u64;
        (Some(start_nanos), None)
    } else {
        // Parse explicit start/end times
        let start_time = start
            .as_ref()
            .map(|s| parse_timestamp(s))
            .transpose()
            .map_err(|e| UrpoError::config(format!("Invalid start time: {}", e)))?;

        let end_time = end
            .as_ref()
            .map(|s| parse_timestamp(s))
            .transpose()
            .map_err(|e| UrpoError::config(format!("Invalid end time: {}", e)))?;

        (start_time, end_time)
    };

    // Create exporter
    let storage_guard = storage_trait.read().await;
    let trace_exporter = TraceExporter::new(&*storage_guard);

    if let Some(trace_id_str) = trace_id {
        // Export specific trace
        let trace_id = TraceId::new(trace_id_str)?;

        // Get trace spans
        let spans = storage_guard
            .get_trace_spans(&trace_id)
            .await
            .map_err(|e| UrpoError::config(format!("Failed to get trace: {}", e)))?;

        if spans.is_empty() {
            return Err(UrpoError::config(format!("Trace not found: {}", trace_id.as_str())));
        }

        // Export the trace
        let export_options = ExportOptions {
            format: export_format,
            output: output.clone(),
            service: None,
            start_time: None,
            end_time: None,
            limit: Some(1),
            errors_only: false,
        };

        let export_result = trace_exporter
            .export_single_trace(&trace_id, &spans, &export_options)
            .await?;

        // Write output
        if let Some(output_path) = output {
            tokio::fs::write(output_path, export_result)
                .await
                .map_err(|e| UrpoError::config(format!("Failed to write output: {}", e)))?;
        } else {
            print!("{}", export_result);
        }
    } else {
        // Export multiple traces based on filters
        let export_options = ExportOptions {
            format: export_format,
            output: output.clone(),
            service,
            start_time,
            end_time,
            limit: Some(limit),
            errors_only,
        };

        let export_result = trace_exporter.export_traces(&export_options).await?;

        // Write output
        if let Some(output_path) = output {
            tokio::fs::write(output_path, export_result)
                .await
                .map_err(|e| UrpoError::config(format!("Failed to write output: {}", e)))?;
        } else {
            print!("{}", export_result);
        }
    }

    Ok(())
}

/// Parse a duration string like "1h", "30m", "24h"
fn parse_duration(s: &str) -> Option<std::time::Duration> {
    use std::time::Duration;

    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: u64 = num_str.parse().ok()?;

    match unit {
        "s" => Some(Duration::from_secs(num)),
        "m" => Some(Duration::from_secs(num * 60)),
        "h" => Some(Duration::from_secs(num * 3600)),
        "d" => Some(Duration::from_secs(num * 86400)),
        _ => None,
    }
}

/// Parse a timestamp string (ISO 8601 or Unix timestamp)
fn parse_timestamp(s: &str) -> Result<u64> {
    // Try parsing as Unix timestamp
    if let Ok(ts) = s.parse::<u64>() {
        // Assume it's in seconds if it's a reasonable Unix timestamp
        if ts < 10_000_000_000 {
            return Ok(ts * 1_000_000_000); // Convert to nanoseconds
        }
        return Ok(ts); // Already in milliseconds or nanoseconds
    }

    // Try parsing as ISO 8601
    use chrono::DateTime;
    let dt = DateTime::parse_from_rfc3339(s)
        .map_err(|e| UrpoError::config(format!("Invalid timestamp: {}", e)))?;

    Ok(dt.timestamp_nanos_opt().unwrap_or(0) as u64)
}

async fn start_with_ui(config: Config, cli: &Cli) -> Result<()> {
    use crate::{
        api::{start_server as start_api_server, ApiConfig},
        monitoring::Monitor,
        receiver::OtelReceiver,
        storage::{InMemoryStorage, StorageBackend},
    };
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // Initialize storage
    let storage: Arc<RwLock<dyn StorageBackend>> =
        Arc::new(RwLock::new(InMemoryStorage::with_config(&config)));
    let storage_trait = Arc::clone(&storage);

    // Initialize health monitor
    let health_monitor = Arc::new(Monitor::new());

    // Fake span generator completely removed - using real OTEL data only

    // Start OTEL receivers
    let receiver = Arc::new(OtelReceiver::new(
        config.server.grpc_port,
        config.server.http_port,
        Arc::clone(&storage_trait),
        Arc::clone(&health_monitor),
    ));

    let receiver_clone = Arc::clone(&receiver);
    let receiver_handle = tokio::spawn(async move {
        if let Err(e) = receiver_clone.run().await {
            tracing::error!("OTEL receiver error: {}", e);
        }
    });

    // Start HTTP API server if enabled
    let api_handle = if cli.api {
        let api_storage = Arc::clone(&storage_trait);
        let api_config = ApiConfig {
            port: cli.api_port,
            enable_cors: true,
            max_results: 1000,
        };

        tracing::info!("Starting HTTP API server on port {}...", cli.api_port);

        Some(tokio::spawn(async move {
            if let Err(e) = start_api_server(api_storage, api_config).await {
                tracing::error!("API server error: {}", e);
            }
        }))
    } else {
        None
    };

    // Start the minimal terminal UI
    let ui_result = crate::tui::run_tui(storage_trait, health_monitor).await;

    // Cleanup
    receiver_handle.abort();
    if let Some(handle) = api_handle {
        handle.abort();
    }

    ui_result
}

async fn start_headless(config: Config, cli: &Cli) -> Result<()> {
    use crate::{
        api::{start_server as start_api_server, ApiConfig},
        monitoring::Monitor,
        receiver::OtelReceiver,
        storage::{InMemoryStorage, StorageBackend},
    };
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // Initialize storage
    let storage: Arc<RwLock<dyn StorageBackend>> =
        Arc::new(RwLock::new(InMemoryStorage::with_config(&config)));
    let storage_trait = Arc::clone(&storage);

    // Initialize health monitor
    let health_monitor = Arc::new(Monitor::new());

    // Fake span generator completely removed - using real OTEL data only

    // Start OTEL receivers
    let receiver = Arc::new(OtelReceiver::new(
        config.server.grpc_port,
        config.server.http_port,
        Arc::clone(&storage_trait),
        health_monitor,
    ));

    tracing::info!("Urpo running in headless mode");
    tracing::info!("  GRPC receiver on port {}", config.server.grpc_port);
    tracing::info!("  HTTP receiver on port {}", config.server.http_port);

    // Start API server if enabled
    if cli.api {
        tracing::info!("  HTTP API server on port {}", cli.api_port);
        let api_storage = Arc::clone(&storage_trait);
        let api_config = ApiConfig {
            port: cli.api_port,
            enable_cors: true,
            max_results: 1000,
        };

        tokio::spawn(async move {
            if let Err(e) = start_api_server(api_storage, api_config).await {
                tracing::error!("API server error: {}", e);
            }
        });
    }

    // Wait for shutdown signal
    let shutdown = tokio::signal::ctrl_c();

    tokio::select! {
        result = receiver.run() => {
            if let Err(e) = result {
                tracing::error!("Receiver error: {}", e);
                return Err(e);
            }
        }
        _ = shutdown => {
            tracing::info!("Received shutdown signal, stopping...");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cli_defaults() {
        // Test that we can create a CLI with defaults
        let cli = Cli {
            command: None,
            grpc_port: None,
            http_port: None,
            memory_limit: None,
            config: None,
            no_fake: false,
            debug: false,
            headless: false,
            terminal: true,
            check_config: false,
            version: false,
            api: false,
            api_port: 8080,
        };

        assert!(!cli.debug);
        assert!(!cli.no_fake);
        assert!(!cli.headless);
        assert!(!cli.api);
        assert_eq!(cli.api_port, 8080);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("1s"), Some(Duration::from_secs(1)));
        assert_eq!(parse_duration("5m"), Some(Duration::from_secs(300)));
        assert_eq!(parse_duration("2h"), Some(Duration::from_secs(7200)));
        assert_eq!(parse_duration("1d"), Some(Duration::from_secs(86400)));
        assert_eq!(parse_duration("invalid"), None);
    }
}

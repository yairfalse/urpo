//! Command-line interface for Urpo.
//!
//! This module provides a simple, htop-like CLI for Urpo.
//! Just run `urpo` to start with sensible defaults!

use crate::core::{Config, Result, UrpoError};
use clap::Parser;
use std::path::PathBuf;

/// Terminal-native OTEL trace explorer - simple as htop!
#[derive(Parser, Debug)]
#[command(name = "urpo")]
#[command(version, about, long_about = None)]
pub struct Cli {
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

    /// Validate configuration and exit
    #[arg(long)]
    pub check_config: bool,

    /// Show version information
    #[arg(short = 'V', long)]
    pub version: bool,
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
            }
            Err(e) if self.config.is_some() => {
                // User explicitly specified a config file that doesn't exist
                return Err(UrpoError::config(format!(
                    "Failed to read config file {:?}: {}",
                    config_path, e
                )));
            }
            Err(_) => {
                // Default config file doesn't exist, that's OK
                tracing::debug!("No config file found at {:?}, using defaults", config_path);
            }
        }
        
        // 2. Apply CLI overrides
        self.build_config_from_args(builder)
    }
    
    fn build_config_from_args(&self, mut builder: crate::core::config::ConfigBuilder) -> Result<Config> {
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
        
        builder = builder
            .enable_fake_spans(!self.no_fake)
            .debug(self.debug);
        
        builder.build()
    }

    /// Initialize logging based on configuration.
    pub fn init_logging(&self) -> Result<()> {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

        // Determine log level
        let log_level = if self.debug {
            "debug"
        } else {
            std::env::var("URPO_LOG_LEVEL")
                .as_deref()
                .unwrap_or("info")
        };

        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(log_level));

        // Configure logging format
        let fmt_layer = if self.headless {
            // Structured logging for headless mode
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
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
        start_headless(config).await
    } else {
        tracing::info!("Starting Urpo with terminal UI...");
        start_with_ui(config).await
    }
}

async fn start_with_ui(config: Config) -> Result<()> {
    use crate::{
        monitoring::ServiceHealthMonitor,
        receiver::OtelReceiver,
        storage::{InMemoryStorage, SpanGenerator},
        ui::Dashboard,
    };
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // Initialize storage
    let storage = Arc::new(RwLock::new(InMemoryStorage::with_config(&config)));
    let health_monitor = Arc::new(ServiceHealthMonitor::new());

    // Start fake span generator if enabled
    if config.features.enable_fake_spans {
        let gen_storage = Arc::clone(&storage);
        let gen_health = Arc::clone(&health_monitor);
        tokio::spawn(async move {
            let generator = SpanGenerator::new();
            if let Err(e) = generator.run(gen_storage, gen_health).await {
                tracing::error!("Fake span generator error: {}", e);
            }
        });
    }

    // Start OTEL receivers
    let receiver = OtelReceiver::new(
        config.server.grpc_port,
        config.server.http_port,
        Arc::clone(&storage),
        Arc::clone(&health_monitor),
    );
    
    let receiver_handle = tokio::spawn(async move {
        if let Err(e) = receiver.run().await {
            tracing::error!("OTEL receiver error: {}", e);
        }
    });

    // Start the terminal UI
    let mut dashboard = Dashboard::new(storage, health_monitor)?;
    
    // Run UI in a separate task to handle signals properly
    let ui_result = tokio::task::spawn_blocking(move || dashboard.run()).await?;
    
    // Cleanup
    receiver_handle.abort();
    
    ui_result
}

async fn start_headless(config: Config) -> Result<()> {
    use crate::{
        monitoring::ServiceHealthMonitor,
        receiver::OtelReceiver,
        storage::{InMemoryStorage, SpanGenerator},
    };
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // Initialize storage
    let storage = Arc::new(RwLock::new(InMemoryStorage::with_config(&config)));
    let health_monitor = Arc::new(ServiceHealthMonitor::new());

    // Start fake span generator if enabled
    if config.features.enable_fake_spans {
        let gen_storage = Arc::clone(&storage);
        let gen_health = Arc::clone(&health_monitor);
        tokio::spawn(async move {
            let generator = SpanGenerator::new();
            if let Err(e) = generator.run(gen_storage, gen_health).await {
                tracing::error!("Fake span generator error: {}", e);
            }
        });
    }

    // Start OTEL receivers
    let receiver = OtelReceiver::new(
        config.server.grpc_port,
        config.server.http_port,
        storage,
        health_monitor,
    );
    
    tracing::info!("Urpo running in headless mode");
    tracing::info!("  GRPC receiver on port {}", config.server.grpc_port);
    tracing::info!("  HTTP receiver on port {}", config.server.http_port);
    
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

    #[test]
    fn test_cli_defaults() {
        // Test that we can create a CLI with defaults
        let cli = Cli {
            grpc_port: None,
            http_port: None,
            memory_limit: None,
            config: None,
            no_fake: false,
            debug: false,
            headless: false,
            check_config: false,
            version: false,
        };
        
        assert!(!cli.debug);
        assert!(!cli.no_fake);
        assert!(!cli.headless);
    }
}

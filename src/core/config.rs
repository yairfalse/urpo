//! Configuration management for Urpo.
//!
//! This module provides comprehensive configuration handling with:
//! - YAML file support
//! - Environment variable overrides
//! - CLI argument overrides
//! - Validation and defaults

use crate::core::{Result, UrpoError};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Duration;

/// Complete configuration for Urpo
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// Storage configuration
    pub storage: StorageConfig,
    /// UI configuration
    pub ui: UiConfig,
    /// Sampling configuration
    pub sampling: SamplingConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Feature flags
    pub features: FeatureConfig,
    /// Debug mode
    #[serde(skip)]
    pub debug: bool,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// GRPC port for OTEL receiver
    pub grpc_port: u16,
    /// HTTP port for OTEL receiver
    pub http_port: u16,
    /// Bind address for receivers
    pub bind_address: IpAddr,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Connection timeout
    #[serde(with = "humantime_serde")]
    pub connection_timeout: Duration,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Maximum number of spans to store
    pub max_spans: usize,
    /// Maximum memory usage in MB
    pub max_memory_mb: usize,
    /// Span retention duration
    #[serde(with = "humantime_serde")]
    pub retention_duration: Duration,
    /// Cleanup interval
    #[serde(with = "humantime_serde")]
    pub cleanup_interval: Duration,
    /// Enable compression
    pub compression_enabled: bool,
    /// Enable persistent storage to disk
    pub persistent: bool,
    /// Data directory for persistent storage
    pub data_dir: PathBuf,
    /// Hot storage size (in-memory ring buffer)
    pub hot_storage_size: usize,
    /// Warm storage size in MB (memory-mapped files)
    pub warm_storage_mb: usize,
    /// Cold storage retention in hours
    pub cold_retention_hours: usize,
    /// Enable archival storage for compressed historical data
    pub enable_archival: bool,
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// UI refresh rate
    #[serde(with = "humantime_serde")]
    pub refresh_rate: Duration,
    /// Color theme
    pub theme: Theme,
    /// Enable vim keybindings
    pub vim_mode: bool,
    /// Show help on startup
    pub show_help: bool,
    /// Default view
    pub default_view: ViewMode,
}

/// Sampling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingConfig {
    /// Default sampling rate (0.0 to 1.0)
    pub default_rate: f64,
    /// Per-service sampling rates
    pub per_service: std::collections::HashMap<String, f64>,
    /// Adaptive sampling enabled
    pub adaptive: bool,
    /// Target spans per second for adaptive sampling
    pub target_sps: Option<usize>,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Health check interval
    #[serde(with = "humantime_serde")]
    pub health_check_interval: Duration,
    /// Metrics enabled
    pub metrics_enabled: bool,
    /// Metrics port
    pub metrics_port: Option<u16>,
    /// Alert thresholds
    pub alerts: AlertConfig,
    /// Maximum metrics to store
    pub max_metrics: usize,
    /// Maximum services to track
    pub max_services: usize,
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Error rate threshold (percentage)
    pub error_rate_threshold: f64,
    /// P95 latency threshold
    #[serde(with = "humantime_serde")]
    pub p95_latency_threshold: Duration,
    /// Minimum sample size for alerts
    pub min_sample_size: usize,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: LogLevel,
    /// Log file path
    pub file: Option<PathBuf>,
    /// Log rotation
    pub rotation: LogRotation,
    /// Structured logging format
    pub structured: bool,
    /// Maximum logs to store in OTLP receiver
    pub max_logs: usize,
    /// Log retention duration
    #[serde(with = "humantime_serde")]
    pub log_retention: Duration,
}

/// Feature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    /// Enable experimental features
    pub experimental: bool,
    /// Enable performance profiling
    pub profiling: bool,
}

/// Color themes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    Auto,
}

/// View modes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    Services,
    Traces,
    Spans,
}

/// Log levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Log rotation strategies
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogRotation {
    Daily,
    Hourly,
    Size,
    Never,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            ui: UiConfig::default(),
            sampling: SamplingConfig::default(),
            monitoring: MonitoringConfig::default(),
            logging: LoggingConfig::default(),
            features: FeatureConfig::default(),
            debug: false,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            grpc_port: 4317,
            http_port: 4318,
            bind_address: "0.0.0.0".parse().expect("Valid default IP address"),
            max_connections: 1000,
            connection_timeout: Duration::from_secs(30),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig {
            max_spans: 100_000,
            max_memory_mb: 1024,
            retention_duration: Duration::from_secs(3600), // 1 hour
            cleanup_interval: Duration::from_secs(30),
            compression_enabled: false,
            persistent: false,
            data_dir: PathBuf::from("./urpo_data"),
            hot_storage_size: 10_000, // 10k spans in hot ring
            warm_storage_mb: 512,     // 512MB warm storage
            cold_retention_hours: 24, // Keep cold data for 24 hours
            enable_archival: false,   // Disabled by default
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        UiConfig {
            refresh_rate: Duration::from_millis(100),
            theme: Theme::Dark,
            vim_mode: true,
            show_help: true,
            default_view: ViewMode::Services,
        }
    }
}

impl Default for SamplingConfig {
    fn default() -> Self {
        SamplingConfig {
            default_rate: 1.0,
            per_service: std::collections::HashMap::new(),
            adaptive: false,
            target_sps: None,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        MonitoringConfig {
            health_check_interval: Duration::from_secs(10),
            metrics_enabled: true,
            metrics_port: None,
            alerts: AlertConfig::default(),
            max_metrics: 1_048_576, // 1M metrics
            max_services: 1000,      // 1000 services
        }
    }
}

impl Default for AlertConfig {
    fn default() -> Self {
        AlertConfig {
            error_rate_threshold: 5.0, // 5%
            p95_latency_threshold: Duration::from_secs(1),
            min_sample_size: 100,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: LogLevel::Info,
            file: None,
            rotation: LogRotation::Daily,
            structured: false,
            max_logs: 100_000,                        // 100K logs
            log_retention: Duration::from_secs(3600), // 1 hour
        }
    }
}

impl Default for FeatureConfig {
    fn default() -> Self {
        FeatureConfig {
            experimental: false,
            profiling: false,
        }
    }
}

impl Config {
    /// Create new config with defaults
    pub fn new() -> Result<Self> {
        let config = Config::default();
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Server validation
        if self.server.grpc_port == self.server.http_port {
            return Err(UrpoError::config(format!(
                "GRPC and HTTP ports must be different: both set to {}",
                self.server.grpc_port
            )));
        }

        if self.server.max_connections == 0 {
            return Err(UrpoError::config("max_connections must be greater than 0"));
        }

        // Storage validation
        if self.storage.max_spans == 0 {
            return Err(UrpoError::config("max_spans must be greater than 0"));
        }

        if self.storage.max_memory_mb == 0 {
            return Err(UrpoError::config("max_memory_mb must be greater than 0"));
        }

        // Sampling validation
        if self.sampling.default_rate < 0.0 || self.sampling.default_rate > 1.0 {
            return Err(UrpoError::InvalidSamplingRate(self.sampling.default_rate));
        }

        for (service, rate) in &self.sampling.per_service {
            if *rate < 0.0 || *rate > 1.0 {
                return Err(UrpoError::config(format!(
                    "Invalid sampling rate for service '{}': {}",
                    service, rate
                )));
            }
        }

        // Alert validation
        if self.monitoring.alerts.error_rate_threshold < 0.0
            || self.monitoring.alerts.error_rate_threshold > 100.0
        {
            return Err(UrpoError::config(format!(
                "Error rate threshold must be between 0 and 100, got {}",
                self.monitoring.alerts.error_rate_threshold
            )));
        }

        Ok(())
    }

    /// Check if a port is available
    pub async fn check_port_available(port: u16) -> Result<()> {
        use tokio::net::TcpListener;

        match TcpListener::bind(("127.0.0.1", port)).await {
            Ok(_) => Ok(()),
            Err(e) => Err(UrpoError::config(format!("Port {} is not available: {}", port, e))),
        }
    }

    /// Validate port availability
    pub async fn validate_ports(&self) -> Result<()> {
        Self::check_port_available(self.server.grpc_port).await?;
        Self::check_port_available(self.server.http_port).await?;

        if let Some(metrics_port) = self.monitoring.metrics_port {
            Self::check_port_available(metrics_port).await?;
        }

        Ok(())
    }

    /// Get sampling rate for a service
    pub fn get_sampling_rate(&self, service: &str) -> f64 {
        self.sampling
            .per_service
            .get(service)
            .copied()
            .unwrap_or(self.sampling.default_rate)
    }

    /// Should sample based on service and rate
    pub fn should_sample(&self, service: &str) -> bool {
        let rate = self.get_sampling_rate(service);
        if rate >= 1.0 {
            true
        } else if rate <= 0.0 {
            false
        } else {
            rand::random::<f64>() < rate
        }
    }
}

impl LogLevel {
    /// Convert to tracing filter string
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

/// Configuration builder for programmatic construction
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    /// Create a new builder with defaults
    pub fn new() -> Self {
        ConfigBuilder {
            config: Config::default(),
        }
    }

    /// Load configuration from YAML string
    pub fn from_yaml(mut self, yaml: &str) -> Result<Self> {
        self.config = serde_yaml::from_str(yaml)
            .map_err(|e| UrpoError::config(format!("Failed to parse YAML config: {}", e)))?;
        Ok(self)
    }

    /// Set GRPC port
    pub fn grpc_port(mut self, port: u16) -> Self {
        self.config.server.grpc_port = port;
        self
    }

    /// Set HTTP port
    pub fn http_port(mut self, port: u16) -> Self {
        self.config.server.http_port = port;
        self
    }

    /// Set max memory
    pub fn max_memory_mb(mut self, mb: usize) -> Self {
        self.config.storage.max_memory_mb = mb;
        self
    }

    /// Set max spans
    pub fn max_spans(mut self, count: usize) -> Self {
        self.config.storage.max_spans = count;
        self
    }

    /// Enable persistent storage
    pub fn persistent(mut self, enable: bool) -> Self {
        self.config.storage.persistent = enable;
        self
    }

    /// Set data directory
    pub fn data_dir(mut self, path: PathBuf) -> Self {
        self.config.storage.data_dir = path;
        self
    }

    /// Set sampling rate
    pub fn sampling_rate(mut self, rate: f64) -> Self {
        self.config.sampling.default_rate = rate;
        self
    }

    /// Set debug mode
    pub fn debug(mut self, debug: bool) -> Self {
        self.config.debug = debug;
        self
    }

    /// Build and validate the configuration
    pub fn build(self) -> Result<Config> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Watch configuration file for changes
pub struct ConfigWatcher {
    path: PathBuf,
    tx: tokio::sync::watch::Sender<Config>,
    rx: tokio::sync::watch::Receiver<Config>,
}

impl ConfigWatcher {
    /// Create a new configuration watcher
    pub fn new(path: PathBuf, initial: Config) -> Self {
        let (tx, rx) = tokio::sync::watch::channel(initial);
        ConfigWatcher { path, tx, rx }
    }

    /// Get a receiver for configuration updates
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<Config> {
        self.rx.clone()
    }

    /// Start watching for configuration changes
    pub async fn watch(self) -> Result<()> {
        use notify::{RecursiveMode, Watcher};
        use std::sync::mpsc::channel;

        let (tx, rx) = channel();

        let mut watcher = notify::recommended_watcher(move |res| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })
        .map_err(|e| UrpoError::config(format!("Failed to create file watcher: {}", e)))?;

        watcher
            .watch(&self.path, RecursiveMode::NonRecursive)
            .map_err(|e| UrpoError::config(format!("Failed to watch config file: {}", e)))?;

        tracing::info!("Watching configuration file: {:?}", self.path);

        // Process file change events
        while let Ok(event) = rx.recv() {
            if matches!(event.kind, notify::EventKind::Modify(_)) {
                tracing::info!("Configuration file changed, reloading...");

                match tokio::fs::read_to_string(&self.path).await {
                    Ok(content) => {
                        match serde_yaml::from_str::<Config>(&content) {
                            Ok(mut new_config) => {
                                if let Err(e) = new_config.validate() {
                                    tracing::error!("Invalid configuration: {}", e);
                                    continue;
                                }

                                // Preserve runtime-only settings
                                new_config.debug = self.tx.borrow().debug;

                                if let Err(e) = self.tx.send(new_config) {
                                    tracing::error!("Failed to update configuration: {}", e);
                                }

                                tracing::info!("Configuration reloaded successfully");
                            },
                            Err(e) => {
                                tracing::error!("Failed to parse configuration: {}", e);
                            },
                        }
                    },
                    Err(e) => {
                        tracing::error!("Failed to read configuration file: {}", e);
                    },
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_sampling_rate() {
        let mut config = Config::default();
        config.sampling.default_rate = 1.5;
        assert!(config.validate().is_err());

        config.sampling.default_rate = -0.1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_port_conflict() {
        let mut config = Config::default();
        config.server.grpc_port = 8080;
        config.server.http_port = 8080;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .grpc_port(9090)
            .http_port(9091)
            .max_memory_mb(2048)
            .sampling_rate(0.5)
            .debug(true)
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.server.grpc_port, 9090);
        assert_eq!(config.server.http_port, 9091);
        assert_eq!(config.storage.max_memory_mb, 2048);
        assert_eq!(config.sampling.default_rate, 0.5);
        assert!(config.debug);
    }

    #[test]
    fn test_sampling_logic() {
        let mut config = Config::default();
        config.sampling.default_rate = 0.0;
        assert!(!config.should_sample("test-service"));

        config.sampling.default_rate = 1.0;
        assert!(config.should_sample("test-service"));

        config
            .sampling
            .per_service
            .insert("special".to_string(), 0.0);
        assert!(!config.should_sample("special"));
        assert!(config.should_sample("other"));
    }

    #[test]
    fn test_yaml_parsing() {
        let yaml = r#"
server:
  bind_address: "127.0.0.1"
  grpc_port: 5317
  http_port: 5318
  max_connections: 1000
  connection_timeout: 30s
storage:
  max_spans: 50000
  max_memory_mb: 512
sampling:
  default_rate: 0.8
  per_service:
    high-volume: 0.1
"#;

        let config = ConfigBuilder::new().from_yaml(yaml).unwrap().build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.server.grpc_port, 5317);
        assert_eq!(config.server.http_port, 5318);
        assert_eq!(config.storage.max_spans, 50000);
        assert_eq!(config.sampling.default_rate, 0.8);
        assert_eq!(config.sampling.per_service.get("high-volume"), Some(&0.1));
    }
}

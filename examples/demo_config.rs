#!/usr/bin/env cargo +nightly -Zscript

//! Demonstration of Urpo's configuration system.
//! 
//! Run with: `cargo +nightly -Zscript demo_config.rs`

use std::collections::HashMap;
use std::time::Duration;
use std::net::IpAddr;

// Simplified configuration structures for demo
#[derive(Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub ui: UiConfig,
    pub sampling: SamplingConfig,
    pub monitoring: MonitoringConfig,
    pub logging: LoggingConfig,
    pub features: FeatureConfig,
    pub debug: bool,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub grpc_port: u16,
    pub http_port: u16,
    pub bind_address: IpAddr,
    pub max_connections: usize,
    pub connection_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub max_spans: usize,
    pub max_memory_mb: usize,
    pub retention_duration: Duration,
    pub cleanup_interval: Duration,
    pub compression_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct UiConfig {
    pub refresh_rate: Duration,
    pub theme: Theme,
    pub vim_mode: bool,
    pub show_help: bool,
    pub default_view: ViewMode,
}

#[derive(Debug, Clone)]
pub struct SamplingConfig {
    pub default_rate: f64,
    pub per_service: HashMap<String, f64>,
    pub adaptive: bool,
    pub target_sps: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub health_check_interval: Duration,
    pub metrics_enabled: bool,
    pub metrics_port: Option<u16>,
    pub alerts: AlertConfig,
}

#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub error_rate_threshold: f64,
    pub p95_latency_threshold: Duration,
    pub min_sample_size: usize,
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub file: Option<std::path::PathBuf>,
    pub rotation: LogRotation,
    pub structured: bool,
}

#[derive(Debug, Clone)]
pub struct FeatureConfig {
    pub enable_fake_spans: bool,
    pub experimental: bool,
    pub profiling: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Theme { Dark, Light, Auto }

#[derive(Debug, Clone, Copy)]
pub enum ViewMode { Services, Traces, Spans }

#[derive(Debug, Clone, Copy)]
pub enum LogLevel { Trace, Debug, Info, Warn, Error }

#[derive(Debug, Clone, Copy)]
pub enum LogRotation { Daily, Hourly, Size, Never }

impl Default for Config {
    fn default() -> Self {
        Config {
            server: ServerConfig {
                grpc_port: 4317,
                http_port: 4318,
                bind_address: "0.0.0.0".parse().unwrap(),
                max_connections: 1000,
                connection_timeout: Duration::from_secs(30),
            },
            storage: StorageConfig {
                max_spans: 100_000,
                max_memory_mb: 1024,
                retention_duration: Duration::from_secs(3600), // 1 hour
                cleanup_interval: Duration::from_secs(30),
                compression_enabled: false,
            },
            ui: UiConfig {
                refresh_rate: Duration::from_millis(100),
                theme: Theme::Dark,
                vim_mode: true,
                show_help: true,
                default_view: ViewMode::Services,
            },
            sampling: SamplingConfig {
                default_rate: 1.0,
                per_service: HashMap::new(),
                adaptive: false,
                target_sps: None,
            },
            monitoring: MonitoringConfig {
                health_check_interval: Duration::from_secs(10),
                metrics_enabled: true,
                metrics_port: None,
                alerts: AlertConfig {
                    error_rate_threshold: 5.0, // 5%
                    p95_latency_threshold: Duration::from_secs(1),
                    min_sample_size: 100,
                },
            },
            logging: LoggingConfig {
                level: LogLevel::Info,
                file: None,
                rotation: LogRotation::Daily,
                structured: false,
            },
            features: FeatureConfig {
                enable_fake_spans: true,
                experimental: false,
                profiling: false,
            },
            debug: false,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        // Port validation
        if self.server.grpc_port == self.server.http_port {
            return Err(format!(
                "GRPC and HTTP ports must be different: both set to {}",
                self.server.grpc_port
            ));
        }
        
        if self.server.max_connections == 0 {
            return Err("max_connections must be greater than 0".to_string());
        }

        // Storage validation
        if self.storage.max_spans == 0 {
            return Err("max_spans must be greater than 0".to_string());
        }
        
        if self.storage.max_memory_mb == 0 {
            return Err("max_memory_mb must be greater than 0".to_string());
        }

        // Sampling validation
        if self.sampling.default_rate < 0.0 || self.sampling.default_rate > 1.0 {
            return Err(format!("Invalid default sampling rate: {}", self.sampling.default_rate));
        }
        
        for (service, rate) in &self.sampling.per_service {
            if *rate < 0.0 || *rate > 1.0 {
                return Err(format!(
                    "Invalid sampling rate for service '{}': {}",
                    service, rate
                ));
            }
        }

        // Alert validation
        if self.monitoring.alerts.error_rate_threshold < 0.0 
            || self.monitoring.alerts.error_rate_threshold > 100.0 {
            return Err(format!(
                "Error rate threshold must be between 0 and 100, got {}",
                self.monitoring.alerts.error_rate_threshold
            ));
        }

        Ok(())
    }

    pub fn get_sampling_rate(&self, service: &str) -> f64 {
        self.sampling
            .per_service
            .get(service)
            .copied()
            .unwrap_or(self.sampling.default_rate)
    }

    pub fn should_sample(&self, service: &str) -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let rate = self.get_sampling_rate(service);
        if rate >= 1.0 {
            true
        } else if rate <= 0.0 {
            false
        } else {
            // Deterministic sampling based on service name hash
            let mut hasher = DefaultHasher::new();
            service.hash(&mut hasher);
            let hash = hasher.finish();
            (hash as f64 / u64::MAX as f64) < rate
        }
    }
}

pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        ConfigBuilder {
            config: Config::default(),
        }
    }

    pub fn grpc_port(mut self, port: u16) -> Self {
        self.config.server.grpc_port = port;
        self
    }

    pub fn http_port(mut self, port: u16) -> Self {
        self.config.server.http_port = port;
        self
    }

    pub fn max_memory_mb(mut self, mb: usize) -> Self {
        self.config.storage.max_memory_mb = mb;
        self
    }

    pub fn max_spans(mut self, count: usize) -> Self {
        self.config.storage.max_spans = count;
        self
    }

    pub fn sampling_rate(mut self, rate: f64) -> Self {
        self.config.sampling.default_rate = rate;
        self
    }

    pub fn enable_fake_spans(mut self, enable: bool) -> Self {
        self.config.features.enable_fake_spans = enable;
        self
    }

    pub fn debug(mut self, debug: bool) -> Self {
        self.config.debug = debug;
        self
    }

    pub fn with_service_sampling(mut self, service: &str, rate: f64) -> Self {
        self.config.sampling.per_service.insert(service.to_string(), rate);
        self
    }

    pub fn build(self) -> Result<Config, String> {
        self.config.validate()?;
        Ok(self.config)
    }
}

fn main() {
    println!("🚀 Urpo Configuration System Demo\n");

    // Example 1: Default configuration
    println!("=== 1. Default Configuration ===");
    let default_config = Config::default();
    match default_config.validate() {
        Ok(()) => println!("✓ Default configuration is valid"),
        Err(e) => println!("✗ Default configuration error: {}", e),
    }
    
    println!("Default config:");
    println!("  GRPC port: {}", default_config.server.grpc_port);
    println!("  HTTP port: {}", default_config.server.http_port);
    println!("  Max memory: {}MB", default_config.storage.max_memory_mb);
    println!("  Max spans: {}", default_config.storage.max_spans);
    println!("  Theme: {:?}", default_config.ui.theme);
    println!("  Vim mode: {}", default_config.ui.vim_mode);

    // Example 2: Builder pattern
    println!("\n=== 2. Configuration Builder ===");
    let custom_config = ConfigBuilder::new()
        .grpc_port(9090)
        .http_port(9091)
        .max_memory_mb(2048)
        .sampling_rate(0.5)
        .debug(true)
        .with_service_sampling("high-volume-service", 0.1)
        .with_service_sampling("debug-service", 1.0)
        .enable_fake_spans(false)
        .build();

    match custom_config {
        Ok(config) => {
            println!("✓ Custom configuration built successfully");
            println!("  GRPC port: {}", config.server.grpc_port);
            println!("  HTTP port: {}", config.server.http_port);
            println!("  Memory limit: {}MB", config.storage.max_memory_mb);
            println!("  Default sampling: {}", config.sampling.default_rate);
            println!("  Debug mode: {}", config.debug);
            println!("  Fake spans: {}", config.features.enable_fake_spans);
            
            println!("  Per-service sampling:");
            for (service, rate) in &config.sampling.per_service {
                println!("    {}: {}", service, rate);
            }
        }
        Err(e) => println!("✗ Configuration build failed: {}", e),
    }

    // Example 3: Validation examples
    println!("\n=== 3. Configuration Validation ===");
    
    // Valid configuration
    let valid = ConfigBuilder::new()
        .grpc_port(4317)
        .http_port(4318)
        .sampling_rate(0.8)
        .build();
    
    match valid {
        Ok(_) => println!("✓ Valid configuration accepted"),
        Err(e) => println!("✗ Unexpected validation error: {}", e),
    }
    
    // Invalid sampling rate
    let invalid_sampling = ConfigBuilder::new()
        .sampling_rate(1.5)
        .build();
    
    match invalid_sampling {
        Ok(_) => println!("✗ Should have rejected invalid sampling rate"),
        Err(e) => println!("✓ Correctly rejected invalid sampling rate: {}", e),
    }
    
    // Same ports
    let invalid_ports = ConfigBuilder::new()
        .grpc_port(8080)
        .http_port(8080)
        .build();
    
    match invalid_ports {
        Ok(_) => println!("✗ Should have rejected duplicate ports"),
        Err(e) => println!("✓ Correctly rejected duplicate ports: {}", e),
    }

    // Example 4: Sampling logic
    println!("\n=== 4. Sampling Logic Demo ===");
    let sampling_config = ConfigBuilder::new()
        .sampling_rate(0.5)
        .with_service_sampling("always-sample", 1.0)
        .with_service_sampling("never-sample", 0.0)
        .with_service_sampling("quarter-sample", 0.25)
        .build()
        .unwrap();

    let test_services = [
        "default-service",
        "always-sample", 
        "never-sample", 
        "quarter-sample",
        "unknown-service",
    ];

    for service in &test_services {
        let rate = sampling_config.get_sampling_rate(service);
        let should_sample = sampling_config.should_sample(service);
        println!("  Service '{}': rate={}, sampled={}", service, rate, should_sample);
    }

    // Example 5: Environment-like configuration
    println!("\n=== 5. Environment-style Configuration ===");
    
    // Simulate different environments
    let environments = [
        ("development", ConfigBuilder::new()
            .debug(true)
            .sampling_rate(1.0)
            .max_memory_mb(512)
            .enable_fake_spans(true)),
        ("staging", ConfigBuilder::new()
            .debug(false)
            .sampling_rate(0.5)
            .max_memory_mb(1024)
            .enable_fake_spans(false)),
        ("production", ConfigBuilder::new()
            .debug(false)
            .sampling_rate(0.1)
            .max_memory_mb(2048)
            .enable_fake_spans(false)
            .with_service_sampling("critical-service", 1.0)),
    ];

    for (env_name, builder) in environments {
        match builder.build() {
            Ok(config) => {
                println!("  {} environment:", env_name.to_uppercase());
                println!("    Debug: {}", config.debug);
                println!("    Sampling: {}", config.sampling.default_rate);
                println!("    Memory: {}MB", config.storage.max_memory_mb);
                println!("    Fake spans: {}", config.features.enable_fake_spans);
            }
            Err(e) => println!("  {} environment failed: {}", env_name, e),
        }
    }

    println!("\n=== Configuration Demo Complete ===");
    println!("Key features demonstrated:");
    println!("• Default configuration with sensible values");
    println!("• Builder pattern for programmatic configuration");
    println!("• Comprehensive validation with clear error messages");
    println!("• Per-service sampling rate configuration");
    println!("• Environment-specific configuration patterns");
    println!("• Type-safe configuration with Rust enums and structs");
    
    println!("\nNext steps:");
    println!("• Add YAML file loading support");
    println!("• Environment variable override support");
    println!("• Hot-reload configuration watching");
    println!("• Integration with CLI argument parsing");
}
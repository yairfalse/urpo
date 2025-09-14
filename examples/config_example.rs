//! Configuration example demonstrating various features.

use urpo_lib::core::{Config, ConfigBuilder, ConfigWatcher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Example 1: Using builder pattern
    println!("=== Configuration Builder Example ===");

    let config = ConfigBuilder::new()
        .grpc_port(9090)
        .http_port(9091)
        .max_memory_mb(2048)
        .sampling_rate(0.5)
        .debug(true)
        .build()?;

    println!("Built config with GRPC port: {}", config.server.grpc_port);
    println!("Built config with HTTP port: {}", config.server.http_port);
    println!("Built config with memory limit: {}MB", config.storage.max_memory_mb);
    println!("Built config with sampling rate: {}", config.sampling.default_rate);

    // Example 2: Loading from YAML
    println!("\n=== YAML Configuration Example ===");

    let yaml_config = r#"
server:
  grpc_port: 5317
  http_port: 5318
  max_connections: 500

storage:
  max_spans: 75000
  max_memory_mb: 1536
  retention_duration: 2h

sampling:
  default_rate: 0.8
  per_service:
    "high-volume-service": 0.1
    "debug-service": 1.0

ui:
  refresh_rate: 50ms
  theme: light
  vim_mode: false

features:
  enable_fake_spans: false
  experimental: true
"#;

    let yaml_config = ConfigBuilder::new().from_yaml(yaml_config)?.build()?;

    println!("YAML config GRPC port: {}", yaml_config.server.grpc_port);
    println!("YAML config max spans: {}", yaml_config.storage.max_spans);
    println!("YAML config theme: {:?}", yaml_config.ui.theme);
    println!("YAML config retention: {:?}", yaml_config.storage.retention_duration);

    // Show per-service sampling rates
    println!("Per-service sampling rates:");
    for (service, rate) in &yaml_config.sampling.per_service {
        println!("  {}: {}", service, rate);
    }

    // Example 3: Validation
    println!("\n=== Configuration Validation Example ===");

    let valid_config = Config::default();
    match valid_config.validate() {
        Ok(()) => println!("✓ Default configuration is valid"),
        Err(e) => println!("✗ Default configuration error: {}", e),
    }

    // Test invalid config
    let invalid_yaml = r#"
sampling:
  default_rate: 1.5  # Invalid: > 1.0
server:
  grpc_port: 4317
  http_port: 4317    # Invalid: same as GRPC port
"#;

    match ConfigBuilder::new()
        .from_yaml(invalid_yaml)
        .and_then(|b| b.build())
    {
        Ok(_) => println!("✗ Should have failed validation"),
        Err(e) => println!("✓ Correctly caught validation error: {}", e),
    }

    // Example 4: Sampling logic
    println!("\n=== Sampling Logic Example ===");

    let sampling_config = ConfigBuilder::new()
        .sampling_rate(0.5)
        .from_yaml(
            r#"
sampling:
  per_service:
    "always-sample": 1.0
    "never-sample": 0.0
    "half-sample": 0.5
"#,
        )?
        .build()?;

    let services = ["default-service", "always-sample", "never-sample", "half-sample"];
    for service in &services {
        let rate = sampling_config.get_sampling_rate(service);
        println!("Service '{}' sampling rate: {}", service, rate);
    }

    // Example 5: Port validation
    println!("\n=== Port Validation Example ===");

    let test_config = ConfigBuilder::new()
        .grpc_port(9999) // Hopefully available
        .http_port(9998) // Hopefully available
        .build()?;

    match test_config.validate_ports().await {
        Ok(()) => println!(
            "✓ Ports {} and {} are available",
            test_config.server.grpc_port, test_config.server.http_port
        ),
        Err(e) => println!("✗ Port validation failed: {}", e),
    }

    // Example 6: Configuration file watching (demo only)
    println!("\n=== Configuration Watching Example ===");

    // Create a temporary config file
    let temp_dir = tempfile::tempdir()?;
    let config_path = temp_dir.path().join("urpo_config.yaml");

    let initial_config_content = r#"
server:
  grpc_port: 6317
  http_port: 6318
storage:
  max_memory_mb: 512
"#;

    tokio::fs::write(&config_path, initial_config_content).await?;

    let initial_config = ConfigBuilder::new()
        .from_yaml(initial_config_content)?
        .build()?;

    let watcher = ConfigWatcher::new(config_path.clone(), initial_config);
    let mut config_rx = watcher.subscribe();

    // Start watching in background
    let watch_handle = tokio::spawn(async move {
        if let Err(e) = watcher.watch().await {
            eprintln!("Config watcher error: {}", e);
        }
    });

    println!("Initial config memory limit: {}MB", config_rx.borrow().storage.max_memory_mb);

    // Update the config file
    let updated_config_content = r#"
server:
  grpc_port: 6317
  http_port: 6318
storage:
  max_memory_mb: 1024  # Changed from 512
"#;

    tokio::fs::write(&config_path, updated_config_content).await?;

    // Wait a bit for the file watcher to pick up the change
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Check if config was updated
    if config_rx.has_changed()? {
        let updated_config = config_rx.borrow_and_update();
        println!("Updated config memory limit: {}MB", updated_config.storage.max_memory_mb);
    } else {
        println!("Config file watching not triggered (this is expected in some environments)");
    }

    // Clean up
    watch_handle.abort();

    println!("\n=== Configuration Examples Complete ===");
    println!("This demonstrates Urpo's flexible configuration system:");
    println!("- Builder pattern for programmatic configuration");
    println!("- YAML file loading with validation");
    println!("- Per-service sampling configuration");
    println!("- Port availability checking");
    println!("- Hot-reload configuration watching");
    println!("\nFor production use, place your config at:");
    println!("  ~/.config/urpo/config.yaml");
    println!("Or specify with --config flag");

    Ok(())
}

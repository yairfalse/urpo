//! Configuration system tests.

use urpo_lib::core::{Config, ConfigBuilder};

#[test]
fn test_default_config() {
    let config = Config::default();
    assert!(config.validate().is_ok());
    assert_eq!(config.server.grpc_port, 4317);
    assert_eq!(config.server.http_port, 4318);
    assert_eq!(config.storage.max_spans, 100_000);
    assert_eq!(config.sampling.default_rate, 1.0);
}

#[test]
fn test_config_builder() {
    let config = ConfigBuilder::new()
        .grpc_port(9090)
        .http_port(9091)
        .max_memory_mb(2048)
        .sampling_rate(0.5)
        .debug(true)
        .build()
        .unwrap();

    assert_eq!(config.server.grpc_port, 9090);
    assert_eq!(config.server.http_port, 9091);
    assert_eq!(config.storage.max_memory_mb, 2048);
    assert_eq!(config.sampling.default_rate, 0.5);
    assert!(config.debug);
}

#[test]
fn test_yaml_config() {
    let yaml = r#"
server:
  grpc_port: 5317
  http_port: 5318
storage:
  max_spans: 50000
  max_memory_mb: 1024
sampling:
  default_rate: 0.8
  per_service:
    "high-volume": 0.1
    "debug": 1.0
ui:
  theme: light
  vim_mode: false
features:
  enable_fake_spans: false
"#;

    let config = ConfigBuilder::new()
        .from_yaml(yaml)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(config.server.grpc_port, 5317);
    assert_eq!(config.server.http_port, 5318);
    assert_eq!(config.storage.max_spans, 50000);
    assert_eq!(config.sampling.default_rate, 0.8);
    assert_eq!(config.sampling.per_service.get("high-volume"), Some(&0.1));
    assert_eq!(config.sampling.per_service.get("debug"), Some(&1.0));
    assert!(!config.features.enable_fake_spans);
}

#[test]
fn test_config_validation() {
    // Valid config should pass
    let valid_config = Config::default();
    assert!(valid_config.validate().is_ok());

    // Invalid sampling rate
    let invalid_config = ConfigBuilder::new().sampling_rate(1.5).build();
    assert!(invalid_config.is_err());

    // Same ports
    let invalid_config = ConfigBuilder::new().grpc_port(8080).http_port(8080).build();
    assert!(invalid_config.is_err());
}

#[test]
fn test_sampling_logic() {
    let config = ConfigBuilder::new()
        .sampling_rate(0.5)
        .from_yaml(
            r#"
sampling:
  per_service:
    "always": 1.0
    "never": 0.0
    "half": 0.5
"#,
        )
        .unwrap()
        .build()
        .unwrap();

    // Test service-specific rates
    assert_eq!(config.get_sampling_rate("always"), 1.0);
    assert_eq!(config.get_sampling_rate("never"), 0.0);
    assert_eq!(config.get_sampling_rate("half"), 0.5);

    // Test default rate for unknown service
    assert_eq!(config.get_sampling_rate("unknown"), 0.5);

    // Test sampling decisions
    assert!(config.should_sample("always"));
    assert!(!config.should_sample("never"));
}

#[test]
fn test_error_handling() {
    // Invalid YAML
    let result = ConfigBuilder::new().from_yaml("invalid: yaml: content: [");
    assert!(result.is_err());

    // Invalid field values
    let result = ConfigBuilder::new().from_yaml(
        r#"
sampling:
  default_rate: "not_a_number"
"#,
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn test_port_validation() {
    let config = ConfigBuilder::new()
        .grpc_port(0) // Port 0 will bind to any available port
        .http_port(0)
        .build()
        .unwrap();

    // This should succeed since port 0 is always available
    assert!(config.validate_ports().await.is_ok());
}

#[test]
fn test_config_themes() {
    let yaml = r#"
ui:
  theme: auto
  refresh_rate: 200ms
logging:
  level: debug
  rotation: hourly
"#;

    let config = ConfigBuilder::new()
        .from_yaml(yaml)
        .unwrap()
        .build()
        .unwrap();

    assert!(matches!(config.ui.theme, urpo::core::config::Theme::Auto));
    assert!(matches!(
        config.logging.level,
        urpo::core::config::LogLevel::Debug
    ));
    assert!(matches!(
        config.logging.rotation,
        urpo::core::config::LogRotation::Hourly
    ));
    assert_eq!(config.ui.refresh_rate.as_millis(), 200);
}

#[test]
fn test_humantime_durations() {
    let yaml = r#"
storage:
  retention_duration: 3h
  cleanup_interval: 5m
monitoring:
  health_check_interval: 30s
ui:
  refresh_rate: 100ms
"#;

    let config = ConfigBuilder::new()
        .from_yaml(yaml)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(config.storage.retention_duration.as_secs(), 3 * 60 * 60); // 3 hours
    assert_eq!(config.storage.cleanup_interval.as_secs(), 5 * 60); // 5 minutes
    assert_eq!(config.monitoring.health_check_interval.as_secs(), 30); // 30 seconds
    assert_eq!(config.ui.refresh_rate.as_millis(), 100); // 100 ms
}

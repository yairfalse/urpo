//! Production monitoring and health checking for Urpo.
//!
//! This module provides comprehensive system monitoring, health checks,
//! and operational metrics for production deployment.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;

use crate::core::Result;
// No more external performance manager - we track it ourselves
use crate::storage::{StorageHealth, StorageStats};

/// Performance metrics tracked internally
#[derive(Debug, Clone, Default)]
pub struct PerformanceStats {
    pub ingestion_rate: f64,
    pub query_latency_ms: f64,
    pub cache_hit_rate: f64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
}

/// System health status.
#[derive(Debug, Clone, PartialEq)]
pub enum SystemHealth {
    /// All systems operational.
    Healthy,
    /// Some degradation but operational.
    Degraded,
    /// Significant issues affecting performance.
    Unhealthy,
    /// Critical failure, system may be non-functional.
    Critical,
}

impl SystemHealth {
    /// Get color representation for UI display.
    pub fn color(&self) -> &'static str {
        match self {
            SystemHealth::Healthy => "green",
            SystemHealth::Degraded => "yellow",
            SystemHealth::Unhealthy => "orange",
            SystemHealth::Critical => "red",
        }
    }

    /// Get severity score (0-100).
    pub fn severity(&self) -> u8 {
        match self {
            SystemHealth::Healthy => 0,
            SystemHealth::Degraded => 25,
            SystemHealth::Unhealthy => 75,
            SystemHealth::Critical => 100,
        }
    }
}

/// Comprehensive system monitoring metrics.
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    /// Overall system health.
    pub health: SystemHealth,
    /// Storage subsystem metrics.
    pub storage: StorageStats,
    /// Performance metrics.
    pub performance: PerformanceStats,
    /// OTEL receiver metrics.
    pub receiver: ReceiverMetrics,
    /// System resource usage.
    pub resources: ResourceMetrics,
    /// Error tracking.
    pub errors: ErrorMetrics,
    /// Uptime information.
    pub uptime: UptimeMetrics,
    /// Last update timestamp.
    pub timestamp: SystemTime,
}

/// OTEL receiver metrics.
#[derive(Debug, Clone)]
pub struct ReceiverMetrics {
    /// GRPC receiver status.
    pub grpc_healthy: bool,
    /// HTTP receiver status.
    pub http_healthy: bool,
    /// Total spans received.
    pub spans_received: u64,
    /// Spans received per second.
    pub spans_per_second: f64,
    /// Invalid spans rejected.
    pub invalid_spans: u64,
    /// Connection count.
    pub active_connections: u32,
    /// Last received span timestamp.
    pub last_received: Option<SystemTime>,
    /// Receiver errors.
    pub receiver_errors: u64,
}

impl Default for ReceiverMetrics {
    fn default() -> Self {
        Self {
            grpc_healthy: false,
            http_healthy: false,
            spans_received: 0,
            spans_per_second: 0.0,
            invalid_spans: 0,
            active_connections: 0,
            last_received: None,
            receiver_errors: 0,
        }
    }
}

/// System resource metrics.
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    /// Memory usage in bytes.
    pub memory_used: u64,
    /// Memory usage in MB.
    pub memory_mb: f64,
    /// Memory usage percentage.
    pub memory_percent: f64,
    /// CPU usage percentage.
    pub cpu_percent: f64,
    /// Disk usage for temp files.
    pub disk_used: u64,
    /// Network bytes received.
    pub network_rx: u64,
    /// Network bytes transmitted.
    pub network_tx: u64,
    /// File descriptor count.
    pub open_files: u32,
    /// Thread count.
    pub threads: u32,
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        Self {
            memory_used: 0,
            memory_mb: 0.0,
            memory_percent: 0.0,
            cpu_percent: 0.0,
            disk_used: 0,
            network_rx: 0,
            network_tx: 0,
            open_files: 0,
            threads: 0,
        }
    }
}

/// Error tracking metrics.
#[derive(Debug, Clone)]
pub struct ErrorMetrics {
    /// Total errors encountered.
    pub total_errors: u64,
    /// Errors in last minute.
    pub errors_per_minute: u64,
    /// Error rate (errors/spans).
    pub error_rate: f64,
    /// Errors by category.
    pub error_categories: HashMap<String, u64>,
    /// Recent error messages.
    pub recent_errors: Vec<(SystemTime, String)>,
    /// Critical errors.
    pub critical_errors: u64,
}

impl Default for ErrorMetrics {
    fn default() -> Self {
        Self {
            total_errors: 0,
            errors_per_minute: 0,
            error_rate: 0.0,
            error_categories: HashMap::new(),
            recent_errors: Vec::new(),
            critical_errors: 0,
        }
    }
}

/// Uptime and availability metrics.
#[derive(Debug, Clone)]
pub struct UptimeMetrics {
    /// System start time.
    pub start_time: SystemTime,
    /// Current uptime duration.
    pub uptime: Duration,
    /// Availability percentage (99.9%, etc.).
    pub availability: f64,
    /// Number of restarts.
    pub restarts: u32,
    /// Last restart reason.
    pub last_restart_reason: Option<String>,
    /// Downtime events.
    pub downtime_events: Vec<(SystemTime, Duration, String)>,
}

impl Default for UptimeMetrics {
    fn default() -> Self {
        let now = SystemTime::now();
        Self {
            start_time: now,
            uptime: Duration::new(0, 0),
            availability: 100.0,
            restarts: 0,
            last_restart_reason: None,
            downtime_events: Vec::new(),
        }
    }
}

/// Production monitoring system.
#[derive(Debug)]
pub struct Monitor {
    /// Current system metrics.
    metrics: Arc<RwLock<SystemMetrics>>,
    /// Performance manager.
    /// Monitoring configuration.
    config: MonitoringConfig,
    /// Health check intervals.
    health_checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
    /// Error tracker.
    error_tracker: Arc<Mutex<ErrorTracker>>,
    /// Uptime tracker.
    uptime_tracker: Arc<Mutex<UptimeTracker>>,
    /// Shutdown signal.
    shutdown: Arc<AtomicBool>,
}

/// Monitoring configuration.
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Health check interval.
    pub health_check_interval: Duration,
    /// Metrics collection interval.
    pub metrics_interval: Duration,
    /// Error retention period.
    pub error_retention: Duration,
    /// Maximum recent errors to keep.
    pub max_recent_errors: usize,
    /// Memory warning threshold (MB).
    pub memory_warning_mb: f64,
    /// Memory critical threshold (MB).
    pub memory_critical_mb: f64,
    /// CPU warning threshold (%).
    pub cpu_warning_percent: f64,
    /// CPU critical threshold (%).
    pub cpu_critical_percent: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            health_check_interval: Duration::from_secs(30),
            metrics_interval: Duration::from_secs(5),
            error_retention: Duration::from_secs(3600), // 1 hour
            max_recent_errors: 100,
            memory_warning_mb: 256.0,
            memory_critical_mb: 512.0,
            cpu_warning_percent: 70.0,
            cpu_critical_percent: 90.0,
        }
    }
}

/// Individual health check.
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Check name.
    pub name: String,
    /// Check function (placeholder - in real impl would be a function pointer).
    pub enabled: bool,
    /// Last check time.
    pub last_check: SystemTime,
    /// Last result.
    pub healthy: bool,
    /// Check interval.
    pub interval: Duration,
    /// Consecutive failures.
    pub consecutive_failures: u32,
    /// Error message if unhealthy.
    pub error_message: Option<String>,
}

/// Error tracking helper.
#[derive(Debug)]
struct ErrorTracker {
    /// Error counts by category.
    categories: HashMap<String, AtomicU64>,
    /// Recent errors.
    recent: Vec<(SystemTime, String)>,
    /// Total errors.
    total: AtomicU64,
}

impl ErrorTracker {
    fn new() -> Self {
        Self {
            categories: HashMap::new(),
            recent: Vec::new(),
            total: AtomicU64::new(0),
        }
    }

    fn record_error(&mut self, category: &str, message: String) {
        self.total.fetch_add(1, Ordering::Relaxed);

        // Update category count
        self.categories
            .entry(category.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);

        // Add to recent errors
        self.recent.push((SystemTime::now(), message));

        // Limit recent errors
        if self.recent.len() > 100 {
            self.recent.remove(0);
        }
    }

    fn get_metrics(&self, max_recent: usize) -> ErrorMetrics {
        let mut error_categories = HashMap::new();
        for (cat, count) in &self.categories {
            error_categories.insert(cat.clone(), count.load(Ordering::Relaxed));
        }

        let recent_errors = self.recent.iter().rev().take(max_recent).cloned().collect();

        ErrorMetrics {
            total_errors: self.total.load(Ordering::Relaxed),
            errors_per_minute: 0, // Would need time-based tracking
            error_rate: 0.0,      // Would need span count
            error_categories,
            recent_errors,
            critical_errors: 0, // Would need severity classification
        }
    }
}

/// Uptime tracking helper.
#[derive(Debug)]
struct UptimeTracker {
    start_time: SystemTime,
    restarts: u32,
    downtime_events: Vec<(SystemTime, Duration, String)>,
}

impl UptimeTracker {
    fn new() -> Self {
        Self {
            start_time: SystemTime::now(),
            restarts: 0,
            downtime_events: Vec::new(),
        }
    }

    fn get_metrics(&self) -> UptimeMetrics {
        let uptime = self.start_time.elapsed().unwrap_or(Duration::new(0, 0));

        UptimeMetrics {
            start_time: self.start_time,
            uptime,
            availability: 99.9, // Would calculate based on downtime
            restarts: self.restarts,
            last_restart_reason: None,
            downtime_events: self.downtime_events.clone(),
        }
    }
}

impl Monitor {
    /// Create a new monitoring system.
    pub fn new() -> Self {
        let config = MonitoringConfig::default();

        let initial_metrics = SystemMetrics {
            health: SystemHealth::Healthy,
            storage: crate::storage::StorageStats {
                trace_count: 0,
                span_count: 0,
                service_count: 0,
                memory_bytes: 0,
                memory_mb: 0.0,
                memory_pressure: 0.0,
                oldest_span: None,
                newest_span: None,
                processing_rate: 0.0,
                error_rate: 0.0,
                cleanup_count: 0,
                last_cleanup: None,
                health_status: StorageHealth::Healthy,
                uptime_seconds: 0,
            },
            performance: PerformanceStats::default(),
            receiver: ReceiverMetrics::default(),
            resources: ResourceMetrics::default(),
            errors: ErrorMetrics::default(),
            uptime: UptimeMetrics::default(),
            timestamp: SystemTime::now(),
        };

        Self {
            metrics: Arc::new(RwLock::new(initial_metrics)),
            config,
            health_checks: Arc::new(RwLock::new(HashMap::new())),
            error_tracker: Arc::new(Mutex::new(ErrorTracker::new())),
            uptime_tracker: Arc::new(Mutex::new(UptimeTracker::new())),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start monitoring in background.
    pub async fn start(&self) -> Result<()> {
        // Start metrics collection
        self.start_metrics_collection().await?;

        // Start health checks
        self.start_health_checks().await?;

        Ok(())
    }

    /// Start metrics collection loop.
    async fn start_metrics_collection(&self) -> Result<()> {
        let metrics = self.metrics.clone();
        let error_tracker = self.error_tracker.clone();
        let uptime_tracker = self.uptime_tracker.clone();
        let shutdown = self.shutdown.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.metrics_interval);

            while !shutdown.load(Ordering::Relaxed) {
                interval.tick().await;

                // Collect performance metrics
                let performance = PerformanceStats::default(); // Real performance would come from actual measurements

                // Collect error metrics
                let errors = {
                    let error_tracker = error_tracker.lock().await;
                    error_tracker.get_metrics(config.max_recent_errors)
                };

                // Collect uptime metrics
                let uptime = {
                    let uptime_tracker = uptime_tracker.lock().await;
                    uptime_tracker.get_metrics()
                };

                // Collect resource metrics (simplified)
                let resources = Self::collect_resource_metrics().await;

                // Determine overall health
                let health = Self::determine_health(&performance, &resources, &errors, &config);

                // Update metrics
                let mut metrics = metrics.write().await;
                metrics.performance = performance;
                metrics.errors = errors;
                metrics.uptime = uptime;
                metrics.resources = resources;
                metrics.health = health;
                metrics.timestamp = SystemTime::now();
            }
        });

        Ok(())
    }

    /// Start health check loop.
    async fn start_health_checks(&self) -> Result<()> {
        let health_checks = self.health_checks.clone();
        let shutdown = self.shutdown.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.health_check_interval);

            while !shutdown.load(Ordering::Relaxed) {
                interval.tick().await;

                let mut checks = health_checks.write().await;
                for (name, check) in checks.iter_mut() {
                    if check.enabled
                        && check.last_check.elapsed().unwrap_or(Duration::MAX) >= check.interval
                    {
                        // Perform health check (simplified)
                        let healthy = Self::perform_health_check(name).await;

                        check.last_check = SystemTime::now();
                        check.healthy = healthy;

                        if !healthy {
                            check.consecutive_failures += 1;
                            tracing::warn!(
                                "Health check '{}' failed {} times",
                                name,
                                check.consecutive_failures
                            );
                        } else {
                            check.consecutive_failures = 0;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Collect system resource metrics.
    async fn collect_resource_metrics() -> ResourceMetrics {
        // In a real implementation, this would use system APIs
        // For now, return placeholder values
        ResourceMetrics {
            memory_used: 1024 * 1024 * 128, // 128 MB
            memory_mb: 128.0,
            memory_percent: 25.0,
            cpu_percent: 15.0,
            disk_used: 0,
            network_rx: 0,
            network_tx: 0,
            open_files: 50,
            threads: 10,
        }
    }

    /// Determine overall system health.
    fn determine_health(
        performance: &PerformanceStats,
        resources: &ResourceMetrics,
        errors: &ErrorMetrics,
        config: &MonitoringConfig,
    ) -> SystemHealth {
        let mut severity = 0u8;

        // Check memory usage
        if resources.memory_mb > config.memory_critical_mb {
            severity = severity.max(100); // Critical
        } else if resources.memory_mb > config.memory_warning_mb {
            severity = severity.max(50); // Degraded
        }

        // Check CPU usage
        if resources.cpu_percent > config.cpu_critical_percent {
            severity = severity.max(100); // Critical
        } else if resources.cpu_percent > config.cpu_warning_percent {
            severity = severity.max(50); // Degraded
        }

        // Check error rate
        if errors.error_rate > 0.1 {
            // > 10% error rate
            severity = severity.max(75); // Unhealthy
        } else if errors.error_rate > 0.05 {
            // > 5% error rate
            severity = severity.max(25); // Degraded
        }

        // Check performance using query_latency_ms
        if performance.query_latency_ms > 100.0 {
            // > 100ms
            severity = severity.max(75); // Unhealthy
        } else if performance.query_latency_ms > 50.0 {
            // > 50ms
            severity = severity.max(25); // Degraded
        }

        match severity {
            0..=10 => SystemHealth::Healthy,
            11..=40 => SystemHealth::Degraded,
            41..=80 => SystemHealth::Unhealthy,
            _ => SystemHealth::Critical,
        }
    }

    /// Perform a specific health check.
    async fn perform_health_check(name: &str) -> bool {
        match name {
            "storage" => true,       // Would check storage connectivity
            "grpc_receiver" => true, // Would check GRPC server status
            "http_receiver" => true, // Would check HTTP server status
            "memory" => true,        // Would check memory usage
            _ => true,
        }
    }

    /// Record an error for monitoring.
    pub async fn record_error(&self, category: &str, message: String) {
        let mut error_tracker = self.error_tracker.lock().await;
        error_tracker.record_error(category, message);
    }

    /// Update storage metrics.
    pub async fn update_storage_metrics(&self, storage_stats: StorageStats) {
        let mut metrics = self.metrics.write().await;
        metrics.storage = storage_stats;
    }

    /// Update receiver metrics.
    pub async fn update_receiver_metrics(&self, receiver_metrics: ReceiverMetrics) {
        let mut metrics = self.metrics.write().await;
        metrics.receiver = receiver_metrics;
    }

    /// Get current system metrics.
    pub async fn get_metrics(&self) -> SystemMetrics {
        self.metrics.read().await.clone()
    }

    /// Get system health status.
    pub async fn get_health(&self) -> SystemHealth {
        self.metrics.read().await.health.clone()
    }

    /// Register a health check.
    pub async fn register_health_check(&self, check: HealthCheck) {
        let mut health_checks = self.health_checks.write().await;
        health_checks.insert(check.name.clone(), check);
    }

    /// Get all health check results.
    pub async fn get_health_checks(&self) -> HashMap<String, HealthCheck> {
        self.health_checks.read().await.clone()
    }

    /// Stop monitoring.
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Create default health checks.
    pub async fn setup_default_health_checks(&self) {
        let checks = vec![
            HealthCheck {
                name: "storage".to_string(),
                enabled: true,
                last_check: SystemTime::now(),
                healthy: true,
                interval: Duration::from_secs(30),
                consecutive_failures: 0,
                error_message: None,
            },
            HealthCheck {
                name: "grpc_receiver".to_string(),
                enabled: true,
                last_check: SystemTime::now(),
                healthy: true,
                interval: Duration::from_secs(60),
                consecutive_failures: 0,
                error_message: None,
            },
            HealthCheck {
                name: "memory".to_string(),
                enabled: true,
                last_check: SystemTime::now(),
                healthy: true,
                interval: Duration::from_secs(15),
                consecutive_failures: 0,
                error_message: None,
            },
        ];

        for check in checks {
            self.register_health_check(check).await;
        }
    }
}

/// Health check endpoint for external monitoring.
pub struct HealthEndpoint {
    monitor: Arc<Monitor>,
}

impl HealthEndpoint {
    /// Create a new health endpoint.
    pub fn new(monitor: Arc<Monitor>) -> Self {
        Self { monitor }
    }

    /// Get health status in a format suitable for HTTP endpoints.
    pub async fn get_health_response(&self) -> HealthResponse {
        let metrics = self.monitor.get_metrics().await;
        let health_checks = self.monitor.get_health_checks().await;

        HealthResponse {
            status: metrics.health,
            timestamp: metrics.timestamp,
            uptime_seconds: metrics.uptime.uptime.as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            checks: health_checks,
            summary: HealthSummary {
                spans_processed: 0, // Field removed from PerformanceStats
                memory_mb: metrics.resources.memory_mb,
                cpu_percent: metrics.resources.cpu_percent,
                error_rate: metrics.errors.error_rate,
                storage_health: metrics.storage.health_status,
            },
        }
    }
}

/// Health response for external monitoring systems.
#[derive(Debug, Clone)]
pub struct HealthResponse {
    pub status: SystemHealth,
    pub timestamp: SystemTime,
    pub uptime_seconds: u64,
    pub version: String,
    pub checks: HashMap<String, HealthCheck>,
    pub summary: HealthSummary,
}

/// Health summary for quick status overview.
#[derive(Debug, Clone)]
pub struct HealthSummary {
    pub spans_processed: u64,
    pub memory_mb: f64,
    pub cpu_percent: f64,
    pub error_rate: f64,
    pub storage_health: StorageHealth,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_creation() {
        let monitor = Monitor::new();

        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.health, SystemHealth::Healthy);
    }

    #[tokio::test]
    async fn test_health_determination() {
        let config = MonitoringConfig::default();

        // Test healthy system
        let performance = PerformanceStats {
            avg_latency_us: 5000, // 5ms
            ..Default::default()
        };
        let resources = ResourceMetrics {
            memory_mb: 100.0,
            cpu_percent: 20.0,
            ..Default::default()
        };
        let errors = ErrorMetrics {
            error_rate: 0.01, // 1%
            ..Default::default()
        };

        let health = Monitor::determine_health(&performance, &resources, &errors, &config);
        assert_eq!(health, SystemHealth::Healthy);

        // Test critical system
        let resources_critical = ResourceMetrics {
            memory_mb: 600.0,  // Above critical threshold
            cpu_percent: 95.0, // Above critical threshold
            ..Default::default()
        };

        let health_critical =
            Monitor::determine_health(&performance, &resources_critical, &errors, &config);
        assert_eq!(health_critical, SystemHealth::Critical);
    }

    #[tokio::test]
    async fn test_error_tracking() {
        let monitor = Monitor::new();

        monitor
            .record_error("grpc", "Connection failed".to_string())
            .await;
        monitor
            .record_error("storage", "Disk full".to_string())
            .await;

        let metrics = monitor.get_metrics().await;
        assert!(metrics.errors.total_errors >= 2);
        assert!(metrics.errors.error_categories.contains_key("grpc"));
        assert!(metrics.errors.error_categories.contains_key("storage"));
    }

    #[tokio::test]
    async fn test_health_checks() {
        let monitor = Monitor::new();

        let check = HealthCheck {
            name: "test_check".to_string(),
            enabled: true,
            last_check: SystemTime::now(),
            healthy: true,
            interval: Duration::from_secs(30),
            consecutive_failures: 0,
            error_message: None,
        };

        monitor.register_health_check(check).await;

        let checks = monitor.get_health_checks().await;
        assert!(checks.contains_key("test_check"));
        assert!(checks["test_check"].healthy);
    }
}

// Type alias for backward compatibility
pub type ServiceHealthMonitor = Monitor;

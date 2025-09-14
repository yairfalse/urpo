//! Diagnostics and error reporting utilities.

use crate::core::UrpoError;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Error statistics for monitoring
#[derive(Debug, Clone)]
pub struct ErrorStats {
    /// Total errors by category
    pub by_category: HashMap<String, u64>,
    /// Total errors by operation
    pub by_operation: HashMap<String, u64>,
    /// Recent errors (last 100)
    pub recent_errors: Vec<ErrorInfo>,
    /// Error rate per second
    pub error_rate: f64,
    /// Time window for rate calculation
    pub window: Duration,
}

/// Information about a single error occurrence
#[derive(Debug, Clone)]
pub struct ErrorInfo {
    /// When the error occurred
    pub timestamp: Instant,
    /// Error category
    pub category: String,
    /// Error message
    pub message: String,
    /// Operation that failed
    pub operation: Option<String>,
    /// Whether it was recovered
    pub recovered: bool,
}

/// Error diagnostics collector
pub struct DiagnosticsCollector {
    /// Error counts by category
    category_counts: Arc<RwLock<HashMap<String, AtomicU64>>>,
    /// Error counts by operation
    operation_counts: Arc<RwLock<HashMap<String, AtomicU64>>>,
    /// Recent errors (circular buffer)
    recent_errors: Arc<RwLock<Vec<ErrorInfo>>>,
    /// Maximum recent errors to keep
    max_recent: usize,
    /// Start time for rate calculations
    start_time: Instant,
    /// Total error count
    total_errors: AtomicU64,
}

impl DiagnosticsCollector {
    /// Create a new diagnostics collector
    pub fn new(max_recent: usize) -> Self {
        Self {
            category_counts: Arc::new(RwLock::new(HashMap::new())),
            operation_counts: Arc::new(RwLock::new(HashMap::new())),
            recent_errors: Arc::new(RwLock::new(Vec::with_capacity(max_recent))),
            max_recent,
            start_time: Instant::now(),
            total_errors: AtomicU64::new(0),
        }
    }

    /// Record an error occurrence
    pub async fn record_error(
        &self,
        error: &UrpoError,
        operation: Option<String>,
        recovered: bool,
    ) {
        let category = error.category().to_string();
        let message = error.to_string();

        // Update category counts
        {
            let mut counts = self.category_counts.write().await;
            counts
                .entry(category.clone())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }

        // Update operation counts if provided
        if let Some(ref op) = operation {
            let mut counts = self.operation_counts.write().await;
            counts
                .entry(op.clone())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }

        // Add to recent errors
        {
            let mut recent = self.recent_errors.write().await;
            if recent.len() >= self.max_recent {
                recent.remove(0);
            }
            recent.push(ErrorInfo {
                timestamp: Instant::now(),
                category,
                message,
                operation,
                recovered,
            });
        }

        self.total_errors.fetch_add(1, Ordering::Relaxed);

        // Log based on recovery status
        if recovered {
            tracing::warn!("Error recovered: {}", error);
        } else {
            tracing::error!("Error not recovered: {}", error);
        }
    }

    /// Get current error statistics
    pub async fn get_stats(&self) -> ErrorStats {
        let elapsed = self.start_time.elapsed();
        let total = self.total_errors.load(Ordering::Relaxed);
        let error_rate = if elapsed.as_secs() > 0 {
            total as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let category_counts = self.category_counts.read().await;
        let by_category: HashMap<String, u64> = category_counts
            .iter()
            .map(|(k, v)| (k.clone(), v.load(Ordering::Relaxed)))
            .collect();

        let operation_counts = self.operation_counts.read().await;
        let by_operation: HashMap<String, u64> = operation_counts
            .iter()
            .map(|(k, v)| (k.clone(), v.load(Ordering::Relaxed)))
            .collect();

        let recent = self.recent_errors.read().await;

        ErrorStats {
            by_category,
            by_operation,
            recent_errors: recent.clone(),
            error_rate,
            window: elapsed,
        }
    }

    /// Clear all statistics
    pub async fn clear(&self) {
        self.category_counts.write().await.clear();
        self.operation_counts.write().await.clear();
        self.recent_errors.write().await.clear();
        self.total_errors.store(0, Ordering::Relaxed);
    }

    /// Get a user-friendly error summary
    pub async fn get_summary(&self) -> String {
        let stats = self.get_stats().await;

        let mut summary = String::new();
        summary.push_str("Error Summary:\n");
        summary.push_str(&format!(
            "  Total errors: {}\n",
            self.total_errors.load(Ordering::Relaxed)
        ));
        summary.push_str(&format!("  Error rate: {:.2}/sec\n", stats.error_rate));

        if !stats.by_category.is_empty() {
            summary.push_str("\nErrors by category:\n");
            let mut categories: Vec<_> = stats.by_category.iter().collect();
            categories.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
            for (category, count) in categories.iter().take(5) {
                summary.push_str(&format!("  {}: {}\n", category, count));
            }
        }

        if !stats.recent_errors.is_empty() {
            summary.push_str("\nRecent errors:\n");
            for error in stats.recent_errors.iter().rev().take(3) {
                let ago = Instant::now().duration_since(error.timestamp);
                summary.push_str(&format!(
                    "  [{:?} ago] {}: {}\n",
                    ago, error.category, error.message
                ));
            }
        }

        summary
    }
}

/// Format an error for user display with helpful context
pub fn format_user_error(error: &UrpoError) -> String {
    let mut output = String::new();

    // Main error message
    output.push_str(&format!("Error: {}\n", error));

    // Add category-specific help
    output.push_str("\n");
    match error.category() {
        "config" => {
            output.push_str("Configuration issue detected. Please check:\n");
            output.push_str("  • Your config file syntax (YAML format)\n");
            output.push_str("  • Required fields are present\n");
            output.push_str("  • Port numbers are valid (1-65535)\n");
            output.push_str("  • File paths exist and are accessible\n");
            output.push_str("\nRun 'urpo --check-config' to validate your configuration.\n");
        }
        "network" => {
            output.push_str("Network issue detected. Please check:\n");
            output.push_str("  • The OTEL collectors are running\n");
            output.push_str("  • Firewall rules allow connections\n");
            output.push_str("  • Ports 4317 (GRPC) and 4318 (HTTP) are available\n");
            output.push_str("  • Network connectivity to remote services\n");
        }
        "storage" => {
            output.push_str("Storage issue detected. Consider:\n");
            output.push_str("  • Increasing memory limits (--memory-limit)\n");
            output.push_str("  • Reducing span retention period\n");
            output.push_str("  • Enabling sampling to reduce data volume\n");
            output.push_str("  • Checking available system memory\n");
        }
        "resource" => {
            output.push_str("Resource limit exceeded. Try:\n");
            output.push_str("  • Increasing memory allocation\n");
            output.push_str("  • Reducing concurrent connections\n");
            output.push_str("  • Enabling sampling (--sampling-rate 0.1)\n");
            output.push_str("  • Restarting to clear accumulated data\n");
        }
        "validation" => {
            output.push_str("Validation error. Please ensure:\n");
            output.push_str("  • Sampling rates are between 0.0 and 1.0\n");
            output.push_str("  • Service names are valid\n");
            output.push_str("  • Span data follows OTEL specifications\n");
        }
        _ => {
            output.push_str("For more information:\n");
            output.push_str("  • Check the logs with --debug flag\n");
            output.push_str("  • Verify your configuration\n");
            output.push_str("  • Report issues at https://github.com/user/urpo\n");
        }
    }

    // Add recovery hint if applicable
    if error.is_recoverable() {
        output.push_str("\nThis error may be temporary. Urpo will automatically retry.\n");
    }

    output
}

/// System health check
pub struct HealthChecker {
    checks: Vec<Box<dyn HealthCheck>>,
}

#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    /// Perform the health check
    async fn check(&self) -> HealthStatus;

    /// Get the check name
    fn name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub message: String,
    pub details: HashMap<String, String>,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Add a health check
    pub fn add_check(&mut self, check: Box<dyn HealthCheck>) {
        self.checks.push(check);
    }

    /// Run all health checks
    pub async fn check_all(&self) -> Vec<(String, HealthStatus)> {
        let mut results = Vec::new();

        for check in &self.checks {
            let name = check.name().to_string();
            let status = check.check().await;
            results.push((name, status));
        }

        results
    }

    /// Get overall health status
    pub async fn is_healthy(&self) -> bool {
        for check in &self.checks {
            let status = check.check().await;
            if !status.healthy {
                return false;
            }
        }
        true
    }
}

/// Port availability check
pub struct PortCheck {
    port: u16,
    name: String,
}

impl PortCheck {
    pub fn new(name: &str, port: u16) -> Self {
        Self {
            port,
            name: name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for PortCheck {
    async fn check(&self) -> HealthStatus {
        use tokio::net::TcpListener;

        match TcpListener::bind(("127.0.0.1", self.port)).await {
            Ok(_) => HealthStatus {
                healthy: true,
                message: format!("Port {} is available", self.port),
                details: HashMap::new(),
            },
            Err(e) => HealthStatus {
                healthy: false,
                message: format!("Port {} is not available: {}", self.port, e),
                details: HashMap::new(),
            },
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_diagnostics_collector() {
        let collector = DiagnosticsCollector::new(10);

        // Record some errors
        collector
            .record_error(
                &UrpoError::network("test error"),
                Some("test_operation".to_string()),
                false,
            )
            .await;

        collector
            .record_error(&UrpoError::config("config error"), None, true)
            .await;

        // Check stats
        let stats = collector.get_stats().await;
        assert_eq!(stats.by_category.get("network"), Some(&1));
        assert_eq!(stats.by_category.get("config"), Some(&1));
        assert_eq!(stats.recent_errors.len(), 2);
    }

    #[test]
    fn test_format_user_error() {
        let error = UrpoError::config("Invalid port number");
        let formatted = format_user_error(&error);

        assert!(formatted.contains("Configuration issue"));
        assert!(formatted.contains("check-config"));
    }

    #[tokio::test]
    async fn test_health_checker() {
        let mut checker = HealthChecker::new();

        // Add a port check for an available port
        checker.add_check(Box::new(PortCheck::new("test_port", 0)));

        let results = checker.check_all().await;
        assert_eq!(results.len(), 1);

        // Port 0 should bind to any available port
        assert!(results[0].1.healthy);
    }
}

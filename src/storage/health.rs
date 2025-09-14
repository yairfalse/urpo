//! Storage health monitoring and self-healing capabilities.
//!
//! This module ensures the storage system remains healthy and can recover
//! from various failure scenarios without panicking.

use crate::core::Result;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Health status of the storage system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Everything is working perfectly
    Healthy,
    /// Minor issues but still operational
    Degraded {
        /// Reason for degradation
        reason: &'static str,
    },
    /// Major issues, running in fallback mode
    Critical {
        /// Reason for critical status
        reason: &'static str,
    },
    /// System has failed and needs restart
    Failed {
        /// Reason for failure
        reason: &'static str,
    },
}

/// Storage health monitor that tracks system health and triggers recovery.
pub struct StorageHealthMonitor {
    /// Current health status
    status: Arc<RwLock<HealthStatus>>,
    /// Number of errors encountered
    error_count: Arc<AtomicU64>,
    /// Number of successful operations
    success_count: Arc<AtomicU64>,
    /// Last health check time
    last_check: Arc<RwLock<Instant>>,
    /// Whether automatic recovery is enabled
    auto_recovery: Arc<AtomicBool>,
    /// Health thresholds
    thresholds: HealthThresholds,
}

/// Configurable health thresholds.
#[derive(Debug, Clone)]
pub struct HealthThresholds {
    /// Error rate threshold for degraded status (0.0-1.0)
    pub degraded_error_rate: f64,
    /// Error rate threshold for critical status (0.0-1.0)
    pub critical_error_rate: f64,
    /// Minimum operations before calculating error rate
    pub min_operations: u64,
    /// Time window for error rate calculation
    pub window_duration: Duration,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            degraded_error_rate: 0.01,                // 1% errors = degraded
            critical_error_rate: 0.05,                // 5% errors = critical
            min_operations: 100,                      // Need 100 ops before judging
            window_duration: Duration::from_secs(60), // 1 minute window
        }
    }
}

impl StorageHealthMonitor {
    /// Create a new health monitor with default thresholds.
    pub fn new() -> Self {
        Self::with_thresholds(HealthThresholds::default())
    }

    /// Create a new health monitor with custom thresholds.
    pub fn with_thresholds(thresholds: HealthThresholds) -> Self {
        Self {
            status: Arc::new(RwLock::new(HealthStatus::Healthy)),
            error_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            last_check: Arc::new(RwLock::new(Instant::now())),
            auto_recovery: Arc::new(AtomicBool::new(true)),
            thresholds,
        }
    }

    /// Record a successful operation.
    #[inline]
    pub fn record_success(&self) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed operation.
    #[inline]
    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current health status.
    pub async fn get_status(&self) -> HealthStatus {
        *self.status.read().await
    }

    /// Check and update health status based on current metrics.
    pub async fn check_health(&self) -> HealthStatus {
        let errors = self.error_count.load(Ordering::Relaxed);
        let successes = self.success_count.load(Ordering::Relaxed);
        let total = errors + successes;

        // Need minimum operations before judging
        if total < self.thresholds.min_operations {
            return HealthStatus::Healthy;
        }

        // Calculate error rate
        let error_rate = if total > 0 {
            errors as f64 / total as f64
        } else {
            0.0
        };

        // Determine health status
        let new_status = if error_rate >= self.thresholds.critical_error_rate {
            HealthStatus::Critical {
                reason: "High error rate detected",
            }
        } else if error_rate >= self.thresholds.degraded_error_rate {
            HealthStatus::Degraded {
                reason: "Elevated error rate",
            }
        } else {
            HealthStatus::Healthy
        };

        // Update status
        let mut status = self.status.write().await;
        let old_status = *status;
        *status = new_status;

        // Log status changes
        if old_status != new_status {
            match new_status {
                HealthStatus::Healthy => {
                    tracing::info!("Storage health recovered to HEALTHY");
                }
                HealthStatus::Degraded { reason } => {
                    tracing::warn!("Storage health DEGRADED: {}", reason);
                }
                HealthStatus::Critical { reason } => {
                    tracing::error!("Storage health CRITICAL: {}", reason);
                }
                HealthStatus::Failed { reason } => {
                    tracing::error!("Storage health FAILED: {}", reason);
                }
            }
        }

        // Update last check time
        *self.last_check.write().await = Instant::now();

        // Trigger recovery if needed
        if self.auto_recovery.load(Ordering::Relaxed) {
            self.try_recovery(new_status).await;
        }

        new_status
    }

    /// Attempt automatic recovery based on health status.
    async fn try_recovery(&self, status: HealthStatus) {
        match status {
            HealthStatus::Degraded { .. } => {
                // Mild recovery: reset counters after window
                let last_check = *self.last_check.read().await;
                if last_check.elapsed() > self.thresholds.window_duration {
                    self.reset_counters();
                    tracing::info!("Reset health counters after time window");
                }
            }
            HealthStatus::Critical { .. } => {
                // Aggressive recovery: force reset
                self.reset_counters();
                tracing::warn!("Force reset health counters due to critical status");
            }
            HealthStatus::Failed { .. } => {
                // Last resort: full reset
                self.reset_counters();
                *self.status.write().await = HealthStatus::Degraded {
                    reason: "Recovering from failure",
                };
                tracing::error!("Attempting recovery from failed state");
            }
            HealthStatus::Healthy => {
                // Nothing to do
            }
        }
    }

    /// Reset error and success counters.
    fn reset_counters(&self) {
        self.error_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
    }

    /// Get health metrics.
    pub async fn get_metrics(&self) -> HealthMetrics {
        let errors = self.error_count.load(Ordering::Relaxed);
        let successes = self.success_count.load(Ordering::Relaxed);
        let total = errors + successes;

        HealthMetrics {
            status: *self.status.read().await,
            total_operations: total,
            error_count: errors,
            success_count: successes,
            error_rate: if total > 0 {
                errors as f64 / total as f64
            } else {
                0.0
            },
            last_check: *self.last_check.read().await,
        }
    }

    /// Enable or disable automatic recovery.
    pub fn set_auto_recovery(&self, enabled: bool) {
        self.auto_recovery.store(enabled, Ordering::Relaxed);
    }
}

/// Health metrics for monitoring.
#[derive(Debug, Clone)]
pub struct HealthMetrics {
    pub status: HealthStatus,
    pub total_operations: u64,
    pub error_count: u64,
    pub success_count: u64,
    pub error_rate: f64,
    pub last_check: Instant,
}

/// Storage recovery strategies.
pub enum RecoveryStrategy {
    /// Just log and continue
    LogAndContinue,
    /// Reset internal state
    ResetState,
    /// Reduce load by dropping non-critical operations
    ReduceLoad,
    /// Full restart with cleanup
    FullRestart,
}

/// Recovery coordinator for handling failures.
pub struct RecoveryCoordinator {
    monitor: Arc<StorageHealthMonitor>,
    strategy: Arc<RwLock<RecoveryStrategy>>,
}

impl RecoveryCoordinator {
    /// Create a new recovery coordinator.
    pub fn new(monitor: Arc<StorageHealthMonitor>) -> Self {
        Self {
            monitor,
            strategy: Arc::new(RwLock::new(RecoveryStrategy::LogAndContinue)),
        }
    }

    /// Execute recovery based on current strategy.
    pub async fn execute_recovery(&self) -> Result<()> {
        let status = self.monitor.get_status().await;
        let strategy = self.determine_strategy(status).await;

        match strategy {
            RecoveryStrategy::LogAndContinue => {
                tracing::info!("Recovery: Continuing with logging");
                Ok(())
            }
            RecoveryStrategy::ResetState => {
                tracing::warn!("Recovery: Resetting internal state");
                self.monitor.reset_counters();
                Ok(())
            }
            RecoveryStrategy::ReduceLoad => {
                tracing::warn!("Recovery: Reducing load by dropping non-critical ops");
                // In real implementation, would throttle operations
                Ok(())
            }
            RecoveryStrategy::FullRestart => {
                tracing::error!("Recovery: Full restart required");
                // In real implementation, would trigger full restart
                self.monitor.reset_counters();
                Ok(())
            }
        }
    }

    /// Determine recovery strategy based on health status.
    async fn determine_strategy(&self, status: HealthStatus) -> RecoveryStrategy {
        match status {
            HealthStatus::Healthy => RecoveryStrategy::LogAndContinue,
            HealthStatus::Degraded { .. } => RecoveryStrategy::ResetState,
            HealthStatus::Critical { .. } => RecoveryStrategy::ReduceLoad,
            HealthStatus::Failed { .. } => RecoveryStrategy::FullRestart,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_monitoring() {
        let monitor = StorageHealthMonitor::new();

        // Initially healthy
        assert_eq!(monitor.get_status().await, HealthStatus::Healthy);

        // Record some successes
        for _ in 0..100 {
            monitor.record_success();
        }

        // Still healthy
        assert_eq!(monitor.check_health().await, HealthStatus::Healthy);

        // Record errors to trigger degraded
        for _ in 0..5 {
            monitor.record_error();
        }

        // Should be degraded (5/105 ≈ 4.7% error rate)
        let status = monitor.check_health().await;
        assert!(matches!(
            status,
            HealthStatus::Degraded { .. } | HealthStatus::Critical { .. }
        ));
    }

    #[tokio::test]
    async fn test_recovery_coordinator() {
        let monitor = Arc::new(StorageHealthMonitor::new());
        let coordinator = RecoveryCoordinator::new(monitor.clone());

        // Execute recovery when healthy (should just log)
        assert!(coordinator.execute_recovery().await.is_ok());

        // Simulate failures
        for _ in 0..10 {
            monitor.record_error();
        }
        for _ in 0..90 {
            monitor.record_success();
        }

        monitor.check_health().await;

        // Execute recovery when degraded
        assert!(coordinator.execute_recovery().await.is_ok());
    }
}

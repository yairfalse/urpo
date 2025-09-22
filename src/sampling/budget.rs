//! Budget-aware sampling that respects storage limits
//!
//! PERFORMANCE: Zero-allocation decisions with atomic counters

use super::SamplingPriority;
use std::sync::atomic::{AtomicU64, Ordering};

/// Budget-aware sampler that manages storage capacity
pub struct BudgetAwareSampler {
    /// Total storage budget in bytes
    budget_bytes: AtomicU64,
    /// Current storage usage in bytes
    used_bytes: AtomicU64,
    /// Reserved space for critical traces (20% of budget)
    reserved_bytes: AtomicU64,
    /// Average trace size in bytes (adaptive)
    avg_trace_size: AtomicU64,
}

impl BudgetAwareSampler {
    /// Create new budget-aware sampler
    pub fn new(storage_budget_gb: u64) -> Self {
        let budget_bytes = storage_budget_gb * 1_000_000_000;
        let reserved = budget_bytes / 5; // Reserve 20% for critical

        Self {
            budget_bytes: AtomicU64::new(budget_bytes),
            used_bytes: AtomicU64::new(0),
            reserved_bytes: AtomicU64::new(reserved),
            avg_trace_size: AtomicU64::new(10_000), // Start with 10KB estimate
        }
    }

    /// Check if we have capacity for more traces
    #[inline(always)]
    pub async fn has_capacity(&self) -> bool {
        let used = self.used_bytes.load(Ordering::Relaxed);
        let budget = self.budget_bytes.load(Ordering::Relaxed);
        let reserved = self.reserved_bytes.load(Ordering::Relaxed);

        used < (budget - reserved)
    }

    /// Check if we can store a trace of given priority
    #[inline(always)]
    pub fn can_store(&self, priority: SamplingPriority, estimated_size: u64) -> bool {
        let used = self.used_bytes.load(Ordering::Relaxed);
        let budget = self.budget_bytes.load(Ordering::Relaxed);
        let _reserved = self.reserved_bytes.load(Ordering::Relaxed);

        match priority {
            SamplingPriority::Critical => {
                // Critical can use full budget
                used + estimated_size < budget
            },
            SamplingPriority::High => {
                // High priority can use 80% of budget
                used + estimated_size < (budget * 4 / 5)
            },
            SamplingPriority::Medium => {
                // Medium can use 60% of budget
                used + estimated_size < (budget * 3 / 5)
            },
            SamplingPriority::Low => {
                // Low can use 40% of budget
                used + estimated_size < (budget * 2 / 5)
            },
            SamplingPriority::Minimal => {
                // Minimal only if under 20% usage
                used + estimated_size < (budget / 5)
            },
        }
    }

    /// Update storage usage
    pub async fn update_usage(&self, used_gb: u64) {
        let used_bytes = used_gb * 1_000_000_000;
        self.used_bytes.store(used_bytes, Ordering::Relaxed);
    }

    /// Record trace storage
    pub fn record_trace(&self, size_bytes: u64) {
        self.used_bytes.fetch_add(size_bytes, Ordering::Relaxed);

        // Update average (exponential moving average)
        let current_avg = self.avg_trace_size.load(Ordering::Relaxed);
        let new_avg = (current_avg * 9 + size_bytes) / 10;
        self.avg_trace_size.store(new_avg, Ordering::Relaxed);
    }

    /// Get capacity statistics
    pub fn get_stats(&self) -> BudgetStats {
        let budget = self.budget_bytes.load(Ordering::Relaxed);
        let used = self.used_bytes.load(Ordering::Relaxed);
        let reserved = self.reserved_bytes.load(Ordering::Relaxed);
        let avg_size = self.avg_trace_size.load(Ordering::Relaxed);

        BudgetStats {
            budget_gb: budget / 1_000_000_000,
            used_gb: used / 1_000_000_000,
            available_gb: (budget - used) / 1_000_000_000,
            reserved_gb: reserved / 1_000_000_000,
            usage_percent: (used as f64 / budget as f64) * 100.0,
            avg_trace_kb: avg_size / 1000,
            estimated_traces_remaining: (budget - used) / avg_size.max(1),
        }
    }

    /// Emergency cleanup - calculate how much to drop
    pub fn calculate_cleanup_target(&self) -> u64 {
        let used = self.used_bytes.load(Ordering::Relaxed);
        let budget = self.budget_bytes.load(Ordering::Relaxed);

        if used > budget * 9 / 10 {
            // Over 90%, free 20%
            budget / 5
        } else if used > budget * 8 / 10 {
            // Over 80%, free 10%
            budget / 10
        } else {
            0
        }
    }
}

/// Budget statistics
#[derive(Debug, Clone)]
pub struct BudgetStats {
    pub budget_gb: u64,
    pub used_gb: u64,
    pub available_gb: u64,
    pub reserved_gb: u64,
    pub usage_percent: f64,
    pub avg_trace_kb: u64,
    pub estimated_traces_remaining: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_budget_capacity() {
        let sampler = BudgetAwareSampler::new(100); // 100GB budget

        assert!(sampler.has_capacity().await);

        // Use 90GB
        sampler.update_usage(90).await;

        // Should still have capacity (under 80% non-reserved)
        assert!(!sampler.has_capacity().await);
    }

    #[tokio::test]
    async fn test_priority_based_storage() {
        let sampler = BudgetAwareSampler::new(100);

        // Use 50GB
        sampler.update_usage(50).await;

        // Critical should still work
        assert!(sampler.can_store(SamplingPriority::Critical, 1_000_000_000));

        // Low priority should not (over 40%)
        assert!(!sampler.can_store(SamplingPriority::Low, 1_000_000_000));

        // Minimal should definitely not
        assert!(!sampler.can_store(SamplingPriority::Minimal, 1_000_000_000));
    }

    #[test]
    fn test_average_trace_size() {
        let sampler = BudgetAwareSampler::new(100);

        // Record some traces
        sampler.record_trace(5_000);
        sampler.record_trace(15_000);
        sampler.record_trace(10_000);

        let stats = sampler.get_stats();
        // Should converge towards recent values
        assert!(stats.avg_trace_kb > 5 && stats.avg_trace_kb < 15);
    }

    #[tokio::test]
    async fn test_cleanup_calculation() {
        let sampler = BudgetAwareSampler::new(100);

        // No cleanup needed at low usage
        sampler.update_usage(50).await;
        assert_eq!(sampler.calculate_cleanup_target(), 0);

        // Cleanup at 85%
        sampler.update_usage(85).await;
        assert!(sampler.calculate_cleanup_target() > 0);

        // More aggressive at 95%
        sampler.update_usage(95).await;
        assert!(sampler.calculate_cleanup_target() >= 10_000_000_000);
    }
}

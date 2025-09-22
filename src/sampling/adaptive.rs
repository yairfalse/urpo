//! Adaptive sampling that adjusts rates based on system load
//!
//! PERFORMANCE: Lock-free rate adjustment using atomics

use super::{SamplingPriority, SystemMetrics, TraceCharacteristics};
use std::sync::atomic::{AtomicU64, Ordering};

/// Adaptive sampler that dynamically adjusts sampling rates
pub struct AdaptiveSampler {
    /// Current sampling rate (0-10000 representing 0.00% - 100.00%)
    sampling_rate: AtomicU64,
    /// Target traces per second
    target_tps: AtomicU64,
    /// Error trace boost factor
    error_boost: AtomicU64,
}

impl AdaptiveSampler {
    /// Create new adaptive sampler
    pub fn new() -> Self {
        Self {
            sampling_rate: AtomicU64::new(100), // Start at 1%
            target_tps: AtomicU64::new(1000),   // Target 1000 traces/sec
            error_boost: AtomicU64::new(10),    // 10x boost for errors
        }
    }

    /// Fast sampling decision for head sampling (<10ns with atomics)
    #[inline(always)]
    pub fn should_sample_fast(&self, hash: u64) -> bool {
        let rate = self.sampling_rate.load(Ordering::Relaxed);
        let threshold = (u64::MAX / 10000) * rate;
        hash < threshold
    }

    /// Tail-based sampling with full trace context
    pub async fn should_sample_tail(&self, characteristics: &TraceCharacteristics) -> bool {
        match characteristics.priority {
            SamplingPriority::Critical => true,
            SamplingPriority::High => {
                // Sample 50% of high priority
                fast_hash_decision(&characteristics.trace_id, 5000)
            },
            SamplingPriority::Medium => {
                // Sample 10% of medium priority
                fast_hash_decision(&characteristics.trace_id, 1000)
            },
            SamplingPriority::Low => {
                // Sample 1% of low priority
                fast_hash_decision(&characteristics.trace_id, 100)
            },
            SamplingPriority::Minimal => false,
        }
    }

    /// Adjust sampling rate based on system metrics
    pub async fn adjust_rate(&self, metrics: &SystemMetrics) {
        let current_tps = metrics.traces_per_second;
        let target = self.target_tps.load(Ordering::Relaxed) as f64;

        // Calculate new rate based on load
        let ratio = target / current_tps.max(1.0);
        let new_rate = if ratio > 1.0 {
            // Under target, increase sampling
            (self.sampling_rate.load(Ordering::Relaxed) as f64 * ratio.min(1.5)) as u64
        } else {
            // Over target, decrease sampling
            (self.sampling_rate.load(Ordering::Relaxed) as f64 * ratio.max(0.5)) as u64
        };

        // Clamp between 0.01% and 100%
        let clamped = new_rate.clamp(1, 10000);
        self.sampling_rate.store(clamped, Ordering::Relaxed);

        // Boost rate if error rate is high
        if metrics.error_rate > 0.05 {
            let boosted = clamped.saturating_mul(self.error_boost.load(Ordering::Relaxed));
            self.sampling_rate
                .store(boosted.min(10000), Ordering::Relaxed);
        }
    }

    /// Get current sampling rate as percentage
    pub fn get_rate_percent(&self) -> f64 {
        self.sampling_rate.load(Ordering::Relaxed) as f64 / 100.0
    }
}

impl Default for AdaptiveSampler {
    fn default() -> Self {
        Self::new()
    }
}

/// Fast hash-based decision
#[inline(always)]
fn fast_hash_decision(trace_id: &crate::core::TraceId, rate_per_10000: u64) -> bool {
    use rustc_hash::FxHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = FxHasher::default();
    trace_id.hash(&mut hasher);
    let hash = hasher.finish();

    let threshold = (u64::MAX / 10000) * rate_per_10000;
    hash < threshold
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TraceId;

    #[tokio::test]
    async fn test_adaptive_rate_adjustment() {
        let sampler = AdaptiveSampler::new();

        // Initially 1%
        assert_eq!(sampler.get_rate_percent(), 1.0);

        // Adjust with low load
        let metrics = SystemMetrics {
            traces_per_second: 100.0,
            error_rate: 0.01,
            storage_used_gb: 10,
            storage_total_gb: 100,
            cpu_usage: 0.3,
            memory_usage: 0.4,
        };

        sampler.adjust_rate(&metrics).await;

        // Should increase rate when under target
        assert!(sampler.get_rate_percent() > 1.0);
    }

    #[tokio::test]
    async fn test_error_boost() {
        let sampler = AdaptiveSampler::new();

        // High error rate
        let metrics = SystemMetrics {
            traces_per_second: 1000.0,
            error_rate: 0.1, // 10% errors
            storage_used_gb: 10,
            storage_total_gb: 100,
            cpu_usage: 0.3,
            memory_usage: 0.4,
        };

        sampler.adjust_rate(&metrics).await;

        // Should boost sampling rate
        assert!(sampler.get_rate_percent() > 1.0);
    }
}

//! Smart trace sampling system for production-grade observability
//!
//! PERFORMANCE TARGETS:
//! - <100ns sampling decision (head sampling)
//! - <1μs tail-based evaluation
//! - 90% storage reduction while keeping 100% critical traces

pub mod adaptive;
pub mod tail_based;
pub mod budget;
pub mod pattern;

pub use adaptive::AdaptiveSampler;
pub use tail_based::TailBasedSampler;
pub use budget::BudgetAwareSampler;
pub use pattern::PatternDetector;

use crate::core::{Result, SpanStatus, TraceId};
use std::sync::Arc;
use std::time::Duration;

/// Sampling decision result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingDecision {
    /// Keep this trace
    Keep,
    /// Drop this trace
    Drop,
    /// Defer decision until trace completes (tail-based)
    Defer,
}

/// Sampling priority for intelligent retention
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SamplingPriority {
    /// Must keep (errors, critical paths)
    Critical = 0,
    /// Should keep (slow requests, anomalies)
    High = 1,
    /// Nice to keep (interesting patterns)
    Medium = 2,
    /// Keep if space available
    Low = 3,
    /// Drop unless explicitly needed
    Minimal = 4,
}

/// Trace characteristics for sampling decisions
#[derive(Debug, Clone)]
pub struct TraceCharacteristics {
    pub trace_id: TraceId,
    pub has_error: bool,
    pub duration_ms: Option<u64>,
    pub span_count: usize,
    pub service_count: usize,
    pub is_anomalous: bool,
    pub priority: SamplingPriority,
}

/// Smart sampler combining multiple strategies
pub struct SmartSampler {
    /// Adaptive sampler for dynamic rate adjustment
    adaptive: Arc<AdaptiveSampler>,
    /// Tail-based sampler for complete trace analysis
    tail_based: Arc<TailBasedSampler>,
    /// Budget-aware sampler for storage management
    budget: Arc<BudgetAwareSampler>,
    /// Pattern detector for anomaly detection
    pattern: Arc<PatternDetector>,
}

impl SmartSampler {
    /// Create new smart sampler with default configuration
    pub fn new(storage_budget_gb: u64) -> Self {
        Self {
            adaptive: Arc::new(AdaptiveSampler::new()),
            tail_based: Arc::new(TailBasedSampler::new()),
            budget: Arc::new(BudgetAwareSampler::new(storage_budget_gb)),
            pattern: Arc::new(PatternDetector::new()),
        }
    }

    /// Make head sampling decision (fast path <100ns)
    #[inline(always)]
    pub fn should_sample_head(&self, trace_id: &TraceId) -> SamplingDecision {
        // Fast hash-based probabilistic sampling
        let hash = fast_hash(trace_id);
        
        // Always sample if under 1% baseline
        if hash < u64::MAX / 100 {
            return SamplingDecision::Keep;
        }

        // Check adaptive rate
        if self.adaptive.should_sample_fast(hash) {
            return SamplingDecision::Defer; // Let tail-based decide
        }

        SamplingDecision::Drop
    }

    /// Make tail-based sampling decision (complete trace available)
    pub async fn should_sample_tail(&self, characteristics: &TraceCharacteristics) -> SamplingDecision {
        // Priority 1: Always keep errors
        if characteristics.has_error {
            return SamplingDecision::Keep;
        }

        // Priority 2: Keep slow requests (>1s)
        if let Some(duration) = characteristics.duration_ms {
            if duration > 1000 {
                return SamplingDecision::Keep;
            }
        }

        // Priority 3: Anomaly detection
        if self.pattern.is_anomalous(characteristics).await {
            return SamplingDecision::Keep;
        }

        // Priority 4: Budget check
        if !self.budget.has_capacity().await {
            return SamplingDecision::Drop;
        }

        // Priority 5: Adaptive sampling rate
        if self.adaptive.should_sample_tail(characteristics).await {
            return SamplingDecision::Keep;
        }

        SamplingDecision::Drop
    }

    /// Update sampling rates based on system load
    pub async fn adjust_rates(&self, metrics: &SystemMetrics) {
        self.adaptive.adjust_rate(metrics).await;
        self.budget.update_usage(metrics.storage_used_gb).await;
    }
}

/// System metrics for adaptive sampling
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub traces_per_second: f64,
    pub error_rate: f64,
    pub storage_used_gb: u64,
    pub storage_total_gb: u64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
}

/// Fast non-cryptographic hash for sampling decisions
#[inline(always)]
fn fast_hash(trace_id: &TraceId) -> u64 {
    // Use FxHash for speed (not cryptographic)
    use rustc_hash::FxHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = FxHasher::default();
    trace_id.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_head_sampling_speed() {
        let sampler = SmartSampler::new(100);
        let trace_id = TraceId::new("test123".to_string()).unwrap();
        
        let start = std::time::Instant::now();
        for _ in 0..1_000_000 {
            let _ = sampler.should_sample_head(&trace_id);
        }
        let elapsed = start.elapsed();
        
        let ns_per_decision = elapsed.as_nanos() / 1_000_000;
        assert!(ns_per_decision < 100, "Head sampling too slow: {}ns", ns_per_decision);
    }

    #[tokio::test]
    async fn test_error_traces_always_kept() {
        let sampler = SmartSampler::new(100);
        
        let characteristics = TraceCharacteristics {
            trace_id: TraceId::new("error_trace".to_string()).unwrap(),
            has_error: true,
            duration_ms: Some(100),
            span_count: 10,
            service_count: 3,
            is_anomalous: false,
            priority: SamplingPriority::Critical,
        };
        
        let decision = sampler.should_sample_tail(&characteristics).await;
        assert_eq!(decision, SamplingDecision::Keep);
    }
}
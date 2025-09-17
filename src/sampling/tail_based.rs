//! Tail-based sampling that makes decisions after seeing complete traces
//!
//! PERFORMANCE: Deferred decisions with bounded memory usage

use super::{SamplingDecision, TraceCharacteristics};
use crate::core::{Result, TraceId};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Tail-based sampler that evaluates complete traces
pub struct TailBasedSampler {
    /// Pending traces awaiting decision
    pending: DashMap<TraceId, PendingTrace>,
    /// Maximum time to wait for trace completion
    max_wait: Duration,
    /// Statistics
    total_evaluated: AtomicU64,
    total_kept: AtomicU64,
    /// Memory limit for pending traces
    max_pending: AtomicUsize,
}

/// Trace pending tail-based decision
struct PendingTrace {
    start_time: Instant,
    span_count: AtomicUsize,
    has_error: AtomicU64, // 0 or 1, using u64 for atomic
    max_duration_ns: AtomicU64,
    services_seen: DashMap<String, ()>,
}

impl TailBasedSampler {
    /// Create new tail-based sampler
    pub fn new() -> Self {
        Self {
            pending: DashMap::new(),
            max_wait: Duration::from_secs(30),
            total_evaluated: AtomicU64::new(0),
            total_kept: AtomicU64::new(0),
            max_pending: AtomicUsize::new(10_000),
        }
    }

    /// Register a span for tail-based evaluation
    pub fn register_span(&self, trace_id: TraceId, has_error: bool, duration_ns: u64, service: String) {
        // Enforce memory limit
        if self.pending.len() >= self.max_pending.load(Ordering::Relaxed) {
            self.cleanup_old_traces();
        }

        let pending = self.pending.entry(trace_id).or_insert_with(|| PendingTrace {
            start_time: Instant::now(),
            span_count: AtomicUsize::new(0),
            has_error: AtomicU64::new(0),
            max_duration_ns: AtomicU64::new(0),
            services_seen: DashMap::new(),
        });

        pending.span_count.fetch_add(1, Ordering::Relaxed);
        if has_error {
            pending.has_error.store(1, Ordering::Relaxed);
        }
        pending.max_duration_ns.fetch_max(duration_ns, Ordering::Relaxed);
        pending.services_seen.insert(service, ());
    }

    /// Evaluate if trace should be kept
    pub fn evaluate(&self, trace_id: &TraceId) -> SamplingDecision {
        if let Some((_, pending)) = self.pending.remove(trace_id) {
            self.total_evaluated.fetch_add(1, Ordering::Relaxed);

            // Decision criteria (in priority order)
            let decision = if pending.has_error.load(Ordering::Relaxed) == 1 {
                // Always keep errors
                SamplingDecision::Keep
            } else if pending.max_duration_ns.load(Ordering::Relaxed) > 1_000_000_000 {
                // Keep slow traces (>1s)
                SamplingDecision::Keep
            } else if pending.span_count.load(Ordering::Relaxed) > 100 {
                // Keep complex traces
                SamplingDecision::Keep
            } else if pending.services_seen.len() > 5 {
                // Keep traces crossing many services
                SamplingDecision::Keep
            } else {
                // Apply probabilistic sampling for normal traces
                if fast_probability(trace_id, 100) {
                    // Keep 1% of normal traces
                    SamplingDecision::Keep
                } else {
                    SamplingDecision::Drop
                }
            };

            if decision == SamplingDecision::Keep {
                self.total_kept.fetch_add(1, Ordering::Relaxed);
            }

            decision
        } else {
            // Not in pending, apply default
            SamplingDecision::Drop
        }
    }

    /// Cleanup old pending traces
    fn cleanup_old_traces(&self) {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for entry in self.pending.iter() {
            if now.duration_since(entry.value().start_time) > self.max_wait {
                to_remove.push(entry.key().clone());
            }
        }

        for trace_id in to_remove {
            self.pending.remove(&trace_id);
        }
    }

    /// Get sampling statistics
    pub fn get_stats(&self) -> TailSamplingStats {
        let total = self.total_evaluated.load(Ordering::Relaxed);
        let kept = self.total_kept.load(Ordering::Relaxed);
        
        TailSamplingStats {
            total_evaluated: total,
            total_kept: kept,
            keep_ratio: if total > 0 { kept as f64 / total as f64 } else { 0.0 },
            pending_count: self.pending.len(),
        }
    }
}

impl Default for TailBasedSampler {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for tail-based sampling
#[derive(Debug, Clone)]
pub struct TailSamplingStats {
    pub total_evaluated: u64,
    pub total_kept: u64,
    pub keep_ratio: f64,
    pub pending_count: usize,
}

/// Fast probability check using trace ID hash
#[inline(always)]
fn fast_probability(trace_id: &TraceId, per_10000: u64) -> bool {
    use rustc_hash::FxHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = FxHasher::default();
    trace_id.hash(&mut hasher);
    let hash = hasher.finish();
    
    let threshold = (u64::MAX / 10000) * per_10000;
    hash < threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_traces_kept() {
        let sampler = TailBasedSampler::new();
        let trace_id = TraceId::new("error_trace".to_string()).unwrap();
        
        sampler.register_span(
            trace_id.clone(),
            true, // has error
            100_000_000,
            "service1".to_string(),
        );
        
        let decision = sampler.evaluate(&trace_id);
        assert_eq!(decision, SamplingDecision::Keep);
    }

    #[test]
    fn test_slow_traces_kept() {
        let sampler = TailBasedSampler::new();
        let trace_id = TraceId::new("slow_trace".to_string()).unwrap();
        
        sampler.register_span(
            trace_id.clone(),
            false,
            2_000_000_000, // 2 seconds
            "service1".to_string(),
        );
        
        let decision = sampler.evaluate(&trace_id);
        assert_eq!(decision, SamplingDecision::Keep);
    }

    #[test]
    fn test_complex_traces_kept() {
        let sampler = TailBasedSampler::new();
        let trace_id = TraceId::new("complex_trace".to_string()).unwrap();
        
        // Register many spans
        for i in 0..150 {
            sampler.register_span(
                trace_id.clone(),
                false,
                10_000_000,
                format!("service{}", i % 3),
            );
        }
        
        let decision = sampler.evaluate(&trace_id);
        assert_eq!(decision, SamplingDecision::Keep);
    }

    #[test]
    fn test_normal_traces_sampled() {
        let sampler = TailBasedSampler::new();
        let mut kept = 0;
        let mut dropped = 0;
        
        for i in 0..1000 {
            let trace_id = TraceId::new(format!("normal_{}", i)).unwrap();
            sampler.register_span(
                trace_id.clone(),
                false,
                10_000_000,
                "service1".to_string(),
            );
            
            match sampler.evaluate(&trace_id) {
                SamplingDecision::Keep => kept += 1,
                SamplingDecision::Drop => dropped += 1,
                _ => {}
            }
        }
        
        // Should keep roughly 1%
        assert!(kept > 0 && kept < 50);
        assert!(dropped > 950);
    }
}
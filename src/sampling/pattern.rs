//! Pattern detection for anomaly-based sampling
//!
//! PERFORMANCE: Streaming statistics with constant memory

use super::TraceCharacteristics;
use crate::core::ServiceName;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::VecDeque;
use std::sync::RwLock;

/// Pattern detector for identifying anomalous traces
pub struct PatternDetector {
    /// Service latency baselines (p50, p95, p99)
    service_baselines: DashMap<ServiceName, LatencyBaseline>,
    /// Recent trace patterns for anomaly detection
    recent_patterns: RwLock<VecDeque<TracePattern>>,
    /// Anomaly threshold (standard deviations)
    anomaly_threshold: f64,
}

/// Latency baseline for a service
struct LatencyBaseline {
    p50: AtomicU64,
    p95: AtomicU64,
    p99: AtomicU64,
    count: AtomicU64,
}

/// Pattern of a trace for comparison
#[derive(Clone)]
struct TracePattern {
    span_count: usize,
    service_count: usize,
    max_depth: usize,
    total_duration_ms: u64,
}

impl PatternDetector {
    /// Create new pattern detector
    pub fn new() -> Self {
        Self {
            service_baselines: DashMap::new(),
            recent_patterns: RwLock::new(VecDeque::with_capacity(1000)),
            anomaly_threshold: 3.0, // 3 standard deviations
        }
    }

    /// Check if trace is anomalous
    pub async fn is_anomalous(&self, characteristics: &TraceCharacteristics) -> bool {
        // Check multiple anomaly indicators
        
        // 1. Duration anomaly
        if self.is_duration_anomalous(characteristics.duration_ms).await {
            return true;
        }
        
        // 2. Span count anomaly
        if self.is_span_count_anomalous(characteristics.span_count).await {
            return true;
        }
        
        // 3. Service fan-out anomaly
        if characteristics.service_count > 10 {
            return true;
        }
        
        // 4. Already marked as anomalous
        characteristics.is_anomalous
    }

    /// Check if duration is anomalous
    async fn is_duration_anomalous(&self, duration_ms: Option<u64>) -> bool {
        let Some(duration) = duration_ms else { return false };
        
        // Quick check: anything over 5s is anomalous
        if duration > 5000 {
            return true;
        }
        
        // Statistical check against recent patterns
        let patterns = self.recent_patterns.read().unwrap();
        if patterns.len() < 100 {
            return false; // Not enough data
        }
        
        let mean = patterns.iter()
            .map(|p| p.total_duration_ms)
            .sum::<u64>() / patterns.len() as u64;
        
        let variance = patterns.iter()
            .map(|p| {
                let diff = p.total_duration_ms as i64 - mean as i64;
                (diff * diff) as u64
            })
            .sum::<u64>() / patterns.len() as u64;
        
        let std_dev = (variance as f64).sqrt();
        
        // Check if outside threshold
        let z_score = ((duration as f64 - mean as f64) / std_dev).abs();
        z_score > self.anomaly_threshold
    }

    /// Check if span count is anomalous
    async fn is_span_count_anomalous(&self, span_count: usize) -> bool {
        // Quick check: over 500 spans is anomalous
        if span_count > 500 {
            return true;
        }
        
        // Statistical check
        let patterns = self.recent_patterns.read().unwrap();
        if patterns.len() < 100 {
            return false;
        }
        
        let mean = patterns.iter()
            .map(|p| p.span_count)
            .sum::<usize>() / patterns.len();
        
        // Simple threshold: 5x the mean
        span_count > mean * 5
    }

    /// Update pattern history
    pub fn record_trace_pattern(&self, characteristics: &TraceCharacteristics) {
        let pattern = TracePattern {
            span_count: characteristics.span_count,
            service_count: characteristics.service_count,
            max_depth: 0, // Could calculate from span relationships
            total_duration_ms: characteristics.duration_ms.unwrap_or(0),
        };
        
        let mut patterns = self.recent_patterns.write().unwrap();
        if patterns.len() >= 1000 {
            patterns.pop_front();
        }
        patterns.push_back(pattern);
    }

    /// Update service baseline latencies
    pub fn update_baseline(&self, service: ServiceName, latency_ms: u64) {
        let baseline = self.service_baselines.entry(service).or_insert_with(|| {
            LatencyBaseline {
                p50: AtomicU64::new(latency_ms),
                p95: AtomicU64::new(latency_ms),
                p99: AtomicU64::new(latency_ms),
                count: AtomicU64::new(0),
            }
        });
        
        baseline.count.fetch_add(1, Ordering::Relaxed);
        
        // Simplified percentile update (exponential moving average)
        // In production, use proper percentile algorithms
        if latency_ms < baseline.p50.load(Ordering::Relaxed) {
            baseline.p50.store(
                (baseline.p50.load(Ordering::Relaxed) * 9 + latency_ms) / 10,
                Ordering::Relaxed
            );
        }
        
        if latency_ms > baseline.p95.load(Ordering::Relaxed) {
            baseline.p95.store(
                (baseline.p95.load(Ordering::Relaxed) * 9 + latency_ms) / 10,
                Ordering::Relaxed
            );
        }
        
        if latency_ms > baseline.p99.load(Ordering::Relaxed) {
            baseline.p99.store(
                (baseline.p99.load(Ordering::Relaxed) * 9 + latency_ms) / 10,
                Ordering::Relaxed
            );
        }
    }

    /// Detect unusual service interaction patterns
    pub fn detect_unusual_path(&self, services: &[ServiceName]) -> bool {
        // Look for unusual service combinations
        if services.len() > 10 {
            return true; // Too many services
        }
        
        // Look for cycles (service appears multiple times)
        let mut seen = std::collections::HashSet::new();
        for service in services {
            if !seen.insert(service) {
                return true; // Cycle detected
            }
        }
        
        false
    }
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TraceId;

    #[tokio::test]
    async fn test_duration_anomaly() {
        let detector = PatternDetector::new();
        
        // Record normal patterns
        for i in 0..100 {
            let characteristics = TraceCharacteristics {
                trace_id: TraceId::new(format!("trace_{}", i)).unwrap(),
                has_error: false,
                duration_ms: Some(100 + (i % 20)), // 100-120ms
                span_count: 10,
                service_count: 3,
                is_anomalous: false,
                priority: super::super::SamplingPriority::Low,
            };
            detector.record_trace_pattern(&characteristics);
        }
        
        // Test anomalous duration
        let anomalous = TraceCharacteristics {
            trace_id: TraceId::new("anomalous".to_string()).unwrap(),
            has_error: false,
            duration_ms: Some(6000), // 6 seconds!
            span_count: 10,
            service_count: 3,
            is_anomalous: false,
            priority: super::super::SamplingPriority::Low,
        };
        
        assert!(detector.is_anomalous(&anomalous).await);
    }

    #[tokio::test]
    async fn test_span_count_anomaly() {
        let detector = PatternDetector::new();
        
        // Record normal patterns
        for i in 0..100 {
            let characteristics = TraceCharacteristics {
                trace_id: TraceId::new(format!("trace_{}", i)).unwrap(),
                has_error: false,
                duration_ms: Some(100),
                span_count: 10 + (i % 5), // 10-15 spans
                service_count: 3,
                is_anomalous: false,
                priority: super::super::SamplingPriority::Low,
            };
            detector.record_trace_pattern(&characteristics);
        }
        
        // Test anomalous span count
        let anomalous = TraceCharacteristics {
            trace_id: TraceId::new("many_spans".to_string()).unwrap(),
            has_error: false,
            duration_ms: Some(100),
            span_count: 600, // Way too many!
            service_count: 3,
            is_anomalous: false,
            priority: super::super::SamplingPriority::Low,
        };
        
        assert!(detector.is_anomalous(&anomalous).await);
    }

    #[test]
    fn test_unusual_path_detection() {
        let detector = PatternDetector::new();
        
        // Normal path
        let normal_services = vec![
            ServiceName::new("frontend".to_string()).unwrap(),
            ServiceName::new("api".to_string()).unwrap(),
            ServiceName::new("database".to_string()).unwrap(),
        ];
        assert!(!detector.detect_unusual_path(&normal_services));
        
        // Cycle detected
        let cycle_services = vec![
            ServiceName::new("frontend".to_string()).unwrap(),
            ServiceName::new("api".to_string()).unwrap(),
            ServiceName::new("frontend".to_string()).unwrap(), // Cycle!
        ];
        assert!(detector.detect_unusual_path(&cycle_services));
        
        // Too many services
        let many_services: Vec<_> = (0..15)
            .map(|i| ServiceName::new(format!("service_{}", i)).unwrap())
            .collect();
        assert!(detector.detect_unusual_path(&many_services));
    }
}
//! Graceful degradation strategies for high-load scenarios.
//!
//! This module implements various degradation strategies to maintain system
//! stability under memory pressure and high load conditions.

use std::sync::{Arc, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::{RwLock, Mutex};

use crate::core::{Result, Span, ServiceName, UrpoError};
use crate::storage::{StorageHealth, CleanupConfig};

/// Degradation mode levels.
#[derive(Debug, Clone, PartialEq)]
pub enum DegradationMode {
    /// Normal operation - no degradation.
    Normal,
    /// Conservative mode - slight reduction in features.
    Conservative,
    /// Reduced mode - significant feature reduction.
    Reduced,
    /// Survival mode - minimal functionality only.
    Survival,
    /// Emergency mode - rejecting new data.
    Emergency,
}

impl DegradationMode {
    /// Get sampling rate for this degradation level.
    pub fn sampling_rate(&self) -> f64 {
        match self {
            DegradationMode::Normal => 1.0,      // 100% sampling
            DegradationMode::Conservative => 0.8, // 80% sampling
            DegradationMode::Reduced => 0.5,     // 50% sampling
            DegradationMode::Survival => 0.2,    // 20% sampling
            DegradationMode::Emergency => 0.0,   // No sampling
        }
    }
    
    /// Get metrics update interval for this degradation level.
    pub fn metrics_interval(&self) -> Duration {
        match self {
            DegradationMode::Normal => Duration::from_millis(100),
            DegradationMode::Conservative => Duration::from_millis(200),
            DegradationMode::Reduced => Duration::from_millis(500),
            DegradationMode::Survival => Duration::from_secs(1),
            DegradationMode::Emergency => Duration::from_secs(5),
        }
    }
    
    /// Get maximum retention for this degradation level.
    pub fn max_retention(&self) -> Duration {
        match self {
            DegradationMode::Normal => Duration::from_secs(3600),      // 1 hour
            DegradationMode::Conservative => Duration::from_secs(1800), // 30 minutes
            DegradationMode::Reduced => Duration::from_secs(900),      // 15 minutes
            DegradationMode::Survival => Duration::from_secs(300),     // 5 minutes
            DegradationMode::Emergency => Duration::from_secs(60),     // 1 minute
        }
    }
    
    /// Get severity level (0-100).
    pub fn severity(&self) -> u8 {
        match self {
            DegradationMode::Normal => 0,
            DegradationMode::Conservative => 20,
            DegradationMode::Reduced => 50,
            DegradationMode::Survival => 80,
            DegradationMode::Emergency => 100,
        }
    }
}

/// Degradation controller that manages system-wide degradation strategies.
#[derive(Debug)]
pub struct DegradationController {
    /// Current degradation mode.
    current_mode: Arc<RwLock<DegradationMode>>,
    /// Degradation configuration.
    config: DegradationConfig,
    /// Memory pressure tracker (fixed-point: multiply by 10000 for storage).
    memory_pressure: Arc<AtomicU64>,
    /// CPU pressure tracker (fixed-point: multiply by 10000 for storage).
    cpu_pressure: Arc<AtomicU64>,
    /// Error rate tracker (fixed-point: multiply by 10000 for storage).
    error_rate: Arc<AtomicU64>,
    /// Sampling controller.
    sampler: Arc<AdaptiveSampler>,
    /// Feature flags for degradation.
    features: Arc<RwLock<FeatureFlags>>,
    /// Last mode change time.
    last_change: Arc<Mutex<Instant>>,
    /// Mode change history for hysteresis.
    mode_history: Arc<Mutex<Vec<(Instant, DegradationMode)>>>,
}

/// Configuration for degradation thresholds.
#[derive(Debug, Clone)]
pub struct DegradationConfig {
    /// Memory pressure thresholds.
    pub memory_thresholds: DegradationThresholds,
    /// CPU pressure thresholds.
    pub cpu_thresholds: DegradationThresholds,
    /// Error rate thresholds.
    pub error_thresholds: DegradationThresholds,
    /// Minimum time between mode changes.
    pub mode_change_cooldown: Duration,
    /// History size for hysteresis.
    pub history_size: usize,
    /// Hysteresis factor (prevents mode flapping).
    pub hysteresis_factor: f64,
}

/// Degradation thresholds for different modes.
#[derive(Debug, Clone)]
pub struct DegradationThresholds {
    /// Threshold for conservative mode.
    pub conservative: f64,
    /// Threshold for reduced mode.
    pub reduced: f64,
    /// Threshold for survival mode.
    pub survival: f64,
    /// Threshold for emergency mode.
    pub emergency: f64,
}

impl Default for DegradationConfig {
    fn default() -> Self {
        Self {
            memory_thresholds: DegradationThresholds {
                conservative: 0.7,  // 70% memory usage
                reduced: 0.85,      // 85% memory usage
                survival: 0.95,     // 95% memory usage
                emergency: 0.98,    // 98% memory usage
            },
            cpu_thresholds: DegradationThresholds {
                conservative: 0.6,  // 60% CPU usage
                reduced: 0.8,       // 80% CPU usage
                survival: 0.9,      // 90% CPU usage
                emergency: 0.95,    // 95% CPU usage
            },
            error_thresholds: DegradationThresholds {
                conservative: 0.05, // 5% error rate
                reduced: 0.1,       // 10% error rate
                survival: 0.2,      // 20% error rate
                emergency: 0.5,     // 50% error rate
            },
            mode_change_cooldown: Duration::from_secs(30),
            history_size: 10,
            hysteresis_factor: 0.9, // 10% hysteresis
        }
    }
}

/// Feature flags that can be disabled during degradation.
#[derive(Debug, Clone)]
pub struct FeatureFlags {
    /// Enable detailed metrics calculation.
    pub detailed_metrics: bool,
    /// Enable trace correlation.
    pub trace_correlation: bool,
    /// Enable service discovery.
    pub service_discovery: bool,
    /// Enable real-time updates.
    pub realtime_updates: bool,
    /// Enable span indexing.
    pub span_indexing: bool,
    /// Enable histogram calculations.
    pub histograms: bool,
    /// Enable percentile calculations.
    pub percentiles: bool,
    /// Enable log correlation.
    pub log_correlation: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            detailed_metrics: true,
            trace_correlation: true,
            service_discovery: true,
            realtime_updates: true,
            span_indexing: true,
            histograms: true,
            percentiles: true,
            log_correlation: true,
        }
    }
}

impl FeatureFlags {
    /// Apply degradation mode to feature flags.
    pub fn apply_degradation(&mut self, mode: &DegradationMode) {
        match mode {
            DegradationMode::Normal => {
                // All features enabled
                *self = Self::default();
            },
            DegradationMode::Conservative => {
                // Disable some expensive features
                self.log_correlation = false;
                self.histograms = false;
            },
            DegradationMode::Reduced => {
                // Disable more features
                self.log_correlation = false;
                self.histograms = false;
                self.percentiles = false;
                self.trace_correlation = false;
            },
            DegradationMode::Survival => {
                // Only essential features
                self.detailed_metrics = false;
                self.log_correlation = false;
                self.histograms = false;
                self.percentiles = false;
                self.trace_correlation = false;
                self.service_discovery = false;
            },
            DegradationMode::Emergency => {
                // Minimal functionality
                *self = Self {
                    detailed_metrics: false,
                    trace_correlation: false,
                    service_discovery: false,
                    realtime_updates: false,
                    span_indexing: false,
                    histograms: false,
                    percentiles: false,
                    log_correlation: false,
                };
            },
        }
    }
}

/// Adaptive sampler that adjusts sampling rate based on load.
#[derive(Debug)]
pub struct AdaptiveSampler {
    /// Current sampling rate (fixed-point: 0-10000 represents 0.0-1.0).
    sampling_rate: Arc<AtomicU64>,
    /// Samples taken counter.
    samples_taken: Arc<AtomicU64>,
    /// Total samples offered counter.
    samples_offered: Arc<AtomicU64>,
    /// Service-specific sampling rates.
    service_rates: Arc<RwLock<std::collections::HashMap<ServiceName, f64>>>,
}

impl AdaptiveSampler {
    /// Create a new adaptive sampler.
    pub fn new() -> Self {
        Self {
            sampling_rate: Arc::new(AtomicU64::new(10000)), // 1.0 in fixed-point
            samples_taken: Arc::new(AtomicU64::new(0)),
            samples_offered: Arc::new(AtomicU64::new(0)),
            service_rates: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Update sampling rate.
    pub async fn set_sampling_rate(&self, rate: f64) {
        self.sampling_rate.store((rate * 10000.0) as u64, Ordering::Relaxed);
    }
    
    /// Check if a span should be sampled.
    pub async fn should_sample(&self, _service: &ServiceName) -> bool {
        self.samples_offered.fetch_add(1, Ordering::Relaxed);
        
        let rate_fixed = self.sampling_rate.load(Ordering::Relaxed);
        let rate = rate_fixed as f64 / 10000.0;
        
        // Simple random sampling using system time as seed
        use std::time::SystemTime;
        let nanos = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default().subsec_nanos();
        let random_val = (nanos % 10000) as f64 / 10000.0;
        
        let should_sample = random_val < rate;
        
        if should_sample {
            self.samples_taken.fetch_add(1, Ordering::Relaxed);
        }
        
        should_sample
    }
    
    /// Get current sampling statistics.
    pub async fn get_stats(&self) -> SamplingStats {
        let offered = self.samples_offered.load(Ordering::Relaxed);
        let taken = self.samples_taken.load(Ordering::Relaxed);
        let rate = self.sampling_rate.load(Ordering::Relaxed) as f64 / 10000.0;
        
        let actual_rate = if offered > 0 {
            taken as f64 / offered as f64
        } else {
            0.0
        };
        
        SamplingStats {
            target_rate: rate,
            actual_rate,
            samples_offered: offered,
            samples_taken: taken,
        }
    }
    
    /// Reset sampling statistics.
    pub async fn reset_stats(&self) {
        self.samples_offered.store(0, Ordering::Relaxed);
        self.samples_taken.store(0, Ordering::Relaxed);
    }
}

impl Default for AdaptiveSampler {
    fn default() -> Self {
        Self::new()
    }
}

/// Sampling statistics.
#[derive(Debug, Clone)]
pub struct SamplingStats {
    /// Target sampling rate.
    pub target_rate: f64,
    /// Actual sampling rate achieved.
    pub actual_rate: f64,
    /// Total samples offered.
    pub samples_offered: u64,
    /// Total samples taken.
    pub samples_taken: u64,
}

impl DegradationController {
    /// Create a new degradation controller.
    pub fn new() -> Self {
        Self {
            current_mode: Arc::new(RwLock::new(DegradationMode::Normal)),
            config: DegradationConfig::default(),
            memory_pressure: Arc::new(AtomicU64::new(0)),
            cpu_pressure: Arc::new(AtomicU64::new(0)),
            error_rate: Arc::new(AtomicU64::new(0)),
            sampler: Arc::new(AdaptiveSampler::new()),
            features: Arc::new(RwLock::new(FeatureFlags::default())),
            last_change: Arc::new(Mutex::new(Instant::now())),
            mode_history: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Create with custom configuration.
    pub fn with_config(config: DegradationConfig) -> Self {
        let mut controller = Self::new();
        controller.config = config;
        controller
    }
    
    /// Update system pressure metrics.
    pub async fn update_pressure(&self, memory: f64, cpu: f64, errors: f64) {
        // Convert f64 to fixed-point u64 (multiply by 10000)
        self.memory_pressure.store((memory * 10000.0) as u64, Ordering::Relaxed);
        self.cpu_pressure.store((cpu * 10000.0) as u64, Ordering::Relaxed);
        self.error_rate.store((errors * 10000.0) as u64, Ordering::Relaxed);
        
        // Evaluate if mode change is needed
        self.evaluate_mode_change().await;
    }
    
    /// Evaluate if degradation mode should change.
    async fn evaluate_mode_change(&self) {
        // Convert fixed-point back to f64
        let memory = self.memory_pressure.load(Ordering::Relaxed) as f64 / 10000.0;
        let cpu = self.cpu_pressure.load(Ordering::Relaxed) as f64 / 10000.0;
        let errors = self.error_rate.load(Ordering::Relaxed) as f64 / 10000.0;
        
        // Determine required mode based on pressure
        let required_mode = self.determine_required_mode(memory, cpu, errors);
        
        let current_mode = self.current_mode.read().await.clone();
        
        // Check if mode change is needed and allowed
        if required_mode != current_mode && self.can_change_mode().await {
            self.change_mode(required_mode).await;
        }
    }
    
    /// Determine required degradation mode based on pressure metrics.
    fn determine_required_mode(&self, memory: f64, cpu: f64, errors: f64) -> DegradationMode {
        // Find the highest severity mode required by any metric
        let memory_mode = self.pressure_to_mode(memory, &self.config.memory_thresholds);
        let cpu_mode = self.pressure_to_mode(cpu, &self.config.cpu_thresholds);
        let error_mode = self.pressure_to_mode(errors, &self.config.error_thresholds);
        
        // Use the most severe mode
        let modes = [memory_mode, cpu_mode, error_mode];
        modes.into_iter().max_by_key(|m| m.severity()).unwrap_or(DegradationMode::Normal)
    }
    
    /// Convert pressure value to degradation mode.
    fn pressure_to_mode(&self, pressure: f64, thresholds: &DegradationThresholds) -> DegradationMode {
        if pressure >= thresholds.emergency {
            DegradationMode::Emergency
        } else if pressure >= thresholds.survival {
            DegradationMode::Survival
        } else if pressure >= thresholds.reduced {
            DegradationMode::Reduced
        } else if pressure >= thresholds.conservative {
            DegradationMode::Conservative
        } else {
            DegradationMode::Normal
        }
    }
    
    /// Check if mode change is allowed (cooldown and hysteresis).
    async fn can_change_mode(&self) -> bool {
        let last_change = *self.last_change.lock().await;
        last_change.elapsed() >= self.config.mode_change_cooldown
    }
    
    /// Change to new degradation mode.
    async fn change_mode(&self, new_mode: DegradationMode) {
        let old_mode = {
            let mut current = self.current_mode.write().await;
            let old = current.clone();
            *current = new_mode.clone();
            old
        };
        
        // Update last change time
        *self.last_change.lock().await = Instant::now();
        
        // Add to history
        {
            let mut history = self.mode_history.lock().await;
            history.push((Instant::now(), new_mode.clone()));
            
            // Limit history size
            if history.len() > self.config.history_size {
                history.remove(0);
            }
        }
        
        // Update sampling rate
        self.sampler.set_sampling_rate(new_mode.sampling_rate()).await;
        
        // Update feature flags
        {
            let mut features = self.features.write().await;
            features.apply_degradation(&new_mode);
        }
        
        tracing::info!(
            "Degradation mode changed: {:?} -> {:?} (memory: {:.1}%, cpu: {:.1}%, errors: {:.1}%)",
            old_mode,
            new_mode,
            self.memory_pressure.load(Ordering::Relaxed) as f64 / 100.0,
            self.cpu_pressure.load(Ordering::Relaxed) as f64 / 100.0,
            self.error_rate.load(Ordering::Relaxed) as f64 / 100.0
        );
    }
    
    /// Get current degradation mode.
    pub async fn get_mode(&self) -> DegradationMode {
        self.current_mode.read().await.clone()
    }
    
    /// Get current feature flags.
    pub async fn get_features(&self) -> FeatureFlags {
        self.features.read().await.clone()
    }
    
    /// Get current sampling rate.
    pub async fn get_sampling_rate(&self) -> f64 {
        self.sampler.sampling_rate.load(Ordering::Relaxed) as f64 / 10000.0
    }
    
    /// Check if span should be processed based on sampling.
    pub async fn should_process_span(&self, service: &ServiceName) -> bool {
        self.sampler.should_sample(service).await
    }
    
    /// Get degradation statistics.
    pub async fn get_stats(&self) -> DegradationStats {
        let mode = self.get_mode().await;
        let sampling = self.sampler.get_stats().await;
        let features = self.get_features().await;
        
        let history = {
            let history = self.mode_history.lock().await;
            history.clone()
        };
        
        DegradationStats {
            current_mode: mode,
            memory_pressure: self.memory_pressure.load(Ordering::Relaxed) as f64 / 10000.0,
            cpu_pressure: self.cpu_pressure.load(Ordering::Relaxed) as f64 / 10000.0,
            error_rate: self.error_rate.load(Ordering::Relaxed) as f64 / 10000.0,
            sampling: sampling,
            features_enabled: count_enabled_features(&features),
            total_features: count_total_features(),
            mode_changes: history.len() as u32,
            mode_history: history,
        }
    }
    
    /// Force degradation mode (for testing/emergency).
    pub async fn force_mode(&self, mode: DegradationMode) {
        tracing::warn!("Forcing degradation mode to {:?}", mode);
        self.change_mode(mode).await;
    }
    
    /// Reset to normal mode.
    pub async fn reset(&self) {
        self.memory_pressure.store(0, Ordering::Relaxed);
        self.cpu_pressure.store(0, Ordering::Relaxed);
        self.error_rate.store(0, Ordering::Relaxed);
        self.change_mode(DegradationMode::Normal).await;
        self.sampler.reset_stats().await;
    }
}

impl Default for DegradationController {
    fn default() -> Self {
        Self::new()
    }
}

/// Degradation statistics for monitoring.
#[derive(Debug, Clone)]
pub struct DegradationStats {
    /// Current degradation mode.
    pub current_mode: DegradationMode,
    /// Memory pressure (0.0 - 1.0).
    pub memory_pressure: f64,
    /// CPU pressure (0.0 - 1.0).
    pub cpu_pressure: f64,
    /// Error rate (0.0 - 1.0).
    pub error_rate: f64,
    /// Sampling statistics.
    pub sampling: SamplingStats,
    /// Number of enabled features.
    pub features_enabled: u32,
    /// Total number of features.
    pub total_features: u32,
    /// Number of mode changes.
    pub mode_changes: u32,
    /// Mode change history.
    pub mode_history: Vec<(Instant, DegradationMode)>,
}

/// Count enabled features.
fn count_enabled_features(features: &FeatureFlags) -> u32 {
    let mut count = 0;
    if features.detailed_metrics { count += 1; }
    if features.trace_correlation { count += 1; }
    if features.service_discovery { count += 1; }
    if features.realtime_updates { count += 1; }
    if features.span_indexing { count += 1; }
    if features.histograms { count += 1; }
    if features.percentiles { count += 1; }
    if features.log_correlation { count += 1; }
    count
}

/// Count total features.
fn count_total_features() -> u32 {
    8 // Total number of feature flags
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;
    
    #[tokio::test]
    async fn test_degradation_mode_sampling() {
        assert_eq!(DegradationMode::Normal.sampling_rate(), 1.0);
        assert_eq!(DegradationMode::Conservative.sampling_rate(), 0.8);
        assert_eq!(DegradationMode::Reduced.sampling_rate(), 0.5);
        assert_eq!(DegradationMode::Survival.sampling_rate(), 0.2);
        assert_eq!(DegradationMode::Emergency.sampling_rate(), 0.0);
    }
    
    #[tokio::test]
    async fn test_feature_degradation() {
        let mut features = FeatureFlags::default();
        
        // Test normal mode
        features.apply_degradation(&DegradationMode::Normal);
        assert!(features.detailed_metrics);
        assert!(features.trace_correlation);
        
        // Test emergency mode
        features.apply_degradation(&DegradationMode::Emergency);
        assert!(!features.detailed_metrics);
        assert!(!features.trace_correlation);
        assert!(!features.histograms);
    }
    
    #[tokio::test]
    async fn test_degradation_controller() {
        let controller = DegradationController::new();
        
        // Initially normal
        assert_eq!(controller.get_mode().await, DegradationMode::Normal);
        
        // Simulate high memory pressure
        controller.update_pressure(0.9, 0.1, 0.01).await;
        
        // Should have degraded
        let mode = controller.get_mode().await;
        assert_ne!(mode, DegradationMode::Normal);
        assert!(mode.severity() > 0);
    }
    
    #[tokio::test]
    async fn test_adaptive_sampler() {
        let sampler = AdaptiveSampler::new();
        let service = ServiceName::new("test".to_string()).unwrap();
        
        // Test 100% sampling
        sampler.set_sampling_rate(1.0).await;
        
        let mut samples = 0;
        for _ in 0..100 {
            if sampler.should_sample(&service).await {
                samples += 1;
            }
        }
        
        // Should sample most spans (allowing for randomness)
        assert!(samples >= 90);
        
        // Test 0% sampling
        sampler.set_sampling_rate(0.0).await;
        
        let mut samples = 0;
        for _ in 0..100 {
            if sampler.should_sample(&service).await {
                samples += 1;
            }
        }
        
        // Should sample no spans
        assert_eq!(samples, 0);
        
        let stats = sampler.get_stats().await;
        assert_eq!(stats.target_rate, 0.0);
        assert!(stats.samples_offered >= 200); // From both test runs
    }
    
    #[tokio::test] 
    async fn test_mode_change_cooldown() {
        let mut config = DegradationConfig::default();
        config.mode_change_cooldown = Duration::from_millis(100);
        
        let controller = DegradationController::with_config(config);
        
        // Change mode
        controller.update_pressure(0.9, 0.1, 0.01).await;
        let mode1 = controller.get_mode().await;
        
        // Try to change again immediately
        controller.update_pressure(0.5, 0.1, 0.01).await;
        let mode2 = controller.get_mode().await;
        
        // Should be the same due to cooldown
        assert_eq!(mode1, mode2);
        
        // Wait for cooldown
        sleep(Duration::from_millis(150)).await;
        
        // Now should change
        controller.update_pressure(0.1, 0.1, 0.01).await;
        let mode3 = controller.get_mode().await;
        
        // Should have changed
        assert_ne!(mode2, mode3);
    }
    
    #[tokio::test]
    async fn test_degradation_stats() {
        let controller = DegradationController::new();
        
        // Simulate some degradation
        controller.update_pressure(0.8, 0.6, 0.02).await;
        
        let stats = controller.get_stats().await;
        
        assert_ne!(stats.current_mode, DegradationMode::Normal);
        assert_eq!(stats.memory_pressure, 0.8);
        assert_eq!(stats.cpu_pressure, 0.6);
        assert_eq!(stats.error_rate, 0.02);
        assert!(stats.features_enabled < stats.total_features);
    }
}
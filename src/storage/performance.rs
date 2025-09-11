//! Production-grade performance management for real-time OTEL processing.
//!
//! This module provides adaptive performance tuning, backpressure handling,
//! and efficient batching to maintain optimal performance under load.

use std::collections::VecDeque;
use std::sync::{Arc, atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering}};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{RwLock, Mutex, Semaphore};
use tokio::time::timeout;

use crate::core::{Result, Span, UrpoError};

/// Performance monitoring and adaptive tuning.
#[derive(Debug)]
pub struct PerformanceManager {
    /// Current processing rate (spans/second).
    processing_rate: Arc<AtomicU64>,
    /// Peak processing rate observed.
    peak_rate: Arc<AtomicU64>,
    /// Current latency in microseconds.
    current_latency: Arc<AtomicU64>,
    /// Target latency in microseconds.
    target_latency: u64,
    /// Adaptive update interval.
    update_interval: Arc<RwLock<Duration>>,
    /// Current load factor (0.0 - 1.0).
    load_factor: Arc<RwLock<f64>>,
    /// Performance statistics.
    stats: Arc<Mutex<PerformanceStats>>,
    /// Backpressure indicator.
    backpressure: Arc<AtomicBool>,
    /// Rate limiter semaphore.
    rate_limiter: Arc<Semaphore>,
}

/// Performance statistics and metrics.
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// Total spans processed.
    pub total_spans: u64,
    /// Spans processed in last second.
    pub spans_per_second: u64,
    /// Average processing latency.
    pub avg_latency_us: u64,
    /// 95th percentile latency.
    pub p95_latency_us: u64,
    /// 99th percentile latency.
    pub p99_latency_us: u64,
    /// Current memory usage in bytes.
    pub memory_usage: u64,
    /// CPU usage estimate (0.0 - 1.0).
    pub cpu_usage: f64,
    /// Backpressure events.
    pub backpressure_events: u64,
    /// Dropped spans due to overload.
    pub dropped_spans: u64,
    /// Queue depth.
    pub queue_depth: usize,
    /// Batch efficiency (avg batch size).
    pub batch_efficiency: f64,
    /// Time to last update.
    pub last_update: SystemTime,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            total_spans: 0,
            spans_per_second: 0,
            avg_latency_us: 0,
            p95_latency_us: 0,
            p99_latency_us: 0,
            memory_usage: 0,
            cpu_usage: 0.0,
            backpressure_events: 0,
            dropped_spans: 0,
            queue_depth: 0,
            batch_efficiency: 0.0,
            last_update: SystemTime::now(),
        }
    }
}

impl PerformanceManager {
    /// Create a new performance manager with default settings.
    pub fn new() -> Self {
        Self {
            processing_rate: Arc::new(AtomicU64::new(0)),
            peak_rate: Arc::new(AtomicU64::new(0)),
            current_latency: Arc::new(AtomicU64::new(0)),
            target_latency: 10_000, // 10ms target
            update_interval: Arc::new(RwLock::new(Duration::from_millis(100))),
            load_factor: Arc::new(RwLock::new(0.0)),
            stats: Arc::new(Mutex::new(PerformanceStats::default())),
            backpressure: Arc::new(AtomicBool::new(false)),
            rate_limiter: Arc::new(Semaphore::new(10000)), // Initial capacity
        }
    }
    
    /// Create with custom target latency.
    pub fn with_target_latency(target_latency_ms: u64) -> Self {
        let mut manager = Self::new();
        manager.target_latency = target_latency_ms * 1000; // Convert to microseconds
        manager
    }
    
    /// Record span processing timing.
    pub async fn record_processing(&self, spans_count: usize, duration: Duration) {
        let latency_us = duration.as_micros() as u64;
        self.current_latency.store(latency_us, Ordering::Relaxed);
        
        // Update processing rate
        let rate = if duration.as_secs_f64() > 0.0 {
            (spans_count as f64 / duration.as_secs_f64()) as u64
        } else {
            0
        };
        
        self.processing_rate.store(rate, Ordering::Relaxed);
        
        // Update peak rate
        let current_peak = self.peak_rate.load(Ordering::Relaxed);
        if rate > current_peak {
            self.peak_rate.store(rate, Ordering::Relaxed);
        }
        
        // Update statistics
        let mut stats = self.stats.lock().await;
        stats.total_spans += spans_count as u64;
        stats.spans_per_second = rate;
        stats.avg_latency_us = latency_us;
        stats.last_update = SystemTime::now();
        
        // Calculate load factor and adjust performance
        self.update_load_factor(latency_us, rate).await;
        self.adjust_performance().await;
    }
    
    /// Update load factor based on current metrics.
    async fn update_load_factor(&self, latency_us: u64, rate: u64) {
        let peak_rate = self.peak_rate.load(Ordering::Relaxed);
        
        // Calculate load factor based on latency and throughput
        let latency_factor = latency_us as f64 / self.target_latency as f64;
        let throughput_factor = if peak_rate > 0 {
            rate as f64 / peak_rate as f64
        } else {
            0.0
        };
        
        // Weighted combination (latency is more important)
        let load_factor = (latency_factor * 0.7 + throughput_factor * 0.3).min(1.0);
        
        *self.load_factor.write().await = load_factor;
        
        // Set backpressure if load is too high
        self.backpressure.store(load_factor > 0.8, Ordering::Relaxed);
    }
    
    /// Dynamically adjust performance parameters.
    async fn adjust_performance(&self) {
        let load_factor = *self.load_factor.read().await;
        
        // Adjust update interval based on load
        let new_interval = if load_factor > 0.9 {
            // High load: slower updates to reduce overhead
            Duration::from_millis(500)
        } else if load_factor > 0.7 {
            // Medium load: balanced updates
            Duration::from_millis(200)
        } else if load_factor > 0.3 {
            // Low load: faster updates for responsiveness
            Duration::from_millis(100)
        } else {
            // Very low load: very fast updates
            Duration::from_millis(50)
        };
        
        *self.update_interval.write().await = new_interval;
        
        // Adjust rate limiter capacity
        let target_capacity = if load_factor > 0.8 {
            5000  // Reduce capacity under high load
        } else if load_factor > 0.5 {
            10000 // Normal capacity
        } else {
            20000 // Increase capacity under low load
        };
        
        // Note: Semaphore capacity can't be dynamically adjusted in tokio
        // In a real implementation, we'd recreate the semaphore or use a different approach
    }
    
    /// Get optimal update interval based on current load.
    pub async fn get_update_interval(&self) -> Duration {
        *self.update_interval.read().await
    }
    
    /// Check if system is under backpressure.
    pub fn is_backpressure(&self) -> bool {
        self.backpressure.load(Ordering::Relaxed)
    }
    
    /// Get current load factor (0.0 = idle, 1.0 = maximum load).
    pub async fn get_load_factor(&self) -> f64 {
        *self.load_factor.read().await
    }
    
    /// Acquire rate limiting permit.
    pub async fn acquire_permit(&self) -> Result<()> {
        match timeout(Duration::from_millis(100), self.rate_limiter.acquire()).await {
            Ok(Ok(_permit)) => Ok(()),
            Ok(Err(_)) => Err(UrpoError::ChannelSend),
            Err(_) => {
                // Timeout: system is overloaded
                self.backpressure.store(true, Ordering::Relaxed);
                let mut stats = self.stats.lock().await;
                stats.backpressure_events += 1;
                Err(UrpoError::Timeout { timeout_ms: 100 })
            }
        }
    }
    
    /// Get comprehensive performance statistics.
    pub async fn get_stats(&self) -> PerformanceStats {
        let mut stats = self.stats.lock().await;
        stats.spans_per_second = self.processing_rate.load(Ordering::Relaxed);
        stats.avg_latency_us = self.current_latency.load(Ordering::Relaxed);
        stats.clone()
    }
    
    /// Reset performance counters.
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.lock().await;
        *stats = PerformanceStats::default();
        self.processing_rate.store(0, Ordering::Relaxed);
        self.current_latency.store(0, Ordering::Relaxed);
        self.backpressure.store(false, Ordering::Relaxed);
    }
}

/// Adaptive batch processor for efficient span handling.
#[derive(Debug)]
pub struct AdaptiveBatcher {
    /// Pending spans buffer.
    buffer: Arc<Mutex<VecDeque<Span>>>,
    /// Current batch size.
    batch_size: Arc<AtomicUsize>,
    /// Minimum batch size.
    min_batch_size: usize,
    /// Maximum batch size.
    max_batch_size: usize,
    /// Batch timeout.
    batch_timeout: Arc<RwLock<Duration>>,
    /// Performance manager for feedback.
    perf_manager: Arc<PerformanceManager>,
    /// Last batch timestamp.
    last_batch: Arc<Mutex<Instant>>,
}

impl AdaptiveBatcher {
    /// Create a new adaptive batcher.
    pub fn new(perf_manager: Arc<PerformanceManager>) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            batch_size: Arc::new(AtomicUsize::new(100)),
            min_batch_size: 10,
            max_batch_size: 1000,
            batch_timeout: Arc::new(RwLock::new(Duration::from_millis(100))),
            perf_manager,
            last_batch: Arc::new(Mutex::new(Instant::now())),
        }
    }
    
    /// Add span to batch buffer with optimized lock usage.
    pub async fn add_span(&self, span: Span) -> Result<Option<Vec<Span>>> {
        // Fast path: check rate limiting without acquiring lock
        if self.perf_manager.is_backpressure() {
            return Err(UrpoError::Timeout { timeout_ms: 0 });
        }
        
        // Try non-blocking rate limiting first
        if let Ok(_permit) = self.perf_manager.rate_limiter.try_acquire() {
            // Fast path: got permit immediately
        } else {
            // Slow path: use async acquire with short timeout
            self.perf_manager.acquire_permit().await?;
        }
        
        let target_batch_size = self.batch_size.load(Ordering::Relaxed);
        
        // Use try_lock to avoid blocking on contention
        let batch_result = match self.buffer.try_lock() {
            Ok(mut buffer) => {
                buffer.push_back(span);
                let current_size = buffer.len();
                
                // Check if we should flush (avoid async call in lock)
                let should_flush_now = current_size >= target_batch_size;
                
                if should_flush_now {
                    let batch: Vec<Span> = buffer.drain(..).collect();
                    Some((batch, current_size))
                } else {
                    None
                }
            },
            Err(_) => {
                // Buffer is locked, defer to background flush
                return Ok(None);
            }
        };
        
        if let Some((batch, size)) = batch_result {
            // Update timestamp after lock is released
            *self.last_batch.lock().await = Instant::now();
            
            // Adjust batch size asynchronously (non-blocking)
            let batch_len = batch.len();
            let perf_manager = self.perf_manager.clone();
            let batch_size = self.batch_size.clone();
            let batch_timeout = self.batch_timeout.clone();
            let min_batch = self.min_batch_size;
            let max_batch = self.max_batch_size;
            
            tokio::spawn(async move {
                let load_factor = perf_manager.get_load_factor().await;
                let current_batch_size = batch_size.load(Ordering::Relaxed);
                
                let new_batch_size = if load_factor > 0.8 {
                    (current_batch_size * 110 / 100).min(max_batch)
                } else if load_factor < 0.3 {
                    (current_batch_size * 90 / 100).max(min_batch)
                } else {
                    current_batch_size
                };
                
                batch_size.store(new_batch_size, Ordering::Relaxed);
                
                let new_timeout = if load_factor > 0.7 {
                    Duration::from_millis(50)
                } else {
                    Duration::from_millis(100)
                };
                
                *batch_timeout.write().await = new_timeout;
            });
            
            Ok(Some(batch))
        } else {
            Ok(None)
        }
    }
    
    /// Check if batch should be flushed due to timeout.
    async fn should_flush(&self) -> bool {
        let last_batch = *self.last_batch.lock().await;
        let timeout = *self.batch_timeout.read().await;
        last_batch.elapsed() >= timeout
    }
    
    /// Adjust batch size based on performance feedback.
    async fn adjust_batch_size(&self, actual_batch_size: usize, buffer_size: usize) {
        let load_factor = self.perf_manager.get_load_factor().await;
        let current_batch_size = self.batch_size.load(Ordering::Relaxed);
        
        let new_batch_size = if load_factor > 0.8 {
            // High load: increase batch size to improve throughput
            (current_batch_size * 110 / 100).min(self.max_batch_size)
        } else if load_factor < 0.3 {
            // Low load: decrease batch size for better latency
            (current_batch_size * 90 / 100).max(self.min_batch_size)
        } else {
            // Medium load: keep current size
            current_batch_size
        };
        
        self.batch_size.store(new_batch_size, Ordering::Relaxed);
        
        // Adjust timeout based on load
        let new_timeout = if load_factor > 0.7 {
            Duration::from_millis(50)  // Faster batching under load
        } else {
            Duration::from_millis(100) // Normal batching
        };
        
        *self.batch_timeout.write().await = new_timeout;
    }
    
    /// Force flush current batch.
    pub async fn flush(&self) -> Vec<Span> {
        let mut buffer = self.buffer.lock().await;
        let batch: Vec<Span> = buffer.drain(..).collect();
        *self.last_batch.lock().await = Instant::now();
        batch
    }
    
    /// Get current buffer size.
    pub async fn buffer_size(&self) -> usize {
        self.buffer.lock().await.len()
    }
    
    /// Get current batch configuration.
    pub async fn get_config(&self) -> (usize, Duration) {
        let batch_size = self.batch_size.load(Ordering::Relaxed);
        let timeout = *self.batch_timeout.read().await;
        (batch_size, timeout)
    }
}

/// Background performance monitor that runs periodic optimization.
#[derive(Debug)]
pub struct PerformanceMonitor {
    /// Performance manager.
    perf_manager: Arc<PerformanceManager>,
    /// Monitoring interval.
    monitor_interval: Duration,
    /// Shutdown signal.
    shutdown: Arc<AtomicBool>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor.
    pub fn new(perf_manager: Arc<PerformanceManager>) -> Self {
        Self {
            perf_manager,
            monitor_interval: Duration::from_secs(5),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// Start monitoring in background.
    pub async fn start(&self) -> Result<()> {
        let perf_manager = self.perf_manager.clone();
        let monitor_interval = self.monitor_interval;
        let shutdown = self.shutdown.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(monitor_interval);
            let mut stats_cache = None;
            let mut cache_time = Instant::now();
            
            while !shutdown.load(Ordering::Relaxed) {
                interval.tick().await;
                
                // Use cached stats if recent (reduce lock contention)
                let stats = if cache_time.elapsed() < Duration::from_millis(500) && stats_cache.is_some() {
                    stats_cache.clone().unwrap()
                } else {
                    let s = perf_manager.get_stats().await;
                    stats_cache = Some(s.clone());
                    cache_time = Instant::now();
                    s
                };
                
                // Batch logging to reduce syscall overhead
                let load_factor = perf_manager.get_load_factor().await;
                let is_backpressure = perf_manager.is_backpressure();
                
                // Only log if significant changes (reduce log spam)
                static mut LAST_LOG_TIME: Option<Instant> = None;
                let should_log = unsafe {
                    LAST_LOG_TIME.map_or(true, |t| t.elapsed() > Duration::from_secs(10))
                        || stats.avg_latency_us > 50_000
                        || is_backpressure
                        || stats.dropped_spans > 0
                };
                
                if should_log {
                    unsafe { LAST_LOG_TIME = Some(Instant::now()); }
                    
                    // Batch all logging at once
                    if stats.avg_latency_us > 50_000 {
                        tracing::warn!(
                            "PERFORMANCE ALERT: {}μs latency (target: {}μs), {}spans/s, load={:.2}, backpressure={}, dropped={}",
                            stats.avg_latency_us,
                            perf_manager.target_latency,
                            stats.spans_per_second,
                            load_factor,
                            is_backpressure,
                            stats.dropped_spans
                        );
                    } else {
                        tracing::debug!(
                            "Performance: {}spans/s, {}μs latency, load={:.2}",
                            stats.spans_per_second,
                            stats.avg_latency_us,
                            load_factor
                        );
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Stop monitoring.
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

/// Circuit breaker for protecting against overload.
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Current state.
    state: Arc<RwLock<CircuitState>>,
    /// Failure threshold.
    failure_threshold: usize,
    /// Success threshold for recovery.
    success_threshold: usize,
    /// Timeout before retry.
    timeout: Duration,
    /// Failure counter.
    failures: Arc<AtomicUsize>,
    /// Success counter.
    successes: Arc<AtomicUsize>,
    /// Last failure time.
    last_failure: Arc<Mutex<Option<Instant>>>,
}

#[derive(Debug, Clone, PartialEq, Copy)]
enum CircuitState {
    Closed,  // Normal operation
    Open,    // Failing, rejecting requests
    HalfOpen, // Testing recovery
}

impl CircuitBreaker {
    /// Create a new circuit breaker.
    pub fn new(failure_threshold: usize, success_threshold: usize, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_threshold,
            success_threshold,
            timeout,
            failures: Arc::new(AtomicUsize::new(0)),
            successes: Arc::new(AtomicUsize::new(0)),
            last_failure: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Check if request should be allowed.
    pub async fn allow_request(&self) -> bool {
        let state = *self.state.read().await;
        
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                if let Some(last_failure) = *self.last_failure.lock().await {
                    if last_failure.elapsed() >= self.timeout {
                        // Transition to half-open
                        *self.state.write().await = CircuitState::HalfOpen;
                        self.successes.store(0, Ordering::Relaxed);
                        return true;
                    }
                }
                false
            },
            CircuitState::HalfOpen => true, // Allow limited requests to test recovery
        }
    }
    
    /// Record successful operation.
    pub async fn record_success(&self) {
        let state = *self.state.read().await;
        
        match state {
            CircuitState::Closed => {
                // Reset failure counter on success
                self.failures.store(0, Ordering::Relaxed);
            },
            CircuitState::HalfOpen => {
                let successes = self.successes.fetch_add(1, Ordering::Relaxed) + 1;
                if successes >= self.success_threshold {
                    // Transition back to closed
                    *self.state.write().await = CircuitState::Closed;
                    self.failures.store(0, Ordering::Relaxed);
                    self.successes.store(0, Ordering::Relaxed);
                }
            },
            CircuitState::Open => {
                // Should not happen, but reset if it does
                *self.state.write().await = CircuitState::Closed;
                self.failures.store(0, Ordering::Relaxed);
            }
        }
    }
    
    /// Record failed operation.
    pub async fn record_failure(&self) {
        let failures = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        
        if failures >= self.failure_threshold {
            // Transition to open
            *self.state.write().await = CircuitState::Open;
            *self.last_failure.lock().await = Some(Instant::now());
            self.successes.store(0, Ordering::Relaxed);
        }
    }
    
    /// Get current state.
    pub async fn get_state(&self) -> CircuitState {
        *self.state.read().await
    }
    
    /// Get failure count.
    pub fn get_failures(&self) -> usize {
        self.failures.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;
    
    #[tokio::test]
    async fn test_performance_manager() {
        let perf_manager = PerformanceManager::new();
        
        // Simulate processing
        perf_manager.record_processing(100, Duration::from_millis(5)).await;
        
        let stats = perf_manager.get_stats().await;
        assert!(stats.spans_per_second > 0);
        assert!(stats.avg_latency_us > 0);
        
        let load_factor = perf_manager.get_load_factor().await;
        assert!(load_factor >= 0.0 && load_factor <= 1.0);
    }
    
    #[tokio::test]
    async fn test_adaptive_batcher() {
        let perf_manager = Arc::new(PerformanceManager::new());
        let batcher = AdaptiveBatcher::new(perf_manager);
        
        // Add spans to batch
        for i in 0..10 {
            let span = crate::core::Span::builder()
                .trace_id(crate::core::TraceId::new(format!("trace_{}", i)).unwrap())
                .span_id(crate::core::SpanId::new(format!("span_{}", i)).unwrap())
                .service_name(crate::core::ServiceName::new("test".to_string()).unwrap())
                .operation_name("test_op")
                .start_time(SystemTime::now())
                .duration(Duration::from_millis(100))
                .status(crate::core::SpanStatus::Ok)
                .build()
                .unwrap();
            
            let batch = batcher.add_span(span).await.unwrap();
            
            // Should get a batch when buffer fills up
            if i == 9 {
                assert!(batch.is_some());
                assert_eq!(batch.unwrap().len(), 10);
            }
        }
    }
    
    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));
        
        // Initially closed
        assert!(breaker.allow_request().await);
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        
        // Record failures
        for _ in 0..3 {
            breaker.record_failure().await;
        }
        
        // Should be open now
        assert_eq!(breaker.get_state().await, CircuitState::Open);
        assert!(!breaker.allow_request().await);
        
        // Wait for timeout
        sleep(Duration::from_millis(150)).await;
        
        // Should transition to half-open
        assert!(breaker.allow_request().await);
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);
        
        // Record successes to close circuit
        for _ in 0..2 {
            breaker.record_success().await;
        }
        
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
    }
    
    #[tokio::test]
    async fn test_backpressure_detection() {
        let perf_manager = PerformanceManager::with_target_latency(10); // 10ms target
        
        // Simulate high latency
        perf_manager.record_processing(100, Duration::from_millis(50)).await; // 50ms actual
        
        // Should detect backpressure
        assert!(perf_manager.is_backpressure());
        
        let load_factor = perf_manager.get_load_factor().await;
        assert!(load_factor > 0.8);
    }
}
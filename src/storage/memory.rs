//! In-memory storage backend for trace data.
//!
//! Production-ready in-memory storage implementation with advanced memory management,
//! bounded capacity, and efficient cleanup mechanisms. Designed for high-throughput
//! trace ingestion with strict memory limits.

use crate::core::{Config, Result, ServiceMetrics, ServiceName, Span, SpanId, TraceId};
use super::{StorageBackend, TraceInfo, StorageStats, StorageHealth, aggregator};
use dashmap::DashMap;
use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, atomic::{AtomicU64, AtomicUsize, Ordering}};
use std::time::{SystemTime, Duration, Instant};
use tokio::sync::{RwLock, Mutex};

/// Memory cleanup configuration.
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    /// Maximum memory usage in bytes before cleanup.
    pub max_memory_bytes: usize,
    /// Warning threshold (0.0 - 1.0).
    pub warning_threshold: f64,
    /// Critical threshold (0.0 - 1.0).
    pub critical_threshold: f64,
    /// Emergency threshold (0.0 - 1.0).
    pub emergency_threshold: f64,
    /// Span retention period.
    pub retention_period: Duration,
    /// Cleanup interval.
    pub cleanup_interval: Duration,
    /// Minimum spans to keep per service.
    pub min_spans_per_service: usize,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 512 * 1024 * 1024, // 512MB
            warning_threshold: 0.7,
            critical_threshold: 0.85,
            emergency_threshold: 0.95,
            retention_period: Duration::from_secs(3600), // 1 hour
            cleanup_interval: Duration::from_secs(30),
            min_spans_per_service: 100,
        }
    }
}

/// Performance and monitoring counters.
#[derive(Debug)]
struct StorageCounters {
    /// Total spans processed.
    spans_processed: AtomicU64,
    /// Processing errors.
    processing_errors: AtomicU64,
    /// Cleanup operations performed.
    cleanup_operations: AtomicU64,
    /// Memory bytes estimate.
    memory_bytes: AtomicUsize,
    /// Spans evicted.
    spans_evicted: AtomicU64,
    /// Start time for rate calculations.
    start_time: Instant,
}

/// Production-ready in-memory storage with advanced memory management.
pub struct InMemoryStorage {
    /// Spans indexed by span ID.
    spans: Arc<DashMap<SpanId, Span>>,
    /// Trace ID to span IDs mapping.
    traces: Arc<DashMap<TraceId, Vec<SpanId>>>,
    /// Service to span IDs mapping with timestamps for efficient querying.
    services: Arc<DashMap<ServiceName, VecDeque<(SystemTime, SpanId)>>>,
    /// Ordered list of span IDs by insertion time for LRU eviction.
    span_order: Arc<RwLock<VecDeque<(SystemTime, SpanId)>>>,
    /// Maximum number of spans to store.
    max_spans: usize,
    /// Maximum spans per service.
    max_spans_per_service: usize,
    /// Memory cleanup configuration.
    cleanup_config: CleanupConfig,
    /// Performance counters.
    counters: StorageCounters,
    /// Last cleanup operation time.
    last_cleanup: Arc<Mutex<Instant>>,
    /// Active service names for efficient listing.
    active_services: Arc<RwLock<HashMap<ServiceName, SystemTime>>>,
}

impl InMemoryStorage {
    /// Create a new production-ready in-memory storage with specified limits.
    pub fn new(max_spans: usize) -> Self {
        Self {
            spans: Arc::new(DashMap::new()),
            traces: Arc::new(DashMap::new()),
            services: Arc::new(DashMap::new()),
            span_order: Arc::new(RwLock::new(VecDeque::new())),
            max_spans,
            max_spans_per_service: max_spans / 10, // Allow each service ~10% of total capacity
            cleanup_config: CleanupConfig::default(),
            counters: StorageCounters {
                spans_processed: AtomicU64::new(0),
                processing_errors: AtomicU64::new(0),
                cleanup_operations: AtomicU64::new(0),
                memory_bytes: AtomicUsize::new(0),
                spans_evicted: AtomicU64::new(0),
                start_time: Instant::now(),
            },
            last_cleanup: Arc::new(Mutex::new(Instant::now())),
            active_services: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create storage with custom cleanup configuration.
    pub fn with_cleanup_config(max_spans: usize, cleanup_config: CleanupConfig) -> Self {
        let mut storage = Self::new(max_spans);
        storage.cleanup_config = cleanup_config;
        storage
    }
    
    /// Create storage from application configuration.
    pub fn with_config(config: &Config) -> Self {
        let cleanup_config = CleanupConfig {
            max_memory_bytes: config.storage.max_memory_mb * 1024 * 1024,
            warning_threshold: 0.7,
            critical_threshold: 0.85,
            emergency_threshold: 0.95,
            retention_period: config.storage.retention_duration,
            cleanup_interval: config.storage.cleanup_interval,
            min_spans_per_service: 100,
        };
        
        let mut storage = Self::new(config.storage.max_spans);
        storage.cleanup_config = cleanup_config;
        storage.max_spans_per_service = config.storage.max_spans / 10;
        storage
    }

    /// Production-grade span eviction with memory tracking (async-runtime friendly).
    async fn evict_oldest_spans(&self, count: usize) -> usize {
        let batch_size = 100; // Process in batches to avoid blocking
        let mut total_removed = 0;
        let mut total_memory_freed = 0;
        let mut remaining = count;

        while remaining > 0 {
            let batch_count = remaining.min(batch_size);
            let mut span_ids_to_remove = Vec::new();
            
            // Batch 1: Collect span IDs while holding lock minimally
            {
                let mut span_order = self.span_order.write().await;
                for _ in 0..batch_count {
                    if let Some((_, span_id)) = span_order.pop_front() {
                        span_ids_to_remove.push(span_id);
                    } else {
                        break;
                    }
                }
            } // Lock released here
            
            if span_ids_to_remove.is_empty() {
                break;
            }
            
            // Batch 2: Process removals without holding span_order lock
            let mut batch_memory_freed = 0;
            let mut batch_removed = 0;
            
            for span_id in span_ids_to_remove {
                if let Some((_, span)) = self.spans.remove(&span_id) {
                    // Estimate memory freed
                    batch_memory_freed += self.estimate_span_memory(&span);
                    
                    // Remove from trace index
                    if let Some(mut trace_spans) = self.traces.get_mut(&span.trace_id) {
                        trace_spans.retain(|id| id != &span_id);
                        if trace_spans.is_empty() {
                            drop(trace_spans);
                            self.traces.remove(&span.trace_id);
                        }
                    }

                    // Remove from service index
                    if let Some(mut service_spans) = self.services.get_mut(&span.service_name) {
                        service_spans.retain(|(_, id)| id != &span_id);
                        if service_spans.is_empty() {
                            drop(service_spans);
                            self.services.remove(&span.service_name);
                        }
                    }

                    batch_removed += 1;
                }
            }
            
            total_removed += batch_removed;
            total_memory_freed += batch_memory_freed;
            remaining -= batch_count;
            
            // Yield to async runtime after each batch
            if remaining > 0 {
                tokio::task::yield_now().await;
            }
        }
        
        // Update memory tracking
        self.counters.memory_bytes.fetch_sub(total_memory_freed, Ordering::Relaxed);
        self.counters.spans_evicted.fetch_add(total_removed as u64, Ordering::Relaxed);

        if total_removed > 0 {
            tracing::debug!(
                "Evicted {} spans in batches, freed ~{}KB memory", 
                total_removed, 
                total_memory_freed / 1024
            );
        }

        total_removed
    }
    
    /// Estimate memory usage of a span in bytes.
    fn estimate_span_memory(&self, span: &Span) -> usize {
        // Conservative estimate including overhead
        let base_size = std::mem::size_of::<Span>();
        let string_sizes = span.trace_id.as_str().len() +
                          span.span_id.as_str().len() +
                          span.service_name.as_str().len() +
                          span.operation_name.len();
        let tags_size = span.tags.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>();
        
        base_size + string_sizes + tags_size + 200 // 200 bytes overhead
    }

    /// Enforce per-service limits with memory awareness (async-runtime friendly).
    async fn enforce_service_limits(&self) {
        let batch_size = 50; // Process services in batches
        let services_to_process: Vec<_> = self.services.iter()
            .map(|entry| entry.key().clone())
            .collect();
        
        for service_chunk in services_to_process.chunks(batch_size) {
            for service_name in service_chunk {
                if let Some(mut entry) = self.services.get_mut(service_name) {
                    let service_spans = entry.value_mut();
                    let mut spans_to_remove = Vec::new();
                    
                    // Collect spans to remove
                    while service_spans.len() > self.max_spans_per_service {
                        if let Some((_, old_span_id)) = service_spans.pop_front() {
                            spans_to_remove.push(old_span_id);
                        } else {
                            break;
                        }
                    }
                    
                    // Keep service active if it has recent spans
                    if !service_spans.is_empty() {
                        let latest_time = service_spans.back().map(|(t, _)| *t).unwrap_or(SystemTime::now());
                        self.active_services.write().await.insert(service_name.clone(), latest_time);
                    }
                    
                    drop(entry); // Release the dashmap entry lock
                    
                    // Process removals outside the service lock
                    for old_span_id in spans_to_remove {
                        if let Some((_, span)) = self.spans.remove(&old_span_id) {
                            // Update memory tracking
                            let memory_freed = self.estimate_span_memory(&span);
                            self.counters.memory_bytes.fetch_sub(memory_freed, Ordering::Relaxed);
                            
                            // Remove from trace index
                            if let Some(mut trace_spans) = self.traces.get_mut(&span.trace_id) {
                                trace_spans.retain(|id| id != &old_span_id);
                                if trace_spans.is_empty() {
                                    drop(trace_spans);
                                    self.traces.remove(&span.trace_id);
                                }
                            }
                            
                            // Remove from span order (batch update)
                            {
                                let mut span_order = self.span_order.write().await;
                                span_order.retain(|(_, id)| id != &old_span_id);
                            }
                            
                            self.counters.spans_evicted.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
            
            // Yield after each batch to prevent blocking the runtime
            tokio::task::yield_now().await;
        }
    }
    
    /// Aggressive cleanup for memory pressure situations.
    async fn emergency_cleanup_internal(&self) -> Result<usize> {
        let mut removed = 0;
        
        // 1. Remove expired spans based on retention period
        let cutoff_time = SystemTime::now() - self.cleanup_config.retention_period;
        removed += self.cleanup_expired_spans(cutoff_time).await;
        
        // 2. Remove incomplete traces (orphaned spans)
        removed += self.cleanup_incomplete_traces().await;
        
        // 3. Remove inactive services
        removed += self.cleanup_inactive_services().await;
        
        // 4. If still over limit, do aggressive LRU eviction
        let current_memory = self.counters.memory_bytes.load(Ordering::Relaxed);
        if current_memory > self.cleanup_config.max_memory_bytes {
            let target_memory = (self.cleanup_config.max_memory_bytes as f64 * 0.8) as usize;
            let spans_to_remove = ((current_memory - target_memory) / 1024).max(100); // Rough estimate
            removed += self.evict_oldest_spans(spans_to_remove).await;
        }
        
        self.counters.cleanup_operations.fetch_add(1, Ordering::Relaxed);
        
        if removed > 0 {
            tracing::info!(
                "Emergency cleanup completed: removed {} spans, memory: {}MB",
                removed,
                self.counters.memory_bytes.load(Ordering::Relaxed) / 1024 / 1024
            );
        }
        
        Ok(removed)
    }
    
    /// Remove spans older than the retention period (async-runtime friendly).
    async fn cleanup_expired_spans(&self, cutoff_time: SystemTime) -> usize {
        let batch_size = 100;
        let mut total_removed = 0;
        
        loop {
            let mut expired_spans = Vec::new();
            
            // Batch 1: Collect expired span IDs while minimizing lock time
            {
                let mut span_order = self.span_order.write().await;
                for _ in 0..batch_size {
                    if let Some((timestamp, _span_id)) = span_order.front() {
                        if *timestamp < cutoff_time {
                            // SAFE: We just checked front() exists
                            if let Some((_, span_id)) = span_order.pop_front() {
                                expired_spans.push(span_id);
                            }
                        } else {
                            break; // Spans are ordered by time
                        }
                    } else {
                        break;
                    }
                }
            } // Lock released here
            
            if expired_spans.is_empty() {
                break;
            }
            
            // Batch 2: Process removals without holding span_order lock
            for span_id in expired_spans {
                if let Some((_, span)) = self.spans.remove(&span_id) {
                    // Remove from all indices (optimized to avoid repeated locks)
                    self.remove_span_from_indices_fast(&span, &span_id).await;
                    total_removed += 1;
                }
            }
            
            // Yield to async runtime after each batch
            tokio::task::yield_now().await;
        }
        
        total_removed
    }
    
    /// Remove incomplete traces (traces with only one span that's been around too long).
    async fn cleanup_incomplete_traces(&self) -> usize {
        let mut removed = 0;
        let cutoff = SystemTime::now() - Duration::from_secs(300); // 5 minutes
        let batch_size = 100;
        
        let traces_to_check: Vec<_> = self.traces.iter()
            .filter(|entry| entry.value().len() == 1)
            .map(|entry| (entry.key().clone(), entry.value()[0].clone()))
            .collect();
        
        for trace_chunk in traces_to_check.chunks(batch_size) {
            for (_trace_id, span_id) in trace_chunk {
                if let Some(span) = self.spans.get(span_id) {
                    if span.start_time < cutoff {
                        drop(span);
                        if let Some((_, span)) = self.spans.remove(span_id) {
                            self.remove_span_from_indices_fast(&span, span_id).await;
                            removed += 1;
                        }
                    }
                }
            }
            
            // Yield after each batch
            tokio::task::yield_now().await;
        }
        
        removed
    }
    
    /// Remove services that haven't seen activity recently (async-runtime friendly).
    async fn cleanup_inactive_services(&self) -> usize {
        let mut removed = 0;
        let cutoff = SystemTime::now() - Duration::from_secs(900); // 15 minutes
        let batch_size = 20; // Smaller batches for service cleanup
        
        let inactive_services: Vec<_> = {
            let active_services = self.active_services.read().await;
            active_services.iter()
                .filter(|(_, &last_seen)| last_seen < cutoff)
                .map(|(service, _)| service.clone())
                .collect()
        };
        
        for service_chunk in inactive_services.chunks(batch_size) {
            for service_name in service_chunk {
                if let Some((_, service_spans)) = self.services.remove(service_name) {
                    let span_ids: Vec<_> = service_spans.into_iter().map(|(_, span_id)| span_id).collect();
                    
                    // Process span removals in smaller sub-batches
                    for span_chunk in span_ids.chunks(50) {
                        for span_id in span_chunk {
                            if let Some((_, span)) = self.spans.remove(span_id) {
                                self.remove_span_from_indices_fast(&span, span_id).await;
                                removed += 1;
                            }
                        }
                        
                        // Micro-yield within service processing
                        if span_chunk.len() == 50 {
                            tokio::task::yield_now().await;
                        }
                    }
                    
                    // Remove from active services
                    self.active_services.write().await.remove(service_name);
                }
            }
            
            // Yield after each service batch
            tokio::task::yield_now().await;
        }
        
        removed
    }
    
    /// Helper to remove span from all indices.
    async fn remove_span_from_indices(&self, span: &Span, span_id: &SpanId) {
        // Update memory tracking
        let memory_freed = self.estimate_span_memory(span);
        self.counters.memory_bytes.fetch_sub(memory_freed, Ordering::Relaxed);
        
        // Remove from trace index
        if let Some(mut trace_spans) = self.traces.get_mut(&span.trace_id) {
            trace_spans.retain(|id| id != span_id);
            if trace_spans.is_empty() {
                drop(trace_spans);
                self.traces.remove(&span.trace_id);
            }
        }
        
        // Remove from service index
        if let Some(mut service_spans) = self.services.get_mut(&span.service_name) {
            service_spans.retain(|(_, id)| id != span_id);
            if service_spans.is_empty() {
                drop(service_spans);
                self.services.remove(&span.service_name);
            }
        }
        
        // Remove from span order
        let mut span_order = self.span_order.write().await;
        span_order.retain(|(_, id)| id != span_id);
    }
    
    /// Fast version that skips span_order cleanup (for batch operations).
    /// span_order is already cleaned up in the calling batch method.
    async fn remove_span_from_indices_fast(&self, span: &Span, span_id: &SpanId) {
        // Update memory tracking
        let memory_freed = self.estimate_span_memory(span);
        self.counters.memory_bytes.fetch_sub(memory_freed, Ordering::Relaxed);
        
        // Remove from trace index
        if let Some(mut trace_spans) = self.traces.get_mut(&span.trace_id) {
            trace_spans.retain(|id| id != span_id);
            if trace_spans.is_empty() {
                drop(trace_spans);
                self.traces.remove(&span.trace_id);
            }
        }
        
        // Remove from service index
        if let Some(mut service_spans) = self.services.get_mut(&span.service_name) {
            service_spans.retain(|(_, id)| id != span_id);
            if service_spans.is_empty() {
                drop(service_spans);
                self.services.remove(&span.service_name);
            }
        }
        
        // Note: span_order cleanup is handled by the batch caller to avoid repeated locks
    }
    
    /// Check if cleanup is needed based on memory pressure.
    pub async fn should_cleanup(&self) -> bool {
        let last_cleanup = *self.last_cleanup.lock().await;
        let memory_usage = self.counters.memory_bytes.load(Ordering::Relaxed);
        let memory_pressure = memory_usage as f64 / self.cleanup_config.max_memory_bytes as f64;
        
        // Always cleanup if over critical threshold
        if memory_pressure >= self.cleanup_config.critical_threshold {
            return true;
        }
        
        // Regular cleanup interval
        last_cleanup.elapsed() >= self.cleanup_config.cleanup_interval
    }
    
    /// Get current memory pressure level.
    pub fn get_memory_pressure(&self) -> f64 {
        let memory_usage = self.counters.memory_bytes.load(Ordering::Relaxed);
        memory_usage as f64 / self.cleanup_config.max_memory_bytes as f64
    }
    
    /// Get storage health status.
    pub fn get_health_status(&self) -> StorageHealth {
        let pressure = self.get_memory_pressure();
        
        if pressure >= self.cleanup_config.emergency_threshold {
            StorageHealth::Critical
        } else if pressure >= self.cleanup_config.critical_threshold {
            StorageHealth::Critical
        } else if pressure >= self.cleanup_config.warning_threshold {
            StorageHealth::Degraded
        } else {
            StorageHealth::Healthy
        }
    }
    
    /// List all active service names.
    pub async fn list_active_services(&self) -> Vec<ServiceName> {
        self.active_services.read().await.keys().cloned().collect()
    }

    /// Get detailed statistics for monitoring.
    pub async fn get_detailed_stats(&self) -> StorageStats {
        let span_count = self.spans.len();
        let trace_count = self.traces.len();
        let service_count = self.services.len();
        let memory_bytes = self.counters.memory_bytes.load(Ordering::Relaxed);
        let memory_mb = memory_bytes as f64 / 1024.0 / 1024.0;
        let memory_pressure = self.get_memory_pressure();
        
        // Calculate processing rate
        let elapsed = self.counters.start_time.elapsed().as_secs_f64();
        let spans_processed = self.counters.spans_processed.load(Ordering::Relaxed);
        let processing_errors = self.counters.processing_errors.load(Ordering::Relaxed);
        let processing_rate = if elapsed > 0.0 { spans_processed as f64 / elapsed } else { 0.0 };
        let error_rate = if spans_processed > 0 { 
            processing_errors as f64 / spans_processed as f64 
        } else { 
            0.0 
        };
        
        // Find oldest and newest spans
        let span_order = self.span_order.read().await;
        let oldest_span = span_order.front().map(|(t, _)| *t);
        let newest_span = span_order.back().map(|(t, _)| *t);
        
        StorageStats {
            trace_count,
            span_count,
            service_count,
            memory_bytes,
            memory_mb,
            memory_pressure,
            oldest_span,
            newest_span,
            processing_rate,
            error_rate,
            cleanup_count: self.counters.cleanup_operations.load(Ordering::Relaxed),
            last_cleanup: Some(SystemTime::now()), // Approximate
            health_status: self.get_health_status(),
            uptime_seconds: self.counters.start_time.elapsed().as_secs(),
        }
    }
}

#[async_trait::async_trait]
impl StorageBackend for InMemoryStorage {
    async fn store_span(&self, span: Span) -> Result<()> {
        // Increment processing counter
        self.counters.spans_processed.fetch_add(1, Ordering::Relaxed);
        
        let span_id = span.span_id.clone();
        let trace_id = span.trace_id.clone();
        let service_name = span.service_name.clone();
        let start_time = span.start_time;
        
        // Estimate memory for this span
        let span_memory = self.estimate_span_memory(&span);
        
        // Check memory pressure and perform cleanup if needed
        let memory_pressure = self.get_memory_pressure();
        if memory_pressure >= self.cleanup_config.warning_threshold || self.should_cleanup().await {
            if memory_pressure >= self.cleanup_config.emergency_threshold {
                // Emergency: drop new spans if at emergency threshold
                self.counters.processing_errors.fetch_add(1, Ordering::Relaxed);
                return Err(crate::core::UrpoError::Storage(
                    "Storage at emergency capacity, dropping span".to_string()
                ));
            } else if memory_pressure >= self.cleanup_config.critical_threshold {
                // Critical: aggressive cleanup
                let _ = self.emergency_cleanup_internal().await;
                *self.last_cleanup.lock().await = Instant::now();
            } else {
                // Warning: regular cleanup
                let to_evict = (self.max_spans / 20).max(10); // Evict 5% when at warning
                self.evict_oldest_spans(to_evict).await;
                *self.last_cleanup.lock().await = Instant::now();
            }
        }
        
        // Check if we need to evict spans before storing
        if self.spans.len() >= self.max_spans {
            let to_evict = (self.max_spans / 10).max(1); // Evict 10% when at capacity
            self.evict_oldest_spans(to_evict).await;
        }

        // Store the span
        self.spans.insert(span_id.clone(), span);
        
        // Update memory tracking
        self.counters.memory_bytes.fetch_add(span_memory, Ordering::Relaxed);

        // Update trace index
        self.traces
            .entry(trace_id)
            .or_insert_with(Vec::new)
            .push(span_id.clone());

        // Update service index with timestamp for efficient time-based queries
        self.services
            .entry(service_name.clone())
            .or_insert_with(VecDeque::new)
            .push_back((start_time, span_id.clone()));

        // Add to span order for LRU eviction
        {
            let mut span_order = self.span_order.write().await;
            span_order.push_back((start_time, span_id));
        }
        
        // Update active services tracking
        self.active_services.write().await.insert(service_name, start_time);

        // Enforce per-service limits
        self.enforce_service_limits().await;

        Ok(())
    }

    async fn get_span(&self, span_id: &SpanId) -> Result<Option<Span>> {
        Ok(self.spans.get(span_id).map(|entry| entry.clone()))
    }

    async fn get_trace_spans(&self, trace_id: &TraceId) -> Result<Vec<Span>> {
        if let Some(span_ids) = self.traces.get(trace_id) {
            let mut spans = Vec::with_capacity(span_ids.len());
            for span_id in span_ids.iter() {
                if let Some(span) = self.spans.get(span_id) {
                    spans.push(span.clone());
                }
            }
            // Sort by start time
            spans.sort_by_key(|s| s.start_time);
            Ok(spans)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_service_spans(
        &self,
        service: &ServiceName,
        since: SystemTime,
    ) -> Result<Vec<Span>> {
        if let Some(service_spans) = self.services.get(service) {
            let mut spans = Vec::new();
            for (timestamp, span_id) in service_spans.iter() {
                if *timestamp >= since {
                    if let Some(span) = self.spans.get(span_id) {
                        spans.push(span.clone());
                    }
                }
            }
            Ok(spans)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_service_metrics(&self) -> Result<Vec<ServiceMetrics>> {
        // Delegate to the aggregator module
        aggregator::calculate_service_metrics(self).await
    }

    async fn get_span_count(&self) -> Result<usize> {
        Ok(self.spans.len())
    }

    async fn enforce_limits(&self) -> Result<usize> {
        let current_count = self.spans.len();
        if current_count > self.max_spans {
            let to_remove = current_count - self.max_spans;
            Ok(self.evict_oldest_spans(to_remove).await)
        } else {
            Ok(0)
        }
    }
    
    async fn list_services(&self) -> Result<Vec<ServiceName>> {
        Ok(self.list_active_services().await)
    }
    
    async fn get_storage_stats(&self) -> Result<StorageStats> {
        Ok(self.get_detailed_stats().await)
    }
    
    async fn emergency_cleanup(&self) -> Result<usize> {
        self.emergency_cleanup_internal().await
    }
    
    fn get_health(&self) -> StorageHealth {
        self.get_health_status()
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    async fn list_recent_traces(&self, limit: usize, service_filter: Option<&ServiceName>) -> Result<Vec<TraceInfo>> {
        let mut trace_infos = Vec::new();
        
        // Collect trace information
        for entry in self.traces.iter() {
            let trace_id = entry.key().clone();
            let span_ids = entry.value();
            
            if span_ids.is_empty() {
                continue;
            }
            
            // Get all spans for this trace
            let mut spans = Vec::new();
            let mut services = std::collections::HashSet::new();
            let mut has_error = false;
            
            for span_id in span_ids.iter() {
                if let Some(span) = self.spans.get(span_id) {
                    services.insert(span.service_name.clone());
                    if span.status.is_error() {
                        has_error = true;
                    }
                    spans.push(span.clone());
                }
            }
            
            if spans.is_empty() {
                continue;
            }
            
            // Find root span (no parent)
            // SAFE: Already checked spans.is_empty() above
            let root_span = spans.iter()
                .find(|s| s.parent_span_id.is_none())
                .or_else(|| spans.first())
                .expect("spans not empty");
            
            // Apply service filter if provided
            if let Some(filter) = service_filter {
                if !services.contains(filter) {
                    continue;
                }
            }
            
            // Calculate total duration (from earliest start to latest end)
            // SAFE: Already checked spans.is_empty() above
            let min_start = spans.iter().map(|s| s.start_time).min()
                .expect("spans not empty");
            let max_end = spans.iter()
                .map(|s| s.start_time + s.duration)
                .max()
                .expect("spans not empty");
            let duration = max_end.duration_since(min_start)
                .unwrap_or_else(|_| Duration::ZERO);
            
            trace_infos.push(TraceInfo {
                trace_id,
                root_service: root_span.service_name.clone(),
                root_operation: root_span.operation_name.clone(),
                span_count: spans.len(),
                duration,
                start_time: min_start,
                has_error,
                services: services.into_iter().collect(),
            });
        }
        
        // Sort by start time (most recent first)
        trace_infos.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        
        // Limit results
        trace_infos.truncate(limit);
        
        Ok(trace_infos)
    }
    
    async fn search_traces(&self, query: &str, limit: usize) -> Result<Vec<TraceInfo>> {
        let query_lower = query.to_lowercase();
        let mut matching_traces = Vec::new();
        
        for entry in self.traces.iter() {
            let trace_id = entry.key();
            let span_ids = entry.value();
            
            let mut match_found = false;
            let mut spans = Vec::new();
            let mut services = std::collections::HashSet::new();
            let mut has_error = false;
            
            // Check if any span in the trace matches the query
            for span_id in span_ids.iter() {
                if let Some(span) = self.spans.get(span_id) {
                    services.insert(span.service_name.clone());
                    if span.status.is_error() {
                        has_error = true;
                    }
                    
                    // Search in operation name and attributes
                    if span.operation_name.to_lowercase().contains(&query_lower) {
                        match_found = true;
                    }
                    
                    for (key, value) in &span.attributes {
                        if key.to_lowercase().contains(&query_lower) || 
                           value.to_lowercase().contains(&query_lower) {
                            match_found = true;
                            break;
                        }
                    }
                    
                    for (key, value) in &span.tags {
                        if key.to_lowercase().contains(&query_lower) || 
                           value.to_lowercase().contains(&query_lower) {
                            match_found = true;
                            break;
                        }
                    }
                    
                    spans.push(span.clone());
                }
            }
            
            if match_found && !spans.is_empty() {
                let root_span = spans.iter()
                    .find(|s| s.parent_span_id.is_none())
                    .or_else(|| spans.first())
                    .expect("spans not empty");
                
                let min_start = spans.iter().map(|s| s.start_time).min().expect("spans not empty");
                let max_end = spans.iter()
                    .map(|s| s.start_time + s.duration)
                    .max()
                    .expect("spans not empty");
                let duration = max_end.duration_since(min_start)
                .unwrap_or_else(|_| Duration::ZERO);
                
                matching_traces.push(TraceInfo {
                    trace_id: trace_id.clone(),
                    root_service: root_span.service_name.clone(),
                    root_operation: root_span.operation_name.clone(),
                    span_count: spans.len(),
                    duration,
                    start_time: min_start,
                    has_error,
                    services: services.into_iter().collect(),
                });
            }
        }
        
        // Sort by start time (most recent first)
        matching_traces.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        matching_traces.truncate(limit);
        
        Ok(matching_traces)
    }
    
    async fn get_error_traces(&self, limit: usize) -> Result<Vec<TraceInfo>> {
        let mut error_traces = Vec::new();
        
        for entry in self.traces.iter() {
            let trace_id = entry.key();
            let span_ids = entry.value();
            
            let mut has_error = false;
            let mut spans = Vec::new();
            let mut services = std::collections::HashSet::new();
            
            for span_id in span_ids.iter() {
                if let Some(span) = self.spans.get(span_id) {
                    services.insert(span.service_name.clone());
                    if span.status.is_error() {
                        has_error = true;
                    }
                    spans.push(span.clone());
                }
            }
            
            if has_error && !spans.is_empty() {
                let root_span = spans.iter()
                    .find(|s| s.parent_span_id.is_none())
                    .or_else(|| spans.first())
                    .expect("spans not empty");
                
                let min_start = spans.iter().map(|s| s.start_time).min().expect("spans not empty");
                let max_end = spans.iter()
                    .map(|s| s.start_time + s.duration)
                    .max()
                    .expect("spans not empty");
                let duration = max_end.duration_since(min_start)
                .unwrap_or_else(|_| Duration::ZERO);
                
                error_traces.push(TraceInfo {
                    trace_id: trace_id.clone(),
                    root_service: root_span.service_name.clone(),
                    root_operation: root_span.operation_name.clone(),
                    span_count: spans.len(),
                    duration,
                    start_time: min_start,
                    has_error: true,
                    services: services.into_iter().collect(),
                });
            }
        }
        
        // Sort by start time (most recent first)
        error_traces.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        error_traces.truncate(limit);
        
        Ok(error_traces)
    }
    
    async fn get_slow_traces(&self, threshold: Duration, limit: usize) -> Result<Vec<TraceInfo>> {
        let mut slow_traces = Vec::new();
        
        for entry in self.traces.iter() {
            let trace_id = entry.key();
            let span_ids = entry.value();
            
            let mut spans = Vec::new();
            let mut services = std::collections::HashSet::new();
            let mut has_error = false;
            
            for span_id in span_ids.iter() {
                if let Some(span) = self.spans.get(span_id) {
                    services.insert(span.service_name.clone());
                    if span.status.is_error() {
                        has_error = true;
                    }
                    spans.push(span.clone());
                }
            }
            
            if spans.is_empty() {
                continue;
            }
            
            let root_span = spans.iter()
                .find(|s| s.parent_span_id.is_none())
                .or_else(|| spans.first())
                .expect("spans not empty");
            
            let min_start = spans.iter().map(|s| s.start_time).min().expect("spans not empty");
            let max_end = spans.iter()
                .map(|s| s.start_time + s.duration)
                .max()
                .expect("spans not empty");
            let duration = max_end.duration_since(min_start)
                .unwrap_or_else(|_| Duration::ZERO);
            
            if duration >= threshold {
                slow_traces.push(TraceInfo {
                    trace_id: trace_id.clone(),
                    root_service: root_span.service_name.clone(),
                    root_operation: root_span.operation_name.clone(),
                    span_count: spans.len(),
                    duration,
                    start_time: min_start,
                    has_error,
                    services: services.into_iter().collect(),
                });
            }
        }
        
        // Sort by duration (slowest first)
        slow_traces.sort_by(|a, b| b.duration.cmp(&a.duration));
        slow_traces.truncate(limit);
        
        Ok(slow_traces)
    }
    
    async fn list_traces(
        &self,
        service: Option<&str>,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: usize,
    ) -> Result<Vec<TraceInfo>> {
        let mut matching_traces = Vec::new();
        
        for entry in self.traces.iter() {
            let trace_id = entry.key();
            let span_ids = entry.value();
            
            let mut spans = Vec::new();
            let mut services = std::collections::HashSet::new();
            let mut has_error = false;
            let mut match_found = true;
            
            for span_id in span_ids.iter() {
                if let Some(span) = self.spans.get(span_id) {
                    // Apply service filter
                    if let Some(svc) = service {
                        if span.service_name.as_str() != svc {
                            match_found = false;
                            break;
                        }
                    }
                    
                    // Apply time filter
                    if let Some(start) = start_time {
                        let span_time = span.start_time.duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default().as_nanos() as u64;
                        if span_time < start {
                            match_found = false;
                            break;
                        }
                    }
                    
                    if let Some(end) = end_time {
                        let span_time = span.start_time.duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default().as_nanos() as u64;
                        if span_time > end {
                            match_found = false;
                            break;
                        }
                    }
                    
                    services.insert(span.service_name.clone());
                    if span.status.is_error() {
                        has_error = true;
                    }
                    spans.push(span.clone());
                }
            }
            
            if match_found && !spans.is_empty() {
                let root_span = spans.iter()
                    .find(|s| s.parent_span_id.is_none())
                    .or_else(|| spans.first())
                    .expect("spans not empty");
                
                let min_start = spans.iter().map(|s| s.start_time).min().expect("spans not empty");
                let max_end = spans.iter()
                    .map(|s| s.start_time + s.duration)
                    .max()
                    .expect("spans not empty");
                let duration = max_end.duration_since(min_start)
                .unwrap_or_else(|_| Duration::ZERO);
                
                matching_traces.push(TraceInfo {
                    trace_id: trace_id.clone(),
                    root_service: root_span.service_name.clone(),
                    root_operation: root_span.operation_name.clone(),
                    span_count: spans.len(),
                    duration,
                    start_time: min_start,
                    has_error,
                    services: services.into_iter().collect(),
                });
            }
        }
        
        // Sort by start time (most recent first)
        matching_traces.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        matching_traces.truncate(limit);
        
        Ok(matching_traces)
    }
    
    async fn get_service_metrics_map(&self) -> Result<HashMap<ServiceName, ServiceMetrics>> {
        let metrics = self.get_service_metrics().await?;
        let mut map = HashMap::new();
        for metric in metrics {
            map.insert(metric.name.clone(), metric);
        }
        Ok(map)
    }
    
    async fn search_spans(
        &self,
        query: &str,
        service: Option<&str>,
        attribute_key: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Span>> {
        let mut matching_spans = Vec::new();
        let query_lower = query.to_lowercase();
        
        for entry in self.spans.iter() {
            let span = entry.value();
            
            // Apply service filter
            if let Some(svc) = service {
                if span.service_name.as_str() != svc {
                    continue;
                }
            }
            
            // Search in operation name
            let mut match_found = false;
            if span.operation_name.to_lowercase().contains(&query_lower) {
                match_found = true;
            }
            
            // Search in attributes
            if !match_found {
                for (key, value) in &span.attributes {
                    // Apply attribute key filter
                    if let Some(attr_key) = attribute_key {
                        if key != attr_key {
                            continue;
                        }
                    }
                    
                    if key.to_lowercase().contains(&query_lower) || 
                       value.to_lowercase().contains(&query_lower) {
                        match_found = true;
                        break;
                    }
                }
            }
            
            if match_found {
                matching_spans.push(span.clone());
                if matching_spans.len() >= limit {
                    break;
                }
            }
        }
        
        Ok(matching_spans)
    }
    
    async fn get_stats(&self) -> Result<StorageStats> {
        let detailed_stats = self.get_storage_stats().await?;
        Ok(StorageStats {
            trace_count: detailed_stats.trace_count,
            span_count: detailed_stats.span_count,
            service_count: detailed_stats.service_count,
            memory_bytes: detailed_stats.memory_bytes,
            memory_mb: detailed_stats.memory_mb,
            memory_pressure: detailed_stats.memory_pressure,
            oldest_span: detailed_stats.oldest_span,
            newest_span: detailed_stats.newest_span,
            processing_rate: detailed_stats.processing_rate,
            error_rate: detailed_stats.error_rate,
            cleanup_count: detailed_stats.cleanup_count,
            last_cleanup: detailed_stats.last_cleanup,
            health_status: detailed_stats.health_status,
            uptime_seconds: self.counters.start_time.elapsed().as_secs(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn create_test_span(trace_num: u32, span_num: u32, service: &str) -> Span {
        Span::builder()
            .trace_id(TraceId::new(format!("trace_{:04}", trace_num)).unwrap())
            .span_id(SpanId::new(format!("span_{:04}", span_num)).unwrap())
            .service_name(ServiceName::new(service.to_string()).unwrap())
            .operation_name(format!("operation_{}", span_num))
            .start_time(SystemTime::now())
            .duration(Duration::from_millis(100))
            .status(crate::core::SpanStatus::Ok)
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_store_and_retrieve_span() {
        let storage = InMemoryStorage::new(100);
        let span = create_test_span(1, 1, "test-service").await;
        let span_id = span.span_id.clone();

        storage.store_span(span.clone()).await.unwrap();
        
        let retrieved = storage.get_span(&span_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().span_id, span_id);
    }

    #[tokio::test]
    async fn test_get_trace_spans() {
        let storage = InMemoryStorage::new(100);
        let trace_id = TraceId::new("trace_0001".to_string()).unwrap();
        
        for i in 1..=3 {
            let mut span = create_test_span(1, i, "test-service").await;
            span.trace_id = trace_id.clone();
            storage.store_span(span).await.unwrap();
        }

        let spans = storage.get_trace_spans(&trace_id).await.unwrap();
        assert_eq!(spans.len(), 3);
    }

    #[tokio::test]
    async fn test_storage_limits() {
        let storage = InMemoryStorage::new(5); // Max 5 spans
        
        for i in 1..=10 {
            let span = create_test_span(i, i, "test-service").await;
            storage.store_span(span).await.unwrap();
        }
        
        // Should have enforced limit
        assert!(storage.spans.len() <= 5);
    }

    #[tokio::test]
    async fn test_service_spans_time_filter() {
        let storage = InMemoryStorage::new(100);
        let service_name = ServiceName::new("test-service".to_string()).unwrap();
        
        // Store some spans
        for i in 1..=5 {
            let span = create_test_span(i, i, "test-service").await;
            storage.store_span(span).await.unwrap();
        }
        
        // Query spans from now (should get all)
        let since = SystemTime::now() - Duration::from_secs(60);
        let spans = storage.get_service_spans(&service_name, since).await.unwrap();
        assert_eq!(spans.len(), 5);
        
        // Query spans from future (should get none)
        let future = SystemTime::now() + Duration::from_secs(60);
        let spans = storage.get_service_spans(&service_name, future).await.unwrap();
        assert_eq!(spans.len(), 0);
    }
}
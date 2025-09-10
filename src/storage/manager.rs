//! Storage manager for coordinating storage operations.

use super::{
    StorageBackend, StorageStats, StorageHealth, TraceInfo, InMemoryStorage,
    archive, archive_manager,
};
use super::engine::{StorageEngine, StorageMode, StorageStats as EngineStorageStats};
use crate::core::{Config, Result, ServiceMetrics, ServiceName, Span, TraceId};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH, Instant};
use tokio::sync::RwLock;

/// Storage manager for coordinating storage operations.
pub struct StorageManager {
    backend: Arc<dyn StorageBackend>,
    /// Persistent storage engine (optional)
    persistent_engine: Option<Arc<RwLock<StorageEngine>>>,
    /// Archive manager for long-term compressed storage (optional)
    archive_manager: Option<Arc<RwLock<archive_manager::ArchiveManager>>>,
    /// Creation time for uptime tracking
    start_time: Instant,
}

impl StorageManager {
    /// Create a new storage manager with in-memory backend.
    pub fn new_in_memory(max_spans: usize) -> Self {
        let backend = Arc::new(InMemoryStorage::new(max_spans));
        Self { 
            backend,
            persistent_engine: None,
            archive_manager: None,
            start_time: Instant::now(),
        }
    }
    
    /// Create a new storage manager with persistent backend.
    pub fn new_persistent(config: &Config) -> Result<Self> {
        let backend = Arc::new(InMemoryStorage::with_config(config));
        
        // Create persistent storage engine
        let storage_mode = if config.storage.persistent {
            StorageMode::Persistent {
                hot_size: config.storage.max_spans / 10,  // 10% in hot ring
                warm_path: config.storage.data_dir.join("warm"),
                cold_path: config.storage.data_dir.join("cold"),
            }
        } else {
            StorageMode::InMemory {
                max_traces: config.storage.max_spans / 100, // Avg 100 spans per trace
            }
        };
        
        let engine = StorageEngine::new(storage_mode)?;
        
        // Create archive manager if archival is enabled
        let archive_manager = if config.storage.enable_archival {
            let archive_config = archive_manager::ArchiveConfig {
                archive_dir: config.storage.data_dir.join("archives"),
                granularity: archive::PartitionGranularity::Daily,
                max_traces_per_partition: 100_000,
                retention_period: Duration::from_secs(90 * 24 * 3600), // 90 days
                ..Default::default()
            };
            
            let mut manager = archive_manager::ArchiveManager::new(archive_config)?;
            manager.start()?;
            Some(Arc::new(RwLock::new(manager)))
        } else {
            None
        };

        Ok(Self {
            backend,
            persistent_engine: Some(Arc::new(RwLock::new(engine))),
            archive_manager,
            start_time: Instant::now(),
        })
    }

    /// Get the storage backend.
    pub fn backend(&self) -> Arc<dyn StorageBackend> {
        self.backend.clone()
    }
    
    /// Store a span.
    pub async fn store_span(&self, span: Span) -> Result<()> {
        // Store in main backend
        self.backend.store_span(span.clone()).await?;
        
        // Also store in persistent engine if enabled
        if let Some(engine) = &self.persistent_engine {
            let engine = engine.read().await;
            engine.ingest_span(span)?;
        }
        
        Ok(())
    }

    /// Get service metrics.
    pub async fn get_service_metrics(&self) -> Result<Vec<ServiceMetrics>> {
        self.backend.get_service_metrics().await
    }

    /// Run periodic cleanup to enforce storage limits.
    pub async fn run_cleanup(&self) -> Result<()> {
        let removed = self.backend.enforce_limits().await?;
        if removed > 0 {
            tracing::debug!("Cleaned up {} old spans", removed);
        }
        
        // Also trigger tier migration in persistent engine
        if let Some(engine) = &self.persistent_engine {
            let mut engine = engine.write().await;
            engine.migrate_tiers()?;
        }
        
        Ok(())
    }
    
    /// Query traces from persistent storage.
    pub async fn query_persistent_traces(
        &self,
        service: Option<&ServiceName>,
        start_time: Option<SystemTime>,
        end_time: Option<SystemTime>,
        limit: usize,
    ) -> Result<Vec<TraceInfo>> {
        if let Some(engine) = &self.persistent_engine {
            let engine = engine.read().await;
            let traces = engine.query_traces(
                service.map(|s| s.as_str()),
                start_time.map(|t| t.duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64),
                end_time.map(|t| t.duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64),
                limit,
            )?;
            
            // Convert to TraceInfo
            let mut trace_infos = Vec::new();
            for trace_id in traces {
                if let Ok(spans) = self.backend.get_trace_spans(&TraceId::new(format!("{:032x}", trace_id)).unwrap()).await {
                    if !spans.is_empty() {
                        let root_span = spans.iter()
                            .find(|s| s.parent_span_id.is_none())
                            .or_else(|| spans.first())
                            .unwrap();
                        
                        let min_start = spans.iter().map(|s| s.start_time).min().unwrap();
                        let max_end = spans.iter()
                            .map(|s| s.start_time + s.duration)
                            .max()
                            .unwrap();
                        let duration = max_end.duration_since(min_start).unwrap_or(Duration::ZERO);
                        
                        let services: HashSet<_> = spans.iter()
                            .map(|s| s.service_name.clone())
                            .collect();
                        
                        let has_error = spans.iter().any(|s| s.status.is_error());
                        
                        trace_infos.push(TraceInfo {
                            trace_id: TraceId::new(format!("{:032x}", trace_id)).unwrap(),
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
            }
            
            Ok(trace_infos)
        } else {
            // Fallback to in-memory backend
            self.backend.list_recent_traces(limit, service).await
        }
    }
    
    /// Get storage statistics including persistent storage.
    pub async fn get_full_stats(&self) -> Result<(StorageStats, Option<EngineStorageStats>)> {
        let backend_stats = self.get_stats().await?;
        
        let engine_stats = if let Some(engine) = &self.persistent_engine {
            let engine = engine.read().await;
            Some(engine.get_stats())
        } else {
            None
        };
        
        Ok((backend_stats, engine_stats))
    }

    /// Get comprehensive storage statistics.
    pub async fn get_stats(&self) -> Result<StorageStats> {
        // Delegate to the backend if it's InMemoryStorage
        if let Some(in_memory) = self.backend.as_any().downcast_ref::<InMemoryStorage>() {
            Ok(in_memory.get_detailed_stats().await)
        } else {
            // Fallback for other storage backends
            let span_count = self.backend.get_span_count().await?;
            let avg_span_size = 1024; // bytes per span
            let memory_bytes = span_count * avg_span_size;
            
            Ok(StorageStats {
                trace_count: 0,
                span_count,
                service_count: 0,
                memory_bytes,
                memory_mb: memory_bytes as f64 / 1024.0 / 1024.0,
                memory_pressure: 0.0,
                oldest_span: None,
                newest_span: None,
                processing_rate: 0.0,
                error_rate: 0.0,
                cleanup_count: 0,
                last_cleanup: None,
                health_status: StorageHealth::Healthy,
                uptime_seconds: self.start_time.elapsed().as_secs(),
            })
        }
    }
}
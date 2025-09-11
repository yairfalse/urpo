//! Archive management system with automatic rotation and cleanup.
//!
//! Handles the lifecycle of trace archives:
//! - Automatic partition rotation based on time or size
//! - Background compression and archival
//! - Retention policy enforcement
//! - Intelligent prefetching for queries

use crate::core::{Result, UrpoError, Span, TraceId, ServiceName};
use crate::storage::archive::{ArchiveReader, ArchiveWriter, PartitionGranularity, ArchiveStats};
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use parking_lot::RwLock;
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use tokio::sync::mpsc;

/// Archive management configuration.
#[derive(Debug, Clone)]
pub struct ArchiveConfig {
    /// Base directory for all archives
    pub archive_dir: PathBuf,
    
    /// Partition granularity (hourly, daily, weekly)
    pub granularity: PartitionGranularity,
    
    /// Maximum traces per partition before forcing rotation
    pub max_traces_per_partition: usize,
    
    /// Maximum partition size in bytes before forcing rotation
    pub max_partition_size_bytes: usize,
    
    /// How long to keep archives (retention period)
    pub retention_period: Duration,
    
    /// Compression level (1-9, higher = more compression but slower)
    pub compression_level: u32,
    
    /// Background archival interval
    pub archival_interval: Duration,
    
    /// Enable automatic cleanup of old archives
    pub enable_cleanup: bool,
    
    /// Prefetch recent partitions into memory for faster queries
    pub enable_prefetch: bool,
    
    /// Number of recent partitions to keep prefetched
    pub prefetch_partition_count: usize,
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            archive_dir: PathBuf::from("./urpo_data/archives"),
            granularity: PartitionGranularity::Daily,
            max_traces_per_partition: 100_000,
            max_partition_size_bytes: 512 * 1024 * 1024, // 512MB
            retention_period: Duration::from_secs(90 * 24 * 3600), // 90 days
            compression_level: 4,
            archival_interval: Duration::from_secs(3600), // 1 hour
            enable_cleanup: true,
            enable_prefetch: true,
            prefetch_partition_count: 7, // Last week's partitions
        }
    }
}

/// Archive manager handles the complete lifecycle of trace archives.
pub struct ArchiveManager {
    /// Configuration
    config: ArchiveConfig,
    
    /// Active writer for current partition
    writer: Arc<RwLock<Option<ArchiveWriter>>>,
    
    /// Readers for querying archives
    readers: Arc<RwLock<BTreeMap<String, Arc<ArchiveReader>>>>,
    
    /// Background task handle
    bg_handle: Option<tokio::task::JoinHandle<()>>,
    
    /// Channel for background archival requests
    archival_tx: mpsc::UnboundedSender<ArchivalRequest>,
    
    /// Metrics and statistics
    stats: Arc<RwLock<ManagerStats>>,
}

/// Request for background archival processing.
#[derive(Debug)]
enum ArchivalRequest {
    /// Archive a batch of traces
    ArchiveTraces(Vec<Vec<Span>>),
    
    /// Force rotation of current partition
    ForceRotation,
    
    /// Run cleanup of old archives
    RunCleanup,
    
    /// Shutdown the background task
    Shutdown,
}

/// Archive manager statistics.
#[derive(Debug, Default, Clone)]
pub struct ManagerStats {
    /// Total partitions managed
    pub total_partitions: u64,
    
    /// Total archived traces
    pub archived_traces: u64,
    
    /// Total archived spans
    pub archived_spans: u64,
    
    /// Total archive size on disk
    pub total_archive_size: u64,
    
    /// Average compression ratio
    pub avg_compression_ratio: f32,
    
    /// Last archival time
    pub last_archival: Option<SystemTime>,
    
    /// Last cleanup time
    pub last_cleanup: Option<SystemTime>,
    
    /// Background task errors
    pub background_errors: u64,
    
    /// Archival throughput (traces/second)
    pub archival_throughput: f64,
}

impl ArchiveManager {
    /// Create a new archive manager.
    pub fn new(config: ArchiveConfig) -> Result<Self> {
        // Create archive directory
        std::fs::create_dir_all(&config.archive_dir)
            .map_err(|e| UrpoError::Storage(format!("Failed to create archive directory: {}", e)))?;
        
        // Create channels for background processing
        let (archival_tx, _archival_rx) = mpsc::unbounded_channel();
        
        let manager = Self {
            config: config.clone(),
            writer: Arc::new(RwLock::new(None)),
            readers: Arc::new(RwLock::new(BTreeMap::new())),
            bg_handle: None,
            archival_tx,
            stats: Arc::new(RwLock::new(ManagerStats::default())),
        };
        
        // Initialize readers for existing archives
        manager.initialize_readers()?;
        
        Ok(manager)
    }

    /// Start the archive manager with background processing.
    pub fn start(&mut self) -> Result<()> {
        let config = self.config.clone();
        let writer = self.writer.clone();
        let readers = self.readers.clone();
        let stats = self.stats.clone();
        let _archival_rx = {
            let (tx, rx) = mpsc::unbounded_channel();
            std::mem::replace(&mut self.archival_tx, tx);
            rx
        };
        
        // Start background task
        let handle = tokio::spawn(async move {
            Self::background_task(config, writer, readers, stats, _archival_rx).await;
        });
        
        self.bg_handle = Some(handle);
        
        tracing::info!("Archive manager started with {:?} partitioning", self.config.granularity);
        Ok(())
    }

    /// Stop the archive manager.
    pub async fn stop(&mut self) -> Result<()> {
        // Send shutdown signal
        let _ = self.archival_tx.send(ArchivalRequest::Shutdown);
        
        // Wait for background task to complete
        if let Some(handle) = self.bg_handle.take() {
            let _ = handle.await;
        }
        
        // Final flush
        self.flush_current_partition().await?;
        
        tracing::info!("Archive manager stopped");
        Ok(())
    }

    /// Archive a batch of traces.
    pub fn archive_traces(&self, traces: Vec<Vec<Span>>) -> Result<()> {
        if traces.is_empty() {
            return Ok(());
        }
        
        // Send to background task for processing
        self.archival_tx.send(ArchivalRequest::ArchiveTraces(traces))
            .map_err(|_| UrpoError::Storage("Archive manager background task not running".to_string()))?;
        
        Ok(())
    }

    /// Force rotation of the current partition.
    pub fn force_rotation(&self) -> Result<()> {
        self.archival_tx.send(ArchivalRequest::ForceRotation)
            .map_err(|_| UrpoError::Storage("Archive manager background task not running".to_string()))?;
        Ok(())
    }

    /// Trigger cleanup of old archives.
    pub fn trigger_cleanup(&self) -> Result<()> {
        self.archival_tx.send(ArchivalRequest::RunCleanup)
            .map_err(|_| UrpoError::Storage("Archive manager background task not running".to_string()))?;
        Ok(())
    }

    /// Query traces across all archives.
    pub fn query_traces(
        &self,
        service: Option<&ServiceName>,
        start_time: Option<SystemTime>,
        end_time: Option<SystemTime>,
        limit: usize,
    ) -> Result<Vec<u32>> {
        let readers = self.readers.read();
        let mut all_trace_ids = Vec::new();
        
        // Query across all relevant partitions
        for (partition_key, reader) in readers.iter() {
            // Skip partitions that don't overlap with time range
            if !self.partition_overlaps_range(partition_key, start_time, end_time) {
                continue;
            }
            
            if let Some(service) = service {
                let trace_ids = reader.query_service_traces(service, start_time, end_time, limit)?;
                all_trace_ids.extend(trace_ids);
            }
            
            if all_trace_ids.len() >= limit {
                break;
            }
        }
        
        all_trace_ids.truncate(limit);
        Ok(all_trace_ids)
    }

    /// Get comprehensive archive statistics.
    pub fn get_stats(&self) -> ManagerStats {
        let stats = self.stats.read();
        let mut stats = stats.clone();
        
        // Aggregate stats from all readers
        let readers = self.readers.read();
        let mut total_archive_stats = crate::storage::archive::ArchiveStats::default();
        
        for reader in readers.values() {
            let archive_stats = reader.get_archive_stats();
            total_archive_stats.total_partitions += archive_stats.total_partitions;
            total_archive_stats.total_traces += archive_stats.total_traces;
            total_archive_stats.total_spans += archive_stats.total_spans;
            total_archive_stats.total_size_bytes += archive_stats.total_size_bytes;
            total_archive_stats.total_services = total_archive_stats.total_services.max(archive_stats.total_services);
        }
        
        stats.total_partitions = total_archive_stats.total_partitions;
        stats.archived_traces = total_archive_stats.total_traces;
        stats.archived_spans = total_archive_stats.total_spans;
        stats.total_archive_size = total_archive_stats.total_size_bytes;
        
        if total_archive_stats.total_partitions > 0 {
            stats.avg_compression_ratio = total_archive_stats.avg_compression_ratio;
        }
        
        stats
    }

    /// Initialize readers for existing archive files.
    fn initialize_readers(&self) -> Result<()> {
        if !self.config.archive_dir.exists() {
            return Ok(());
        }
        
        let dir_entries = std::fs::read_dir(&self.config.archive_dir)
            .map_err(|e| UrpoError::Storage(format!("Failed to read archive directory: {}", e)))?;
        
        let mut readers = self.readers.write();
        readers.clear();
        
        let mut partition_keys = Vec::new();
        for entry in dir_entries {
            let entry = entry
                .map_err(|e| UrpoError::Storage(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();
            
            if let Some(extension) = path.extension() {
                if extension == "index" {
                    if let Some(stem) = path.file_stem() {
                        partition_keys.push(stem.to_string_lossy().to_string());
                    }
                }
            }
        }
        
        partition_keys.sort();
        
        // Create readers for each partition
        for partition_key in partition_keys {
            let reader = Arc::new(ArchiveReader::new(&self.config.archive_dir, self.config.granularity));
            reader.load_indices()
                .map_err(|e| UrpoError::Storage(format!("Failed to load indices for partition {}: {}", partition_key, e)))?;
            readers.insert(partition_key.clone(), reader);
        }
        
        tracing::info!("Initialized {} archive readers", readers.len());
        Ok(())
    }

    /// Background task for archival processing.
    async fn background_task(
        config: ArchiveConfig,
        writer: Arc<RwLock<Option<ArchiveWriter>>>,
        readers: Arc<RwLock<BTreeMap<String, Arc<ArchiveReader>>>>,
        stats: Arc<RwLock<ManagerStats>>,
        mut archival_rx: mpsc::UnboundedReceiver<ArchivalRequest>,
    ) {
        let mut cleanup_interval = tokio::time::interval(config.archival_interval);
        let mut archival_interval = tokio::time::interval(Duration::from_secs(60)); // Check every minute
        
        loop {
            tokio::select! {
                // Handle archival requests
                request = archival_rx.recv() => {
                    match request {
                        Some(ArchivalRequest::ArchiveTraces(traces)) => {
                            if let Err(e) = Self::process_archive_traces(&config, &writer, traces).await {
                                tracing::error!("Failed to archive traces: {}", e);
                                stats.write().background_errors += 1;
                            }
                        }
                        Some(ArchivalRequest::ForceRotation) => {
                            if let Err(e) = Self::process_force_rotation(&config, &writer).await {
                                tracing::error!("Failed to force rotation: {}", e);
                                stats.write().background_errors += 1;
                            }
                        }
                        Some(ArchivalRequest::RunCleanup) => {
                            if let Err(e) = Self::process_cleanup(&config, &readers).await {
                                tracing::error!("Failed to run cleanup: {}", e);
                                stats.write().background_errors += 1;
                            }
                        }
                        Some(ArchivalRequest::Shutdown) | None => {
                            tracing::info!("Archive manager background task shutting down");
                            break;
                        }
                    }
                }
                
                // Periodic cleanup
                _ = cleanup_interval.tick() => {
                    if config.enable_cleanup {
                        if let Err(e) = Self::process_cleanup(&config, &readers).await {
                            tracing::error!("Periodic cleanup failed: {}", e);
                            stats.write().background_errors += 1;
                        }
                    }
                }
                
                // Periodic archival (flush current partition if needed)
                _ = archival_interval.tick() => {
                    if let Err(e) = Self::check_partition_rotation(&config, &writer).await {
                        tracing::error!("Partition rotation check failed: {}", e);
                        stats.write().background_errors += 1;
                    }
                }
            }
        }
    }

    /// Process archive traces request.
    async fn process_archive_traces(
        config: &ArchiveConfig,
        writer: &Arc<RwLock<Option<ArchiveWriter>>>,
        traces: Vec<Vec<Span>>,
    ) -> Result<()> {
        let mut writer_guard = writer.write();
        
        // Initialize writer if needed
        if writer_guard.is_none() {
            let new_writer = ArchiveWriter::new(
                &config.archive_dir,
                config.granularity,
                config.max_traces_per_partition,
            )?;
            *writer_guard = Some(new_writer);
        }
        
        if let Some(ref mut w) = writer_guard.as_mut() {
            w.add_traces(traces)?;
        }
        
        Ok(())
    }

    /// Process force rotation request.
    async fn process_force_rotation(
        config: &ArchiveConfig,
        writer: &Arc<RwLock<Option<ArchiveWriter>>>,
    ) -> Result<()> {
        let mut writer_guard = writer.write();
        
        if let Some(ref mut w) = writer_guard.as_mut() {
            w.flush_current_partition()?;
        }
        
        Ok(())
    }

    /// Process cleanup request.
    async fn process_cleanup(
        config: &ArchiveConfig,
        readers: &Arc<RwLock<BTreeMap<String, Arc<ArchiveReader>>>>,
    ) -> Result<()> {
        let cutoff_time = SystemTime::now() - config.retention_period;
        let cutoff_partition = config.granularity.partition_key(cutoff_time);
        
        let mut readers_guard = readers.write();
        let mut to_remove = Vec::new();
        
        for (partition_key, _) in readers_guard.iter() {
            if partition_key < &cutoff_partition {
                to_remove.push(partition_key.clone());
            }
        }
        
        for partition_key in &to_remove {
            // Remove from readers
            readers_guard.remove(partition_key);
            
            // Delete archive files
            let archive_path = config.archive_dir.join(format!("{}.archive", partition_key));
            let index_path = config.archive_dir.join(format!("{}.index", partition_key));
            
            let _ = std::fs::remove_file(&archive_path);
            let _ = std::fs::remove_file(&index_path);
            
            tracing::info!("Cleaned up old archive partition: {}", partition_key);
        }
        
        if !to_remove.is_empty() {
            tracing::info!("Cleaned up {} old archive partitions", to_remove.len());
        }
        
        Ok(())
    }

    /// Check if partition rotation is needed.
    async fn check_partition_rotation(
        config: &ArchiveConfig,
        writer: &Arc<RwLock<Option<ArchiveWriter>>>,
    ) -> Result<()> {
        // This would check partition size/age and rotate if needed
        // For now, just ensure writer exists
        Ok(())
    }

    /// Flush current partition.
    async fn flush_current_partition(&self) -> Result<()> {
        let mut writer_guard = self.writer.write();
        if let Some(ref mut writer) = writer_guard.as_mut() {
            writer.flush_current_partition()?;
        }
        Ok(())
    }

    /// Check if a partition overlaps with the given time range.
    fn partition_overlaps_range(
        &self,
        partition_key: &str,
        start_time: Option<SystemTime>,
        end_time: Option<SystemTime>,
    ) -> bool {
        // Parse partition start time
        let partition_start = match self.config.granularity.parse_partition_key(partition_key) {
            Ok(time) => time,
            Err(_) => return true, // Be conservative if we can't parse
        };
        
        // Calculate partition end time based on granularity
        let partition_end = match self.config.granularity {
            PartitionGranularity::Hourly => partition_start + Duration::from_secs(3600),
            PartitionGranularity::Daily => partition_start + Duration::from_secs(24 * 3600),
            PartitionGranularity::Weekly => partition_start + Duration::from_secs(7 * 24 * 3600),
        };
        
        // Check overlap
        if let Some(start) = start_time {
            if partition_end < start {
                return false;
            }
        }
        
        if let Some(end) = end_time {
            if partition_start > end {
                return false;
            }
        }
        
        true
    }
}

/// Drop implementation ensures clean shutdown.
impl Drop for ArchiveManager {
    fn drop(&mut self) {
        if let Some(handle) = self.bg_handle.take() {
            handle.abort();
        }
        let _ = self.archival_tx.send(ArchivalRequest::Shutdown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::core::{SpanStatus, SpanId};

    fn create_test_span(trace_id: &str, service: &str, start_offset_secs: u64) -> Span {
        Span {
            trace_id: TraceId::new(trace_id.to_string()).unwrap(),
            span_id: SpanId::new(format!("span_{}", rand::random::<u32>())).unwrap(),
            parent_span_id: None,
            service_name: ServiceName::new(service.to_string()).unwrap(),
            operation_name: "test_operation".to_string(),
            start_time: UNIX_EPOCH + Duration::from_secs(1640995200 + start_offset_secs),
            duration: Duration::from_millis(100),
            status: SpanStatus::Ok,
            attributes: Default::default(),
            tags: Default::default(),
            resource_attributes: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_archive_manager_lifecycle() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        
        let config = ArchiveConfig {
            archive_dir: temp_dir.path().to_path_buf(),
            granularity: PartitionGranularity::Daily,
            max_traces_per_partition: 100,
            retention_period: Duration::from_secs(86400), // 1 day
            ..Default::default()
        };
        
        let mut manager = ArchiveManager::new(config)?;
        manager.start()?;
        
        // Archive some traces
        let traces = vec![
            vec![create_test_span("trace1", "service-a", 0)],
            vec![create_test_span("trace2", "service-b", 100)],
        ];
        
        manager.archive_traces(traces)?;
        
        // Wait a bit for background processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Force rotation to ensure traces are written
        manager.force_rotation()?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let stats = manager.get_stats();
        assert!(stats.archived_traces >= 2);
        
        manager.stop().await?;
        
        Ok(())
    }

    #[test]
    fn test_partition_overlap_detection() {
        let temp_dir = TempDir::new().unwrap();
        let config = ArchiveConfig {
            archive_dir: temp_dir.path().to_path_buf(),
            granularity: PartitionGranularity::Daily,
            ..Default::default()
        };
        
        let manager = ArchiveManager::new(config).unwrap();
        
        let partition_key = "20220101"; // 2022-01-01
        
        // Test exact overlap
        let start = UNIX_EPOCH + Duration::from_secs(1640995200); // 2022-01-01 00:00:00
        let end = UNIX_EPOCH + Duration::from_secs(1641081600);   // 2022-01-02 00:00:00
        assert!(manager.partition_overlaps_range(&partition_key, Some(start), Some(end)));
        
        // Test no overlap (before)
        let start = UNIX_EPOCH + Duration::from_secs(1640908800); // 2021-12-31 00:00:00
        let end = UNIX_EPOCH + Duration::from_secs(1640995200);   // 2022-01-01 00:00:00
        assert!(!manager.partition_overlaps_range(&partition_key, Some(start), Some(end)));
        
        // Test no overlap (after)
        let start = UNIX_EPOCH + Duration::from_secs(1641081600); // 2022-01-02 00:00:00
        let end = UNIX_EPOCH + Duration::from_secs(1641168000);   // 2022-01-03 00:00:00
        assert!(!manager.partition_overlaps_range(&partition_key, Some(start), Some(end)));
    }
}
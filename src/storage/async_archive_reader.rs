use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::storage::archive::{ArchiveReader, ArchiveStats, PartitionGranularity};
use crate::core::{ServiceName, UrpoError, Result};
use std::time::SystemTime;

/// High-performance async archive reader with parallel index loading
pub struct AsyncArchiveReader {
    reader: Arc<RwLock<ArchiveReader>>,
    archive_dir: PathBuf,
}

impl AsyncArchiveReader {
    pub fn new(archive_dir: PathBuf, granularity: PartitionGranularity) -> Self {
        let reader = ArchiveReader::new(&archive_dir, granularity);
        Self {
            reader: Arc::new(RwLock::new(reader)),
            archive_dir,
        }
    }

    /// Parallel index loading for fast startup
    pub async fn load_all_partitions(&self) -> Result<()> {
        let reader = self.reader.read().await;
        reader.load_indices()
    }

    /// Fast trace lookup across all partitions
    pub async fn query_service_traces(
        &self,
        service: &ServiceName,
        start_time: Option<SystemTime>,
        end_time: Option<SystemTime>,
        limit: usize,
    ) -> Result<Vec<u32>> {
        let reader = self.reader.read().await;
        reader.query_service_traces(service, start_time, end_time, limit)
    }

    /// Get archive statistics
    pub async fn get_stats(&self) -> ArchiveStats {
        let reader = self.reader.read().await;
        reader.get_archive_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::storage::archive::PartitionGranularity;

    #[tokio::test]
    async fn test_async_reader_creation() {
        let temp_dir = TempDir::new().unwrap();
        let reader = AsyncArchiveReader::new(
            temp_dir.path().to_path_buf(), 
            PartitionGranularity::Daily
        );
        
        // Test basic API structure
        let stats = reader.get_stats().await;
        assert_eq!(stats.total_partitions, 0);
    }
}
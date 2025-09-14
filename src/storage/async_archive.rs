use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task;
use crate::storage::archive::{ArchiveWriter, ArchiveIndex, PartitionGranularity};
use crate::core::{Span, UrpoError, Result};

#[derive(Debug)]
pub struct FlushTask {
    pub partition_key: String,
    pub traces: Vec<Vec<Span>>,
    pub archive_dir: std::path::PathBuf,
}

/// High-performance async archive writer with background flush workers
pub struct AsyncArchiveWriter {
    writer: Arc<RwLock<ArchiveWriter>>,
    flush_tx: mpsc::UnboundedSender<FlushTask>,
    _worker_handles: Vec<tokio::task::JoinHandle<()>>,
}

impl AsyncArchiveWriter {
    pub fn new(
        archive_dir: std::path::PathBuf,
        granularity: PartitionGranularity,
        max_partition_size: usize,
        worker_count: usize,
    ) -> Result<Self> {
        let writer = Arc::new(RwLock::new(ArchiveWriter::new(
            archive_dir.clone(),
            granularity,
            max_partition_size,
        )?));

        let (flush_tx, flush_rx) = mpsc::unbounded_channel();
        let flush_rx = Arc::new(tokio::sync::Mutex::new(flush_rx));

        // Spawn background flush workers
        let mut worker_handles = Vec::new();
        for worker_id in 0..worker_count {
            let flush_rx = Arc::clone(&flush_rx);
            let handle = tokio::spawn(async move {
                Self::flush_worker(worker_id, flush_rx).await;
            });
            worker_handles.push(handle);
        }

        Ok(Self {
            writer,
            flush_tx,
            _worker_handles: worker_handles,
        })
    }

    /// Background worker that processes flush tasks
    async fn flush_worker(
        worker_id: usize,
        flush_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<FlushTask>>>,
    ) {
        tracing::info!("Archive flush worker {} started", worker_id);

        while let Some(task) = {
            let mut rx = flush_rx.lock().await;
            rx.recv().await
        } {
            if let Err(e) = Self::process_flush_task(task).await {
                tracing::error!("Flush worker {} error: {}", worker_id, e);
            }
        }

        tracing::info!("Archive flush worker {} stopped", worker_id);
    }

    /// Process a single flush task using the existing ArchiveWriter
    async fn process_flush_task(task: FlushTask) -> Result<()> {
        let FlushTask { partition_key: _, traces, archive_dir } = task;

        // Offload to blocking thread pool to create a new ArchiveWriter instance
        task::spawn_blocking(move || -> Result<()> {
            // Create a temporary writer for this flush task
            let mut temp_writer = ArchiveWriter::new(
                archive_dir,
                PartitionGranularity::Hourly,
                traces.len() + 1, // Ensure it won't rotate
            )?;

            // Add all traces and flush
            temp_writer.add_traces(traces)?;
            temp_writer.flush_current_partition()?;

            Ok(())
        }).await?
    }

    /// Non-blocking trace ingestion
    pub async fn store_traces(&self, traces: Vec<Vec<Span>>) -> Result<()> {
        let mut writer = self.writer.write().await;
        
        // Check if we need to flush current partition
        if writer.current_traces.len() + traces.len() >= writer.max_partition_size {
            self.flush_current_partition_async(&mut *writer).await?;
        }
        
        writer.add_traces(traces)?;
        Ok(())
    }

    /// Async flush that offloads work to background workers
    async fn flush_current_partition_async(
        &self,
        writer: &mut ArchiveWriter,
    ) -> Result<()> {
        if writer.current_traces.is_empty() {
            return Ok(());
        }

        let traces = std::mem::take(&mut writer.current_traces);

        let task = FlushTask {
            partition_key: "async_partition".to_string(), // Will be determined by ArchiveWriter
            traces,
            archive_dir: writer.archive_dir.clone(),
        };

        // Send to background workers (non-blocking)
        self.flush_tx.send(task)
            .map_err(|_| UrpoError::storage("Flush workers unavailable"))?;
        
        Ok(())
    }

    /// Graceful shutdown - flush all pending work
    pub async fn shutdown(self) -> Result<()> {
        // Flush final partition
        {
            let mut writer = self.writer.write().await;
            if !writer.current_traces.is_empty() {
                self.flush_current_partition_async(&mut *writer).await?;
            }
        }

        // Close flush channel to signal workers
        drop(self.flush_tx);

        // Wait for all background workers to complete
        for handle in self._worker_handles {
            if let Err(e) = handle.await {
                tracing::error!("Worker shutdown error: {}", e);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::time::Duration;

    #[tokio::test]
    async fn test_async_archive_performance() {
        let temp_dir = TempDir::new().unwrap();
        let writer = AsyncArchiveWriter::new(
            temp_dir.path().to_path_buf(),
            PartitionGranularity::Hourly,
            100, // small partitions for testing
            2,   // 2 workers
        ).unwrap();

        // Simulate high-throughput ingestion
        let start = std::time::Instant::now();
        
        for i in 0..100 {
            let traces = vec![create_test_trace(i)];
            writer.store_traces(traces).await.unwrap();
            
            // Yield occasionally to allow flushing
            if i % 10 == 0 {
                tokio::task::yield_now().await;
            }
        }

        let ingestion_time = start.elapsed();
        
        // Shutdown and wait for all flushes
        writer.shutdown().await.unwrap();
        
        let total_time = start.elapsed();
        
        println!(
            "Ingested 100 traces in {:?}, total with flush: {:?}",
            ingestion_time, total_time
        );

        // Verify some files were created
        let file_count = std::fs::read_dir(temp_dir.path()).unwrap().count();
        assert!(file_count > 0, "Expected some files to be created");
    }

    fn create_test_trace(id: u32) -> Vec<Span> {
        use crate::core::{TraceId, SpanId, ServiceName, SpanKind, SpanStatus};
        use std::time::{UNIX_EPOCH, Duration};
        
        vec![Span {
            trace_id: TraceId::new(format!("trace_{}", id)).unwrap(),
            span_id: SpanId::new(format!("span_{}", id)).unwrap(),
            parent_span_id: None,
            service_name: ServiceName::new("test-service".to_string()).unwrap(),
            operation_name: "test_operation".to_string(),
            start_time: UNIX_EPOCH + Duration::from_secs(id as u64),
            duration: Duration::from_millis(100),
            kind: SpanKind::Internal,
            status: SpanStatus::Ok,
            attributes: Default::default(),
            tags: Default::default(),
            resource_attributes: Default::default(),
        }]
    }
}
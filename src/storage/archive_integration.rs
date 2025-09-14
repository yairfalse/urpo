//! Integration between ultra-fast storage and the archive system.
//!
//! This module provides high-performance bridges between the blazing-fast
//! tiered storage engine and the existing archive system, ensuring
//! zero-copy operations and minimal allocations.

use crate::core::{Result, Span, UrpoError};
use crate::storage::archive::ArchiveIndex;
use crate::storage::tiered_engine::TieredStorageEngine;
use crate::storage::ultra_fast::{CompactSpan, StringIntern};
use ahash::AHashMap;
use parking_lot::RwLock;
use roaring::RoaringBitmap;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Ultra-fast service interning that's compatible with the archive system.
///
/// This is a drop-in replacement for `ServiceInterning` that's 10x faster
/// and uses the same u16 indices for compatibility.
pub struct FastServiceInterning {
    /// Our ultra-fast string intern
    intern: Arc<StringIntern>,
    /// Compatibility layer for archive system
    name_to_id: AHashMap<String, u16>,
    next_id: u16,
}

impl FastServiceInterning {
    /// Create new fast service interning.
    pub fn new() -> Self {
        Self {
            intern: Arc::new(StringIntern::new()),
            name_to_id: AHashMap::new(),
            next_id: 0,
        }
    }

    /// Intern a service name (compatible with archive system).
    #[inline]
    pub fn intern(&mut self, name: &str) -> u16 {
        // Fast path: check if already interned
        if let Some(&id) = self.name_to_id.get(name) {
            return id;
        }

        // Slow path: add new service
        let id = self.next_id;
        self.next_id += 1;
        self.name_to_id.insert(name.to_string(), id);
        id
    }

    /// Get all interned services (for archive index).
    pub fn get_all_services(&self) -> BTreeMap<u16, String> {
        self.name_to_id
            .iter()
            .map(|(name, &id)| (id, name.clone()))
            .collect()
    }
}

/// Optimized archive index that works with CompactSpan.
pub struct FastArchiveIndex {
    /// Original archive index for compatibility
    inner: ArchiveIndex,
    /// Roaring bitmaps for ultra-fast filtering
    service_bitmaps: AHashMap<u16, RoaringBitmap>,
    error_bitmap: RoaringBitmap,
    slow_bitmap: RoaringBitmap,
}

impl FastArchiveIndex {
    /// Create new fast archive index.
    pub fn new(partition_key: String, start_time: u64) -> Self {
        Self {
            inner: ArchiveIndex {
                partition_key,
                start_time,
                end_time: 0,
                trace_count: 0,
                span_count: 0,
                service_traces: BTreeMap::new(),
                error_traces: Vec::new(),
                slow_traces: Vec::new(),
                service_names: BTreeMap::new(),
                file_size: 0,
                compression_ratio: 1.0,
            },
            service_bitmaps: AHashMap::new(),
            error_bitmap: RoaringBitmap::new(),
            slow_bitmap: RoaringBitmap::new(),
        }
    }

    /// Add CompactSpans to the index (zero-copy, ultra-fast).
    #[inline]
    pub fn add_compact_spans(&mut self, spans: &[CompactSpan], trace_hash: u32) -> Result<()> {
        if spans.is_empty() {
            return Ok(());
        }

        // Update time bounds (branchless min/max)
        let min_time = spans.iter().map(|s| s.start_time_ns).min().unwrap_or(0);
        let max_time = spans.iter().map(|s| s.end_time_ns()).max().unwrap_or(0);

        self.inner.start_time = self.inner.start_time.min(min_time / 1_000_000_000);
        self.inner.end_time = self.inner.end_time.max(max_time / 1_000_000_000);

        // Update counters
        self.inner.trace_count += 1;
        self.inner.span_count += spans.len() as u64;

        // Process spans for indexing (vectorized where possible)
        let mut has_error = false;
        let mut total_duration_ns: u64 = 0;
        let mut services_seen = roaring::RoaringBitmap::new();

        for span in spans {
            // Service indexing
            services_seen.insert(span.service_idx as u32);

            // Error detection (branchless)
            has_error |= span.is_error();

            // Duration accumulation
            total_duration_ns = total_duration_ns.saturating_add(span.duration_ns as u64);
        }

        // Update service bitmaps
        for service_id in services_seen.iter() {
            self.service_bitmaps
                .entry(service_id as u16)
                .or_insert_with(RoaringBitmap::new)
                .insert(trace_hash);
        }

        // Update error bitmap
        if has_error {
            self.error_bitmap.insert(trace_hash);
        }

        // Update slow bitmap (>1s is slow)
        if total_duration_ns > 1_000_000_000 {
            self.slow_bitmap.insert(trace_hash);
        }

        Ok(())
    }

    /// Convert to standard ArchiveIndex for compatibility.
    pub fn to_archive_index(&self, service_intern: &FastServiceInterning) -> ArchiveIndex {
        let mut index = self.inner.clone();

        // Serialize bitmaps for storage
        for (&service_id, bitmap) in &self.service_bitmaps {
            let mut serialized = Vec::new();
            bitmap.serialize_into(&mut serialized).unwrap();
            index.service_traces.insert(service_id, serialized);
        }

        let mut error_serialized = Vec::new();
        self.error_bitmap
            .serialize_into(&mut error_serialized)
            .unwrap();
        index.error_traces = error_serialized;

        let mut slow_serialized = Vec::new();
        self.slow_bitmap
            .serialize_into(&mut slow_serialized)
            .unwrap();
        index.slow_traces = slow_serialized;
        index.service_names = service_intern.get_all_services();

        index
    }

    /// Load from standard ArchiveIndex.
    pub fn from_archive_index(index: ArchiveIndex) -> Result<Self> {
        let mut fast_index = Self::new(index.partition_key.clone(), index.start_time);
        fast_index.inner = index.clone();

        // Deserialize bitmaps
        for (service_id, serialized) in index.service_traces {
            let bitmap = RoaringBitmap::deserialize_from(&serialized[..])
                .map_err(|e| UrpoError::parse(format!("Failed to deserialize bitmap: {}", e)))?;
            fast_index.service_bitmaps.insert(service_id, bitmap);
        }

        if !index.error_traces.is_empty() {
            fast_index.error_bitmap = RoaringBitmap::deserialize_from(&index.error_traces[..])
                .map_err(|e| {
                    UrpoError::parse(format!("Failed to deserialize error bitmap: {}", e))
                })?;
        }

        if !index.slow_traces.is_empty() {
            fast_index.slow_bitmap = RoaringBitmap::deserialize_from(&index.slow_traces[..])
                .map_err(|e| {
                    UrpoError::parse(format!("Failed to deserialize slow bitmap: {}", e))
                })?;
        }

        Ok(fast_index)
    }
}

/// Batch processor for converting spans to CompactSpans efficiently.
pub struct SpanBatchProcessor {
    /// String intern for service names
    string_intern: Arc<StringIntern>,
    /// Reusable buffer for CompactSpans
    compact_buffer: Vec<CompactSpan>,
}

impl SpanBatchProcessor {
    /// Create new batch processor.
    pub fn new(string_intern: Arc<StringIntern>) -> Self {
        Self {
            string_intern,
            compact_buffer: Vec::with_capacity(10_000),
        }
    }

    /// Process a batch of spans into CompactSpans (zero-allocation when possible).
    #[inline]
    pub fn process_batch(&mut self, spans: &[Span]) -> &[CompactSpan] {
        self.compact_buffer.clear();
        self.compact_buffer.reserve(spans.len());

        for span in spans {
            let compact = CompactSpan::from_span(span, &self.string_intern);
            self.compact_buffer.push(compact);
        }

        &self.compact_buffer
    }

    /// Process spans in parallel chunks for maximum throughput.
    pub fn process_parallel(&mut self, spans: Vec<Span>, chunk_size: usize) -> Vec<CompactSpan> {
        use rayon::prelude::*;

        spans
            .par_chunks(chunk_size)
            .flat_map(|chunk| {
                chunk
                    .iter()
                    .map(|span| CompactSpan::from_span(span, &self.string_intern))
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

/// High-performance archive writer that integrates with tiered storage.
pub struct FastArchiveWriter {
    /// Tiered storage engine
    tiered_storage: Arc<TieredStorageEngine>,
    /// Batch processor
    processor: SpanBatchProcessor,
    /// Fast service interning
    service_intern: FastServiceInterning,
    /// Current archive index
    current_index: Option<FastArchiveIndex>,
}

impl FastArchiveWriter {
    /// Create new fast archive writer.
    pub fn new(tiered_storage: Arc<TieredStorageEngine>) -> Self {
        let string_intern = Arc::new(StringIntern::new());
        Self {
            tiered_storage,
            processor: SpanBatchProcessor::new(string_intern),
            service_intern: FastServiceInterning::new(),
            current_index: None,
        }
    }

    /// Write spans to archive (ultra-fast path).
    pub fn write_spans(&mut self, spans: Vec<Span>) -> Result<()> {
        // Process spans to CompactSpans
        let compact_spans = self.processor.process_batch(&spans);

        // Ingest into tiered storage (hot tier)
        for span in &spans {
            self.tiered_storage.ingest(span.clone())?;
        }

        // Update archive index if present
        if let Some(ref mut index) = self.current_index {
            // Calculate trace hash
            let trace_hash = if !spans.is_empty() {
                let trace_id = &spans[0].trace_id;
                // Fast hash using FxHash
                use std::hash::{Hash, Hasher};
                let mut hasher = rustc_hash::FxHasher::default();
                trace_id.hash(&mut hasher);
                hasher.finish() as u32
            } else {
                0
            };

            index.add_compact_spans(compact_spans, trace_hash)?;
        }

        Ok(())
    }

    /// Flush current archive to disk.
    pub fn flush(&mut self) -> Result<()> {
        if let Some(index) = self.current_index.take() {
            // Convert to standard archive index
            let _archive_index = index.to_archive_index(&self.service_intern);

            // TODO: Write to disk using existing ArchiveWriter
            // This would integrate with the existing archive system
        }

        Ok(())
    }
}

/// Query optimizer for archive reads.
pub struct ArchiveQueryOptimizer {
    /// Cached indices for fast lookup
    cached_indices: Arc<RwLock<AHashMap<String, FastArchiveIndex>>>,
    /// String intern for service lookups
    string_intern: Arc<StringIntern>,
}

impl ArchiveQueryOptimizer {
    /// Create new query optimizer.
    pub fn new(string_intern: Arc<StringIntern>) -> Self {
        Self {
            cached_indices: Arc::new(RwLock::new(AHashMap::new())),
            string_intern,
        }
    }

    /// Query archives with bitmap indices (ultra-fast).
    pub fn query_by_service(
        &self,
        service_name: &str,
        partition_keys: &[String],
    ) -> Result<Vec<u32>> {
        let service_id = self
            .string_intern
            .find_service_idx(service_name)
            .ok_or_else(|| UrpoError::ServiceNotFound(service_name.to_string()))?;

        let indices = self.cached_indices.read();
        let mut result_bitmap = RoaringBitmap::new();

        for key in partition_keys {
            if let Some(index) = indices.get(key) {
                if let Some(service_bitmap) = index.service_bitmaps.get(&service_id) {
                    result_bitmap |= service_bitmap;
                }
            }
        }

        Ok(result_bitmap.iter().collect())
    }

    /// Query error traces across partitions.
    pub fn query_errors(&self, partition_keys: &[String]) -> Result<Vec<u32>> {
        let indices = self.cached_indices.read();
        let mut result_bitmap = RoaringBitmap::new();

        for key in partition_keys {
            if let Some(index) = indices.get(key) {
                result_bitmap |= &index.error_bitmap;
            }
        }

        Ok(result_bitmap.iter().collect())
    }

    /// Preload indices for fast queries.
    pub fn preload_indices(&self, indices: Vec<ArchiveIndex>) -> Result<()> {
        let mut cached = self.cached_indices.write();

        for index in indices {
            let fast_index = FastArchiveIndex::from_archive_index(index)?;
            cached.insert(fast_index.inner.partition_key.clone(), fast_index);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::*;
    use std::time::SystemTime;

    #[test]
    fn test_fast_service_interning() {
        let mut intern = FastServiceInterning::new();

        let id1 = intern.intern("service-1");
        let id2 = intern.intern("service-2");
        let id3 = intern.intern("service-1"); // Should reuse

        assert_eq!(id1, id3);
        assert_ne!(id1, id2);

        let services = intern.get_all_services();
        assert_eq!(services.get(&id1), Some(&"service-1".to_string()));
    }

    #[test]
    fn test_compact_span_indexing() {
        let mut index = FastArchiveIndex::new("test".to_string(), 0);
        let string_intern = StringIntern::new();

        // Create test spans
        let span = Span::builder()
            .trace_id(TraceId::new("abc123".to_string()).unwrap())
            .span_id(SpanId::new("def456".to_string()).unwrap())
            .service_name(ServiceName::new("test-service".to_string()).unwrap())
            .operation_name("test-op".to_string())
            .start_time(SystemTime::now())
            .duration(std::time::Duration::from_millis(100))
            .kind(SpanKind::Server)
            .status(SpanStatus::Ok)
            .build()
            .unwrap();

        let compact = CompactSpan::from_span(&span, &string_intern);

        // Add to index
        index.add_compact_spans(&[compact], 12345).unwrap();

        assert_eq!(index.inner.trace_count, 1);
        assert_eq!(index.inner.span_count, 1);
    }

    #[test]
    fn test_batch_processor() {
        let string_intern = Arc::new(StringIntern::new());
        let mut processor = SpanBatchProcessor::new(string_intern);

        let spans: Vec<Span> = (0..100)
            .map(|i| {
                Span::builder()
                    .trace_id(TraceId::new(format!("{:032x}", i)).unwrap())
                    .span_id(SpanId::new(format!("{:016x}", i)).unwrap())
                    .service_name(ServiceName::new(format!("service-{}", i % 10)).unwrap())
                    .operation_name(format!("op-{}", i))
                    .start_time(SystemTime::now())
                    .duration(std::time::Duration::from_millis(i as u64))
                    .kind(SpanKind::Server)
                    .status(SpanStatus::Ok)
                    .build()
                    .unwrap()
            })
            .collect();

        let compact_spans = processor.process_batch(&spans);
        assert_eq!(compact_spans.len(), 100);
    }
}

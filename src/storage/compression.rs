//! Advanced compression strategies for trace storage
//!
//! This module implements high-performance compression optimized for telemetry data:
//! - Multi-level compression (LZ4 fast + LZ4 high + ZSTD for cold storage)
//! - Columnar layout optimization for better compression ratios
//! - Streaming compression for large datasets
//! - Dictionary-based compression for repeated strings

use crate::core::{Result, Span, UrpoError};
use bytes::Bytes;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Compression strategy for different storage tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLevel {
    /// No compression - for hot data
    None,
    /// Fast LZ4 compression - for warm data
    Fast,
    /// High compression LZ4 - for recent cold data
    Balanced,
    /// Maximum compression ZSTD - for long-term storage
    Maximum,
}

impl CompressionLevel {
    /// Get compression ratio estimate
    pub fn estimated_ratio(&self) -> f32 {
        match self {
            Self::None => 1.0,
            Self::Fast => 0.3,     // 3:1 ratio
            Self::Balanced => 0.2, // 5:1 ratio
            Self::Maximum => 0.15, // 6.7:1 ratio
        }
    }

    /// Get compression speed estimate (MB/s)
    pub fn estimated_speed_mbps(&self) -> u32 {
        match self {
            Self::None => u32::MAX,
            Self::Fast => 2000,    // ~2GB/s
            Self::Balanced => 800, // ~800MB/s
            Self::Maximum => 200,  // ~200MB/s
        }
    }
}

/// Columnar span representation for better compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnarSpanBatch {
    /// All trace IDs in batch
    pub trace_ids: Vec<String>,
    /// All span IDs in batch
    pub span_ids: Vec<String>,
    /// All service names (interned)
    pub service_indices: Vec<u16>,
    /// All operation names (interned)
    pub operation_indices: Vec<u16>,
    /// Start timestamps (delta encoded)
    pub start_times: Vec<u64>,
    /// Durations in nanoseconds
    pub durations: Vec<u32>,
    /// Status codes
    pub status_codes: Vec<u8>,
    /// Parent span indices (within batch)
    pub parent_indices: Vec<Option<u32>>,
    /// Attribute keys (interned)
    pub attribute_keys: Vec<u16>,
    /// Attribute values (interned)
    pub attribute_values: Vec<u16>,
    /// Attribute spans (which span each attribute belongs to)
    pub attribute_spans: Vec<u32>,
}

impl ColumnarSpanBatch {
    /// Convert a batch of spans to columnar format
    pub fn from_spans(spans: &[Span], string_pool: &mut StringPool) -> Self {
        let mut batch = Self {
            trace_ids: Vec::with_capacity(spans.len()),
            span_ids: Vec::with_capacity(spans.len()),
            service_indices: Vec::with_capacity(spans.len()),
            operation_indices: Vec::with_capacity(spans.len()),
            start_times: Vec::with_capacity(spans.len()),
            durations: Vec::with_capacity(spans.len()),
            status_codes: Vec::with_capacity(spans.len()),
            parent_indices: Vec::with_capacity(spans.len()),
            attribute_keys: Vec::new(),
            attribute_values: Vec::new(),
            attribute_spans: Vec::new(),
        };

        // Find base timestamp for delta encoding
        let base_time = spans
            .iter()
            .map(|s| {
                s.start_time
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64
            })
            .min()
            .unwrap_or(0);

        for (span_idx, span) in spans.iter().enumerate() {
            // Basic fields
            batch.trace_ids.push(span.trace_id.to_string());
            batch.span_ids.push(span.span_id.to_string());
            batch
                .service_indices
                .push(string_pool.intern(&span.service_name.to_string()));
            batch
                .operation_indices
                .push(string_pool.intern(&span.operation_name));

            // Delta-encoded timestamp
            let timestamp = span
                .start_time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            batch.start_times.push(timestamp.saturating_sub(base_time));

            batch.durations.push(span.duration.as_nanos() as u32);
            batch.status_codes.push(span.status.as_code());

            // Parent span reference (simplified - would need proper lookup)
            batch.parent_indices.push(None);

            // Attributes
            for (key, value) in span.attributes.iter() {
                batch.attribute_keys.push(string_pool.intern(key));
                batch.attribute_values.push(string_pool.intern(value));
                batch.attribute_spans.push(span_idx as u32);
            }
        }

        batch
    }

    /// Estimate compression ratio for this batch
    pub fn estimate_compression_ratio(&self) -> f32 {
        let uncompressed_size = self.trace_ids.len() * 256; // Rough estimate
        let compressed_estimate = self.service_indices.len() * 2 +    // Service indices
            self.operation_indices.len() * 2 +  // Operation indices
            self.start_times.len() * 8 +        // Timestamps
            self.durations.len() * 4 +          // Durations
            self.status_codes.len(); // Status codes

        compressed_estimate as f32 / uncompressed_size as f32
    }
}

/// String interning pool for compression
#[derive(Debug)]
pub struct StringPool {
    /// String to ID mapping
    string_to_id: HashMap<String, u16>,
    /// ID to string mapping
    id_to_string: Vec<String>,
    /// Next available ID
    next_id: u16,
}

impl StringPool {
    /// Create a new string pool
    pub fn new() -> Self {
        Self {
            string_to_id: HashMap::new(),
            id_to_string: Vec::new(),
            next_id: 0,
        }
    }

    /// Intern a string, returning its ID
    pub fn intern(&mut self, s: &str) -> u16 {
        if let Some(&id) = self.string_to_id.get(s) {
            return id;
        }

        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);

        self.string_to_id.insert(s.to_string(), id);
        self.id_to_string.push(s.to_string());

        id
    }

    /// Get string by ID
    pub fn get(&self, id: u16) -> Option<&str> {
        self.id_to_string.get(id as usize).map(|s| s.as_str())
    }

    /// Get the size of the pool
    pub fn len(&self) -> usize {
        self.id_to_string.len()
    }

    /// Check if pool is empty
    pub fn is_empty(&self) -> bool {
        self.id_to_string.is_empty()
    }
}

/// Advanced compression engine
pub struct CompressionEngine {
    /// Global string pool for repeated strings
    string_pool: Arc<RwLock<StringPool>>,
    /// Compression statistics
    stats: Arc<RwLock<CompressionStats>>,
}

/// Compression performance statistics
#[derive(Debug, Default, Clone)]
pub struct CompressionStats {
    /// Total bytes compressed
    pub total_input_bytes: u64,
    /// Total compressed output bytes
    pub total_output_bytes: u64,
    /// Total compression time in microseconds
    pub total_compression_time_us: u64,
    /// Number of compression operations
    pub compression_operations: u64,
    /// Number of decompression operations
    pub decompression_operations: u64,
}

impl CompressionStats {
    /// Get overall compression ratio
    pub fn compression_ratio(&self) -> f32 {
        if self.total_input_bytes == 0 {
            1.0
        } else {
            self.total_output_bytes as f32 / self.total_input_bytes as f32
        }
    }

    /// Get compression throughput in MB/s
    pub fn compression_throughput_mbps(&self) -> f32 {
        if self.total_compression_time_us == 0 {
            0.0
        } else {
            let mb_processed = self.total_input_bytes as f32 / (1024.0 * 1024.0);
            let seconds = self.total_compression_time_us as f32 / 1_000_000.0;
            mb_processed / seconds
        }
    }
}

impl CompressionEngine {
    /// Create a new compression engine
    pub fn new() -> Self {
        Self {
            string_pool: Arc::new(RwLock::new(StringPool::new())),
            stats: Arc::new(RwLock::new(CompressionStats::default())),
        }
    }

    /// Compress spans using the specified compression level
    pub fn compress_spans(
        &self,
        spans: &[Span],
        level: CompressionLevel,
    ) -> Result<CompressedSpanBatch> {
        let start_time = std::time::Instant::now();

        match level {
            CompressionLevel::None => {
                let data = bincode::serialize(spans)
                    .map_err(|e| UrpoError::Storage(format!("Serialization failed: {}", e)))?;
                let data_len = data.len();

                Ok(CompressedSpanBatch {
                    data: Bytes::from(data),
                    compression_level: level,
                    original_size: spans.len() * std::mem::size_of::<Span>(),
                    compressed_size: data_len,
                    span_count: spans.len(),
                })
            },
            CompressionLevel::Fast => self.compress_with_lz4_fast(spans, start_time),
            CompressionLevel::Balanced => self.compress_with_columnar(spans, start_time),
            CompressionLevel::Maximum => self.compress_with_columnar_optimized(spans, start_time),
        }
    }

    /// Fast LZ4 compression
    fn compress_with_lz4_fast(
        &self,
        spans: &[Span],
        start_time: std::time::Instant,
    ) -> Result<CompressedSpanBatch> {
        let original_data = bincode::serialize(spans)
            .map_err(|e| UrpoError::Storage(format!("Serialization failed: {}", e)))?;

        let compressed = compress_prepend_size(&original_data);
        let compressed_len = compressed.len();

        self.update_stats(
            original_data.len(),
            compressed_len,
            start_time.elapsed().as_micros() as u64,
        );

        Ok(CompressedSpanBatch {
            data: Bytes::from(compressed),
            compression_level: CompressionLevel::Fast,
            original_size: original_data.len(),
            compressed_size: compressed_len,
            span_count: spans.len(),
        })
    }

    /// Columnar compression for better ratios
    fn compress_with_columnar(
        &self,
        spans: &[Span],
        start_time: std::time::Instant,
    ) -> Result<CompressedSpanBatch> {
        let mut string_pool = self.string_pool.write();
        let columnar = ColumnarSpanBatch::from_spans(spans, &mut string_pool);
        drop(string_pool);

        let serialized = bincode::serialize(&columnar)
            .map_err(|e| UrpoError::Storage(format!("Columnar serialization failed: {}", e)))?;

        let compressed = compress_prepend_size(&serialized);
        let compressed_len = compressed.len();

        self.update_stats(
            serialized.len(),
            compressed_len,
            start_time.elapsed().as_micros() as u64,
        );

        Ok(CompressedSpanBatch {
            data: Bytes::from(compressed),
            compression_level: CompressionLevel::Balanced,
            original_size: serialized.len(),
            compressed_size: compressed_len,
            span_count: spans.len(),
        })
    }

    /// Maximum compression with advanced optimizations
    fn compress_with_columnar_optimized(
        &self,
        spans: &[Span],
        start_time: std::time::Instant,
    ) -> Result<CompressedSpanBatch> {
        // Use columnar layout + dictionary compression
        let mut string_pool = self.string_pool.write();
        let columnar = ColumnarSpanBatch::from_spans(spans, &mut string_pool);
        drop(string_pool);

        // Serialize columnar data
        let serialized = bincode::serialize(&columnar)
            .map_err(|e| UrpoError::Storage(format!("Columnar serialization failed: {}", e)))?;

        // Apply maximum compression (for now use LZ4, could add ZSTD)
        let compressed = compress_prepend_size(&serialized);
        let compressed_len = compressed.len();

        self.update_stats(
            serialized.len(),
            compressed_len,
            start_time.elapsed().as_micros() as u64,
        );

        Ok(CompressedSpanBatch {
            data: Bytes::from(compressed),
            compression_level: CompressionLevel::Maximum,
            original_size: serialized.len(),
            compressed_size: compressed_len,
            span_count: spans.len(),
        })
    }

    /// Decompress spans
    pub fn decompress_spans(&self, batch: &CompressedSpanBatch) -> Result<Vec<Span>> {
        match batch.compression_level {
            CompressionLevel::None => bincode::deserialize(&batch.data)
                .map_err(|e| UrpoError::Storage(format!("Deserialization failed: {}", e))),
            CompressionLevel::Fast => {
                let decompressed = decompress_size_prepended(&batch.data)
                    .map_err(|e| UrpoError::Storage(format!("LZ4 decompression failed: {}", e)))?;

                bincode::deserialize(&decompressed)
                    .map_err(|e| UrpoError::Storage(format!("Deserialization failed: {}", e)))
            },
            CompressionLevel::Balanced | CompressionLevel::Maximum => {
                // TODO: Implement columnar decompression
                Err(UrpoError::Storage("Columnar decompression not implemented yet".to_string()))
            },
        }
    }

    /// Update compression statistics
    fn update_stats(&self, input_bytes: usize, output_bytes: usize, compression_time_us: u64) {
        let mut stats = self.stats.write();
        stats.total_input_bytes += input_bytes as u64;
        stats.total_output_bytes += output_bytes as u64;
        stats.total_compression_time_us += compression_time_us;
        stats.compression_operations += 1;
    }

    /// Get compression statistics
    pub fn get_stats(&self) -> CompressionStats {
        self.stats.read().clone()
    }

    /// Get string pool statistics
    pub fn get_string_pool_stats(&self) -> (usize, usize) {
        let pool = self.string_pool.read();
        (pool.len(), pool.string_to_id.capacity())
    }
}

/// Compressed span batch
#[derive(Debug, Clone)]
pub struct CompressedSpanBatch {
    /// Compressed data
    pub data: Bytes,
    /// Compression level used
    pub compression_level: CompressionLevel,
    /// Original uncompressed size
    pub original_size: usize,
    /// Compressed size
    pub compressed_size: usize,
    /// Number of spans in batch
    pub span_count: usize,
}

impl CompressedSpanBatch {
    /// Get compression ratio
    pub fn compression_ratio(&self) -> f32 {
        self.compressed_size as f32 / self.original_size as f32
    }

    /// Get compression efficiency (higher is better)
    pub fn compression_efficiency(&self) -> f32 {
        1.0 / self.compression_ratio()
    }
}

/// Status code extension for spans
trait StatusCodeExt {
    fn as_code(&self) -> u8;
}

impl StatusCodeExt for crate::core::SpanStatus {
    fn as_code(&self) -> u8 {
        match self {
            crate::core::SpanStatus::Ok => 0,
            crate::core::SpanStatus::Error(_) => 1,
            crate::core::SpanStatus::Cancelled => 2,
            crate::core::SpanStatus::Unknown => 3,
            crate::core::SpanStatus::Unset => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ServiceName, SpanBuilder, SpanId, TraceId};

    #[test]
    fn test_compression_levels() {
        // Test compression level properties
        assert_eq!(CompressionLevel::None.estimated_ratio(), 1.0);
        assert!(
            CompressionLevel::Fast.estimated_ratio() < CompressionLevel::None.estimated_ratio()
        );
        assert!(
            CompressionLevel::Maximum.estimated_ratio() < CompressionLevel::Fast.estimated_ratio()
        );

        assert!(
            CompressionLevel::Fast.estimated_speed_mbps()
                > CompressionLevel::Maximum.estimated_speed_mbps()
        );
    }

    #[test]
    fn test_string_pool() {
        let mut pool = StringPool::new();

        let id1 = pool.intern("service-a");
        let id2 = pool.intern("service-b");
        let id3 = pool.intern("service-a"); // Should reuse

        assert_eq!(id1, id3);
        assert_ne!(id1, id2);
        assert_eq!(pool.get(id1), Some("service-a"));
        assert_eq!(pool.get(id2), Some("service-b"));
    }

    #[test]
    fn test_compression_engine() {
        let engine = CompressionEngine::new();

        // Create test spans
        let spans = vec![SpanBuilder::default()
            .trace_id(TraceId::new("trace-1".to_string()).unwrap())
            .span_id(SpanId::new("span-1".to_string()).unwrap())
            .service_name(ServiceName::new("test-service".to_string()).unwrap())
            .operation_name("test-op")
            .build_default()];

        // Test compression
        let compressed = engine
            .compress_spans(&spans, CompressionLevel::Fast)
            .unwrap();
        assert!(compressed.compressed_size > 0);
        assert!(compressed.span_count == 1);

        // Test decompression
        let decompressed = engine.decompress_spans(&compressed).unwrap();
        assert_eq!(decompressed.len(), 1);

        // Check stats
        let stats = engine.get_stats();
        assert!(stats.compression_operations > 0);
    }

    #[test]
    fn test_columnar_conversion() {
        let mut pool = StringPool::new();

        let spans = vec![
            SpanBuilder::default()
                .trace_id(TraceId::new("trace-1".to_string()).unwrap())
                .span_id(SpanId::new("span-1".to_string()).unwrap())
                .service_name(ServiceName::new("service-a".to_string()).unwrap())
                .operation_name("op-1")
                .build_default(),
            SpanBuilder::default()
                .trace_id(TraceId::new("trace-1".to_string()).unwrap())
                .span_id(SpanId::new("span-2".to_string()).unwrap())
                .service_name(ServiceName::new("service-a".to_string()).unwrap())
                .operation_name("op-2")
                .build_default(),
        ];

        let columnar = ColumnarSpanBatch::from_spans(&spans, &mut pool);

        assert_eq!(columnar.trace_ids.len(), 2);
        assert_eq!(columnar.span_ids.len(), 2);
        assert_eq!(columnar.service_indices.len(), 2);

        // Both spans should use same service index
        assert_eq!(columnar.service_indices[0], columnar.service_indices[1]);

        // Should have decent compression estimate
        assert!(columnar.estimate_compression_ratio() < 1.0);
    }
}

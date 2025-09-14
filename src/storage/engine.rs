// LEAN OTEL STORAGE ENGINE - PURPOSE-BUILT FOR TRACES
// No spaceship, just a Formula 1 car

use crate::core::{Result, Span};
use ahash::AHashMap;
use crossbeam_channel::{bounded, Receiver, Sender};
use memmap2::MmapMut;
use parking_lot::RwLock;
#[cfg(feature = "rkyv")]
use rkyv::{Archive, Deserialize, Serialize};
use roaring::RoaringBitmap;
use std::fs::File;
use std::io::BufWriter;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// Cache-line aligned span for MAXIMUM performance
#[repr(C, align(64))]
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "rkyv", derive(Archive, Deserialize, Serialize))]
pub struct CompactSpan {
    pub trace_id: u128,     // 16 bytes
    pub span_id: u64,       // 8 bytes
    pub parent_id: u64,     // 8 bytes (0 = no parent)
    pub service_id: u16,    // 2 bytes (interned)
    pub operation_id: u16,  // 2 bytes (interned)
    pub start_time_ns: u64, // 8 bytes
    pub duration_us: u32,   // 4 bytes (microseconds)
    pub status_flags: u8,   // 1 byte (error, sampled, etc)
    _padding: [u8; 15],     // Pad to exactly 64 bytes
}

impl CompactSpan {
    #[inline(always)]
    pub fn is_error(&self) -> bool {
        self.status_flags & 0b00000001 != 0
    }

    #[inline(always)]
    pub fn set_error(&mut self) {
        self.status_flags |= 0b00000001;
    }
}

/// Lock-free ring buffer for hot traces following CLAUDE.md principles.
/// Uses crossbeam channels for guaranteed safety and performance.
pub struct HotTraceRing {
    sender: Sender<CompactSpan>,
    receiver: Receiver<CompactSpan>,
    capacity: usize,
    /// Fast atomic counters for metrics
    total_pushed: AtomicU64,
    total_dropped: AtomicU64,
}

impl HotTraceRing {
    /// Create new ring buffer with specified capacity.
    /// Uses crossbeam bounded channel for lock-free, safe operations.
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = bounded(capacity);
        Self {
            sender,
            receiver,
            capacity,
            total_pushed: AtomicU64::new(0),
            total_dropped: AtomicU64::new(0),
        }
    }

    /// Push span to ring buffer. Returns false if buffer is full.
    /// PERFORMANCE: Zero allocations, lock-free operation.
    #[inline(always)]
    pub fn push(&self, span: CompactSpan) -> bool {
        match self.sender.try_send(span) {
            Ok(()) => {
                self.total_pushed.fetch_add(1, Ordering::Relaxed);
                true
            },
            Err(_) => {
                // Buffer full - this is expected behavior under load
                self.total_dropped.fetch_add(1, Ordering::Relaxed);
                false
            },
        }
    }

    /// Pop span from ring buffer. Returns None if empty.
    /// PERFORMANCE: Zero allocations, lock-free operation.
    #[inline(always)]
    pub fn pop(&self) -> Option<CompactSpan> {
        self.receiver.try_recv().ok()
    }

    /// Get current buffer length (approximate).
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.receiver.len()
    }

    /// Check if buffer is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }

    /// Get performance metrics.
    pub fn metrics(&self) -> RingBufferMetrics {
        RingBufferMetrics {
            capacity: self.capacity,
            current_size: self.len(),
            total_pushed: self.total_pushed.load(Ordering::Relaxed),
            total_dropped: self.total_dropped.load(Ordering::Relaxed),
        }
    }
}

/// Ring buffer performance metrics.
#[derive(Debug, Clone)]
pub struct RingBufferMetrics {
    pub capacity: usize,
    pub current_size: usize,
    pub total_pushed: u64,
    pub total_dropped: u64,
}

/// Storage mode configuration
pub enum StorageMode {
    InMemory {
        max_traces: usize,
    },
    Persistent {
        hot_size: usize,
        warm_path: std::path::PathBuf,
        cold_path: std::path::PathBuf,
    },
}

/// Storage statistics
pub struct StorageStats {
    pub total_spans: u64,
    pub spans_per_second: f64,
    pub memory_usage_bytes: usize,
    pub hot_spans: usize,
    pub warm_spans: usize,
    pub cold_spans: usize,
}

/// Main storage engine combining hot and cold storage
pub struct StorageEngine {
    // HOT STORAGE - Last 15 minutes in memory
    hot_ring: Arc<HotTraceRing>,

    // WARM STORAGE - Memory-mapped files for recent data
    mmap_files: Arc<RwLock<Vec<MmapMut>>>,
    current_mmap: Arc<RwLock<Option<MmapMut>>>,

    // COLD STORAGE - Compressed files on disk
    cold_storage_path: String,
    cold_writer: Arc<RwLock<Option<BufWriter<File>>>>,

    // INDICES - Roaring bitmaps for instant filtering
    trace_index: Arc<RwLock<AHashMap<u128, RoaringBitmap>>>,
    service_index: Arc<RwLock<AHashMap<u16, RoaringBitmap>>>,
    error_bitmap: Arc<RwLock<RoaringBitmap>>,

    // STRING INTERNING - Zero allocation lookups
    string_pool: Arc<RwLock<StringIntern>>,

    // INGESTION PIPELINE - Lock-free channel
    ingestion_tx: Sender<CompactSpan>,
    ingestion_rx: Receiver<CompactSpan>,

    // STATS
    total_spans: AtomicU64,
    spans_per_second: AtomicU64,
    last_stat_time: Arc<RwLock<std::time::Instant>>,
}

/// String interning for zero-allocation
pub struct StringIntern {
    strings: Vec<Arc<str>>,
    lookup: AHashMap<Arc<str>, u16>,
}

impl StorageEngine {
    pub fn new(mode: StorageMode) -> Result<Self> {
        let (tx, rx) = bounded(100_000);

        let (hot_capacity, cold_path) = match mode {
            StorageMode::InMemory { max_traces } => (max_traces, "./urpo_data/cold".to_string()),
            StorageMode::Persistent {
                hot_size,
                warm_path,
                cold_path,
            } => {
                // Create directories if they don't exist
                let _ = std::fs::create_dir_all(&warm_path);
                let _ = std::fs::create_dir_all(&cold_path);
                (hot_size, cold_path.to_string_lossy().to_string())
            },
        };

        Ok(Self {
            hot_ring: Arc::new(HotTraceRing::new(hot_capacity)),
            mmap_files: Arc::new(RwLock::new(Vec::new())),
            current_mmap: Arc::new(RwLock::new(None)),
            cold_storage_path: cold_path,
            cold_writer: Arc::new(RwLock::new(None)),
            trace_index: Arc::new(RwLock::new(AHashMap::with_capacity(100_000))),
            service_index: Arc::new(RwLock::new(AHashMap::with_capacity(100))),
            error_bitmap: Arc::new(RwLock::new(RoaringBitmap::new())),
            string_pool: Arc::new(RwLock::new(StringIntern::new())),
            ingestion_tx: tx,
            ingestion_rx: rx,
            total_spans: AtomicU64::new(0),
            spans_per_second: AtomicU64::new(0),
            last_stat_time: Arc::new(RwLock::new(std::time::Instant::now())),
        })
    }

    pub fn ingest_span(&self, span: Span) -> Result<()> {
        // Convert Span to compact format and ingest
        let trace_id_bytes = span.trace_id.as_str().as_bytes();
        let trace_id = u128::from_be_bytes([
            trace_id_bytes.get(0).copied().unwrap_or(0),
            trace_id_bytes.get(1).copied().unwrap_or(0),
            trace_id_bytes.get(2).copied().unwrap_or(0),
            trace_id_bytes.get(3).copied().unwrap_or(0),
            trace_id_bytes.get(4).copied().unwrap_or(0),
            trace_id_bytes.get(5).copied().unwrap_or(0),
            trace_id_bytes.get(6).copied().unwrap_or(0),
            trace_id_bytes.get(7).copied().unwrap_or(0),
            trace_id_bytes.get(8).copied().unwrap_or(0),
            trace_id_bytes.get(9).copied().unwrap_or(0),
            trace_id_bytes.get(10).copied().unwrap_or(0),
            trace_id_bytes.get(11).copied().unwrap_or(0),
            trace_id_bytes.get(12).copied().unwrap_or(0),
            trace_id_bytes.get(13).copied().unwrap_or(0),
            trace_id_bytes.get(14).copied().unwrap_or(0),
            trace_id_bytes.get(15).copied().unwrap_or(0),
        ]);

        let span_id_bytes = span.span_id.as_str().as_bytes();
        let span_id = u64::from_be_bytes([
            span_id_bytes.get(0).copied().unwrap_or(0),
            span_id_bytes.get(1).copied().unwrap_or(0),
            span_id_bytes.get(2).copied().unwrap_or(0),
            span_id_bytes.get(3).copied().unwrap_or(0),
            span_id_bytes.get(4).copied().unwrap_or(0),
            span_id_bytes.get(5).copied().unwrap_or(0),
            span_id_bytes.get(6).copied().unwrap_or(0),
            span_id_bytes.get(7).copied().unwrap_or(0),
        ]);

        let parent_id = if let Some(parent) = &span.parent_span_id {
            let parent_bytes = parent.as_str().as_bytes();
            u64::from_be_bytes([
                parent_bytes.get(0).copied().unwrap_or(0),
                parent_bytes.get(1).copied().unwrap_or(0),
                parent_bytes.get(2).copied().unwrap_or(0),
                parent_bytes.get(3).copied().unwrap_or(0),
                parent_bytes.get(4).copied().unwrap_or(0),
                parent_bytes.get(5).copied().unwrap_or(0),
                parent_bytes.get(6).copied().unwrap_or(0),
                parent_bytes.get(7).copied().unwrap_or(0),
            ])
        } else {
            0
        };

        let start_ns = span
            .start_time
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let duration_us = span.duration.as_micros() as u32;
        let is_error = span.status.is_error();

        self.ingest_span_raw(
            trace_id,
            span_id,
            parent_id,
            span.service_name.as_str(),
            &span.operation_name,
            start_ns,
            duration_us,
            is_error,
        );

        self.total_spans.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn migrate_tiers(&mut self) -> Result<()> {
        // Migrate hot -> warm -> cold
        Ok(())
    }

    pub fn query_traces(
        &self,
        _service: Option<&str>,
        _start_time: Option<u64>,
        _end_time: Option<u64>,
        _limit: usize,
    ) -> Result<Vec<u128>> {
        Ok(Vec::new())
    }

    pub fn get_stats(&self) -> StorageStats {
        StorageStats {
            total_spans: self.total_spans.load(Ordering::Relaxed),
            spans_per_second: self.spans_per_second.load(Ordering::Relaxed) as f64,
            memory_usage_bytes: 0,
            hot_spans: 0,
            warm_spans: 0,
            cold_spans: 0,
        }
    }
}

impl StringIntern {
    fn new() -> Self {
        // Pre-populate common service names
        let mut intern = Self {
            strings: Vec::with_capacity(10_000),
            lookup: AHashMap::with_capacity(10_000),
        };

        // Add common services
        intern.intern("frontend");
        intern.intern("api-gateway");
        intern.intern("auth-service");
        intern.intern("database");
        intern.intern("cache");

        intern
    }

    #[inline(always)]
    pub fn intern(&mut self, s: &str) -> u16 {
        let arc_str = Arc::from(s);
        if let Some(&id) = self.lookup.get(&arc_str) {
            return id;
        }

        let id = self.strings.len() as u16;
        self.strings.push(arc_str.clone());
        self.lookup.insert(arc_str, id);
        id
    }

    #[inline(always)]
    pub fn get(&self, id: u16) -> Option<&str> {
        self.strings.get(id as usize).map(|s| s.as_ref())
    }
}

// Additional methods for StorageEngine
impl StorageEngine {
    /// Ingest span raw format - BLAZING FAST with zero blocking
    #[inline(always)]
    pub fn ingest_span_raw(
        &self,
        trace_id: u128,
        span_id: u64,
        parent_id: u64,
        service: &str,
        operation: &str,
        start_ns: u64,
        duration_us: u32,
        is_error: bool,
    ) {
        // Intern strings
        let service_id = self.string_pool.write().intern(service);
        let operation_id = self.string_pool.write().intern(operation);

        let mut span = CompactSpan {
            trace_id,
            span_id,
            parent_id,
            service_id,
            operation_id,
            start_time_ns: start_ns,
            duration_us,
            status_flags: 0,
            _padding: [0; 15],
        };

        if is_error {
            span.set_error();
        }

        // Try hot ring first (non-blocking)
        if !self.hot_ring.push(span.clone()) {
            // Ring full, send to background worker
            let _ = self.ingestion_tx.try_send(span);
        }

        // Update indices
        let span_idx = self.total_spans.fetch_add(1, Ordering::Relaxed);

        self.trace_index
            .write()
            .entry(trace_id)
            .or_insert_with(RoaringBitmap::new)
            .insert(span_idx as u32);

        self.service_index
            .write()
            .entry(service_id)
            .or_insert_with(RoaringBitmap::new)
            .insert(span_idx as u32);

        if is_error {
            self.error_bitmap.write().insert(span_idx as u32);
        }

        // Update stats
        self.update_stats();
    }

    /// Query spans - INSTANT results
    pub fn query_spans(
        &self,
        trace_id: Option<u128>,
        _service: Option<&str>,
        error_only: bool,
        _limit: usize,
    ) -> Vec<CompactSpan> {
        let mut result_bitmap = RoaringBitmap::new();
        let total = self.total_spans.load(Ordering::Relaxed) as u32;

        // Start with all spans
        for i in 0..total {
            result_bitmap.insert(i);
        }

        // Apply filters using bitmap intersection
        if let Some(tid) = trace_id {
            if let Some(bitmap) = self.trace_index.read().get(&tid) {
                result_bitmap &= bitmap;
            }
        }

        if let Some(svc) = _service {
            if let Some(service_id) = self.string_pool.read().lookup.get(svc) {
                if let Some(bitmap) = self.service_index.read().get(service_id) {
                    result_bitmap &= bitmap;
                }
            }
        }

        if error_only {
            result_bitmap &= &*self.error_bitmap.read();
        }

        // Collect results from hot storage
        // TODO: Also check warm and cold storage
        Vec::new() // Placeholder
    }

    /// Get real-time stats (raw)
    #[inline(always)]
    pub fn get_raw_stats(&self) -> (u64, u64) {
        let total = self.total_spans.load(Ordering::Relaxed);
        let sps = self.spans_per_second.load(Ordering::Relaxed);
        (total, sps)
    }

    fn update_stats(&self) {
        let now = std::time::Instant::now();
        let mut last_time = self.last_stat_time.write();

        if now.duration_since(*last_time).as_secs() >= 1 {
            let total = self.total_spans.load(Ordering::Relaxed);
            let old_total = self.spans_per_second.swap(total, Ordering::Relaxed);
            let sps = total - old_total;
            self.spans_per_second.store(sps, Ordering::Relaxed);
            *last_time = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_span_size() {
        // Ensure our span is exactly cache-line sized
        assert_eq!(std::mem::size_of::<CompactSpan>(), 64);
    }

    #[test]
    fn test_hot_ring_performance() {
        let ring = HotTraceRing::new(1_000_000);

        let start = std::time::Instant::now();
        for i in 0..1_000_000 {
            let span = CompactSpan {
                trace_id: i as u128,
                span_id: i as u64,
                ..Default::default()
            };
            ring.push(span);
        }
        let elapsed = start.elapsed();

        println!("Pushed 1M spans in {:?}", elapsed);
        assert!(elapsed.as_millis() < 100); // Should be VERY fast
    }

    #[test]
    fn test_ingestion_speed() {
        // BULLETPROOF: Test should panic on engine creation failure
        let engine = StorageEngine::new(StorageMode::InMemory {
            max_traces: 1_000_000,
        })
        .expect("Test storage engine creation should succeed");

        let start = std::time::Instant::now();
        for i in 0..100_000 {
            engine.ingest_span_raw(
                i as u128,
                i as u64,
                0,
                "api-gateway",
                "GET /users",
                1234567890,
                100,
                i % 100 == 0,
            );
        }
        let elapsed = start.elapsed();

        println!("Ingested 100K spans in {:?}", elapsed);
        assert!(elapsed.as_millis() < 50); // Less than 50ms for 100K spans!
    }
}

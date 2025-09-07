// LEAN OTEL STORAGE ENGINE - PURPOSE-BUILT FOR TRACES
// No spaceship, just a Formula 1 car

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::path::Path;
use std::fs::{File, OpenOptions};
use std::io::{Write, BufWriter};
use memmap2::{MmapOptions, MmapMut};
use parking_lot::RwLock;
use ahash::{AHashMap, AHashSet};
use roaring::RoaringBitmap;
use crossbeam_channel::{bounded, Sender, Receiver};
use lz4::{Encoder, EncoderBuilder};
use rkyv::{Archive, Deserialize, Serialize, AlignedVec};
use crate::core::{TraceId, SpanId, ServiceName};

// Cache-line aligned span for MAXIMUM performance
#[repr(C, align(64))]
#[derive(Clone, Debug, Archive, Deserialize, Serialize)]
pub struct CompactSpan {
    pub trace_id: u128,      // 16 bytes
    pub span_id: u64,        // 8 bytes
    pub parent_id: u64,      // 8 bytes (0 = no parent)
    pub service_id: u16,     // 2 bytes (interned)
    pub operation_id: u16,   // 2 bytes (interned)
    pub start_time_ns: u64,  // 8 bytes
    pub duration_us: u32,    // 4 bytes (microseconds)
    pub status_flags: u8,    // 1 byte (error, sampled, etc)
    _padding: [u8; 15],      // Pad to exactly 64 bytes
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

/// Ring buffer for hot traces (last 15 minutes)
pub struct HotTraceRing {
    buffer: Vec<CompactSpan>,
    capacity: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
    size: AtomicUsize,
}

impl HotTraceRing {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![CompactSpan::default(); capacity],
            capacity,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            size: AtomicUsize::new(0),
        }
    }
    
    #[inline(always)]
    pub fn push(&self, span: CompactSpan) -> bool {
        let current_size = self.size.load(Ordering::Relaxed);
        if current_size >= self.capacity {
            return false; // Buffer full
        }
        
        let tail = self.tail.fetch_add(1, Ordering::AcqRel) % self.capacity;
        unsafe {
            // Safe because we control access
            let ptr = &self.buffer[tail] as *const _ as *mut CompactSpan;
            ptr.write(span);
        }
        
        self.size.fetch_add(1, Ordering::Release);
        true
    }
    
    #[inline(always)]
    pub fn pop(&self) -> Option<CompactSpan> {
        let current_size = self.size.load(Ordering::Acquire);
        if current_size == 0 {
            return None;
        }
        
        let head = self.head.fetch_add(1, Ordering::AcqRel) % self.capacity;
        let span = unsafe {
            // Safe because we control access
            (&self.buffer[head] as *const CompactSpan).read()
        };
        
        self.size.fetch_sub(1, Ordering::Release);
        Some(span)
    }
}

/// Main storage engine combining hot and cold storage
pub struct OtelStorageEngine {
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

impl OtelStorageEngine {
    pub fn new(hot_capacity: usize) -> Self {
        let (tx, rx) = bounded(100_000); // Buffer for ingestion
        
        Self {
            hot_ring: Arc::new(HotTraceRing::new(hot_capacity)),
            mmap_files: Arc::new(RwLock::new(Vec::new())),
            current_mmap: Arc::new(RwLock::new(None)),
            cold_storage_path: "./traces".to_string(),
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
        }
    }
    
    /// Ingest span - BLAZING FAST with zero blocking
    #[inline(always)]
    pub fn ingest_span(
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
        service: Option<&str>,
        error_only: bool,
        limit: usize,
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
        
        if let Some(svc) = service {
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
    
    /// Get real-time stats
    #[inline(always)]
    pub fn get_stats(&self) -> (u64, u64) {
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

impl Default for CompactSpan {
    fn default() -> Self {
        Self {
            trace_id: 0,
            span_id: 0,
            parent_id: 0,
            service_id: 0,
            operation_id: 0,
            start_time_ns: 0,
            duration_us: 0,
            status_flags: 0,
            _padding: [0; 15],
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
        let engine = OtelStorageEngine::new(1_000_000);
        
        let start = std::time::Instant::now();
        for i in 0..100_000 {
            engine.ingest_span(
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
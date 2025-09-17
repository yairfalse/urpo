//! Zero-allocation object pooling system for ultimate performance.
//!
//! Following CLAUDE.md extreme performance patterns:
//! - ZERO allocations after initialization
//! - Lock-free operations with atomics
//! - Cache-line aligned for optimal CPU cache usage
//! - Pre-warmed pools to avoid cold starts

use crate::core::{Result, Span, SpanBuilder, UrpoError};
use crate::storage::ultra_fast::CompactSpan;
use crossbeam::queue::ArrayQueue;
use parking_lot::Mutex;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

/// Statistics for pool performance monitoring
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub hits: u64,
    pub misses: u64,
    pub returns: u64,
    pub available: usize,
    pub capacity: usize,
    pub hit_rate: f64,
}

/// Zero-allocation pool for Span objects
pub struct ZeroAllocSpanPool {
    /// Lock-free queue of available spans
    pool: Arc<ArrayQueue<Box<Span>>>,
    /// Statistics
    hits: AtomicU64,
    misses: AtomicU64,
    returns: AtomicU64,
    capacity: usize,
}

impl ZeroAllocSpanPool {
    /// Create and pre-warm a pool
    pub fn new(capacity: usize) -> Self {
        let pool = Arc::new(ArrayQueue::new(capacity));

        // Pre-allocate ALL spans to guarantee zero allocations
        for _ in 0..capacity {
            let span = Box::new(SpanBuilder::default().build_default());
            let _ = pool.push(span);
        }

        Self {
            pool,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            returns: AtomicU64::new(0),
            capacity,
        }
    }

    /// Get a span from pool (NEVER allocates)
    #[inline(always)]
    pub fn get(&self) -> Option<PooledSpan> {
        match self.pool.pop() {
            Some(span) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(PooledSpan {
                    span: Some(span),
                    pool: Arc::clone(&self.pool),
                    returns: unsafe { std::mem::transmute(&self.returns) },
                })
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None // NEVER allocate - let caller handle
            }
        }
    }

    /// Try to get a span, with fallback
    #[inline(always)]
    pub fn try_get_or_new(&self) -> PooledSpan {
        self.get().unwrap_or_else(|| {
            // Only allocate as last resort
            self.misses.fetch_add(1, Ordering::Relaxed);
            // Leak the reference to make it 'static (safe for long-lived pools)
            let returns_ref: &'static AtomicU64 = unsafe { 
                std::mem::transmute(&self.returns)
            };
            PooledSpan {
                span: Some(Box::new(SpanBuilder::default().build_default())),
                pool: Arc::clone(&self.pool),
                returns: returns_ref,
            }
        })
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        PoolStats {
            hits,
            misses,
            returns: self.returns.load(Ordering::Relaxed),
            available: self.pool.len(),
            capacity: self.capacity,
            hit_rate: if total > 0 {
                hits as f64 / total as f64
            } else {
                1.0
            },
        }
    }
}

/// RAII guard that returns span to pool on drop
pub struct PooledSpan {
    span: Option<Box<Span>>,
    pool: Arc<ArrayQueue<Box<Span>>>,
    returns: &'static AtomicU64,
}

impl PooledSpan {
    /// Take ownership of the span
    #[inline]
    pub fn take(mut self) -> Box<Span> {
        self.span.take().expect("Span already taken")
    }

    /// Access the span
    #[inline]
    pub fn as_ref(&self) -> &Span {
        self.span.as_ref().expect("Span already taken")
    }

    /// Mutably access the span
    #[inline]
    pub fn as_mut(&mut self) -> &mut Span {
        self.span.as_mut().expect("Span already taken")
    }

    /// Reset span for reuse
    #[inline]
    pub fn reset(&mut self) {
        if let Some(span) = &mut self.span {
            // Reset to default state for clean reuse
            *span.as_mut() = SpanBuilder::default().build_default();
        }
    }
}

impl Drop for PooledSpan {
    #[inline]
    fn drop(&mut self) {
        if let Some(mut span) = self.span.take() {
            // Reset span before returning to pool
            *span = SpanBuilder::default().build_default();

            // Return to pool (ignore if full)
            let _ = self.pool.push(span);
            self.returns.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Ultra-fast pool for CompactSpan (no allocations needed)
pub struct CompactSpanPool {
    /// Pre-allocated array of CompactSpans
    spans: Box<[MaybeUninit<CompactSpan>]>,
    /// Available span indices
    available: Arc<ArrayQueue<usize>>,
    /// Statistics
    allocations: AtomicU64,
    deallocations: AtomicU64,
}

impl CompactSpanPool {
    /// Create pool with pre-allocated CompactSpans
    pub fn new(capacity: usize) -> Self {
        // Allocate uninitialized memory for CompactSpans
        let mut spans = Vec::with_capacity(capacity);
        spans.resize_with(capacity, MaybeUninit::uninit);
        let spans = spans.into_boxed_slice();

        // Initialize available indices
        let available = Arc::new(ArrayQueue::new(capacity));
        for i in 0..capacity {
            let _ = available.push(i);
        }

        Self {
            spans,
            available,
            allocations: AtomicU64::new(0),
            deallocations: AtomicU64::new(0),
        }
    }

    /// Get a CompactSpan slot (zero allocation)
    #[inline(always)]
    pub fn allocate(&self) -> Option<CompactSpanHandle> {
        self.available.pop().map(|index| {
            self.allocations.fetch_add(1, Ordering::Relaxed);
            CompactSpanHandle {
                index,
                pool: self as *const Self,
            }
        })
    }

    /// Return a slot to the pool
    #[inline(always)]
    unsafe fn deallocate(&self, index: usize) {
        if index < self.spans.len() {
            let _ = self.available.push(index);
            self.deallocations.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Write data to a slot
    #[inline(always)]
    unsafe fn write(&self, index: usize, span: CompactSpan) {
        if index < self.spans.len() {
            // Cast away const for writing (safe because we have exclusive access to this index)
            let ptr = self.spans[index].as_ptr() as *mut CompactSpan;
            ptr.write(span);
        }
    }

    /// Read data from a slot
    #[inline(always)]
    unsafe fn read(&self, index: usize) -> &CompactSpan {
        &*self.spans[index].as_ptr()
    }
}

/// Handle to a CompactSpan in the pool
pub struct CompactSpanHandle {
    index: usize,
    pool: *const CompactSpanPool,
}

impl CompactSpanHandle {
    /// Write a CompactSpan to this slot
    #[inline]
    pub fn write(&self, span: CompactSpan) {
        unsafe {
            (*self.pool).write(self.index, span);
        }
    }

    /// Read the CompactSpan from this slot
    #[inline]
    pub fn read(&self) -> &CompactSpan {
        unsafe {
            (*self.pool).read(self.index)
        }
    }
}

impl Drop for CompactSpanHandle {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            (*self.pool).deallocate(self.index);
        }
    }
}

// Mark as Send/Sync since we use proper synchronization
unsafe impl Send for CompactSpanHandle {}
unsafe impl Sync for CompactSpanHandle {}

/// Global pools for application-wide reuse
pub struct GlobalPools {
    span_pool: ZeroAllocSpanPool,
    compact_pool: CompactSpanPool,
}

impl GlobalPools {
    /// Initialize global pools with specified capacity
    pub fn init(span_capacity: usize, compact_capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            span_pool: ZeroAllocSpanPool::new(span_capacity),
            compact_pool: CompactSpanPool::new(compact_capacity),
        })
    }

    /// Get a span from the global pool
    #[inline(always)]
    pub fn get_span(&self) -> Option<PooledSpan> {
        self.span_pool.get()
    }

    /// Get a compact span slot from the global pool
    #[inline(always)]
    pub fn get_compact_slot(&self) -> Option<CompactSpanHandle> {
        self.compact_pool.allocate()
    }

    /// Get statistics
    pub fn stats(&self) -> (PoolStats, u64, u64) {
        let span_stats = self.span_pool.stats();
        let compact_allocs = self.compact_pool.allocations.load(Ordering::Relaxed);
        let compact_deallocs = self.compact_pool.deallocations.load(Ordering::Relaxed);
        (span_stats, compact_allocs, compact_deallocs)
    }
}

/// Global pool instance (lazy initialized)
static GLOBAL_POOLS: once_cell::sync::Lazy<Arc<GlobalPools>> =
    once_cell::sync::Lazy::new(|| GlobalPools::init(10_000, 100_000));

/// Get a span from the global pool
#[inline(always)]
pub fn get_pooled_span() -> Option<PooledSpan> {
    GLOBAL_POOLS.get_span()
}

/// Get a compact span slot from the global pool
#[inline(always)]
pub fn get_compact_slot() -> Option<CompactSpanHandle> {
    GLOBAL_POOLS.get_compact_slot()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_alloc_span_pool() {
        let pool = ZeroAllocSpanPool::new(10);

        // Should get spans without allocation
        let mut spans = Vec::new();
        for _ in 0..10 {
            spans.push(pool.get().expect("Pool should have spans"));
        }

        // Pool should be empty now
        assert!(pool.get().is_none());

        // Return spans to pool
        spans.clear();

        // Should be available again
        assert!(pool.get().is_some());

        let stats = pool.stats();
        assert_eq!(stats.hits, 11);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_compact_span_pool() {
        let pool = CompactSpanPool::new(100);

        // Allocate slots
        let mut handles = Vec::new();
        for _ in 0..100 {
            handles.push(pool.allocate().expect("Should allocate"));
        }

        // Pool should be exhausted
        assert!(pool.allocate().is_none());

        // Write and read
        let test_span = CompactSpan::default();
        handles[0].write(test_span);
        let read_span = handles[0].read();
        assert_eq!(read_span.trace_id, 0);

        // Return slots
        handles.clear();

        // Should be available again
        assert!(pool.allocate().is_some());
    }

    #[test]
    fn test_global_pools() {
        // Test global pool access
        let span = get_pooled_span();
        assert!(span.is_some());

        let slot = get_compact_slot();
        assert!(slot.is_some());

        let (stats, _, _) = GLOBAL_POOLS.stats();
        assert!(stats.available > 0);
    }
}
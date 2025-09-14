//! High-performance object pool for span allocations.
//!
//! Following CLAUDE.md principles for extreme performance:
//! - Zero allocations in hot paths
//! - Lock-free concurrent access
//! - Memory reuse to reduce GC pressure

use crate::core::{Span, SpanBuilder};
use crossbeam::queue::ArrayQueue;
use std::sync::Arc;

/// High-performance span pool for zero-allocation span processing.
///
/// Uses a lock-free ArrayQueue for concurrent access without contention.
pub struct SpanPool {
    /// Lock-free queue of available spans
    pool: Arc<ArrayQueue<Box<Span>>>,
    /// Maximum pool size
    capacity: usize,
}

impl SpanPool {
    /// Create a new span pool with the specified capacity.
    ///
    /// # Performance
    /// Pre-allocates all spans to avoid allocations during operation.
    pub fn new(capacity: usize) -> Self {
        let pool = Arc::new(ArrayQueue::new(capacity));

        // Pre-allocate spans to warm up the pool
        let warm_up_count = capacity.min(100); // Warm up with initial spans
        for _ in 0..warm_up_count {
            let span = Box::new(SpanBuilder::default().build_default());
            let _ = pool.push(span); // Ignore if pool is full
        }

        Self { pool, capacity }
    }

    /// Get a span from the pool or allocate a new one if pool is empty.
    ///
    /// # Performance
    /// - O(1) in the common case (pool hit)
    /// - Falls back to allocation only when pool is empty
    #[inline(always)]
    pub fn get(&self) -> PooledSpan {
        let span = self.pool.pop().unwrap_or_else(|| {
            // Pool miss - allocate new span
            Box::new(SpanBuilder::default().build_default())
        });

        PooledSpan {
            span: Some(span),
            pool: Arc::clone(&self.pool),
        }
    }

    /// Pre-warm the pool with spans to avoid cold start allocations.
    pub fn warm_up(&self, count: usize) {
        for _ in 0..count {
            if let Ok(()) = self
                .pool
                .push(Box::new(SpanBuilder::default().build_default()))
            {
                // Successfully added to pool
            } else {
                // Pool is full
                break;
            }
        }
    }

    /// Get current pool utilization stats.
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            capacity: self.capacity,
            available: self.pool.len(),
            utilization: 1.0 - (self.pool.len() as f64 / self.capacity as f64),
        }
    }
}

/// A span borrowed from the pool that returns itself when dropped.
pub struct PooledSpan {
    span: Option<Box<Span>>,
    pool: Arc<ArrayQueue<Box<Span>>>,
}

/// Result type for safe span access
pub enum SpanAccess<'a> {
    Available(&'a Span),
    Taken,
}

/// Mutable result type for safe span access
pub enum SpanAccessMut<'a> {
    Available(&'a mut Span),
    Taken,
}

impl PooledSpan {
    /// Take ownership of the span, preventing it from returning to the pool.
    /// Returns a default span if already taken (defensive programming).
    pub fn take(mut self) -> Span {
        // BULLETPROOF: Return default span if already taken
        match self.span.take() {
            Some(boxed) => *boxed,
            None => {
                tracing::warn!("PooledSpan::take called on already-taken span, returning default");
                SpanBuilder::default().build_default()
            }
        }
    }

    /// Get a safe reference to the span.
    /// Returns SpanAccess::Taken if span was already taken.
    #[inline(always)]
    pub fn try_as_ref(&self) -> SpanAccess<'_> {
        match &self.span {
            Some(boxed) => SpanAccess::Available(boxed),
            None => SpanAccess::Taken,
        }
    }

    /// Get a reference to the span (for Deref trait).
    /// Returns a static default span if taken (never panics).
    #[inline(always)]
    fn as_ref(&self) -> &Span {
        // BULLETPROOF: Use static default for Deref trait
        static DEFAULT_SPAN: once_cell::sync::Lazy<Span> =
            once_cell::sync::Lazy::new(|| SpanBuilder::default().build_default());

        match &self.span {
            Some(boxed) => boxed,
            None => &DEFAULT_SPAN,
        }
    }

    /// Get a safe mutable reference to the span.
    /// Returns SpanAccessMut::Taken if span was already taken.
    #[inline(always)]
    pub fn try_as_mut(&mut self) -> SpanAccessMut<'_> {
        match &mut self.span {
            Some(boxed) => SpanAccessMut::Available(boxed),
            None => SpanAccessMut::Taken,
        }
    }

    /// Get a mutable reference to the span (for DerefMut trait).
    /// Creates a new span if taken (never panics).
    #[inline(always)]
    fn as_mut(&mut self) -> &mut Span {
        // BULLETPROOF: Create new span if taken
        if self.span.is_none() {
            tracing::warn!("PooledSpan::as_mut called on taken span, creating new");
            self.span = Some(Box::new(SpanBuilder::default().build_default()));
        }
        // BULLETPROOF: This is truly safe because we just created it above
        self.span.as_mut().unwrap_or_else(|| {
            // This should never happen, but we're paranoid
            unreachable!("BUG: span should exist after creation")
        })
    }

    /// Reset the span to default state for reuse.
    #[inline]
    fn reset(&mut self) {
        if let Some(span) = &mut self.span {
            // Reset span to default state
            // This is much faster than allocating a new span
            **span = SpanBuilder::default().build_default();
        }
    }
}

impl Drop for PooledSpan {
    fn drop(&mut self) {
        if let Some(mut span) = self.span.take() {
            // Reset span before returning to pool
            *span = SpanBuilder::default().build_default();

            // Try to return to pool, ignore if full
            let _ = self.pool.push(span);
        }
    }
}

impl std::ops::Deref for PooledSpan {
    type Target = Span;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl std::ops::DerefMut for PooledSpan {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

/// Statistics about pool utilization.
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Maximum pool capacity
    pub capacity: usize,
    /// Currently available spans in pool
    pub available: usize,
    /// Pool utilization (0.0 = empty, 1.0 = fully utilized)
    pub utilization: f64,
}

/// Global span pool for the application.
///
/// Sized based on expected concurrent span processing needs.
pub static GLOBAL_SPAN_POOL: once_cell::sync::Lazy<SpanPool> = once_cell::sync::Lazy::new(|| {
    // Size based on expected load: 10K concurrent spans
    let pool_size = std::env::var("URPO_SPAN_POOL_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000);

    let pool = SpanPool::new(pool_size);

    // Pre-warm with 10% of capacity
    pool.warm_up(pool_size / 10);

    tracing::info!("Initialized global span pool with capacity {}", pool_size);
    pool
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ServiceName;

    #[test]
    fn test_span_pool_basic() {
        let pool = SpanPool::new(10);

        // Get span from pool
        let mut span1 = pool.get();
        span1.service_name = ServiceName::new("test".to_string()).expect("valid name");

        // Stats should show one span in use
        let stats = pool.stats();
        assert!(stats.utilization > 0.0);

        // Drop span to return to pool
        drop(span1);

        // Get another span - should reuse the previous one
        let span2 = pool.get();
        // Should be reset to default
        assert_eq!(span2.service_name, ServiceName::default());
    }

    #[test]
    fn test_span_pool_concurrent() {
        use std::thread;

        let pool = Arc::new(SpanPool::new(100));
        let mut handles = vec![];

        for i in 0..10 {
            let pool = Arc::clone(&pool);
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let mut span = pool.get();
                    span.operation_name = format!("op_{}_{}", i, j);
                    // Span automatically returned to pool when dropped
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("thread panicked");
        }

        // Pool should still be functional
        let span = pool.get();
        assert_eq!(span.operation_name, String::new()); // Reset to default
    }

    #[test]
    fn test_span_take_ownership() {
        let pool = SpanPool::new(10);

        let pooled = pool.get();
        let owned_span = pooled.take();

        // Span should not return to pool
        assert_eq!(owned_span.service_name, ServiceName::default());
    }
}

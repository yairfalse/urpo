//! Simple zero-allocation pool without lifetime issues

use crate::core::{Span, SpanBuilder};
use crossbeam::queue::ArrayQueue;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Simple pool stats
#[derive(Debug, Clone)]
pub struct SimplePoolStats {
    pub hits: u64,
    pub misses: u64,
    pub available: usize,
    pub capacity: usize,
}

/// Zero-allocation span pool
pub struct SimpleSpanPool {
    pool: Arc<ArrayQueue<Box<Span>>>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    capacity: usize,
}

impl SimpleSpanPool {
    /// Create a new pool pre-warmed with spans
    pub fn new(capacity: usize) -> Self {
        let pool = Arc::new(ArrayQueue::new(capacity));

        // Pre-allocate spans
        for _ in 0..capacity {
            let span = Box::new(SpanBuilder::default().build_default());
            let _ = pool.push(span);
        }

        Self {
            pool,
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            capacity,
        }
    }

    /// Get a span from pool (zero allocation if available)
    #[inline(always)]
    pub fn get(&self) -> Option<SimplePooledSpan> {
        match self.pool.pop() {
            Some(span) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(SimplePooledSpan {
                    span: Some(span),
                    pool: Arc::clone(&self.pool),
                })
            },
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            },
        }
    }

    /// Get stats
    pub fn stats(&self) -> SimplePoolStats {
        SimplePoolStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            available: self.pool.len(),
            capacity: self.capacity,
        }
    }
}

/// RAII wrapper that returns span to pool on drop
pub struct SimplePooledSpan {
    span: Option<Box<Span>>,
    pool: Arc<ArrayQueue<Box<Span>>>,
}

impl SimplePooledSpan {
    /// Access the span
    #[inline]
    pub fn as_ref(&self) -> &Span {
        self.span.as_ref().expect("Span taken")
    }

    /// Mutably access the span
    #[inline]
    pub fn as_mut(&mut self) -> &mut Span {
        self.span.as_mut().expect("Span taken")
    }

    /// Take ownership (won't return to pool)
    #[inline]
    pub fn take(mut self) -> Box<Span> {
        self.span.take().expect("Span already taken")
    }
}

impl Drop for SimplePooledSpan {
    #[inline]
    fn drop(&mut self) {
        if let Some(mut span) = self.span.take() {
            // Reset span for reuse
            *span = SpanBuilder::default().build_default();

            // Return to pool (ignore if full)
            let _ = self.pool.push(span);
        }
    }
}

/// Global pool for easy access
static GLOBAL_SIMPLE_POOL: once_cell::sync::Lazy<SimpleSpanPool> =
    once_cell::sync::Lazy::new(|| SimpleSpanPool::new(10_000));

/// Get a span from the global pool
#[inline(always)]
pub fn get_span() -> Option<SimplePooledSpan> {
    GLOBAL_SIMPLE_POOL.get()
}

/// Get global pool stats
pub fn global_stats() -> SimplePoolStats {
    GLOBAL_SIMPLE_POOL.stats()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_pool() {
        let pool = SimpleSpanPool::new(10);

        // Should get spans from pool
        let mut spans = Vec::new();
        for _ in 0..10 {
            spans.push(pool.get().expect("Should get span"));
        }

        // Pool should be empty
        assert!(pool.get().is_none());

        let stats = pool.stats();
        assert_eq!(stats.hits, 10);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.available, 0);

        // Return spans
        spans.clear();

        // Should be available again
        assert!(pool.get().is_some());
    }
}

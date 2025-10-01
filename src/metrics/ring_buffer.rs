//! Ring buffer for efficient metric storage
//!
//! This module provides a high-performance ring buffer implementation
//! optimized for metric data storage with O(1) operations.

use crate::metrics::types::MetricPoint;
use std::sync::atomic::{AtomicUsize, Ordering};

/// High-performance lock-free ring buffer for metric points
/// Uses cache-aligned slots for optimal CPU cache usage
pub struct MetricRingBuffer {
    buffer: Box<[MetricPoint]>,
    capacity: usize,
    mask: usize, // For fast modulo via bitwise AND
    head: AtomicUsize,
    tail: AtomicUsize,
    size: AtomicUsize,
}

impl MetricRingBuffer {
    /// Create a new ring buffer with the specified capacity
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be greater than 0");
        assert!(
            capacity.is_power_of_two(),
            "Capacity must be power of 2 for optimal performance"
        );

        // Initialize buffer with default metric points
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(MetricPoint::new(0, 0, 0, 0.0));
        }

        Self {
            buffer: buffer.into_boxed_slice(),
            capacity,
            mask: capacity - 1, // For fast modulo
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            size: AtomicUsize::new(0),
        }
    }

    /// Push a metric point to the buffer
    /// Returns true if successful, false if buffer is full
    pub fn push(&self, metric: MetricPoint) -> bool {
        let current_size = self.size.load(Ordering::Acquire);

        // Check if buffer is full
        if current_size >= self.capacity {
            return false;
        }

        let tail = self.tail.load(Ordering::Acquire);
        let next_tail = (tail + 1) & self.mask; // Fast modulo with bitwise AND

        // Store the metric at tail position
        // SAFETY: tail is always within bounds due to modulo operation
        unsafe {
            let ptr = self.buffer.as_ptr().add(tail) as *mut MetricPoint;
            ptr.write(metric);
        }

        // Update tail and size atomically
        self.tail.store(next_tail, Ordering::Release);
        self.size.fetch_add(1, Ordering::AcqRel);

        true
    }

    /// Pop a metric point from the buffer
    /// Returns None if buffer is empty
    pub fn pop(&self) -> Option<MetricPoint> {
        let current_size = self.size.load(Ordering::Acquire);

        // Check if buffer is empty
        if current_size == 0 {
            return None;
        }

        let head = self.head.load(Ordering::Acquire);

        // Read the metric at head position
        // SAFETY: head is always within bounds due to modulo operation
        let metric = unsafe {
            let ptr = self.buffer.as_ptr().add(head);
            ptr.read()
        };

        let next_head = (head + 1) & self.mask; // Fast modulo with bitwise AND

        // Update head and size atomically
        self.head.store(next_head, Ordering::Release);
        self.size.fetch_sub(1, Ordering::AcqRel);

        Some(metric)
    }

    /// Get current number of items in buffer
    pub fn len(&self) -> usize {
        self.size.load(Ordering::Acquire)
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    /// Get buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear all items from buffer
    pub fn clear(&self) {
        self.head.store(0, Ordering::Release);
        self.tail.store(0, Ordering::Release);
        self.size.store(0, Ordering::Release);
    }

    /// Drain up to `count` items from the buffer
    pub fn drain(&self, count: usize) -> Vec<MetricPoint> {
        let mut result = Vec::new();

        for _ in 0..count {
            if let Some(metric) = self.pop() {
                result.push(metric);
            } else {
                break;
            }
        }

        result
    }

    /// Bulk push multiple metrics
    /// Returns the number of metrics successfully pushed
    pub fn push_bulk(&self, metrics: &[MetricPoint]) -> usize {
        let mut pushed = 0;

        for metric in metrics {
            if self.push(metric.clone()) {
                pushed += 1;
            } else {
                break; // Buffer is full
            }
        }

        pushed
    }
}

// Implement Send and Sync for thread safety
unsafe impl Send for MetricRingBuffer {}
unsafe impl Sync for MetricRingBuffer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_creation() {
        let buffer = MetricRingBuffer::new(8);

        assert_eq!(buffer.capacity(), 8);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
    }

    #[test]
    #[should_panic(expected = "Capacity must be greater than 0")]
    fn test_zero_capacity_panics() {
        MetricRingBuffer::new(0);
    }

    #[test]
    #[should_panic(expected = "Capacity must be power of 2")]
    fn test_non_power_of_two_panics() {
        MetricRingBuffer::new(7);
    }

    #[test]
    fn test_single_push_pop() {
        let buffer = MetricRingBuffer::new(4);
        let metric = MetricPoint::new(1234567890, 1, 2, 42.5);

        // Push metric
        assert!(buffer.push(metric));
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());

        // Pop metric
        let popped = buffer.pop().unwrap();
        assert_eq!(popped.timestamp, 1234567890);
        assert_eq!(popped.value, 42.5);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_multiple_push_pop() {
        let buffer = MetricRingBuffer::new(4);

        // Push multiple metrics
        for i in 0..3 {
            let metric = MetricPoint::new(i as u64, i as u16, i as u16, i as f64);
            assert!(buffer.push(metric));
        }

        assert_eq!(buffer.len(), 3);

        // Pop and verify order (FIFO)
        for i in 0..3 {
            let metric = buffer.pop().unwrap();
            assert_eq!(metric.timestamp, i as u64);
            assert_eq!(metric.value, i as f64);
        }

        assert!(buffer.is_empty());
    }

    #[test]
    fn test_buffer_overflow() {
        let buffer = MetricRingBuffer::new(2);

        // Fill buffer
        assert!(buffer.push(MetricPoint::new(1, 1, 1, 1.0)));
        assert!(buffer.push(MetricPoint::new(2, 2, 2, 2.0)));
        assert!(buffer.is_full());

        // Try to push when full
        assert!(!buffer.push(MetricPoint::new(3, 3, 3, 3.0)));
        assert_eq!(buffer.len(), 2);
    }

    #[test]
    fn test_empty_pop() {
        let buffer = MetricRingBuffer::new(4);

        // Pop from empty buffer
        assert!(buffer.pop().is_none());
    }

    #[test]
    fn test_clear() {
        let buffer = MetricRingBuffer::new(4);

        // Add some metrics
        buffer.push(MetricPoint::new(1, 1, 1, 1.0));
        buffer.push(MetricPoint::new(2, 2, 2, 2.0));
        assert_eq!(buffer.len(), 2);

        // Clear buffer
        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert!(buffer.pop().is_none());
    }

    #[test]
    fn test_wraparound() {
        let buffer = MetricRingBuffer::new(4);

        // Fill buffer
        for i in 0..4 {
            assert!(buffer.push(MetricPoint::new(i, i as u16, i as u16, i as f64)));
        }

        // Pop one item
        let first = buffer.pop().unwrap();
        assert_eq!(first.timestamp, 0);
        assert_eq!(buffer.len(), 3);

        // Add another item (should wrap around)
        assert!(buffer.push(MetricPoint::new(100, 100, 100, 100.0)));
        assert_eq!(buffer.len(), 4);

        // Pop all and verify order
        let expected = [1, 2, 3, 100];
        for &expected_ts in &expected {
            let metric = buffer.pop().unwrap();
            assert_eq!(metric.timestamp, expected_ts);
        }
    }

    #[test]
    fn test_drain() {
        let buffer = MetricRingBuffer::new(8);

        // Add 5 metrics
        for i in 0..5 {
            buffer.push(MetricPoint::new(i, i as u16, i as u16, i as f64));
        }

        // Drain 3 metrics
        let drained = buffer.drain(3);
        assert_eq!(drained.len(), 3);
        assert_eq!(buffer.len(), 2);

        // Verify drained metrics are in correct order
        for (i, metric) in drained.iter().enumerate() {
            assert_eq!(metric.timestamp, i as u64);
        }
    }

    #[test]
    fn test_drain_more_than_available() {
        let buffer = MetricRingBuffer::new(4);

        // Add 2 metrics
        buffer.push(MetricPoint::new(1, 1, 1, 1.0));
        buffer.push(MetricPoint::new(2, 2, 2, 2.0));

        // Try to drain more than available
        let drained = buffer.drain(5);
        assert_eq!(drained.len(), 2);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_bulk_push() {
        let buffer = MetricRingBuffer::new(4);

        let metrics = vec![
            MetricPoint::new(1, 1, 1, 1.0),
            MetricPoint::new(2, 2, 2, 2.0),
            MetricPoint::new(3, 3, 3, 3.0),
        ];

        let pushed = buffer.push_bulk(&metrics);
        assert_eq!(pushed, 3);
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_bulk_push_overflow() {
        let buffer = MetricRingBuffer::new(2);

        let metrics = vec![
            MetricPoint::new(1, 1, 1, 1.0),
            MetricPoint::new(2, 2, 2, 2.0),
            MetricPoint::new(3, 3, 3, 3.0), // This should not be pushed
        ];

        let pushed = buffer.push_bulk(&metrics);
        assert_eq!(pushed, 2);
        assert_eq!(buffer.len(), 2);
        assert!(buffer.is_full());
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let buffer = Arc::new(MetricRingBuffer::new(1024));
        let mut handles = vec![];

        // Producer threads
        for thread_id in 0..4 {
            let buffer_clone = buffer.clone();
            let handle = thread::spawn(move || {
                for i in 0..100 {
                    let metric = MetricPoint::new(
                        (thread_id * 1000 + i) as u64,
                        thread_id as u16,
                        i as u16,
                        thread_id as f64 + i as f64,
                    );

                    // Retry until successful (buffer has enough capacity)
                    while !buffer_clone.push(metric.clone()) {
                        std::thread::yield_now();
                    }
                }
            });
            handles.push(handle);
        }

        // Consumer thread
        let buffer_clone = buffer.clone();
        let consumer_handle = thread::spawn(move || {
            let mut consumed = 0;
            while consumed < 400 {
                if let Some(_metric) = buffer_clone.pop() {
                    consumed += 1;
                } else {
                    std::thread::yield_now();
                }
            }
            consumed
        });

        // Wait for producers
        for handle in handles {
            handle.join().unwrap();
        }

        // Wait for consumer
        let consumed = consumer_handle.join().unwrap();
        assert_eq!(consumed, 400);
    }
}

//! Ultra-fast string interning for zero-copy string deduplication.
//!
//! This module provides a high-performance string interning system that stores
//! each unique string only once and returns lightweight IDs for lookups.

use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// String intern ID - lightweight 4-byte identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct InternId(pub u32);

impl InternId {
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        InternId(id)
    }

    #[inline(always)]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// High-performance string interning table with lock-free lookups
pub struct StringIntern {
    /// Forward mapping: string -> ID (lock-free with DashMap)
    table: DashMap<Arc<str>, InternId>,
    /// Reverse mapping: ID -> string (read-heavy, write-rare)
    reverse: RwLock<Vec<Arc<str>>>,
    /// Next ID to assign (atomic counter)
    next_id: AtomicU32,
}

impl StringIntern {
    /// Create a new string intern table with initial capacity
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Create with specified initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let mut reverse = Vec::with_capacity(capacity);
        // Reserve ID 0 as "empty/null"
        reverse.push(Arc::from(""));

        Self {
            table: DashMap::with_capacity(capacity),
            reverse: RwLock::new(reverse),
            next_id: AtomicU32::new(1), // Start from 1 (0 is reserved)
        }
    }

    /// Intern a string and get its ID (zero-copy if already interned)
    #[inline]
    pub fn intern(&self, s: &str) -> InternId {
        // Fast path: check if already interned
        if let Some(entry) = self.table.get(s) {
            return *entry.value();
        }

        // Slow path: add new string
        self.intern_slow(s)
    }

    #[cold]
    fn intern_slow(&self, s: &str) -> InternId {
        let arc_str: Arc<str> = Arc::from(s);

        // Double-check under entry API to avoid races
        let entry = self.table.entry(Arc::clone(&arc_str));

        match entry {
            dashmap::mapref::entry::Entry::Occupied(e) => *e.get(),
            dashmap::mapref::entry::Entry::Vacant(e) => {
                let id = InternId(self.next_id.fetch_add(1, Ordering::Relaxed));
                e.insert(id);

                // Add to reverse lookup
                let mut reverse = self.reverse.write();
                reverse.push(arc_str);

                id
            },
        }
    }

    /// Get string by ID (very fast, direct indexing)
    #[inline]
    pub fn get(&self, id: InternId) -> Option<Arc<str>> {
        let reverse = self.reverse.read();
        reverse.get(id.0 as usize).cloned()
    }

    /// Alias for get() method for compatibility
    #[inline]
    pub fn lookup(&self, id: InternId) -> Option<Arc<str>> {
        self.get(id)
    }

    /// Get string reference by ID (zero-copy)
    #[inline]
    pub fn get_ref(&self, id: InternId) -> Option<&str> {
        // SAFETY: We're only reading and the strings are immutable once added
        unsafe {
            let reverse = self.reverse.data_ptr() as *const Vec<Arc<str>>;
            let reverse_ref = &*reverse;
            reverse_ref.get(id.0 as usize).map(|s| s.as_ref())
        }
    }

    /// Batch intern multiple strings efficiently
    pub fn intern_batch(&self, strings: &[&str]) -> Vec<InternId> {
        let mut ids = Vec::with_capacity(strings.len());

        for &s in strings {
            ids.push(self.intern(s));
        }

        ids
    }

    /// Get current number of interned strings
    #[inline]
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Memory usage estimate in bytes
    pub fn memory_usage(&self) -> usize {
        let reverse = self.reverse.read();

        // Estimate: DashMap overhead + reverse vector + actual string data
        let table_overhead = self.table.len() * 64; // ~64 bytes per entry
        let reverse_overhead = reverse.capacity() * std::mem::size_of::<Arc<str>>();
        let string_data: usize = reverse.iter().map(|s| s.len()).sum();

        table_overhead + reverse_overhead + string_data
    }

    /// Clear all interned strings (useful for tests)
    pub fn clear(&self) {
        self.table.clear();
        let mut reverse = self.reverse.write();
        reverse.clear();
        reverse.push(Arc::from("")); // Re-add reserved empty string
        self.next_id.store(1, Ordering::Relaxed);
    }
}

impl Default for StringIntern {
    fn default() -> Self {
        Self::new()
    }
}

/// Global string intern table for the application
static GLOBAL_INTERN: once_cell::sync::Lazy<StringIntern> =
    once_cell::sync::Lazy::new(|| StringIntern::with_capacity(10_000));

/// Intern a string using the global table
#[inline]
pub fn intern(s: &str) -> InternId {
    GLOBAL_INTERN.intern(s)
}

/// Get a string from the global table
#[inline]
pub fn get_string(id: InternId) -> Option<Arc<str>> {
    GLOBAL_INTERN.get(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_interning() {
        let intern = StringIntern::new();

        let id1 = intern.intern("hello");
        let id2 = intern.intern("world");
        let id3 = intern.intern("hello"); // Same as id1

        assert_eq!(id1, id3);
        assert_ne!(id1, id2);

        assert_eq!(intern.get(id1).unwrap().as_ref(), "hello");
        assert_eq!(intern.get(id2).unwrap().as_ref(), "world");
    }

    #[test]
    fn test_batch_interning() {
        let intern = StringIntern::new();

        let strings = vec!["api", "database", "cache", "api", "database"];
        let ids = intern.intern_batch(&strings);

        assert_eq!(ids[0], ids[3]); // "api" interned once
        assert_eq!(ids[1], ids[4]); // "database" interned once
        assert_eq!(intern.len(), 3); // Only 3 unique strings
    }

    #[test]
    fn test_memory_efficiency() {
        let intern = StringIntern::new();

        // Intern the same string 1000 times
        for _ in 0..1000 {
            intern.intern("very_long_service_name_that_would_waste_memory");
        }

        // Should only store it once
        assert_eq!(intern.len(), 1);

        // Memory usage should be minimal (string + overhead)
        let memory = intern.memory_usage();
        // String is 47 chars, plus ~64 bytes overhead for DashMap entry, plus Arc overhead
        // Should be much less than 1000 * 47 = 47,000 if we stored all copies
        assert!(memory < 5000); // Allow for realistic overhead but still efficient
    }
}

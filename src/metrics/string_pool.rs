//! String interning pool for efficient metric name storage
//!
//! This module provides a high-performance string interning system
//! that reduces memory usage and improves cache locality.

use dashmap::DashMap;
use std::sync::Arc;

/// Index into the string pool
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StringId(pub u16);

/// String interning pool for metric names and labels
pub struct StringPool {
    strings: DashMap<Arc<str>, StringId>,
    reverse: DashMap<StringId, Arc<str>>,
    next_id: std::sync::atomic::AtomicU16,
}

impl StringPool {
    pub fn new() -> Self {
        Self {
            strings: DashMap::new(),
            reverse: DashMap::new(),
            next_id: std::sync::atomic::AtomicU16::new(0),
        }
    }

    pub fn intern(&self, s: &str) -> StringId {
        let arc_str: Arc<str> = Arc::from(s);

        if let Some(id) = self.strings.get(&arc_str) {
            return *id;
        }

        let id = StringId(
            self.next_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        );

        self.strings.insert(Arc::clone(&arc_str), id);
        self.reverse.insert(id, arc_str);

        id
    }

    pub fn get(&self, id: StringId) -> Option<Arc<str>> {
        self.reverse.get(&id).map(|entry| entry.clone())
    }

    pub fn get_or_intern(&self, s: &str) -> (StringId, Arc<str>) {
        let arc_str: Arc<str> = Arc::from(s);

        if let Some(id) = self.strings.get(&arc_str) {
            return (*id, arc_str);
        }

        let id = StringId(
            self.next_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        );

        self.strings.insert(Arc::clone(&arc_str), id);
        self.reverse.insert(id, Arc::clone(&arc_str));

        (id, arc_str)
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn clear(&self) {
        self.strings.clear();
        self.reverse.clear();
        self.next_id.store(0, std::sync::atomic::Ordering::SeqCst);
    }
}

impl Default for StringPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_returns_same_id() {
        let pool = StringPool::new();

        let id1 = pool.intern("http.request.duration");
        let id2 = pool.intern("http.request.duration");

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_intern_different_strings() {
        let pool = StringPool::new();

        let id1 = pool.intern("metric.one");
        let id2 = pool.intern("metric.two");

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_get_interned_string() {
        let pool = StringPool::new();

        let id = pool.intern("database.query.time");
        let retrieved = pool.get(id);

        assert_eq!(retrieved, Some(Arc::from("database.query.time")));
    }

    #[test]
    fn test_get_nonexistent_string() {
        let pool = StringPool::new();

        let fake_id = StringId(9999);
        let retrieved = pool.get(fake_id);

        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_get_or_intern() {
        let pool = StringPool::new();

        let (id1, s1) = pool.get_or_intern("cpu.usage");
        let (id2, s2) = pool.get_or_intern("cpu.usage");

        assert_eq!(id1, id2);
        assert_eq!(s1, s2);
        assert_eq!(&*s1, "cpu.usage");
    }

    #[test]
    fn test_pool_length() {
        let pool = StringPool::new();

        assert_eq!(pool.len(), 0);

        pool.intern("metric1");
        assert_eq!(pool.len(), 1);

        pool.intern("metric2");
        assert_eq!(pool.len(), 2);

        pool.intern("metric1");
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn test_clear_pool() {
        let pool = StringPool::new();

        pool.intern("metric1");
        pool.intern("metric2");
        assert_eq!(pool.len(), 2);

        pool.clear();
        assert_eq!(pool.len(), 0);

        let id = pool.intern("metric3");
        assert_eq!(id.0, 0);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc as StdArc;
        use std::thread;

        let pool = StdArc::new(StringPool::new());
        let mut handles = vec![];

        for i in 0..10 {
            let pool_clone = Arc::clone(&pool);
            let handle = thread::spawn(move || {
                let metric_name = format!("metric.thread.{}", i);
                pool_clone.intern(&metric_name)
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(pool.len(), 10);
    }

    #[test]
    fn test_string_reuse_saves_memory() {
        let pool = StringPool::new();

        let long_string = "very.long.metric.name.that.takes.up.memory";

        let id1 = pool.intern(long_string);
        let id2 = pool.intern(long_string);

        let str1 = pool.get(id1).unwrap();
        let str2 = pool.get(id2).unwrap();

        assert!(Arc::ptr_eq(&str1, &str2));
    }
}

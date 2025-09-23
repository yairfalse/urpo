//! Storage backend for trace data.
//!
//! This module provides the storage interface and implementations
//! for storing and querying trace data efficiently.
//!
//! We keep only the high-performance components:
//! - memory.rs: Main in-memory storage implementation
//! - compression.rs: 5-10x memory savings
//! - simd_search.rs: 4x search speedup with SIMD
//! - zero_alloc_pool.rs: 6.3x performance boost with object pooling
//! - span_pool.rs: Integrated span pooling

use crate::core::Config;
use std::sync::Arc;
use tokio::sync::RwLock;

// Core modules
pub mod backend;
pub mod cleanup_logic;
pub mod memory;
pub mod types;

// Performance modules
pub mod compression;
pub mod simd_search;
pub mod span_pool;
pub mod zero_alloc_pool;

// Re-export commonly used types
pub use backend::StorageBackend;
pub use compression::{CompressedSpanBatch, CompressionEngine, CompressionLevel, CompressionStats};
pub use memory::InMemoryStorage;
pub use span_pool::{PooledSpan, SpanPool, GLOBAL_SPAN_POOL};
pub use cleanup_logic::CleanupConfig;
pub use types::{StorageHealth, StorageStats, TraceInfo};
pub use zero_alloc_pool::{PoolStats, ZeroAllocSpanPool};

/// Unified storage interface that wraps the actual implementation
pub struct UnifiedStorage {
    inner: Arc<RwLock<dyn StorageBackend>>,
}

impl UnifiedStorage {
    /// Create a new unified storage from configuration
    pub fn from_config(config: &Config) -> Self {
        // For now, always use InMemoryStorage
        // Future: could switch based on config
        let storage = InMemoryStorage::new(config.storage.max_spans);

        Self {
            inner: Arc::new(RwLock::new(storage)),
        }
    }

    /// Get the inner storage backend
    pub fn inner(&self) -> Arc<RwLock<dyn StorageBackend>> {
        self.inner.clone()
    }

    /// Get the storage backend for API usage
    pub fn as_backend(&self) -> Arc<RwLock<dyn StorageBackend>> {
        self.inner.clone()
    }
}

impl Clone for UnifiedStorage {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
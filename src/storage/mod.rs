//! Storage backend for trace data.
//!
//! This module provides the storage interface and implementations
//! for storing and querying trace data efficiently.

use crate::core::Config;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod aggregator;
pub mod archive;
pub mod archive_manager;
pub mod degradation;
pub mod engine;
pub mod fake_spans;
pub mod performance;
pub mod search;
// pub mod async_archive;
// pub mod async_archive_reader;
pub mod archive_integration;
pub mod backend;
pub mod buffered;
pub mod health;
pub mod manager;
pub mod memory;
pub mod span_pool;
pub mod tiered_engine;
pub mod types;
pub mod ultra_fast;

// Re-export commonly used types
pub use backend::StorageBackend;
pub use buffered::{BufferConfig, BufferStats, BufferedStorage};
pub use fake_spans::SpanGenerator;
pub use manager::StorageManager;
pub use memory::InMemoryStorage;
pub use performance::PerformanceManager;
pub use span_pool::{PooledSpan, SpanPool, GLOBAL_SPAN_POOL};
pub use types::{CleanupConfig, StorageHealth, StorageStats, TraceInfo};
// pub use async_archive::{AsyncArchiveWriter, FlushTask};
// pub use async_archive_reader::{AsyncArchiveReader, MappedArchive, ArchiveStats};
pub use tiered_engine::{TieredConfig, TieredStorageEngine, TieredStorageStats};
pub use ultra_fast::{BitmapIndices, CompactSpan, HotTraceRing, StringIntern, UltraFastStorage};

// UnifiedStorage is defined below

/// Unified storage wrapper for consistent backend usage across the codebase.
///
/// This provides a clean interface for creating storage backends without
/// exposing the complex Arc<RwLock<dyn StorageBackend>> type everywhere.
pub struct UnifiedStorage {
    inner: Arc<RwLock<InMemoryStorage>>,
}

impl UnifiedStorage {
    /// Create a new unified storage with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(InMemoryStorage::new(capacity))),
        }
    }

    /// Create a new unified storage with custom cleanup configuration.
    pub fn with_cleanup_config(capacity: usize, cleanup_config: memory::CleanupConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(InMemoryStorage::with_cleanup_config(
                capacity,
                cleanup_config,
            ))),
        }
    }

    /// Create a new unified storage from application configuration.
    pub fn from_config(config: &Config) -> Self {
        Self {
            inner: Arc::new(RwLock::new(InMemoryStorage::with_config(config))),
        }
    }

    /// Get the storage backend for use with APIs and services.
    ///
    /// This returns the standard Arc<RwLock<dyn StorageBackend>> type
    /// that's used throughout the application.
    pub fn as_backend(&self) -> Arc<RwLock<dyn StorageBackend>> {
        self.inner.clone() as Arc<RwLock<dyn StorageBackend>>
    }

    /// Get direct access to the inner storage for type-specific operations.
    ///
    /// Use this when you need access to InMemoryStorage-specific methods
    /// that aren't part of the StorageBackend trait.
    pub fn inner(&self) -> &Arc<RwLock<InMemoryStorage>> {
        &self.inner
    }

    /// Clone the storage reference for use in multiple contexts.
    pub fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Default for UnifiedStorage {
    fn default() -> Self {
        Self::new(100_000) // Default to 100k spans capacity
    }
}

// Re-export for backward compatibility - all types now defined in submodules

#[cfg(test)]
mod unified_storage_tests {
    use super::*;

    #[test]
    fn test_unified_storage_creation() {
        // Simple creation
        let storage = UnifiedStorage::new(1000);
        let _backend = storage.as_backend();
        // tokio::sync::RwLock doesn't have is_poisoned method

        // With default
        let storage = UnifiedStorage::default();
        let _backend = storage.as_backend();

        // Clone works correctly
        let storage2 = storage.clone();
        assert!(Arc::ptr_eq(storage.inner(), storage2.inner()));
    }
}

/* Example usage patterns for UnifiedStorage:

```rust,no_run
use urpo_lib::storage::UnifiedStorage;
use urpo_lib::api::{ApiConfig, start_server_with_storage};
use urpo_lib::receiver::OtelReceiver;
use urpo_lib::monitoring::Monitor;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create unified storage - much cleaner than Arc<RwLock<dyn StorageBackend>>
    let storage = UnifiedStorage::new(100_000);

    // Use with API server
    let api_config = ApiConfig::default();
    tokio::spawn(start_server_with_storage(&storage, api_config));

    // Use with OTEL receiver
    let monitor = Arc::new(Monitor::new("urpo"));
    let receiver = OtelReceiver::with_storage(4317, 4318, &storage, monitor);

    // Storage can be shared easily without complex type annotations
    let storage_clone = storage.clone();
    tokio::spawn(async move {
        // Use storage_clone in another task
        let _backend = storage_clone.as_backend();
        // ... work with backend
    });

    Ok(())
}
```
*/

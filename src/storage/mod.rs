//! Storage backend for trace data.
//!
//! This module provides the storage interface and implementations
//! for storing and querying trace data efficiently.

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::core::Config;

pub mod fake_spans;
pub mod aggregator;
pub mod performance;
pub mod degradation;
pub mod search;
pub mod engine;
pub mod archive;
pub mod archive_manager;
pub mod memory;
pub mod manager;
pub mod types;
pub mod backend;

// Re-export commonly used types
pub use fake_spans::SpanGenerator;
pub use performance::PerformanceManager;
pub use memory::InMemoryStorage;
pub use manager::StorageManager;
pub use types::{TraceInfo, StorageStats, StorageHealth, CleanupConfig};
pub use backend::StorageBackend;

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
            inner: Arc::new(RwLock::new(InMemoryStorage::with_cleanup_config(capacity, cleanup_config))),
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


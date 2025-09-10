//! Storage backend for trace data.
//!
//! This module provides the storage interface and implementations
//! for storing and querying trace data efficiently.

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

// Re-export for backward compatibility - all types now defined in submodules


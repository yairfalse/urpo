//! TraceQL-like query language for Urpo.
//!
//! Provides a powerful yet simple query language for filtering and searching traces.
//! Inspired by Grafana Tempo's TraceQL but optimized for Urpo's architecture.

pub mod ast;
pub mod executor;
pub mod parser;

use crate::core::{Result, UrpoError};
use crate::storage::StorageBackend;
use std::sync::Arc;

pub use ast::{Query, QueryFilter, Operator, Value, LogicalOp};
pub use executor::QueryExecutor;
pub use parser::parse_query;

/// High-level query API
pub struct QueryEngine {
    executor: QueryExecutor,
}

impl QueryEngine {
    /// Create a new query engine with the given storage backend
    pub fn new(storage: Arc<tokio::sync::RwLock<dyn StorageBackend>>) -> Self {
        Self {
            executor: QueryExecutor::new(storage),
        }
    }

    /// Execute a TraceQL query string
    pub async fn execute(&self, query_str: &str, limit: Option<usize>) -> Result<QueryResult> {
        // Parse the query
        let query = parse_query(query_str)?;

        // Execute it
        self.executor.execute(query, limit).await
    }

    /// Validate a query without executing it
    pub fn validate(&self, query_str: &str) -> Result<()> {
        parse_query(query_str)?;
        Ok(())
    }
}

/// Query execution result
#[derive(Debug, Clone, serde::Serialize)]
pub struct QueryResult {
    /// Matching trace IDs
    pub trace_ids: Vec<String>,
    /// Total matches (before limit)
    pub total_matches: usize,
    /// Query execution time in milliseconds
    pub query_time_ms: u64,
    /// Whether results were limited
    pub limited: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_queries() {
        // Test that basic queries parse correctly
        assert!(parse_query("service=\"api\"").is_ok());
        assert!(parse_query("duration > 100ms").is_ok());
        assert!(parse_query("status = error").is_ok());
    }

    #[test]
    fn test_complex_queries() {
        // Test complex boolean queries
        assert!(parse_query("service=\"api\" && duration > 100ms").is_ok());
        assert!(parse_query("status = error || duration > 1s").is_ok());
        assert!(parse_query("service=\"frontend\" && (status = error || duration > 500ms)").is_ok());
    }

    #[test]
    fn test_attribute_queries() {
        // Test attribute queries
        assert!(parse_query("http.status_code = 500").is_ok());
        assert!(parse_query("span.kind = \"server\"").is_ok());
    }
}
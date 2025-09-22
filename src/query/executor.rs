//! Query executor that runs parsed queries against the storage backend.

use super::ast::*;
use super::QueryResult;
use crate::core::{Result, ServiceName, SpanStatus};
use crate::storage::StorageBackend;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

/// Query executor that runs queries against the storage
pub struct QueryExecutor {
    storage: Arc<tokio::sync::RwLock<dyn StorageBackend>>,
}

impl QueryExecutor {
    /// Create a new query executor
    pub fn new(storage: Arc<tokio::sync::RwLock<dyn StorageBackend>>) -> Self {
        Self { storage }
    }

    /// Execute a parsed query
    pub async fn execute(&self, query: Query, limit: Option<usize>) -> Result<QueryResult> {
        let start = Instant::now();
        let limit = limit.unwrap_or(1000).min(10000); // Cap at 10k for safety

        // Get all trace IDs matching the filter
        let storage = self.storage.read().await;
        let matching_traces = self.execute_filter(&*storage, &query.filter, limit).await?;

        // Convert to string IDs
        let trace_ids: Vec<String> = matching_traces
            .iter()
            .take(limit)
            .map(|id| format!("{:032x}", id))
            .collect();

        let total_matches = matching_traces.len();
        let limited = total_matches > limit;
        let query_time_ms = start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            trace_ids,
            total_matches,
            query_time_ms,
            limited,
        })
    }

    /// Execute a filter against the storage
    async fn execute_filter(
        &self,
        storage: &dyn StorageBackend,
        filter: &QueryFilter,
        limit: usize,
    ) -> Result<Vec<u128>> {
        match filter {
            QueryFilter::All => {
                // Return all traces (up to limit)
                let _stats = storage.get_stats().await?;
                // We need to get all traces - this is inefficient but works for now
                // In a real implementation, we'd have a method to list all trace IDs
                Ok(vec![]) // TODO: Implement get_all_trace_ids in StorageBackend
            },

            QueryFilter::Comparison { field, op, value } => {
                self.execute_comparison(storage, field, op, value, limit)
                    .await
            },

            QueryFilter::Logical { op, left, right } => {
                let left_results = Box::pin(self.execute_filter(storage, left, limit * 2)).await?;
                let right_results =
                    Box::pin(self.execute_filter(storage, right, limit * 2)).await?;

                match op {
                    LogicalOp::And => {
                        // Intersection
                        let left_set: HashSet<u128> = left_results.into_iter().collect();
                        let result: Vec<u128> = right_results
                            .into_iter()
                            .filter(|id| left_set.contains(id))
                            .take(limit)
                            .collect();
                        Ok(result)
                    },
                    LogicalOp::Or => {
                        // Union
                        let mut seen = HashSet::new();
                        let mut result = Vec::new();

                        for id in left_results.into_iter().chain(right_results) {
                            if seen.insert(id) && result.len() < limit {
                                result.push(id);
                            }
                            if result.len() >= limit {
                                break;
                            }
                        }
                        Ok(result)
                    },
                }
            },

            QueryFilter::Group(inner) => Box::pin(self.execute_filter(storage, inner, limit)).await,
        }
    }

    /// Execute a comparison filter
    async fn execute_comparison(
        &self,
        storage: &dyn StorageBackend,
        field: &Field,
        op: &Operator,
        value: &Value,
        limit: usize,
    ) -> Result<Vec<u128>> {
        match field {
            Field::Service => {
                // Service name comparison
                if let Value::String(service_str) = value {
                    if *op == Operator::Eq {
                        // Fast path: exact service match
                        if let Ok(service_name) = ServiceName::new(service_str.clone()) {
                            let spans = storage
                                .get_service_spans(
                                    &service_name,
                                    std::time::SystemTime::now()
                                        - std::time::Duration::from_secs(3600 * 24), // Last 24 hours
                                )
                                .await?;

                            // Extract unique trace IDs
                            let mut trace_ids = HashSet::new();
                            for span in spans.iter().take(limit * 10) {
                                let trace_id_str = span.trace_id.as_str();
                                if let Ok(trace_id) = u128::from_str_radix(trace_id_str, 16) {
                                    trace_ids.insert(trace_id);
                                    if trace_ids.len() >= limit {
                                        break;
                                    }
                                }
                            }

                            return Ok(trace_ids.into_iter().collect());
                        }
                    }
                }
                Ok(vec![])
            },

            Field::Status => {
                // Status comparison
                if *op == Operator::Eq {
                    match value {
                        Value::Status(StatusValue::Error) => {
                            // Get error traces
                            let error_traces = self.get_error_traces(storage, limit).await?;
                            Ok(error_traces)
                        },
                        Value::String(s) if s == "error" => {
                            // Get error traces
                            let error_traces = self.get_error_traces(storage, limit).await?;
                            Ok(error_traces)
                        },
                        _ => Ok(vec![]),
                    }
                } else {
                    Ok(vec![])
                }
            },

            Field::Duration => {
                // Duration comparison
                if let Value::Duration(duration_val) = value {
                    let threshold_micros = duration_val.to_micros();

                    // We need to scan spans and filter by duration
                    // This is inefficient without proper indexing
                    let spans = self.get_recent_spans(storage, limit * 10).await?;

                    let mut trace_ids = HashSet::new();
                    for span in spans {
                        let duration_micros = span.duration.as_micros() as u64;

                        let matches = match op {
                            Operator::Gt => duration_micros > threshold_micros,
                            Operator::Gte => duration_micros >= threshold_micros,
                            Operator::Lt => duration_micros < threshold_micros,
                            Operator::Lte => duration_micros <= threshold_micros,
                            Operator::Eq => duration_micros == threshold_micros,
                            Operator::NotEq => duration_micros != threshold_micros,
                            _ => false,
                        };

                        if matches {
                            let trace_id_str = span.trace_id.as_str();
                            if let Ok(trace_id) = u128::from_str_radix(trace_id_str, 16) {
                                trace_ids.insert(trace_id);
                                if trace_ids.len() >= limit {
                                    break;
                                }
                            }
                        }
                    }

                    Ok(trace_ids.into_iter().collect())
                } else {
                    Ok(vec![])
                }
            },

            Field::Name
            | Field::TraceId
            | Field::SpanId
            | Field::ParentSpanId
            | Field::SpanKind
            | Field::Attribute(_) => {
                // For now, these require scanning all spans
                // In a production system, we'd have proper indexing for these
                Ok(vec![])
            },
        }
    }

    /// Get error traces from storage
    async fn get_error_traces(
        &self,
        storage: &dyn StorageBackend,
        limit: usize,
    ) -> Result<Vec<u128>> {
        // Get recent spans and filter for errors
        let spans = self.get_recent_spans(storage, limit * 10).await?;

        let mut trace_ids = HashSet::new();
        for span in spans {
            if matches!(span.status, SpanStatus::Error(_)) {
                let trace_id_str = span.trace_id.as_str();
                if let Ok(trace_id) = u128::from_str_radix(trace_id_str, 16) {
                    trace_ids.insert(trace_id);
                    if trace_ids.len() >= limit {
                        break;
                    }
                }
            }
        }

        Ok(trace_ids.into_iter().collect())
    }

    /// Get recent spans from storage
    async fn get_recent_spans(
        &self,
        storage: &dyn StorageBackend,
        _limit: usize,
    ) -> Result<Vec<crate::core::Span>> {
        // This is a simplified implementation
        // In production, we'd have a more efficient way to get recent spans

        // Try to get spans from a known service (this is a hack)
        // In reality, we'd need a get_recent_spans method on StorageBackend
        let _stats = storage.get_stats().await?;

        // For now, return empty - this needs proper implementation
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;

    #[tokio::test]
    async fn test_executor_basic() {
        let storage = InMemoryStorage::new(1000);
        let storage: Arc<tokio::sync::RwLock<dyn StorageBackend>> =
            Arc::new(tokio::sync::RwLock::new(storage));

        let executor = QueryExecutor::new(storage);

        // Test executing a simple query
        let query = Query {
            filter: QueryFilter::Comparison {
                field: Field::Service,
                op: Operator::Eq,
                value: Value::String("api".to_string()),
            },
        };

        let result = executor.execute(query, Some(10)).await.unwrap();
        assert_eq!(result.trace_ids.len(), 0); // No data yet
    }
}

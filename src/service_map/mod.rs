//! Service dependency mapping and analysis.
//!
//! This module automatically analyzes traces to build service dependency graphs,
//! showing how services call each other, with performance and error metrics.

use crate::core::{Result, ServiceName, Span, TraceId};
use crate::storage::StorageBackend;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Service dependency edge with metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEdge {
    /// Source service
    pub from: ServiceName,
    /// Target service
    pub to: ServiceName,
    /// Number of calls
    pub call_count: u64,
    /// Error count
    pub error_count: u64,
    /// Average latency in microseconds
    pub avg_latency_us: u64,
    /// P99 latency in microseconds
    pub p99_latency_us: u64,
    /// Operations between these services
    pub operations: HashSet<String>,
}

/// Service node in the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceNode {
    /// Service name
    pub name: ServiceName,
    /// Total request count
    pub request_count: u64,
    /// Error rate (0.0 - 1.0)
    pub error_rate: f64,
    /// Average latency
    pub avg_latency_us: u64,
    /// Is this a root service (no incoming edges)?
    pub is_root: bool,
    /// Is this a leaf service (no outgoing edges)?
    pub is_leaf: bool,
    /// Service tier (0 = root, higher = deeper)
    pub tier: u32,
}

/// Service dependency map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMap {
    /// All services in the system
    pub nodes: Vec<ServiceNode>,
    /// Dependencies between services
    pub edges: Vec<ServiceEdge>,
    /// Timestamp when this map was generated
    pub generated_at: std::time::SystemTime,
    /// Number of traces analyzed
    pub trace_count: u64,
    /// Time window of traces (in seconds)
    pub time_window_seconds: u64,
}

/// Service map builder that analyzes traces.
pub struct ServiceMapBuilder<'a> {
    storage: &'a dyn StorageBackend,
    /// Service -> (request_count, error_count, total_latency)
    service_metrics: HashMap<ServiceName, (u64, u64, u64)>,
    /// (from, to) -> edge data
    edges: HashMap<(ServiceName, ServiceName), EdgeBuilder>,
    /// Track all services
    services: HashSet<ServiceName>,
}

/// Helper for building edges incrementally.
#[derive(Default)]
struct EdgeBuilder {
    call_count: u64,
    error_count: u64,
    latencies: Vec<u64>,
    operations: HashSet<String>,
}

impl<'a> ServiceMapBuilder<'a> {
    /// Create a new service map builder.
    pub fn new(storage: &'a dyn StorageBackend) -> Self {
        Self {
            storage,
            service_metrics: HashMap::new(),
            edges: HashMap::new(),
            services: HashSet::new(),
        }
    }
    
    /// Build service map from recent traces.
    pub async fn build_from_recent_traces(
        &mut self,
        limit: usize,
        time_window_seconds: u64,
    ) -> Result<ServiceMap> {
        // Get recent traces
        let traces = self.storage.list_traces(None, None, None, limit).await?;
        
        if traces.is_empty() {
            return Ok(ServiceMap {
                nodes: Vec::new(),
                edges: Vec::new(),
                generated_at: std::time::SystemTime::now(),
                trace_count: 0,
                time_window_seconds,
            });
        }
        
        // Analyze each trace
        for trace_info in &traces {
            self.analyze_trace(&trace_info.trace_id).await?;
        }
        
        // Build the final map
        Ok(self.build_map(traces.len() as u64, time_window_seconds))
    }
    
    /// Analyze a single trace to extract dependencies.
    async fn analyze_trace(&mut self, trace_id: &TraceId) -> Result<()> {
        let spans = self.storage.get_trace_spans(trace_id).await?;
        
        if spans.is_empty() {
            return Ok(());
        }
        
        // Build span lookup map
        let mut span_map: HashMap<String, &Span> = HashMap::new();
        for span in &spans {
            span_map.insert(span.span_id.as_str().to_string(), span);
            self.services.insert(span.service_name.clone());
        }
        
        // Process each span to find service calls
        for span in &spans {
            // Update service metrics
            let metrics = self.service_metrics
                .entry(span.service_name.clone())
                .or_insert((0, 0, 0));
            metrics.0 += 1; // request count
            if span.status.is_error() {
                metrics.1 += 1; // error count
            }
            metrics.2 += span.duration.as_micros() as u64; // total latency
            
            // Find parent span to detect service-to-service calls
            if let Some(parent_id) = &span.parent_span_id {
                if let Some(parent_span) = span_map.get(parent_id.as_str()) {
                    // Different service = service-to-service call
                    if parent_span.service_name != span.service_name {
                        self.record_edge(
                            parent_span.service_name.clone(),
                            span.service_name.clone(),
                            &span.operation_name,
                            span.duration.as_micros() as u64,
                            span.status.is_error(),
                        );
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Record a service-to-service call.
    fn record_edge(
        &mut self,
        from: ServiceName,
        to: ServiceName,
        operation: &str,
        latency_us: u64,
        is_error: bool,
    ) {
        let edge = self.edges
            .entry((from, to))
            .or_insert_with(EdgeBuilder::default);
        
        edge.call_count += 1;
        if is_error {
            edge.error_count += 1;
        }
        edge.latencies.push(latency_us);
        edge.operations.insert(operation.to_string());
    }
    
    /// Build the final service map.
    fn build_map(&self, trace_count: u64, time_window_seconds: u64) -> ServiceMap {
        // Identify root and leaf services
        let mut has_incoming: HashSet<ServiceName> = HashSet::new();
        let mut has_outgoing: HashSet<ServiceName> = HashSet::new();
        
        for (from, to) in self.edges.keys() {
            has_outgoing.insert(from.clone());
            has_incoming.insert(to.clone());
        }
        
        // Build nodes
        let mut nodes = Vec::new();
        for service in &self.services {
            let metrics = self.service_metrics.get(service);
            let (request_count, error_count, total_latency) = metrics
                .cloned()
                .unwrap_or((0, 0, 0));
            
            let error_rate = if request_count > 0 {
                error_count as f64 / request_count as f64
            } else {
                0.0
            };
            
            let avg_latency_us = if request_count > 0 {
                total_latency / request_count
            } else {
                0
            };
            
            let is_root = !has_incoming.contains(service);
            let is_leaf = !has_outgoing.contains(service);
            
            // Calculate tier (distance from root)
            let tier = self.calculate_tier(service, &has_incoming);
            
            nodes.push(ServiceNode {
                name: service.clone(),
                request_count,
                error_rate,
                avg_latency_us,
                is_root,
                is_leaf,
                tier,
            });
        }
        
        // Build edges
        let mut edges = Vec::new();
        for ((from, to), builder) in &self.edges {
            let avg_latency_us = if !builder.latencies.is_empty() {
                builder.latencies.iter().sum::<u64>() / builder.latencies.len() as u64
            } else {
                0
            };
            
            let p99_latency_us = if !builder.latencies.is_empty() {
                let mut sorted = builder.latencies.clone();
                sorted.sort_unstable();
                let p99_idx = (sorted.len() as f64 * 0.99) as usize;
                sorted[p99_idx.min(sorted.len() - 1)]
            } else {
                0
            };
            
            edges.push(ServiceEdge {
                from: from.clone(),
                to: to.clone(),
                call_count: builder.call_count,
                error_count: builder.error_count,
                avg_latency_us,
                p99_latency_us,
                operations: builder.operations.clone(),
            });
        }
        
        // Sort nodes by tier, then by name
        nodes.sort_by(|a, b| {
            a.tier.cmp(&b.tier)
                .then(a.name.as_str().cmp(b.name.as_str()))
        });
        
        ServiceMap {
            nodes,
            edges,
            generated_at: std::time::SystemTime::now(),
            trace_count,
            time_window_seconds,
        }
    }
    
    /// Calculate service tier (depth from root).
    fn calculate_tier(&self, service: &ServiceName, has_incoming: &HashSet<ServiceName>) -> u32 {
        if !has_incoming.contains(service) {
            return 0; // Root service
        }
        
        // Simple BFS to find distance from any root
        // In production, we'd cache this calculation
        let mut visited = HashSet::new();
        let mut current_tier = 0;
        let mut current_level = HashSet::new();
        
        // Start with all root services
        for s in &self.services {
            if !has_incoming.contains(s) {
                current_level.insert(s.clone());
            }
        }
        
        while !current_level.is_empty() && !current_level.contains(service) {
            let mut next_level = HashSet::new();
            
            for current in &current_level {
                visited.insert(current.clone());
                
                // Find all services this one calls
                for ((from, to), _) in &self.edges {
                    if from == current && !visited.contains(to) {
                        next_level.insert(to.clone());
                    }
                }
            }
            
            current_level = next_level;
            current_tier += 1;
        }
        
        current_tier
    }
}

/// HTTP API endpoints for service map.
pub mod api {
    use super::*;
    use axum::{extract::State, response::IntoResponse, Json};
    
    /// GET /api/service-map - Get current service dependency map
    pub async fn get_service_map(
        State(storage): State<Arc<RwLock<dyn StorageBackend>>>,
    ) -> impl IntoResponse {
        let storage_guard = storage.read().await;
        let mut builder = ServiceMapBuilder::new(&*storage_guard);
        
        match builder.build_from_recent_traces(1000, 3600).await {
            Ok(map) => Json(map).into_response(),
            Err(e) => {
                tracing::error!("Failed to build service map: {}", e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": format!("Failed to build service map: {}", e)
                    })),
                ).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::{SpanBuilder, SpanId}, storage::InMemoryStorage};
    
    #[tokio::test]
    async fn test_service_map_builder() {
        let storage = InMemoryStorage::new(10000);
        
        // Create test spans with service dependencies
        // frontend -> backend -> database
        let trace_id = TraceId::new("test-trace-123".to_string()).unwrap();
        
        let frontend_span = SpanBuilder::default()
            .trace_id(trace_id.clone())
            .span_id(SpanId::new("span-1".to_string()).unwrap())
            .service_name(ServiceName::new("frontend".to_string()).unwrap())
            .operation_name("GET /api".to_string())
            .build()
            .unwrap();
        
        let backend_span = SpanBuilder::default()
            .trace_id(trace_id.clone())
            .span_id(SpanId::new("span-2".to_string()).unwrap())
            .parent_span_id(SpanId::new("span-1".to_string()).unwrap())
            .service_name(ServiceName::new("backend".to_string()).unwrap())
            .operation_name("process_request".to_string())
            .build()
            .unwrap();
        
        let db_span = SpanBuilder::default()
            .trace_id(trace_id.clone())
            .span_id(SpanId::new("span-3".to_string()).unwrap())
            .parent_span_id(SpanId::new("span-2".to_string()).unwrap())
            .service_name(ServiceName::new("database".to_string()).unwrap())
            .operation_name("query".to_string())
            .build()
            .unwrap();
        
        // Store spans
        storage.store_span(frontend_span).await.unwrap();
        storage.store_span(backend_span).await.unwrap();
        storage.store_span(db_span).await.unwrap();
        
        // Build service map
        let mut builder = ServiceMapBuilder::new(&storage);
        let map = builder.build_from_recent_traces(10, 3600).await.unwrap();
        
        // Verify nodes
        assert_eq!(map.nodes.len(), 3);
        assert!(map.nodes.iter().any(|n| n.name.as_str() == "frontend" && n.is_root));
        assert!(map.nodes.iter().any(|n| n.name.as_str() == "database" && n.is_leaf));
        
        // Verify edges
        assert_eq!(map.edges.len(), 2);
        assert!(map.edges.iter().any(|e| 
            e.from.as_str() == "frontend" && e.to.as_str() == "backend"
        ));
        assert!(map.edges.iter().any(|e| 
            e.from.as_str() == "backend" && e.to.as_str() == "database"
        ));
    }
}
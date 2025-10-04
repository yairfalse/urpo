//! High-performance log storage with search capabilities

use crate::core::{Result, TraceId};
use crate::logs::types::{LogRecord, LogSeverity};
use crate::metrics::string_pool::StringPool;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Log storage configuration
pub struct LogStorageConfig {
    /// Maximum number of logs to keep in memory
    pub max_logs: usize,
    /// Maximum age of logs before eviction
    pub max_age: Duration,
    /// Enable full-text indexing
    pub enable_search: bool,
}

impl Default for LogStorageConfig {
    fn default() -> Self {
        Self {
            max_logs: 100_000,
            max_age: Duration::from_secs(3600), // 1 hour
            enable_search: true,
        }
    }
}

/// High-performance log storage engine
pub struct LogStorage {
    /// Configuration
    config: LogStorageConfig,
    /// Ring buffer for log records
    logs: Arc<RwLock<VecDeque<LogRecord>>>,
    /// String interning pool
    string_pool: Arc<StringPool>,
    /// Inverted index for search (word -> log indices)
    search_index: Arc<DashMap<String, HashSet<usize>>>,
    /// Trace ID index (trace_id -> log indices)
    trace_index: Arc<DashMap<TraceId, Vec<usize>>>,
    /// Service index (service_id -> log indices)
    service_index: Arc<DashMap<u16, Vec<usize>>>,
    /// Current log counter for indexing
    log_counter: Arc<RwLock<usize>>,
}

impl LogStorage {
    /// Create new log storage
    pub fn new(config: LogStorageConfig) -> Self {
        Self {
            logs: Arc::new(RwLock::new(VecDeque::with_capacity(config.max_logs))),
            string_pool: Arc::new(StringPool::new()),
            search_index: Arc::new(DashMap::new()),
            trace_index: Arc::new(DashMap::new()),
            service_index: Arc::new(DashMap::new()),
            log_counter: Arc::new(RwLock::new(0)),
            config,
        }
    }

    /// Store a log record
    pub fn store_log(&self, log: LogRecord) -> Result<()> {
        let mut logs = self.logs.write();
        let mut counter = self.log_counter.write();

        // Check capacity
        if logs.len() >= self.config.max_logs {
            // Evict oldest log
            if let Some(old_log) = logs.pop_front() {
                self.remove_from_indices(&old_log, *counter - logs.len());
            }
        }

        // Index the log
        let log_index = *counter;
        *counter += 1;

        // Add to trace index if present
        if let Some(ref trace_id) = log.trace_id {
            self.trace_index
                .entry(trace_id.clone())
                .or_default()
                .push(log_index);
        }

        // Add to service index
        self.service_index
            .entry(log.service_id)
            .or_default()
            .push(log_index);

        // Add to search index if enabled
        if self.config.enable_search {
            self.index_log_text(&log.body, log_index);
        }

        logs.push_back(log);
        Ok(())
    }

    /// Search logs by text query
    pub fn search_logs(&self, query: &str, limit: usize) -> Result<Vec<LogRecord>> {
        if !self.config.enable_search || query.is_empty() {
            return Ok(Vec::new());
        }

        // Tokenize query
        let query_tokens: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if query_tokens.is_empty() {
            return Ok(Vec::new());
        }

        // Find matching log indices
        let mut matching_indices = HashSet::new();
        for token in &query_tokens {
            if let Some(indices) = self.search_index.get(token) {
                for &idx in indices.iter() {
                    matching_indices.insert(idx);
                }
            }
        }

        // Retrieve matching logs
        let logs = self.logs.read();
        let counter = *self.log_counter.read();
        let base_index = counter.saturating_sub(logs.len());

        let mut results = Vec::new();
        for &idx in &matching_indices {
            if idx >= base_index {
                if let Some(log) = logs.get(idx - base_index) {
                    results.push(log.clone());
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }

        // Sort by timestamp (newest first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(results)
    }

    /// Get logs by trace ID
    pub fn get_logs_by_trace(&self, trace_id: &TraceId) -> Result<Vec<LogRecord>> {
        if let Some(indices) = self.trace_index.get(trace_id) {
            let logs = self.logs.read();
            let counter = *self.log_counter.read();
            let base_index = counter.saturating_sub(logs.len());

            let results = indices
                .iter()
                .filter_map(|&idx| {
                    if idx >= base_index {
                        logs.get(idx - base_index).cloned()
                    } else {
                        None
                    }
                })
                .collect();
            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get logs by service
    pub fn get_logs_by_service(&self, service_id: u16, limit: usize) -> Vec<LogRecord> {
        if let Some(indices) = self.service_index.get(&service_id) {
            let logs = self.logs.read();
            let counter = *self.log_counter.read();
            let base_index = counter.saturating_sub(logs.len());

            indices
                .iter()
                .rev() // Newest first
                .take(limit)
                .filter_map(|&idx| {
                    if idx >= base_index {
                        logs.get(idx - base_index).cloned()
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Filter logs by severity
    pub fn filter_by_severity(&self, min_severity: LogSeverity, limit: usize) -> Vec<LogRecord> {
        let logs = self.logs.read();
        logs.iter()
            .rev() // Newest first
            .filter(|log| log.severity >= min_severity)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get recent logs with optional severity filter
    pub fn get_recent_logs(&self, limit: usize, min_severity: Option<LogSeverity>) -> Result<Vec<LogRecord>> {
        let logs = self.logs.read();
        let filtered: Vec<LogRecord> = if let Some(severity) = min_severity {
            logs.iter()
                .rev() // Newest first
                .filter(|log| log.severity >= severity)
                .take(limit)
                .cloned()
                .collect()
        } else {
            logs.iter().rev().take(limit).cloned().collect()
        };
        Ok(filtered)
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> LogStorageStats {
        let logs = self.logs.read();
        let total_logs = logs.len();

        let severity_counts = logs.iter().fold(HashMap::new(), |mut acc, log| {
            *acc.entry(log.severity).or_insert(0) += 1;
            acc
        });

        let memory_usage: usize = logs.iter().map(|log| log.memory_size()).sum();

        LogStorageStats {
            total_logs,
            severity_counts,
            memory_usage_bytes: memory_usage,
            indexed_terms: self.search_index.len(),
            traced_logs: self.trace_index.len(),
        }
    }

    /// Index log text for search
    fn index_log_text(&self, text: &str, log_index: usize) {
        // Simple tokenization - can be improved with better NLP
        let tokens: Vec<String> = text
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 2) // Skip very short words
            .map(|s| s.to_string())
            .collect();

        for token in tokens {
            self.search_index
                .entry(token)
                .or_default()
                .insert(log_index);
        }
    }

    /// Remove log from indices when evicted
    fn remove_from_indices(&self, log: &LogRecord, log_index: usize) {
        // Remove from trace index
        if let Some(ref trace_id) = log.trace_id {
            if let Some(mut indices) = self.trace_index.get_mut(trace_id) {
                indices.retain(|&idx| idx != log_index);
            }
        }

        // Remove from service index
        if let Some(mut indices) = self.service_index.get_mut(&log.service_id) {
            indices.retain(|&idx| idx != log_index);
        }

        // Note: We don't clean search index as it's too expensive
        // It will naturally age out as indices become invalid
    }

    /// Clear all logs
    pub fn clear(&self) {
        self.logs.write().clear();
        self.search_index.clear();
        self.trace_index.clear();
        self.service_index.clear();
        *self.log_counter.write() = 0;
    }
}

/// Log storage statistics
#[derive(Debug)]
pub struct LogStorageStats {
    pub total_logs: usize,
    pub severity_counts: HashMap<LogSeverity, usize>,
    pub memory_usage_bytes: usize,
    pub indexed_terms: usize,
    pub traced_logs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn create_test_storage() -> LogStorage {
        LogStorage::new(LogStorageConfig::default())
    }

    fn create_test_log(body: &str, severity: LogSeverity) -> LogRecord {
        LogRecord::new(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            1, // service_id
            severity,
            body.to_string(),
        )
    }

    #[test]
    fn test_store_and_retrieve_log() {
        let storage = create_test_storage();
        let log = create_test_log("Test log message", LogSeverity::Info);

        storage.store_log(log).unwrap();

        let recent = storage.get_recent_logs(10, None).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].body, "Test log message");
    }

    #[test]
    fn test_search_logs() {
        let storage = create_test_storage();

        storage
            .store_log(create_test_log("Error in database connection", LogSeverity::Error))
            .unwrap();
        storage
            .store_log(create_test_log("Successfully connected to database", LogSeverity::Info))
            .unwrap();
        storage
            .store_log(create_test_log("Network timeout occurred", LogSeverity::Warn))
            .unwrap();

        let results = storage.search_logs("database", 10).unwrap();
        assert_eq!(results.len(), 2);

        let results = storage.search_logs("error", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, LogSeverity::Error);
    }

    #[test]
    fn test_filter_by_severity() {
        let storage = create_test_storage();

        storage
            .store_log(create_test_log("Debug message", LogSeverity::Debug))
            .unwrap();
        storage
            .store_log(create_test_log("Info message", LogSeverity::Info))
            .unwrap();
        storage
            .store_log(create_test_log("Warning message", LogSeverity::Warn))
            .unwrap();
        storage
            .store_log(create_test_log("Error message", LogSeverity::Error))
            .unwrap();

        let results = storage.filter_by_severity(LogSeverity::Warn, 10);
        assert_eq!(results.len(), 2); // Warn and Error

        let results = storage.filter_by_severity(LogSeverity::Error, 10);
        assert_eq!(results.len(), 1); // Only Error
    }

    #[test]
    fn test_trace_correlation() {
        let storage = create_test_storage();
        let trace_id = TraceId::new("abc123".to_string()).unwrap();

        let log1 =
            create_test_log("Request started", LogSeverity::Info).with_trace_id(trace_id.clone());
        let log2 =
            create_test_log("Processing data", LogSeverity::Debug).with_trace_id(trace_id.clone());
        let log3 = create_test_log("Unrelated log", LogSeverity::Info);

        storage.store_log(log1).unwrap();
        storage.store_log(log2).unwrap();
        storage.store_log(log3).unwrap();

        let trace_logs = storage.get_logs_by_trace(&trace_id).unwrap();
        assert_eq!(trace_logs.len(), 2);
    }

    #[test]
    fn test_service_filtering() {
        let storage = create_test_storage();

        let mut log1 = create_test_log("Service 1 log", LogSeverity::Info);
        log1.service_id = 1;

        let mut log2 = create_test_log("Service 2 log", LogSeverity::Info);
        log2.service_id = 2;

        let mut log3 = create_test_log("Another Service 1 log", LogSeverity::Info);
        log3.service_id = 1;

        storage.store_log(log1).unwrap();
        storage.store_log(log2).unwrap();
        storage.store_log(log3).unwrap();

        let service1_logs = storage.get_logs_by_service(1, 10);
        assert_eq!(service1_logs.len(), 2);

        let service2_logs = storage.get_logs_by_service(2, 10);
        assert_eq!(service2_logs.len(), 1);
    }

    #[test]
    fn test_capacity_eviction() {
        let config = LogStorageConfig {
            max_logs: 3,
            max_age: Duration::from_secs(3600),
            enable_search: false,
        };
        let storage = LogStorage::new(config);

        for i in 0..5 {
            let log = create_test_log(&format!("Log {}", i), LogSeverity::Info);
            storage.store_log(log).unwrap();
        }

        let recent = storage.get_recent_logs(10, None).unwrap();
        assert_eq!(recent.len(), 3); // Only 3 most recent
        assert_eq!(recent[0].body, "Log 4"); // Newest first
        assert_eq!(recent[2].body, "Log 2"); // Oldest retained
    }

    #[test]
    fn test_storage_stats() {
        let storage = create_test_storage();

        storage
            .store_log(create_test_log("Error", LogSeverity::Error))
            .unwrap();
        storage
            .store_log(create_test_log("Warning", LogSeverity::Warn))
            .unwrap();
        storage
            .store_log(create_test_log("Info 1", LogSeverity::Info))
            .unwrap();
        storage
            .store_log(create_test_log("Info 2", LogSeverity::Info))
            .unwrap();

        let stats = storage.get_stats();
        assert_eq!(stats.total_logs, 4);
        assert_eq!(*stats.severity_counts.get(&LogSeverity::Error).unwrap(), 1);
        assert_eq!(*stats.severity_counts.get(&LogSeverity::Warn).unwrap(), 1);
        assert_eq!(*stats.severity_counts.get(&LogSeverity::Info).unwrap(), 2);
        assert!(stats.memory_usage_bytes > 0);
    }

    #[test]
    fn test_clear_storage() {
        let storage = create_test_storage();

        storage
            .store_log(create_test_log("Test", LogSeverity::Info))
            .unwrap();
        assert_eq!(storage.get_recent_logs(10, None).unwrap().len(), 1);

        storage.clear();
        assert_eq!(storage.get_recent_logs(10, None).unwrap().len(), 0);

        let stats = storage.get_stats();
        assert_eq!(stats.total_logs, 0);
    }
}

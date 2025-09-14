# Urpo Metrics and Logs Design Document
## Ultra-Fast OpenTelemetry Metrics and Logs Implementation

**Version:** 1.0
**Date:** 2024-09-14
**Author:** Technical Product Manager

---

## Executive Summary

This document defines the architecture and implementation strategy for adding OpenTelemetry Metrics and Logs signals to Urpo, while maintaining its core philosophy of ultra-fast performance, minimal memory usage, and terminal-native experience.

**Key Design Principles:**
- **Performance First**: <10μs per operation, <100MB total memory
- **Zero-Configuration**: Works out-of-the-box with sensible defaults
- **Terminal-Native**: Keyboard-driven UI with vim-like bindings
- **Unified Experience**: Seamless correlation between traces, metrics, and logs
- **Modular Architecture**: Extensible plugin system for future growth

---

## 1. METRICS DESIGN

### 1.1 Data Model

```rust
/// OpenTelemetry metric types with ultra-fast processing
#[derive(Debug, Clone)]
pub enum MetricType {
    /// Monotonically increasing counter (requests, errors)
    Counter {
        value: f64,
        exemplars: Option<Vec<Exemplar>>,
    },
    /// Up/down counter (active connections, queue size)
    UpDownCounter {
        value: f64,
        exemplars: Option<Vec<Exemplar>>,
    },
    /// Point-in-time measurement (CPU usage, memory)
    Gauge {
        value: f64,
        exemplars: Option<Vec<Exemplar>>,
    },
    /// Latency/size distributions with pre-computed percentiles
    Histogram {
        buckets: CompactHistogram,
        sum: f64,
        count: u64,
        exemplars: Option<Vec<Exemplar>>,
    },
}

/// Ultra-compact histogram using bit-packed buckets
#[derive(Debug, Clone)]
pub struct CompactHistogram {
    /// Bit-packed bucket counts (4-bit counters for hot path)
    hot_buckets: u64,
    /// Full precision for cold/overflow buckets
    cold_buckets: Vec<u32>,
    /// Bucket boundaries (shared across metrics)
    boundaries_ref: u16, // Index into global boundaries table
}

/// Metric data point optimized for cache efficiency
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct MetricPoint {
    /// Timestamp (8 bytes)
    timestamp: u64,
    /// Service name (2 bytes index)
    service_idx: u16,
    /// Metric name (2 bytes index)
    metric_idx: u16,
    /// Value (8 bytes)
    value: f64,
    /// Attributes hash (4 bytes)
    attr_hash: u32,
    /// Metric type + flags (1 byte)
    type_flags: u8,
    /// Padding for alignment
    _padding: u8,
}

/// String interning for zero-allocation metric processing
#[derive(Debug)]
pub struct MetricInternPool {
    service_names: StringPool<ServiceName>,
    metric_names: StringPool<String>,
    attribute_keys: StringPool<String>,
    attribute_values: StringPool<String>,
}
```

### 1.2 Storage Strategy

**Multi-Tier Storage Architecture:**

```rust
/// BLAZING FAST: Three-tier storage for optimal performance
pub struct MetricStorage {
    /// HOT PATH: Ring buffer for last 5 minutes (in-memory)
    hot_ring: HotMetricsRing,

    /// WARM PATH: Compressed time series for last 6 hours
    warm_store: CompressedTimeSeries,

    /// COLD PATH: Archived aggregates for long-term retention
    cold_archive: DiskArchive,

    /// BLAZING FAST: Pre-aggregated service health metrics
    service_health: DashMap<ServiceName, ServiceHealthMetrics>,

    /// String interning pool
    intern_pool: MetricInternPool,
}

/// Ring buffer optimized for metrics ingestion
pub struct HotMetricsRing {
    /// Circular buffer of metric points (cache-aligned)
    points: Box<[MetricPoint; HOT_RING_SIZE]>,
    /// Write position (atomic)
    write_pos: AtomicUsize,
    /// Read position for consumers
    read_pos: AtomicUsize,
    /// Generation counter to detect overwrites
    generation: AtomicU64,
}

/// Delta-compressed time series for memory efficiency
pub struct CompressedTimeSeries {
    /// Service -> metric name -> compressed series
    series: DashMap<ServiceName, DashMap<String, DeltaCompressedSeries>>,
    /// Retention policy
    retention: Duration,
}
```

**Storage Performance Targets:**
- **Hot Path Ingestion**: <5μs per metric point
- **Query Latency**: <1ms for dashboard queries
- **Memory Usage**: 50MB for 1M metric points
- **Compression Ratio**: 10:1 for time series data

### 1.3 Aggregation Approach

**Pre-Aggregation for Dashboard Speed:**

```rust
/// Real-time service health aggregation
pub struct ServiceHealthAggregator {
    /// Per-service sliding windows (1m, 5m, 15m, 1h)
    windows: DashMap<ServiceName, ServiceTimeWindows>,

    /// Atomic counters for instant updates
    request_counters: DashMap<ServiceName, AtomicU64>,
    error_counters: DashMap<ServiceName, AtomicU64>,

    /// Histogram collectors for latency percentiles
    latency_histograms: DashMap<ServiceName, ConcurrentHistogram>,
}

#[inline(always)]
pub fn update_service_health(
    aggregator: &ServiceHealthAggregator,
    service: &ServiceName,
    metric_type: &str,
    value: f64,
) {
    // ZERO ALLOCATION: Direct atomic updates
    match metric_type {
        "http_requests_total" => {
            aggregator.request_counters
                .entry(service.clone())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(value as u64, Ordering::Relaxed);
        }
        "http_request_duration_seconds" => {
            // BLAZING FAST: Lock-free histogram updates
            aggregator.latency_histograms
                .entry(service.clone())
                .or_insert_with(ConcurrentHistogram::new)
                .record(value);
        }
        _ => {} // Custom metrics stored separately
    }
}
```

### 1.4 Query Patterns

**Optimized for Dashboard Queries:**

```rust
/// Query interface optimized for terminal dashboard
#[async_trait]
pub trait MetricQueryEngine {
    /// Get current service health metrics (RPS, error rate, latency)
    async fn get_service_health(&self, service: &ServiceName) -> Result<ServiceHealthMetrics>;

    /// Get time series data for metric visualization
    async fn query_time_series(
        &self,
        query: &MetricQuery,
        time_range: TimeRange,
    ) -> Result<TimeSeries>;

    /// Get metric exemplars (links to traces)
    async fn get_exemplars(
        &self,
        query: &MetricQuery,
        limit: usize,
    ) -> Result<Vec<Exemplar>>;
}

/// Specialized queries for Urpo's dashboard
pub struct MetricQuery {
    pub service: Option<ServiceName>,
    pub metric_name: String,
    pub attributes: HashMap<String, String>,
    pub aggregation: AggregationType,
}

pub enum AggregationType {
    Sum,
    Rate,           // For counters
    Average,        // For gauges
    Percentile(f64), // For histograms
}
```

### 1.5 Memory Management

**Bounded Memory with Intelligent Eviction:**

```rust
/// Memory-bounded metric storage with LRU eviction
pub struct BoundedMetricStorage {
    /// Current memory usage
    memory_usage: AtomicUsize,

    /// Memory limit (configurable, default 50MB)
    memory_limit: usize,

    /// LRU tracker for service metrics
    service_lru: Arc<RwLock<LruCache<ServiceName, Instant>>>,

    /// Priority queue for eviction
    eviction_queue: SegQueue<(ServiceName, u64)>, // (service, priority)
}

impl BoundedMetricStorage {
    #[inline]
    pub fn maybe_evict(&self) -> Result<()> {
        let current = self.memory_usage.load(Ordering::Relaxed);
        if current > self.memory_limit {
            // BLAZING FAST: Background eviction to avoid blocking
            self.trigger_background_eviction();
        }
        Ok(())
    }

    fn trigger_background_eviction(&self) {
        // Evict oldest, least-accessed service data
        // Keep last 15 minutes for all services
        // Beyond that, keep only active services
    }
}
```

## 2. LOGS DESIGN

### 2.1 Data Model

```rust
/// Ultra-compact log record for high-volume ingestion
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct LogRecord {
    /// Timestamp (8 bytes)
    timestamp: u64,

    /// Log level (1 byte: TRACE=0, DEBUG=1, INFO=2, WARN=3, ERROR=4, FATAL=5)
    level: u8,

    /// Service name index (2 bytes)
    service_idx: u16,

    /// Message content index (4 bytes - into content store)
    message_idx: u32,

    /// Trace/span correlation (16 bytes)
    trace_id: Option<u128>,
    span_id: Option<u64>,

    /// Attributes hash for grouping (4 bytes)
    attr_hash: u32,

    /// Flags (1 byte: structured=1, error=2, etc.)
    flags: u8,
}

/// Efficient log content storage with deduplication
pub struct LogContentStore {
    /// Deduplicated message content
    messages: StringPool<String>,

    /// Attribute key-value pairs
    attributes: AttributeStore,

    /// Full-text search index (optional)
    search_index: Option<TantivyIndex>,
}

/// Streaming log buffer for zero-allocation processing
pub struct LogStreamBuffer {
    /// Ring buffer for incoming logs
    buffer: RingBuffer<LogRecord, LOG_BUFFER_SIZE>,

    /// Write position
    write_pos: AtomicUsize,

    /// Consumer positions
    consumers: Vec<AtomicUsize>,
}
```

### 2.2 Storage Strategy

**Write-Optimized with Fast Search:**

```rust
/// Three-tier log storage matching metric storage pattern
pub struct LogStorage {
    /// HOT: Recent logs in memory (last 5 minutes)
    hot_logs: LogRingBuffer,

    /// WARM: Compressed logs on disk (last 24 hours)
    warm_logs: CompressedLogFiles,

    /// COLD: Archived logs (configurable retention)
    cold_archive: ArchiveStorage,

    /// BLAZING FAST: Error log index for quick error detection
    error_index: DashMap<ServiceName, ErrorLogIndex>,

    /// Content deduplication
    content_store: LogContentStore,
}

/// Optimized for log write performance
pub struct LogRingBuffer {
    /// Cache-aligned ring buffer
    records: Box<[LogRecord; HOT_LOG_SIZE]>,

    /// Atomic write position
    write_pos: AtomicUsize,

    /// Service-specific cursors for filtering
    service_cursors: DashMap<ServiceName, AtomicUsize>,
}

/// Error-specific index for fast error correlation
pub struct ErrorLogIndex {
    /// Recent error timestamps
    error_times: RingBuffer<u64, 1000>,

    /// Error message patterns (for grouping)
    error_patterns: LruCache<u32, ErrorPattern>,

    /// Links to related traces
    trace_correlations: Vec<(u128, u64)>, // (trace_id, span_id)
}
```

### 2.3 Search Capabilities

**Fast Log Search Without Full-Text Overhead:**

```rust
/// Hybrid search: structured for speed, full-text for completeness
pub struct LogSearchEngine {
    /// Primary: Fast structured search on indexed fields
    structured_index: StructuredLogIndex,

    /// Secondary: Full-text search for complex queries (optional)
    fulltext_index: Option<TantivySearchIndex>,

    /// Service-specific filters
    service_filters: DashMap<ServiceName, ServiceLogFilter>,
}

/// Lightning-fast structured search
pub struct StructuredLogIndex {
    /// Time-based index (primary key)
    time_index: BTreeMap<u64, Vec<LogRecordRef>>,

    /// Level-based index
    level_index: [Vec<LogRecordRef>; 6], // One per log level

    /// Service-based index
    service_index: DashMap<ServiceName, Vec<LogRecordRef>>,

    /// Error pattern index
    error_index: DashMap<u32, Vec<LogRecordRef>>, // hash -> records
}

/// Search query optimized for terminal interaction
pub struct LogSearchQuery {
    /// Time range filter
    pub time_range: Option<TimeRange>,

    /// Service filter
    pub services: Vec<ServiceName>,

    /// Log level filter
    pub min_level: LogLevel,

    /// Message pattern (regex or simple string)
    pub message_pattern: Option<String>,

    /// Trace correlation
    pub trace_id: Option<TraceId>,

    /// Limit results
    pub limit: usize,
}
```

### 2.4 Correlation with Traces

**Zero-Latency Trace-Log Correlation:**

```rust
/// Instant correlation between traces and logs
pub struct TraceLogCorrelator {
    /// Active traces with log links
    active_traces: DashMap<TraceId, TraceLogLinks>,

    /// Span -> log mappings
    span_logs: DashMap<SpanId, Vec<LogRecordRef>>,

    /// Service -> recent logs cache
    service_logs: DashMap<ServiceName, RecentLogCache>,
}

pub struct TraceLogLinks {
    /// Direct log references from spans
    span_logs: HashMap<SpanId, Vec<LogRecordRef>>,

    /// Service logs during trace timeframe
    contextual_logs: Vec<LogRecordRef>,

    /// Error logs for error correlation
    error_logs: Vec<LogRecordRef>,
}

#[inline(always)]
pub fn correlate_log_with_trace(
    correlator: &TraceLogCorrelator,
    log: &LogRecord,
) {
    if let (Some(trace_id), Some(span_id)) = (log.trace_id, log.span_id) {
        // ZERO ALLOCATION: Direct insertion into existing structures
        correlator.span_logs
            .entry(SpanId::from(span_id))
            .or_insert_with(Vec::new)
            .push(LogRecordRef::new(log));
    }
}
```

## 3. INTEGRATION DESIGN

### 3.1 Unified Signal Processing

**Single Pipeline for All Signals:**

```rust
/// Unified OTEL signal processor
pub struct UnifiedSignalProcessor {
    /// Shared string interning across all signals
    intern_pool: Arc<GlobalInternPool>,

    /// Signal-specific processors
    trace_processor: TraceProcessor,
    metric_processor: MetricProcessor,
    log_processor: LogProcessor,

    /// Cross-signal correlator
    correlator: SignalCorrelator,

    /// Shared storage backend
    storage: Arc<RwLock<dyn StorageBackend>>,
}

/// Cross-signal correlation engine
pub struct SignalCorrelator {
    /// Active service contexts
    service_contexts: DashMap<ServiceName, ServiceContext>,

    /// Trace -> metrics/logs links
    trace_links: DashMap<TraceId, CrossSignalLinks>,

    /// Time-based correlation windows
    correlation_windows: SlidingTimeWindows,
}

pub struct CrossSignalLinks {
    /// Metrics recorded during trace
    metrics: Vec<MetricRef>,

    /// Logs emitted during trace
    logs: Vec<LogRecordRef>,

    /// Service health at trace time
    health_snapshot: ServiceHealthSnapshot,
}
```

### 3.2 Unified Query Interface

**Single API for All Signal Types:**

```rust
/// Universal query interface for all OTEL signals
#[async_trait]
pub trait UnifiedQueryEngine {
    /// Service overview with all signal types
    async fn get_service_overview(
        &self,
        service: &ServiceName,
        time_range: TimeRange,
    ) -> Result<ServiceOverview>;

    /// Cross-signal search
    async fn search_signals(
        &self,
        query: &UniversalQuery,
    ) -> Result<SignalSearchResults>;

    /// Trace with correlated metrics and logs
    async fn get_trace_with_context(
        &self,
        trace_id: &TraceId,
    ) -> Result<TraceContext>;
}

/// Service overview combining all signal types
pub struct ServiceOverview {
    /// Basic service info
    pub service: ServiceInfo,

    /// Health metrics (RPS, errors, latency)
    pub health: ServiceHealthMetrics,

    /// Recent traces
    pub traces: Vec<TraceInfo>,

    /// Error logs
    pub error_logs: Vec<LogRecordSummary>,

    /// Key metrics
    pub key_metrics: Vec<MetricSummary>,

    /// Service map connections
    pub connections: ServiceConnections,
}

/// Universal search across all signals
pub struct UniversalQuery {
    pub services: Vec<ServiceName>,
    pub time_range: TimeRange,
    pub search_text: Option<String>,
    pub signal_types: Vec<SignalType>,
    pub limit: usize,
}
```

### 3.3 Resource Sharing

**Optimal Memory Usage Across Signals:**

```rust
/// Shared resources to minimize memory overhead
pub struct SharedSignalResources {
    /// Global string interning (services, operations, attributes)
    pub intern_pool: Arc<GlobalInternPool>,

    /// Shared time bucketing for all signals
    pub time_buckets: Arc<TimeBucketManager>,

    /// Service registry (shared metadata)
    pub service_registry: Arc<ServiceRegistry>,

    /// Resource attributes cache
    pub resource_cache: Arc<ResourceCache>,
}

/// Global string interning for all signal types
pub struct GlobalInternPool {
    /// Service names (shared across traces, metrics, logs)
    services: StringPool<ServiceName>,

    /// Operation/metric names
    names: StringPool<String>,

    /// Attribute keys (shared pool)
    attr_keys: StringPool<String>,

    /// Attribute values (with LRU eviction)
    attr_values: LruStringPool<String>,
}

impl GlobalInternPool {
    #[inline(always)]
    pub fn intern_service(&self, name: &str) -> u16 {
        self.services.intern(name)
    }

    #[inline(always)]
    pub fn intern_metric_name(&self, name: &str) -> u16 {
        self.names.intern(name)
    }
}
```

## 4. PERFORMANCE REQUIREMENTS

### 4.1 Target Latencies

| Operation | Target Latency | Memory Impact |
|-----------|---------------|---------------|
| **Metric Ingestion** | <5μs per point | <64 bytes |
| **Log Ingestion** | <3μs per record | <96 bytes |
| **Service Health Query** | <1ms | 0 bytes (cached) |
| **Time Series Query** | <10ms | <1KB |
| **Log Search (structured)** | <5ms | <4KB |
| **Cross-Signal Correlation** | <2ms | <512 bytes |
| **Dashboard Refresh** | <50ms total | <16KB |

### 4.2 Memory Budgets

**Total Memory Target: 100MB for comprehensive observability**

| Component | Memory Budget | Data Capacity |
|-----------|---------------|---------------|
| **Traces (existing)** | 40MB | 100K spans |
| **Metrics Hot Path** | 25MB | 1M metric points |
| **Logs Hot Path** | 20MB | 500K log records |
| **String Interning** | 8MB | 100K unique strings |
| **Indices & Caches** | 5MB | Various |
| **Buffer Overhead** | 2MB | Ring buffers, atomics |

### 4.3 Throughput Targets

| Signal Type | Ingestion Rate | Query Rate |
|-------------|----------------|------------|
| **Traces** | 10K spans/sec | 100 queries/sec |
| **Metrics** | 50K points/sec | 500 queries/sec |
| **Logs** | 100K records/sec | 200 queries/sec |

### 4.4 Cache Strategies

**Multi-Level Caching for Sub-Millisecond Responses:**

```rust
/// Hierarchical caching for maximum performance
pub struct SignalCacheHierarchy {
    /// L1: Hot service health metrics (atomic counters)
    l1_service_health: DashMap<ServiceName, AtomicServiceHealth>,

    /// L2: Pre-aggregated time series (last 5 minutes)
    l2_time_series: LruCache<QueryKey, CachedTimeSeries>,

    /// L3: Compressed historical data
    l3_historical: CompressedCache<QueryKey, CompressedData>,
}

/// Atomic service health for instant dashboard updates
#[repr(C, align(64))] // Cache line aligned
pub struct AtomicServiceHealth {
    request_count: AtomicU64,
    error_count: AtomicU64,
    total_latency_ns: AtomicU64,
    last_request_time: AtomicU64,
}
```

## 5. RELEASE MILESTONES

### 5.1 Phase 1: Metrics Foundation (4 weeks)
**Goal**: Basic metrics ingestion and service health dashboard

**Features:**
- [ ] Metric data model and storage
- [ ] OTLP/gRPC metrics receiver
- [ ] Service health aggregation
- [ ] Basic terminal metrics dashboard
- [ ] Memory-bounded storage with eviction

**Success Criteria:**
- Ingestion: >25K metric points/sec
- Dashboard refresh: <50ms
- Memory usage: <30MB for 500K points

### 5.2 Phase 2: Logs Foundation (4 weeks)
**Goal**: Log ingestion with structured search

**Features:**
- [ ] Log data model and storage
- [ ] OTLP/gRPC logs receiver
- [ ] Structured log search (time, level, service)
- [ ] Basic log viewer in terminal
- [ ] Trace-log correlation

**Success Criteria:**
- Ingestion: >50K log records/sec
- Search latency: <10ms for recent logs
- Memory usage: <50MB total (traces + metrics + logs)

### 5.3 Phase 3: Integration & Polish (3 weeks)
**Goal**: Unified observability experience

**Features:**
- [ ] Cross-signal correlation UI
- [ ] Service overview with all signal types
- [ ] Universal search interface
- [ ] Performance optimizations
- [ ] Configuration options

**Success Criteria:**
- Cross-signal queries: <5ms
- Dashboard shows traces, metrics, logs together
- Memory usage: <75MB total
- Zero-config startup works perfectly

### 5.4 Phase 4: Advanced Features (4 weeks)
**Goal**: Production-ready advanced capabilities

**Features:**
- [ ] Full-text log search (optional)
- [ ] Metric alerting/thresholds
- [ ] Export capabilities (Prometheus, Jaeger)
- [ ] Historical data archiving
- [ ] Plugin architecture foundation

**Success Criteria:**
- Full feature parity with specialized tools
- Performance maintains targets under load
- Memory usage: <100MB total
- Production deployment ready

## 6. COMPETITIVE DIFFERENTIATION

### 6.1 vs Jaeger + Prometheus + Loki Stack

| Aspect | Urpo | Traditional Stack |
|--------|------|-------------------|
| **Setup Complexity** | Single binary, zero config | Multiple services, complex config |
| **Memory Usage** | 100MB total | 500MB+ per service |
| **Query Speed** | <10ms cross-signal | 100ms+ per service |
| **Terminal Native** | Optimized for keyboard | Web-only interfaces |
| **Correlation** | Built-in, instant | Manual, slow |

### 6.2 vs Vector + ClickHouse

| Aspect | Urpo | Vector + ClickHouse |
|--------|------|---------------------|
| **Deployment** | Single process | Multiple components |
| **Resource Usage** | <100MB | 1GB+ |
| **Developer UX** | Terminal-first | SQL/web-based |
| **Real-time** | Instant updates | Batch processing |

### 6.3 vs Grafana + Tempo

| Aspect | Urpo | Grafana Stack |
|--------|------|---------------|
| **Learning Curve** | Zero (familiar terminal) | High (dashboards, queries) |
| **Performance** | Sub-millisecond queries | Second+ queries |
| **Correlation** | Automatic | Manual configuration |
| **Local Development** | Perfect | Overkill |

## 7. SUCCESS METRICS & KPIs

### 7.1 Performance KPIs

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Startup Time** | <200ms | Time to first signal processed |
| **Memory Efficiency** | <100MB | Resident memory for 1M+ data points |
| **Query Performance** | <10ms | 95th percentile query latency |
| **Throughput** | 100K+ signals/sec | Combined ingestion rate |
| **Correlation Speed** | <2ms | Trace-to-logs lookup time |

### 7.2 User Experience KPIs

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Time to Insight** | <30 seconds | Problem detection to root cause |
| **Zero Config Success** | >95% | Percentage working out-of-box |
| **Terminal Efficiency** | <10 keystrokes | Average actions to view trace+logs |
| **Cross-Signal Usage** | >50% | Users viewing correlated data |

### 7.3 Adoption KPIs

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Developer Retention** | >80% | Weekly active users after 30 days |
| **Performance vs Jaeger** | 10x faster | Comparative query benchmarks |
| **Resource vs Stack** | 5x less memory | vs Prometheus+Loki+Jaeger |
| **Setup Time** | <5 minutes | Fresh install to working dashboard |

---

## Conclusion

This design maintains Urpo's core philosophy while expanding to full observability coverage. The ultra-fast, memory-efficient approach differentiates Urpo in a crowded market by focusing on developer productivity and terminal-native experience.

**Key Success Factors:**
1. **Performance Never Compromised**: Every design decision optimized for speed
2. **Memory Bounded**: Predictable resource usage prevents system impact
3. **Developer Centric**: Terminal-first UX optimized for debugging workflows
4. **Zero Configuration**: Works perfectly out of the box
5. **Unified Experience**: Seamless correlation across all signal types

The phased rollout ensures each component meets performance targets before adding complexity, maintaining Urpo's reputation for blazing-fast observability tools.

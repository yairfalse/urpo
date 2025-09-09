# URPO: Rust-Only Project Guidelines for Claude
**THIS IS A RUST PROJECT - NO GO, NO OTHER LANGUAGES**
**PERFORMANCE TARGET: WORLD-CLASS SPEED**

## ⚠️ CRITICAL: Language Requirements
- **THIS IS A RUST PROJECT** - All code MUST be in Rust
- **NO GO CODE** - Never generate Go code (no `map[string]interface{}`, no `func`, no `:=`)
- **NO OTHER LANGUAGES** - Only Rust, TOML (for Cargo), and YAML (for CI)
- **STRONG TYPING ONLY** - Never use dynamic typing patterns

## 🚀 PERFORMANCE MANIFESTO

**Our Goal:** Build the fastest OTEL trace explorer in existence
- **Startup Time:** <200ms (industry-leading)
- **Span Processing:** <10μs per span (10,000+ spans/second)
- **Memory Usage:** <100MB for 1M spans (highly efficient)
- **UI Response:** <16ms frame time (60fps smooth)
- **Search:** <1ms across 100K traces

## Core Principles

### 1. **Zero-Allocation Hot Paths**
```rust
// ✅ BLAZING FAST: Use string slices and borrowing
pub fn parse_trace_id(input: &str) -> Result<TraceId> {
    // Validate WITHOUT allocating
    if input.len() != 32 || !input.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(UrpoError::InvalidTraceId);
    }
    // Only allocate on success
    Ok(TraceId::new_unchecked(input.to_string()))
}

// ❌ SLOW: Multiple allocations
pub fn parse_trace_id_slow(input: &str) -> Result<TraceId> {
    let cleaned = input.replace("-", "");  // Allocation 1
    let uppercased = cleaned.to_uppercase(); // Allocation 2
    TraceId::new(uppercased) // Allocation 3
}
```

### 2. **Inline Performance Critical Code**
```rust
// ✅ Force inlining for hot functions
#[inline(always)]
pub fn is_error_span(status: &SpanStatus) -> bool {
    matches!(status, SpanStatus::Error(_))
}

// ✅ Use const for compile-time computation
pub const MAX_SPANS_PER_TRACE: usize = 1000;
pub const SPAN_POOL_SIZE: usize = 10_000;

// ✅ Zero-cost abstractions with const generics
pub struct BoundedVec<T, const N: usize> {
    data: Vec<T>,
}

impl<T, const N: usize> BoundedVec<T, N> {
    #[inline]
    pub fn push(&mut self, item: T) -> Result<()> {
        if self.data.len() >= N {
            return Err(UrpoError::CapacityExceeded);
        }
        self.data.push(item);
        Ok(())
    }
}
```

### 3. **Lock-Free Data Structures**
```rust
// ✅ Use atomic operations for counters
use std::sync::atomic::{AtomicU64, Ordering};

pub struct MetricsAggregator {
    span_count: AtomicU64,
    error_count: AtomicU64,
    total_duration_ns: AtomicU64,
}

impl MetricsAggregator {
    #[inline]
    pub fn record_span(&self, duration_ns: u64, is_error: bool) {
        self.span_count.fetch_add(1, Ordering::Relaxed);
        self.total_duration_ns.fetch_add(duration_ns, Ordering::Relaxed);
        if is_error {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

// ✅ Use dashmap for concurrent hash maps
use dashmap::DashMap;

pub struct ServiceRegistry {
    services: DashMap<ServiceName, ServiceMetrics>,
}
```

### 4. **Memory Pool Allocation**
```rust
// ✅ Object pooling for frequent allocations
use object_pool::Pool;

pub struct SpanPool {
    pool: Pool<Span>,
}

impl SpanPool {
    pub fn new() -> Self {
        Self {
            pool: Pool::new(|| Span::default(), |span| span.reset()),
        }
    }

    #[inline]
    pub fn get(&self) -> object_pool::Reusable<Span> {
        self.pool.pull()
    }
}

// ✅ Arena allocation for temporary data
use bumpalo::Bump;

pub fn process_spans_in_arena(spans: &[SpanData]) -> Result<Vec<ServiceMetrics>> {
    let arena = Bump::new();
    // All temporary allocations use the arena
    let temp_metrics = arena.alloc_slice_fill_copy(spans.len(), ServiceMetrics::default());

    // Process without individual allocations
    for (i, span) in spans.iter().enumerate() {
        temp_metrics[i].update_from_span(span);
    }

    // Only allocate final result
    Ok(temp_metrics.to_vec())
}
```

### 5. **SIMD and Vectorization**
```rust
// ✅ Use SIMD for batch operations
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

// ✅ Batch process spans for cache efficiency
pub fn calculate_percentiles_batch(durations: &mut [u64]) -> Percentiles {
    // Sort in-place for cache efficiency
    durations.sort_unstable();

    let len = durations.len();
    Percentiles {
        p50: durations[len / 2],
        p95: durations[len * 95 / 100],
        p99: durations[len * 99 / 100],
    }
}

// ✅ Use bit manipulation for fast operations
#[inline(always)]
pub fn next_power_of_two(n: usize) -> usize {
    1 << (64 - n.leading_zeros())
}
```

---

## 🔥 EXTREME PERFORMANCE PATTERNS

### 1. **Zero-Copy Parsing**
```rust
// ✅ Parse without allocation using nom or custom parsers
use nom::{bytes::complete::take, IResult};

pub fn parse_trace_header(input: &[u8]) -> IResult<&[u8], TraceHeader> {
    let (input, trace_id) = take(16u8)(input)?;
    let (input, span_id) = take(8u8)(input)?;
    let (input, flags) = take(1u8)(input)?;

    Ok((input, TraceHeader {
        trace_id: TraceId::from_bytes(trace_id),
        span_id: SpanId::from_bytes(span_id),
        flags: flags[0],
    }))
}

// ✅ Use Cow for zero-copy when possible
use std::borrow::Cow;

pub fn normalize_service_name(name: &str) -> Cow<'_, str> {
    if name.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
        Cow::Borrowed(name)  // Zero allocation!
    } else {
        Cow::Owned(name.to_ascii_lowercase().replace('_', "-"))
    }
}
```

### 2. **Cache-Optimized Data Layouts**
```rust
// ✅ Struct of Arrays (SoA) for better cache usage
pub struct SpanBatch {
    trace_ids: Vec<TraceId>,
    span_ids: Vec<SpanId>,
    start_times: Vec<u64>,
    durations: Vec<u32>,     // Store as u32 nanoseconds for cache efficiency
    service_names: Vec<u16>, // Index into string interning table
}

// ✅ Pack small values to reduce memory usage
#[repr(packed)]
pub struct PackedSpan {
    trace_id: u128,      // 16 bytes
    span_id: u64,        // 8 bytes
    start_time: u64,     // 8 bytes
    duration_ns: u32,    // 4 bytes
    service_idx: u16,    // 2 bytes - index into service table
    flags: u8,           // 1 byte
    _padding: u8,        // 1 byte - total 40 bytes
}
```

### 3. **Async Performance Optimization**
```rust
// ✅ Use channels with optimal buffer sizes
use tokio::sync::mpsc;

pub fn create_optimized_channels() -> (SpanSender, SpanReceiver) {
    // Buffer size tuned for L3 cache
    let buffer_size = 8192;
    mpsc::channel(buffer_size)
}

// ✅ Batch async operations
pub async fn flush_metrics_batch(
    aggregator: &MetricsAggregator,
    batch_size: usize,
) -> Result<()> {
    let mut batch = Vec::with_capacity(batch_size);

    while let Some(metric) = aggregator.try_recv() {
        batch.push(metric);

        if batch.len() >= batch_size {
            process_metrics_batch(&batch).await?;
            batch.clear();
        }
    }

    if !batch.is_empty() {
        process_metrics_batch(&batch).await?;
    }

    Ok(())
}

// ✅ Use spawn_blocking for CPU-intensive work
pub async fn process_large_trace(trace: LargeTrace) -> Result<TraceAnalysis> {
    tokio::task::spawn_blocking(move || {
        // CPU-intensive computation on thread pool
        analyze_trace_patterns(&trace)
    }).await?
}
```

### 4. **Memory Management Excellence**
```rust
// ✅ Use specific allocators for performance
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// ✅ Implement custom Drop for optimal cleanup
impl Drop for TraceStorage {
    fn drop(&mut self) {
        // Batch free memory for better performance
        self.spans.clear();
        self.spans.shrink_to_fit();
    }
}

// ✅ Use MaybeUninit for uninitialized arrays
use std::mem::MaybeUninit;

pub fn process_span_array() -> [ProcessedSpan; 1000] {
    let mut spans: [MaybeUninit<ProcessedSpan>; 1000] = unsafe {
        MaybeUninit::uninit().assume_init()
    };

    for i in 0..1000 {
        spans[i] = MaybeUninit::new(ProcessedSpan::default());
    }

    unsafe { std::mem::transmute(spans) }
}
```

---

## 📊 BENCHMARKING REQUIREMENTS

### 1. **Mandatory Benchmarks**
```rust
// ✅ Benchmark critical paths
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_span_processing(c: &mut Criterion) {
    let spans = generate_test_spans(10_000);

    c.bench_function("process_spans", |b| {
        b.iter(|| {
            let processor = SpanProcessor::new();
            black_box(processor.process_batch(black_box(&spans)))
        })
    });
}

fn bench_service_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("aggregation");

    for size in [100, 1000, 10_000, 100_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                let spans = generate_test_spans(size);
                b.iter(|| aggregate_service_metrics(black_box(&spans)))
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_span_processing, bench_service_aggregation);
criterion_main!(benches);
```

### 2. **Performance Targets (Enforce with CI)**
```rust
// ✅ Regression tests in CI
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_span_processing_performance() {
        let spans = generate_test_spans(10_000);
        let start = Instant::now();

        let processor = SpanProcessor::new();
        let _result = processor.process_batch(&spans);

        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 100,
            "Span processing too slow: {}ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_memory_usage() {
        let initial_memory = get_memory_usage();

        {
            let storage = TraceStorage::new(100_000);
            let spans = generate_test_spans(50_000);
            storage.store_spans(spans);

            let peak_memory = get_memory_usage();
            assert!(
                peak_memory - initial_memory < 100_000_000, // 100MB
                "Memory usage too high: {} bytes",
                peak_memory - initial_memory
            );
        }

        // Ensure cleanup
        force_gc();
        let final_memory = get_memory_usage();
        assert!(
            final_memory - initial_memory < 10_000_000, // 10MB
            "Memory leak detected: {} bytes",
            final_memory - initial_memory
        );
    }
}
```

---

## 🛠️ TAURI-SPECIFIC PERFORMANCE

### 1. **Optimal Tauri Commands**
```rust
// ✅ Batch Tauri commands to reduce IPC overhead
#[tauri::command]
async fn get_service_metrics_batch(
    window: tauri::Window,
    service_names: Vec<String>,
) -> Result<Vec<ServiceMetrics>, String> {
    // Process all requests in one command
    let metrics = service_names
        .into_iter()
        .map(|name| get_service_metrics(&name))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(metrics)
}

// ✅ Use streaming for large datasets
#[tauri::command]
async fn stream_trace_data(
    window: tauri::Window,
    trace_id: String,
) -> Result<(), String> {
    let chunks = get_trace_chunks(&trace_id).map_err(|e| e.to_string())?;

    for chunk in chunks {
        window.emit("trace-chunk", &chunk).map_err(|e| e.to_string())?;
        // Yield to prevent blocking
        tokio::task::yield_now().await;
    }

    window.emit("trace-complete", ()).map_err(|e| e.to_string())?;
    Ok(())
}
```

### 2. **Frontend Performance Guidelines**
```typescript
// ✅ Use efficient data structures in frontend
interface OptimizedSpanData {
  traceId: string;
  spanId: string;
  duration: number;    // pre-calculated for sorting
  startTime: number;   // unix timestamp for fast comparison
  serviceIndex: number; // index into services array
}

// ✅ Batch UI updates
const updateTraceView = useMemo(() =>
  debounce((spans: SpanData[]) => {
    // Batch DOM updates
    requestAnimationFrame(() => {
      setTraceData(spans);
    });
  }, 16) // 60fps
, []);

// ✅ Use virtual scrolling for large lists
const VirtualSpanList = memo(({ spans }: { spans: SpanData[] }) => {
  return (
    <FixedSizeList
      height={600}
      itemCount={spans.length}
      itemSize={24}
      overscanCount={10}
    >
      {SpanRow}
    </FixedSizeList>
  );
});
```

---

## ⚡ EXTREME OPTIMIZATION CHECKLIST

### Before Every Commit:
- [ ] **No `.unwrap()` in hot paths** - Use `unsafe` if needed
- [ ] **No allocations in tight loops** - Pre-allocate or use iterators
- [ ] **Profile with `perf`** - Verify no unexpected bottlenecks
- [ ] **Benchmark regressions** - All benchmarks must pass
- [ ] **Memory leak check** - Run with valgrind or similar
- [ ] **SIMD opportunities** - Check if batch operations can be vectorized
- [ ] **Cache misses** - Profile with `perf stat -e cache-misses`
- [ ] **Branch prediction** - Use `likely/unlikely` hints if applicable

### Cargo.toml Performance Settings:
```toml
[profile.release]
lto = "fat"              # Full link-time optimization
codegen-units = 1        # Better optimization
panic = "abort"          # Smaller binary, faster panic
opt-level = 3            # Maximum optimization
debug = false            # No debug info in release

[profile.bench]
inherits = "release"
debug = true             # Keep debug info for profiling
```

### Dependencies for Speed:
```toml
[dependencies]
# Ultra-fast allocator
mimalloc = "0.1"

# SIMD operations
wide = "0.7"

# Fast hashing
rustc-hash = "1.0"

# Lock-free data structures
dashmap = "5.0"
crossbeam = "0.8"

# Zero-copy serialization
rkyv = "0.7"

# Fast random numbers
fastrand = "2.0"

# Object pooling
object-pool = "2.0"

# Arena allocation
bumpalo = "3.0"
```

---

## 🎯 FINAL PERFORMANCE MANIFESTO

**REMEMBER:** Every microsecond counts. We're not just building software, we're building **the fastest trace explorer in existence**.

**Urpo shows traces in 200ms with minimal resource usage.**
**Urpo uses only 50MB of RAM for efficient operation.**
**Urpo handles 100,000+ spans with ease.**

**Build for excellence and performance.** 🔥⚡

---

## Key Rules Summary

1. **Zero allocations in hot paths** - Profile and optimize aggressively
2. **Lock-free where possible** - Use atomic operations and channels
3. **Batch everything** - Network calls, UI updates, disk writes
4. **Memory pools** - Reuse objects, avoid GC pressure
5. **SIMD when possible** - Vectorize calculations
6. **Benchmark everything** - No performance regressions allowed
7. **Profile relentlessly** - Use perf, flamegraphs, criterion
8. **Cache-conscious** - Optimize data layout for CPU cache

**Remember**: We're building the Ferrari of trace explorers. Every line of code should scream SPEED! 🏎️💨

---

## 📋 OPENTELEMETRY COMPLIANCE REQUIREMENTS

**Based on [Official OTEL Library Guidelines](https://opentelemetry.io/docs/specs/otel/library-guidelines/)**

### 1. **Protocol Implementation Standards**
```rust
// ✅ Support both OTLP transport protocols
pub struct OtelReceiver {
    grpc_server: GrpcServer,     // Port 4317 - Primary protocol
    http_server: HttpServer,     // Port 4318 - JSON over HTTP
}

// ✅ Handle all OTLP signal types
pub enum OtelSignal {
    Traces(TraceData),
    Metrics(MetricData),   // Future: metrics support
    Logs(LogData),         // Future: logs support
}

// ✅ Implement proper OTLP status codes
#[derive(Debug, Clone)]
pub enum ExportResult {
    Success,
    PartialSuccess { dropped_items: u32, error_message: String },
    Failure(String),
}
```

### 2. **Resource Detection Compliance**
```rust
// ✅ Extract resource information per OTEL spec
pub fn extract_service_info(resource: &Resource) -> ServiceInfo {
    let mut service_name = "unknown_service".to_string();
    let mut service_version = None;
    let mut service_namespace = None;

    for attribute in &resource.attributes {
        match attribute.key.as_str() {
            "service.name" => service_name = extract_string_value(&attribute.value),
            "service.version" => service_version = Some(extract_string_value(&attribute.value)),
            "service.namespace" => service_namespace = Some(extract_string_value(&attribute.value)),
            _ => {} // Store in metadata map
        }
    }

    ServiceInfo {
        name: ServiceName::new(service_name).unwrap_or_default(),
        version: service_version,
        namespace: service_namespace,
    }
}

// ✅ Support semantic conventions
pub mod semantic_conventions {
    pub const SERVICE_NAME: &str = "service.name";
    pub const SERVICE_VERSION: &str = "service.version";
    pub const HTTP_METHOD: &str = "http.method";
    pub const HTTP_STATUS_CODE: &str = "http.status_code";
    pub const DB_SYSTEM: &str = "db.system";
    pub const RPC_SERVICE: &str = "rpc.service";
}
```

### 3. **Wire Protocol Efficiency**
```rust
// ✅ Zero-copy protobuf parsing where possible
use prost::Message;
use bytes::Bytes;

pub fn parse_trace_request_zero_copy(
    data: Bytes,
) -> Result<ExportTraceServiceRequest> {
    // Parse without intermediate allocations
    ExportTraceServiceRequest::decode(data.as_ref())
        .map_err(|e| UrpoError::ProtocolError(e.to_string()))
}

// ✅ Batch processing for optimal performance
pub async fn process_spans_batch(
    spans: Vec<opentelemetry_proto::tonic::trace::v1::Span>,
    resource: &Resource,
) -> Result<Vec<ProcessedSpan>> {
    // Pre-allocate based on input size
    let mut processed = Vec::with_capacity(spans.len());
    let service_info = extract_service_info(resource);

    // Batch convert without individual allocations
    for span in spans {
        let processed_span = convert_span_zero_copy(span, &service_info)?;
        processed.push(processed_span);
    }

    Ok(processed)
}
```

### 4. **Exporter Interface Compatibility**
```rust
// ✅ Implement standard OTEL exporter interface for plugins
#[async_trait]
pub trait SpanExporter: Send + Sync {
    /// Export a batch of spans
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult;

    /// Force flush any buffered spans
    async fn force_flush(&mut self) -> ExportResult;

    /// Shutdown the exporter
    async fn shutdown(&mut self) -> ExportResult;
}

// ✅ Built-in exporters following OTEL spec
pub struct StdoutExporter;
pub struct JaegerExporter;
pub struct ZipkinExporter;
pub struct PrometheusExporter; // For metrics

impl SpanExporter for StdoutExporter {
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult {
        for span in batch {
            println!("{}", serde_json::to_string(&span).unwrap());
        }
        ExportResult::Success
    }
}
```

### 5. **Sampling Implementation**
```rust
// ✅ Implement OTEL sampling spec
pub trait Sampler: Send + Sync {
    fn should_sample(
        &self,
        context: &SpanContext,
        trace_id: TraceId,
        name: &str,
        kind: SpanKind,
        attributes: &[KeyValue],
        links: &[Link],
    ) -> SamplingResult;

    fn description(&self) -> String;
}

// ✅ Standard samplers per OTEL spec
pub struct AlwaysOnSampler;
pub struct AlwaysOffSampler;
pub struct TraceIdRatioBasedSampler { ratio: f64 };

impl Sampler for TraceIdRatioBasedSampler {
    fn should_sample(&self, ctx: &SpanContext, trace_id: TraceId, ...) -> SamplingResult {
        let trace_id_int = u64::from_be_bytes(
            trace_id.as_bytes()[8..16].try_into().unwrap()
        );
        let threshold = (self.ratio * (u64::MAX as f64)) as u64;

        if trace_id_int < threshold {
            SamplingResult::RecordAndSample
        } else {
            SamplingResult::Drop
        }
    }
}
```

### 6. **Performance & Blocking Compliance**
```rust
// ✅ Non-blocking API calls per OTEL performance spec
pub struct AsyncSpanProcessor {
    sender: tokio::sync::mpsc::UnboundedSender<SpanProcessorMessage>,
    _handle: tokio::task::JoinHandle<()>,
}

impl AsyncSpanProcessor {
    pub fn new(exporter: Box<dyn SpanExporter>) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        let handle = tokio::spawn(async move {
            let mut batch = Vec::with_capacity(512);
            let mut exporter = exporter;

            // Process spans without blocking API calls
            while let Some(msg) = receiver.recv().await {
                match msg {
                    SpanProcessorMessage::ExportSpan(span) => {
                        batch.push(span);

                        // Batch for efficiency
                        if batch.len() >= 512 {
                            let _ = exporter.export(std::mem::take(&mut batch)).await;
                        }
                    }
                    SpanProcessorMessage::ForceFlush => {
                        if !batch.is_empty() {
                            let _ = exporter.export(std::mem::take(&mut batch)).await;
                        }
                        let _ = exporter.force_flush().await;
                    }
                    SpanProcessorMessage::Shutdown => break,
                }
            }
        });

        Self { sender, _handle: handle }
    }

    // ✅ Never block - always return immediately
    pub fn on_end(&self, span: SpanData) {
        let _ = self.sender.send(SpanProcessorMessage::ExportSpan(span));
    }
}
```

### 7. **Version and Compatibility Management**
```rust
// ✅ Support semantic versioning per OTEL spec
pub const URPO_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const OTEL_SPEC_VERSION: &str = "1.29.0";
pub const SUPPORTED_OTLP_VERSION: &str = "1.0.0";

// ✅ Version validation for protocol compatibility
pub fn validate_otlp_version(client_version: &str) -> Result<()> {
    let client_ver = semver::Version::parse(client_version)?;
    let supported_ver = semver::Version::parse(SUPPORTED_OTLP_VERSION)?;

    if client_ver.major != supported_ver.major {
        return Err(UrpoError::UnsupportedVersion {
            client: client_version.to_string(),
            supported: SUPPORTED_OTLP_VERSION.to_string(),
        });
    }

    Ok(())
}

// ✅ Telemetry headers per OTEL spec
pub const USER_AGENT: &str = concat!(
    "urpo/", env!("CARGO_PKG_VERSION"),
    " (", env!("TARGET"), ")",
    " rust/", env!("RUSTC_VERSION")
);
```

---

## 🏆 OTEL COMPLIANCE CHECKLIST

### Protocol Support:
- [ ] **OTLP/gRPC** - Primary protocol (port 4317)
- [ ] **OTLP/HTTP** - JSON over HTTP (port 4318)
- [ ] **Zipkin JSON** - Legacy compatibility
- [ ] **Jaeger Thrift** - Legacy compatibility

### Signal Types:
- [x] **Traces** - Distributed tracing (primary focus)
- [ ] **Metrics** - Application metrics (future)
- [ ] **Logs** - Structured logging (future)

### Standard Exporters:
- [x] **Console/Stdout** - Debug output
- [ ] **Zipkin** - Legacy trace backend
- [ ] **Jaeger** - Legacy trace backend
- [ ] **Prometheus** - Metrics export (future)

### Sampling:
- [x] **Always On** - Sample everything
- [x] **Always Off** - Sample nothing
- [x] **Ratio-based** - Probabilistic sampling
- [ ] **Rate-limited** - Adaptive sampling (future)

### Resource Detection:
- [x] **Service identification** - service.name, service.version
- [ ] **Environment detection** - Cloud provider, K8s metadata
- [ ] **Host information** - OS, architecture, hostname

### Performance Requirements:
- [x] **Non-blocking API** - Never block caller threads
- [x] **Bounded memory** - Prevent memory leaks
- [x] **Batch processing** - Optimize throughput
- [x] **Fast startup** - <200ms initialization

**Remember**: We're building the Ferrari of trace explorers. Every line of code should scream SPEED! 🏎️💨

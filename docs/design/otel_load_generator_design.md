# OTEL Load Generator Design Session

## Design Session Checklist

### ✅ What problem are we solving?

We need to **PROVE** Urpo's performance superiority with **REAL OpenTelemetry data**, not simulations. Current issues:
- No way to demonstrate actual OTLP protocol handling
- Can't show real-time ingestion rates visually
- Can't stress test with realistic trace patterns
- Can't compare against Jaeger/Tempo with same workload

**Goal**: Create a load generator that sends massive amounts of OTLP data and shows Urpo handling it effortlessly.

### ✅ What's the simplest solution?

A standalone Rust binary that:
1. Generates realistic OTEL trace data
2. Sends it via OTLP/gRPC to Urpo (port 4317)
3. Shows real-time metrics (spans/sec, latency, errors)
4. Has visual progress bars and performance indicators
5. Can scale from 1K to 1M+ spans/second

### ✅ Can we break it into smaller functions?

```rust
// Core components:
1. SpanGenerator     - Creates realistic span data
2. TraceBuilder      - Assembles spans into traces
3. OtlpSender        - Sends via gRPC with batching
4. MetricsCollector  - Tracks performance metrics
5. VisualDisplay     - Shows real-time performance
6. LoadController    - Controls rate and patterns
```

### ✅ What interfaces do we need?

```rust
// Main interfaces:

trait LoadPattern {
    fn generate_next_batch(&mut self) -> Vec<Span>;
}

trait MetricsReporter {
    fn report_sent(&self, count: usize, latency: Duration);
    fn report_error(&self, error: &str);
    fn get_stats(&self) -> LoadStats;
}

struct LoadConfig {
    target_url: String,        // localhost:4317
    spans_per_second: u32,     // Target rate
    batch_size: usize,         // Spans per batch
    total_spans: Option<u64>,  // Total to send (or infinite)
    pattern: LoadPattern,      // Traffic pattern
    services: Vec<String>,     // Service names to use
    error_rate: f32,          // % of error spans
}
```

### ✅ What can go wrong?

1. **Network saturation** - Need backpressure handling
2. **Memory explosion** - Must use bounded channels
3. **CPU bottleneck** - Use parallel generation
4. **Receiver overwhelm** - Implement rate limiting
5. **Unrealistic data** - Need diverse, realistic patterns
6. **No visibility** - Must show clear metrics

### ✅ Draw the flow (ASCII diagram)

```
┌─────────────────────────────────────────────────────────────┐
│                    OTEL LOAD GENERATOR                       │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐                │
│  │  Config  │──▶│Generator │──▶│  Batch   │                │
│  │  Parser  │   │  Pool    │   │  Queue   │                │
│  └──────────┘   └──────────┘   └──────────┘                │
│                      │               │                       │
│                      ▼               ▼                       │
│              ┌──────────────────────────┐                   │
│              │   Parallel Workers (N)    │                  │
│              ├──────────────────────────┤                   │
│              │ • Generate spans         │                   │
│              │ • Build traces           │                   │
│              │ • Add realistic data     │                   │
│              └──────────────────────────┘                   │
│                           │                                  │
│                           ▼                                  │
│                   ┌──────────────┐                          │
│                   │   Batcher    │ (Optimal batch size)    │
│                   └──────────────┘                          │
│                           │                                  │
│                           ▼                                  │
│                   ┌──────────────┐                          │
│                   │ OTLP Sender  │ (gRPC/HTTP)             │
│                   └──────────────┘                          │
│                           │                                  │
│                           ▼                                  │
│                    ╔════════════╗                           │
│                    ║   URPO     ║ :4317                     │
│                    ╚════════════╝                           │
│                           │                                  │
│                           ▼                                  │
│  ┌────────────────────────────────────────────────────┐    │
│  │              REAL-TIME METRICS DISPLAY              │    │
│  ├────────────────────────────────────────────────────┤    │
│  │  Sent: 1,234,567 spans                             │    │
│  │  Rate: 156,789/sec [████████████████░░░░]         │    │
│  │  Latency: 0.8ms p50, 1.2ms p99                    │    │
│  │  Errors: 0 (0.00%)                                 │    │
│  │  Memory: 45MB                                       │    │
│  │  CPU: 12%                                          │    │
│  └────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Strategy

### Phase 1: Core Generator (MVP)
- [ ] Basic span generation with realistic data
- [ ] OTLP/gRPC client using tonic
- [ ] Simple rate control (spans/sec)
- [ ] Basic console output

### Phase 2: Advanced Patterns
- [ ] Multiple trace patterns (microservices, monolith, serverless)
- [ ] Realistic latency distributions
- [ ] Error injection patterns
- [ ] Service dependency graphs

### Phase 3: Visual Excellence
- [ ] Real-time TUI with ratatui
- [ ] Performance graphs
- [ ] Comparison mode (send to multiple endpoints)
- [ ] Record & replay functionality

### Phase 4: Stress Testing
- [ ] Burst mode (sudden spikes)
- [ ] Gradual ramp-up
- [ ] Sustained load tests
- [ ] Chaos patterns

## Load Patterns to Implement

### 1. **E-commerce Pattern**
```
frontend -> api-gateway -> [cart, catalog, payment]
- High volume on catalog
- Bursts on cart
- Careful on payment
```

### 2. **Microservices Mesh**
```
service-a -> service-b -> service-c
         \-> service-d -> service-e
- Deep traces (10+ spans)
- Fan-out patterns
- Circuit breaker scenarios
```

### 3. **Serverless Pattern**
```
lambda-invoker -> [lambda-1, lambda-2, ...lambda-n]
- Cold starts (high latency)
- Rapid scaling
- Short-lived traces
```

### 4. **Database Heavy**
```
api -> db-primary -> db-replica
- Slow queries
- Connection pool exhaustion
- Replication lag
```

## Performance Targets

The generator itself must be FAST:
- Generate 1M+ spans/second on modern hardware
- Use <100MB RAM for generator
- <5% CPU overhead vs raw network capacity
- Zero allocations in hot path
- Batching for optimal network usage

## Success Metrics

1. **Visual Impact**: See the bars fill up, numbers climb
2. **Real Load**: Actual OTLP protocol, not fake data
3. **Comparison Ready**: Can point at Jaeger/Tempo too
4. **Reproducible**: Same load every time
5. **Impressive**: Make engineers say "WOW!"

## Example Usage

```bash
# Basic load test
urpo-load --rate 10000 --duration 60s

# Stress test with visualization
urpo-load --rate 100000 --pattern microservices --visual

# Comparison mode
urpo-load --rate 50000 --targets urpo:4317,jaeger:4317 --compare

# Burst test
urpo-load --burst --peak 500000 --normal 10000

# Replay production pattern
urpo-load --replay production-trace-pattern.json
```

## Key Differentiators

Unlike existing tools (Jaeger's load generator, etc):
1. **BEAUTIFUL** - Not just text, but visual bars and colors
2. **REALISTIC** - Actual production patterns, not random data
3. **FAST** - Can actually stress test (most generators bottleneck)
4. **COMPARATIVE** - Send to multiple endpoints simultaneously
5. **RUST** - Zero overhead, maximum performance

## Next Steps

1. Start with basic OTLP sender (Phase 1)
2. Add visual display immediately (impact!)
3. Implement one realistic pattern
4. Test against local Urpo
5. Add comparison mode
6. Create demo video showing Urpo destroying competition

---

**Remember**: This isn't just a load generator - it's a **PERFORMANCE DEMONSTRATION TOOL** that will make people choose Urpo!
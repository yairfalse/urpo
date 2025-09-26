# 🔥 URPO TEST SUITE DESIGN - PRODUCTION GRADE

## 📊 Current Coverage Analysis

**Coverage Status:**
- **Files with tests:** 35/52 (67%)
- **Test functions:** ~317 tests
- **Critical gaps:** Integration, E2E, Performance, Load

## 🎯 Test Strategy - "Trust but Verify Everything"

### Level 1: Unit Tests (Fast, Isolated)
**Target:** <1ms per test, runs on every save

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_point_size() {
        // Verify cache-line optimization
        assert_eq!(std::mem::size_of::<MetricPoint>(), 32);
    }

    #[test]
    fn test_span_pool_zero_alloc() {
        // Verify no allocations in hot path
        let pool = ZeroAllocSpanPool::new(100);
        let before = get_allocation_count();
        let span = pool.try_get_or_new();
        let after = get_allocation_count();
        assert_eq!(before, after, "Pool should not allocate");
    }
}
```

### Level 2: Integration Tests (Component Boundaries)
**Target:** <100ms per test, runs on commit

```rust
// tests/integration/otlp_receiver_test.rs
#[tokio::test]
async fn test_otlp_grpc_full_pipeline() {
    // Start receiver
    let receiver = create_test_receiver().await;

    // Send OTLP data
    let client = create_otlp_client("localhost:4317");
    client.send_traces(generate_test_traces(1000)).await;

    // Verify storage
    let storage = receiver.storage();
    assert_eq!(storage.span_count(), 1000);

    // Verify metrics updated
    let metrics = receiver.metrics();
    assert!(metrics.spans_per_second > 0.0);
}
```

### Level 3: Performance Tests (Benchmarks)
**Target:** Regression detection, runs on PR

```rust
// benches/critical_paths.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_span_ingestion(c: &mut Criterion) {
    let mut group = c.benchmark_group("span_ingestion");

    for size in [100, 1000, 10_000, 100_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                let spans = generate_spans(size);
                b.iter(|| {
                    process_spans(black_box(&spans))
                });
            }
        );
    }

    // MUST maintain <10μs per span
    group.finish();
}
```

### Level 4: Load Tests (Production Simulation)
**Target:** Stability under stress, runs nightly

```rust
// tests/load/stress_test.rs
#[tokio::test]
async fn test_sustained_load() {
    let receiver = start_production_receiver().await;

    // Simulate production load
    let handles = (0..10).map(|i| {
        tokio::spawn(async move {
            let client = create_client(i);
            for _ in 0..100_000 {
                client.send_span(generate_span()).await;
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        })
    });

    // Run for 1 minute
    tokio::time::sleep(Duration::from_secs(60)).await;

    // Verify no memory leaks
    assert!(get_memory_usage() < 100_000_000); // <100MB

    // Verify no dropped spans
    assert_eq!(receiver.dropped_spans(), 0);
}
```

### Level 5: End-to-End Tests (User Scenarios)
**Target:** Real workflows, runs before release

```rust
// tests/e2e/user_journey_test.rs
#[tokio::test]
async fn test_complete_user_workflow() {
    // Start Urpo
    let urpo = UrpoApplication::new(test_config()).await;
    urpo.start().await;

    // Send traces from multiple services
    simulate_microservices_traffic().await;

    // Query via API
    let client = UrpoApiClient::new("localhost:8080");
    let traces = client.query_traces("service.name='payment'").await;
    assert!(traces.len() > 0);

    // Export to Jaeger
    let exported = client.export_jaeger(traces[0].id).await;
    assert!(exported.is_ok());

    // Check TUI renders
    let tui_output = capture_tui_output().await;
    assert!(tui_output.contains("payment"));
}
```

## 🏗️ Test Infrastructure

### 1. Test Fixtures & Factories

```rust
// src/test_utils/fixtures.rs
pub struct TestFixtures {
    spans: Vec<Span>,
    metrics: Vec<MetricPoint>,
    logs: Vec<LogEntry>,
}

impl TestFixtures {
    pub fn realistic_traces(count: usize) -> Vec<Span> {
        // Generate realistic trace patterns
        let mut spans = Vec::with_capacity(count);
        for i in 0..count/10 {
            spans.extend(Self::create_trace_tree(i, 10));
        }
        spans
    }

    fn create_trace_tree(trace_num: usize, depth: usize) -> Vec<Span> {
        // Realistic parent-child relationships
        // Proper timing relationships
        // Error injection (10% error rate)
    }
}
```

### 2. Mock OTLP Client

```rust
// src/test_utils/otlp_mock.rs
pub struct MockOtlpClient {
    endpoint: String,
    error_rate: f32,
}

impl MockOtlpClient {
    pub async fn send_realistic_traffic(&self, duration: Duration) {
        let start = Instant::now();
        while start.elapsed() < duration {
            // Send varied traffic patterns
            self.send_burst_traffic().await;
            self.send_steady_traffic().await;
            self.send_error_spike().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
```

### 3. Assertion Helpers

```rust
// src/test_utils/assertions.rs
#[macro_export]
macro_rules! assert_performance {
    ($op:expr, $limit:expr) => {
        let start = Instant::now();
        let result = $op;
        let elapsed = start.elapsed();
        assert!(
            elapsed < $limit,
            "Performance assertion failed: {:?} > {:?}",
            elapsed, $limit
        );
        result
    };
}

#[macro_export]
macro_rules! assert_no_allocations {
    ($op:expr) => {
        let before = ALLOCATIONS.load(Ordering::SeqCst);
        let result = $op;
        let after = ALLOCATIONS.load(Ordering::SeqCst);
        assert_eq!(before, after, "Unexpected allocations detected");
        result
    };
}
```

## 🎭 Test Scenarios

### Critical Path Tests

1. **OTLP Protocol Compliance**
   - Valid spans accepted
   - Invalid spans rejected gracefully
   - Resource semantics extracted
   - All OTLP fields preserved

2. **Performance Boundaries**
   - 100K spans/second sustained
   - <10μs per span processing
   - <100MB memory at 1M spans
   - Zero allocations in hot paths

3. **Error Resilience**
   - Malformed protobuf handling
   - Network interruptions
   - Storage failures
   - Memory pressure

4. **Data Integrity**
   - No span loss under load
   - Correct parent-child relationships
   - Accurate timing calculations
   - Proper attribute preservation

### Negative Tests

```rust
#[test]
fn test_malformed_trace_id() {
    let result = TraceId::new("not-a-hex-string");
    assert!(result.is_err());
}

#[test]
fn test_span_duration_overflow() {
    let span = create_span_with_duration(u64::MAX);
    assert!(validate_span(&span).is_err());
}

#[test]
async fn test_storage_full() {
    let storage = create_storage_with_limit(100);
    for i in 0..200 {
        storage.store_span(create_span()).await;
    }
    assert_eq!(storage.span_count(), 100);
    assert!(storage.oldest_span_age() < Duration::from_secs(60));
}
```

### Chaos Tests

```rust
#[tokio::test]
async fn test_random_failures() {
    let chaos = ChaosMonkey::new()
        .with_network_failures(0.1)
        .with_random_delays(0..100)
        .with_memory_pressure(0.05);

    let receiver = create_receiver_with_chaos(chaos);

    // Send traffic for 5 minutes
    let client = create_client();
    for _ in 0..100_000 {
        let _ = client.send_span(generate_span()).await;
    }

    // System should still be functional
    assert!(receiver.is_healthy());
    assert!(receiver.span_loss_rate() < 0.01); // <1% loss
}
```

## 📈 Test Metrics

### Coverage Targets
- **Line Coverage:** >80%
- **Branch Coverage:** >70%
- **Critical Path Coverage:** 100%

### Performance Targets
- **Unit Tests:** <5 seconds total
- **Integration Tests:** <30 seconds total
- **E2E Tests:** <2 minutes total

### Quality Gates
- No test can be skipped
- No flaky tests allowed
- Performance regressions block merge
- Coverage can't decrease

## 🔧 Implementation Plan

### Phase 1: Unit Test Enhancement (Day 1)
```bash
# Add missing unit tests
cargo test --lib -- --nocapture
# Target: 100% critical path coverage
```

### Phase 2: Integration Tests (Day 2)
```bash
# Create integration test suite
cargo test --test '*' -- --test-threads=1
# Target: All component boundaries tested
```

### Phase 3: Performance Tests (Day 3)
```bash
# Setup criterion benchmarks
cargo bench --bench critical_paths
# Target: Baseline established, no regressions
```

### Phase 4: Load/Chaos Tests (Day 4)
```bash
# Run stress tests
cargo test --test load_test -- --ignored
# Target: Stable under 10x expected load
```

### Phase 5: E2E Tests (Day 5)
```bash
# Full user journey tests
cargo test --test e2e -- --ignored
# Target: All user workflows verified
```

## 🚀 CI/CD Integration

```yaml
# .github/workflows/test.yml
name: Comprehensive Test Suite

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    timeout-minutes: 5
    steps:
      - run: cargo test --lib

  integration-tests:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - run: cargo test --test '*'

  performance-tests:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - run: cargo bench -- --save-baseline PR_${{ github.event.number }}
      - run: cargo bench -- --baseline main
      - name: Check regression
        run: |
          if grep -q "regression" bench_output.txt; then
            exit 1
          fi

  load-tests:
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    schedule:
      - cron: '0 2 * * *' # Nightly
    steps:
      - run: cargo test --test load_test -- --ignored
```

## ✅ Success Criteria

1. **Zero flaky tests** - Every test passes 100% of the time
2. **Fast feedback** - Unit tests complete in <5 seconds
3. **Production confidence** - E2E tests cover real scenarios
4. **Performance guard** - No regressions get through
5. **Easy debugging** - Clear test names and error messages

## 🎯 Ready to Build World-Class Test Suite!

With this test design, Urpo will be:
- **Battle-tested** - Every edge case covered
- **Performance-verified** - No surprises in production
- **Regression-proof** - Automated quality gates
- **Documentation** - Tests show how to use the system

The test suite IS the quality guarantee! 🚀
# Metrics Implementation Design Session

## Design Session Checklist

### ✅ What problem are we solving?
- Need to add OTEL metrics support to Urpo
- Must handle 50K metric points/second
- Must use <25MB memory for 1M metric points
- Must provide <5μs ingestion latency
- Must integrate with existing trace infrastructure

### ✅ What's the simplest solution?
1. **Ring buffer** for hot metrics (last 5 minutes)
2. **String interning** to avoid allocations
3. **Atomic counters** for lock-free aggregation
4. **Pre-aggregated service health** for instant queries
5. Reuse existing receiver pattern from traces

### ✅ Can we break it into smaller functions?
Yes! Small, testable chunks:
- `intern_string()` - String interning (10 lines)
- `push_metric()` - Add to ring buffer (15 lines)
- `aggregate_health()` - Update service health (20 lines)
- `query_metrics()` - Get metrics for time range (25 lines)
- `compress_cold()` - Move old metrics to compressed storage (30 lines)

### ✅ What interfaces do we need?
```rust
trait MetricStorage {
    fn store_metric(&self, metric: MetricPoint) -> Result<()>;
    fn query_metrics(&self, query: &MetricQuery) -> Result<Vec<MetricPoint>>;
    fn get_service_health(&self, service: &ServiceName) -> ServiceHealthMetrics;
}

trait MetricAggregator {
    fn aggregate(&self, metrics: &[MetricPoint]) -> AggregatedMetrics;
}
```

### ✅ What can go wrong?
- **Memory overflow** → Use bounded ring buffer with eviction
- **String explosion** → LRU cache for attribute values
- **Lock contention** → Use lock-free atomics and DashMap
- **Slow queries** → Pre-aggregate common queries
- **Data loss** → Background flush to disk

### ✅ Draw the flow (ASCII diagram)

```
OTEL Metrics Input
       ↓
┌─────────────────┐
│  OTLP Receiver  │ (Port 4317/4318)
└────────┬────────┘
         ↓
┌─────────────────┐
│ String Interning│ (<1μs)
└────────┬────────┘
         ↓
┌─────────────────┐
│  Ring Buffer    │ (5 min hot data)
│  (Lock-free)    │
└────────┬────────┘
         ↓
    ┌────┴────┐
    ↓         ↓
┌───────┐ ┌──────────┐
│Health │ │Compressed│
│Aggreg │ │Time Series│
└───────┘ └──────────┘
    ↓         ↓
┌─────────────────┐
│  Query Engine   │
└─────────────────┘
         ↓
    Terminal UI
```

## Implementation Order (Test-First)

1. **Core Types** (Day 1-2)
   - Write test for MetricType enum
   - Implement MetricType
   - Write test for MetricPoint struct
   - Implement MetricPoint

2. **String Interning** (Day 3-4)
   - Write test for string pool
   - Implement StringPool with tests passing
   - Benchmark allocation performance

3. **Ring Buffer** (Day 5-6)
   - Write test for ring buffer operations
   - Implement HotMetricsRing
   - Test concurrent access

4. **Service Health Aggregation** (Day 7-8)
   - Write test for aggregation
   - Implement atomic aggregators
   - Verify lock-free performance

5. **OTLP Receiver** (Day 9-10)
   - Write integration test
   - Add metrics endpoint to receiver
   - Test with real OTEL data

## Performance Targets to Test

- [ ] Ingestion: <5μs per metric point
- [ ] Memory: <25MB for 1M points
- [ ] Query: <1ms for service health
- [ ] Zero allocations in hot path
- [ ] 50K metrics/second throughput

## First Test to Write

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_type_counter() {
        let counter = MetricType::Counter {
            value: 42.0,
            exemplars: None
        };

        assert_eq!(counter.value(), 42.0);
        assert_eq!(counter.kind(), MetricKind::Counter);
    }

    #[test]
    fn test_metric_point_size() {
        // Ensure struct is cache-line optimized
        assert_eq!(std::mem::size_of::<MetricPoint>(), 32);
    }
}
```

## Next Steps
1. Create `src/metrics/` directory
2. Write first test file
3. Implement minimal code to pass
4. Run cargo fmt + clippy
5. Commit with test passing
6. Repeat for next component
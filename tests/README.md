# URPO Test Organization

## Test Structure Following Rust Best Practices

### Directory Layout
```
urpo/
├── src/                      # Source code with unit tests
│   ├── receiver/
│   │   └── mod.rs           # Contains #[cfg(test)] mod tests
│   ├── storage/
│   │   └── mod.rs           # Contains #[cfg(test)] mod tests
│   └── metrics/
│       └── storage.rs       # Contains #[cfg(test)] mod tests
├── tests/                    # Integration tests
│   ├── common/              # Shared test utilities
│   │   └── mod.rs
│   ├── otlp_integration.rs # OTLP protocol integration tests
│   ├── storage_integration.rs
│   ├── trace_exploration.rs
│   └── performance_tests.rs
└── benches/                  # Performance benchmarks
    ├── span_processing.rs
    ├── storage_bench.rs
    └── zero_alloc_benchmark.rs
```

## Test Categories

### 1. Unit Tests (in src/ files)
**Location**: Within each module using `#[cfg(test)]`
**Naming**: `test_<function_name>_<scenario>`
**Purpose**: Test individual functions in isolation

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_span_timing_valid() {
        // Test normal case
    }

    #[test]
    fn test_extract_span_timing_overflow() {
        // Test edge case
    }
}
```

### 2. Integration Tests (tests/ directory)

#### OTLP Protocol Tests (`otlp_integration.rs`)
- Full OTLP gRPC receiver tests
- Protocol compliance validation
- Resource extraction tests
- Batch processing tests

#### Storage Tests (`storage_integration.rs`)
- End-to-end storage operations
- Concurrent access patterns
- Memory limit enforcement
- Cleanup and retention

#### Trace Exploration Tests (`trace_exploration.rs`)
- Query functionality
- Service discovery
- Trace reconstruction
- Parent-child relationships

#### Performance Tests (`performance_tests.rs`)
- Load testing (100K spans/second)
- Memory usage validation (<100MB for 1M spans)
- Latency requirements (<10μs per span)
- Zero-allocation verification

### 3. Benchmarks (benches/ directory)

#### Span Processing (`span_processing.rs`)
- Single span processing time
- Batch processing throughput
- Protocol parsing overhead

#### Storage Operations (`storage_bench.rs`)
- Insert performance
- Query performance
- Concurrent operations

#### Zero Allocation (`zero_alloc_benchmark.rs`)
- Verify no allocations in hot paths
- Memory pool efficiency
- SIMD operation performance

## Running Tests

### Run all tests
```bash
cargo test
```

### Run unit tests only
```bash
cargo test --lib
```

### Run integration tests only
```bash
cargo test --test '*'
```

### Run specific integration test
```bash
cargo test --test otlp_integration
```

### Run with output
```bash
cargo test -- --nocapture
```

### Run benchmarks
```bash
cargo bench
```

### Run specific benchmark
```bash
cargo bench --bench span_processing
```

## Test Utilities (tests/common/)

### TestFixtures
- Generate realistic OTLP data
- Create test spans with proper relationships
- Mock service configurations

### TestServer
- Start test OTLP receiver
- Configure with test parameters
- Capture metrics for validation

### Assertions
- Performance assertions (timing requirements)
- Memory usage assertions
- Zero-allocation verification macros

## Coverage Requirements

- **Unit Tests**: 80% line coverage
- **Integration Tests**: All public APIs
- **Critical Paths**: 100% coverage required
  - OTLP protocol parsing
  - Span storage operations
  - Memory management
  - Error handling

## CI Integration

Tests run in this order:
1. `cargo fmt --check` - Code formatting
2. `cargo clippy` - Linting
3. `cargo test --lib` - Unit tests (fast)
4. `cargo test --test '*'` - Integration tests
5. `cargo bench --no-run` - Benchmark compilation
6. Performance regression check (on PR)
# Urpo OTEL Testing Suite

Comprehensive testing infrastructure for validating Urpo's OpenTelemetry (OTEL) packet handling, performance, and resilience.

## Prerequisites

### Install otelgen
```bash
# macOS
brew install krzko/tap/otelgen

# Linux/Other
# Download from: https://github.com/krzko/otelgen/releases
```

### Start Urpo
```bash
# Run Urpo in release mode for accurate performance testing
cargo run --release
```

## Test Scripts

### 1. `otelgen_stress_test.sh`
Main stress testing script for evaluating Urpo's performance and resilience.

#### Usage
```bash
# Interactive menu mode
./tests/otelgen_stress_test.sh

# Run all tests automatically (excludes memory leak test)
./tests/otelgen_stress_test.sh --all
```

#### Test Scenarios

| Test Type | Description | Load | Duration | Total Traces |
|-----------|-------------|------|----------|--------------|
| **Performance - Normal** | Baseline performance test | 100/sec | 30s | 3,000 |
| **Performance - High** | High throughput test | 1,000/sec | 60s | 60,000 |
| **Stress - Burst** | Sudden traffic spike | 10,000/sec | 10s | 100,000 |
| **Stress - Sustained** | Extended extreme load | 5,000/sec | 120s | 600,000 |
| **Negative - Large Payload** | Oversized attributes | 10/sec | 5s | 50 |
| **Negative - Protocol Switch** | Rapid gRPC/HTTP switching | 200/sec | 20s | 4,000 |
| **Negative - Connection Interrupt** | Abrupt disconnections | 500/sec | Variable | Variable |
| **Chaos - Concurrent** | 20 parallel generators | 100-600/sec each | 30s | ~180,000 |
| **Memory Leak** | Long-running stability | 200/sec | 300s | 60,000 |

### 2. `verify_urpo_data.sh`
Monitoring and verification script to validate Urpo's data processing.

#### Usage
```bash
# Generate full verification report
./tests/verify_urpo_data.sh --report

# Real-time monitoring (updates every 2 seconds)
./tests/verify_urpo_data.sh --monitor

# Check specific service data
./tests/verify_urpo_data.sh --service <service-name>

# Interactive menu
./tests/verify_urpo_data.sh
```

#### Features
- Health check validation
- Memory usage monitoring (target: <50MB)
- CPU usage monitoring
- Service and trace statistics
- Real-time metrics display

## Performance Targets

Based on Urpo's CLAUDE.md requirements:

| Metric | Target | Test Validation |
|--------|--------|-----------------|
| **Startup Time** | <200ms | Check with `time cargo run --release` |
| **Span Processing** | 10,000+ spans/sec | Stress - Burst test |
| **Memory Usage** | <100MB for 1M spans | Memory leak test + verification |
| **Error Recovery** | Immediate | Connection interrupt test |

## Testing Workflow

### 1. Basic Validation
```bash
# Start Urpo
cargo run --release

# In another terminal, run basic tests
./tests/otelgen_stress_test.sh
# Select option 2 (Normal load)

# Verify data reception
./tests/verify_urpo_data.sh --report
```

### 2. Performance Testing
```bash
# Terminal 1: Run Urpo
cargo run --release

# Terminal 2: Monitor resources
./tests/verify_urpo_data.sh --monitor

# Terminal 3: Run performance tests
./tests/otelgen_stress_test.sh
# Select option 3 (High load) or 4 (Burst)
```

### 3. Stress Testing
```bash
# Terminal 1: Run Urpo with logging
RUST_LOG=debug cargo run --release 2>&1 | tee urpo_stress.log

# Terminal 2: Run all stress tests
./tests/otelgen_stress_test.sh --all

# Terminal 3: Monitor system impact
watch -n 1 './tests/verify_urpo_data.sh --report'
```

### 4. Memory Leak Detection
```bash
# Start Urpo
cargo run --release

# Run long duration test
./tests/otelgen_stress_test.sh
# Select option 10 (Memory leak test - 5 minutes)

# Monitor memory in another terminal
./tests/verify_urpo_data.sh --monitor
```

## Interpreting Results

### Success Indicators
- ✅ All tests complete without Urpo crashing
- ✅ Memory stays under 100MB during normal operations
- ✅ CPU usage remains reasonable (<80% on average)
- ✅ Urpo recovers from connection interruptions
- ✅ No data loss during protocol switching

### Warning Signs
- ⚠️ Memory continuously increasing (potential leak)
- ⚠️ CPU pinned at 100% during normal load
- ⚠️ Urpo becomes unresponsive during burst tests
- ⚠️ Connection refused errors during stress tests

### Failure Indicators
- ❌ Urpo process crashes
- ❌ Memory exceeds 500MB
- ❌ Unable to handle 1,000 traces/sec
- ❌ Data corruption or missing traces

## Continuous Testing

For CI/CD integration, create a simple test runner:

```bash
#!/bin/bash
# ci_test.sh

# Start Urpo in background
cargo run --release &
URPO_PID=$!

# Wait for Urpo to start
sleep 5

# Run performance tests
./tests/otelgen_stress_test.sh --all

# Generate report
./tests/verify_urpo_data.sh --report > test_report.txt

# Check results
if grep -q "Memory usage within acceptable limits" test_report.txt; then
    echo "Tests PASSED"
    kill $URPO_PID
    exit 0
else
    echo "Tests FAILED"
    kill $URPO_PID
    exit 1
fi
```

## Troubleshooting

### otelgen not found
```bash
# Install otelgen
brew install krzko/tap/otelgen

# Or download binary
wget https://github.com/krzko/otelgen/releases/latest/download/otelgen_Linux_x86_64.tar.gz
tar -xzf otelgen_Linux_x86_64.tar.gz
sudo mv otelgen /usr/local/bin/
```

### Urpo not responding on port 4317
```bash
# Check if Urpo is running
ps aux | grep urpo

# Check port availability
lsof -i :4317

# Check Urpo logs
RUST_LOG=debug cargo run --release
```

### High memory usage
```bash
# Profile memory usage
valgrind --leak-check=full cargo run --release

# Or use heaptrack (Linux)
heaptrack cargo run --release
```

## Advanced Testing

### Custom Load Patterns
```bash
# Create custom test with specific parameters
otelgen traces multi \
    --otel-exporter-otlp-endpoint localhost:4317 \
    --protocol grpc \
    --insecure \
    --duration 60 \
    --rate 2500 \
    --otel-attributes service.name=custom-test \
    --otel-attributes custom.attribute=value
```

### Protocol-Specific Testing
```bash
# Test gRPC endpoint
otelgen traces multi --otel-exporter-otlp-endpoint localhost:4317 --protocol grpc --insecure

# Test HTTP endpoint
otelgen traces multi --otel-exporter-otlp-endpoint localhost:4318 --protocol http/protobuf --insecure
```

## Contributing

When adding new test scenarios:

1. Update `otelgen_stress_test.sh` with the new test function
2. Add verification logic to `verify_urpo_data.sh` if needed
3. Document the test scenario in this README
4. Include expected performance metrics

## License

Part of the Urpo project - see main LICENSE file.
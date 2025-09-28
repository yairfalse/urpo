#!/bin/bash

# URPO OTEL Stress Testing Script
# Tests how Urpo handles various OTEL packet scenarios

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
URPO_GRPC_ENDPOINT="${URPO_GRPC_ENDPOINT:-localhost:4317}"
URPO_HTTP_ENDPOINT="${URPO_HTTP_ENDPOINT:-localhost:4318}"
OTELGEN_BINARY="otelgen"

# Check if otelgen is installed
check_otelgen() {
    if ! command -v $OTELGEN_BINARY &> /dev/null; then
        echo -e "${RED}Error: otelgen not found!${NC}"
        echo "Install with: brew install krzko/tap/otelgen"
        echo "Or download from: https://github.com/krzko/otelgen/releases"
        exit 1
    fi
}

# Check if Urpo is running
check_urpo() {
    echo -e "${BLUE}Checking if Urpo is running...${NC}"
    if ! nc -z localhost 4317 2>/dev/null; then
        echo -e "${RED}Error: Urpo doesn't appear to be running on port 4317${NC}"
        echo "Start Urpo first with: cargo run --release"
        exit 1
    fi
    echo -e "${GREEN}✓ Urpo is running${NC}"
}

# Performance test - Normal load
test_performance_normal() {
    echo -e "\n${YELLOW}=== Performance Test: Normal Load ===${NC}"
    echo "Sending 100 traces/sec for 30 seconds (3,000 total traces)"

    $OTELGEN_BINARY traces multi \
        --otel-exporter-otlp-endpoint $URPO_GRPC_ENDPOINT \
        --protocol grpc \
        --insecure \
        --duration 30 \
        --rate 100 \
        --otel-attributes service.name=perf-test-normal \
        --otel-attributes environment=stress-test \
        --otel-attributes test.type=normal_load

    echo -e "${GREEN}✓ Normal load test completed${NC}"
}

# Performance test - High load
test_performance_high() {
    echo -e "\n${YELLOW}=== Performance Test: High Load ===${NC}"
    echo "Sending 1,000 traces/sec for 60 seconds (60,000 total traces)"

    $OTELGEN_BINARY traces multi \
        --otel-exporter-otlp-endpoint $URPO_GRPC_ENDPOINT \
        --protocol grpc \
        --insecure \
        --duration 60 \
        --rate 1000 \
        --otel-attributes service.name=perf-test-high \
        --otel-attributes environment=stress-test \
        --otel-attributes test.type=high_load

    echo -e "${GREEN}✓ High load test completed${NC}"
}

# Stress test - Burst traffic
test_stress_burst() {
    echo -e "\n${YELLOW}=== Stress Test: Burst Traffic ===${NC}"
    echo "Sending 10,000 traces/sec burst for 10 seconds (100,000 total traces)"

    $OTELGEN_BINARY traces multi \
        --otel-exporter-otlp-endpoint $URPO_GRPC_ENDPOINT \
        --protocol grpc \
        --insecure \
        --duration 10 \
        --rate 10000 \
        --otel-attributes service.name=stress-test-burst \
        --otel-attributes environment=stress-test \
        --otel-attributes test.type=burst

    echo -e "${GREEN}✓ Burst traffic test completed${NC}"
}

# Stress test - Sustained extreme load
test_stress_sustained() {
    echo -e "\n${YELLOW}=== Stress Test: Sustained Extreme Load ===${NC}"
    echo "Sending 5,000 traces/sec for 120 seconds (600,000 total traces)"
    echo -e "${RED}Warning: This test will generate significant load!${NC}"

    $OTELGEN_BINARY traces multi \
        --otel-exporter-otlp-endpoint $URPO_GRPC_ENDPOINT \
        --protocol grpc \
        --insecure \
        --duration 120 \
        --rate 5000 \
        --otel-attributes service.name=stress-test-sustained \
        --otel-attributes environment=stress-test \
        --otel-attributes test.type=sustained_extreme

    echo -e "${GREEN}✓ Sustained extreme load test completed${NC}"
}

# Negative test - Malformed data (simulate with very large attributes)
test_negative_large_payload() {
    echo -e "\n${YELLOW}=== Negative Test: Large Payload ===${NC}"
    echo "Sending traces with extremely large attributes"

    # Generate a very long string
    LONG_VALUE=$(printf 'x%.0s' {1..10000})

    $OTELGEN_BINARY traces single \
        --otel-exporter-otlp-endpoint $URPO_GRPC_ENDPOINT \
        --protocol grpc \
        --insecure \
        --duration 5 \
        --rate 10 \
        --otel-attributes service.name=negative-test-large \
        --otel-attributes huge.attribute="$LONG_VALUE" \
        --otel-attributes test.type=large_payload

    echo -e "${GREEN}✓ Large payload test completed${NC}"
}

# Negative test - Protocol switching
test_negative_protocol_switch() {
    echo -e "\n${YELLOW}=== Negative Test: Protocol Switching ===${NC}"
    echo "Rapidly switching between gRPC and HTTP protocols"

    for i in {1..10}; do
        echo "  Iteration $i: Sending via gRPC..."
        $OTELGEN_BINARY traces single \
            --otel-exporter-otlp-endpoint $URPO_GRPC_ENDPOINT \
            --protocol grpc \
            --insecure \
            --duration 2 \
            --rate 100 \
            --otel-attributes service.name=protocol-switch-grpc \
            --otel-attributes iteration=$i &

        echo "  Iteration $i: Sending via HTTP..."
        $OTELGEN_BINARY traces single \
            --otel-exporter-otlp-endpoint $URPO_HTTP_ENDPOINT \
            --protocol http/protobuf \
            --insecure \
            --duration 2 \
            --rate 100 \
            --otel-attributes service.name=protocol-switch-http \
            --otel-attributes iteration=$i &

        wait
    done

    echo -e "${GREEN}✓ Protocol switching test completed${NC}"
}

# Negative test - Connection interruption
test_negative_connection_interrupt() {
    echo -e "\n${YELLOW}=== Negative Test: Connection Interruption ===${NC}"
    echo "Starting long-running trace generation and interrupting it"

    # Start a long-running otelgen process in the background
    $OTELGEN_BINARY traces multi \
        --otel-exporter-otlp-endpoint $URPO_GRPC_ENDPOINT \
        --protocol grpc \
        --insecure \
        --duration 60 \
        --rate 500 \
        --otel-attributes service.name=interrupt-test \
        --otel-attributes test.type=connection_interrupt &

    OTELGEN_PID=$!

    # Let it run for 5 seconds
    sleep 5

    # Kill it abruptly
    echo "  Interrupting connection..."
    kill -9 $OTELGEN_PID 2>/dev/null || true

    sleep 2

    # Start sending again to test recovery
    echo "  Resuming transmission..."
    $OTELGEN_BINARY traces single \
        --otel-exporter-otlp-endpoint $URPO_GRPC_ENDPOINT \
        --protocol grpc \
        --insecure \
        --duration 5 \
        --rate 100 \
        --otel-attributes service.name=interrupt-test-recovery \
        --otel-attributes test.type=recovery_after_interrupt

    echo -e "${GREEN}✓ Connection interruption test completed${NC}"
}

# Chaos test - Multiple concurrent generators
test_chaos_concurrent() {
    echo -e "\n${YELLOW}=== Chaos Test: Multiple Concurrent Generators ===${NC}"
    echo "Starting 20 concurrent otelgen instances with different configurations"

    PIDS=()

    for i in {1..20}; do
        # Vary the rate and protocol
        RATE=$((RANDOM % 500 + 100))
        if [ $((i % 2)) -eq 0 ]; then
            PROTOCOL="grpc"
            ENDPOINT=$URPO_GRPC_ENDPOINT
        else
            PROTOCOL="http/protobuf"
            ENDPOINT=$URPO_HTTP_ENDPOINT
        fi

        $OTELGEN_BINARY traces multi \
            --otel-exporter-otlp-endpoint $ENDPOINT \
            --protocol $PROTOCOL \
            --insecure \
            --duration 30 \
            --rate $RATE \
            --otel-attributes service.name=chaos-service-$i \
            --otel-attributes test.type=chaos_concurrent \
            --otel-attributes generator.id=$i &

        PIDS+=($!)
        echo "  Started generator $i (rate: $RATE, protocol: $PROTOCOL)"
    done

    echo "  Waiting for all generators to complete..."
    for pid in ${PIDS[@]}; do
        wait $pid
    done

    echo -e "${GREEN}✓ Chaos concurrent test completed${NC}"
}

# Memory leak test - Long running with steady load
test_memory_leak() {
    echo -e "\n${YELLOW}=== Memory Leak Test: Long Running ===${NC}"
    echo "Running steady load for 5 minutes to check for memory leaks"
    echo "Monitor Urpo's memory usage during this test"

    $OTELGEN_BINARY traces multi \
        --otel-exporter-otlp-endpoint $URPO_GRPC_ENDPOINT \
        --protocol grpc \
        --insecure \
        --duration 300 \
        --rate 200 \
        --otel-attributes service.name=memory-leak-test \
        --otel-attributes environment=stress-test \
        --otel-attributes test.type=memory_leak

    echo -e "${GREEN}✓ Memory leak test completed${NC}"
}

# Main menu
show_menu() {
    echo -e "\n${BLUE}===== URPO OTEL Stress Testing Suite =====${NC}"
    echo "1) Run all tests (except memory leak)"
    echo "2) Performance - Normal load (3K traces)"
    echo "3) Performance - High load (60K traces)"
    echo "4) Stress - Burst traffic (100K traces)"
    echo "5) Stress - Sustained extreme load (600K traces)"
    echo "6) Negative - Large payload"
    echo "7) Negative - Protocol switching"
    echo "8) Negative - Connection interruption"
    echo "9) Chaos - Multiple concurrent generators"
    echo "10) Memory leak test (5 minutes)"
    echo "0) Exit"
    echo -e "${BLUE}========================================${NC}"
}

# Run all tests
run_all_tests() {
    echo -e "${BLUE}Running all stress tests (except memory leak)...${NC}"
    test_performance_normal
    sleep 5
    test_performance_high
    sleep 5
    test_stress_burst
    sleep 5
    test_negative_large_payload
    sleep 5
    test_negative_protocol_switch
    sleep 5
    test_negative_connection_interrupt
    sleep 5
    test_chaos_concurrent
    echo -e "\n${GREEN}All tests completed!${NC}"
    echo -e "${YELLOW}Note: Memory leak test skipped (run separately with option 10)${NC}"
}

# Main execution
main() {
    check_otelgen
    check_urpo

    if [ "$1" == "--all" ]; then
        run_all_tests
        exit 0
    fi

    while true; do
        show_menu
        read -p "Select test to run: " choice

        case $choice in
            1) run_all_tests ;;
            2) test_performance_normal ;;
            3) test_performance_high ;;
            4) test_stress_burst ;;
            5) test_stress_sustained ;;
            6) test_negative_large_payload ;;
            7) test_negative_protocol_switch ;;
            8) test_negative_connection_interrupt ;;
            9) test_chaos_concurrent ;;
            10) test_memory_leak ;;
            0) echo "Exiting..."; exit 0 ;;
            *) echo -e "${RED}Invalid option${NC}" ;;
        esac

        echo -e "\nPress Enter to continue..."
        read
    done
}

# Run main function
main "$@"
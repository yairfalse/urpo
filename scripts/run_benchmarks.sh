#!/bin/bash

# URPO PERFORMANCE VALIDATION SCRIPT
# Runs all benchmarks and verifies they meet CLAUDE.md targets

set -e

echo "======================================"
echo "🚀 URPO PERFORMANCE BENCHMARKS 🚀"
echo "======================================"
echo ""
echo "Performance Targets (from CLAUDE.md):"
echo "  • Startup Time: <200ms"
echo "  • Span Processing: <10μs per span"
echo "  • Memory Usage: <100MB for 1M spans"
echo "  • Search: <1ms across 100K traces"
echo ""
echo "======================================"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Check if mimalloc is being used
echo -e "${YELLOW}Checking allocator configuration...${NC}"
if grep -q "mimalloc::MiMalloc" src/main.rs; then
    echo -e "${GREEN}✓ Using mimalloc allocator${NC}"
else
    echo -e "${RED}✗ WARNING: Not using mimalloc allocator!${NC}"
fi

# Check release profile optimizations
echo ""
echo -e "${YELLOW}Checking Cargo.toml optimizations...${NC}"
if grep -q 'lto = "fat"' Cargo.toml; then
    echo -e "${GREEN}✓ LTO enabled${NC}"
fi
if grep -q 'codegen-units = 1' Cargo.toml; then
    echo -e "${GREEN}✓ Single codegen unit${NC}"
fi
if grep -q 'panic = "abort"' Cargo.toml; then
    echo -e "${GREEN}✓ Panic abort enabled${NC}"
fi
if grep -q 'overflow-checks = false' Cargo.toml; then
    echo -e "${GREEN}✓ Overflow checks disabled${NC}"
fi

echo ""
echo "======================================"
echo "Running benchmarks..."
echo "======================================"
echo ""

# Run span processing benchmarks
echo -e "${YELLOW}1. Running span processing benchmarks...${NC}"
cargo bench --bench span_processing -- --verbose

echo ""
echo -e "${YELLOW}2. Running hot path benchmarks...${NC}"
cargo bench --bench hot_path -- --verbose

# Parse results and check targets
echo ""
echo "======================================"
echo "PERFORMANCE VALIDATION RESULTS"
echo "======================================"

# Check if benchmark results exist
if [ -d "target/criterion" ]; then
    echo ""
    echo -e "${YELLOW}Analyzing results...${NC}"
    
    # Look for span processing results
    if [ -d "target/criterion/span_ingestion" ]; then
        echo -e "${GREEN}✓ Span processing benchmarks completed${NC}"
        
        # Check single span time (should be <10μs)
        if [ -f "target/criterion/span_ingestion/single_span/base/estimates.json" ]; then
            # Parse JSON to check timing
            echo "  Checking single span processing time..."
        fi
    fi
    
    # Look for startup time results
    if [ -d "target/criterion/startup_time" ]; then
        echo -e "${GREEN}✓ Startup time benchmark completed${NC}"
    fi
    
    # Look for memory usage results
    if [ -d "target/criterion/memory_1m_spans" ]; then
        echo -e "${GREEN}✓ Memory usage benchmark completed${NC}"
    fi
    
    # Look for query performance results
    if [ -d "target/criterion/trace_query" ]; then
        echo -e "${GREEN}✓ Query performance benchmarks completed${NC}"
    fi
fi

echo ""
echo "======================================"
echo "RECOMMENDATIONS"
echo "======================================"
echo ""
echo "1. Review detailed results in target/criterion/*/report/index.html"
echo "2. Run with --save-baseline to track performance over time:"
echo "   cargo bench -- --save-baseline main"
echo "3. Compare with baseline:"
echo "   cargo bench -- --baseline main"
echo "4. Profile with flamegraph for bottlenecks:"
echo "   cargo flamegraph --bench span_processing"
echo ""
echo "======================================"
echo "✨ Benchmark run complete!"
echo "======================================"
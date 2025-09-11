#!/bin/bash

# Script to find and categorize unwrap() calls in Urpo codebase
# Following CLAUDE.md performance requirements

echo "======================================"
echo "URPO: Finding unwrap() violations"
echo "======================================"
echo ""

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Count total unwraps
TOTAL=$(grep -r "\.unwrap()" src/ src-tauri/src/ --include="*.rs" | wc -l)
echo -e "${YELLOW}Total unwrap() calls found: $TOTAL${NC}"
echo ""

# Find unwraps in hot paths (CRITICAL)
echo -e "${RED}CRITICAL - unwrap() in hot paths:${NC}"
grep -r "\.unwrap()" src/ --include="*.rs" | grep -E "(ingest|store|process|handle|receive|parse_span|convert_span)" | grep -v "test" | grep -v "mod tests"
echo ""

# Find unwraps in Tauri commands (HIGH)
echo -e "${YELLOW}HIGH - unwrap() in Tauri commands:${NC}"
grep -n "\.unwrap()" src-tauri/src/main.rs | grep -v "test"
echo ""

# Find unwraps in storage layer (HIGH)
echo -e "${YELLOW}HIGH - unwrap() in storage:${NC}"
grep -r "\.unwrap()" src/storage/ --include="*.rs" | grep -v "test" | grep -v "mod tests" | head -10
echo ""

# Find unwraps in receiver (HIGH)
echo -e "${YELLOW}HIGH - unwrap() in receiver:${NC}"
grep -r "\.unwrap()" src/receiver/ --include="*.rs" | grep -v "test"
echo ""

# Find unwraps in API handlers (MEDIUM)
echo -e "${GREEN}MEDIUM - unwrap() in API:${NC}"
grep -r "\.unwrap()" src/api/ --include="*.rs" | grep -v "test" | head -5
echo ""

# Summary
echo "======================================"
echo "RECOMMENDATIONS (per CLAUDE.md):"
echo "1. Replace unwrap() in hot paths with:"
echo "   - unwrap_or_default() for non-critical defaults"
echo "   - map_err() with proper error propagation"
echo "   - unsafe { value.unwrap_unchecked() } if 100% certain (benchmark first)"
echo ""
echo "2. Use expect() only for:"
echo "   - Static configuration that cannot fail"
echo "   - Compile-time constants"
echo ""
echo "3. In hot paths, prefer:"
echo "   - Pre-validation to avoid runtime checks"
echo "   - Batch error handling"
echo "   - Arena allocation for temporary errors"
echo "======================================"
#!/bin/bash

# URPO Data Verification Script
# Checks if Urpo is properly receiving and processing OTEL data

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
URPO_API_ENDPOINT="${URPO_API_ENDPOINT:-http://localhost:8080}"

# Check if Urpo API is accessible
check_urpo_api() {
    echo -e "${BLUE}Checking Urpo API health...${NC}"
    if ! curl -s -f "${URPO_API_ENDPOINT}/health" > /dev/null 2>&1; then
        echo -e "${RED}Error: Cannot reach Urpo API at ${URPO_API_ENDPOINT}${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓ Urpo API is healthy${NC}"
}

# Get service statistics
get_service_stats() {
    echo -e "\n${YELLOW}=== Service Statistics ===${NC}"

    # Fetch services (assuming there's an API endpoint)
    RESPONSE=$(curl -s "${URPO_API_ENDPOINT}/api/services" 2>/dev/null || echo "{}")

    if [ "$RESPONSE" != "{}" ]; then
        echo "$RESPONSE" | jq -r '.services[] | "Service: \(.name), Traces: \(.trace_count), Spans: \(.span_count)"' 2>/dev/null || echo "Unable to parse service data"
    else
        echo "No service data available (API endpoint might not exist yet)"
    fi
}

# Get trace statistics
get_trace_stats() {
    echo -e "\n${YELLOW}=== Trace Statistics ===${NC}"

    # Try to get trace count
    RESPONSE=$(curl -s "${URPO_API_ENDPOINT}/api/traces/stats" 2>/dev/null || echo "{}")

    if [ "$RESPONSE" != "{}" ]; then
        echo "$RESPONSE" | jq -r '"Total Traces: \(.total_traces)\nError Traces: \(.error_traces)\nAvg Duration: \(.avg_duration_ms)ms"' 2>/dev/null || echo "Unable to parse trace data"
    else
        echo "No trace statistics available"
    fi
}

# Check memory usage
check_memory_usage() {
    echo -e "\n${YELLOW}=== Memory Usage ===${NC}"

    # Get Urpo process PID
    URPO_PID=$(pgrep -f "urpo" | head -1)

    if [ -z "$URPO_PID" ]; then
        echo -e "${RED}Urpo process not found${NC}"
        return
    fi

    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        MEM_INFO=$(ps -o pid,rss,vsz,comm -p $URPO_PID | tail -1)
        RSS=$(echo $MEM_INFO | awk '{print $2}')
        VSZ=$(echo $MEM_INFO | awk '{print $3}')
        RSS_MB=$((RSS / 1024))
        VSZ_MB=$((VSZ / 1024))
        echo "PID: $URPO_PID"
        echo "RSS (Resident Set Size): ${RSS_MB} MB"
        echo "VSZ (Virtual Size): ${VSZ_MB} MB"
    else
        # Linux
        MEM_INFO=$(ps aux | grep -E "^[^ ]*[ ]*$URPO_PID" | head -1)
        RSS=$(echo $MEM_INFO | awk '{print $6}')
        VSZ=$(echo $MEM_INFO | awk '{print $5}')
        RSS_MB=$((RSS / 1024))
        VSZ_MB=$((VSZ / 1024))
        echo "PID: $URPO_PID"
        echo "RSS (Resident Set Size): ${RSS_MB} MB"
        echo "VSZ (Virtual Size): ${VSZ_MB} MB"
    fi

    # Check against CLAUDE.md requirements (50MB for efficient operation)
    if [ "$RSS_MB" -gt 100 ]; then
        echo -e "${YELLOW}Warning: Memory usage exceeds 100MB (target: <50MB for normal operation)${NC}"
    else
        echo -e "${GREEN}✓ Memory usage within acceptable limits${NC}"
    fi
}

# Check CPU usage
check_cpu_usage() {
    echo -e "\n${YELLOW}=== CPU Usage ===${NC}"

    URPO_PID=$(pgrep -f "urpo" | head -1)

    if [ -z "$URPO_PID" ]; then
        echo -e "${RED}Urpo process not found${NC}"
        return
    fi

    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        CPU=$(ps -o pid,%cpu -p $URPO_PID | tail -1 | awk '{print $2}')
    else
        # Linux
        CPU=$(ps aux | grep -E "^[^ ]*[ ]*$URPO_PID" | head -1 | awk '{print $3}')
    fi

    echo "CPU Usage: ${CPU}%"

    # Check if CPU is reasonable
    CPU_INT=${CPU%.*}
    if [ "$CPU_INT" -gt 80 ]; then
        echo -e "${YELLOW}Warning: High CPU usage detected${NC}"
    else
        echo -e "${GREEN}✓ CPU usage normal${NC}"
    fi
}

# Monitor real-time metrics
monitor_realtime() {
    echo -e "\n${YELLOW}=== Real-time Monitoring (Press Ctrl+C to stop) ===${NC}"

    while true; do
        clear
        echo -e "${BLUE}===== URPO Real-time Stats =====${NC}"
        echo "Timestamp: $(date)"

        # Memory
        URPO_PID=$(pgrep -f "urpo" | head -1)
        if [ ! -z "$URPO_PID" ]; then
            if [[ "$OSTYPE" == "darwin"* ]]; then
                MEM_INFO=$(ps -o rss -p $URPO_PID | tail -1)
                RSS_MB=$((MEM_INFO / 1024))
                CPU=$(ps -o %cpu -p $URPO_PID | tail -1)
            else
                MEM_INFO=$(ps aux | grep -E "^[^ ]*[ ]*$URPO_PID" | head -1)
                RSS=$(echo $MEM_INFO | awk '{print $6}')
                RSS_MB=$((RSS / 1024))
                CPU=$(echo $MEM_INFO | awk '{print $3}')
            fi

            echo -e "\nMemory: ${RSS_MB} MB"
            echo "CPU: ${CPU}%"
        fi

        # Try to get trace count from API
        TRACE_COUNT=$(curl -s "${URPO_API_ENDPOINT}/api/traces/count" 2>/dev/null || echo "N/A")
        echo -e "\nTotal Traces: $TRACE_COUNT"

        sleep 2
    done
}

# Test specific service data
test_service_data() {
    SERVICE_NAME="$1"
    echo -e "\n${YELLOW}=== Testing Service: $SERVICE_NAME ===${NC}"

    # Check if service exists in Urpo
    RESPONSE=$(curl -s "${URPO_API_ENDPOINT}/api/services/${SERVICE_NAME}" 2>/dev/null || echo "{}")

    if [ "$RESPONSE" != "{}" ]; then
        echo "$RESPONSE" | jq . 2>/dev/null || echo "$RESPONSE"
    else
        echo "Service data not available"
    fi
}

# Generate report
generate_report() {
    echo -e "\n${BLUE}===== URPO Verification Report =====${NC}"
    echo "Generated at: $(date)"
    echo "================================"

    check_urpo_api
    get_service_stats
    get_trace_stats
    check_memory_usage
    check_cpu_usage

    echo -e "\n${GREEN}===== Report Complete =====${NC}"
}

# Menu
show_menu() {
    echo -e "\n${BLUE}===== URPO Data Verification =====${NC}"
    echo "1) Full verification report"
    echo "2) Service statistics"
    echo "3) Trace statistics"
    echo "4) Memory usage check"
    echo "5) CPU usage check"
    echo "6) Real-time monitoring"
    echo "7) Test specific service"
    echo "0) Exit"
    echo -e "${BLUE}================================${NC}"
}

# Main
main() {
    if [ "$1" == "--report" ]; then
        generate_report
        exit 0
    fi

    if [ "$1" == "--monitor" ]; then
        monitor_realtime
        exit 0
    fi

    if [ "$1" == "--service" ] && [ ! -z "$2" ]; then
        check_urpo_api
        test_service_data "$2"
        exit 0
    fi

    while true; do
        show_menu
        read -p "Select option: " choice

        case $choice in
            1) generate_report ;;
            2) check_urpo_api; get_service_stats ;;
            3) check_urpo_api; get_trace_stats ;;
            4) check_memory_usage ;;
            5) check_cpu_usage ;;
            6) monitor_realtime ;;
            7)
                read -p "Enter service name: " service_name
                check_urpo_api
                test_service_data "$service_name"
                ;;
            0) echo "Exiting..."; exit 0 ;;
            *) echo -e "${RED}Invalid option${NC}" ;;
        esac

        echo -e "\nPress Enter to continue..."
        read
    done
}

main "$@"
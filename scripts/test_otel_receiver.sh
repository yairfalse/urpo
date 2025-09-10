#!/bin/bash

# Test script to verify OTEL receiver is working

echo "Testing OTEL receiver on port 4317 (gRPC)..."

# First, check if the port is open
nc -zv localhost 4317 2>/dev/null
if [ $? -eq 0 ]; then
    echo "✓ Port 4317 is open"
else
    echo "✗ Port 4317 is not open - receiver may not be running"
    echo "Start the Urpo receiver first with: cargo run"
    exit 1
fi

# Try sending a test trace using curl (HTTP endpoint)
echo ""
echo "Testing OTEL receiver on port 4318 (HTTP)..."

# OTLP JSON payload
TRACE_JSON='{
  "resourceSpans": [{
    "resource": {
      "attributes": [{
        "key": "service.name",
        "value": {"stringValue": "test-service"}
      }]
    },
    "scopeSpans": [{
      "scope": {
        "name": "test-scope"
      },
      "spans": [{
        "traceId": "5B8EFFF798038103D269B633813FC60C",
        "spanId": "EEE19B7EC3C1B174",
        "parentSpanId": "EEE19B7EC3C1B173",
        "name": "test-span",
        "startTimeUnixNano": "'$(date +%s)000000000'",
        "endTimeUnixNano": "'$(date +%s)000000100'",
        "kind": 2,
        "attributes": [{
          "key": "http.method",
          "value": {"stringValue": "GET"}
        }]
      }]
    }]
  }]
}'

# Send to HTTP endpoint
RESPONSE=$(curl -s -X POST http://localhost:4318/v1/traces \
  -H "Content-Type: application/json" \
  -d "$TRACE_JSON" \
  -w "\nHTTP_STATUS:%{http_code}" 2>/dev/null)

HTTP_STATUS=$(echo "$RESPONSE" | grep "HTTP_STATUS:" | cut -d: -f2)

if [ "$HTTP_STATUS" = "200" ]; then
    echo "✓ Successfully sent test trace via HTTP"
else
    echo "✗ Failed to send trace via HTTP (status: $HTTP_STATUS)"
    echo "Note: HTTP receiver may not be implemented yet"
fi

echo ""
echo "To fully test the gRPC endpoint, use a tool like grpcurl or the OTEL collector"
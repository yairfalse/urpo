#!/bin/bash

# Test script to verify HTTP OTLP receiver on port 4318

echo "🚀 Testing Urpo HTTP OTLP Receiver"
echo "====================================="

# Check if port 4318 is available
if nc -z localhost 4318 2>/dev/null; then
    echo "✓ Port 4318 is open - Urpo HTTP receiver is running"
else
    echo "✗ Port 4318 is not open"
    echo "  Start Urpo first with: cargo run"
    echo "  Or run the GUI: cd src-tauri && cargo tauri dev"
    exit 1
fi

echo ""
echo "1. Testing health endpoint..."

# Test health endpoint
HEALTH_RESPONSE=$(curl -s http://localhost:4318/health)
if [ $? -eq 0 ]; then
    echo "✓ Health endpoint responding:"
    echo "$HEALTH_RESPONSE" | jq . 2>/dev/null || echo "$HEALTH_RESPONSE"
else
    echo "✗ Health endpoint failed"
fi

echo ""
echo "2. Testing root endpoint..."

# Test root endpoint
ROOT_RESPONSE=$(curl -s http://localhost:4318/)
if [ $? -eq 0 ]; then
    echo "✓ Root endpoint responding:"
    echo "$ROOT_RESPONSE" | jq . 2>/dev/null || echo "$ROOT_RESPONSE"
else
    echo "✗ Root endpoint failed"
fi

echo ""
echo "3. Sending test OTLP traces..."

# Create a test trace
TRACE_ID=$(openssl rand -hex 16)
SPAN_ID=$(openssl rand -hex 8)
TIMESTAMP=$(date +%s)000000000  # nanoseconds

TRACE_JSON='{
  "resourceSpans": [{
    "resource": {
      "attributes": [{
        "key": "service.name",
        "value": {"stringValue": "test-http-service"}
      }, {
        "key": "service.version", 
        "value": {"stringValue": "1.0.0"}
      }]
    },
    "scopeSpans": [{
      "scope": {
        "name": "test-http-scope"
      },
      "spans": [{
        "traceId": "'$TRACE_ID'",
        "spanId": "'$SPAN_ID'",
        "name": "test-http-span",
        "startTimeUnixNano": "'$TIMESTAMP'",
        "endTimeUnixNano": "'$(($TIMESTAMP + 100000000))'",
        "kind": 2,
        "attributes": [{
          "key": "http.method",
          "value": {"stringValue": "GET"}
        }, {
          "key": "http.url",
          "value": {"stringValue": "/test"}
        }, {
          "key": "http.status_code",
          "value": {"intValue": 200}
        }]
      }]
    }]
  }]
}'

# Send the trace
HTTP_STATUS=$(curl -s -w "%{http_code}" -o /dev/null \
  -X POST http://localhost:4318/v1/traces \
  -H "Content-Type: application/json" \
  -d "$TRACE_JSON")

if [ "$HTTP_STATUS" = "200" ]; then
    echo "✓ Successfully sent test trace (HTTP $HTTP_STATUS)"
    echo "  Trace ID: $TRACE_ID"
    echo "  Span ID: $SPAN_ID"
else
    echo "✗ Failed to send test trace (HTTP $HTTP_STATUS)"
fi

echo ""
echo "4. Testing with invalid JSON..."

# Test error handling with invalid JSON
ERROR_STATUS=$(curl -s -w "%{http_code}" -o /dev/null \
  -X POST http://localhost:4318/v1/traces \
  -H "Content-Type: application/json" \
  -d '{"invalid": "json"')

if [ "$ERROR_STATUS" = "400" ]; then
    echo "✓ Correctly rejected invalid JSON (HTTP $ERROR_STATUS)"
else
    echo "? Unexpected response to invalid JSON (HTTP $ERROR_STATUS)"
fi

echo ""
echo "🎉 HTTP OTLP Receiver Test Complete!"
echo ""
echo "Next steps:"
echo "- View traces in Urpo UI"
echo "- Send more traces: cargo run --example send_http_traces"
echo "- Check gRPC receiver: cargo run --example send_otel_traces"
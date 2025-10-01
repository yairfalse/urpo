#!/bin/bash
# Test script for OTLP/gRPC receiver on port 4317

set -e

echo "🧪 Testing Urpo OTLP/gRPC Receiver"
echo "===================================="
echo ""

# Check if grpcurl is installed
if ! command -v grpcurl &> /dev/null; then
    echo "❌ grpcurl not found. Installing via brew..."
    brew install grpcurl
fi

echo "📡 Testing gRPC receiver on localhost:4317"
echo ""

# Test 1: List available services
echo "1️⃣  Testing service discovery..."
if grpcurl -plaintext localhost:4317 list; then
    echo "✅ Service discovery successful"
else
    echo "❌ Service discovery failed - is Urpo running?"
    exit 1
fi
echo ""

# Test 2: Send a sample trace via gRPC
echo "2️⃣  Sending sample OTLP trace..."
cat <<'EOF' > /tmp/urpo-test-trace.json
{
  "resourceSpans": [{
    "resource": {
      "attributes": [{
        "key": "service.name",
        "value": {"stringValue": "test-service"}
      }]
    },
    "scopeSpans": [{
      "scope": {
        "name": "test-instrumentation",
        "version": "1.0.0"
      },
      "spans": [{
        "traceId": "0102030405060708090a0b0c0d0e0f10",
        "spanId": "0102030405060708",
        "name": "test-operation",
        "kind": 2,
        "startTimeUnixNano": "1700000000000000000",
        "endTimeUnixNano": "1700000001000000000",
        "attributes": [{
          "key": "http.method",
          "value": {"stringValue": "GET"}
        }]
      }]
    }]
  }]
}
EOF

if grpcurl -plaintext \
    -d @ \
    localhost:4317 \
    opentelemetry.proto.collector.trace.v1.TraceService/Export \
    < /tmp/urpo-test-trace.json; then
    echo "✅ Trace sent successfully via gRPC"
else
    echo "❌ Failed to send trace via gRPC"
    exit 1
fi

rm /tmp/urpo-test-trace.json
echo ""
echo "✅ All gRPC tests passed!"
echo "👀 Check the Urpo UI to see the trace (press 's' for settings)"

#!/bin/bash
# Test script for OTLP/HTTP receiver on port 4318

set -e

echo "🧪 Testing Urpo OTLP/HTTP Receiver"
echo "===================================="
echo ""

echo "📡 Testing HTTP receiver on localhost:4318"
echo ""

# Test 1: Health check
echo "1️⃣  Testing health endpoint..."
if curl -s http://localhost:4318/health | jq .; then
    echo "✅ Health check successful"
else
    echo "❌ Health check failed - is Urpo running?"
    exit 1
fi
echo ""

# Test 2: Root endpoint
echo "2️⃣  Testing root endpoint..."
if curl -s http://localhost:4318/ | jq .; then
    echo "✅ Root endpoint successful"
else
    echo "❌ Root endpoint failed"
    exit 1
fi
echo ""

# Test 3: Send a sample trace via HTTP/JSON
echo "3️⃣  Sending sample OTLP trace (JSON)..."
cat <<'EOF' > /tmp/urpo-test-http-trace.json
{
  "resourceSpans": [{
    "resource": {
      "attributes": [{
        "key": "service.name",
        "value": {"stringValue": "http-test-service"}
      }]
    },
    "scopeSpans": [{
      "scope": {
        "name": "http-test-instrumentation",
        "version": "1.0.0"
      },
      "spans": [{
        "traceId": "aabbccddeeff00112233445566778899",
        "spanId": "1122334455667788",
        "name": "http-test-operation",
        "kind": 2,
        "startTimeUnixNano": "1700000000000000000",
        "endTimeUnixNano": "1700000002000000000",
        "attributes": [{
          "key": "http.method",
          "value": {"stringValue": "POST"}
        }, {
          "key": "http.url",
          "value": {"stringValue": "/api/test"}
        }]
      }]
    }]
  }]
}
EOF

if curl -X POST \
    -H "Content-Type: application/json" \
    -d @/tmp/urpo-test-http-trace.json \
    http://localhost:4318/v1/traces | jq .; then
    echo "✅ Trace sent successfully via HTTP/JSON"
else
    echo "❌ Failed to send trace via HTTP"
    exit 1
fi

rm /tmp/urpo-test-http-trace.json
echo ""

# Test 4: Send protobuf trace (if protoc is available)
echo "4️⃣  Testing protobuf endpoint..."
echo "ℹ️  Skipping protobuf test (requires protoc, use JSON for now)"
echo ""

echo "✅ All HTTP tests passed!"
echo "👀 Check the Urpo UI to see the traces (press 's' for settings)"

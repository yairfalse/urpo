#!/bin/bash

# Send some OTEL traces to Urpo

echo "🚀 Sending OTEL traces to Urpo..."

# Function to generate random trace and span IDs
random_hex() {
    openssl rand -hex $1
}

# Send a few different traces
for i in {1..5}; do
    TRACE_ID=$(random_hex 16)
    SPAN_ID=$(random_hex 8)
    START_TIME=$(date +%s%09d)
    sleep 0.1
    END_TIME=$(date +%s%09d)
    
    # Different services
    SERVICES=("frontend" "api-gateway" "auth-service" "database" "cache")
    SERVICE=${SERVICES[$((i-1))]}
    
    # Different operations
    OPS=("GET /users" "authenticate" "db.query" "cache.get" "render")
    OP=${OPS[$((i-1))]}
    
    # Some have errors
    STATUS=""
    if [ $i -eq 2 ] || [ $i -eq 4 ]; then
        STATUS=',"status":{"code":2,"message":"Internal error"}'
    fi
    
    curl -X POST http://localhost:4318/v1/traces \
      -H 'Content-Type: application/json' \
      -d '{
        "resourceSpans": [{
          "resource": {
            "attributes": [
              {"key": "service.name", "value": {"stringValue": "'$SERVICE'"}}
            ]
          },
          "scopeSpans": [{
            "spans": [{
              "traceId": "'$TRACE_ID'",
              "spanId": "'$SPAN_ID'",
              "name": "'$OP'",
              "startTimeUnixNano": "'$START_TIME'",
              "endTimeUnixNano": "'$END_TIME'",
              "attributes": [
                {"key": "http.method", "value": {"stringValue": "GET"}},
                {"key": "http.status_code", "value": {"intValue": "200"}}
              ]'$STATUS'
            }]
          }]
        }]
      }' 2>/dev/null
    
    echo "✓ Sent trace from $SERVICE"
    sleep 0.5
done

echo "✅ Done! Check Urpo's Traces tab"
#!/bin/bash

echo "Sending multiple test traces to Urpo OTEL receiver..."

# Helper function to generate trace ID
generate_trace_id() {
    echo $(openssl rand -hex 16)
}

# Helper function to generate span ID
generate_span_id() {
    echo $(openssl rand -hex 8)
}

# Send traces for different services
services=("frontend-service" "api-gateway" "auth-service" "user-service" "payment-service" "order-service")
operations=("GET /api/users" "POST /api/orders" "GET /api/products" "PUT /api/profile" "DELETE /api/cart")

for i in {1..20}; do
    SERVICE=${services[$((RANDOM % ${#services[@]}))]}
    OPERATION=${operations[$((RANDOM % ${#operations[@]}))]}
    TRACE_ID=$(generate_trace_id)
    SPAN_ID=$(generate_span_id)
    DURATION=$((50 + RANDOM % 450))  # Random duration 50-500ms
    ERROR_STATUS=$((RANDOM % 10))  # 10% error rate

    if [ $ERROR_STATUS -eq 0 ]; then
        STATUS_CODE=2  # Error
        HTTP_STATUS=500
    else
        STATUS_CODE=1  # OK
        HTTP_STATUS=200
    fi

    START_TIME=$(date +%s)
    END_TIME=$((START_TIME + DURATION / 1000))

    curl -X POST http://localhost:4318/v1/traces \
      -H "Content-Type: application/json" \
      -s \
      -d "{
        \"resourceSpans\": [{
          \"resource\": {
            \"attributes\": [{
              \"key\": \"service.name\",
              \"value\": { \"stringValue\": \"$SERVICE\" }
            }]
          },
          \"scopeSpans\": [{
            \"spans\": [{
              \"traceId\": \"$TRACE_ID\",
              \"spanId\": \"$SPAN_ID\",
              \"name\": \"$OPERATION\",
              \"kind\": 2,
              \"startTimeUnixNano\": \"${START_TIME}000000000\",
              \"endTimeUnixNano\": \"${END_TIME}000000000\",
              \"attributes\": [
                { \"key\": \"http.method\", \"value\": { \"stringValue\": \"${OPERATION%% *}\" }},
                { \"key\": \"http.url\", \"value\": { \"stringValue\": \"${OPERATION#* }\" }},
                { \"key\": \"http.status_code\", \"value\": { \"intValue\": \"$HTTP_STATUS\" }}
              ],
              \"status\": { \"code\": $STATUS_CODE }
            }]
          }]
        }]
      }"

    echo "✓ Sent trace for $SERVICE - $OPERATION (${DURATION}ms)"
    sleep 0.1
done

echo "✅ Sent 20 test traces! Check the UI to see the data."
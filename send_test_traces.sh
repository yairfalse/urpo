#!/bin/bash

# Send test OTEL trace data to Urpo
echo "Sending test traces to Urpo OTEL receiver on port 4318..."

# Sample OTLP trace in JSON format
curl -X POST http://localhost:4318/v1/traces \
  -H "Content-Type: application/json" \
  -d '{
    "resourceSpans": [{
      "resource": {
        "attributes": [{
          "key": "service.name",
          "value": { "stringValue": "frontend-service" }
        }]
      },
      "scopeSpans": [{
        "spans": [{
          "traceId": "5b8efff798038103d269b633813fc60c",
          "spanId": "eee19b7ec3c1b173",
          "name": "HTTP GET /api/users",
          "kind": 2,
          "startTimeUnixNano": "'$(date +%s)'000000000",
          "endTimeUnixNano": "'$(date +%s)'000000100",
          "attributes": [
            { "key": "http.method", "value": { "stringValue": "GET" }},
            { "key": "http.url", "value": { "stringValue": "/api/users" }},
            { "key": "http.status_code", "value": { "intValue": "200" }}
          ],
          "status": { "code": 1 }
        }]
      }]
    }]
  }'

echo "Test trace sent! Check the UI to see the data."
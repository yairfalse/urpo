#!/bin/bash

# Test script to send traces to Urpo GUI (ports 4327/4328)
# This will populate the empty tabs in the Tauri application

echo "🚀 Sending 5 test traces to Urpo (HTTP port 4328)..."
echo ""

for i in {1..5}; do
  SERVICE_NAME="service-$i"
  TRACE_ID=$(printf '%032x' $((RANDOM * RANDOM)))
  SPAN_ID=$(printf '%016x' $((RANDOM)))

  echo "📊 Sending trace $i: $SERVICE_NAME (trace: ${TRACE_ID:0:16}...)"

  curl -X POST \
    -H "Content-Type: application/json" \
    -d "{\"resourceSpans\":[{\"resource\":{\"attributes\":[{\"key\":\"service.name\",\"value\":{\"stringValue\":\"$SERVICE_NAME\"}}]},\"scopeSpans\":[{\"spans\":[{\"traceId\":\"$TRACE_ID\",\"spanId\":\"$SPAN_ID\",\"name\":\"operation-$i\",\"startTimeUnixNano\":\"$(date +%s)000000000\",\"endTimeUnixNano\":\"$(date +%s)000000000\"}]}]}]}" \
    http://localhost:4328/v1/traces \
    2>&1 | grep -q "Empty reply" && echo "   ✅ Sent successfully" || echo "   ❌ Failed to send"

  sleep 0.5
done

echo ""
echo "✅ Done! Check the Urpo GUI:"
echo "   • Tab 1 (Dashboard): Should show 5 services and 5 traces"
echo "   • Tab 2 (Services): Should show service-1 through service-5"
echo "   • Tab 3 (Traces): Should show 5 traces"
echo "   • Tab 4 (Health): Should show service health metrics"
echo ""
echo "If tabs are still empty, check the DevTools console (Cmd+Option+I)"

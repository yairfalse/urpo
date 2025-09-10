#!/bin/bash

# Test script for Urpo OTEL integration

echo "=== Urpo OTEL Integration Test ==="
echo

# Start Urpo in headless mode
echo "1. Starting Urpo in headless mode..."
cargo run -- --debug start --headless &
URPO_PID=$!

# Wait for Urpo to start
sleep 3

# Send test trace
echo "2. Sending test trace..."
cargo run --example send_test_trace

# Check if Urpo is still running
if ps -p $URPO_PID > /dev/null; then
    echo "✅ Urpo is running and accepting OTEL data!"
else
    echo "❌ Urpo crashed!"
    exit 1
fi

# Send continuous traces for 5 seconds
echo "3. Sending continuous traces for 5 seconds..."
timeout 5 cargo run --example continuous_sender 2>/dev/null || true

# Kill Urpo
echo "4. Stopping Urpo..."
kill $URPO_PID 2>/dev/null

echo
echo "=== Test Complete ==="
echo "To test with UI, run:"
echo "  cargo run start        # In terminal 1"
echo "  cargo run --example continuous_sender  # In terminal 2"
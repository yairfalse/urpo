#!/bin/bash

# Quick script to start receiver and send test data
echo "Starting OTLP receiver on port 4317..."

# Use curl to call the Tauri backend to start the receiver
# This assumes Tauri is running and the command is exposed

# Check if otelgen is installed
if ! command -v otelgen &> /dev/null; then
    echo "otelgen not found. Installing..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        brew install krzko/tap/otelgen
    else
        echo "Please install otelgen from: https://github.com/krzko/otelgen/releases"
        exit 1
    fi
fi

echo "Waiting for receiver to start..."
sleep 2

# Check if port 4317 is open
if nc -z localhost 4317 2>/dev/null; then
    echo "✓ OTLP receiver is running on port 4317"
else
    echo "Warning: Port 4317 not open. Starting receiver may have failed."
    echo "Try logging into the app first, then run this script again."
fi

echo ""
echo "Sending test traces..."
echo "1. Sending 50 traces from 'frontend' service..."

otelgen traces single \
    --otel-exporter-otlp-endpoint localhost:4317 \
    --protocol grpc \
    --insecure \
    --duration 10 \
    --rate 5 \
    --otel-attributes service.name=frontend \
    --otel-attributes environment=development \
    --otel-attributes version=1.0.0

echo ""
echo "2. Sending 100 traces from 'backend' service..."

otelgen traces multi \
    --otel-exporter-otlp-endpoint localhost:4317 \
    --protocol grpc \
    --insecure \
    --duration 10 \
    --rate 10 \
    --otel-attributes service.name=backend \
    --otel-attributes environment=development \
    --otel-attributes version=2.0.0

echo ""
echo "3. Sending error traces from 'payment' service..."

otelgen traces single \
    --otel-exporter-otlp-endpoint localhost:4317 \
    --protocol grpc \
    --insecure \
    --duration 5 \
    --rate 3 \
    --otel-attributes service.name=payment \
    --otel-attributes environment=production \
    --otel-attributes error=true

echo ""
echo "✓ Test data sent! Check Urpo UI for traces."
#!/bin/bash

# Urpo Build Script - Optimized for performance

set -e

echo "Building Urpo - High-Performance OTEL Trace Explorer"
echo "===================================================="

# Check dependencies
if ! command -v npm &> /dev/null; then
    echo "Error: npm is not installed. Please install Node.js first."
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed. Please install Rust first."
    exit 1
fi

# Install frontend dependencies
echo "Installing frontend dependencies..."
npm install

# Build the frontend with optimizations
echo "Building React frontend with virtualization support..."
npm run build

# Install Tauri CLI if not present
if ! command -v cargo-tauri &> /dev/null; then
    echo "Installing Tauri CLI..."
    cargo install tauri-cli
fi

# Build the Tauri app with release optimizations
echo "Building Tauri application..."
cargo tauri build

echo ""
echo "Build complete!"
echo ""
echo "Application location:"
echo "  target/release/urpo-gui"
echo ""
echo "To run in development mode:"
echo "  npm run tauri dev"
echo ""
echo "Performance targets:"
echo "  - Startup time: <200ms"
echo "  - Memory usage: <100MB for 1M spans"
echo "  - UI frame rate: 60fps"
#!/bin/bash

# Urpo Development Runner

set -e

echo "Starting Urpo in development mode"
echo "================================="

# Check if dependencies are installed
if ! [ -d "node_modules" ]; then
    echo "Installing dependencies..."
    npm install
fi

# Start the development server with hot reload
echo "Starting development server..."
echo "  - React frontend with hot module replacement"
echo "  - Rust backend with automatic recompilation"
echo "  - Performance monitoring enabled"
echo ""

# Set development environment variables for optimal performance
export RUST_LOG=urpo=debug,tauri=info
export TAURI_DEBUG=1

# Start Tauri in development mode
npm run tauri dev
# Urpo Architecture

## Project Structure

Urpo is a hybrid application that can run both as a terminal UI (TUI) application and as a desktop GUI application using Tauri.

### Directory Layout

```
urpo/
├── src/                    # Frontend (React/TypeScript)
│   ├── components/         # React components
│   ├── hooks/              # React hooks
│   ├── services/           # Frontend services
│   ├── types/              # TypeScript type definitions
│   ├── utils/              # Frontend utilities
│   ├── App.tsx             # Main React application
│   ├── main.tsx            # React entry point
│   └── index.css           # Global styles
│
├── src/                    # Backend Library (Rust)
│   ├── api/                # HTTP API endpoints
│   ├── cli/                # Command-line interface
│   ├── core/               # Core domain models and logic
│   ├── export/             # Trace export functionality
│   ├── monitoring/         # Health monitoring
│   ├── receiver/           # OTEL receivers (GRPC/HTTP)
│   ├── service_map/        # Service dependency mapping
│   ├── storage/            # Storage backends
│   │   ├── backend.rs      # Storage trait definition
│   │   ├── memory.rs       # In-memory storage
│   │   ├── manager.rs      # Storage coordination
│   │   └── types.rs        # Storage data types
│   ├── tui/                # Terminal UI (ratatui)
│   ├── lib.rs              # Library entry point
│   └── main.rs             # CLI binary entry point
│
├── src-tauri/              # Tauri Application
│   └── src/
│       └── main.rs         # Tauri backend entry point
│
├── tests/                  # Test files
├── scripts/                # Build and utility scripts
├── docs/                   # Documentation
└── examples/               # Example configurations

```

## Architecture Components

### 1. Core Library (`urpo_lib`)
- **Location**: Root `src/` directory (Rust files)
- **Purpose**: Shared business logic used by both TUI and GUI
- **Key Components**:
  - OTEL receivers for trace ingestion
  - Storage backends for trace data
  - Service health monitoring
  - Trace search and analysis

### 2. Terminal UI (TUI)
- **Location**: `src/tui/`
- **Purpose**: Interactive terminal interface using ratatui
- **Features**:
  - Real-time service health dashboard
  - Trace exploration
  - Span details viewer
  - Vim-like navigation

### 3. Desktop GUI
- **Frontend**: `src/` directory (React/TypeScript files)
- **Backend**: `src-tauri/`
- **Purpose**: Rich desktop application with web technologies
- **Features**:
  - Advanced visualizations
  - Service dependency graphs
  - Interactive trace timelines
  - Command palette

## Data Flow

```
OTEL Data Sources
      ↓
[GRPC:4317 / HTTP:4318]
      ↓
Receiver Module
      ↓
Storage Backend
      ↓
    ┌─┴─┐
    │   │
   TUI  GUI
```

## Storage Architecture

The storage system has been refactored into focused modules:

- **`backend.rs`**: Defines the `StorageBackend` trait
- **`types.rs`**: Common data structures
- **`memory.rs`**: In-memory storage implementation
- **`manager.rs`**: Storage coordination and management
- **`engine.rs`**: Persistent storage engine
- **`archive.rs`**: Long-term trace archival

## Building and Running

### Terminal UI
```bash
cargo run --bin urpo
```

### Desktop GUI
```bash
npm run tauri dev
```

### Library Usage
```rust
use urpo_lib::{Config, StorageManager};

let config = Config::default();
let storage = StorageManager::new(config)?;
```

## Performance Targets

- Startup time: <200ms
- Span processing: <10μs per span
- Memory usage: <100MB for 1M spans
- UI response: <16ms (60fps)

## Technology Stack

- **Backend**: Rust, Tokio, Tonic, OpenTelemetry
- **Terminal UI**: Ratatui, Crossterm
- **Desktop Frontend**: React, TypeScript, Tailwind CSS
- **Desktop Backend**: Tauri
- **Storage**: In-memory with optional persistence
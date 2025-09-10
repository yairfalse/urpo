# Urpo Project Structure

## Overview
This document describes the organized structure of the Urpo project after cleanup.

## Directory Structure

```
urpo/
├── src/                    # Mixed Rust backend + React frontend (to be separated)
│   ├── api/               # HTTP API for external tools
│   ├── cli/               # Command-line interface  
│   ├── core/              # Core domain models and logic
│   ├── export/            # Multi-format trace export
│   ├── monitoring/        # Health monitoring
│   ├── receiver/          # OTEL protocol receivers
│   ├── storage/           # Storage backends (needs refactoring)
│   ├── ui/                # Terminal UI components
│   ├── components/        # React frontend components
│   ├── services/          # Frontend services
│   ├── utils/             # Frontend utilities
│   ├── lib.rs            # Rust library entry point
│   ├── main.rs           # Rust binary entry point
│   └── App.tsx           # React app entry point
│
├── tests/                  # All test files (organized)
│   ├── archive_integration.rs
│   ├── config_test.rs
│   ├── integration_test.rs
│   ├── storage_integration_test.rs
│   ├── trace_exploration_test.rs
│   └── ui_non_blocking_test.rs
│
├── scripts/                # Shell scripts for testing
│   ├── test_http_receiver.sh
│   ├── test_integration.sh
│   └── test_otel_receiver.sh
│
├── examples/               # Usage examples and configs
│   ├── basic_usage.rs
│   ├── config_example.rs
│   ├── config.example.yaml
│   ├── demo_config.rs
│   └── send_*.rs          # Various trace sender examples
│
├── docs/                   # Historical documentation
│   ├── ITERATION_8_SUMMARY.md
│   ├── ITERATION4_COMPLETE.md
│   └── RUNNING_INSTRUCTIONS.md
│
├── Cargo.toml             # Rust dependencies
├── package.json           # Frontend dependencies  
├── README.md             # Main project documentation
├── CLAUDE.md             # Development guidelines
└── STRUCTURE.md          # This file
```

## Issues Still to Address

### 1. Frontend/Backend Separation
- **Problem**: React and Rust files mixed in `src/`
- **Solution**: Move to proper Tauri structure with `src-tauri/src/` for backend

### 2. Storage Module Bloat  
- **Problem**: `src/storage/mod.rs` is 1503 lines, other modules 600+ lines
- **Solution**: Break into focused sub-modules with clear responsibilities

### 3. Dead Code
- **Problem**: Unused `Application` struct in `lib.rs`
- **Solution**: Remove unused code or implement proper functionality

## Next Steps

1. **Phase 1**: Separate frontend/backend into proper Tauri structure
2. **Phase 2**: Refactor storage module into manageable pieces  
3. **Phase 3**: Remove dead code and consolidate similar functionality
4. **Phase 4**: Add proper module documentation and examples

## Benefits of Current Cleanup

✅ **Organized**: Test files, scripts, docs, and examples in proper directories  
✅ **Reduced Clutter**: Root directory is much cleaner  
✅ **Better Navigation**: Developers can find files more easily  
✅ **Clear Separation**: Different types of files have dedicated locations  

This structure provides a solid foundation for the remaining refactoring phases.
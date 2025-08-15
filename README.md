# Urpo
**Terminal-native OpenTelemetry trace explorer**

> *"htop for microservices"*

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

Urpo is a fast, terminal-native OpenTelemetry trace explorer designed to provide immediate insights into distributed system health. Like `htop` gives you instant visibility into system processes, Urpo gives you instant visibility into your microservices.

## ⚠️ Project Status: Early Development

**Current State**: Week 0 foundation with compilation errors to fix

This project is in very early development. The core architecture and vision are in place, but the code currently has compilation errors and is not yet functional. We're being completely transparent about this because we believe in the vision and want to build it right.

**What works**: 
- ✅ Solid architecture design with clear module separation
- ✅ Comprehensive data models for spans, traces, and service metrics  
- ✅ Well-designed CLI interface with multiple commands
- ✅ Strong Rust foundations following best practices

**What doesn't work yet**:
- ❌ Compilation errors (missing dependencies, feature flags)
- ❌ OTEL receiver implementation incomplete
- ❌ Terminal UI not yet functional
- ❌ Storage backends need implementation

**Next immediate steps**:
1. Fix compilation errors (missing `rand`, `hex`, `async_trait` deps, tokio `fs` feature)
2. Complete OTEL GRPC/HTTP receiver implementation
3. Build terminal UI with ratatui
4. Implement in-memory storage backend

## Vision

### The Problem
Current trace exploration tools like Jaeger are web-based, slow to start, and break your terminal workflow. When debugging a production issue, you want answers **now**, not after clicking through a heavy web interface.

### The Solution
Urpo provides:

- **Immediate startup**: Just run `urpo` and start receiving traces
- **Terminal-native**: Stays in your existing workflow, no browser context switching  
- **Two-view approach**: 
  - **Service health dashboard**: High-level RPS, error rates, latency percentiles
  - **Individual trace explorer**: Detailed span analysis and drill-down
- **Real-time updates**: Sub-second refresh rates for live system monitoring
- **Zero configuration**: Sensible defaults, works out of the box

## Planned Features

### Core Functionality
- **OTEL Protocol Support**: Both GRPC (4317) and HTTP (4318) receivers
- **Real-time Aggregation**: Service-level metrics with configurable time windows
- **Efficient Storage**: Bounded memory usage with automatic trace eviction
- **Fast Search**: Filter traces by service, operation, duration, error status
- **Keyboard Navigation**: Vim-like bindings for efficient terminal usage

### Interface Design
```
┌─ Urpo: Service Health ────────────────────────────────────────────────────┐
│ Services (5)          RPS    Error%   P50    P95    P99    Status         │
├───────────────────────────────────────────────────────────────────────────┤
│ → user-service       45.2     0.1%    12ms   45ms   120ms  ●  Healthy     │
│   auth-service       12.8     0.0%     8ms   18ms    25ms  ●  Healthy     │
│   payment-service     3.4     2.1%    95ms  340ms   890ms  ⚠  Degraded    │
│   inventory-service  28.1     0.3%    15ms   32ms    78ms  ●  Healthy     │
│   notification-svc    8.9     0.0%     5ms    9ms    15ms  ●  Healthy     │
├───────────────────────────────────────────────────────────────────────────┤
│ [ENTER] Drill down  [j/k] Navigate  [r] Refresh  [q] Quit                │
└───────────────────────────────────────────────────────────────────────────┘
```

### Command Line Interface
```bash
# Start with default settings (most common usage)
urpo

# Start with custom ports
urpo start --grpc-port 4317 --http-port 4318

# Run in headless mode (no UI, just log metrics)
urpo start --headless

# Export traces for analysis
urpo export --output traces.json --service user-service

# Check service health status
urpo health --service payment-service
```

## Architecture

Urpo is built with a modular architecture in Rust for maximum performance:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   OTEL Clients  │───▶│   Receivers     │───▶│   Storage       │
│                 │    │  GRPC + HTTP    │    │   In-memory     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                       │
┌─────────────────┐    ┌─────────────────┐           │
│   Terminal UI   │◀───│   Aggregation   │◀──────────┘
│   (ratatui)     │    │   Engine        │
└─────────────────┘    └─────────────────┘
```

**Modules**:
- **`receiver/`**: OTEL GRPC and HTTP protocol implementation
- **`storage/`**: Pluggable storage backends (in-memory, future: disk, cloud)
- **`core/`**: Domain models (Span, Trace, ServiceMetrics) and business logic
- **`ui/`**: Terminal interface with real-time updates
- **`cli/`**: Command-line interface and configuration

## Installation (Coming Soon)

```bash
# Via Cargo (when published)
cargo install urpo

# Via GitHub releases (when available)
curl -L https://github.com/yairfalse/urpo/releases/latest/download/urpo-x86_64-linux.tar.gz | tar xz

# From source (current method)
git clone https://github.com/yairfalse/urpo
cd urpo
cargo build --release
```

## Quick Start (Future)

1. **Start Urpo**:
   ```bash
   urpo
   ```

2. **Configure your applications** to send traces to `localhost:4317` (GRPC) or `localhost:4318` (HTTP)

3. **Navigate the interface**:
   - Use `j`/`k` to navigate services
   - Press `Enter` to drill down into traces
   - Press `r` to refresh data
   - Press `q` to quit


## Development Status & Roadmap

### Milestone 1: Core Foundation (Current) 
- [x] Architecture design and module structure
- [x] Core data models (Span, Trace, ServiceMetrics)
- [x] CLI interface design
- [ ] **Fix compilation errors** (immediate priority)
- [ ] OTEL receiver implementation
- [ ] Basic terminal UI

### Milestone 2: MVP (Target: 4-6 weeks)
- [ ] Service health dashboard
- [ ] Basic trace viewer
- [ ] In-memory storage with bounded eviction
- [ ] Real-time metric aggregation
- [ ] Keyboard navigation

### Milestone 3: Polish (Target: 8-10 weeks)
- [ ] Advanced filtering and search
- [ ] Configuration file support
- [ ] Export/import functionality
- [ ] Performance optimizations
- [ ] Comprehensive documentation

### Future Considerations
- Persistent storage backends
- Distributed deployment modes
- Plugin architecture for custom metrics
- Integration with alerting systems

## Contributing

We welcome contributions! This is an open-source project that aims to improve the developer debugging experience.

### Current Contribution Opportunities
1. **Fix compilation errors**: Help resolve missing dependencies and feature flags
2. **OTEL protocol implementation**: Contribute to GRPC/HTTP receiver development
3. **Terminal UI development**: Help build the ratatui-based interface
4. **Testing**: Write tests for core data models and business logic
5. **Documentation**: Improve code documentation and user guides

### Development Setup
```bash
git clone https://github.com/yairfalse/urpo
cd urpo
cargo check  # Currently fails - help us fix this!
cargo test   # Run tests
```

### Contribution Guidelines
- Follow Rust best practices (see `CLAUDE.md` for our standards)
- Write tests for new functionality
- Keep the terminal-first philosophy in mind
- Maintain the zero-configuration user experience

## Philosophy

Urpo is built on the belief that developer tools should:

1. **Start instantly**: No lengthy setup or deployment processes
2. **Stay in terminal**: Integrate with existing development workflows  
3. **Show what matters**: Focus on actionable insights, not data dump
4. **Perform fast**: Sub-second response times for real-time debugging
5. **Scale naturally**: Work equally well for single services and complex systems

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

- Inspired by `htop` for its immediate utility and terminal-native design
- Built on the OpenTelemetry standard for observability
- Powered by Rust's performance and reliability
- UI built with `ratatui` for responsive terminal interfaces

---

**Ready to help build the future of terminal-native observability?** 

Check out our [issues](https://github.com/yairfalse/urpo/issues) or join the discussion!

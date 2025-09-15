# Urpo

High-performance OpenTelemetry trace explorer with terminal and GUI interfaces.

## Performance

| Operation | Time | Notes |
|-----------|------|-------|
| Span ingestion | <10μs | Lock-free ring buffer |
| Search (100K spans) | <1ms | SIMD acceleration |
| Memory (1M spans) | <100MB | Arc<str> string interning |
| Cold start | <200ms | Zero-allocation initialization |

## Features

- **OpenTelemetry Protocol**: Full OTLP compliance with official protobuf types
- **W3C TraceContext**: Standard distributed tracing propagation
- **Tiered Storage**: Hot (ring buffer), Warm (mmap), Cold (LZ4)
- **SIMD Search**: AVX2 vectorized operations for pattern matching
- **Lock-free Ingestion**: Zero-contention span processing
- **String Interning**: Shared Arc<str> storage for 10-100x memory reduction

## Installation

```bash
git clone https://github.com/yairfalse/urpo.git
cd urpo
npm install
npm run tauri dev
```

## Configuration

OpenTelemetry clients send traces to:
- GRPC: `localhost:4317`
- HTTP: `localhost:4318`

```python
# Python example
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter

exporter = OTLPSpanExporter(endpoint="localhost:4317", insecure=True)
```

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   OTEL Clients  │    │    Receivers    │    │  Storage Engine │
│                 │    │                 │    │                 │
│ • Python Apps   │───▶│ • GRPC :4317    │───▶│ ┌─────────────┐ │
│ • Java Apps     │    │ • HTTP :4318    │    │ │ Hot Tier    │ │
│ • Go Services   │    │ • W3C Context   │    │ │ Ring Buffer │ │
│ • Any OTEL SDK  │    │ • OTLP Protocol │    │ │ <10μs       │ │
└─────────────────┘    └─────────────────┘    │ └─────────────┘ │
                                              │ ┌─────────────┐ │
                                              │ │ Warm Tier   │ │
                                              │ │ Memory-map  │ │
                                              │ │ <100μs      │ │
                                              │ └─────────────┘ │
                                              │ ┌─────────────┐ │
                                              │ │ Cold Tier   │ │
                                              │ │ LZ4 Archive │ │
                                              │ │ <1ms        │ │
                                              │ └─────────────┘ │
                                              └─────────────────┘
                                                       │
                                                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  User Interface │    │  Query Engine   │    │   Aggregation   │
│                 │    │                 │    │                 │
│ • Terminal UI   │◀───│ • SIMD Search   │◀───│ • Service Map   │
│ • Tauri GUI     │    │ • TraceQL       │    │ • Health Check  │
│ • 60fps Render  │    │ • <1ms Results  │    │ • Metrics       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

**Storage Engine**:
- Lock-free ring buffer for recent spans
- Memory-mapped files for medium-term storage  
- LZ4 compressed archives for long-term retention

**Query Engine**:
- SIMD-accelerated pattern matching
- TraceQL-inspired syntax
- Real-time execution

```sql
-- Find slow API requests
service = "api" && duration > 100ms

-- Find all errors
status = error

-- Complex queries with grouping
service = "frontend" && (status = error || duration > 500ms)
```

📖 **[Query Language Docs](docs/query-language.md)** | **[API Reference](docs/api.md)**

## Development

```bash
# Tests
cargo test

# Benchmarks  
cargo bench

# Release build
cargo build --release
```

## Requirements

- Rust 1.70+
- Node.js 18+ (for GUI)
- CPU with AVX2 support (for SIMD)

## License

MIT OR Apache-2.0
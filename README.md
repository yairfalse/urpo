# Urpo

A high-performance OpenTelemetry trace explorer with terminal and GUI interfaces.

## What's in the Name?

**Urpo** is Finnish for "fool" or "simpleton" - but like the Fool card in Tarot (card 0), it represents new beginnings, infinite potential, and the courage to step into the unknown. The Fool embarks on a journey with optimism and openness, carrying only what's essential. Similarly, Urpo approaches distributed tracing with a fresh perspective - lean, fast, and unburdened by unnecessary complexity.

## Features

### Blazing Fast Performance
- **<200ms startup time** - Ready before you finish typing
- **60fps UI** - Smooth, responsive interface
- **Handles 100K+ spans** without breaking a sweat
- **10μs per span processing** - Real-time ingestion at scale

### Powerful Trace Exploration
- **Natural language search** - Find traces using intuitive queries
- **Live service map** - Visualize your system with breathing, pulsing nodes
- **Service dependency graphs** - Auto-discovered from trace data
- **Instant filtering** - Roaring bitmap indexes for sub-millisecond queries

### Flexible Storage
- **In-memory mode** - Perfect for development and debugging
- **Persistent storage** - Production-ready with tiered architecture:
  - Hot tier: Lock-free ring buffer for recent traces
  - Warm tier: Memory-mapped files for medium-term storage
  - Cold tier: LZ4 compressed archives for long-term retention

### Rich Visualizations
- **Command palette** (Cmd+K) - Quick access to any action
- **Minimap navigation** - See your entire trace timeline at a glance
- **Span details view** - Deep dive into individual spans
- **Real-time updates** - Watch your system breathe

## Quick Start

### Prerequisites
- Rust 1.70+
- Node.js 18+
- npm or yarn

### Installation

```bash
# Clone the repository
git clone https://github.com/yairfalse/urpo.git
cd urpo

# Install dependencies
npm install

# Build and run the GUI
npm run tauri dev

# Or run the terminal interface
cargo run --bin urpo
```

### Running with Persistent Storage

```bash
# Enable persistent storage with environment variables
URPO_PERSISTENT=true URPO_DATA_DIR=./data npm run tauri dev
```

## Receiving Traces

Urpo implements the OpenTelemetry protocol and listens on standard OTEL ports:
- **GRPC**: Port 4317
- **HTTP**: Port 4318

Configure your applications to send traces to Urpo:

```yaml
# Example OTEL configuration
exporters:
  otlp:
    endpoint: localhost:4317
    insecure: true
```

### Python Example

```python
from opentelemetry import trace
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor

# Setup tracing
trace.set_tracer_provider(TracerProvider())
tracer = trace.get_tracer(__name__)

# Configure OTLP exporter to send to Urpo
otlp_exporter = OTLPSpanExporter(
    endpoint="localhost:4317",
    insecure=True,
)

# Add the exporter to the tracer
trace.get_tracer_provider().add_span_processor(
    BatchSpanProcessor(otlp_exporter)
)

# Create spans
with tracer.start_as_current_span("my-operation"):
    # Your code here
    pass
```

## Configuration

Create a `urpo.yaml` file to customize settings:

```yaml
server:
  grpc_port: 4317
  http_port: 4318

storage:
  persistent: true
  data_dir: ./urpo_data
  max_spans: 100000
  hot_storage_size: 10000
  warm_storage_mb: 512
  cold_retention_hours: 24

ui:
  theme: dark
  refresh_rate: 100ms
```

## Interface Options

### Tauri GUI
Modern, React-based interface with rich visualizations:
- Live service dependency graphs
- Interactive trace timeline
- Command palette for quick navigation
- Real-time metrics dashboard

![GUI Interface](docs/images/gui-preview.png)

### Terminal UI
Fast, keyboard-driven interface for terminal enthusiasts:
```
┌─ Urpo: Service Health ────────────────────────────────────────────────────┐
│ Services (5)          RPS    Error%   P50    P95    P99    Status         │
├───────────────────────────────────────────────────────────────────────────┤
│ > user-service       45.2     0.1%    12ms   45ms   120ms  [OK]  Healthy  │
│   auth-service       12.8     0.0%     8ms   18ms    25ms  [OK]  Healthy  │
│   payment-service     3.4     2.1%    95ms  340ms   890ms  [!!]  Degraded │
│   inventory-service  28.1     0.3%    15ms   32ms    78ms  [OK]  Healthy  │
│   notification-svc    8.9     0.0%     5ms    9ms    15ms  [OK]  Healthy  │
├───────────────────────────────────────────────────────────────────────────┤
│ [ENTER] Drill down  [j/k] Navigate  [r] Refresh  [q] Quit                │
└───────────────────────────────────────────────────────────────────────────┘
```

## Architecture

Urpo is built with performance as the primary goal:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   OTEL Clients  │--->│   Receivers     │--->│   Storage       │
│                 │    │  GRPC + HTTP    │    │   Tiered        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                       |
                                                       v
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Tauri GUI or   │<---│   Aggregation   │<---│   Query Engine  │
│  Terminal UI     │    │   Engine        │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

**Key Components**:
- **Rust backend**: Zero-allocation hot paths, lock-free data structures
- **Tauri framework**: Native performance with web technologies
- **React frontend**: Virtualized rendering for massive datasets
- **Single binary**: No complex deployments or dependencies

## Testing

Send test traces using the included Python script:

```bash
# Send sample traces
python examples/send_traces.py

# Or use the quick trace script
./send_quick_traces.sh
```

## Development

```bash
# Run tests
cargo test

# Run benchmarks
cargo bench

# Build release version
npm run tauri build

# Build terminal-only version
cargo build --release --bin urpo
```

## Performance

Urpo is designed for extreme performance:

| Operation | Performance | Notes |
|-----------|------------|-------|
| Span ingestion | <10μs | Lock-free data structures |
| Hot storage access | <10μs | Ring buffer in memory |
| Warm storage access | <100μs | Memory-mapped files |
| Cold storage access | <1ms | LZ4 decompression |
| Search (100K spans) | <1ms | Roaring bitmap indexes |
| UI refresh | 60fps | Virtualized rendering |

## License

MIT OR Apache-2.0

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

Built with:
- [Tauri](https://tauri.app/) - Build smaller, faster, and more secure desktop applications
- [OpenTelemetry](https://opentelemetry.io/) - High-quality, ubiquitous, and portable telemetry
- [Ratatui](https://ratatui.rs/) - Terminal UI framework for Rust
- [React](https://react.dev/) - Library for web and native user interfaces

Special thanks to the OpenTelemetry community for establishing excellent standards for observability.

---

*"In the beginner's mind there are many possibilities, in the expert's mind there are few."* - Shunryu Suzuki
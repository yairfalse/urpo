# Urpo Configuration Guide

This guide explains how to configure Urpo using either file-based configuration or UI settings.

## Quick Start

Urpo works out-of-the-box with sensible defaults:

```bash
# Start with defaults (gRPC: 4317, HTTP: 4318)
cargo run

# Override ports via CLI
cargo run -- --grpc-port 5317 --http-port 5318

# Use custom config file
cargo run -- --config my-config.yaml
```

## Configuration Methods

Urpo supports **three ways** to configure settings (in order of precedence):

1. **CLI Arguments** (highest priority)
2. **Environment Variables**
3. **Configuration File** (lowest priority)

### 1. File-Based Configuration

Create a YAML config file at `~/.config/urpo/config.yaml`:

```yaml
server:
  grpc_port: 4317
  http_port: 4318
  bind_address: "0.0.0.0"

storage:
  max_spans: 100000
  max_memory_mb: 1024
  retention_duration: 1h

sampling:
  default_rate: 1.0  # Sample 100% of traces
```

**Example config:** See `config.example.yaml` for a complete template with all options.

#### Copy Example Config

```bash
# Create config directory
mkdir -p ~/.config/urpo

# Copy example config
cp config.example.yaml ~/.config/urpo/config.yaml

# Edit your config
vim ~/.config/urpo/config.yaml
```

### 2. CLI Arguments

Override specific settings from the command line:

```bash
# Override receiver ports
urpo --grpc-port 5317 --http-port 5318

# Set memory limit
urpo --memory-limit 2048

# Use custom config file
urpo --config /path/to/config.yaml

# Enable debug logging
urpo --debug

# Run in headless mode (no UI)
urpo --headless

# Validate configuration without starting
urpo --check-config
```

### 3. Environment Variables

Set configuration via environment variables:

```bash
export URPO_GRPC_PORT=5317
export URPO_HTTP_PORT=5318
export URPO_MEMORY_LIMIT=2048
export URPO_DEBUG=true

urpo
```

## UI Settings Panel

**View current configuration in the UI:**

1. Start Urpo: `cargo run`
2. Press **`s`** to open Settings
3. View all current configuration values
4. Press **`Esc`** to go back

The settings panel displays:
- Server configuration (ports, bind address)
- Storage configuration (limits, retention)
- UI preferences (theme, refresh rate)
- Sampling rates
- Monitoring thresholds

**Note:** The UI currently shows settings but doesn't allow editing. To change settings:
- Edit `~/.config/urpo/config.yaml` (hot-reloaded when running)
- Or use CLI flags when starting Urpo

## Configuration Options

### Server Configuration

```yaml
server:
  grpc_port: 4317           # OTLP/gRPC receiver port
  http_port: 4318           # OTLP/HTTP receiver port
  bind_address: "0.0.0.0"   # Bind to all interfaces
  max_connections: 1000     # Maximum concurrent connections
  connection_timeout: 30s   # Connection timeout
```

**CLI Flags:**
- `--grpc-port PORT`
- `--http-port PORT`

### Storage Configuration

```yaml
storage:
  max_spans: 100000                # Maximum spans in memory
  max_memory_mb: 1024              # Memory limit in MB
  retention_duration: 1h           # How long to keep spans
  cleanup_interval: 30s            # Cleanup frequency
  compression_enabled: false       # Enable compression
  persistent: false                # Enable disk persistence
  data_dir: ./urpo_data            # Data directory
```

**CLI Flags:**
- `--memory-limit MB`

### Sampling Configuration

```yaml
sampling:
  default_rate: 1.0     # Default sampling rate (0.0-1.0)
  per_service:
    high-volume-service: 0.1    # Sample 10%
    critical-service: 1.0        # Sample 100%
  adaptive: false       # Enable adaptive sampling
```

### UI Configuration

```yaml
ui:
  refresh_rate: 100ms   # UI refresh interval
  theme: dark           # Color theme: dark, light, auto
  vim_mode: true        # Enable vim keybindings
  show_help: true       # Show help on startup
  default_view: services # Default view
```

### Monitoring Configuration

```yaml
monitoring:
  health_check_interval: 10s
  metrics_enabled: true
  alerts:
    error_rate_threshold: 5.0       # % error rate alert
    p95_latency_threshold: 1s       # P95 latency alert
    min_sample_size: 100            # Minimum samples for alerts
```

## Testing Your Configuration

### 1. Validate Configuration

```bash
# Check if your config is valid
cargo run -- --check-config

# Output shows all current settings:
# Configuration is valid!
#   GRPC port: 4317
#   HTTP port: 4318
#   Memory limit: 1024MB
#   Max spans: 100000
```

### 2. Test gRPC Receiver

```bash
# In terminal 1: Start Urpo
cargo run

# In terminal 2: Send test trace
./test-grpc.sh
```

### 3. Test HTTP Receiver

```bash
# In terminal 1: Start Urpo
cargo run

# In terminal 2: Send test trace
./test-http.sh
```

## Common Configuration Scenarios

### High-Volume Production

```yaml
server:
  grpc_port: 4317
  http_port: 4318
  max_connections: 5000

storage:
  max_spans: 1000000      # 1M spans
  max_memory_mb: 4096     # 4GB
  retention_duration: 6h  # Keep 6 hours
  compression_enabled: true

sampling:
  adaptive: true
  target_sps: 100000      # 100k spans/sec
```

### Development / Testing

```yaml
server:
  grpc_port: 4317
  http_port: 4318

storage:
  max_spans: 10000        # Smaller buffer
  max_memory_mb: 256      # Less memory

sampling:
  default_rate: 1.0       # Sample everything

logging:
  level: debug            # Verbose logging
```

### Resource-Constrained

```yaml
server:
  grpc_port: 4317
  http_port: 4318

storage:
  max_spans: 5000
  max_memory_mb: 128
  retention_duration: 15m
  compression_enabled: true

sampling:
  default_rate: 0.1       # Sample 10%
```

## Troubleshooting

### Port Already in Use

```bash
# Check what's using the port
lsof -i :4317

# Use different ports
cargo run -- --grpc-port 5317 --http-port 5318
```

### Configuration Not Loading

```bash
# Check config file location
ls -la ~/.config/urpo/config.yaml

# Validate config syntax
cargo run -- --check-config --config ~/.config/urpo/config.yaml

# Use debug mode to see what's loaded
cargo run -- --debug
```

### Memory Issues

```yaml
# Reduce memory usage in config
storage:
  max_spans: 50000
  max_memory_mb: 512
```

## Next Steps

- See `README.md` for general usage
- See `config.example.yaml` for complete configuration options
- Run test scripts: `./test-grpc.sh` and `./test-http.sh`
- Press `s` in the UI to view current settings

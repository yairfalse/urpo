# Urpo Quick Start Guide

## 🚀 Getting Started in 30 Seconds

```bash
# 1. Build Urpo
cargo build --release

# 2. Start Urpo
cargo run

# 3. Send a test trace (in another terminal)
./test-http.sh

# 4. View your traces in the UI!
# Press 's' to see settings
```

## 📋 What You Get

Urpo now has **two ways** to configure gRPC/HTTP receivers:

### ✅ Option 1: File-Based Configuration

Create `~/.config/urpo/config.yaml`:

```yaml
server:
  grpc_port: 4317
  http_port: 4318
```

See `config.example.yaml` for all options.

### ✅ Option 2: UI Settings Panel

1. Run `cargo run`
2. Press **`s`** in the UI
3. View all configuration settings

**Keybindings:**
- `s` - Open settings panel
- `Esc` - Exit settings
- `q` - Quit
- `Tab` - Switch views (Services/Traces)
- `↑↓` - Navigate lists

## 🧪 Testing Receivers

### Test gRPC (Port 4317)

```bash
# Start Urpo
cargo run

# In another terminal
./test-grpc.sh
```

### Test HTTP (Port 4318)

```bash
# Start Urpo
cargo run

# In another terminal
./test-http.sh
```

## 🎯 What Works Now

✅ **Configuration:**
- ✅ File-based config (`~/.config/urpo/config.yaml`)
- ✅ CLI arguments (`--grpc-port`, `--http-port`)
- ✅ Environment variables (`URPO_GRPC_PORT`, etc.)
- ✅ UI settings panel (read-only, press `s`)

✅ **Receivers:**
- ✅ gRPC receiver on port 4317
- ✅ HTTP receiver on port 4318
- ✅ Both protocols working simultaneously

✅ **Testing:**
- ✅ Test scripts: `test-grpc.sh` and `test-http.sh`
- ✅ Health checks
- ✅ Sample trace ingestion

## 📖 Documentation

- **CONFIGURATION.md** - Complete configuration guide
- **config.example.yaml** - Example config with all options
- **test-grpc.sh** - Test gRPC receiver
- **test-http.sh** - Test HTTP receiver

## ⚙️ Common Commands

```bash
# Use custom ports
cargo run -- --grpc-port 5317 --http-port 5318

# Use custom config file
cargo run -- --config my-config.yaml

# Validate config
cargo run -- --check-config

# Debug mode
cargo run -- --debug

# Headless mode (no UI)
cargo run -- --headless
```

## 🔧 Configuration Precedence

1. **CLI arguments** (highest priority)
2. **Environment variables**
3. **Config file** (`~/.config/urpo/config.yaml`)
4. **Defaults** (lowest priority)

Example:
```bash
# Config file says port 4317
# Override with CLI:
cargo run -- --grpc-port 9999  # Uses 9999
```

## 💡 Tips

- Press `s` in the UI to see current configuration
- Config file at `~/.config/urpo/config.yaml` is hot-reloaded
- Use `--check-config` to validate before starting
- Test scripts require `curl` and `jq` (install via brew)

## 🐛 Troubleshooting

**Port already in use?**
```bash
# Check what's using it
lsof -i :4317

# Use different port
cargo run -- --grpc-port 5317
```

**Can't see traces?**
- Check the Settings panel (`s` key)
- Verify receivers are running (should see ports in logs)
- Run test scripts to send sample data

**Config not loading?**
```bash
# Validate config
cargo run -- --check-config

# Use debug mode
cargo run -- --debug
```

## 🎉 Success!

You now have:
1. ✅ File-based configuration system
2. ✅ UI settings panel (press `s`)
3. ✅ Working gRPC and HTTP receivers
4. ✅ Test scripts to verify everything works

Enjoy using Urpo! 🚀

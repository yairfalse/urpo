# Running Urpo - Terminal UI with Fake Data

## Iteration 1 Complete: Fake Data Terminal Display

The first iteration of Urpo is now complete! The application displays a terminal-based dashboard with fake service metrics that update in real-time.

## How to Run

```bash
# Build the project
cargo build

# Run the application (starts the terminal UI)
cargo run

# Or simply:
./target/debug/urpo
```

## What You'll See

When you run `cargo run`, you'll see a terminal dashboard that displays:

```
┌─ Urpo: Service Health (5 services) ───────────────────────────────────────┐
│ Service           RPS    Error%   P50    P95    P99    Status             │
├────────────────────────────────────────────────────────────────────────────┤
│ → api-gateway    245.2    0.1%    12ms   45ms   89ms   ● Healthy          │
│   user-service   156.7    0.8%    23ms   78ms   156ms  ● Healthy          │
│   payment-service 89.3   12.1%   234ms   567ms   1.2s   ● Unhealthy        │
│   inventory-svc  234.1    2.3%    34ms   89ms   234ms  ⚠ Degraded         │
│   notification-api 67.2   0.2%    15ms   38ms   72ms   ● Healthy          │
├────────────────────────────────────────────────────────────────────────────┤
│ [q] Quit  [j/k] Navigate  [Enter] Details  [r] Refresh                     │
└────────────────────────────────────────────────────────────────────────────┘
```

## Features Implemented

### 1. **Terminal UI with ratatui**
- Clean, professional-looking dashboard
- Table layout with service metrics
- Color-coded health status (green=healthy, yellow=degraded, red=unhealthy)
- Responsive terminal interface

### 2. **Fake Data Generation**
- 5 predefined services with realistic names
- Metrics that vary slightly each second to simulate live data
- Includes RPS, error rates, and latency percentiles (P50, P95, P99)
- Random variations and occasional error spikes for realism

### 3. **Interactive Controls**
- `q` - Quit the application
- `j`/`k` or `↑`/`↓` - Navigate up/down in the service list
- `Enter` - Select a service (will navigate to traces view in future iterations)
- `r` - Manually refresh data
- `Tab` - Switch between tabs (Services, Traces, Spans)
- `/` - Enter search mode (foundation for future filtering)

### 4. **Live Updates**
- Metrics automatically update every second
- Smooth variations in values to simulate real service behavior
- Maintains service identity between updates

### 5. **Professional UI Elements**
- Title bar showing "Urpo: Service Health" with service count
- Table headers with clear column labels
- Row highlighting for selected service
- Status indicators with appropriate symbols (● for healthy/unhealthy, ⚠ for degraded)
- Footer with keyboard shortcuts

## Code Structure

### New Files Created:
- `/src/ui/fake_data.rs` - Fake data generator with realistic service metrics
- Updated `/src/ui/mod.rs` - Complete terminal UI implementation
- Updated `/src/cli/mod.rs` - CLI integration to launch the UI

### Key Components:
1. **FakeDataGenerator** - Generates realistic service metrics with variations
2. **App** - Application state management and keyboard handling
3. **TerminalUI** - Terminal setup, event loop, and rendering
4. **Drawing functions** - Modular UI rendering for different views

## Testing

Run the tests to verify everything works:
```bash
cargo test ui --lib
```

All UI tests should pass:
- ✓ App creation with initial data
- ✓ Tab navigation
- ✓ Quit handling (q key and Ctrl+C)
- ✓ Search mode activation

## Next Steps (Future Iterations)

This foundation sets up for:
- Phase 1, Iteration 2: Real OTEL receiver integration
- Phase 1, Iteration 3: In-memory storage with proper data structures
- Phase 2: Trace detail views and span analysis
- Phase 3: Advanced features like filtering, export, and persistence

## Technical Notes

- The UI gracefully handles terminal resize
- Proper cleanup on exit (restores terminal state)
- No panics - all errors are properly handled
- Memory efficient - fake data is regenerated, not accumulated
- Cross-platform compatible (uses crossterm backend)

## Troubleshooting

If you see an error about "Device not configured", make sure you're running in an actual terminal (not in a CI environment or non-interactive shell).

The application requires a terminal that supports:
- Raw mode
- Alternate screen buffer
- ANSI color codes
- Unicode characters for status symbols

Most modern terminals (iTerm2, Terminal.app, Windows Terminal, etc.) support these features.
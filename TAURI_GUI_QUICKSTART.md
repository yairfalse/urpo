# Urpo Tauri GUI - Quick Start

## Why Tabs Appear Empty

**This is normal!** The tabs are empty because no trace data has been sent yet.

The Urpo GUI works like this:
1. **Backend receives traces** → stores them in memory
2. **Frontend queries data** → displays in tabs
3. **Real-time updates** → new traces appear instantly

If you haven't sent any traces, all tabs (except Dashboard) will show:
```
No services detected
Start sending OTLP data to see services here
```

## Quick Fix: Send Test Data

### Step 1: Start the Tauri App

```bash
npm run tauri:dev
```

Wait for the app window to open. You should see the login screen.

### Step 2: Send Test Traces

In a **new terminal**, run:

```bash
./test-gui-traces.sh
```

This will send 5 test traces to the backend.

### Step 3: Check the Tabs

Now navigate through the tabs:
- **Tab 1 (Dashboard)**: Shows overview with 5 services
- **Tab 2 (Services)**: Shows service-1 through service-5
- **Tab 3 (Traces)**: Shows 5 recent traces
- **Tab 4 (Health)**: Shows service health metrics

## Important Port Information

**The Tauri GUI uses different ports than the standalone CLI:**

| Component | Tauri GUI | Standalone CLI |
|-----------|-----------|----------------|
| gRPC      | **4327**  | 4317           |
| HTTP      | **4328**  | 4318           |

Why? To avoid conflicts when running both simultaneously.

## Manual Testing

If you want to send traces manually:

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"resourceSpans":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"my-service"}}]},"scopeSpans":[{"spans":[{"traceId":"0102030405060708090a0b0c0d0e0f10","spanId":"0102030405060708","name":"my-operation","startTimeUnixNano":"1700000000000000000","endTimeUnixNano":"1700000001000000000"}]}]}]}' \
  http://localhost:4328/v1/traces
```

**Note the port: 4328 (not 4318)**

## Troubleshooting

### Problem: "Failed to connect to localhost port 4328"

**Solution**: The Tauri app isn't running yet. Start it with:
```bash
npm run tauri:dev
```

### Problem: Tabs still empty after sending traces

**Solution 1**: Check the browser DevTools console (in the Tauri window):
- Press `Cmd+Option+I` (Mac) or `Ctrl+Shift+I` (Linux/Windows)
- Look for logs like:
  ```
  Dashboard data: { services: 5, traces: 5, system: {...} }
  ```

**Solution 2**: Check if traces were received in the backend logs:
```
Successfully stored 5 spans
Broadcasting trace event: TraceEvent { trace_id: "...", ... }
```

### Problem: "Address already in use (os error 48)"

**Solution**: Another process is using ports 4327/4328. Kill it:
```bash
lsof -ti:4327,4328 | xargs kill -9
```

Then restart the Tauri app.

## Real-time Updates

Once you've sent traces, **new traces will appear instantly** without refreshing!

Try it:
```bash
# Terminal 1: Tauri app running
npm run tauri:dev

# Terminal 2: Send traces continuously
while true; do
  ./test-gui-traces.sh
  sleep 2
done
```

You should see the tabs update in real-time as new traces arrive! 🚀

## Next Steps

- **Connect your app**: Configure your application to send OTLP traces to `http://localhost:4328/v1/traces`
- **Explore features**: Try the flamegraph view, search functionality, and service health monitoring
- **Read the docs**: See `CONFIGURATION.md` for advanced configuration options

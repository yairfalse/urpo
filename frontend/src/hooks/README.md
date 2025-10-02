# Real-time Traces Hook

## Usage

Add real-time trace updates to any component:

```typescript
import { useRealtimeTraces } from './hooks';

function MyComponent() {
  // Enable real-time updates
  useRealtimeTraces(true);

  // Your component will automatically receive new traces
  // No polling needed!
  const { data: traces } = useTauriData('list_recent_traces', {
    limit: 100
  });

  return <TraceList traces={traces} />;
}
```

## How It Works

1. **Backend:** When a new trace arrives via OTLP, the receiver broadcasts a `TraceEvent`
2. **Tauri:** The event is emitted to all frontend windows as `trace_received`
3. **Frontend:** `useRealtimeTraces` listens for events and updates React Query cache
4. **UI:** Your components re-render with the new data automatically

## Performance

- **Latency:** Sub-100ms from trace arrival to UI update
- **No polling:** Zero overhead when no traces arrive
- **Automatic deduplication:** React Query handles cache updates efficiently
- **Buffer:** Events are buffered (1000 max) to prevent overwhelming the UI

## API

### `useRealtimeTraces(enabled?: boolean)`

Subscribe to all new trace events.

```typescript
// Enable/disable dynamically
const [enabled, setEnabled] = useState(true);
useRealtimeTraces(enabled);
```

### `useRealtimeTrace(traceId: string | null, enabled?: boolean)`

Subscribe to updates for a specific trace (useful for trace detail views).

```typescript
function TraceDetailView({ traceId }: { traceId: string }) {
  // Auto-refresh when new spans arrive for this trace
  useRealtimeTrace(traceId, true);

  const { data: spans } = useTauriData('get_trace_spans', {
    trace_id: traceId
  });

  return <SpanList spans={spans} />;
}
```

## Example: Full Integration

```typescript
import { useRealtimeTraces } from './hooks';
import { useTauriData } from './hooks';

export function Dashboard() {
  // Enable real-time updates
  useRealtimeTraces(true);

  // These queries will auto-update when new traces arrive
  const { data: traces } = useTauriData('list_recent_traces', { limit: 50 });
  const { data: services } = useTauriData('get_service_metrics');

  return (
    <div>
      <h1>Live Trace Dashboard</h1>
      <ServiceList services={services} />
      <TraceList traces={traces} />
      {/* New traces appear instantly! */}
    </div>
  );
}
```

## Testing

Send a test trace:

```bash
# Run Urpo
npm run tauri dev

# In another terminal
./test-http.sh

# Watch traces appear instantly in the UI!
```

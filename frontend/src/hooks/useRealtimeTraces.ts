import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useQueryClient } from '@tanstack/react-query';
import { TraceInfo } from '../types';

/**
 * Real-time trace event from backend
 */
interface TraceEvent {
  trace_id: string;
  service_name: string;
  span_count: number;
  timestamp: number;
}

/**
 * Hook for real-time trace ingestion events
 *
 * Automatically updates React Query cache when new traces arrive
 * Provides sub-second latency for trace visualization
 */
export function useRealtimeTraces(enabled = true) {
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!enabled) return;

    let unlisten: (() => void) | null = null;

    // Subscribe to trace_received events from Tauri backend
    const setupListener = async () => {
      const unlistenFn = await listen<TraceEvent>('trace_received', (event) => {
        const traceEvent = event.payload;

        console.log('[Real-time] New trace received:', traceEvent.trace_id);

        // Update traces query cache
        queryClient.setQueryData(['traces'], (old: TraceInfo[] | undefined) => {
          // Create new trace entry
          const newTrace: TraceInfo = {
            trace_id: traceEvent.trace_id,
            service_name: traceEvent.service_name,
            span_count: traceEvent.span_count,
            start_time: traceEvent.timestamp,
            duration: 0, // Will be updated when trace is complete
            status: 'ok' as const,
            error_count: 0,
          };

          // Prepend new trace, keep max 1000 traces
          const updated = [newTrace, ...(old || [])].slice(0, 1000);

          return updated;
        });

        // Optionally invalidate to refresh from backend
        queryClient.invalidateQueries({ queryKey: ['service_metrics'] });
      });

      unlisten = unlistenFn;
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [enabled, queryClient]);
}

/**
 * Hook for real-time trace updates for a specific trace
 *
 * Listens for span additions to a specific trace
 */
export function useRealtimeTrace(traceId: string | null, enabled = true) {
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!enabled || !traceId) return;

    let unlisten: (() => void) | null = null;

    const setupListener = async () => {
      const unlistenFn = await listen<TraceEvent>('trace_received', (event) => {
        const traceEvent = event.payload;

        // Only process events for our trace
        if (traceEvent.trace_id === traceId) {
          console.log('[Real-time] Trace updated:', traceId, 'spans:', traceEvent.span_count);

          // Invalidate trace spans query to refresh
          queryClient.invalidateQueries({
            queryKey: ['trace_spans', traceId]
          });
        }
      });

      unlisten = unlistenFn;
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [enabled, traceId, queryClient]);
}

/**
 * React hooks for Tauri commands with React Query integration
 *
 * These hooks provide:
 * - Automatic caching and invalidation
 * - Background refetching
 * - Loading and error states
 * - Optimistic updates
 * - Request deduplication
 */

import { useQuery, useMutation, useQueryClient, type UseQueryOptions } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { TauriClient, TauriEvents, isTauriAvailable } from './client';
import type {
  ServiceMetrics,
  TraceInfo,
  SystemMetrics,
  StorageInfo,
  ServiceMap,
  SpanData,
  TelemetryUpdate,
  ListTracesParams,
  SearchTracesParams,
  GetTraceSpansParams,
  ServiceMetricsBatchParams,
  GetServiceMapParams,
} from './types';

// ============================================================================
// QUERY KEYS
// ============================================================================

export const queryKeys = {
  all: ['tauri'] as const,
  serviceMetrics: () => [...queryKeys.all, 'service-metrics'] as const,
  serviceMetricsBatch: (names: string[]) => [...queryKeys.all, 'service-metrics-batch', names] as const,
  traces: () => [...queryKeys.all, 'traces'] as const,
  recentTraces: (params: ListTracesParams) => [...queryKeys.traces(), 'recent', params] as const,
  errorTraces: (limit: number) => [...queryKeys.traces(), 'errors', limit] as const,
  traceSpans: (traceId: string) => [...queryKeys.traces(), 'spans', traceId] as const,
  searchTraces: (params: SearchTracesParams) => [...queryKeys.traces(), 'search', params] as const,
  systemMetrics: () => [...queryKeys.all, 'system-metrics'] as const,
  storageInfo: () => [...queryKeys.all, 'storage-info'] as const,
  serviceMap: (params?: GetServiceMapParams) => [...queryKeys.all, 'service-map', params] as const,
} as const;

// ============================================================================
// DEFAULT QUERY OPTIONS
// ============================================================================

const defaultOptions: Partial<UseQueryOptions> = {
  enabled: isTauriAvailable(),
  staleTime: 100, // Consider data fresh for only 100ms - BLAZING FAST!
  gcTime: 10 * 60 * 1000, // Keep in cache for 10 minutes (renamed from cacheTime)
  refetchOnWindowFocus: false, // Don't refetch on window focus by default
  retry: 2,
};

// ============================================================================
// SERVICE METRICS HOOKS
// ============================================================================

/**
 * Hook to fetch all service metrics
 */
export function useServiceMetrics(options?: Partial<UseQueryOptions<ServiceMetrics[]>>) {
  return useQuery({
    queryKey: queryKeys.serviceMetrics(),
    queryFn: TauriClient.getServiceMetrics,
    refetchInterval: 500, // BLAZING FAST: Auto-refresh every 500ms for real-time updates!
    ...defaultOptions,
    ...options,
  });
}

/**
 * Hook to fetch specific service metrics in batch
 */
export function useServiceMetricsBatch(
  serviceNames: string[],
  options?: Partial<UseQueryOptions<ServiceMetrics[]>>
) {
  return useQuery({
    queryKey: queryKeys.serviceMetricsBatch(serviceNames),
    queryFn: () => TauriClient.getServiceMetricsBatch({ service_names: serviceNames }),
    enabled: isTauriAvailable() && serviceNames.length > 0,
    ...defaultOptions,
    ...options,
  });
}

// ============================================================================
// TRACE HOOKS
// ============================================================================

/**
 * Hook to fetch recent traces
 */
export function useRecentTraces(
  params: ListTracesParams,
  options?: Partial<UseQueryOptions<TraceInfo[]>>
) {
  return useQuery({
    queryKey: queryKeys.recentTraces(params),
    queryFn: () => TauriClient.listRecentTraces(params),
    refetchInterval: 750, // BLAZING FAST: Auto-refresh every 750ms for real-time trace updates!
    ...defaultOptions,
    ...options,
  });
}

/**
 * Hook to fetch error traces
 */
export function useErrorTraces(
  limit: number = 100,
  options?: Partial<UseQueryOptions<TraceInfo[]>>
) {
  return useQuery({
    queryKey: queryKeys.errorTraces(limit),
    queryFn: () => TauriClient.getErrorTraces(limit),
    refetchInterval: 10000,
    ...defaultOptions,
    ...options,
  });
}

/**
 * Hook to fetch spans for a specific trace
 */
export function useTraceSpans(
  traceId: string,
  options?: Partial<UseQueryOptions<SpanData[]>>
) {
  return useQuery({
    queryKey: queryKeys.traceSpans(traceId),
    queryFn: () => TauriClient.getTraceSpans({ trace_id: traceId }),
    enabled: isTauriAvailable() && !!traceId,
    ...defaultOptions,
    ...options,
  });
}

/**
 * Hook to search traces
 */
export function useSearchTraces(
  params: SearchTracesParams,
  options?: Partial<UseQueryOptions<TraceInfo[]>>
) {
  return useQuery({
    queryKey: queryKeys.searchTraces(params),
    queryFn: () => TauriClient.searchTraces(params),
    enabled: isTauriAvailable() && params.query.length > 0,
    ...defaultOptions,
    ...options,
  });
}

// ============================================================================
// SYSTEM HOOKS
// ============================================================================

/**
 * Hook to fetch system metrics with real-time updates
 */
export function useSystemMetrics(options?: Partial<UseQueryOptions<SystemMetrics>>) {
  const queryClient = useQueryClient();

  // Subscribe to telemetry updates
  useEffect(() => {
    if (!isTauriAvailable()) return;

    let unlisten: (() => void) | undefined;

    TauriEvents.onTelemetryUpdate((update) => {
      // Update the cache with telemetry data
      queryClient.setQueryData(queryKeys.systemMetrics(), (old: SystemMetrics | undefined) => {
        if (!old) return old;

        return {
          ...old,
          heap_usage_mb: update.heap_usage_mb,
          cpu_usage_percent: update.cpu_usage_percent,
          memory_pressure: update.memory_pressure,
          cold_fetch_latency_ms: update.cold_fetch_latency_ms,
          free_space_mb: update.free_space_mb,
          command_latencies: update.command_latencies,
        };
      });
    }).then(fn => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [queryClient]);

  return useQuery({
    queryKey: queryKeys.systemMetrics(),
    queryFn: TauriClient.getSystemMetrics,
    refetchInterval: 5000,
    ...defaultOptions,
    ...options,
  });
}

/**
 * Hook to fetch storage information
 */
export function useStorageInfo(options?: Partial<UseQueryOptions<StorageInfo>>) {
  return useQuery({
    queryKey: queryKeys.storageInfo(),
    queryFn: TauriClient.getStorageInfo,
    refetchInterval: 30000, // Refresh every 30 seconds
    ...defaultOptions,
    ...options,
  });
}

/**
 * Hook to fetch service map
 */
export function useServiceMap(
  params?: GetServiceMapParams,
  options?: Partial<UseQueryOptions<ServiceMap>>
) {
  return useQuery({
    queryKey: queryKeys.serviceMap(params),
    queryFn: () => TauriClient.getServiceMap(params),
    refetchInterval: 15000, // Refresh every 15 seconds
    ...defaultOptions,
    ...options,
  });
}

// ============================================================================
// MUTATION HOOKS
// ============================================================================

/**
 * Hook to start the OTEL receiver
 */
export function useStartReceiver() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: TauriClient.startReceiver,
    onSuccess: () => {
      // Invalidate all queries after starting receiver
      queryClient.invalidateQueries({ queryKey: queryKeys.all });
    },
  });
}

/**
 * Hook to stop the OTEL receiver
 */
export function useStopReceiver() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: TauriClient.stopReceiver,
    onSuccess: () => {
      // Clear all cached data after stopping receiver
      queryClient.removeQueries({ queryKey: queryKeys.all });
    },
  });
}

/**
 * Hook to trigger tier migration
 */
export function useTriggerTierMigration() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: TauriClient.triggerTierMigration,
    onSuccess: () => {
      // Invalidate storage info after migration
      queryClient.invalidateQueries({ queryKey: queryKeys.storageInfo() });
    },
  });
}

// ============================================================================
// STREAMING HOOKS
// ============================================================================

/**
 * Hook to stream large trace data
 */
export function useTraceStream(traceId: string | null) {
  const [chunks, setChunks] = useState<SpanData[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    if (!isTauriAvailable() || !traceId) return;

    let unlistenChunk: (() => void) | undefined;
    let unlistenComplete: (() => void) | undefined;

    const startStream = async () => {
      setIsStreaming(true);
      setError(null);
      setChunks([]);

      try {
        // Set up listeners first
        unlistenChunk = await TauriEvents.onTraceChunk((data) => {
          setChunks(prev => [...prev, ...data]);
        });

        unlistenComplete = await TauriEvents.onTraceComplete(() => {
          setIsStreaming(false);
        });

        // Start streaming
        await TauriClient.streamTraceData({ trace_id: traceId });
      } catch (err) {
        setError(err instanceof Error ? err : new Error('Stream failed'));
        setIsStreaming(false);
      }
    };

    startStream();

    return () => {
      unlistenChunk?.();
      unlistenComplete?.();
    };
  }, [traceId]);

  return { chunks, isStreaming, error };
}

// ============================================================================
// TELEMETRY HOOK
// ============================================================================

/**
 * Hook to subscribe to real-time telemetry updates
 */
export function useTelemetry(callback: (data: TelemetryUpdate) => void) {
  useEffect(() => {
    if (!isTauriAvailable()) return;

    let unlisten: (() => void) | undefined;

    TauriEvents.onTelemetryUpdate(callback).then(fn => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [callback]);
}
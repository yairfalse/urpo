/**
 * Type-safe Tauri client with automatic error handling and retries
 *
 * This is the ONLY place where we interact with Tauri commands
 * All components should use the hooks in hooks.ts instead of calling this directly
 */

import { invoke } from '@tauri-apps/api/tauri';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
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
  StreamTraceDataParams,
  GetServiceMapParams,
  TauriCommand,
  TauriEvent,
  TauriError,
} from './types';

// ============================================================================
// CONFIGURATION
// ============================================================================

const DEFAULT_RETRY_COUNT = 3;
const DEFAULT_RETRY_DELAY = 1000; // ms
const DEFAULT_TIMEOUT = 10000; // ms

interface RetryConfig {
  maxRetries?: number;
  retryDelay?: number;
  timeout?: number;
  onRetry?: (attempt: number, error: unknown) => void;
}

// ============================================================================
// CORE INVOKE WRAPPER
// ============================================================================

async function invokeWithRetry<T>(
  command: TauriCommand,
  args?: any,
  config: RetryConfig = {}
): Promise<T> {
  const {
    maxRetries = DEFAULT_RETRY_COUNT,
    retryDelay = DEFAULT_RETRY_DELAY,
    timeout = DEFAULT_TIMEOUT,
    onRetry,
  } = config;

  let lastError: unknown;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      // Create a timeout promise
      const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(() => reject(new Error(`Command ${command} timed out after ${timeout}ms`)), timeout);
      });

      // Race between the actual command and the timeout
      const result = await Promise.race([
        invoke<T>(command, args),
        timeoutPromise,
      ]);

      // Record performance metrics
      if (typeof window !== 'undefined' && window.performance) {
        performance.mark(`tauri-${command}-end`);
        try {
          performance.measure(`tauri-${command}`, `tauri-${command}-start`, `tauri-${command}-end`);
        } catch {
          // Ignore if start mark doesn't exist
        }
      }

      return result;
    } catch (error) {
      lastError = error;

      if (attempt < maxRetries) {
        onRetry?.(attempt + 1, error);
        await new Promise(resolve => setTimeout(resolve, retryDelay * Math.pow(2, attempt)));
      }
    }
  }

  throw new TauriError(
    `Failed after ${maxRetries} retries`,
    command,
    lastError
  );
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

export const TauriClient = {
  // Service Metrics
  async getServiceMetrics(): Promise<ServiceMetrics[]> {
    console.log('🔥 TauriClient.getServiceMetrics called');
    performance.mark('tauri-get_service_metrics-start');
    const result = await invokeWithRetry<ServiceMetrics[]>('get_service_metrics');
    console.log('🔥 TauriClient.getServiceMetrics result:', result);
    return result;
  },

  async getServiceMetricsBatch(params: ServiceMetricsBatchParams): Promise<ServiceMetrics[]> {
    console.log('🔥 TauriClient.getServiceMetricsBatch called with:', params);
    performance.mark('tauri-get_service_metrics_batch-start');
    return invokeWithRetry<ServiceMetrics[]>('get_service_metrics_batch', params);
  },

  // Traces
  async listRecentTraces(params: ListTracesParams): Promise<TraceInfo[]> {
    console.log('🔥 TauriClient.listRecentTraces called with:', params);
    performance.mark('tauri-list_recent_traces-start');
    const result = await invokeWithRetry<TraceInfo[]>('list_recent_traces', params);
    console.log('🔥 TauriClient.listRecentTraces result:', result);
    return result;
  },

  async getErrorTraces(limit: number): Promise<TraceInfo[]> {
    performance.mark('tauri-get_error_traces-start');
    return invokeWithRetry<TraceInfo[]>('get_error_traces', { limit });
  },

  async getTraceSpans(params: GetTraceSpansParams): Promise<SpanData[]> {
    performance.mark('tauri-get_trace_spans-start');
    return invokeWithRetry<SpanData[]>('get_trace_spans', params);
  },

  async searchTraces(params: SearchTracesParams): Promise<TraceInfo[]> {
    performance.mark('tauri-search_traces-start');
    return invokeWithRetry<TraceInfo[]>('search_traces', params);
  },

  // System
  async getSystemMetrics(): Promise<SystemMetrics> {
    performance.mark('tauri-get_system_metrics-start');
    return invokeWithRetry<SystemMetrics>('get_system_metrics');
  },

  async getStorageInfo(): Promise<StorageInfo> {
    performance.mark('tauri-get_storage_info-start');
    return invokeWithRetry<StorageInfo>('get_storage_info');
  },

  async triggerTierMigration(): Promise<string> {
    performance.mark('tauri-trigger_tier_migration-start');
    return invokeWithRetry<string>('trigger_tier_migration');
  },

  // Service Map
  async getServiceMap(params: GetServiceMapParams = {}): Promise<ServiceMap> {
    performance.mark('tauri-get_service_map-start');
    return invokeWithRetry<ServiceMap>('get_service_map', params);
  },

  // Receiver Control
  async startReceiver(): Promise<void> {
    performance.mark('tauri-start_receiver-start');
    return invokeWithRetry<void>('start_receiver', undefined, {
      maxRetries: 1, // Don't retry receiver start
    });
  },

  async stopReceiver(): Promise<void> {
    performance.mark('tauri-stop_receiver-start');
    return invokeWithRetry<void>('stop_receiver', undefined, {
      maxRetries: 1, // Don't retry receiver stop
    });
  },

  // Streaming
  async streamTraceData(params: StreamTraceDataParams): Promise<void> {
    performance.mark('tauri-stream_trace_data-start');
    return invokeWithRetry<void>('stream_trace_data', params, {
      timeout: 60000, // Longer timeout for streaming
    });
  },
};

// ============================================================================
// EVENT LISTENERS
// ============================================================================

export const TauriEvents = {
  /**
   * Listen to telemetry updates from the backend
   */
  onTelemetryUpdate(callback: (data: TelemetryUpdate) => void): Promise<UnlistenFn> {
    return listen<TelemetryUpdate>('telemetry-update', event => {
      callback(event.payload);
    });
  },

  /**
   * Listen to trace data chunks for streaming large traces
   */
  onTraceChunk(callback: (data: SpanData[]) => void): Promise<UnlistenFn> {
    return listen<SpanData[]>('trace-chunk', event => {
      callback(event.payload);
    });
  },

  /**
   * Listen to trace streaming completion
   */
  onTraceComplete(callback: () => void): Promise<UnlistenFn> {
    return listen('trace-complete', () => {
      callback();
    });
  },
};

// ============================================================================
// HEALTH CHECK
// ============================================================================

/**
 * Check if Tauri is available and responsive
 */
export async function checkTauriHealth(): Promise<boolean> {
  try {
    const metrics = await TauriClient.getSystemMetrics();
    return metrics.uptime_seconds > 0;
  } catch {
    return false;
  }
}

/**
 * Check if we're running in Tauri context
 */
export function isTauriAvailable(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
/**
 * Tauri Integration Library
 *
 * This is the single entry point for all Tauri-related functionality.
 * Import everything you need from this file instead of individual modules.
 *
 * Example usage:
 * ```typescript
 * import { useServiceMetrics, useTraces, TauriClient, useAppStore } from '@/lib/tauri';
 * ```
 */

// ============================================================================
// TYPE EXPORTS
// ============================================================================

export type {
  ServiceMetrics,
  TraceInfo,
  SystemMetrics,
  StorageInfo,
  ServiceMap,
  ServiceNode,
  ServiceEdge,
  SpanData,
  TelemetryUpdate,
  TierHealthInfo,
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
// CLIENT EXPORTS
// ============================================================================

export {
  TauriClient,
  TauriEvents,
  checkTauriHealth,
  isTauriAvailable,
} from './client';

// ============================================================================
// REACT HOOKS EXPORTS
// ============================================================================

export {
  // Query Keys (for advanced usage)
  queryKeys,

  // Service Metrics Hooks
  useServiceMetrics,
  useServiceMetricsBatch,

  // Trace Hooks
  useRecentTraces,
  useErrorTraces,
  useTraceSpans,
  useSearchTraces,

  // System Hooks
  useSystemMetrics,
  useStorageInfo,

  // Service Map Hooks
  useServiceMap,

  // Mutation Hooks
  useStartReceiver,
  useStopReceiver,
  useTriggerTierMigration,

  // Streaming/Event Hooks
  useTraceStream,
  useTelemetry,
} from './hooks';

// ============================================================================
// STORE EXPORTS
// ============================================================================

export {
  useAppStore,
  type ViewMode,
  type ServiceMapViewMode,
  type ColorScheme,
} from './store';

// ============================================================================
// UTILITIES
// ============================================================================

/**
 * Re-export commonly used React Query utilities
 */
export { useQueryClient } from '@tanstack/react-query';

// ============================================================================
// CONVENIENCE HOOKS
// ============================================================================

export { useDashboardData } from './convenience';


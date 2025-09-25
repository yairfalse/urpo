/**
 * Auto-generated types from Rust backend
 * These match the exact serialization format from src-tauri/src/main.rs
 *
 * IMPORTANT: These types are the source of truth for frontend-backend communication
 * DO NOT modify manually - regenerate from Rust structs
 */

// ============================================================================
// CORE DOMAIN TYPES
// ============================================================================

export interface ServiceMetrics {
  name: string;
  request_rate: number;
  error_rate: number;
  latency_p50: number; // milliseconds
  latency_p95: number; // milliseconds
  latency_p99: number; // milliseconds
  active_spans: number;
}

export interface TraceInfo {
  trace_id: string;
  root_service: string;
  root_operation: string;
  start_time: number; // Unix timestamp in milliseconds
  duration: number; // milliseconds
  span_count: number;
  has_error: boolean;
  services: string[];
}

export interface SystemMetrics {
  memory_usage_mb: number;
  cpu_usage_percent: number;
  spans_per_second: number;
  total_spans: number;
  uptime_seconds: number;

  // Advanced performance metrics
  heap_usage_mb: number;
  memory_pressure: number; // 0.0-1.0 scale
  cold_fetch_latency_ms: number;
  command_latencies: Record<string, number>;
  free_space_mb: number;
  tier_health: TierHealthInfo[];
}

export interface TierHealthInfo {
  tier: string;
  status: string;
  health_score: number; // 0.0-1.0 where 1.0 is perfect health
}

export interface StorageInfo {
  mode: string;
  persistent_enabled: boolean;
  data_dir: string;
  hot_size: number;
  warm_size_mb: number;
  cold_retention_hours: number;
  total_spans: number;
  memory_mb: number;
  health: string;
}

// ============================================================================
// SERVICE MAP TYPES
// ============================================================================

export interface ServiceNode {
  name: string;
  request_count: number;
  error_rate: number;
  avg_latency_us: number; // microseconds
  is_root: boolean;
  is_leaf: boolean;
  tier: number;
}

export interface ServiceEdge {
  from: string;
  to: string;
  call_count: number;
  error_count: number;
  avg_latency_us: number; // microseconds
  p99_latency_us: number; // microseconds
  operations: string[];
}

export interface ServiceMap {
  nodes: ServiceNode[];
  edges: ServiceEdge[];
  generated_at: number; // Unix timestamp
  trace_count: number;
  time_window_seconds: number;
}

// ============================================================================
// SPAN TYPES
// ============================================================================

export interface SpanData {
  span_id: string;
  trace_id: string;
  parent_span_id?: string;
  service_name: string;
  operation_name: string;
  start_time: number; // Unix timestamp in nanoseconds
  duration: number; // nanoseconds
  status: 'ok' | 'error';
  error_message?: string;
  attributes: Record<string, any>;
  tags: Record<string, string>;
}

// ============================================================================
// TELEMETRY EVENTS
// ============================================================================

export interface TelemetryUpdate {
  heap_usage_mb: number;
  cpu_usage_percent: number;
  memory_pressure: number;
  cold_fetch_latency_ms: number;
  free_space_mb: number;
  command_latencies: Record<string, number>;
  tier_status: Record<string, string>;
  timestamp: number;
}

// ============================================================================
// COMMAND PARAMETERS
// ============================================================================

export interface ListTracesParams {
  limit: number;
  service_filter?: string;
}

export interface SearchTracesParams {
  query: string;
  limit: number;
}

export interface GetTraceSpansParams {
  trace_id: string;
}

export interface ServiceMetricsBatchParams {
  service_names: string[];
}

export interface StreamTraceDataParams {
  trace_id: string;
}

export interface GetServiceMapParams {
  limit?: number;
  time_window_seconds?: number;
}

// ============================================================================
// ERROR TYPES
// ============================================================================

export class TauriError extends Error {
  constructor(
    message: string,
    public readonly command: string,
    public readonly originalError?: unknown
  ) {
    super(`[${command}] ${message}`);
    this.name = 'TauriError';
  }
}

// ============================================================================
// UTILITY TYPES
// ============================================================================

export type TauriCommand =
  | 'get_service_metrics'
  | 'get_service_metrics_batch'
  | 'list_recent_traces'
  | 'get_error_traces'
  | 'get_trace_spans'
  | 'search_traces'
  | 'get_system_metrics'
  | 'stream_trace_data'
  | 'start_receiver'
  | 'stop_receiver'
  | 'get_storage_info'
  | 'trigger_tier_migration'
  | 'get_service_map';

export type TauriEvent =
  | 'telemetry-update'
  | 'trace-chunk'
  | 'trace-complete';
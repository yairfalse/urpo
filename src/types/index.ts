// Type definitions for Urpo frontend
// These are optimized for performance - no unnecessary fields like Jaeger

export interface ServiceMetrics {
  name: string;
  request_rate: number;
  error_rate: number;
  latency_p50: number;
  latency_p95: number;
  latency_p99: number;
  active_spans: number;
}

export interface TraceInfo {
  trace_id: string;
  root_service: string;
  root_operation: string;
  start_time: number;
  duration: number;
  span_count: number;
  has_error: boolean;
  services: string[];
}

export interface SpanData {
  span_id: string;
  trace_id: string;
  parent_span_id?: string;
  service_name: string;
  operation_name: string;
  start_time: number;
  duration: number;
  status: 'ok' | 'error';
  error_message?: string;
  attributes: Record<string, string>;
  tags: Record<string, string>;
}

export interface SystemMetrics {
  memory_usage_mb: number;
  cpu_usage_percent: number;
  spans_per_second: number;
  total_spans: number;
  uptime_seconds: number;
}
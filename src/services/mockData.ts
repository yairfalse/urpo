/**
 * Mock Data Service
 * Provides sample data when Tauri backend is not available
 */

import { ServiceMetrics, TraceInfo, SystemMetrics } from '../types';

// Mock service metrics data
export const mockServices: ServiceMetrics[] = [
  {
    name: 'payment-service',
    request_rate: 150.5,
    error_rate: 0.02,
    latency_p50: 45.2,
    latency_p95: 127.8,
    latency_p99: 234.1,
    span_count: 15420,
    error_count: 23,
    last_seen: Date.now() - 1000, // 1 second ago
    status: 'healthy',
    dependencies: ['user-service', 'billing-service'],
    version: '2.1.0'
  },
  {
    name: 'user-service',
    request_rate: 89.3,
    error_rate: 0.001,
    latency_p50: 23.1,
    latency_p95: 67.4,
    latency_p99: 145.2,
    span_count: 8934,
    error_count: 1,
    last_seen: Date.now() - 500,
    status: 'healthy',
    dependencies: ['auth-service'],
    version: '1.8.3'
  },
  {
    name: 'billing-service',
    request_rate: 34.7,
    error_rate: 0.05,
    latency_p50: 78.9,
    latency_p95: 189.5,
    latency_p99: 456.7,
    span_count: 3478,
    error_count: 174,
    last_seen: Date.now() - 2000,
    status: 'degraded',
    dependencies: ['payment-gateway'],
    version: '3.0.1'
  },
  {
    name: 'auth-service',
    request_rate: 234.8,
    error_rate: 0.008,
    latency_p50: 12.3,
    latency_p95: 34.5,
    latency_p99: 78.9,
    span_count: 23480,
    error_count: 188,
    last_seen: Date.now() - 800,
    status: 'healthy',
    dependencies: [],
    version: '4.2.1'
  }
];

// Mock trace data
export const mockTraces: TraceInfo[] = [
  {
    trace_id: 'trace_1a2b3c4d5e6f7890',
    root_span_name: 'POST /api/payment/process',
    service_name: 'payment-service',
    start_time: Date.now() - 60000, // 1 minute ago
    duration_ms: 234.5,
    span_count: 12,
    error_count: 0,
    status: 'ok'
  },
  {
    trace_id: 'trace_9z8y7x6w5v4u3210',
    root_span_name: 'GET /api/user/profile',
    service_name: 'user-service',
    start_time: Date.now() - 45000,
    duration_ms: 67.8,
    span_count: 8,
    error_count: 0,
    status: 'ok'
  },
  {
    trace_id: 'trace_error_abc123def456',
    root_span_name: 'POST /api/billing/invoice',
    service_name: 'billing-service',
    start_time: Date.now() - 30000,
    duration_ms: 1234.5,
    span_count: 15,
    error_count: 3,
    status: 'error'
  },
  {
    trace_id: 'trace_slow_xyz789uvw123',
    root_span_name: 'GET /api/analytics/report',
    service_name: 'analytics-service',
    start_time: Date.now() - 120000,
    duration_ms: 5678.9,
    span_count: 45,
    error_count: 0,
    status: 'ok'
  }
];

// Mock system metrics
export const mockSystemMetrics: SystemMetrics = {
  memory_usage_mb: 42.3,
  cpu_usage_percent: 15.7,
  spans_per_second: 847.2,
  total_spans: 156789,
  uptime_seconds: 3245 // ~54 minutes
};

// Simulate real-time updates with slight variations
export const getUpdatedMockServices = (): ServiceMetrics[] => {
  return mockServices.map(service => ({
    ...service,
    request_rate: service.request_rate + (Math.random() - 0.5) * 10,
    error_rate: Math.max(0, service.error_rate + (Math.random() - 0.5) * 0.01),
    latency_p50: service.latency_p50 + (Math.random() - 0.5) * 5,
    latency_p95: service.latency_p95 + (Math.random() - 0.5) * 10,
    latency_p99: service.latency_p99 + (Math.random() - 0.5) * 20,
    span_count: service.span_count + Math.floor(Math.random() * 100),
    last_seen: Date.now() - Math.floor(Math.random() * 5000)
  }));
};

export const getUpdatedMockSystemMetrics = (): SystemMetrics => {
  return {
    memory_usage_mb: Math.max(20, mockSystemMetrics.memory_usage_mb + (Math.random() - 0.5) * 5),
    cpu_usage_percent: Math.max(0, Math.min(100, mockSystemMetrics.cpu_usage_percent + (Math.random() - 0.5) * 10)),
    spans_per_second: Math.max(0, mockSystemMetrics.spans_per_second + (Math.random() - 0.5) * 100),
    total_spans: mockSystemMetrics.total_spans + Math.floor(Math.random() * 1000),
    uptime_seconds: mockSystemMetrics.uptime_seconds + 1
  };
};
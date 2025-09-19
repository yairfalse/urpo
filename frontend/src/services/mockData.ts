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
    active_spans: 15420
  },
  {
    name: 'user-service',
    request_rate: 89.3,
    error_rate: 0.001,
    latency_p50: 23.1,
    latency_p95: 67.4,
    latency_p99: 145.2,
    active_spans: 8934
  },
  {
    name: 'billing-service',
    request_rate: 34.7,
    error_rate: 0.05,
    latency_p50: 78.9,
    latency_p95: 189.5,
    latency_p99: 456.7,
    active_spans: 3478
  },
  {
    name: 'auth-service',
    request_rate: 234.8,
    error_rate: 0.008,
    latency_p50: 12.3,
    latency_p95: 34.5,
    latency_p99: 78.9,
    active_spans: 23480
  }
];

// Mock trace data
export const mockTraces: TraceInfo[] = [
  {
    trace_id: 'trace_1a2b3c4d5e6f7890',
    root_service: 'payment-service',
    root_operation: 'POST /api/payment/process',
    start_time: Math.floor((Date.now() - 60000) / 1000), // 1 minute ago in seconds
    duration: 234.5,
    span_count: 12,
    has_error: false,
    services: ['payment-service', 'user-service', 'billing-service']
  },
  {
    trace_id: 'trace_9z8y7x6w5v4u3210',
    root_service: 'user-service',
    root_operation: 'GET /api/user/profile',
    start_time: Math.floor((Date.now() - 45000) / 1000),
    duration: 67.8,
    span_count: 8,
    has_error: false,
    services: ['user-service', 'auth-service']
  },
  {
    trace_id: 'trace_error_abc123def456',
    root_service: 'billing-service',
    root_operation: 'POST /api/billing/invoice',
    start_time: Math.floor((Date.now() - 30000) / 1000),
    duration: 1234.5,
    span_count: 15,
    has_error: true,
    services: ['billing-service', 'payment-gateway', 'notification-service']
  },
  {
    trace_id: 'trace_slow_xyz789uvw123',
    root_service: 'analytics-service',
    root_operation: 'GET /api/analytics/report',
    start_time: Math.floor((Date.now() - 120000) / 1000),
    duration: 5678.9,
    span_count: 45,
    has_error: false,
    services: ['analytics-service', 'database-service', 'cache-service']
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
    active_spans: service.active_spans + Math.floor(Math.random() * 100)
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
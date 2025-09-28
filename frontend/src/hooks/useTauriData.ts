import { useState, useCallback } from 'react';
import { isTauriAvailable, safeTauriInvoke } from '../utils/tauri';
import { ServiceMetrics, TraceInfo, SystemMetrics } from '../types';
import { getUpdatedMockServices, getUpdatedMockSystemMetrics, mockTraces } from '../services/mockData';

/**
 * Custom hook for managing Tauri/mock data operations
 */
export function useTauriData() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Fetch services and system metrics
  const fetchMetrics = useCallback(async () => {
    try {
      setError(null);
      
      let services: ServiceMetrics[];
      let systemMetrics: SystemMetrics | null;

      if (isTauriAvailable()) {
        // Fetch from Tauri backend
        const [servicesResult, metricsResult] = await Promise.all([
          safeTauriInvoke<ServiceMetrics[]>('get_service_metrics'),
          safeTauriInvoke<SystemMetrics>('get_system_metrics')
        ]);

        services = servicesResult || [];
        systemMetrics = metricsResult;
      } else {
        // Use mock data
        services = getUpdatedMockServices();
        systemMetrics = getUpdatedMockSystemMetrics();
      }

      return { services, systemMetrics };
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to fetch metrics';
      setError(errorMessage);
      
      // Fallback to mock data on error
      return {
        services: getUpdatedMockServices(),
        systemMetrics: getUpdatedMockSystemMetrics()
      };
    }
  }, []);

  // Fetch traces
  const fetchTraces = useCallback(async (limit = 100): Promise<TraceInfo[]> => {
    try {
      setError(null);

      if (isTauriAvailable()) {
        const traces = await safeTauriInvoke<TraceInfo[]>('list_recent_traces', { limit });
        return traces || [];
      } else {
        // Return mock traces
        return mockTraces.slice(0, limit);
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to fetch traces';
      setError(errorMessage);
      
      // Fallback to mock data
      return mockTraces.slice(0, limit);
    }
  }, []);

  // Start OTEL receiver
  const startReceiver = useCallback(async () => {
    if (isTauriAvailable()) {
      try {
        await safeTauriInvoke('start_receiver');
      } catch (err) {
        console.error('Failed to start receiver:', err);
        setError('Failed to start OTEL receiver');
      }
    }
  }, []);

  // Stop OTEL receiver
  const stopReceiver = useCallback(async () => {
    if (isTauriAvailable()) {
      try {
        await safeTauriInvoke('stop_receiver');
      } catch (err) {
        console.error('Failed to stop receiver:', err);
        setError('Failed to stop OTEL receiver');
      }
    }
  }, []);

  // Search traces
  const searchTraces = useCallback(async (query: string, limit = 100): Promise<TraceInfo[]> => {
    try {
      setError(null);

      if (isTauriAvailable()) {
        const traces = await safeTauriInvoke<TraceInfo[]>('search_traces', { query, limit });
        return traces || [];
      } else {
        // Return mock traces
        return mockTraces.filter(trace => trace.root_operation.includes(query)).slice(0, limit);
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to search traces';
      setError(errorMessage);
      
      // Fallback to mock data
      return mockTraces.filter(trace => trace.root_operation.includes(query)).slice(0, limit);
    }
  }, []);

  return {
    loading,
    setLoading,
    error,
    setError,
    fetchMetrics,
    fetchTraces,
    searchTraces,
    startReceiver,
    stopReceiver,
    isTauriMode: isTauriAvailable()
  };
}
/**
 * Tauri integration hooks and utilities
 */

import { useState, useEffect, useCallback } from 'react';
import { useTauriData } from '../hooks/useTauriData';
import { ServiceMetrics, TraceInfo, SystemMetrics } from '../types';

/**
 * Hook for dashboard data with auto-refresh
 */
export function useDashboardData() {
  const [serviceMetrics, setServiceMetrics] = useState<ServiceMetrics[]>([]);
  const [systemMetrics, setSystemMetrics] = useState<SystemMetrics | null>(null);
  const [recentTraces, setRecentTraces] = useState<TraceInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [hasError, setHasError] = useState(false);

  const { fetchMetrics, fetchTraces, error, isTauriMode } = useTauriData();

  const loadData = useCallback(async () => {
    try {
      setIsLoading(true);
      setHasError(false);

      // Fetch all data in parallel
      const [metricsData, tracesData] = await Promise.all([
        fetchMetrics(),
        fetchTraces(100)
      ]);

      setServiceMetrics(metricsData.services);
      setSystemMetrics(metricsData.systemMetrics);
      setRecentTraces(tracesData);
    } catch (err) {
      console.error('Failed to load dashboard data:', err);
      setHasError(true);
    } finally {
      setIsLoading(false);
    }
  }, [fetchMetrics, fetchTraces]);

  useEffect(() => {
    loadData();

    // Auto-refresh every 5 seconds if in Tauri mode
    if (isTauriMode) {
      const interval = setInterval(loadData, 5000);
      return () => clearInterval(interval);
    }
  }, [loadData, isTauriMode]);

  return {
    serviceMetrics,
    systemMetrics,
    recentTraces,
    isLoading,
    hasError,
    refetchAll: loadData,
    isTauriMode
  };
}

/**
 * Hook to start the OTEL receiver on mount
 */
export function useStartReceiver({ onError }: { onError?: (error: any) => void } = {}) {
  const { startReceiver, isTauriMode } = useTauriData();

  useEffect(() => {
    if (isTauriMode) {
      startReceiver().catch(err => {
        console.log('OTEL receiver not available in browser mode');
        onError?.(err);
      });
    }
  }, [isTauriMode, startReceiver, onError]);

  return { startReceiver, isTauriMode };
}


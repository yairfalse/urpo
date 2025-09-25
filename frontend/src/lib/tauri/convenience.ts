/**
 * Convenience hooks that combine multiple hooks for common use cases
 */

import { useServiceMetrics, useSystemMetrics, useRecentTraces, useStorageInfo } from './hooks';

/**
 * Combined hook for dashboard data
 * Returns all essential data for the main dashboard
 */
export const useDashboardData = () => {
  const serviceMetrics = useServiceMetrics();
  const systemMetrics = useSystemMetrics();
  const recentTraces = useRecentTraces({ limit: 50 });
  const storageInfo = useStorageInfo();

  return {
    serviceMetrics,
    systemMetrics,
    recentTraces,
    storageInfo,
    isLoading: serviceMetrics.isLoading || systemMetrics.isLoading || recentTraces.isLoading,
    hasError: serviceMetrics.isError || systemMetrics.isError || recentTraces.isError,
    refetchAll: () => {
      serviceMetrics.refetch();
      systemMetrics.refetch();
      recentTraces.refetch();
      storageInfo.refetch();
    },
  };
};
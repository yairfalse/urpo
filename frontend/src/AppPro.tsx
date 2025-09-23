import { useState, useEffect, useCallback, memo } from 'react';
import { ProfessionalLayout } from './components/layout/ProfessionalLayout';
import { ProSidebar } from './components/layout/ProSidebar';
import { ServiceGraphPro } from './components/charts/ServiceGraphPro';
import { TraceWaterfall } from './components/charts/TraceWaterfall';
import { ServiceHealthDashboard } from './components/tables/ServiceHealthDashboard';
import { TraceExplorer } from './components/tables/TraceExplorer';
import { FlowTable } from './components/tables/FlowTable';
import { VirtualizedFlowTable } from './components/tables/VirtualizedFlowTable';
import { ServiceMetrics, TraceInfo, SystemMetrics as SystemMetricsType } from './types';
import { isTauriAvailable, safeTauriInvoke } from './utils/tauri';
import { POLLING } from './constants/ui';

// Professional metric cards with gradients
const MetricCard = memo(({
  label,
  value,
  change,
  color = 'blue'
}: {
  label: string;
  value: string | number;
  change?: string;
  color?: 'blue' | 'cyan' | 'purple' | 'green' | 'orange' | 'pink';
}) => {
  const colorMap = {
    blue: 'from-data-blue/20 to-data-blue/5 border-data-blue/30',
    cyan: 'from-data-cyan/20 to-data-cyan/5 border-data-cyan/30',
    purple: 'from-data-purple/20 to-data-purple/5 border-data-purple/30',
    green: 'from-semantic-success/20 to-semantic-success/5 border-semantic-success/30',
    orange: 'from-data-orange/20 to-data-orange/5 border-data-orange/30',
    pink: 'from-data-pink/20 to-data-pink/5 border-data-pink/30',
  };

  return (
    <div className={`
      metric-card bg-gradient-to-br ${colorMap[color]}
      relative overflow-hidden
    `}>
      <div className="relative z-10">
        <p className="text-xs text-light-500 uppercase tracking-wider mb-1">{label}</p>
        <div className="flex items-baseline gap-2">
          <span className="text-2xl font-bold text-light-100">{value}</span>
          {change && (
            <span className={`text-xs font-medium ${change.startsWith('+') ? 'text-semantic-success' : 'text-semantic-error'}`}>
              {change}
            </span>
          )}
        </div>
      </div>
      {/* Animated background pattern */}
      <div className="absolute inset-0 opacity-10">
        <div className="absolute -right-4 -top-4 w-24 h-24 bg-gradient-to-br from-white/10 to-transparent rounded-full blur-xl animate-pulse"></div>
        <div className="absolute -left-4 -bottom-4 w-32 h-32 bg-gradient-to-tr from-white/5 to-transparent rounded-full blur-2xl"></div>
      </div>
    </div>
  );
});

MetricCard.displayName = 'MetricCard';

const AppPro = memo(() => {
  const [activeView, setActiveView] = useState<string>('graph');
  const [services, setServices] = useState<ServiceMetrics[]>([]);
  const [traces, setTraces] = useState<TraceInfo[]>([]);
  const [systemMetrics, setSystemMetrics] = useState<SystemMetricsType | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const updateMetrics = useCallback(async () => {
    try {
      if (isTauriAvailable()) {
        const [serviceData, systemData, traceData] = await Promise.all([
          safeTauriInvoke<ServiceMetrics[]>('get_service_metrics'),
          safeTauriInvoke<SystemMetricsType>('get_system_metrics'),
          safeTauriInvoke<TraceInfo[]>('list_recent_traces', { limit: 100 }),
        ]);

        if (serviceData && systemData) {
          requestAnimationFrame(() => {
            setServices(serviceData);
            setSystemMetrics(systemData);
            setTraces(traceData || []);
            setError(null);
          });
        }
      } else {
        // Demo data for development
        requestAnimationFrame(() => {
          setServices([
            { service_name: 'api-gateway', trace_count: 1234, error_count: 12, avg_duration_ms: 45.2, p95_duration_ms: 89.5, p99_duration_ms: 145.8, spans_per_second: 234.5 },
            { service_name: 'auth-service', trace_count: 856, error_count: 3, avg_duration_ms: 12.4, p95_duration_ms: 28.3, p99_duration_ms: 42.1, spans_per_second: 156.2 },
            { service_name: 'payment-processor', trace_count: 423, error_count: 0, avg_duration_ms: 234.5, p95_duration_ms: 456.7, p99_duration_ms: 678.9, spans_per_second: 78.9 },
            { service_name: 'notification-service', trace_count: 2145, error_count: 45, avg_duration_ms: 8.9, p95_duration_ms: 15.2, p99_duration_ms: 22.4, spans_per_second: 345.6 },
            { service_name: 'user-profile', trace_count: 567, error_count: 2, avg_duration_ms: 34.5, p95_duration_ms: 67.8, p99_duration_ms: 98.7, spans_per_second: 89.3 },
          ]);
          setSystemMetrics({
            total_spans: 45678,
            spans_per_second: 1234.5,
            memory_usage_mb: 234.5,
            cpu_usage_percent: 12.3,
            storage_health: 'healthy',
            uptime_seconds: 3600,
            active_connections: 42,
            error_rate: 0.023,
          });
          setError(null);
        });
      }
    } catch (err) {
      console.error('Error updating metrics:', err);
      setError(`Failed to connect to backend`);
    }
  }, []);

  useEffect(() => {
    const init = async () => {
      if (isTauriAvailable()) {
        try {
          await safeTauriInvoke('start_receiver');
        } catch (err) {
          console.error('Failed to start receiver:', err);
        }
      }
      setLoading(false);
    };

    init();
    return () => {
      if (isTauriAvailable()) {
        safeTauriInvoke('stop_receiver').catch(console.error);
      }
    };
  }, []);

  useEffect(() => {
    if (!loading) {
      updateMetrics();
      const interval = setInterval(updateMetrics, POLLING.METRICS_INTERVAL_MS);
      return () => clearInterval(interval);
    }
  }, [loading, updateMetrics]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen bg-gradient-to-br from-dark-0 via-dark-50 to-dark-0">
        <div className="text-center">
          <div className="relative">
            <div className="w-16 h-16 mx-auto rounded-full border-2 border-data-blue/30 animate-spin"></div>
            <div className="absolute inset-0 w-16 h-16 mx-auto rounded-full border-2 border-t-data-cyan border-r-transparent border-b-transparent border-l-transparent animate-spin"></div>
          </div>
          <h2 className="mt-6 text-xl font-bold bg-gradient-to-r from-light-50 to-light-200 bg-clip-text text-transparent">
            Initializing URPO
          </h2>
          <p className="mt-2 text-sm text-light-500">Connecting to observability pipeline...</p>
        </div>
      </div>
    );
  }

  const renderContent = () => {
    switch (activeView) {
      case 'graph':
        return (
          <div className="p-6 h-full">
            <div className="mb-6">
              <h2 className="text-2xl font-bold text-light-100 mb-2">Service Dependency Map</h2>
              <p className="text-light-400">Real-time service interactions and health status</p>
            </div>
            <div className="h-[calc(100%-80px)]">
              <ServiceGraphPro services={services} traces={traces} />
            </div>
          </div>
        );

      case 'flows':
        return (
          <div className="p-6 h-full">
            <div className="mb-6">
              <h2 className="text-2xl font-bold text-light-100 mb-2">Trace Flow Analysis</h2>
              <p className="text-light-400">Detailed request flow through your services</p>
            </div>
            <div className="h-[calc(100%-80px)]">
              {traces.length > 100 ? (
                <VirtualizedFlowTable traces={traces} onRefresh={updateMetrics} />
              ) : (
                <FlowTable traces={traces} onRefresh={updateMetrics} />
              )}
            </div>
          </div>
        );

      case 'traces':
        return (
          <div className="p-6 h-full">
            <div className="mb-6">
              <h2 className="text-2xl font-bold text-light-100 mb-2">Trace Explorer</h2>
              <p className="text-light-400">Search and analyze distributed traces</p>
            </div>
            <div className="grid grid-cols-3 gap-4 mb-6">
              <MetricCard
                label="Total Traces"
                value={traces.length.toLocaleString()}
                color="blue"
              />
              <MetricCard
                label="Error Rate"
                value={`${((traces.filter(t => t.has_error).length / traces.length) * 100).toFixed(2)}%`}
                color="orange"
                change="-12%"
              />
              <MetricCard
                label="Avg Duration"
                value={`${(traces.reduce((acc, t) => acc + t.duration, 0) / traces.length / 1000000).toFixed(2)}ms`}
                color="cyan"
                change="+5%"
              />
            </div>
            <div className="h-[calc(100%-200px)]">
              <TraceExplorer traces={traces} onRefresh={updateMetrics} />
            </div>
          </div>
        );

      case 'health':
        return (
          <div className="p-6">
            <div className="mb-6">
              <h2 className="text-2xl font-bold text-light-100 mb-2">Service Health Matrix</h2>
              <p className="text-light-400">Comprehensive health metrics across all services</p>
            </div>

            {/* Metric cards grid */}
            <div className="grid grid-cols-4 gap-4 mb-6">
              <MetricCard
                label="Healthy Services"
                value={services.filter(s => s.error_count === 0).length}
                color="green"
              />
              <MetricCard
                label="Total Requests"
                value={services.reduce((acc, s) => acc + s.trace_count, 0).toLocaleString()}
                color="blue"
                change="+23%"
              />
              <MetricCard
                label="P95 Latency"
                value={`${Math.max(...services.map(s => s.p95_duration_ms)).toFixed(1)}ms`}
                color="purple"
                change="-8%"
              />
              <MetricCard
                label="Error Count"
                value={services.reduce((acc, s) => acc + s.error_count, 0)}
                color="orange"
                change={services.reduce((acc, s) => acc + s.error_count, 0) > 0 ? '+2' : '0'}
              />
            </div>

            <ServiceHealthDashboard services={services} />
          </div>
        );

      case 'latency':
        return (
          <div className="p-6">
            <div className="mb-6">
              <h2 className="text-2xl font-bold text-light-100 mb-2">Latency Analysis</h2>
              <p className="text-light-400">Performance metrics and bottleneck detection</p>
            </div>
            <div className="grid grid-cols-2 gap-6">
              <div className="chart-container h-96">
                <h3 className="text-sm font-semibold text-light-300 mb-4">Latency Distribution</h3>
                {/* Add latency histogram chart here */}
              </div>
              <div className="chart-container h-96">
                <h3 className="text-sm font-semibold text-light-300 mb-4">Service Comparison</h3>
                {/* Add service comparison chart here */}
              </div>
            </div>
          </div>
        );

      default:
        return (
          <div className="p-6">
            <div className="text-center py-12">
              <p className="text-light-400">View under construction</p>
            </div>
          </div>
        );
    }
  };

  return (
    <ProfessionalLayout
      sidebar={<ProSidebar activeView={activeView} onViewChange={setActiveView} />}
    >
      {error && (
        <div className="px-6 py-3 bg-semantic-error/10 border-b border-semantic-error/30">
          <p className="text-sm text-semantic-error">{error}</p>
        </div>
      )}

      {renderContent()}
    </ProfessionalLayout>
  );
});

AppPro.displayName = 'AppPro';

export default AppPro;
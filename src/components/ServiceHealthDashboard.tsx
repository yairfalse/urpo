import { memo, useMemo } from 'react';
import { ServiceMetrics } from '../types';

interface Props {
  services: ServiceMetrics[];
}

// PERFORMANCE: Memoize component to prevent re-renders
const ServiceHealthDashboard = memo(({ services }: Props) => {
  // PERFORMANCE: Memoize sorted services
  const sortedServices = useMemo(() => {
    return [...services].sort((a, b) => b.request_rate - a.request_rate);
  }, [services]);

  // Calculate health status efficiently
  const getHealthStatus = (errorRate: number) => {
    if (errorRate === 0) return { color: 'text-green-500', bg: 'bg-green-500/10', label: 'Healthy' };
    if (errorRate < 1) return { color: 'text-yellow-500', bg: 'bg-yellow-500/10', label: 'Warning' };
    return { color: 'text-red-500', bg: 'bg-red-500/10', label: 'Critical' };
  };

  const getLatencyColor = (p99: number) => {
    if (p99 < 100) return 'text-green-400';
    if (p99 < 500) return 'text-yellow-400';
    return 'text-red-400';
  };

  if (services.length === 0) {
    return (
      <div className="bg-slate-900 rounded-lg p-8 text-center">
        <p className="text-slate-500">No services detected yet...</p>
        <p className="text-slate-600 text-sm mt-2">
          Waiting for spans (instant display when available)
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-semibold text-slate-200">
        Service Health Dashboard
      </h2>
      
      {/* Service Grid - Optimized rendering */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {sortedServices.map((service) => {
          const health = getHealthStatus(service.error_rate);
          
          return (
            <div
              key={service.name}
              className="bg-slate-900 rounded-lg p-4 border border-slate-800 hover:border-slate-700 transition-colors gpu-accelerated"
            >
              {/* Service Name & Status */}
              <div className="flex items-center justify-between mb-3">
                <h3 className="font-medium text-slate-200 truncate">
                  {service.name}
                </h3>
                <span className={`text-xs px-2 py-1 rounded ${health.bg} ${health.color}`}>
                  {health.label}
                </span>
              </div>

              {/* Metrics Grid */}
              <div className="grid grid-cols-2 gap-2 text-sm">
                <div>
                  <p className="text-slate-500 text-xs">Request Rate</p>
                  <p className="font-mono text-slate-300">
                    {service.request_rate.toFixed(1)}/s
                  </p>
                </div>
                
                <div>
                  <p className="text-slate-500 text-xs">Error Rate</p>
                  <p className={`font-mono ${health.color}`}>
                    {service.error_rate.toFixed(2)}%
                  </p>
                </div>

                <div>
                  <p className="text-slate-500 text-xs">P50 Latency</p>
                  <p className="font-mono text-slate-300">
                    {service.latency_p50}ms
                  </p>
                </div>

                <div>
                  <p className="text-slate-500 text-xs">P99 Latency</p>
                  <p className={`font-mono ${getLatencyColor(service.latency_p99)}`}>
                    {service.latency_p99}ms
                  </p>
                </div>
              </div>

              {/* Activity Indicator */}
              <div className="mt-3 pt-3 border-t border-slate-800">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-slate-500">
                    Active Spans
                  </span>
                  <span className="text-xs font-mono text-slate-400">
                    {service.active_spans}
                  </span>
                </div>
                
                {/* Visual activity bar */}
                <div className="mt-1 h-1 bg-slate-800 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-green-500 transition-all duration-300"
                    style={{
                      width: `${Math.min((service.active_spans / 100) * 100, 100)}%`,
                    }}
                  />
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Summary Stats - Calculated instantly */}
      <div className="bg-slate-900 rounded-lg p-4 border border-slate-800">
        <div className="grid grid-cols-4 gap-4 text-center">
          <div>
            <p className="text-2xl font-bold text-green-500">
              {services.length}
            </p>
            <p className="text-xs text-slate-500">Services</p>
          </div>
          
          <div>
            <p className="text-2xl font-bold text-slate-300">
              {services.reduce((sum, s) => sum + s.request_rate, 0).toFixed(0)}
            </p>
            <p className="text-xs text-slate-500">Total RPS</p>
          </div>
          
          <div>
            <p className="text-2xl font-bold text-yellow-500">
              {(services.reduce((sum, s) => sum + s.error_rate, 0) / services.length).toFixed(2)}%
            </p>
            <p className="text-xs text-slate-500">Avg Error Rate</p>
          </div>
          
          <div>
            <p className="text-2xl font-bold text-blue-500">
              {Math.max(...services.map(s => s.latency_p99))}ms
            </p>
            <p className="text-xs text-slate-500">Max P99</p>
          </div>
        </div>
      </div>
    </div>
  );
});

ServiceHealthDashboard.displayName = 'ServiceHealthDashboard';

export default ServiceHealthDashboard;
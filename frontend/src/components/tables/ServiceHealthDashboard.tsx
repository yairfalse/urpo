import { memo, useMemo } from 'react';
import { Network } from 'lucide-react';
import { ServiceMetrics } from '../../types';

interface Props {
  services: ServiceMetrics[];
}

// PERFORMANCE: Memoize component to prevent re-renders
const ServiceHealthDashboard = memo(({ services }: Props) => {
  // PERFORMANCE: Memoize sorted services
  const sortedServices = useMemo(() => {
    return [...services].sort((a, b) => b.request_rate - a.request_rate);
  }, [services]);

  // Calculate health status with enterprise precision
  const getHealthStatus = (errorRate: number) => {
    if (errorRate === 0) return { 
      color: 'text-text-700', 
      bg: 'bg-surface-100', 
      border: 'border-surface-400',
      indicator: 'healthy',
      label: 'Healthy' 
    };
    if (errorRate < 1) return { 
      color: 'text-status-warning', 
      bg: 'bg-status-warning bg-opacity-5', 
      border: 'border-status-warning border-opacity-20',
      indicator: 'warning',
      label: 'Warning' 
    };
    return { 
      color: 'text-status-error', 
      bg: 'bg-status-error bg-opacity-5', 
      border: 'border-status-error border-opacity-20',
      indicator: 'critical',
      label: 'Critical' 
    };
  };

  const getLatencyColor = (p99: number) => {
    if (p99 < 100) return 'text-text-700';
    if (p99 < 500) return 'text-status-warning';
    return 'text-status-error';
  };

  const getLatencyIndicator = (p99: number) => {
    if (p99 < 100) return 'healthy';
    if (p99 < 500) return 'warning';
    return 'critical';
  };

  if (services.length === 0) {
    return (
      <div className="clean-card p-8 text-center ">
        <div className="w-12 h-12 mx-auto mb-4 rounded-lg bg-surface-100 flex items-center justify-center">
          <Network className="w-6 h-6 text-text-500" />
        </div>
        <p className="text-text-700 font-medium">No services detected</p>
        <p className="text-text-500 text-xs font-mono mt-2">
          Waiting for OpenTelemetry spans
        </p>
        <div className="mt-4 h-1 bg-surface-200 rounded-full overflow-hidden">
          <div className="h-full bg-text-700  w-2/3"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Professional Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h2 className="text-xl font-display font-bold text-text-900 tracking-tight">
            Service Health Dashboard
          </h2>
          <div className="status-indicator healthy"></div>
          <span className="text-xs text-text-500 font-mono uppercase tracking-wide">
            Real-time Monitoring
          </span>
        </div>
        
        <div className="clean-card px-3 py-1 text-xs font-mono">
          <span className="text-text-500">Services:</span>
          <span className="text-text-900 font-medium ml-1">{services.length}</span>
        </div>
      </div>
      
      {/* Professional Service Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {sortedServices.map((service, index) => {
          const health = getHealthStatus(service.error_rate);
          const latencyIndicator = getLatencyIndicator(service.latency_p99);
          
          return (
            <div
              key={service.name}
              className="clean-card p-4 micro-interaction "
              style={{ animationDelay: `${index * 50}ms` }}
            >
              {/* Service Header with Professional Status */}
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-2 flex-1 min-w-0">
                  <div className={`status-indicator ${health.indicator}`}></div>
                  <h3 className="font-mono font-medium text-text-900 truncate text-sm">
                    {service.name}
                  </h3>
                </div>
                
                <div className={`clean-card px-2 py-0.5 ${health.bg} border-0.5`}>
                  <span className={`text-[10px] font-mono font-medium ${health.color}`}>
                    {health.label}
                  </span>
                </div>
              </div>

              {/* Professional Metrics Grid */}
              <div className="grid grid-cols-2 gap-3 mb-4">
                <div className="space-y-1">
                  <div className="flex items-center gap-1">
                    <span className="text-[10px] text-text-500 font-mono uppercase tracking-wide">RPS</span>
                    <div className="h-0.5 flex-1 bg-surface-300"></div>
                  </div>
                  <div className="font-mono text-text-900 font-medium text-sm">
                    {service.request_rate.toFixed(1)}
                  </div>
                </div>
                
                <div className="space-y-1">
                  <div className="flex items-center gap-1">
                    <span className="text-[10px] text-text-500 font-mono uppercase tracking-wide">ERROR</span>
                    <div className="h-0.5 flex-1 bg-surface-300"></div>
                  </div>
                  <div className={`font-mono font-medium text-sm ${health.color}`}>
                    {service.error_rate.toFixed(2)}%
                  </div>
                </div>

                <div className="space-y-1">
                  <div className="flex items-center gap-1">
                    <span className="text-[10px] text-text-500 font-mono uppercase tracking-wide">P50</span>
                    <div className="h-0.5 flex-1 bg-surface-300"></div>
                  </div>
                  <div className="font-mono text-text-900 font-medium text-sm">
                    {service.latency_p50}ms
                  </div>
                </div>

                <div className="space-y-1">
                  <div className="flex items-center gap-1">
                    <span className="text-[10px] text-text-500 font-mono uppercase tracking-wide">P99</span>
                    <div className={`status-indicator ${latencyIndicator}`}></div>
                  </div>
                  <div className={`font-mono font-medium text-sm ${getLatencyColor(service.latency_p99)}`}>
                    {service.latency_p99}ms
                  </div>
                </div>
              </div>

              {/* Professional Activity Indicator */}
              <div className="space-y-2 pt-3 border-t border-surface-300">
                <div className="flex items-center justify-between">
                  <span className="text-[10px] text-text-500 font-mono uppercase tracking-wide">
                    Active Spans
                  </span>
                  <span className="text-xs font-mono text-text-900 font-medium">
                    {service.active_spans.toLocaleString()}
                  </span>
                </div>
                
                {/* Professional Progress Bar */}
                <div className="h-1 bg-surface-200 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-text-700   ease-out"
                    style={{
                      width: `${Math.min((service.active_spans / 100) * 100, 100)}%`,
                    }}
                  >
                  </div>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Professional Summary Stats */}
      <div className="clean-card p-6 " style={{ animationDelay: '300ms' }}>
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-display font-bold text-text-900">System Overview</h3>
          <div className="status-indicator healthy"></div>
        </div>
        
        <div className="grid grid-cols-4 gap-6">
          <div className="text-center space-y-2">
            <div className="flex items-center justify-center gap-2">
              <div className="status-indicator healthy"></div>
              <span className="text-[10px] text-text-500 font-mono uppercase tracking-wide">Services</span>
            </div>
            <div className="text-2xl font-mono font-bold text-text-900">
              {services.length.toLocaleString()}
            </div>
            <div className="h-0.5 bg-text-700 rounded-full"></div>
          </div>
          
          <div className="text-center space-y-2">
            <div className="flex items-center justify-center gap-2">
              <div className="status-indicator info"></div>
              <span className="text-[10px] text-text-500 font-mono uppercase tracking-wide">Total RPS</span>
            </div>
            <div className="text-2xl font-mono font-bold text-text-900">
              {services.reduce((sum, s) => sum + s.request_rate, 0).toFixed(0)}
            </div>
            <div className="h-0.5 bg-text-700 rounded-full"></div>
          </div>
          
          <div className="text-center space-y-2">
            <div className="flex items-center justify-center gap-2">
              <div className="status-indicator warning"></div>
              <span className="text-[10px] text-text-500 font-mono uppercase tracking-wide">Avg Error</span>
            </div>
            <div className="text-2xl font-mono font-bold text-status-warning">
              {services.length > 0 ? (services.reduce((sum, s) => sum + s.error_rate, 0) / services.length).toFixed(2) : '0.00'}%
            </div>
            <div className="h-0.5 bg-status-warning bg-opacity-30 rounded-full"></div>
          </div>
          
          <div className="text-center space-y-2">
            <div className="flex items-center justify-center gap-2">
              <div className="status-indicator critical"></div>
              <span className="text-[10px] text-text-500 font-mono uppercase tracking-wide">Max P99</span>
            </div>
            <div className="text-2xl font-mono font-bold text-status-error">
              {services.length > 0 ? Math.max(...services.map(s => s.latency_p99)) : 0}ms
            </div>
            <div className="h-0.5 bg-status-error bg-opacity-30 rounded-full"></div>
          </div>
        </div>
        
        {/* Professional Performance Indicator */}
        <div className="mt-6 pt-4 border-t border-surface-300">
          <div className="flex items-center justify-center gap-2 text-xs font-mono">
            <div className="status-indicator info "></div>
            <span className="text-text-500">Real-time data • Sub-second updates</span>
          </div>
        </div>
      </div>
    </div>
  );
});

ServiceHealthDashboard.displayName = 'ServiceHealthDashboard';

export { ServiceHealthDashboard };
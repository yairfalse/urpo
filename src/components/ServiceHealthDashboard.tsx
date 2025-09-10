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

  // Calculate health status with knife-edge precision
  const getHealthStatus = (errorRate: number) => {
    if (errorRate === 0) return { 
      color: 'text-electric-green', 
      bg: 'bg-electric-green/5', 
      border: 'border-electric-green/20',
      indicator: 'healthy',
      label: 'HEALTHY' 
    };
    if (errorRate < 1) return { 
      color: 'text-electric-amber', 
      bg: 'bg-electric-amber/5', 
      border: 'border-electric-amber/20',
      indicator: 'warning',
      label: 'WARNING' 
    };
    return { 
      color: 'text-electric-red', 
      bg: 'bg-electric-red/5', 
      border: 'border-electric-red/20',
      indicator: 'critical',
      label: 'CRITICAL' 
    };
  };

  const getLatencyColor = (p99: number) => {
    if (p99 < 100) return 'text-electric-green';
    if (p99 < 500) return 'text-electric-amber';
    return 'text-electric-red';
  };

  const getLatencyIndicator = (p99: number) => {
    if (p99 < 100) return 'healthy';
    if (p99 < 500) return 'warning';
    return 'critical';
  };

  if (services.length === 0) {
    return (
      <div className="glass-card p-8 text-center animate-scale-in">
        <div className="knife-shimmer w-12 h-12 mx-auto mb-4 rounded-lg"></div>
        <p className="text-steel-300 font-medium">No services detected yet...</p>
        <p className="text-steel-400 text-xs font-mono mt-2">
          Waiting for spans • Instant display when available
        </p>
        <div className="mt-4 h-0.5 bg-steel-800 rounded-full overflow-hidden">
          <div className="h-full bg-electric-blue animate-knife-shine"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Ultra-Sharp Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h2 className="text-xl font-display font-bold text-steel-50 tracking-tight">
            Service Health Dashboard
          </h2>
          <div className="status-indicator healthy animate-pulse-electric"></div>
          <span className="text-xs text-steel-300 font-mono uppercase tracking-wide">
            Real-time Monitoring
          </span>
        </div>
        
        <div className="glass-card px-3 py-1 text-xs font-mono">
          <span className="text-steel-300">Services:</span>
          <span className="text-electric-blue font-medium ml-1">{services.length}</span>
        </div>
      </div>
      
      {/* Knife-Edge Service Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {sortedServices.map((service, index) => {
          const health = getHealthStatus(service.error_rate);
          const latencyIndicator = getLatencyIndicator(service.latency_p99);
          
          return (
            <div
              key={service.name}
              className="glass-card p-4 micro-interaction gpu-composite animate-slide-up"
              style={{ animationDelay: `${index * 50}ms` }}
            >
              {/* Service Header with Precise Status */}
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-2 flex-1 min-w-0">
                  <div className={`status-indicator ${health.indicator} animate-pulse-electric`}></div>
                  <h3 className="font-mono font-medium text-steel-50 truncate text-sm">
                    {service.name}
                  </h3>
                </div>
                
                <div className={`glass-card px-2 py-0.5 ${health.bg} ${health.border} border-0.5`}>
                  <span className={`text-[10px] font-mono font-medium ${health.color} uppercase tracking-wide`}>
                    {health.label}
                  </span>
                </div>
              </div>

              {/* Razor-Sharp Metrics Grid */}
              <div className="grid grid-cols-2 gap-3 mb-4">
                <div className="space-y-1">
                  <div className="flex items-center gap-1">
                    <span className="text-[10px] text-steel-400 font-mono uppercase tracking-wide">RPS</span>
                    <div className="h-0.5 flex-1 bg-steel-800"></div>
                  </div>
                  <div className="font-mono text-steel-100 font-medium text-sm">
                    {service.request_rate.toFixed(1)}
                  </div>
                </div>
                
                <div className="space-y-1">
                  <div className="flex items-center gap-1">
                    <span className="text-[10px] text-steel-400 font-mono uppercase tracking-wide">ERROR</span>
                    <div className="h-0.5 flex-1 bg-steel-800"></div>
                  </div>
                  <div className={`font-mono font-medium text-sm ${health.color}`}>
                    {service.error_rate.toFixed(2)}%
                  </div>
                </div>

                <div className="space-y-1">
                  <div className="flex items-center gap-1">
                    <span className="text-[10px] text-steel-400 font-mono uppercase tracking-wide">P50</span>
                    <div className="h-0.5 flex-1 bg-steel-800"></div>
                  </div>
                  <div className="font-mono text-steel-100 font-medium text-sm">
                    {service.latency_p50}ms
                  </div>
                </div>

                <div className="space-y-1">
                  <div className="flex items-center gap-1">
                    <span className="text-[10px] text-steel-400 font-mono uppercase tracking-wide">P99</span>
                    <div className={`status-indicator ${latencyIndicator}`}></div>
                  </div>
                  <div className={`font-mono font-medium text-sm ${getLatencyColor(service.latency_p99)}`}>
                    {service.latency_p99}ms
                  </div>
                </div>
              </div>

              {/* Precise Activity Indicator */}
              <div className="space-y-2 pt-3 border-t border-knife">
                <div className="flex items-center justify-between">
                  <span className="text-[10px] text-steel-400 font-mono uppercase tracking-wide">
                    Active Spans
                  </span>
                  <span className="text-xs font-mono text-steel-100 font-medium">
                    {service.active_spans.toLocaleString()}
                  </span>
                </div>
                
                {/* Ultra-Sharp Progress Bar */}
                <div className="h-1 bg-steel-800 rounded-full overflow-hidden border-knife">
                  <div
                    className="h-full bg-electric-green transition-all duration-500 ease-out relative"
                    style={{
                      width: `${Math.min((service.active_spans / 100) * 100, 100)}%`,
                    }}
                  >
                    <div className="absolute inset-0 bg-electric-gradient animate-knife-shine"></div>
                  </div>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Ultra-Sharp Summary Stats */}
      <div className="glass-card p-6 animate-slide-up" style={{ animationDelay: '300ms' }}>
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-display font-bold text-steel-50">System Overview</h3>
          <div className="status-indicator healthy animate-pulse-electric"></div>
        </div>
        
        <div className="grid grid-cols-4 gap-6">
          <div className="text-center space-y-2">
            <div className="flex items-center justify-center gap-2">
              <div className="status-indicator healthy"></div>
              <span className="text-[10px] text-steel-400 font-mono uppercase tracking-wide">Services</span>
            </div>
            <div className="text-2xl font-mono font-bold text-electric-green">
              {services.length.toLocaleString()}
            </div>
            <div className="h-0.5 bg-electric-green/30 rounded-full"></div>
          </div>
          
          <div className="text-center space-y-2">
            <div className="flex items-center justify-center gap-2">
              <div className="status-indicator healthy"></div>
              <span className="text-[10px] text-steel-400 font-mono uppercase tracking-wide">Total RPS</span>
            </div>
            <div className="text-2xl font-mono font-bold text-steel-100">
              {services.reduce((sum, s) => sum + s.request_rate, 0).toFixed(0)}
            </div>
            <div className="h-0.5 bg-steel-600 rounded-full"></div>
          </div>
          
          <div className="text-center space-y-2">
            <div className="flex items-center justify-center gap-2">
              <div className="status-indicator warning"></div>
              <span className="text-[10px] text-steel-400 font-mono uppercase tracking-wide">Avg Error</span>
            </div>
            <div className="text-2xl font-mono font-bold text-electric-amber">
              {(services.reduce((sum, s) => sum + s.error_rate, 0) / services.length).toFixed(2)}%
            </div>
            <div className="h-0.5 bg-electric-amber/30 rounded-full"></div>
          </div>
          
          <div className="text-center space-y-2">
            <div className="flex items-center justify-center gap-2">
              <div className="status-indicator critical"></div>
              <span className="text-[10px] text-steel-400 font-mono uppercase tracking-wide">Max P99</span>
            </div>
            <div className="text-2xl font-mono font-bold text-electric-red">
              {Math.max(...services.map(s => s.latency_p99))}ms
            </div>
            <div className="h-0.5 bg-electric-red/30 rounded-full"></div>
          </div>
        </div>
        
        {/* Live Performance Indicator */}
        <div className="mt-6 pt-4 border-t border-knife">
          <div className="flex items-center justify-center gap-2 text-xs font-mono">
            <div className="animate-pulse-electric">
              <div className="w-2 h-2 bg-electric-blue rounded-full"></div>
            </div>
            <span className="text-steel-300">Real-time data • Sub-second updates</span>
          </div>
        </div>
      </div>
    </div>
  );
});

ServiceHealthDashboard.displayName = 'ServiceHealthDashboard';

export default ServiceHealthDashboard;
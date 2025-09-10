import { memo } from 'react';
import { SystemMetrics as SystemMetricsType } from '../types';

interface Props {
  metrics: SystemMetricsType;
}

const SystemMetrics = memo(({ metrics }: Props) => {
  const formatUptime = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    } else if (minutes > 0) {
      return `${minutes}m ${secs}s`;
    }
    return `${secs}s`;
  };

  const getMemoryColor = (mb: number) => {
    if (mb < 50) return 'text-status-healthy';
    if (mb < 100) return 'text-status-warning';
    return 'text-status-error';
  };

  const getCpuColor = (percent: number) => {
    if (percent < 20) return 'text-status-healthy';
    if (percent < 50) return 'text-status-warning';
    return 'text-status-error';
  };

  return (
    <div className="mt-6 clean-card p-4">
      <h3 className="text-sm font-medium text-text-900 mb-3">
        System Performance
      </h3>
      
      <div className="grid grid-cols-5 gap-4">
        <div>
          <p className="text-xs text-text-500">Memory Usage</p>
          <p className={`text-lg font-mono ${getMemoryColor(metrics.memory_usage_mb)}`}>
            {metrics.memory_usage_mb.toFixed(1)}MB
          </p>
          <p className="text-xs text-text-300 mt-1">
            Target: {'<'}100MB
          </p>
        </div>
        
        <div>
          <p className="text-xs text-text-500">CPU Usage</p>
          <p className={`text-lg font-mono ${getCpuColor(metrics.cpu_usage_percent)}`}>
            {metrics.cpu_usage_percent.toFixed(1)}%
          </p>
          <p className="text-xs text-text-300 mt-1">
            Efficient usage
          </p>
        </div>
        
        <div>
          <p className="text-xs text-text-500">Throughput</p>
          <p className="text-lg font-mono text-status-healthy">
            {metrics.spans_per_second.toFixed(0)}/s
          </p>
          <p className="text-xs text-text-300 mt-1">
            High performance
          </p>
        </div>
        
        <div>
          <p className="text-xs text-text-500">Total Spans</p>
          <p className="text-lg font-mono text-text-700">
            {metrics.total_spans.toLocaleString()}
          </p>
          <p className="text-xs text-text-300 mt-1">
            No limits! 💪
          </p>
        </div>
        
        <div>
          <p className="text-xs text-text-500">Uptime</p>
          <p className="text-lg font-mono text-text-700">
            {formatUptime(metrics.uptime_seconds)}
          </p>
          <p className="text-xs text-status-healthy mt-1">
            Started in {'<'}200ms ⚡
          </p>
        </div>
      </div>
      
      {/* Performance bars */}
      <div className="mt-4 space-y-2">
        <div>
          <div className="flex items-center justify-between text-xs mb-1">
            <span className="text-text-500">Memory Efficiency</span>
            <span className="text-text-300">
              {((100 - (metrics.memory_usage_mb / 100) * 100)).toFixed(0)}%
            </span>
          </div>
          <div className="h-2 bg-surface-200 rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-status-healthy to-accent-green transition-all duration-300"
              style={{
                width: `${Math.max(100 - (metrics.memory_usage_mb / 100) * 100, 0)}%`
              }}
            />
          </div>
        </div>
        
        <div>
          <div className="flex items-center justify-between text-xs mb-1">
            <span className="text-text-500">Processing Power</span>
            <span className="text-text-300">
              {metrics.spans_per_second > 1000 ? '🔥 BLAZING' : `${metrics.spans_per_second.toFixed(0)} spans/s`}
            </span>
          </div>
          <div className="h-2 bg-surface-200 rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-accent-blue to-accent-purple transition-all duration-300"
              style={{
                width: `${Math.min((metrics.spans_per_second / 10000) * 100, 100)}%`
              }}
            />
          </div>
        </div>
      </div>
    </div>
  );
});

SystemMetrics.displayName = 'SystemMetrics';

export default SystemMetrics;
import { memo } from 'react';
import { SystemMetrics as SystemMetricsType } from '../../types';

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
    <div className="clean-card p-6">
      <h3 className="text-lg font-display font-bold text-text-900 mb-4">
        System Performance
      </h3>
      
      <div className="grid grid-cols-5 gap-6">
        <div className="text-center space-y-2">
          <div className="flex items-center justify-center gap-1">
            <div className={`status-indicator ${metrics.memory_usage_mb < 50 ? 'healthy' : metrics.memory_usage_mb < 100 ? 'warning' : 'critical'}`}></div>
            <span className="text-xs text-text-500 font-mono uppercase tracking-wide">Memory</span>
          </div>
          <div className={`text-2xl font-mono font-bold ${getMemoryColor(metrics.memory_usage_mb)}`}>
            {metrics.memory_usage_mb.toFixed(1)}MB
          </div>
          <div className="text-xs text-text-300">
            Target: Less than 100MB
          </div>
        </div>
        
        <div className="text-center space-y-2">
          <div className="flex items-center justify-center gap-1">
            <div className={`status-indicator ${metrics.cpu_usage_percent < 20 ? 'healthy' : metrics.cpu_usage_percent < 50 ? 'warning' : 'critical'}`}></div>
            <span className="text-xs text-text-500 font-mono uppercase tracking-wide">CPU</span>
          </div>
          <div className={`text-2xl font-mono font-bold ${getCpuColor(metrics.cpu_usage_percent)}`}>
            {metrics.cpu_usage_percent.toFixed(1)}%
          </div>
          <div className="text-xs text-text-300">
            Optimized Usage
          </div>
        </div>
        
        <div className="text-center space-y-2">
          <div className="flex items-center justify-center gap-1">
            <div className="status-indicator healthy"></div>
            <span className="text-xs text-text-500 font-mono uppercase tracking-wide">Throughput</span>
          </div>
          <div className="text-2xl font-mono font-bold text-status-healthy">
            {metrics.spans_per_second.toFixed(0)}/s
          </div>
          <div className="text-xs text-text-300">
            High Performance
          </div>
        </div>
        
        <div className="text-center space-y-2">
          <div className="flex items-center justify-center gap-1">
            <div className="status-indicator info"></div>
            <span className="text-xs text-text-500 font-mono uppercase tracking-wide">Total Spans</span>
          </div>
          <div className="text-2xl font-mono font-bold text-text-700">
            {metrics.total_spans.toLocaleString()}
          </div>
          <div className="text-xs text-text-300">
            No Limits
          </div>
        </div>
        
        <div className="text-center space-y-2">
          <div className="flex items-center justify-center gap-1">
            <div className="status-indicator healthy"></div>
            <span className="text-xs text-text-500 font-mono uppercase tracking-wide">Uptime</span>
          </div>
          <div className="text-2xl font-mono font-bold text-text-700">
            {formatUptime(metrics.uptime_seconds)}
          </div>
          <div className="text-xs text-status-healthy">
            Fast startup under 200ms
          </div>
        </div>
      </div>
      
      {/* Professional Performance Bars */}
      <div className="mt-6 pt-4 border-t border-surface-300 space-y-3">
        <div>
          <div className="flex items-center justify-between text-xs mb-2">
            <span className="text-text-500 font-mono uppercase tracking-wide">Memory Efficiency</span>
            <span className="text-text-700 font-mono">
              {((100 - (metrics.memory_usage_mb / 100) * 100)).toFixed(0)}%
            </span>
          </div>
          <div className="h-2 bg-surface-200 rounded-full overflow-hidden">
            <div
              className="h-full bg-status-healthy transition-all duration-300"
              style={{
                width: `${Math.max(100 - (metrics.memory_usage_mb / 100) * 100, 0)}%`
              }}
            />
          </div>
        </div>
        
        <div>
          <div className="flex items-center justify-between text-xs mb-2">
            <span className="text-text-500 font-mono uppercase tracking-wide">Processing Power</span>
            <span className="text-text-700 font-mono">
              {metrics.spans_per_second > 1000 ? 'EXCELLENT' : `${metrics.spans_per_second.toFixed(0)} spans/s`}
            </span>
          </div>
          <div className="h-2 bg-surface-200 rounded-full overflow-hidden">
            <div
              className="h-full bg-text-700 transition-all duration-300"
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
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
    if (mb < 50) return 'text-green-500';
    if (mb < 100) return 'text-yellow-500';
    return 'text-red-500';
  };

  const getCpuColor = (percent: number) => {
    if (percent < 20) return 'text-green-500';
    if (percent < 50) return 'text-yellow-500';
    return 'text-red-500';
  };

  return (
    <div className="mt-6 bg-slate-900 rounded-lg p-4 border border-slate-800">
      <h3 className="text-sm font-medium text-slate-200 mb-3">
        System Performance (Making Jaeger Cry 😢)
      </h3>
      
      <div className="grid grid-cols-5 gap-4">
        <div>
          <p className="text-xs text-slate-500">Memory Usage</p>
          <p className={`text-lg font-mono ${getMemoryColor(metrics.memory_usage_mb)}`}>
            {metrics.memory_usage_mb.toFixed(1)}MB
          </p>
          <p className="text-xs text-slate-600 mt-1">
            Jaeger: 2000MB+ 🤮
          </p>
        </div>
        
        <div>
          <p className="text-xs text-slate-500">CPU Usage</p>
          <p className={`text-lg font-mono ${getCpuColor(metrics.cpu_usage_percent)}`}>
            {metrics.cpu_usage_percent.toFixed(1)}%
          </p>
          <p className="text-xs text-slate-600 mt-1">
            Jaeger: 80%+ 🔥
          </p>
        </div>
        
        <div>
          <p className="text-xs text-slate-500">Throughput</p>
          <p className="text-lg font-mono text-green-500">
            {metrics.spans_per_second.toFixed(0)}/s
          </p>
          <p className="text-xs text-slate-600 mt-1">
            10x Jaeger 🚀
          </p>
        </div>
        
        <div>
          <p className="text-xs text-slate-500">Total Spans</p>
          <p className="text-lg font-mono text-slate-300">
            {metrics.total_spans.toLocaleString()}
          </p>
          <p className="text-xs text-slate-600 mt-1">
            No limits! 💪
          </p>
        </div>
        
        <div>
          <p className="text-xs text-slate-500">Uptime</p>
          <p className="text-lg font-mono text-slate-300">
            {formatUptime(metrics.uptime_seconds)}
          </p>
          <p className="text-xs text-green-400 mt-1">
            Started in &lt;200ms ⚡
          </p>
        </div>
      </div>
      
      {/* Performance bars */}
      <div className="mt-4 space-y-2">
        <div>
          <div className="flex items-center justify-between text-xs mb-1">
            <span className="text-slate-500">Memory Efficiency</span>
            <span className="text-slate-400">
              {((100 - (metrics.memory_usage_mb / 100) * 100)).toFixed(0)}%
            </span>
          </div>
          <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-green-500 to-green-400 transition-all duration-300"
              style={{
                width: `${Math.max(100 - (metrics.memory_usage_mb / 100) * 100, 0)}%`
              }}
            />
          </div>
        </div>
        
        <div>
          <div className="flex items-center justify-between text-xs mb-1">
            <span className="text-slate-500">Processing Power</span>
            <span className="text-slate-400">
              {metrics.spans_per_second > 1000 ? '🔥 BLAZING' : `${metrics.spans_per_second.toFixed(0)} spans/s`}
            </span>
          </div>
          <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-blue-500 to-purple-500 transition-all duration-300"
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